//! A device pack in one file: an uncompressed tar of the shared system image,
//! the device's patches and `pack.json`, published as one release asset (or
//! split into parts). The engine reads members in place, by byte range, so
//! the 2 GB system image is never copied out of it.
//!
//! The tar is plain POSIX ustar, so 7-Zip, Windows' `tar` and Finder open it.
//! Written deterministically (fixed order, zero times and owners) so the same
//! inputs give the same bytes.

use crate::archive::{sub_spans, MultiVolume, Span};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const BLOCK: u64 = 512;

/// A member's place in the tar.
#[derive(Debug, Clone)]
pub struct Member {
    pub size: u64,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Default)]
pub struct Bundle {
    pub members: BTreeMap<String, Member>,
}

fn octal(field: &[u8]) -> Result<u64> {
    // GNU base-256 for large values: high bit of the first byte set.
    if field.first().is_some_and(|b| b & 0x80 != 0) {
        let mut v: u64 = (field[0] & 0x7f) as u64;
        for &b in &field[1..] {
            v = v.checked_mul(256).context("tar size overflows")? + b as u64;
        }
        return Ok(v);
    }
    let s: String = field
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(s, 8).with_context(|| format!("bad tar number {s:?}"))
}

fn cstr(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// `path` attribute from a pax extended header, if any.
fn pax_path(data: &[u8]) -> Option<String> {
    let mut rest = data;
    while !rest.is_empty() {
        let sp = rest.iter().position(|&b| b == b' ')?;
        let len: usize = std::str::from_utf8(&rest[..sp]).ok()?.parse().ok()?;
        if len == 0 || len > rest.len() {
            return None;
        }
        let rec = &rest[sp + 1..len];
        let rec = rec.strip_suffix(b"\n").unwrap_or(rec);
        if let Some(v) = rec.strip_prefix(b"path=") {
            return Some(String::from_utf8_lossy(v).into_owned());
        }
        rest = &rest[len..];
    }
    None
}

impl Bundle {
    /// Indexes the tar formed by `spans` (one file or its parts, end to end).
    /// Only regular files are kept, by their base name.
    pub fn index(spans: &[Span]) -> Result<Bundle> {
        let mut r = MultiVolume::open_spans(spans)?;
        let total = r.len();
        let mut members = BTreeMap::new();
        let mut pos = 0u64;
        let mut long_name: Option<String> = None;
        let mut header = [0u8; BLOCK as usize];
        let mut ended = false;
        while pos + BLOCK <= total {
            r.seek(SeekFrom::Start(pos))?;
            r.read_exact(&mut header)?;
            if header.iter().all(|&b| b == 0) {
                ended = true; // end of archive
                break;
            }
            // The checksum is the byte sum with its own field read as spaces.
            let stored: u64 = header
                .iter()
                .enumerate()
                .map(|(i, &b)| {
                    if (148..156).contains(&i) {
                        32
                    } else {
                        b as u64
                    }
                })
                .sum();
            if octal(&header[148..156])? != stored {
                bail!("not a tar archive (bad header checksum at byte {pos})");
            }
            let size = octal(&header[124..136])?;
            let kind = header[156];
            let data = pos + BLOCK;
            if data + size > total {
                bail!("the archive is cut short (a member runs past its end)");
            }
            match kind {
                b'L' | b'x' => {
                    if size > 1 << 20 {
                        bail!("tar extended header too large");
                    }
                    let mut buf = vec![0u8; size as usize];
                    r.read_exact(&mut buf)?;
                    long_name = if kind == b'L' {
                        Some(cstr(&buf))
                    } else {
                        pax_path(&buf).or(long_name)
                    };
                }
                b'0' | 0 => {
                    let name = long_name.take().unwrap_or_else(|| {
                        let prefix = cstr(&header[345..500]);
                        let name = cstr(&header[..100]);
                        if prefix.is_empty() {
                            name
                        } else {
                            format!("{prefix}/{name}")
                        }
                    });
                    let base = name.rsplit('/').next().unwrap_or(&name).to_string();
                    if !base.is_empty() {
                        members.insert(
                            base,
                            Member {
                                size,
                                spans: sub_spans(spans, data, size),
                            },
                        );
                    }
                }
                _ => {
                    long_name = None;
                }
            }
            pos = data + size.div_ceil(BLOCK) * BLOCK;
        }
        if !ended {
            bail!("the archive is cut short (no end-of-archive marker)");
        }
        Ok(Bundle { members })
    }

    pub fn get(&self, name: &str) -> Option<&Member> {
        self.members.get(name)
    }
}

fn header(name: &str, size: u64) -> Result<[u8; BLOCK as usize]> {
    if name.len() > 100 || name.contains('/') {
        bail!("bundle member names must be plain names up to 100 bytes: {name}");
    }
    let mut h = [0u8; BLOCK as usize];
    h[..name.len()].copy_from_slice(name.as_bytes());
    h[100..108].copy_from_slice(b"0000644\0");
    h[108..116].copy_from_slice(b"0000000\0");
    h[116..124].copy_from_slice(b"0000000\0");
    if size >= 1 << 33 {
        bail!("{name} is too large for a ustar member");
    }
    h[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
    h[136..148].copy_from_slice(b"00000000000\0");
    h[156] = b'0';
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    h[148..156].fill(b' ');
    let sum: u64 = h.iter().map(|&b| b as u64).sum();
    h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    Ok(h)
}

/// Writes `files` (in this order, by base name) into the tar `out`.
pub fn write(out: &Path, files: &[&Path], progress: &mut dyn FnMut(u64, u64)) -> Result<()> {
    let total: u64 = files
        .iter()
        .map(|f| std::fs::metadata(f).map(|m| m.len()))
        .sum::<io::Result<u64>>()?;
    let mut o = io::BufWriter::with_capacity(1 << 20, File::create(out)?);
    let mut done = 0u64;
    for f in files {
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .with_context(|| format!("bad file name {}", f.display()))?;
        let mut i = File::open(f).with_context(|| format!("opening {}", f.display()))?;
        let size = i.metadata()?.len();
        o.write_all(&header(name, size)?)?;
        let mut left = size;
        let mut buf = vec![0u8; crate::util::CHUNK];
        while left > 0 {
            let want = left.min(buf.len() as u64) as usize;
            let n = i.read(&mut buf[..want])?;
            if n == 0 {
                bail!("{} changed while it was being written", f.display());
            }
            o.write_all(&buf[..n])?;
            left -= n as u64;
            done += n as u64;
            progress(done, total);
        }
        let pad = (size.div_ceil(BLOCK) * BLOCK - size) as usize;
        o.write_all(&vec![0u8; pad])?;
    }
    o.write_all(&[0u8; 2 * BLOCK as usize])?;
    o.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    Ok(())
}

/// GitHub release assets must be smaller than 2 GiB; keep a margin.
pub const MAX_ASSET: u64 = (2 << 30) - (1 << 20);
/// Part size when a device pack has to be split.
pub const PART: u64 = 1_000_000_000;

/// Writes the device pack `out` from `files` and returns its catalog entry
/// (`id`, `file`, `size`, `sha256`, and `parts` when it was larger than
/// `max_asset` and had to be split into `out.001`, `out.002`, ...).
pub fn make(
    out: &Path,
    files: &[&Path],
    max_asset: u64,
    part: u64,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<serde_json::Value> {
    write(out, files, progress)?;
    let size = std::fs::metadata(out)?.len();
    let sha = crate::util::sha256_file(out, |_, _| {})?;
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .context("bad output name")?
        .to_string();
    let mut entry = serde_json::json!({"id": name, "file": name, "size": size, "sha256": sha});
    if size > max_asset {
        let mut parts = Vec::new();
        let mut i = File::open(out)?;
        let mut n = 1;
        let mut left = size;
        while left > 0 {
            let len = left.min(part);
            let pname = format!("{name}.{n:03}");
            let ppath = out.with_file_name(&pname);
            let mut o = File::create(&ppath)?;
            crate::util::copy_exact(&mut i, &mut o, len, &mut |_| {})?;
            o.sync_all()?;
            drop(o);
            let psha = crate::util::sha256_file(&ppath, |_, _| {})?;
            parts
                .push(serde_json::json!({"id": pname, "file": pname, "size": len, "sha256": psha}));
            left -= len;
            n += 1;
        }
        std::fs::remove_file(out)?;
        entry["parts"] = serde_json::Value::Array(parts);
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn read_member(m: &Member) -> Vec<u8> {
        let mut v = Vec::new();
        MultiVolume::open_spans(&m.spans)
            .unwrap()
            .read_to_end(&mut v)
            .unwrap();
        v
    }

    #[test]
    fn writes_and_reads_back_members_in_place() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("pack.json");
        let b = d.path().join("system.img.zst");
        std::fs::write(&a, b"{}").unwrap();
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&b, &big).unwrap();
        let out = d.path().join("x.tar");
        write(&out, &[&a, &b], &mut |_, _| {}).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(bytes.len() % 512, 0);
        // Deterministic.
        let out2 = d.path().join("y.tar");
        write(&out2, &[&a, &b], &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read(&out2).unwrap(), bytes);

        let bun = Bundle::index(&[Span::whole(&out).unwrap()]).unwrap();
        assert_eq!(bun.members.len(), 2);
        assert_eq!(read_member(bun.get("pack.json").unwrap()), b"{}");
        let m = bun.get("system.img.zst").unwrap();
        assert_eq!(m.size, big.len() as u64);
        assert_eq!(read_member(m), big);

        // Split into parts at odd sizes: members cross part boundaries.
        let mut parts = Vec::new();
        for (i, c) in bytes.chunks(7_777).enumerate() {
            let p = d.path().join(format!("x.tar.{:03}", i + 1));
            std::fs::write(&p, c).unwrap();
            parts.push(Span::whole(&p).unwrap());
        }
        let bun = Bundle::index(&parts).unwrap();
        let m = bun.get("system.img.zst").unwrap();
        assert!(m.spans.len() > 1);
        assert_eq!(read_member(m), big);
    }

    #[test]
    fn make_splits_only_when_too_large() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.bin");
        std::fs::write(&a, vec![7u8; 5000]).unwrap();
        let out = d.path().join("pack.tar");
        let e = make(&out, &[&a], 1 << 20, 2048, &mut |_, _| {}).unwrap();
        assert!(e.get("parts").is_none() && out.exists());
        let e = make(&out, &[&a], 4096, 2048, &mut |_, _| {}).unwrap();
        let parts = e["parts"].as_array().unwrap();
        assert!(!out.exists());
        assert_eq!(
            parts.len() as u64,
            e["size"].as_u64().unwrap().div_ceil(2048)
        );
        let spans: Vec<Span> = parts
            .iter()
            .map(|p| Span::whole(&d.path().join(p["file"].as_str().unwrap())).unwrap())
            .collect();
        let bun = Bundle::index(&spans).unwrap();
        assert_eq!(read_member(bun.get("a.bin").unwrap()), vec![7u8; 5000]);
    }

    #[test]
    fn reads_tars_from_other_tools_and_rejects_damage() {
        // GNU tar's long-name record ('L', named "././@LongLink" there).
        let d = tempfile::tempdir().unwrap();
        let long = "n".repeat(120);
        let mut t = Vec::new();
        let mut gnu_long = header("@LongLink", long.len() as u64 + 1).unwrap();
        gnu_long[156] = b'L';
        gnu_long[148..156].fill(b' ');
        let sum: u64 = gnu_long.iter().map(|&b| b as u64).sum();
        gnu_long[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        t.extend_from_slice(&gnu_long);
        let mut name = long.clone().into_bytes();
        name.push(0);
        name.resize(512, 0);
        t.extend_from_slice(&name);
        t.extend_from_slice(&header("short", 3).unwrap());
        let mut data = b"abc".to_vec();
        data.resize(512, 0);
        t.extend_from_slice(&data);
        t.extend_from_slice(&[0u8; 1024]);
        let p = d.path().join("g.tar");
        std::fs::write(&p, &t).unwrap();
        let bun = Bundle::index(&[Span::whole(&p).unwrap()]).unwrap();
        assert_eq!(read_member(bun.get(&long).unwrap()), b"abc");

        // A flipped header byte fails the checksum; a cut-short file is caught.
        let mut bad = t.clone();
        bad[1030] ^= 1; // in the second member's header
        std::fs::write(&p, &bad).unwrap();
        assert!(Bundle::index(&[Span::whole(&p).unwrap()]).is_err());
        for cut in [1100, 1024, 1536 + 512] {
            std::fs::write(&p, &t[..cut]).unwrap();
            assert!(
                Bundle::index(&[Span::whole(&p).unwrap()]).is_err(),
                "cut at {cut}"
            );
        }
    }
}
