//! Finding and running the bundled flashing tools. Each tool runs from a
//! fixed argument template built by the engine; recipes only choose
//! partitions and files.

use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;

/// Looks for `name` in THIMSY_TOOLS_DIR, next to the engine (`tools/`), one
/// level up (`../tools/`, the app bundle layout) and then PATH.
pub fn locate(name: &str) -> Result<PathBuf> {
    let exe = format!("{name}{}", Host::current().exe_suffix());
    let mut dirs = Vec::new();
    dirs.extend(test_tools_dir());
    if let Ok(d) = std::env::var("THIMSY_TOOLS_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            dirs.push(dir.join("tools"));
            dirs.push(dir.join("..").join("tools"));
            dirs.push(dir.to_path_buf());
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    for d in dirs {
        let p = d.join(&exe);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(EngineError::new(
        Code::ToolMissing,
        format!("{exe} isn't bundled with this installer"),
    )
    .hint("Reinstall Thimsy Installer, or set THIMSY_TOOLS_DIR to a folder containing it.")
    .into())
}

/// A tool command line, kept as data so it can be shown (dry run) or run.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub tool: String,
    pub args: Vec<String>,
    /// Lines written to stdin (confirmations the tool asks for).
    pub stdin: Vec<String>,
    /// Text the tool prints when it asks for confirmation. When set, stdin
    /// stays open and the next `stdin` line is written only when the tool's
    /// output shows this text (even without a newline after it); once the
    /// answers run out, or the caller refuses one, stdin is closed so any
    /// further question fails instead of being answered. When unset, every
    /// line is written up front.
    pub answer_prompt: Option<String>,
}

impl Invocation {
    pub fn display(&self) -> String {
        let quote = |a: &String| {
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        };
        std::iter::once(self.tool.clone())
            .chain(self.args.iter().map(quote))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Runs `inv`, forwarding output as log events and calling `on_line` for
/// each line (for progress parsing).
pub fn run(inv: &Invocation, cwd: &Path, rep: &Reporter, on_line: impl FnMut(&str)) -> Result<()> {
    run_answering(inv, cwd, rep, on_line, |_| Ok(()))
}

/// Like [`run`], with `approve(n)` called before the `n`th answer (from 0)
/// is given to the tool's prompt. If it fails, the answer is withheld,
/// stdin is closed (the tool gives up without doing the operation), and
/// its error is returned once the tool has exited.
pub fn run_answering(
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    on_line: impl FnMut(&str),
    approve: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    let exe = locate(&inv.tool)?;
    run_exe(&exe, inv, cwd, rep, on_line, approve)
}

/// What the output readers report, in arrival order.
enum Out {
    Line {
        stderr: bool,
        text: String,
    },
    /// The tool is waiting for an answer.
    Prompt,
}

/// Keeps a console window from opening for a child process on Windows.
pub(crate) fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Makes the child die with the engine (Linux: the kernel sends it SIGKILL
/// when the engine goes away, however it ends).
fn die_with_engine(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        let engine = std::process::id() as libc::pid_t;
        // SAFETY: only async-signal-safe calls between fork and exec.
        unsafe {
            cmd.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // The engine already died between fork and prctl.
                if libc::getppid() != engine {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

/// Windows: a job object that kills the child when its last handle (held
/// by the engine) closes, i.e. when the engine exits or is killed.
#[cfg(windows)]
struct KillOnClose(std::os::windows::io::OwnedHandle);

#[cfg(windows)]
fn kill_with_engine(child: &Child) -> Option<KillOnClose> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    // SAFETY: plain Win32 calls on handles we own; failures leave the child
    // running as before (the answers on demand still keep it harmless).
    unsafe {
        let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if raw.is_null() {
            return None;
        }
        let job = OwnedHandle::from_raw_handle(raw as _);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job.as_raw_handle() as _,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0
            || AssignProcessToJobObject(job.as_raw_handle() as _, child.as_raw_handle() as _) == 0
        {
            return None;
        }
        Some(KillOnClose(job))
    }
}

#[cfg(not(windows))]
fn kill_with_engine(_child: &Child) -> Option<()> {
    None
}

/// Longest partial line kept while waiting for its newline.
const MAX_PENDING: usize = 64 * 1024;

/// Reads one output stream to its end, whatever it contains: bytes are
/// decoded lossily, and the stream keeps being drained even when nobody
/// listens any more, so the tool never blocks or dies writing to it.
fn pump(mut r: impl Read, stderr: bool, prompt: Option<String>, tx: Sender<Out>) {
    let mut buf = [0u8; 8192];
    let mut pending: Vec<u8> = Vec::new();
    let line = |bytes: &[u8]| Out::Line {
        stderr,
        text: String::from_utf8_lossy(bytes)
            .trim_end_matches(['\r', '\n'])
            .to_string(),
    };
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        pending.extend_from_slice(&buf[..n]);
        while let Some(i) = pending.iter().position(|&b| b == b'\n') {
            let l: Vec<u8> = pending.drain(..=i).collect();
            let _ = tx.send(line(&l));
        }
        // A question ends without a newline: look for it in the partial line.
        let asked = prompt.as_ref().map_or(0, |p| count(&pending, p.as_bytes()));
        if asked > 0 || pending.len() > MAX_PENDING {
            let _ = tx.send(line(&pending));
            pending.clear();
            for _ in 0..asked {
                let _ = tx.send(Out::Prompt);
            }
        }
    }
    if !pending.is_empty() {
        let _ = tx.send(line(&pending));
    }
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || hay.len() < needle.len() {
        return 0;
    }
    let mut n = 0;
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            n += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    n
}

pub(crate) fn run_exe(
    exe: &Path,
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    mut on_line: impl FnMut(&str),
    mut approve: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    rep.info(format!("Running {}", inv.display()));
    let mut cmd = Command::new(exe);
    cmd.args(&inv.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.envs(test_env());
    hide_window(&mut cmd);
    die_with_engine(&mut cmd);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {}", exe.display()))?;
    let _job = kill_with_engine(&child);

    let mut stdin = child.stdin.take();
    if inv.answer_prompt.is_none() {
        if let Some(mut w) = stdin.take() {
            for l in &inv.stdin {
                // A tool that exits without reading is reported by its status.
                if writeln!(w, "{l}").is_err() {
                    break;
                }
            }
        }
    } else if inv.stdin.is_empty() {
        stdin = None;
    }

    // Both streams reach on_line in arrival order: spd_dump logs progress
    // (load_partition, BSL_REP_VER) on stderr and prompts on stdout.
    let (tx, rx) = std::sync::mpsc::channel::<Out>();
    let stderr = child.stderr.take().unwrap();
    let tx_err = tx.clone();
    let err_thread = std::thread::spawn(move || pump(stderr, true, None, tx_err));
    let stdout = child.stdout.take().unwrap();
    let prompt = inv.answer_prompt.clone();
    let out_thread = std::thread::spawn(move || pump(stdout, false, prompt, tx));

    let mut err_lines = Vec::new();
    let mut answered = 0;
    let mut refused: Option<anyhow::Error> = None;
    for ev in rx {
        match ev {
            Out::Line { stderr, text } => {
                rep.info(text.clone());
                on_line(&text);
                if stderr {
                    err_lines.push(text);
                }
            }
            Out::Prompt => {
                let answer = inv.stdin.get(answered);
                let (Some(w), Some(answer)) = (stdin.as_mut(), answer) else {
                    stdin = None; // nothing (more) to say: the tool gives up
                    continue;
                };
                if let Err(e) = approve(answered) {
                    refused = Some(e);
                    stdin = None;
                    continue;
                }
                answered += 1;
                if writeln!(w, "{answer}").and_then(|_| w.flush()).is_err()
                    || answered == inv.stdin.len()
                {
                    stdin = None;
                }
            }
        }
    }
    drop(stdin);
    let _ = err_thread.join();
    let _ = out_thread.join();
    let status = child.wait()?;
    if let Some(e) = refused {
        return Err(e);
    }
    if !status.success() {
        let last = err_lines
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_default();
        return Err(EngineError::new(
            Code::ToolFailed,
            format!("{} failed ({status}): {last}", inv.tool),
        )
        .hint("Check the cable and that the device is still in its flashing mode, then try again.")
        .into());
    }
    Ok(())
}

#[cfg(not(test))]
fn test_tools_dir() -> Option<PathBuf> {
    None
}

#[cfg(not(test))]
fn test_env() -> Vec<(String, String)> {
    Vec::new()
}

// Kept last: `dev i18n-strings` stops reading a file at its first
// test-only item.

/// Test hooks: a tools folder and extra environment for children, per
/// thread (tests run in parallel; the real environment is shared).
#[cfg(test)]
fn test_tools_dir() -> Option<PathBuf> {
    test::TOOLS_DIR.with(|d| d.borrow().clone())
}

#[cfg(test)]
fn test_env() -> Vec<(String, String)> {
    test::ENV.with(|e| e.borrow().clone())
}

#[cfg(test)]
pub(crate) mod test {
    use std::cell::RefCell;
    use std::path::PathBuf;
    thread_local! {
        pub static TOOLS_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
        pub static ENV: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    }
}
