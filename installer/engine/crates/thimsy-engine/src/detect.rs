//! USB detection: which known devices and flashing modes are connected.

use crate::catalog::Catalog;
use anyhow::Result;
use nusb::MaybeFuture;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Seen {
    pub vidpid: String,
    pub mode: String,
    pub device: Option<String>,
    pub label: String,
}

/// Generic modes we recognise even without a catalog match.
const GENERIC: &[(u16, u16, &str, &str)] = &[
    (0x1782, 0x4d00, "download", "Unisoc download mode"),
    (0x18d1, 0x4ee0, "fastboot", "Android fastboot"),
    (0x18d1, 0xd00d, "fastboot", "Android fastboot"),
    (0x05c6, 0x9008, "edl", "Qualcomm emergency download (EDL)"),
    (0x0e8d, 0x0003, "brom", "MediaTek BootROM"),
    (0x0e8d, 0x2000, "preloader", "MediaTek preloader"),
];

pub fn classify(cat: &Catalog, vid: u16, pid: u16, product: Option<&str>) -> Option<Seen> {
    let vidpid = format!("{vid:04x}:{pid:04x}");
    // Flashing-mode IDs are shared by every device on a chip family, so only
    // name the device when exactly one catalog entry uses the ID.
    let owners: Vec<_> = cat
        .devices
        .iter()
        .filter_map(|d| {
            d.identify
                .usb
                .iter()
                .find(|u| u.vid == vid && u.pid == pid)
                .map(|u| (d, u))
        })
        .collect();
    if let Some((d, u)) = owners.first() {
        let device = (owners.len() == 1).then(|| d.id.clone());
        return Some(Seen {
            vidpid,
            mode: u.mode.clone(),
            device,
            label: u.label.clone(),
        });
    }
    for d in &cat.devices {
        if let (Some(m), Some(p)) = (&d.identify.adb_model, product) {
            if p == m {
                return Some(Seen {
                    vidpid,
                    mode: "adb".into(),
                    device: Some(d.id.clone()),
                    label: format!("{} (Android)", d.name),
                });
            }
        }
    }
    GENERIC
        .iter()
        .find(|g| g.0 == vid && g.1 == pid)
        .map(|g| Seen {
            vidpid,
            mode: g.2.into(),
            device: None,
            label: g.3.into(),
        })
}

/// (vendor ID, product ID, product string) of a connected USB device.
pub type UsbDevice = (u16, u16, Option<String>);

/// Every USB device connected.
fn list() -> Result<Vec<UsbDevice>> {
    if let Some(fake) = test_devices() {
        return Ok(fake);
    }
    Ok(nusb::list_devices()
        .wait()?
        .map(|i| {
            (
                i.vendor_id(),
                i.product_id(),
                i.product_string().map(str::to_string),
            )
        })
        .collect())
}

/// (vendor ID, product ID) of every USB device connected, whether or not
/// this user may open it.
pub fn scan_ids() -> Result<Vec<(u16, u16)>> {
    Ok(list()?.into_iter().map(|(v, p, _)| (v, p)).collect())
}

pub fn scan(cat: &Catalog) -> Result<Vec<Seen>> {
    let mut out = Vec::new();
    for (vid, pid, product) in list()? {
        if let Some(s) = classify(cat, vid, pid, product.as_deref()) {
            out.push(s);
        }
    }
    out.sort_by(|a, b| a.vidpid.cmp(&b.vidpid));
    Ok(out)
}

#[cfg(not(test))]
fn test_devices() -> Option<Vec<UsbDevice>> {
    None
}

// Kept last: `dev i18n-strings` stops reading a file at its first test-only
// item.

#[cfg(test)]
fn test_devices() -> Option<Vec<UsbDevice>> {
    test::DEVICES.with(|d| d.borrow().clone())
}

/// Test hook: the USB devices [`scan`] sees, per thread.
#[cfg(test)]
pub(crate) mod test {
    use std::cell::RefCell;
    thread_local! {
        pub static DEVICES: RefCell<Option<Vec<super::UsbDevice>>> = const { RefCell::new(None) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_known_ids() {
        let cat = Catalog::bundled().unwrap();
        let s = classify(&cat, 0x1782, 0x4d00, None).unwrap();
        assert_eq!(s.mode, "download");
        // The RG Cube and RG556 share Unisoc's download-mode ID.
        assert_eq!(s.device, None);
        assert_eq!(classify(&cat, 0x05c6, 0x9008, None).unwrap().mode, "edl");
        assert!(classify(&cat, 0x1234, 0x5678, None).is_none());
    }
}
