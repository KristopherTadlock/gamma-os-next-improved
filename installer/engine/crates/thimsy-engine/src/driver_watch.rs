//! Windows: switching a Unisoc device to WinUSB the moment it's connected.
//!
//! When Windows also has another driver package for the download-mode ID
//! (Unisoc's own "SPRD U2S Diag", which unlocking needs), it gives the
//! device that driver, which the flashing tool can't use. Only an install
//! made while the device is connected replaces it, and download mode lasts
//! a few seconds, too short to ask for permission then. So before the user
//! connects the device, one elevated helper starts (one UAC prompt) and
//! waits; when `spd_dump` reports the device on another driver, the engine
//! asks the helper, which forces the WinUSB package already in the driver
//! store onto it (`UpdateDriverForPlugAndPlayDevicesW` with
//! `INSTALLFLAG_FORCE`) and reports back. Unisoc's package stays installed.
//!
//! The engine talks to the helper through files. It writes `trigger` (a
//! count; each increase asks for one switch) and `stop` in a new folder in
//! its temp folder (a local, absolute path the elevated helper can read,
//! unlike a mapped drive). The helper only reads those, as a number or by
//! their existence, and writes its `status` in a new folder under
//! `<ProgramData>\Thimsy Installer` that only administrators and SYSTEM can
//! change (signed-in users may read it), so nothing it writes can be
//! redirected by the user.

use crate::catalog::Device;
use crate::drivers::{ps_literal, ADMIN_PRELUDE, UNSAFE_ROOT};
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// What the helper reports in its `status` file.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Waiting for the engine.
    Ready,
    /// Switch `n` worked; Windows may want a restart to finish it.
    Switched { n: u32, reboot: bool },
    /// Switch `n` failed with this Windows error.
    Failed { n: u32, code: u32 },
}

/// `ERROR_NO_SUCH_DEVINST`: no connected device has the ID (it left
/// download mode before the switch).
pub const NO_DEVICE: u32 = 0xE000_020B;

/// The helper's `status` text: `ready end`, `switched N end`, `switched N
/// reboot end` or `failed N CODE end` (CODE in hex). The final `end` means
/// it was read whole; anything else, such as a file caught half-written,
/// is `None`.
pub fn parse_status(text: &str) -> Option<Status> {
    let mut w: Vec<&str> = text.split_whitespace().collect();
    if w.pop()? != "end" {
        return None;
    }
    match w.as_slice() {
        ["ready"] => Some(Status::Ready),
        ["switched", n] => Some(Status::Switched {
            n: n.parse().ok()?,
            reboot: false,
        }),
        ["switched", n, "reboot"] => Some(Status::Switched {
            n: n.parse().ok()?,
            reboot: true,
        }),
        ["failed", n, c] if c.len() == 8 => Some(Status::Failed {
            n: n.parse().ok()?,
            code: u32::from_str_radix(c, 16).ok()?,
        }),
        _ => None,
    }
}

/// Why a switch failed, for the error message.
pub fn failure_text(code: u32) -> String {
    match code {
        NO_DEVICE => "the device was no longer connected".to_string(),
        5 => "access denied".to_string(),
        c => format!("Windows error 0x{c:08x}"),
    }
}

/// Why the helper stopped, from its exit code.
pub fn exit_text(code: i32) -> String {
    match code {
        4 => "Windows has no WinUSB package for the device".to_string(),
        5 => "its status folder already existed".to_string(),
        6 => UNSAFE_ROOT.to_string(),
        c => format!("exit code {c}"),
    }
}

/// Whether Windows has another driver package than WinUSB for one of the
/// device's download-mode IDs, which it would give the device instead.
pub fn needed(d: &Device) -> bool {
    d.identify
        .usb
        .iter()
        .any(|u| crate::drivers::other_installed(u.vid, u.pid))
}

/// PowerShell's `Test-WinUsbInf $text $key`: the same test as
/// [`crate::drivers::inf_is_winusb_for`] (`$key` like `VID_1782&PID_4D00`).
pub const WINUSB_INF_RULE: &str = r#"function Test-WinUsbInf([string]$text, [string]$key) {
  $t = ($text -replace '\s', '').ToUpperInvariant()
  $services = [regex]::Matches($t, 'ADDSERVICE=').Count
  return $t.Contains($key.ToUpperInvariant()) -and $t.Contains('INCLUDE=WINUSB.INF') -and $services -gt 0 -and [regex]::Matches($t, 'ADDSERVICE=WINUSB,').Count -eq $services
}
"#;

/// The script the elevated PowerShell runs (see the module notes): `ids`
/// the download-mode IDs, `status_name` the name of its status folder under
/// `<ProgramData>\Thimsy Installer` (made fresh, or the script stops),
/// `ipc` the absolute folder with `trigger` and `stop`. Exit codes: 0
/// stopped or timed out (20 minutes), 4 no WinUSB package for an ID, 5 the
/// status folder already existed, 6 the ProgramData folder isn't safe to
/// use, 1 anything else.
pub fn script(ids: &[(u16, u16)], status_name: &str, ipc: &Path) -> String {
    let ids: Vec<String> = ids
        .iter()
        .map(|(v, p)| ps_literal(&format!("USB\\VID_{v:04X}&PID_{p:04X}")))
        .collect();
    let mut s = String::from(ADMIN_PRELUDE);
    s.push_str(WINUSB_INF_RULE);
    s.push_str(&format!(
        "$dir = Join-Path $root {}\n$trigger = {}\n$stop = {}\n$ids = @({})\n",
        ps_literal(status_name),
        ps_literal(&ipc.join("trigger").display().to_string()),
        ps_literal(&ipc.join("stop").display().to_string()),
        ids.join(", ")
    ));
    s.push_str(
        r#"$status = Join-Path $dir 'status'
$made = $false
function Say([string]$text) { [System.IO.File]::WriteAllText($status, "$text end") }
# The engine's count; 0 while the file is missing, being written, or not a
# number.
function Read-Trigger {
  try {
    if (Test-Path -LiteralPath $trigger) {
      $n = 0
      if ([int]::TryParse(([System.IO.File]::ReadAllText($trigger)).Trim(), [ref]$n)) { return $n }
    }
  } catch { }
  return 0
}
try {
  Assert-AdminRoot
  if (Test-Path -LiteralPath $dir) { $code = 5; throw 'the status folder already exists' }
  [void][System.IO.Directory]::CreateDirectory($dir, (New-AdminSecurity $true))
  $made = $true
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ThimsyNewDev {
    [DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool UpdateDriverForPlugAndPlayDevicesW(IntPtr parent, string hardwareId, string infPath, uint flags, out bool reboot);
}
'@
  # The WinUSB package for each ID, as Windows published it (the Windows
  # folder from the system, not the environment).
  $infs = @{}
  $published = Get-ChildItem -LiteralPath (Join-Path ([Environment]::GetFolderPath('Windows')) 'INF') -Filter 'oem*.inf'
  foreach ($id in $ids) {
    $key = $id.Substring(4)
    $inf = $published | Where-Object { try { Test-WinUsbInf (Get-Content -LiteralPath $_.FullName -Raw) $key } catch { $false } } | Select-Object -First 1
    if (-not $inf) { $code = 4; throw "no WinUSB package for $id" }
    $infs[$id] = $inf.FullName
  }
  Say 'ready'
  $done = 0
  $until = [DateTime]::UtcNow.AddMinutes(20)
  while ([DateTime]::UtcNow -lt $until -and -not (Test-Path -LiteralPath $stop)) {
    $n = Read-Trigger
    if ($n -gt $done) {
      $done = $n
      # FORCE | NONINTERACTIVE: replace the device's driver, and fail
      # rather than wait on a dialog nobody sees.
      $switched = $false; $reboot = $false; $err = 0
      foreach ($id in $ids) {
        $r = $false
        if ([ThimsyNewDev]::UpdateDriverForPlugAndPlayDevicesW([IntPtr]::Zero, $id, $infs[$id], 5, [ref]$r)) {
          $switched = $true
          if ($r) { $reboot = $true }
        } else {
          $err = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        }
      }
      if ($switched) {
        if ($reboot) { Say "switched $n reboot" } else { Say "switched $n" }
      } else {
        Say ("failed $n " + ('{0:x8}' -f $err))
      }
    }
    Start-Sleep -Milliseconds 100
  }
  $code = 0
} catch {
  if ($code -eq 0) { $code = 1 }
} finally {
  if ($made) { Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue }
}
exit $code
"#,
    );
    s
}

/// The running helper. Dropping it asks the helper to stop and, once it
/// has, removes the engine's folder for it.
pub struct DriverWatch {
    ipc: PathBuf,
    status: PathBuf,
    /// Switches asked for, and how many of those `trigger` holds.
    requested: u32,
    written: u32,
    last: String,
    process: Option<Process>,
}

impl DriverWatch {
    /// Starts the helper for the device's download-mode IDs (one UAC
    /// prompt) and waits until it's ready.
    pub fn start(d: &Device, rep: &Reporter) -> Result<DriverWatch> {
        let name = status_name();
        let ipc = std::path::absolute(std::env::temp_dir())?.join(format!("thimsy-{name}"));
        std::fs::create_dir(&ipc)?;
        let ids: Vec<(u16, u16)> = d.identify.usb.iter().map(|u| (u.vid, u.pid)).collect();
        let status = program_data()
            .join("Thimsy Installer")
            .join(&name)
            .join("status");
        rep.info("Starting the driver switch; Windows asks for permission");
        let process = match Process::start(&script(&ids, &name, &ipc)) {
            Ok(p) => p,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&ipc);
                return Err(e);
            }
        };
        let mut watch = DriverWatch::new(ipc, status, Some(process));
        watch.wait_ready()?;
        Ok(watch)
    }

    fn new(ipc: PathBuf, status: PathBuf, process: Option<Process>) -> DriverWatch {
        DriverWatch {
            ipc,
            status,
            requested: 0,
            written: 0,
            last: String::new(),
            process,
        }
    }

    /// Asks the helper for one switch (written now, or by the next
    /// [`DriverWatch::step`] if the helper was reading the file).
    pub fn request(&mut self) {
        self.requested += 1;
        self.write_trigger();
    }

    fn write_trigger(&mut self) {
        if self.written < self.requested
            && std::fs::write(self.ipc.join("trigger"), self.requested.to_string()).is_ok()
        {
            self.written = self.requested;
        }
    }

    /// Hands on a pending request and returns the helper's status when it
    /// changed. Fails if the helper stopped.
    pub fn step(&mut self) -> Result<Option<Status>> {
        self.write_trigger();
        // Its last word first: it may have reported before it stopped.
        let status = self.poll();
        if status.is_none() {
            if let Some(code) = self.process.as_ref().and_then(Process::exit_code) {
                return Err(stopped_error(&format!(
                    "the driver switch stopped ({})",
                    exit_text(code)
                )));
            }
        }
        Ok(status)
    }

    /// The helper's status, when it changed since the last call.
    fn poll(&mut self) -> Option<Status> {
        let text = std::fs::read_to_string(&self.status).ok()?;
        let status = parse_status(&text)?;
        if text == self.last {
            return None;
        }
        self.last = text;
        Some(status)
    }

    /// Waits (up to 2 minutes) for the helper to say it's ready.
    fn wait_ready(&mut self) -> Result<()> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while std::time::Instant::now() < until {
            if self.poll().is_some() {
                return Ok(());
            }
            if let Some(code) = self.process.as_ref().and_then(Process::exit_code) {
                return Err(stopped_error(&format!(
                    "the driver switch couldn't start ({})",
                    exit_text(code)
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err(stopped_error(
            "the driver switch couldn't start (it didn't get ready within 2 minutes)",
        ))
    }
}

impl Drop for DriverWatch {
    fn drop(&mut self) {
        let _ = std::fs::write(self.ipc.join("stop"), "");
        // A switch in progress may take a while; the helper stops after it
        // (`stop` stays until then).
        let ended = self.process.as_ref().is_none_or(|p| p.wait(5000));
        if ended {
            let _ = std::fs::remove_dir_all(&self.ipc);
        }
    }
}

/// A fresh name for the helper's folders.
fn status_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("watch-{nanos:x}-{:x}", std::process::id())
}

fn program_data() -> PathBuf {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

fn stopped_error(message: &str) -> anyhow::Error {
    EngineError::new(Code::ToolFailed, format!("{message}; nothing was written"))
        .hint("Accept the Windows permission prompt, or install WinUSB with Zadig.")
        .into()
}

/// The elevated helper process.
#[cfg(windows)]
struct Process(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Process {
    /// Runs `script` in a hidden, elevated Windows PowerShell (one UAC
    /// prompt) without waiting for it.
    fn start(script: &str) -> Result<Process> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };
        let wide = |s: &std::ffi::OsStr| -> Vec<u16> { s.encode_wide().chain([0]).collect() };
        let exe = wide(crate::drivers::powershell_path()?.as_os_str());
        let verb = wide(std::ffi::OsStr::new("runas"));
        let params = wide(std::ffi::OsStr::new(&format!(
            "-NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand {}",
            crate::drivers::encode_command(script)
        )));
        // SAFETY: the strings outlive the call; the process handle is kept
        // and closed when the Process is dropped.
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
            info.lpVerb = verb.as_ptr();
            info.lpFile = exe.as_ptr();
            info.lpParameters = params.as_ptr();
            info.nShow = 0; // SW_HIDE
            if ShellExecuteExW(&mut info) == 0 {
                if GetLastError() == ERROR_CANCELLED {
                    return Err(stopped_error(
                        "the driver switch couldn't start (the Windows permission prompt was declined)",
                    ));
                }
                return Err(std::io::Error::last_os_error().into());
            }
            if info.hProcess.is_null() {
                return Err(stopped_error(
                    "the driver switch couldn't start (Windows didn't start it)",
                ));
            }
            Ok(Process(info.hProcess))
        }
    }

    /// Its exit code, once it has ended.
    fn exit_code(&self) -> Option<i32> {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        // SAFETY: the handle stays open until drop.
        unsafe {
            if WaitForSingleObject(self.0, 0) != WAIT_OBJECT_0 {
                return None;
            }
            let mut code = 0u32;
            (GetExitCodeProcess(self.0, &mut code) != 0).then_some(code as i32)
        }
    }

    /// Waits up to `ms` for it to end; whether it has.
    fn wait(&self, ms: u32) -> bool {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::WaitForSingleObject;
        // SAFETY: the handle stays open until drop.
        unsafe { WaitForSingleObject(self.0, ms) == WAIT_OBJECT_0 }
    }
}

#[cfg(windows)]
impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: the handle is open and closed once, here.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// No elevated helper outside Windows.
#[cfg(not(windows))]
struct Process;

#[cfg(not(windows))]
impl Process {
    fn start(_script: &str) -> Result<Process> {
        Err(EngineError::new(
            Code::UnsupportedHost,
            "the driver switch only runs on Windows",
        )
        .into())
    }
    fn exit_code(&self) -> Option<i32> {
        None
    }
    fn wait(&self, _ms: u32) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_lines_are_read_only_when_whole() {
        assert_eq!(parse_status("ready end"), Some(Status::Ready));
        assert_eq!(parse_status("ready end\r\n"), Some(Status::Ready));
        assert_eq!(
            parse_status("switched 2 end"),
            Some(Status::Switched {
                n: 2,
                reboot: false
            })
        );
        assert_eq!(
            parse_status("switched 1 reboot end"),
            Some(Status::Switched { n: 1, reboot: true })
        );
        assert_eq!(
            parse_status("failed 3 e000020b end"),
            Some(Status::Failed {
                n: 3,
                code: NO_DEVICE
            })
        );
        // Caught while the helper was writing it: never a wrong answer.
        for t in [
            "",
            "ready",
            "swi",
            "switched 1",
            "switched 1 reboot",
            "switched 1 reb",
            "failed 3 e00",
            "failed 3 e000020b",
            "failed 3 e00 end",
            "switched x end",
            "switched 1 maybe end",
            "ready now end",
        ] {
            assert_eq!(parse_status(t), None, "{t:?}");
        }
        assert_eq!(
            failure_text(NO_DEVICE),
            "the device was no longer connected"
        );
        assert_eq!(failure_text(0x103), "Windows error 0x00000103");
        assert_eq!(exit_text(6), UNSAFE_ROOT);
        assert_eq!(exit_text(9), "exit code 9");
    }

    fn sample_script() -> (PathBuf, String) {
        let ipc = PathBuf::from(r"C:\Users\Ann O'Neil\AppData\Local\Temp\thimsy-watch-1");
        let s = script(&[(0x1782, 0x4d00)], "watch-abc", &ipc);
        (ipc, s)
    }

    #[test]
    fn the_helper_script_forces_winusb_and_only_reads_the_engine_folder() {
        let (ipc, s) = sample_script();
        assert!(s.starts_with(ADMIN_PRELUDE));
        assert!(s.contains("$ids = @('USB\\VID_1782&PID_4D00')"), "{s}");
        assert!(s.contains("$dir = Join-Path $root 'watch-abc'"));
        // Quoted so the name's own quote can't end the string.
        for (var, file) in [("trigger", "trigger"), ("stop", "stop")] {
            let lit = ps_literal(&ipc.join(file).display().to_string());
            assert!(lit.contains("Ann O''Neil"), "{lit}");
            assert!(s.contains(&format!("${var} = {lit}")), "{s}");
        }
        // FORCE (1) | NONINTERACTIVE (4), with the published WinUSB package
        // found under the system's Windows folder.
        assert!(s.contains(
            "UpdateDriverForPlugAndPlayDevicesW([IntPtr]::Zero, $id, $infs[$id], 5, [ref]$r)"
        ));
        assert!(s.contains("[Environment]::GetFolderPath('Windows')"));
        assert!(!s.contains("$env:"), "no environment variables: {s}");
        assert!(s.contains("Test-WinUsbInf (Get-Content"));
        // Its folder: in a ProgramData folder that was checked, made fresh,
        // admin-only but readable, and gone when the helper ends.
        let checked = s.find("\n  Assert-AdminRoot\n").expect("checks the root");
        let made = s
            .find("CreateDirectory($dir, (New-AdminSecurity $true))")
            .unwrap();
        assert!(checked < made);
        assert!(s.contains("if (Test-Path -LiteralPath $dir) { $code = 5;"));
        assert!(s.contains("Remove-Item -LiteralPath $dir -Recurse -Force"));
        // Never takes over or re-permissions something that already exists.
        assert!(!s.contains("takeown") && !s.contains("Set-Acl"));
        // Everything it writes goes to its own folder; the engine's folder
        // is only read, and a failed read is just "nothing yet".
        assert!(s.contains("WriteAllText($status, \"$text end\")"));
        assert_eq!(s.matches("WriteAllText(").count(), 1);
        let read = s.find("ReadAllText($trigger)").unwrap();
        let guarded = s[..read].rfind("try {").unwrap();
        assert!(s[guarded..read].lines().count() < 6);
        assert!(s.contains("AddMinutes(20)"));
        assert!(s.trim_end().ends_with("exit $code"));
    }

    #[test]
    fn requests_are_handed_on_and_status_changes_reported_once() {
        let dir = tempfile::tempdir().unwrap();
        let ipc = dir.path().join("ipc");
        std::fs::create_dir(&ipc).unwrap();
        let status = dir.path().join("status");
        let mut w = DriverWatch::new(ipc.clone(), status.clone(), None);
        // Nothing yet.
        assert_eq!(w.step().unwrap(), None);
        w.request();
        assert_eq!(std::fs::read_to_string(ipc.join("trigger")).unwrap(), "1");
        // The helper writes; half a status is ignored, the whole one is
        // reported once.
        std::fs::write(&status, "switched 1").unwrap();
        assert_eq!(w.step().unwrap(), None);
        std::fs::write(&status, "switched 1 end").unwrap();
        assert_eq!(
            w.step().unwrap(),
            Some(Status::Switched {
                n: 1,
                reboot: false
            })
        );
        assert_eq!(w.step().unwrap(), None);
        // A trigger that couldn't be written (here: the path is a folder)
        // is written by a later step.
        std::fs::remove_file(ipc.join("trigger")).unwrap();
        std::fs::create_dir(ipc.join("trigger")).unwrap();
        w.request();
        assert_eq!((w.requested, w.written), (2, 1));
        std::fs::remove_dir(ipc.join("trigger")).unwrap();
        w.step().unwrap();
        assert_eq!(std::fs::read_to_string(ipc.join("trigger")).unwrap(), "2");
        assert_eq!(w.written, 2);
        std::fs::write(&status, "failed 2 e000020b end").unwrap();
        assert_eq!(
            w.step().unwrap(),
            Some(Status::Failed {
                n: 2,
                code: NO_DEVICE
            })
        );
        // Dropped: asks the helper to stop; with it gone, the folder goes.
        drop(w);
        assert!(!ipc.exists());
    }

    /// The scripts run by Windows PowerShell, as the installer runs them.
    #[cfg(windows)]
    fn powershell(script: &str) -> String {
        let out = std::process::Command::new(crate::drivers::powershell_path().unwrap())
            .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
            .arg(crate::drivers::encode_command(script))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[cfg(windows)]
    #[test]
    fn windows_powershell_parses_both_elevated_scripts() {
        let (_, watch) = sample_script();
        let install = crate::drivers::driver_script(
            Path::new(r"C:\Thimsy\tools\wdi-simple.exe"),
            "abc",
            &[(0x1782, 0x4d00, "Unisoc download mode".into())],
        );
        for s in [watch, install] {
            let check = format!(
                "$e = $null; [void][System.Management.Automation.Language.Parser]::ParseInput({}, [ref]$null, [ref]$e); $e.Count",
                ps_literal(&s)
            );
            assert_eq!(powershell(&check), "0", "{s}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_powershell_agrees_on_what_a_winusb_package_is() {
        for (text, _) in crate::drivers::tests::INF_SAMPLES {
            let want = crate::drivers::inf_is_winusb_for(text, 0x1782, 0x4d00);
            let got = powershell(&format!(
                "{WINUSB_INF_RULE}Test-WinUsbInf {} 'VID_1782&PID_4D00'",
                ps_literal(text)
            ));
            assert_eq!(got, if want { "True" } else { "False" }, "{text}");
        }
    }
}
