//! USB driver setup. Only Windows needs it: the bundled libusb tools need
//! WinUSB on the device's flashing-mode IDs, installed with the bundled
//! `wdi-simple` (libwdi) behind one UAC prompt.

use crate::catalog::Device;
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DriverStatus {
    pub status: &'static str, // ok | missing | not_required
    pub detail: String,
}

pub fn status(d: &Device, host: Host) -> DriverStatus {
    if host != Host::Windows {
        return DriverStatus {
            status: "not_required",
            detail: format!("{} needs no driver", host.display()),
        };
    }
    if !d.identify.usb.is_empty()
        && d.identify
            .usb
            .iter()
            .all(|u| winusb_installed(u.vid, u.pid))
    {
        return DriverStatus {
            status: "ok",
            detail: format!("WinUSB is installed for {}", ids(d)),
        };
    }
    match crate::tools::locate("wdi-simple") {
        Ok(_) => DriverStatus {
            status: "missing",
            detail: format!("WinUSB will be installed for {} the first time you flash", ids(d)),
        },
        Err(_) => DriverStatus {
            status: "missing",
            detail: "The driver installer isn't bundled with this build. Install WinUSB for the device's download mode with Zadig.".into(),
        },
    }
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
fn winusb_installed(_vid: u16, _pid: u16) -> bool {
    false
}

fn ids(d: &Device) -> String {
    d.identify
        .usb
        .iter()
        .map(|u| format!("{:04x}:{:04x}", u.vid, u.pid))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Installs WinUSB for every flashing-mode ID of the device (Windows only).
pub fn ensure(d: &Device, host: Host, rep: &Reporter) -> Result<()> {
    if host != Host::Windows {
        rep.info("No driver needed on this computer");
        return Ok(());
    }
    install(d, rep)
}

/// A PowerShell single-quoted string literal.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// One Windows command-line argument, quoted the way the C runtime splits
/// command lines (`Start-Process` joins `-ArgumentList` items with spaces
/// and quotes nothing itself).
fn win_arg(a: &str) -> String {
    if !a.is_empty() && !a.contains([' ', '\t', '"']) {
        return a.to_string();
    }
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in a.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            c => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

/// The PowerShell command that runs `wdi-simple` elevated (one UAC prompt,
/// no console window) for one USB ID and exits with its exit code.
pub fn ps_command(tool: &Path, vid: u16, pid: u16, label: &str, dest: &Path) -> String {
    let args = [
        "--vid".to_string(),
        format!("0x{vid:04x}"),
        "--pid".into(),
        format!("0x{pid:04x}"),
        "--type".into(),
        "0".into(),
        "--name".into(),
        label.to_string(),
        "--inf".into(),
        format!("thimsy_{vid:04x}_{pid:04x}.inf"),
        "--dest".into(),
        dest.display().to_string(),
    ];
    let list = args
        .iter()
        .map(|a| ps_quote(&win_arg(a)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "$p = Start-Process -FilePath {} -ArgumentList @({list}) -Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $p.ExitCode",
        ps_quote(&tool.display().to_string())
    )
}

#[cfg(windows)]
pub fn install(d: &Device, rep: &Reporter) -> Result<()> {
    use std::process::Command;
    let tool = crate::tools::locate("wdi-simple")?;
    let dest = std::env::temp_dir().join("thimsy-driver");
    for u in &d.identify.usb {
        rep.info(format!(
            "Installing WinUSB for {} ({:04x}:{:04x}); Windows asks for permission",
            u.label, u.vid, u.pid
        ));
        // Elevate through PowerShell so only this step needs admin rights.
        if winusb_installed(u.vid, u.pid) {
            rep.info(format!("WinUSB is already installed for {}", u.label));
            continue;
        }
        let ps = ps_command(&tool, u.vid, u.pid, &u.label, &dest);
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
        crate::tools::hide_window(&mut cmd);
        let st = cmd.status()?;
        if !st.success() {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!("driver install for {} failed ({st})", u.label),
            )
            .hint("Accept the Windows permission prompt, or install WinUSB with Zadig.")
            .into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn install(_d: &Device, _rep: &Reporter) -> Result<()> {
    Err(EngineError::new(
        Code::UnsupportedHost,
        "driver installation is only needed on Windows",
    )
    .into())
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
    fn powershell_command_quotes_apostrophes_and_spaces() {
        let cmd = ps_command(
            Path::new(r"C:\Users\Ann O'Neil\Thimsy Installer\tools\wdi-simple.exe"),
            0x1782,
            0x4d00,
            "Unisoc download mode",
            Path::new(r"C:\Users\Ann O'Neil\AppData\Local\Temp\thimsy-driver"),
        );
        assert_eq!(
            cmd,
            concat!(
                r"$p = Start-Process -FilePath 'C:\Users\Ann O''Neil\Thimsy Installer\tools\wdi-simple.exe' ",
                r#"-ArgumentList @('--vid','0x1782','--pid','0x4d00','--type','0','--name','"Unisoc download mode"','--inf','thimsy_1782_4d00.inf','--dest','"C:\Users\Ann O''Neil\AppData\Local\Temp\thimsy-driver"') "#,
                "-Verb RunAs -WindowStyle Hidden -Wait -PassThru; exit $p.ExitCode"
            )
        );
        // Windows command-line quoting: quotes escaped, trailing backslashes doubled.
        assert_eq!(win_arg(r#"a "b" c"#), r#""a \"b\" c""#);
        assert_eq!(win_arg(r"C:\dir with space\"), r#""C:\dir with space\\""#);
        assert_eq!(win_arg(""), r#""""#);
        assert_eq!(win_arg("plain"), "plain");
    }
}
