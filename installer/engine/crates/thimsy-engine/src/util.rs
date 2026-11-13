//! Small I/O helpers shared by the container code.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const CHUNK: usize = 8 << 20;

/// `YYYYMMDD-HHMMSS` (UTC) for a time in seconds since 1970.
pub fn utc_stamp(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rest = secs % 86400;
    // Days to civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a time in seconds since 1970.
pub fn utc_iso(secs: u64) -> String {
    let s = utc_stamp(secs);
    format!(
        "{}-{}-{}T{}:{}:{}Z",
        &s[0..4],
        &s[4..6],
        &s[6..8],
        &s[9..11],
        &s[11..13],
        &s[13..15]
    )
}

/// Seconds since 1970 now.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// sha256 of a whole file, with progress.
pub fn sha256_file(path: &Path, mut progress: impl FnMut(u64, u64)) -> Result<String> {
    let mut f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let total = f.metadata()?.len();
    sha256_range(&mut f, 0, total, &mut progress)
}

/// sha256 of `len` bytes of `f` starting at `offset`.
pub fn sha256_range(
    f: &mut File,
    offset: u64,
    len: u64,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<String> {
    f.seek(SeekFrom::Start(offset))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut left = len;
    while left > 0 {
        let n = (left as usize).min(buf.len());
        f.read_exact(&mut buf[..n])
            .context("reading for checksum")?;
        h.update(&buf[..n]);
        left -= n as u64;
        progress(len - left, len);
    }
    Ok(hex::encode(h.finalize()))
}

/// sha256 of a whole file and, in the same pass, of each `(offset, len)`
/// range in it (ranges may overlap). Fails if a range runs past the end.
pub fn sha256_with_ranges(
    path: &Path,
    ranges: &[(u64, u64)],
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(String, Vec<String>)> {
    let mut f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let total = f.metadata()?.len();
    for &(off, len) in ranges {
        if add(off, len)? > total {
            bail!(
                "a range ({off}+{len}) runs past the end of {}",
                path.display()
            );
        }
    }
    let mut whole = Sha256::new();
    let mut parts: Vec<Sha256> = ranges.iter().map(|_| Sha256::new()).collect();
    let mut buf = vec![0u8; CHUNK];
    let mut pos = 0u64;
    while pos < total {
        let n = ((total - pos) as usize).min(buf.len());
        f.read_exact(&mut buf[..n])
            .context("reading for checksum")?;
        let chunk = &buf[..n];
        whole.update(chunk);
        let end = pos + n as u64;
        for (h, &(off, len)) in parts.iter_mut().zip(ranges) {
            let (a, b) = (off.max(pos), (off + len).min(end));
            if a < b {
                h.update(&chunk[(a - pos) as usize..(b - pos) as usize]);
            }
        }
        pos = end;
        progress(pos, total);
    }
    Ok((
        hex::encode(whole.finalize()),
        parts
            .into_iter()
            .map(|h| hex::encode(h.finalize()))
            .collect(),
    ))
}

/// Standard base64 (with padding).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Copies `len` bytes from `src` (at its current position) to `dst`.
pub fn copy_exact(
    src: &mut dyn Read,
    dst: &mut dyn Write,
    len: u64,
    progress: &mut dyn FnMut(u64),
) -> Result<()> {
    let mut buf = vec![0u8; CHUNK];
    let mut left = len;
    while left > 0 {
        let n = (left as usize).min(buf.len());
        src.read_exact(&mut buf[..n])
            .context("unexpected end of input")?;
        dst.write_all(&buf[..n])?;
        left -= n as u64;
        progress(len - left);
    }
    Ok(())
}

pub fn read_at(f: &mut File, offset: u64, len: usize) -> Result<Vec<u8>> {
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)
        .with_context(|| format!("reading {len} bytes at {offset}"))?;
    Ok(buf)
}

pub fn write_at(f: &mut File, offset: u64, data: &[u8]) -> Result<()> {
    f.seek(SeekFrom::Start(offset))?;
    f.write_all(data)?;
    Ok(())
}

/// Decodes a NUL-terminated UTF-16LE field.
pub fn utf16_field(raw: &[u8]) -> String {
    let units: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

pub fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
pub fn u64_at(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
pub fn u16_at(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(b[off..off + 2].try_into().unwrap())
}

/// Checked `a + b` for offsets read from untrusted headers.
pub fn add(a: u64, b: u64) -> Result<u64> {
    match a.checked_add(b) {
        Some(v) => Ok(v),
        None => bail!("offset overflow ({a} + {b})"),
    }
}

/// Atomically writes a small file (write to `<name>.tmp`, then rename).
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    {
        let mut f = File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.2} {}", UNITS[i])
    }
}
