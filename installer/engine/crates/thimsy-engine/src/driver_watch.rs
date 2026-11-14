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
        // The helper's -1: it failed, but Windows gave no reason.
        0xFFFF_FFFF => "Windows gave no reason".to_string(),
        c => format!("Windows error 0x{c:08x}"),
    }
}

/// Why the helper stopped, from its exit code.
pub fn exit_text(code: i32) -> String {
    match code {
        4 => "Windows has no WinUSB package for the device".to_string(),
        5 => "its status folder already existed".to_string(),
        6 => UNSAFE_ROOT.to_string(),
        7 => "it can't see the installer's temporary folder".to_string(),
        c => format!("exit code {c}"),
    }
}

/// Whether Windows may give the device another driver than WinUSB (another
/// package for one of its download-mode IDs, `other_package`, as the
/// driver check found, or another driver it keeps for one on a USB port),
/// so the helper must be ready before it's connected.
pub fn needed(d: &Device, other_package: bool) -> bool {
    let ids: Vec<(u16, u16)> = d.identify.usb.iter().map(|u| (u.vid, u.pid)).collect();
    other_package || crate::drivers::other_driver_on_ports(&ids)
}

/// PowerShell's `Test-WinUsbInf $text $key`: the same test as
/// [`crate::drivers::inf_is_winusb_for`] (`$key` like `VID_1782&PID_4D00`).
pub const WINUSB_INF_RULE: &str = r#"function Test-WinUsbInf([string]$text, [string]$key) {
  $t = ($text -replace '\s', '').ToUpperInvariant()
  $services = [regex]::Matches($t, 'ADDSERVICE=').Count
  return $t.Contains($key.ToUpperInvariant()) -and $t.Contains('INCLUDE=WINUSB.INF') -and $services -gt 0 -and [regex]::Matches($t, 'ADDSERVICE=WINUSB,').Count -eq $services
}
"#;

/// The C# the helper compiles (`Add-Type`): `[ThimsyNewDev]::StoreInf`
/// and `[ThimsyNewDev]::Switch`.
pub const NEWDEV_CS: &str = r#"using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ThimsyNewDev {
    [DllImport("setupapi.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetupGetInfDriverStoreLocationW(string fileName, IntPtr platform, string locale, StringBuilder buffer, int size, out int needed);
    // The driver store's copy of a published package's .inf (next to its
    // catalog and files, where an install should come from), or null.
    public static string StoreInf(string published) {
        var b = new StringBuilder(1024);
        int needed;
        return SetupGetInfDriverStoreLocationW(published, IntPtr.Zero, null, b, b.Capacity, out needed) ? b.ToString() : null;
    }
    [DllImport("newdev.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool UpdateDriverForPlugAndPlayDevicesW(IntPtr parent, string hardwareId, string infPath, uint flags, out bool reboot);
    // FORCE | NONINTERACTIVE: replace the device's driver, and fail rather
    // than wait on a dialog nobody sees. 0, or the Windows error read right
    // after the call (PowerShell may change it between statements); -1 for
    // a failure Windows gave no error for.
    public static int Switch(string hardwareId, string infPath, out bool reboot) {
        if (UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId, infPath, 5, out reboot)) return 0;
        int e = Marshal.GetLastWin32Error();
        return e != 0 ? e : -1;
    }
}"#;

/// The script the elevated PowerShell runs (see the module notes): `ids`
/// the download-mode IDs, `status_name` the name of its status folder under
/// `<ProgramData>\Thimsy Installer` (made fresh, or the script stops),
/// `ipc` the absolute folder with `trigger` and `stop`, `engine` the
/// engine's process ID (the helper stops when it ends). Exit codes: 0
/// stopped, timed out (20 minutes) or the engine ended, 4 no WinUSB package
/// for an ID, 5 the status folder already existed, 6 a folder in
/// ProgramData isn't safe to use, 7 it can't see `ipc`, 1 anything else.
pub fn script(ids: &[(u16, u16)], status_name: &str, ipc: &Path, engine: u32) -> String {
    let ids: Vec<String> = ids
        .iter()
        .map(|(v, p)| ps_literal(&format!("USB\\VID_{v:04X}&PID_{p:04X}")))
        .collect();
    let mut s = String::from(ADMIN_PRELUDE);
    s.push_str(WINUSB_INF_RULE);
    s.push_str(&format!(
        "$dir = Join-Path $root {}\n$ipc = {}\n$trigger = {}\n$stop = {}\n$ids = @({})\n$enginePid = {engine}\n",
        ps_literal(status_name),
        ps_literal(&ipc.display().to_string()),
        ps_literal(&ipc.join("trigger").display().to_string()),
        ps_literal(&ipc.join("stop").display().to_string()),
        ids.join(", ")
    ));
    s.push_str(
        r#"$status = Join-Path $dir 'status'
$made = $false
# Retried for a while: something else (an antivirus) may hold the file.
function Say([string]$text) {
  for ($i = 0; ; $i++) {
    try { [System.IO.File]::WriteAllText($status, "$text end"); return }
    catch { if ($i -ge 50) { throw } ; Start-Sleep -Milliseconds 100 }
  }
}
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
  # Held from the start, so it's the engine that ends, not a later process
  # with its ID.
  $engine = [System.Diagnostics.Process]::GetProcessById($enginePid)
  if (-not (Test-Path -LiteralPath $ipc -PathType Container)) { $code = 7; throw "can't see $ipc" }
  Assert-AdminRoot
  New-AdminFolder $dir $true
  $made = $true
  Add-Type -TypeDefinition @'
"#,
    );
    s.push_str(NEWDEV_CS);
    s.push_str(
        r#"
'@
  # The WinUSB package for each ID among the published ones (in the
  # Windows folder from the system, not the environment; each read once),
  # installed from its copy in the driver store.
  $published = @(Get-ChildItem -LiteralPath (Join-Path ([Environment]::GetFolderPath('Windows')) 'INF') -Filter 'oem*.inf' | ForEach-Object {
    try { [pscustomobject]@{ Path = $_.FullName; Text = (Get-Content -LiteralPath $_.FullName -Raw) } } catch { }
  })
  $infs = @{}
  foreach ($id in $ids) {
    $key = $id.Substring(4)
    $inf = $published | Where-Object { Test-WinUsbInf $_.Text $key } | Select-Object -First 1
    $stored = if ($inf) { [ThimsyNewDev]::StoreInf($inf.Path) } else { $null }
    if (-not $stored) { $code = 4; throw "no WinUSB package for $id" }
    $infs[$id] = $stored
  }
  Say 'ready'
  $done = 0
  $until = [DateTime]::UtcNow.AddMinutes(20)
  # ERROR_NO_SUCH_DEVINST (0xE000020B): that ID isn't connected.
  $noDevice = -536870389
  while ([DateTime]::UtcNow -lt $until -and -not $engine.HasExited -and -not (Test-Path -LiteralPath $stop)) {
    $n = Read-Trigger
    if ($n -gt $done) {
      $done = $n
      $switched = $false; $reboot = $false; $err = 0
      foreach ($id in $ids) {
        $r = $false
        $e = [ThimsyNewDev]::Switch($id, $infs[$id], [ref]$r)
        if ($e -eq 0) {
          $switched = $true
          if ($r) { $reboot = $true }
        } elseif ($err -eq 0 -or $err -eq $noDevice) {
          # A real failure says more than an ID that isn't connected.
          $err = $e
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
    /// Switches asked for, how many of those `trigger` holds, and the last
    /// one the helper answered.
    requested: u32,
    written: u32,
    answered: u32,
    last: String,
    process: Option<Process>,
    /// `stop` was written.
    stopping: bool,
}

impl DriverWatch {
    /// Starts the helper for the device's download-mode IDs (one UAC
    /// prompt) and waits until it's ready.
    pub fn start(d: &Device, rep: &Reporter) -> Result<DriverWatch> {
        // Unrelated random names: the temp folder's (which others may see)
        // gives nothing away about the status folder's.
        let name = format!("watch-{}", random_hex());
        let ipc =
            std::path::absolute(std::env::temp_dir())?.join(format!("thimsy-{}", random_hex()));
        std::fs::create_dir(&ipc)?;
        let ids: Vec<(u16, u16)> = d.identify.usb.iter().map(|u| (u.vid, u.pid)).collect();
        let status = program_data()
            .join("Thimsy Installer")
            .join(&name)
            .join("status");
        rep.info("Starting the driver switch; Windows asks for permission");
        let process = match Process::start(&script(&ids, &name, &ipc, std::process::id())) {
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
            answered: 0,
            last: String::new(),
            process,
            stopping: false,
        }
    }

    /// Asks the helper for one switch (written now, or by the next
    /// [`DriverWatch::step`] if the helper was reading the file), unless
    /// one it hasn't answered yet is pending. Whether it asked.
    pub fn request(&mut self) -> bool {
        if self.pending() {
            return false;
        }
        self.requested += 1;
        self.write_trigger();
        true
    }

    /// Asks the helper to stop (once spd_dump has the device), without
    /// waiting for it; dropping the watch waits.
    pub fn stop(&mut self) {
        if !self.stopping {
            self.stopping = std::fs::write(self.ipc.join("stop"), "").is_ok();
        }
    }

    /// Whether a switch was asked for that the helper hasn't answered.
    pub fn pending(&self) -> bool {
        self.answered < self.requested
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
        if let Status::Switched { n, .. } | Status::Failed { n, .. } = status {
            self.answered = self.answered.max(n);
        }
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
        Err(unanswered_error(
            "the driver switch couldn't start (it didn't get ready within 2 minutes)",
        ))
    }
}

impl Drop for DriverWatch {
    fn drop(&mut self) {
        self.stop();
        // A switch in progress may take a while; the helper stops after it
        // (`stop` stays until then).
        let ended = self.process.as_ref().is_none_or(|p| p.wait(5000));
        if ended {
            let _ = std::fs::remove_dir_all(&self.ipc);
        }
    }
}

/// 128 random bits in hex (from the randomly keyed hasher std seeds from
/// the system's random source).
fn random_hex() -> String {
    use std::hash::{BuildHasher, Hasher};
    let part = || {
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    };
    format!("{:016x}{:016x}", part(), part())
}

/// ProgramData as the system knows it (what the helper's
/// `GetFolderPath('CommonApplicationData')` gives), not as the environment
/// says.
#[cfg(windows)]
fn program_data() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath};
    let mut p: windows_sys::core::PWSTR = std::ptr::null_mut();
    // SAFETY: Windows allocates the string, freed here.
    unsafe {
        let found =
            SHGetKnownFolderPath(&FOLDERID_ProgramData, 0, std::ptr::null_mut(), &mut p) == 0;
        let path = found.then(|| {
            let n = (0..).take_while(|&i| *p.add(i) != 0).count();
            PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
                p, n,
            )))
        });
        CoTaskMemFree(p.cast());
        path.unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
    }
}

#[cfg(not(windows))]
fn program_data() -> PathBuf {
    PathBuf::from(r"C:\ProgramData")
}

/// How the engine follows the helper's switches, as time goes by.
#[derive(Debug, Default)]
pub struct SwitchFollow {
    /// The last switch that worked, until the device answers or it's time
    /// to ask for it again.
    switched_at: Option<std::time::Instant>,
    /// Until when the device's port may look busy because its driver is
    /// being switched.
    busy_until: Option<std::time::Instant>,
    /// When the helper last answered.
    answered_at: Option<std::time::Instant>,
    /// spd_dump reported another driver while a switch was pending.
    deferred: bool,
}

/// What the engine reports, from [`SwitchFollow::next`].
#[derive(Debug, PartialEq)]
pub enum FollowEvent {
    /// The device now has WinUSB.
    Switched { reboot: bool },
    /// It left download mode before the switch: ask for it again.
    LeftFirst,
    /// The switch failed with this Windows error.
    Failed(u32),
    /// Switched a while ago, but the device hasn't answered: download mode
    /// ended, so it must be connected again.
    Reconnect,
}

/// How long after a switch the device may still answer by itself.
pub const RECONNECT_AFTER: std::time::Duration = std::time::Duration::from_secs(8);
/// How long the port may look busy around a switch.
pub const SWITCH_BUSY: std::time::Duration = std::time::Duration::from_secs(15);
/// How long after an answer a deferred report is looked at again: spd_dump
/// says within a second when what Windows has changed (and with it, the
/// driver it reports).
pub const RETRY_AFTER: std::time::Duration = std::time::Duration::from_millis(1500);

impl SwitchFollow {
    /// A switch was asked for at `now`.
    pub fn requested(&mut self, now: std::time::Instant) {
        self.busy_until = Some(now + SWITCH_BUSY);
        // A new switch: the last one's reconnect no longer applies.
        self.switched_at = None;
    }

    /// spd_dump reported another driver while a switch was pending.
    pub fn deferred(&mut self) {
        self.deferred = true;
    }

    /// Whether to ask for a switch now for a deferred report: once nothing
    /// is pending and spd_dump had time to say what Windows has since the
    /// answer, if the device still has another driver (`other_now`).
    pub fn retry(&mut self, pending: bool, other_now: bool, now: std::time::Instant) -> bool {
        let settled = self
            .answered_at
            .is_some_and(|t| now.duration_since(t) >= RETRY_AFTER);
        if !self.deferred || pending || !settled {
            return false;
        }
        self.deferred = false;
        other_now
    }

    /// Whether a switch is under way at `now` (its port may look busy).
    pub fn switching(&self, now: std::time::Instant) -> bool {
        self.busy_until.is_some_and(|t| now < t)
    }

    /// What to report at `now`, given the helper's new status (if any).
    pub fn next(&mut self, status: Option<Status>, now: std::time::Instant) -> Vec<FollowEvent> {
        let mut out = Vec::new();
        if matches!(
            status,
            Some(Status::Switched { .. } | Status::Failed { .. })
        ) {
            self.answered_at = Some(now);
        }
        match status {
            Some(Status::Switched { reboot, .. }) => {
                self.switched_at = Some(now);
                self.busy_until = Some(now + SWITCH_BUSY);
                out.push(FollowEvent::Switched { reboot });
            }
            Some(Status::Failed {
                code: NO_DEVICE, ..
            }) => {
                self.switched_at = None;
                self.busy_until = None;
                out.push(FollowEvent::LeftFirst);
            }
            Some(Status::Failed { code, .. }) => out.push(FollowEvent::Failed(code)),
            Some(Status::Ready) | None => {}
        }
        if self
            .switched_at
            .is_some_and(|t| now.duration_since(t) >= RECONNECT_AFTER)
        {
            self.switched_at = None;
            out.push(FollowEvent::Reconnect);
        }
        out
    }
}

fn stopped_error(message: &str) -> anyhow::Error {
    EngineError::new(Code::ToolFailed, format!("{message}; nothing was written")).into()
}

/// [`stopped_error`] for a permission prompt that was declined or left
/// unanswered.
fn unanswered_error(message: &str) -> anyhow::Error {
    EngineError::new(Code::ToolFailed, format!("{message}; nothing was written"))
        .hint("Accept the Windows permission prompt, then try again.")
        .into()
}

/// An elevated Windows PowerShell (this helper, or the WinUSB install).
#[cfg(windows)]
pub(crate) struct Process(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Process {
    /// The helper: like [`Process::launch`], declining is an error.
    fn start(script: &str) -> Result<Process> {
        Process::launch(script)?.ok_or_else(|| {
            unanswered_error(
                "the driver switch couldn't start (the Windows permission prompt was declined)",
            )
        })
    }

    /// Runs `script` in a hidden, elevated Windows PowerShell (one UAC
    /// prompt) without waiting for it; `None` when the prompt was
    /// declined. It inherits none of the engine's handles.
    pub(crate) fn launch(script: &str) -> Result<Option<Process>> {
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
                    return Ok(None);
                }
                return Err(std::io::Error::last_os_error().into());
            }
            if info.hProcess.is_null() {
                return Err(stopped_error(
                    "the elevated PowerShell couldn't start (Windows didn't start it)",
                ));
            }
            Ok(Some(Process(info.hProcess)))
        }
    }

    /// Its exit code, once it has ended.
    pub(crate) fn exit_code(&self) -> Option<i32> {
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

    /// Waits up to `ms` (`u32::MAX`: as long as it takes) for it to end;
    /// whether it has.
    pub(crate) fn wait(&self, ms: u32) -> bool {
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

/// No elevated helper outside Windows (tests make one that has ended with
/// `exited`).
#[cfg(not(windows))]
struct Process {
    exited: Option<i32>,
}

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
        self.exited
    }
    fn wait(&self, _ms: u32) -> bool {
        self.exited.is_some()
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
        assert_eq!(failure_text(0xFFFF_FFFF), "Windows gave no reason");
        assert_eq!(failure_text(0x0000_0103), "Windows error 0x00000103");
        assert_eq!(failure_text(0x103), "Windows error 0x00000103");
        assert_eq!(exit_text(6), UNSAFE_ROOT);
        assert_eq!(exit_text(9), "exit code 9");
    }

    fn sample_script() -> (PathBuf, String) {
        let ipc = PathBuf::from(r"C:\Users\Ann O'Neil\AppData\Local\Temp\thimsy-watch-1");
        let s = script(&[(0x1782, 0x4d00)], "watch-abc", &ipc, 4242);
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
        // found under the system's Windows folder, and the error read in
        // the same call.
        assert!(s.contains(
            "if (UpdateDriverForPlugAndPlayDevicesW(IntPtr.Zero, hardwareId, infPath, 5, out reboot)) return 0;\n        int e = Marshal.GetLastWin32Error();\n        return e != 0 ? e : -1;"
        ));
        assert!(s.contains("$e = [ThimsyNewDev]::Switch($id, $infs[$id], [ref]$r)"));
        assert!(
            !s.contains("GetLastWin32Error()\n      }"),
            "not read later"
        );
        assert!(s.contains("elseif ($err -eq 0 -or $err -eq $noDevice)"));
        assert!(s.contains("$noDevice = -536870389"));
        assert_eq!(0xE000_020Bu32 as i32, -536870389);
        assert!(s.contains("[Environment]::GetFolderPath('Windows')"));
        assert!(!s.contains("$env:"), "no environment variables: {s}");
        assert!(s.contains("Test-WinUsbInf $_.Text $key"));
        // Installed from the driver store's copy, next to its catalog.
        assert!(s.contains("[ThimsyNewDev]::StoreInf($inf.Path)"));
        assert!(s.contains(NEWDEV_CS));
        // It ends with the engine (held from the start), and needs to see
        // the engine's folder.
        assert!(s.contains("$enginePid = 4242\n"));
        assert!(s.contains("[System.Diagnostics.Process]::GetProcessById($enginePid)"));
        assert!(s.contains("-not $engine.HasExited"));
        assert!(s.contains("Test-Path -LiteralPath $ipc -PathType Container)) { $code = 7;"));
        // Its folder: in a ProgramData folder that was checked, made fresh
        // and checked, admin-only but readable, and removed only if made.
        let checked = s.find("\n  Assert-AdminRoot\n").expect("checks the root");
        let made = s
            .find("New-AdminFolder $dir $true\n  $made = $true")
            .unwrap();
        assert!(checked < made);
        assert!(s.contains("if ($made) { Remove-Item -LiteralPath $dir -Recurse -Force"));
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
        // One at a time: nothing more until the helper answers.
        assert!(w.pending());
        assert!(!w.request());
        assert_eq!(w.requested, 1);
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
        assert!(!w.pending());
        // Dropped: asks the helper to stop; with it gone, the folder goes.
        drop(w);
        assert!(!ipc.exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn a_helper_that_stopped_is_an_error_after_its_last_word() {
        let dir = tempfile::tempdir().unwrap();
        let ipc = dir.path().join("ipc");
        std::fs::create_dir(&ipc).unwrap();
        let status = dir.path().join("status");
        std::fs::write(&status, "switched 1 end").unwrap();
        let mut w = DriverWatch::new(ipc.clone(), status, Some(Process { exited: Some(4) }));
        // What it said before it stopped comes first...
        assert_eq!(
            w.step().unwrap(),
            Some(Status::Switched {
                n: 1,
                reboot: false
            })
        );
        // ...then that it stopped, and why.
        let e = w.step().unwrap_err();
        assert!(
            e.to_string().contains(
                "the driver switch stopped (Windows has no WinUSB package for the device)"
            ),
            "{e}"
        );
        assert_eq!(crate::error::code_of(&e).0, Code::ToolFailed);
        drop(w);
        assert!(!ipc.exists());
        // One still running keeps its folder (and `stop`) when dropped.
        let ipc2 = dir.path().join("ipc2");
        std::fs::create_dir(&ipc2).unwrap();
        drop(DriverWatch::new(
            ipc2.clone(),
            dir.path().join("none"),
            Some(Process { exited: None }),
        ));
        assert!(ipc2.join("stop").exists());
    }

    #[test]
    fn switches_are_followed_over_time() {
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_millis(s);
        let mut f = SwitchFollow::default();
        assert!(!f.switching(t0));
        assert_eq!(f.next(None, t0), vec![]);
        // Asked: the port may look busy for a while.
        f.requested(t0);
        assert!(f.switching(at(14_000)));
        assert!(!f.switching(at(15_000)));
        // Switched; if the device answers within 8 s nothing more is said
        // (the engine stops following once it's connected)...
        assert_eq!(
            f.next(Some(Status::Switched { n: 1, reboot: true }), at(1_000)),
            vec![FollowEvent::Switched { reboot: true }]
        );
        assert!(f.switching(at(15_000)));
        assert_eq!(f.next(None, at(8_999)), vec![]);
        // ...otherwise it must be connected again, once.
        assert_eq!(f.next(None, at(9_000)), vec![FollowEvent::Reconnect]);
        assert_eq!(f.next(None, at(9_500)), vec![]);
        // Gone before the switch: ask for it again, no reconnect later.
        f.requested(at(10_000));
        assert_eq!(
            f.next(
                Some(Status::Switched {
                    n: 2,
                    reboot: false
                }),
                at(10_500)
            ),
            vec![FollowEvent::Switched { reboot: false }]
        );
        assert_eq!(
            f.next(
                Some(Status::Failed {
                    n: 3,
                    code: NO_DEVICE
                }),
                at(11_000)
            ),
            vec![FollowEvent::LeftFirst]
        );
        assert!(!f.switching(at(11_000)));
        assert_eq!(f.next(None, at(20_000)), vec![]);
        // Any other failure is passed on.
        assert_eq!(
            f.next(Some(Status::Failed { n: 4, code: 5 }), at(21_000)),
            vec![FollowEvent::Failed(5)]
        );
        assert_eq!(f.next(Some(Status::Ready), at(22_000)), vec![]);
    }

    #[test]
    fn a_report_during_a_switch_is_looked_at_once_it_is_answered() {
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        let at = |s: u64| t0 + Duration::from_millis(s);
        let mut f = SwitchFollow::default();
        // Nothing deferred: never.
        assert!(!f.retry(false, true, at(0)));
        f.requested(at(0));
        f.deferred();
        // Still pending, or answered only just now: not yet.
        assert!(!f.retry(true, true, at(500)));
        f.next(
            Some(Status::Switched {
                n: 1,
                reboot: false,
            }),
            at(1_000),
        );
        assert!(!f.retry(false, true, at(2_000)));
        // Settled, and spd_dump still reports another driver: once.
        assert!(f.retry(false, true, at(2_500)));
        assert!(!f.retry(false, true, at(3_000)));
        // Settled with WinUSB now (the report was from before the switch):
        // dropped.
        f.deferred();
        f.next(
            Some(Status::Failed {
                n: 2,
                code: NO_DEVICE,
            }),
            at(4_000),
        );
        assert!(!f.retry(false, false, at(6_000)));
        assert!(!f.retry(false, true, at(7_000)));
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
    fn windows_compiles_the_helpers_csharp_and_its_calls_work() {
        // A published package (any: the runner has some) maps to its copy
        // in the driver store, and a switch for an ID no device has fails
        // with a Windows error read in the same call.
        let out = powershell(&format!(
            "Add-Type -TypeDefinition @'\n{NEWDEV_CS}\n'@\n\
             $f = Get-ChildItem -LiteralPath (Join-Path ([Environment]::GetFolderPath('Windows')) 'INF') -Filter 'oem*.inf' | Select-Object -First 1\n\
             if (-not $f) {{ 'none'; exit 0 }}\n\
             $s = [ThimsyNewDev]::StoreInf($f.FullName)\n\
             $r = $false\n\
             $e = [ThimsyNewDev]::Switch('USB\\VID_FFFF&PID_FFFE', $s, [ref]$r)\n\
             \"$s|$e\""
        ));
        if out == "none" {
            return;
        }
        let (stored, err) = out.rsplit_once('|').expect("path|error");
        assert!(stored.to_ascii_lowercase().contains("driverstore"), "{out}");
        assert_ne!(err, "0", "{out}");
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
