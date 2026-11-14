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

/// Tools that may come from PATH when they aren't bundled. Never the
/// flashing tools: the engine relies on how its own `spd_dump` asks before
/// every write, and `wdi-simple` runs with admin rights.
const FROM_PATH: &[&str] = &["fastboot"];

/// Tools that only ever come from the installer's own folders: never from
/// THIMSY_TOOLS_DIR, even in test builds (it runs with admin rights).
const BUNDLED_ONLY: &[&str] = &["wdi-simple"];

/// Looks for `name` next to the engine (`tools/`), one level up
/// (`../tools/`, the app bundle layout), and then PATH only for the tools
/// in [`FROM_PATH`]. Builds with the `test-hooks` feature (tests and
/// `installer/tests/e2e.sh`, never release artifacts) look in
/// THIMSY_TOOLS_DIR first, except for [`BUNDLED_ONLY`] tools.
pub fn locate(name: &str) -> Result<PathBuf> {
    let exe = format!("{name}{}", Host::current().exe_suffix());
    for d in search_dirs(name, std::env::var_os("PATH")) {
        let p = d.join(&exe);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(EngineError::new(
        Code::ToolMissing,
        format!("{exe} isn't bundled with this installer"),
    )
    .hint("Reinstall Thimsy Installer: one of its tools is missing.")
    .into())
}

/// Where [`locate`] looks for `name`, in order.
pub(crate) fn search_dirs(name: &str, path: Option<std::ffi::OsString>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if !BUNDLED_ONLY.contains(&name) {
        dirs.extend(test_tools_dir());
        #[cfg(feature = "test-hooks")]
        if let Ok(d) = std::env::var("THIMSY_TOOLS_DIR") {
            dirs.push(PathBuf::from(d));
        }
    }
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            dirs.push(dir.join("tools"));
            dirs.push(dir.join("..").join("tools"));
            dirs.push(dir.to_path_buf());
        }
    }
    if FROM_PATH.contains(&name) {
        if let Some(path) = path {
            dirs.extend(std::env::split_paths(&path));
        }
    }
    dirs
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
    /// further question fails instead of being answered; exiting before
    /// every answer was asked for is an error. When unset, every line is
    /// written up front.
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
/// each line (for progress parsing). If `on_line` fails, nothing more is
/// answered, the tool is killed and that error is returned.
pub fn run(
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    on_line: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    run_answering(inv, cwd, rep, on_line, |_| Ok(()))
}

/// Like [`run`], with `approve(n)` called before the `n`th answer (from 0)
/// is given to the tool's prompt. If it fails, the answer is withheld,
/// stdin is closed (the tool gives up without doing the operation), and
/// its error is returned once the tool has exited.
///
/// With `answer_prompt` set, a tool that exits without having asked for
/// every answer fails too: it didn't wait for the confirmations the caller
/// counted on. If the app goes away before the first answer, the tool is
/// killed; after it, the tool is left to finish (stopping a write halfway
/// is worse, and every further step is still confirmed one by one).
pub fn run_answering(
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    on_line: impl FnMut(&str) -> Result<()>,
    approve: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    let exe = locate(&inv.tool)?;
    run_exe(&exe, inv, cwd, rep, on_line, approve)
}

/// Like [`run_answering`], with `tick` also called about every 200 ms while
/// the tool runs (whether or not it prints anything). If it fails, the tool
/// is killed as when `on_line` fails, and its error is returned.
pub fn run_answering_ticking(
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    on_line: impl FnMut(&str) -> Result<()>,
    approve: impl FnMut(usize) -> Result<()>,
    tick: impl FnMut() -> Result<()>,
) -> Result<()> {
    let exe = locate(&inv.tool)?;
    run_exe_ticking(&exe, inv, cwd, rep, on_line, approve, tick)
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
fn kill_with_engine(child: &Child) -> std::io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    // SAFETY: plain Win32 calls on handles we own.
    unsafe {
        let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if raw.is_null() {
            return Err(std::io::Error::last_os_error());
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
            return Err(std::io::Error::last_os_error());
        }
        Ok(job)
    }
}

/// Windows: resumes the threads of a process created suspended (just its
/// main thread).
#[cfg(windows)]
fn resume(pid: u32) -> std::io::Result<()> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
    // SAFETY: plain Win32 calls; every handle opened here is closed here.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let snap = OwnedHandle::from_raw_handle(snap as _);
        let mut te: THREADENTRY32 = std::mem::zeroed();
        te.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut resumed = 0;
        let mut more = Thread32First(snap.as_raw_handle() as _, &mut te);
        while more != 0 {
            if te.th32OwnerProcessID == pid {
                let t = OpenThread(THREAD_SUSPEND_RESUME, 0, te.th32ThreadID);
                if !t.is_null() {
                    if ResumeThread(t) != u32::MAX {
                        resumed += 1;
                    }
                    CloseHandle(t);
                }
            }
            more = Thread32Next(snap.as_raw_handle() as _, &mut te);
        }
        if resumed == 0 {
            return Err(std::io::Error::other(
                "no thread of the new process could be resumed",
            ));
        }
        Ok(())
    }
}

/// Starts a child created by [`run_exe`]. Windows: it was created
/// suspended, so it joins the kill-on-close job before it runs a single
/// instruction; then it's resumed.
#[cfg(windows)]
fn start_child(
    child: &mut Child,
    tool: &str,
    rep: &Reporter,
) -> Result<Option<std::os::windows::io::OwnedHandle>> {
    let job = match kill_with_engine(child) {
        Ok(j) => Some(j),
        Err(e) => {
            rep.warn(format!(
                "{tool} can't be tied to the installer ({e}); if the installer is closed while it runs, it keeps running until it next asks for a confirmation"
            ));
            None
        }
    };
    if let Err(e) = resume(child.id()) {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("couldn't start {tool}: {e}");
    }
    Ok(job)
}

#[cfg(not(windows))]
fn start_child(_child: &mut Child, _tool: &str, _rep: &Reporter) -> Result<Option<()>> {
    Ok(None)
}

/// The running tool's process ID (Unix), for the signal handler: the
/// tool is killed when the engine is stopped with SIGTERM, SIGINT or
/// SIGHUP (on macOS nothing else would stop it).
#[cfg(unix)]
static CHILD_PID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

#[cfg(unix)]
fn set_child_pid(pid: u32) {
    CHILD_PID.store(pid as i32, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(unix))]
fn set_child_pid(_pid: u32) {}

#[cfg(unix)]
extern "C" fn on_stop_signal(sig: libc::c_int) {
    let pid = CHILD_PID.load(std::sync::atomic::Ordering::SeqCst);
    // SAFETY: kill and _exit are async-signal-safe.
    unsafe {
        if pid > 0 {
            libc::kill(pid, libc::SIGKILL);
        }
        libc::_exit(128 + sig);
    }
}

/// Installs the handler for SIGTERM, SIGINT and SIGHUP (once; signals the
/// engine was started ignoring stay ignored).
#[cfg(unix)]
fn install_signal_handlers() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            // SAFETY: sigaction with a zeroed struct and an async-signal-safe
            // handler.
            unsafe {
                let mut old: libc::sigaction = std::mem::zeroed();
                if libc::sigaction(sig, std::ptr::null(), &mut old) != 0
                    || old.sa_sigaction == libc::SIG_IGN
                {
                    continue;
                }
                let mut sa: libc::sigaction = std::mem::zeroed();
                sa.sa_sigaction = on_stop_signal as extern "C" fn(libc::c_int) as usize;
                libc::sigemptyset(&mut sa.sa_mask);
                libc::sigaction(sig, &sa, std::ptr::null_mut());
            }
        }
    });
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

/// Unix: notices when the app that started the engine dies (the engine is
/// handed to another parent) and records it like a failed write to the app
/// (see [`crate::events::parent_gone`]). Call once at startup.
#[cfg(unix)]
pub fn watch_parent() {
    // SAFETY: getppid can't fail.
    let first = unsafe { libc::getppid() };
    if first <= 1 {
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        // SAFETY: as above.
        if unsafe { libc::getppid() } != first {
            crate::events::set_parent_gone();
            return;
        }
    });
}

#[cfg(not(unix))]
pub fn watch_parent() {}

fn app_gone() -> anyhow::Error {
    EngineError::new(
        Code::Cancelled,
        "the installer app went away before anything was written; stopped",
    )
    .into()
}

fn parent_is_gone() -> bool {
    crate::events::parent_gone() || test_parent_gone()
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
            // A question with a newline after it, maybe in the same read.
            let asked = prompt.as_ref().map_or(0, |p| count(&l, p.as_bytes()));
            let _ = tx.send(line(&l));
            for _ in 0..asked {
                let _ = tx.send(Out::Prompt);
            }
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
    on_line: impl FnMut(&str) -> Result<()>,
    approve: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    run_exe_ticking(exe, inv, cwd, rep, on_line, approve, || Ok(()))
}

pub(crate) fn run_exe_ticking(
    exe: &Path,
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    mut on_line: impl FnMut(&str) -> Result<()>,
    mut approve: impl FnMut(usize) -> Result<()>,
    mut tick: impl FnMut() -> Result<()>,
) -> Result<()> {
    if parent_is_gone() {
        return Err(app_gone());
    }
    install_signal_handlers();
    rep.info(format!("Running {}", inv.display()));
    let mut cmd = Command::new(exe);
    cmd.args(&inv.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.envs(test_env());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }
    die_with_engine(&mut cmd);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("starting {}", exe.display()))?;
    let _job = start_child(&mut child, &inv.tool, rep)?;
    set_child_pid(child.id());

    let mut stdin = child.stdin.take();
    let up_front = inv.answer_prompt.is_none();
    if up_front {
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
    // Answers given so far (all of them, when written up front).
    let mut answered = if up_front { inv.stdin.len() } else { 0 };
    // Why the run stopped early: a refused answer, a fatal line, the app
    // going away. Nothing more is answered once it's set.
    let mut stopped: Option<anyhow::Error> = None;
    // Set when the engine killed the tool.
    let mut killed = false;
    // Events taken early (see the prompt below) and not handled yet.
    let mut queue = std::collections::VecDeque::new();
    let mut last_tick = std::time::Instant::now();
    loop {
        let ev = match queue.pop_front() {
            Some(ev) => Some(ev),
            None => match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(ev) => Some(ev),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            },
        };
        if stopped.is_none() && answered == 0 && parent_is_gone() {
            stopped = Some(app_gone());
            stdin = None;
            killed = child.kill().is_ok();
        }
        if stopped.is_none() && last_tick.elapsed() >= std::time::Duration::from_millis(200) {
            last_tick = std::time::Instant::now();
            if let Err(e) = tick() {
                stopped = Some(e);
                stdin = None;
                killed = child.kill().is_ok();
            }
        }
        if killed {
            // Already sent SIGKILL: never signal its ID again once reaped.
            set_child_pid(0);
        }
        if killed && matches!(child.try_wait(), Ok(Some(_))) {
            // A process the tool started may keep its output open: don't
            // wait for that, just log what already arrived.
            while let Some(ev) = queue.pop_front().or_else(|| rx.try_recv().ok()) {
                if let Out::Line { text, .. } = ev {
                    rep.info(text);
                }
            }
            break;
        }
        match ev {
            None => {}
            Some(Out::Line { stderr, text }) => {
                rep.info(text.clone());
                if let Err(e) = on_line(&text) {
                    if stopped.is_none() {
                        stopped = Some(e);
                        stdin = None;
                        killed = child.kill().is_ok();
                    }
                }
                if stderr {
                    err_lines.push(text);
                }
            }
            Some(Out::Prompt) => {
                if stopped.is_some() {
                    continue;
                }
                // stdout and stderr are read separately, so lines the tool
                // printed before asking (say, that the last write failed)
                // may still be on their way: the tool waits for its answer,
                // so once it's quiet, handle them first.
                let mut later = Vec::new();
                while let Ok(ev) = rx.recv_timeout(std::time::Duration::from_millis(50)) {
                    later.push(ev);
                }
                if later.iter().any(|e| matches!(e, Out::Line { .. })) {
                    queue.extend(later);
                    queue.push_back(Out::Prompt);
                    continue;
                }
                queue.extend(later);
                let answer = inv.stdin.get(answered);
                let (Some(w), Some(answer)) = (stdin.as_mut(), answer) else {
                    stdin = None; // nothing (more) to say: the tool gives up
                    continue;
                };
                if let Err(e) = approve(answered) {
                    stopped = Some(e);
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
    if !killed {
        let _ = err_thread.join();
        let _ = out_thread.join();
    }
    // Its output is closed (or it was killed): it's exiting. Forget it
    // before it's reaped so the signal handler can never hit a reused
    // process ID.
    set_child_pid(0);
    let status = child.wait()?;
    if let Some(e) = stopped {
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
    if answered < inv.stdin.len() {
        return Err(EngineError::new(
            Code::ToolFailed,
            format!(
                "{} finished without asking to confirm {} of its {} operations, so they weren't checked before they ran",
                inv.tool,
                inv.stdin.len() - answered,
                inv.stdin.len()
            ),
        )
        .hint("Reinstall Thimsy Installer: its flashing tool isn't the version it expects.")
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

#[cfg(not(test))]
fn test_parent_gone() -> bool {
    false
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
fn test_parent_gone() -> bool {
    test::PARENT_GONE.with(|g| g.get())
}

#[cfg(test)]
pub(crate) mod test {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    thread_local! {
        pub static TOOLS_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
        pub static ENV: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
        /// Pretends the app went away (per thread; the real flag is global).
        pub static PARENT_GONE: Cell<bool> = const { Cell::new(false) };
    }
}
