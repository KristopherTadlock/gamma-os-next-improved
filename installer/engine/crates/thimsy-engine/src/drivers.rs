//! USB driver setup. Only Windows needs it: the bundled libusb tools need
//! WinUSB on the device's flashing-mode IDs, installed with the bundled
//! `wdi-simple` (libwdi) behind one UAC prompt.

use crate::catalog::Device;
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use anyhow::Result;
use serde::Serialize;

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

#[cfg(windows)]
pub fn install(d: &Device, rep: &Reporter) -> Result<()> {
    use std::process::Command;
    let tool = crate::tools::locate("wdi-simple")?;
    for u in &d.identify.usb {
        rep.info(format!(
            "Installing WinUSB for {} ({:04x}:{:04x}); Windows asks for permission",
            u.label, u.vid, u.pid
        ));
        // Elevate through PowerShell so only this step needs admin rights.
        let args = format!(
            "--vid 0x{:04x} --pid 0x{:04x} --type 0 --name '{}' --dest $env:TEMP\\thimsy-driver",
            u.vid,
            u.pid,
            u.label.replace('\'', "")
        );
        let ps = format!(
            "$p = Start-Process -FilePath '{}' -ArgumentList \"{}\" -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
            tool.display(),
            args
        );
        let st = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .status()?;
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
