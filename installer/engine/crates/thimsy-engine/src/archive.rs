//! 7-Zip archives, including password-protected, multi-volume ones
//! (`name.7z.001`, `.002`, ...), as GammaOS publishes its releases.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// A byte range of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub path: PathBuf,
    pub offset: u64,
    pub len: u64,
}

impl Span {
    /// The whole of `path`.
    pub fn whole(path: &Path) -> Result<Span> {
        let len = std::fs::metadata(path)
            .with_context(|| format!("opening {}", path.display()))?
            .len();
        Ok(Span {
            path: path.to_path_buf(),
            offset: 0,
            len,
        })
    }
}

/// The ranges of `spans` (laid end to end) covering `[start, start + len)`.
pub fn sub_spans(spans: &[Span], start: u64, len: u64) -> Vec<Span> {
    let mut out = Vec::new();
    let (mut at, end) = (0u64, start + len);
    for s in spans {
        let (a, b) = (at.max(start), (at + s.len).min(end));
        if a < b {
            out.push(Span {
                path: s.path.clone(),
                offset: s.offset + (a - at),
                len: b - a,
            });
        }
        at += s.len;
    }
    out
}

/// Read + Seek over several files, or ranges of files, laid end to end.
pub struct MultiVolume {
    parts: Vec<(File, u64, u64)>, // file, offset in it, length
    starts: Vec<u64>,
    len: u64,
    pos: u64,
}

impl MultiVolume {
    pub fn open(paths: &[PathBuf]) -> Result<Self> {
        let spans = paths
            .iter()
            .map(|p| Span::whole(p))
            .collect::<Result<Vec<_>>>()?;
        Self::open_spans(&spans)
    }
    pub fn open_spans(spans: &[Span]) -> Result<Self> {
        let mut parts = Vec::new();
        let mut starts = Vec::new();
        let mut len = 0;
        for s in spans {
            let f = File::open(&s.path).with_context(|| format!("opening {}", s.path.display()))?;
            starts.push(len);
            len += s.len;
            parts.push((f, s.offset, s.len));
        }
        if parts.is_empty() {
            bail!("no archive volumes given");
        }
        Ok(MultiVolume {
            parts,
            starts,
            len,
            pos: 0,
        })
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Read for MultiVolume {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        // The last part starting at or before pos (skipping empty ones).
        let i = self.starts.partition_point(|&s| s <= self.pos) - 1;
        let (f, off, l) = &mut self.parts[i];
        let within = self.pos - self.starts[i];
        let n = ((*l - within) as usize).min(buf.len());
        f.seek(SeekFrom::Start(*off + within))?;
        let got = f.read(&mut buf[..n])?;
        self.pos += got as u64;
        Ok(got)
    }
}

impl Seek for MultiVolume {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let new = match to {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(d) => self.len as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if new < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }
        self.pos = new as u64;
        Ok(self.pos)
    }
}

/// Extracts the entry named `want` (exact name or basename) from the archive
/// formed by `volumes` into `out`. Returns its size.
pub fn extract_one(
    volumes: &[PathBuf],
    password: Option<&str>,
    want: &str,
    out: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<u64> {
    let reader = MultiVolume::open(volumes)?;
    let pw = sevenz_rust2::Password::from(password.unwrap_or(""));
    let mut ar = sevenz_rust2::ArchiveReader::new(reader, pw)
        .context("opening 7z archive (wrong password?)")?;
    ar.set_thread_count(
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
            .min(8),
    );
    let mut found = None;
    let mut io_err: Option<anyhow::Error> = None;
    ar.for_each_entries(|entry, rd| {
        let name = entry.name();
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if entry.is_directory() || (name != want && base != want) {
            return Ok(true);
        }
        let total = entry.size();
        let res = (|| -> Result<u64> {
            let mut o = File::create(out).with_context(|| format!("creating {}", out.display()))?;
            let mut buf = vec![0u8; 4 << 20];
            let mut done = 0u64;
            loop {
                let n = rd.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                o.write_all(&buf[..n])?;
                done += n as u64;
                progress(done, total);
            }
            o.sync_all()?;
            Ok(done)
        })();
        match res {
            Ok(n) => found = Some(n),
            Err(e) => io_err = Some(e),
        }
        Ok(false)
    })
    .context("decompressing 7z archive")?;
    if let Some(e) = io_err {
        return Err(e);
    }
    found.with_context(|| format!("{want} is not in the archive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multivolume_reads_across_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let mut paths = Vec::new();
        for (i, chunk) in data.chunks(3000).enumerate() {
            let p = dir.path().join(format!("x.7z.{:03}", i + 1));
            std::fs::write(&p, chunk).unwrap();
            paths.push(p);
        }
        let mut mv = MultiVolume::open(&paths).unwrap();
        assert_eq!(mv.len(), 10_000);
        let mut all = Vec::new();
        mv.read_to_end(&mut all).unwrap();
        assert_eq!(all, data);
        mv.seek(SeekFrom::Start(2990)).unwrap();
        let mut b = [0u8; 20];
        mv.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &data[2990..3010]);
    }

    #[test]
    fn extracts_from_encrypted_split_archive() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("hello.pac");
        let payload: Vec<u8> = (0..200_000u32).map(|i| (i * 7 % 256) as u8).collect();
        std::fs::write(&src, &payload).unwrap();
        let archive = dir.path().join("a.7z");
        let mut w = sevenz_rust2::ArchiveWriter::create(&archive).unwrap();
        w.set_content_methods(vec![
            sevenz_rust2::encoder_options::AesEncoderOptions::new("secret".into()).into(),
            sevenz_rust2::EncoderMethod::LZMA2.into(),
        ]);
        w.push_source_path(&src, |_| true).unwrap();
        w.finish().unwrap();
        // split into volumes
        let bytes = std::fs::read(&archive).unwrap();
        let mut vols = Vec::new();
        for (i, c) in bytes.chunks(bytes.len() / 3 + 1).enumerate() {
            let p = dir.path().join(format!("a.7z.{:03}", i + 1));
            std::fs::write(&p, c).unwrap();
            vols.push(p);
        }
        let out = dir.path().join("out.pac");
        let n = extract_one(&vols, Some("secret"), "hello.pac", &out, &mut |_, _| {}).unwrap();
        assert_eq!(n, payload.len() as u64);
        assert_eq!(std::fs::read(&out).unwrap(), payload);
        assert!(extract_one(&vols, Some("wrong"), "hello.pac", &out, &mut |_, _| {}).is_err());
    }
}
