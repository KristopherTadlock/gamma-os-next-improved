//! What the drive a workspace is on can hold: free space, and whether it's
//! FAT32 (no file over 4 GiB, and the finished image is larger).

use std::path::{Path, PathBuf};

/// Largest file FAT32 can hold.
pub const FAT32_MAX_FILE: u64 = (4 << 30) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Bytes available to this user.
    pub free: u64,
    /// The file system is FAT (FAT12/16/32; not exFAT).
    pub fat32: bool,
}

/// `path`, or its nearest ancestor that exists (a workspace may not have
/// been created yet). A relative path with no existing part is the current
/// folder.
pub fn existing_ancestor(path: &Path) -> PathBuf {
    for a in path.ancestors() {
        let a = if a.as_os_str().is_empty() {
            Path::new(".")
        } else {
            a
        };
        if a.exists() {
            return a.to_path_buf();
        }
    }
    PathBuf::from(".")
}

/// Free space and file system of the drive `path` is on (its nearest
/// existing ancestor). An error means nothing is known: callers treat that
/// as "doesn't fit", never as unlimited space.
pub fn probe(path: &Path) -> std::io::Result<Probe> {
    let at = existing_ancestor(path);
    if let Some(r) = test_probe() {
        return r;
    }
    let free = fs4::available_space(&at)?;
    Ok(Probe {
        free,
        fat32: is_fat(&at)?,
    })
}

#[cfg(target_os = "linux")]
fn is_fat(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::ffi::OsStrExt;
    /// `MSDOS_SUPER_MAGIC`: the kernel's FAT driver (vfat/msdos).
    const MSDOS_SUPER_MAGIC: i64 = 0x4d44;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: statfs fills the zeroed struct; `c` is NUL-terminated.
    unsafe {
        let mut st: libc::statfs = std::mem::zeroed();
        if libc::statfs(c.as_ptr(), &mut st) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        #[allow(clippy::unnecessary_cast)]
        Ok(st.f_type as i64 == MSDOS_SUPER_MAGIC)
    }
}

#[cfg(target_os = "macos")]
fn is_fat(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: statfs fills the zeroed struct; `c` is NUL-terminated and
    // f_fstypename is NUL-terminated by the kernel.
    unsafe {
        let mut st: libc::statfs = std::mem::zeroed();
        if libc::statfs(c.as_ptr(), &mut st) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let name = std::ffi::CStr::from_ptr(st.f_fstypename.as_ptr());
        Ok(name.to_bytes() == b"msdos")
    }
}

#[cfg(windows)]
fn is_fat(path: &Path) -> std::io::Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetVolumeInformationW, GetVolumePathNameW};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut root = [0u16; 1024];
    let mut fs = [0u16; 64];
    // SAFETY: buffers are sized as passed; strings are NUL-terminated.
    unsafe {
        if GetVolumePathNameW(wide.as_ptr(), root.as_mut_ptr(), root.len() as u32) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if GetVolumeInformationW(
            root.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs.as_mut_ptr(),
            fs.len() as u32,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    let end = fs.iter().position(|&c| c == 0).unwrap_or(fs.len());
    Ok(fat_name(&String::from_utf16_lossy(&fs[..end])))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn is_fat(_path: &Path) -> std::io::Result<bool> {
    Ok(false)
}

/// Whether a Windows file system name is FAT ("FAT", "FAT32"; not "exFAT").
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn fat_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("FAT") || name.eq_ignore_ascii_case("FAT32")
}

#[cfg(not(test))]
fn test_probe() -> Option<std::io::Result<Probe>> {
    None
}

// Kept last: `dev i18n-strings` stops reading a file at its first test-only
// item.

#[cfg(test)]
fn test_probe() -> Option<std::io::Result<Probe>> {
    test::PROBE.with(|p| {
        p.borrow()
            .as_ref()
            .map(|r| r.clone().map_err(std::io::Error::other))
    })
}

/// Test hook: what [`probe`] reports, per thread.
#[cfg(test)]
pub(crate) mod test {
    use std::cell::RefCell;
    thread_local! {
        pub static PROBE: RefCell<Option<Result<super::Probe, String>>> = const { RefCell::new(None) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_the_nearest_existing_folder() {
        let d = tempfile::tempdir().unwrap();
        let ws = d.path().join("not/yet/there");
        assert_eq!(existing_ancestor(&ws), d.path());
        assert_eq!(
            existing_ancestor(Path::new("no-such-dir-xyz")),
            Path::new(".")
        );
        let p = probe(&ws).unwrap();
        assert!(p.free > 0 && p.free < u64::MAX);
        assert!(fat_name("FAT32") && fat_name("fat") && !fat_name("exFAT") && !fat_name("NTFS"));
    }
}
