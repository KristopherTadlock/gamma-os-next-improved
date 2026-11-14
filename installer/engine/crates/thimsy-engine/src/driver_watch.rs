//! Windows: switching a Unisoc device to WinUSB the moment it's connected.
//!
//! When Windows also has another driver package for the download-mode ID
//! (Unisoc's own "SPRD U2S Diag", which unlocking needs), it gives the
//! device that driver, which the flashing tool can't use. Only an install
//! made while the device is connected replaces it, and download mode lasts
//! a few seconds, too short to ask for permission then. So before the user
//! connects the device, one elevated helper starts (one UAC prompt) and
//! waits; when `spd_dump` reports the device on another driver, the engine
//! tells the helper, which forces the WinUSB package already in the driver
//! store onto it (`UpdateDriverForPlugAndPlayDevicesW` with
//! `INSTALLFLAG_FORCE`) and reports back. Unisoc's package stays installed.
//!
//! The engine talks to the helper through files: it writes `trigger` (a
//! count; each increase asks for one switch) and `stop` in a folder of the
//! job. The helper only reads those, as numbers or by their existence, and
//! writes its `status` in a folder it creates under ProgramData that only
//! administrators and SYSTEM can change (signed-in users may read it).

use crate::catalog::Device;
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

/// The helper's `status` text: `ready`, `switched N`, `switched N reboot`
/// or `failed N CODE` (CODE in hex). Anything else, such as a file caught
/// half-written, is `None`.
pub fn parse_status(text: &str) -> Option<Status> {
    let mut w = text.split_whitespace();
    match (w.next()?, w.next(), w.next(), w.next()) {
        ("ready", None, ..) => Some(Status::Ready),
        ("switched", Some(n), r, None) => Some(Status::Switched {
            n: n.parse().ok()?,
            reboot: match r {
                None => false,
                Some("reboot") => true,
                Some(_) => return None,
            },
        }),
        ("failed", Some(n), Some(c), None) => Some(Status::Failed {
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

/// Whether Windows has another driver package than WinUSB for one of the
/// device's download-mode IDs, which it would give the device instead.
pub fn needed(d: &Device) -> bool {
    d.identify
        .usb
        .iter()
        .any(|u| crate::drivers::other_installed(u.vid, u.pid))
}

/// The script the elevated PowerShell runs (see the module notes): `ids`
/// the download-mode IDs, `status_name` the name of its status folder under
/// `<ProgramData>\Thimsy Installer` (made fresh, or the script stops),
/// `ipc` the job folder with `trigger` and `stop`. Exit codes: 0 stopped or
/// timed out (20 minutes), 4 no WinUSB package for an ID, 5 the status
/// folder already existed, 1 anything else.
pub fn script(ids: &[(u16, u16)], status_name: &str, ipc: &Path) -> String {
    use crate::drivers::ps_literal;
    let ids: Vec<String> = ids
        .iter()
        .map(|(v, p)| ps_literal(&format!("USB\\VID_{v:04X}&PID_{p:04X}")))
        .collect();
    let mut s = String::from(
        r#"$ErrorActionPreference = 'Stop'
$code = 1
$admins = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-32-544')
$system = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-18')
$users = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-11')
function New-AdminSecurity([bool]$readable) {
  $sec = New-Object System.Security.AccessControl.DirectorySecurity
  $sec.SetAccessRuleProtection($true, $false)
  foreach ($sid in @($admins, $system)) {
    $sec.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
  }
  if ($readable) {
    $sec.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($users, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
  }
  $sec.SetOwner($admins)
  return $sec
}
$root = Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'Thimsy Installer'
"#,
    );
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
function Say([string]$text) { [System.IO.File]::WriteAllText($status, $text) }
try {
  if (Test-Path -LiteralPath $root) {
    & (Join-Path ([Environment]::SystemDirectory) 'takeown.exe') /F $root /A | Out-Null
    Set-Acl -LiteralPath $root -AclObject (New-AdminSecurity $false)
  } else {
    [void][System.IO.Directory]::CreateDirectory($root, (New-AdminSecurity $false))
  }
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
  # The WinUSB package for each ID, as Windows published it.
  $infs = @{}
  $published = Get-ChildItem -LiteralPath (Join-Path $env:SystemRoot 'INF') -Filter 'oem*.inf'
  foreach ($id in $ids) {
    $key = $id.Substring(4)
    $inf = $published | Where-Object { $t = (Get-Content -LiteralPath $_.FullName -Raw).ToUpperInvariant(); $t.Contains($key) -and $t.Contains('WINUSB') } | Select-Object -First 1
    if (-not $inf) { $code = 4; throw "no WinUSB package for $id" }
    $infs[$id] = $inf.FullName
  }
  Say 'ready'
  $done = 0
  $until = [DateTime]::UtcNow.AddMinutes(20)
  while ([DateTime]::UtcNow -lt $until -and -not (Test-Path -LiteralPath $stop)) {
    $n = 0
    if (Test-Path -LiteralPath $trigger) { [void][int]::TryParse(([System.IO.File]::ReadAllText($trigger)).Trim(), [ref]$n) }
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

/// The running helper. Dropping it stops the helper and removes the job's
/// folder for it.
pub struct DriverWatch {
    ipc: PathBuf,
    status: PathBuf,
    triggered: u32,
    last: String,
    #[cfg(windows)]
    process: windows_sys::Win32::Foundation::HANDLE,
}

impl DriverWatch {
    /// Starts the helper for the device's download-mode IDs (one UAC
    /// prompt) with its job folder at `ipc`, and waits until it's ready.
    pub fn start(d: &Device, ipc: &Path, rep: &Reporter) -> Result<DriverWatch> {
        let _ = std::fs::remove_dir_all(ipc);
        std::fs::create_dir_all(ipc)?;
        let ids: Vec<(u16, u16)> = d.identify.usb.iter().map(|u| (u.vid, u.pid)).collect();
        let name = status_name();
        let status = program_data()
            .join("Thimsy Installer")
            .join(&name)
            .join("status");
        rep.info("Starting the driver switch; Windows asks for permission");
        let watch = platform_start(&script(&ids, &name, ipc), ipc, status)?;
        watch.wait_ready()?;
        Ok(watch)
    }

    /// Asks the helper for one switch now.
    pub fn trigger(&mut self) -> Result<()> {
        self.triggered += 1;
        std::fs::write(self.ipc.join("trigger"), self.triggered.to_string())?;
        Ok(())
    }

    /// The helper's status, when it changed since the last call.
    pub fn poll(&mut self) -> Option<Status> {
        let text = std::fs::read_to_string(&self.status).ok()?;
        let status = parse_status(&text)?;
        if text == self.last {
            return None;
        }
        self.last = text;
        Some(status)
    }
}

/// A fresh name for the helper's status folder.
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

fn declined_error(why: &str) -> anyhow::Error {
    EngineError::new(
        Code::ToolFailed,
        format!("the driver switch couldn't start ({why}); nothing was written"),
    )
    .hint("Accept the Windows permission prompt, or install WinUSB with Zadig.")
    .into()
}

#[cfg(windows)]
fn platform_start(script: &str, ipc: &Path, status: PathBuf) -> Result<DriverWatch> {
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
    // SAFETY: the strings outlive the call; the process handle is kept and
    // closed when the watch is dropped.
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
                return Err(declined_error("the Windows permission prompt was declined"));
            }
            return Err(std::io::Error::last_os_error().into());
        }
        if info.hProcess.is_null() {
            return Err(declined_error("Windows didn't start it"));
        }
        Ok(DriverWatch {
            ipc: ipc.to_path_buf(),
            status,
            triggered: 0,
            last: String::new(),
            process: info.hProcess,
        })
    }
}

#[cfg(not(windows))]
fn platform_start(_script: &str, _ipc: &Path, _status: PathBuf) -> Result<DriverWatch> {
    Err(EngineError::new(
        Code::UnsupportedHost,
        "the driver switch only runs on Windows",
    )
    .into())
}

impl DriverWatch {
    /// Waits (up to 2 minutes) for the helper to say it's ready.
    fn wait_ready(&self) -> Result<()> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while std::time::Instant::now() < until {
            if std::fs::read_to_string(&self.status)
                .ok()
                .and_then(|t| parse_status(&t))
                .is_some()
            {
                return Ok(());
            }
            if let Some(code) = self.exit_code() {
                return Err(declined_error(&match code {
                    4 => "Windows has no WinUSB package for the device".to_string(),
                    5 => "its status folder already existed".to_string(),
                    c => format!("exit code {c}"),
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err(declined_error("it didn't get ready within 2 minutes"))
    }

    #[cfg(windows)]
    fn exit_code(&self) -> Option<i32> {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        // SAFETY: the handle stays open until drop.
        unsafe {
            if WaitForSingleObject(self.process, 0) != WAIT_OBJECT_0 {
                return None;
            }
            let mut code = 0u32;
            (GetExitCodeProcess(self.process, &mut code) != 0).then_some(code as i32)
        }
    }

    #[cfg(not(windows))]
    fn exit_code(&self) -> Option<i32> {
        Some(1)
    }
}

impl Drop for DriverWatch {
    fn drop(&mut self) {
        let _ = std::fs::write(self.ipc.join("stop"), "");
        #[cfg(windows)]
        // SAFETY: the handle is open and closed once, here.
        unsafe {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::WaitForSingleObject;
            WaitForSingleObject(self.process, 5000);
            CloseHandle(self.process);
        }
        let _ = std::fs::remove_dir_all(&self.ipc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_lines_are_read_strictly() {
        assert_eq!(parse_status("ready"), Some(Status::Ready));
        assert_eq!(parse_status("ready\r\n"), Some(Status::Ready));
        assert_eq!(
            parse_status("switched 2"),
            Some(Status::Switched {
                n: 2,
                reboot: false
            })
        );
        assert_eq!(
            parse_status("switched 1 reboot"),
            Some(Status::Switched { n: 1, reboot: true })
        );
        assert_eq!(
            parse_status("failed 3 e000020b"),
            Some(Status::Failed {
                n: 3,
                code: NO_DEVICE
            })
        );
        // Half-written or unknown: ignored until it's whole.
        for t in [
            "",
            "swi",
            "switched",
            "switched x",
            "switched 1 maybe",
            "failed 1",
            "ready now",
        ] {
            assert_eq!(parse_status(t), None, "{t:?}");
        }
        assert_eq!(
            failure_text(NO_DEVICE),
            "the device was no longer connected"
        );
        assert_eq!(failure_text(0x103), "Windows error 0x00000103");
    }

    #[test]
    fn the_helper_script_forces_winusb_and_only_reads_the_job_folder() {
        let ipc = Path::new(r"C:\Users\Ann O'Neil\ws\job\driver-watch");
        let s = script(&[(0x1782, 0x4d00)], "watch-abc", ipc);
        assert!(s.contains("$ids = @('USB\\VID_1782&PID_4D00')"), "{s}");
        assert!(s.contains("$dir = Join-Path $root 'watch-abc'"));
        // Quoted so the name's own quote can't end the string.
        for (var, file) in [("trigger", "trigger"), ("stop", "stop")] {
            let lit = crate::drivers::ps_literal(&ipc.join(file).display().to_string());
            assert!(lit.contains("Ann O''Neil"), "{lit}");
            assert!(s.contains(&format!("${var} = {lit}")), "{s}");
        }
        // FORCE (1) | NONINTERACTIVE (4), with the published WinUSB package.
        assert!(s.contains(
            "UpdateDriverForPlugAndPlayDevicesW([IntPtr]::Zero, $id, $infs[$id], 5, [ref]$r)"
        ));
        assert!(s.contains("$t.Contains($key) -and $t.Contains('WINUSB')"));
        // The status folder: fresh, admin-only, readable by signed-in users,
        // and gone when the helper ends.
        assert!(s.contains("if (Test-Path -LiteralPath $dir) { $code = 5;"));
        assert!(s.contains("'S-1-5-11'"));
        assert!(s.contains("CreateDirectory($dir, (New-AdminSecurity $true))"));
        assert!(s.contains("CreateDirectory($root, (New-AdminSecurity $false))"));
        assert!(s.contains("Remove-Item -LiteralPath $dir -Recurse -Force"));
        // Everything it writes goes to its own folder; the job folder is
        // only read.
        assert!(s.contains("WriteAllText($status, $text)"));
        assert_eq!(s.matches("WriteAllText(").count(), 1);
        assert!(s.contains("ReadAllText($trigger)"));
        assert!(s.contains("AddMinutes(20)"));
        assert!(s.trim_end().ends_with("exit $code"));
    }
}
