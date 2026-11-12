//! Finding and running the bundled flashing tools. Each tool runs from a
//! fixed argument template built by the engine; recipes only choose
//! partitions and files.

use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Looks for `name` in THIMSY_TOOLS_DIR, next to the engine (`tools/`), one
/// level up (`../tools/`, the app bundle layout) and then PATH.
pub fn locate(name: &str) -> Result<PathBuf> {
    let exe = format!("{name}{}", Host::current().exe_suffix());
    let mut dirs = Vec::new();
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
pub fn run(
    inv: &Invocation,
    cwd: &Path,
    rep: &Reporter,
    mut on_line: impl FnMut(&str),
) -> Result<()> {
    let exe = locate(&inv.tool)?;
    rep.info(format!("Running {}", inv.display()));
    let mut child = Command::new(&exe)
        .args(&inv.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("starting {}", exe.display()))?;
    {
        let mut stdin = child.stdin.take().unwrap();
        for l in &inv.stdin {
            writeln!(stdin, "{l}")?;
        }
    }
    let stderr = child.stderr.take().unwrap();
    let rep2 = rep.clone();
    let err_thread = std::thread::spawn(move || {
        let mut lines = Vec::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            rep2.info(line.clone());
            lines.push(line);
        }
        lines
    });
    let stdout = child.stdout.take().unwrap();
    // spd_dump prints prompts without newlines; split on '\n' and ':' prompts.
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        on_line(&line);
        rep.info(line);
    }
    let status = child.wait()?;
    let err_lines = err_thread.join().unwrap_or_default();
    if !status.success() {
        let last = err_lines.last().cloned().unwrap_or_default();
        return Err(EngineError::new(
            Code::ToolFailed,
            format!("{} failed ({status}): {last}", inv.tool),
        )
        .hint("Check the cable and that the device is still in its flashing mode, then try again.")
        .into());
    }
    Ok(())
}
