//! USB access for the flashing tools. Windows needs WinUSB on the device's
//! flashing-mode IDs, installed with the bundled `wdi-simple` (libwdi)
//! behind one UAC prompt. Linux needs a udev rule that lets the logged-in
//! user open the device, written once through `pkexec`. macOS needs
//! nothing.

use crate::catalog::Device;
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use crate::i18n::tr;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct DriverStatus {
    pub status: &'static str, // ok | missing | not_required
    pub detail: String,
}

/// Where udev reads rules from (Linux).
pub const UDEV_RULE_DIRS: &[&str] = &[
    "/etc/udev/rules.d",
    "/usr/lib/udev/rules.d",
    "/lib/udev/rules.d",
];

/// The rule file `drivers --install` writes on Linux.
pub const UDEV_RULE_FILE: &str = "/etc/udev/rules.d/70-thimsy-installer.rules";

pub fn status(d: &Device, host: Host) -> DriverStatus {
    match host {
        Host::Windows => windows_status(d),
        Host::Linux => linux_status(d),
        _ => DriverStatus {
            status: "not_required",
            detail: tr("{host} needs no driver").replace("{host}", host.display()),
        },
    }
}

fn windows_status(d: &Device) -> DriverStatus {
    if !d.identify.usb.is_empty()
        && d.identify
            .usb
            .iter()
            .all(|u| winusb_installed(u.vid, u.pid))
    {
        return DriverStatus {
            status: "ok",
            detail: tr("WinUSB is installed for {ids}").replace("{ids}", &ids(d)),
        };
    }
    match crate::tools::locate("wdi-simple") {
        Ok(_) => DriverStatus {
            status: "missing",
            detail: tr("WinUSB will be installed for {ids} the first time you flash")
                .replace("{ids}", &ids(d)),
        },
        Err(_) => DriverStatus {
            status: "missing",
            detail: tr("The driver installer isn't bundled with this build. Install WinUSB for the device's download mode with Zadig."),
        },
    }
}

fn linux_status(d: &Device) -> DriverStatus {
    if is_root() {
        return DriverStatus {
            status: "ok",
            detail: tr("The installer runs as root, so it can open the device"),
        };
    }
    let dirs: Vec<PathBuf> = UDEV_RULE_DIRS.iter().map(PathBuf::from).collect();
    if rules_cover(&dirs, d) {
        return DriverStatus {
            status: "ok",
            detail: tr("USB access is set up for {ids}").replace("{ids}", &ids(d)),
        };
    }
    DriverStatus {
        status: "missing",
        detail: tr(
            "USB access for {ids} isn't set up yet. Setting it up asks for your password once.",
        )
        .replace("{ids}", &ids(d)),
    }
}

#[cfg(unix)]
fn is_root() -> bool {
    // SAFETY: geteuid can't fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    false
}

/// Whether the udev rules in `dirs` mention every USB ID of the device on
/// one (uncommented) line, case-insensitively: a rule matching it.
pub fn rules_cover(dirs: &[PathBuf], d: &Device) -> bool {
    if d.identify.usb.is_empty() {
        return false;
    }
    let mut lines = Vec::new();
    for dir in dirs {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rules") {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    lines.extend(
                        text.lines()
                            .map(|l| l.trim().to_ascii_lowercase())
                            .filter(|l| !l.starts_with('#')),
                    );
                }
            }
        }
    }
    d.identify.usb.iter().all(|u| {
        let (v, p) = (format!("{:04x}", u.vid), format!("{:04x}", u.pid));
        lines.iter().any(|l| l.contains(&v) && l.contains(&p))
    })
}

/// The udev rules giving the logged-in user access to the device's USB
/// IDs (one line per ID).
pub fn udev_rules(d: &Device) -> String {
    let mut out =
        String::from("# Written by Thimsy Installer: lets the logged-in user flash the device.\n");
    for u in &d.identify.usb {
        out.push_str(&format!(
            "SUBSYSTEM==\"usb\", ATTR{{idVendor}}==\"{:04x}\", ATTR{{idProduct}}==\"{:04x}\", TAG+=\"uaccess\"\n",
            u.vid, u.pid
        ));
    }
    out
}

/// Text of an .inf file: UTF-16 (with a byte-order mark) or 8-bit.
pub fn decode_inf(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Whether an .inf installs WinUSB for VID:PID (libwdi's, Zadig's or any
/// other WinUSB package; not Unisoc's own driver, which libusb can't use).
pub fn inf_is_winusb_for(text: &str, vid: u16, pid: u16) -> bool {
    let t = text.to_ascii_uppercase();
    t.contains(&format!("VID_{vid:04X}&PID_{pid:04X}")) && t.contains("WINUSB")
}

/// Looks through Windows' driver store for a WinUSB package covering the ID.
/// Readable without admin rights; the package's file name doesn't matter, so
/// earlier installs (any name, or Zadig) count too.
#[cfg(windows)]
fn winusb_installed(vid: u16, pid: u16) -> bool {
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("System32\\DriverStore\\FileRepository");
    let Ok(dirs) = std::fs::read_dir(&root) else {
        return false;
    };
    dirs.flatten().any(|dir| {
        let Ok(files) = std::fs::read_dir(dir.path()) else {
            return false;
        };
        files.flatten().any(|f| {
            let p = f.path();
            p.extension().is_some_and(|x| x.eq_ignore_ascii_case("inf"))
                && std::fs::read(&p).is_ok_and(|b| inf_is_winusb_for(&decode_inf(&b), vid, pid))
        })
    })
}

#[cfg(not(windows))]
fn winusb_installed(vid: u16, pid: u16) -> bool {
    test_winusb(vid, pid)
}

fn ids(d: &Device) -> String {
    d.identify
        .usb
        .iter()
        .map(|u| format!("{:04x}:{:04x}", u.vid, u.pid))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `drivers --install`: Windows installs WinUSB (one UAC prompt), Linux
/// writes the udev rule (one password prompt); other hosts need nothing.
pub fn install_for(d: &Device, host: Host, rep: &Reporter) -> Result<()> {
    match host {
        Host::Windows => install(d, rep),
        Host::Linux => linux_install(d, rep),
        _ => {
            rep.info("No driver needed on this computer");
            Ok(())
        }
    }
}

/// The flash's driver step: Windows installs WinUSB when it's missing;
/// Linux only warns when no udev rule covers the device (the app offers
/// `drivers --install`; asking for a password in the middle of a flash
/// would be a surprise); other hosts need nothing.
pub fn ensure_for_flash(d: &Device, host: Host, rep: &Reporter) -> Result<()> {
    match host {
        Host::Windows => install(d, rep),
        Host::Linux => {
            if linux_status(d).status != "ok" {
                rep.warn(format!(
                    "No udev rule gives this user access to {}; if the device isn't found, set up USB access and try again",
                    ids(d)
                ));
            }
            Ok(())
        }
        _ => {
            rep.info("No driver needed on this computer");
            Ok(())
        }
    }
}

/// A PowerShell single-quoted string literal. Every character PowerShell
/// treats as a single quote (`'` and U+2018 to U+201B) is doubled, so
/// nothing in `s` can end the literal.
pub fn ps_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// The script the elevated PowerShell runs: copies `wdi-simple.exe` from
/// `tool` into a new folder only administrators and SYSTEM can change
/// (`<ProgramData>\Thimsy Installer\driver-<random GUID>`), checks the copy
/// is the file the engine found (`sha256`), runs it from there for each
/// USB ID with `--dest` inside that folder, removes the folder and exits
/// with the first failing exit code (0 when every ID was installed).
pub fn driver_script(tool: &Path, sha256: &str, usb: &[(u16, u16, String)]) -> String {
    let mut s = String::from(
        r#"$ErrorActionPreference = 'Stop'
$code = 1
$admins = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-32-544')
$system = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-18')
function New-AdminSecurity {
  $sec = New-Object System.Security.AccessControl.DirectorySecurity
  $sec.SetAccessRuleProtection($true, $false)
  foreach ($sid in @($admins, $system)) {
    $sec.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
  }
  $sec.SetOwner($admins)
  return $sec
}
$root = Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'Thimsy Installer'
$dir = Join-Path $root ('driver-' + [guid]::NewGuid().ToString())
try {
  if (Test-Path -LiteralPath $root) {
    & (Join-Path ([Environment]::SystemDirectory) 'takeown.exe') /F $root /A | Out-Null
    Set-Acl -LiteralPath $root -AclObject (New-AdminSecurity)
  } else {
    [void][System.IO.Directory]::CreateDirectory($root, (New-AdminSecurity))
  }
  [void][System.IO.Directory]::CreateDirectory($dir, (New-AdminSecurity))
  $exe = Join-Path $dir 'wdi-simple.exe'
  $dest = Join-Path $dir 'inf'
"#,
    );
    s.push_str(&format!(
        "  Copy-Item -LiteralPath {} -Destination $exe\n",
        ps_literal(&tool.display().to_string())
    ));
    s.push_str(&format!(
        "  if ((Get-FileHash -Algorithm SHA256 -LiteralPath $exe).Hash -ne {}) {{ $code = 3; throw 'wdi-simple.exe changed before it could run' }}\n",
        ps_literal(&sha256.to_ascii_uppercase())
    ));
    s.push_str("  $code = 0\n");
    for (vid, pid, label) in usb {
        // Shown in Device Manager; no characters Windows PowerShell 5.1
        // passes to native programs badly.
        let label: String = label.chars().filter(|c| !matches!(c, '"' | '\\')).collect();
        s.push_str(&format!(
            "  if ($code -eq 0) {{ & $exe '--vid' '0x{vid:04x}' '--pid' '0x{pid:04x}' '--type' '0' '--name' {} '--inf' 'thimsy_{vid:04x}_{pid:04x}.inf' '--dest' $dest; $code = $LASTEXITCODE }}\n",
            ps_literal(&label)
        ));
    }
    s.push_str(
        r#"} catch {
  if ($code -eq 0) { $code = 1 }
} finally {
  Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
}
exit $code
"#,
    );
    s
}

/// PowerShell's `-EncodedCommand` form of a script: base64 of UTF-16LE.
pub fn encode_command(script: &str) -> String {
    let bytes: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    crate::util::base64(&bytes)
}

/// `ERROR_CANCELLED`: the user declined the UAC prompt.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
const DECLINED: i32 = 1223;

/// Installs WinUSB for the IDs not covered yet, all behind one elevated
/// run: `run(script)` runs the script elevated and returns its exit code
/// ([`DECLINED`] when the UAC prompt was declined); `installed(vid, pid)`
/// looks in the driver store. Every ID must be covered afterwards.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn install_ids(
    d: &Device,
    tool: &Path,
    sha256: &str,
    rep: &Reporter,
    installed: &dyn Fn(u16, u16) -> bool,
    run: &mut dyn FnMut(&str) -> Result<Option<i32>>,
) -> Result<()> {
    let todo: Vec<_> = d
        .identify
        .usb
        .iter()
        .filter(|u| {
            let have = installed(u.vid, u.pid);
            if have {
                rep.info(format!("WinUSB is already installed for {}", u.label));
            }
            !have
        })
        .collect();
    if todo.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = todo
        .iter()
        .map(|u| format!("{} ({:04x}:{:04x})", u.label, u.vid, u.pid))
        .collect();
    rep.info(format!(
        "Installing WinUSB for {}; Windows asks for permission",
        names.join(", ")
    ));
    let usb: Vec<(u16, u16, String)> = todo
        .iter()
        .map(|u| (u.vid, u.pid, u.label.clone()))
        .collect();
    let code = run(&driver_script(tool, sha256, &usb))?;
    if code != Some(0) {
        let why = match code {
            Some(DECLINED) => "the Windows permission prompt was declined".to_string(),
            Some(3) => "the driver installer changed before it could run".to_string(),
            Some(c) => format!("exit code {c}"),
            None => "stopped".to_string(),
        };
        return Err(EngineError::new(
            Code::ToolFailed,
            format!("driver install for {} failed ({why})", names.join(", ")),
        )
        .hint("Accept the Windows permission prompt, or install WinUSB with Zadig.")
        .into());
    }
    for u in todo {
        if !installed(u.vid, u.pid) {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!(
                    "the driver installer finished, but Windows still has no WinUSB driver for {} ({:04x}:{:04x})",
                    u.label, u.vid, u.pid
                ),
            )
            .hint("Accept the Windows permission prompt, or install WinUSB with Zadig.")
            .into());
        }
    }
    Ok(())
}

/// Installs WinUSB for every flashing-mode ID of the device.
fn install(d: &Device, rep: &Reporter) -> Result<()> {
    if let Some(r) = test_install(d, rep) {
        return r;
    }
    platform_install(d, rep)
}

#[cfg(windows)]
fn platform_install(d: &Device, rep: &Reporter) -> Result<()> {
    let tool = crate::tools::locate("wdi-simple")?;
    let sha = crate::util::sha256_file(&tool, |_, _| {})?;
    install_ids(d, &tool, &sha, rep, &winusb_installed, &mut |script| {
        run_elevated(script)
    })
}

#[cfg(not(windows))]
fn platform_install(_d: &Device, _rep: &Reporter) -> Result<()> {
    Err(EngineError::new(
        Code::UnsupportedHost,
        "WinUSB can only be installed on Windows",
    )
    .into())
}

/// Windows PowerShell, by absolute path in the system folder.
#[cfg(windows)]
fn powershell_path() -> Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    let mut buf = [0u16; 512];
    // SAFETY: the buffer is as long as passed.
    let n = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() {
        return Err(std::io::Error::last_os_error().into());
    }
    let sys = PathBuf::from(std::ffi::OsString::from_wide(&buf[..n]));
    Ok(sys.join("WindowsPowerShell\\v1.0\\powershell.exe"))
}

/// Runs `script` in a hidden, elevated Windows PowerShell (one UAC prompt)
/// and waits for it: its exit code, or [`DECLINED`] when the prompt was
/// declined. The elevated process inherits none of the engine's handles.
#[cfg(windows)]
fn run_elevated(script: &str) -> Result<Option<i32>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, WaitForSingleObject, INFINITE,
    };
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    let wide = |s: &std::ffi::OsStr| -> Vec<u16> { s.encode_wide().chain([0]).collect() };
    let exe = wide(powershell_path()?.as_os_str());
    let verb = wide(std::ffi::OsStr::new("runas"));
    let params = wide(std::ffi::OsStr::new(&format!(
        "-NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand {}",
        encode_command(script)
    )));
    // SAFETY: the strings outlive the call; the process handle is closed here.
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
                return Ok(Some(DECLINED));
            }
            return Err(std::io::Error::last_os_error().into());
        }
        if info.hProcess.is_null() {
            return Ok(None);
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        let ok = GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        if ok == 0 {
            return Ok(None);
        }
        Ok(Some(code as i32))
    }
}

/// First of `candidates` that exists.
fn first_existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// The fixed shell script `pkexec` runs as root: writes the rules (`$1`)
/// to [`UDEV_RULE_FILE`] and reloads udev with `$2` (udevadm's absolute
/// path). The rules and path are arguments, never part of the script.
pub const UDEV_INSTALL_SCRIPT: &str = "set -e; f=/etc/udev/rules.d/70-thimsy-installer.rules; umask 022; mkdir -p /etc/udev/rules.d; printf '%s' \"$1\" > \"$f.tmp\"; mv -f \"$f.tmp\" \"$f\"; \"$2\" control --reload-rules; \"$2\" trigger";

/// Linux: writes the udev rule through `pkexec` (one password prompt),
/// then checks it took effect.
fn linux_install(d: &Device, rep: &Reporter) -> Result<()> {
    if linux_status(d).status == "ok" {
        rep.info("USB access is already set up");
        return Ok(());
    }
    let pkexec = first_existing(&["/usr/bin/pkexec", "/bin/pkexec", "/usr/local/bin/pkexec"])
        .ok_or_else(|| {
            EngineError::new(Code::ToolMissing, "pkexec isn't installed, so USB access can't be set up from the installer")
                .hint("Install polkit (pkexec), or add a udev rule for the device's download mode yourself, then try again.")
        })?;
    let udevadm = first_existing(&[
        "/usr/bin/udevadm",
        "/bin/udevadm",
        "/usr/sbin/udevadm",
        "/sbin/udevadm",
    ])
    .ok_or_else(|| {
        EngineError::new(Code::ToolMissing, "udevadm isn't installed")
            .hint("Install polkit (pkexec), or add a udev rule for the device's download mode yourself, then try again.")
    })?;
    rep.info(format!(
        "Writing {UDEV_RULE_FILE} for {}; Linux asks for your password",
        ids(d)
    ));
    let status = std::process::Command::new(&pkexec)
        .arg("/bin/sh")
        .arg("-c")
        .arg(UDEV_INSTALL_SCRIPT)
        .arg("sh")
        .arg(udev_rules(d))
        .arg(&udevadm)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    match status.code() {
        Some(0) => {}
        Some(126) | Some(127) => {
            return Err(EngineError::new(
                Code::ToolFailed,
                "setting up USB access was cancelled (the password prompt was declined)",
            )
            .hint("Enter your password when asked, then try again.")
            .into())
        }
        c => {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!("setting up USB access failed (pkexec exit {c:?})"),
            )
            .hint("Install polkit (pkexec), or add a udev rule for the device's download mode yourself, then try again.")
            .into())
        }
    }
    if linux_status(d).status != "ok" {
        return Err(EngineError::new(
            Code::ToolFailed,
            format!("{UDEV_RULE_FILE} was written, but udev still has no rule for {}", ids(d)),
        )
        .hint("Install polkit (pkexec), or add a udev rule for the device's download mode yourself, then try again.")
        .into());
    }
    Ok(())
}

#[cfg(not(test))]
fn test_install(_d: &Device, _rep: &Reporter) -> Option<Result<()>> {
    None
}

#[cfg(all(not(test), not(windows)))]
fn test_winusb(_vid: u16, _pid: u16) -> bool {
    false
}

// Kept last: `dev i18n-strings` stops reading a file at its first test-only
// item.

/// Test hook: pretend to be Windows with this elevated-run exit code.
#[cfg(test)]
fn test_install(d: &Device, rep: &Reporter) -> Option<Result<()>> {
    let code = test::ELEVATED_EXIT.with(|c| c.get())?;
    let tool = Path::new(r"C:\Program Files\Thimsy Installer\engine\tools\wdi-simple.exe");
    Some(install_ids(
        d,
        tool,
        &"0".repeat(64),
        rep,
        &winusb_installed,
        &mut |_| {
            if code == 0 {
                test::WINUSB.with(|w| w.set(true));
            }
            Ok(Some(code))
        },
    ))
}

#[cfg(all(test, not(windows)))]
fn test_winusb(_vid: u16, _pid: u16) -> bool {
    test::WINUSB.with(|w| w.get())
}

#[cfg(test)]
pub(crate) mod test {
    use std::cell::Cell;
    thread_local! {
        /// Some(code): `install` runs the Windows path with this exit code.
        pub static ELEVATED_EXIT: Cell<Option<i32>> = const { Cell::new(None) };
        /// What the driver store says (non-Windows test builds).
        pub static WINUSB: Cell<bool> = const { Cell::new(false) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_winusb_packages_but_not_vendor_drivers() {
        let libwdi = "[Devices.NTamd64]\r\n\"Unisoc download mode\" = USB_Install, USB\\VID_1782&PID_4D00\r\n[USB_Install]\r\nInclude = winusb.inf\r\nNeeds = WINUSB.NT\r\n";
        assert!(inf_is_winusb_for(libwdi, 0x1782, 0x4d00));
        assert!(!inf_is_winusb_for(libwdi, 0x1782, 0x4d01));
        let vendor = "[SPRD]\r\n%SPRD.DeviceDesc% = SPRD_Install, USB\\VID_1782&PID_4D00\r\nServiceBinary = %12%\\sprdvcom.sys\r\n";
        assert!(!inf_is_winusb_for(vendor, 0x1782, 0x4d00));
        // Windows keeps some packages as UTF-16.
        let mut utf16 = vec![0xFF, 0xFE];
        for u in libwdi.encode_utf16() {
            utf16.extend_from_slice(&u.to_le_bytes());
        }
        assert!(inf_is_winusb_for(&decode_inf(&utf16), 0x1782, 0x4d00));
    }

    #[test]
    fn powershell_literals_double_every_kind_of_single_quote() {
        assert_eq!(ps_literal("plain"), "'plain'");
        assert_eq!(ps_literal("Ann O'Neil"), "'Ann O''Neil'");
        // PowerShell ends a single-quoted string at any of these.
        for q in ['\'', '\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'] {
            let s = format!("a{q}b; Remove-Item C:\\ -Recurse #");
            let lit = ps_literal(&s);
            assert_eq!(lit, format!("'a{q}{q}b; Remove-Item C:\\ -Recurse #'"));
            // Inside the quotes, every quote character comes in pairs.
            let inner: Vec<char> = lit[1..lit.len() - 1].chars().collect();
            let mut i = 0;
            while i < inner.len() {
                if matches!(
                    inner[i],
                    '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}'
                ) {
                    assert!(i + 1 < inner.len() && inner[i + 1] == inner[i], "{lit}");
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
        // Not quote characters: left alone.
        assert_eq!(ps_literal("\u{201C}x\u{201D}"), "'\u{201C}x\u{201D}'");
    }

    #[test]
    fn driver_script_copies_checks_runs_and_cleans_up() {
        let s = driver_script(
            Path::new("C:\\Users\\Ann O\u{2019}Neil\\Thimsy Installer\\tools\\wdi-simple.exe"),
            "abc123",
            &[(0x1782, 0x4d00, "Unisoc \"download\" mode".into())],
        );
        assert!(s.contains("Copy-Item -LiteralPath 'C:\\Users\\Ann O\u{2019}\u{2019}Neil\\Thimsy Installer\\tools\\wdi-simple.exe' -Destination $exe"), "{s}");
        assert!(s.contains("[guid]::NewGuid()"));
        assert!(s.contains("GetFolderPath('CommonApplicationData')) 'Thimsy Installer'"));
        assert!(s.contains("SetAccessRuleProtection($true, $false)"));
        assert!(s.contains("'S-1-5-32-544'") && s.contains("'S-1-5-18'"));
        assert!(s.contains(".Hash -ne 'ABC123'"));
        assert!(s.contains("& $exe '--vid' '0x1782' '--pid' '0x4d00' '--type' '0' '--name' 'Unisoc download mode' '--inf' 'thimsy_1782_4d00.inf' '--dest' $dest; $code = $LASTEXITCODE"), "{s}");
        // The folder is made admin-only before anything is copied into it,
        // and removed whatever happens.
        let made = s.find("CreateDirectory($dir").unwrap();
        assert!(made < s.find("Copy-Item").unwrap());
        assert!(s.find("Copy-Item").unwrap() < s.find("Get-FileHash").unwrap());
        assert!(s.contains("finally {\n  Remove-Item -LiteralPath $dir -Recurse -Force"));
        assert!(s.trim_end().ends_with("exit $code"));
        // The encoded form PowerShell decodes back to the script.
        let enc = encode_command("exit 0");
        assert_eq!(enc, "ZQB4AGkAdAAgADAA");
    }

    #[test]
    fn declined_or_ineffective_driver_installs_fail_clearly() {
        let cat = crate::catalog::Catalog::bundled().unwrap();
        let d = cat.device("anbernic-rg-cube").unwrap();
        let rep = Reporter::new(std::sync::Arc::new(crate::events::MemorySink::default()));
        let tool = Path::new(r"C:\Thimsy\tools\wdi-simple.exe");
        let sha = "0".repeat(64);
        let tool_failed = |e: &anyhow::Error| crate::error::code_of(e).0 == Code::ToolFailed;
        // UAC declined.
        let mut ran = Vec::new();
        let err = install_ids(d, tool, &sha, &rep, &|_, _| false, &mut |ps| {
            ran.push(ps.to_string());
            Ok(Some(DECLINED))
        })
        .unwrap_err();
        assert!(
            tool_failed(&err) && err.to_string().contains("declined"),
            "{err}"
        );
        assert!(ran[0].starts_with("$ErrorActionPreference = 'Stop'"));
        // PowerShell stopped by an error.
        let err =
            install_ids(d, tool, &sha, &rep, &|_, _| false, &mut |_| Ok(Some(1))).unwrap_err();
        assert!(err.to_string().contains("exit code 1"), "{err}");
        // Reported success, but Windows still has no driver for the ID.
        let err =
            install_ids(d, tool, &sha, &rep, &|_, _| false, &mut |_| Ok(Some(0))).unwrap_err();
        assert!(
            tool_failed(&err) && err.to_string().contains("still has no WinUSB driver"),
            "{err}"
        );
        // Installed for real.
        let done = std::cell::Cell::new(false);
        install_ids(d, tool, &sha, &rep, &|_, _| done.get(), &mut |_| {
            done.set(true);
            Ok(Some(0))
        })
        .unwrap();
        // Already there: nothing runs.
        install_ids(d, tool, &sha, &rep, &|_, _| true, &mut |_| {
            panic!("nothing to install")
        })
        .unwrap();
    }

    #[test]
    fn linux_udev_rules_are_fixed_text_and_found_case_insensitively() {
        let cat = crate::catalog::Catalog::bundled().unwrap();
        let d = cat.device("anbernic-rg-cube").unwrap();
        let rules = udev_rules(d);
        assert!(rules.contains(
            "SUBSYSTEM==\"usb\", ATTR{idVendor}==\"1782\", ATTR{idProduct}==\"4d00\", TAG+=\"uaccess\"\n"
        ));
        assert_eq!(
            rules.lines().filter(|l| !l.starts_with('#')).count(),
            d.identify.usb.len()
        );
        // The script pkexec runs takes the rules and udevadm as arguments.
        assert!(!UDEV_INSTALL_SCRIPT.contains("1782"));
        assert!(UDEV_INSTALL_SCRIPT.contains("printf '%s' \"$1\""));
        let dir = tempfile::tempdir().unwrap();
        let dirs = vec![dir.path().to_path_buf(), dir.path().join("missing")];
        assert!(!rules_cover(&dirs, d));
        std::fs::write(dir.path().join("50-other.rules"), "# 1782 4d00 commented out\nSUBSYSTEM==\"usb\", ATTR{idVendor}==\"18d1\", MODE=\"0666\"\n").unwrap();
        assert!(!rules_cover(&dirs, d));
        std::fs::write(dir.path().join("51-android.rules"), "SUBSYSTEM==\"usb\", ATTRS{idVendor}==\"1782\", ATTRS{idProduct}==\"4D00\", MODE=\"0660\", GROUP=\"plugdev\"\n").unwrap();
        assert!(rules_cover(&dirs, d));
        // Only .rules files count.
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("notes.txt"), rules).unwrap();
        assert!(!rules_cover(&[other.path().to_path_buf()], d));
    }
}
