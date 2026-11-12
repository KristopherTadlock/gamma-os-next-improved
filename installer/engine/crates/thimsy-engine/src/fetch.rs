//! Downloads with resume and sha256 checks, local imports, and the cache.

use crate::catalog::Artifact;
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::util;
use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where verified files live: `<workspace>/cache/<file>`, with a
/// `<file>.ok` marker recording the hash it was verified against.
pub struct Cache {
    pub dir: PathBuf,
    pub imports: Vec<PathBuf>,
}

impl Cache {
    pub fn new(dir: PathBuf, imports: Vec<PathBuf>) -> Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Cache { dir, imports })
    }

    fn marker(&self, a: &Artifact) -> PathBuf {
        self.dir.join(format!("{}.ok", a.file))
    }

    fn is_verified(&self, a: &Artifact, path: &Path) -> bool {
        let Ok(meta) = fs::metadata(path) else {
            return false;
        };
        if meta.len() != a.size {
            return false;
        }
        fs::read_to_string(self.marker(a)).is_ok_and(|m| m.trim() == a.sha256)
    }

    fn verify(&self, a: &Artifact, path: &Path, rep: &Reporter) -> Result<()> {
        let got = util::sha256_file(path, |d, t| {
            rep.progress(&format!("Checking {}", a.file), d, t)
        })?;
        if got != a.sha256 {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!("{} has sha256 {got}, expected {}", a.file, a.sha256),
            )
            .hint("The file is damaged or a different version. Delete it and download again.")
            .into());
        }
        util::write_atomic(&self.marker(a), a.sha256.as_bytes())?;
        Ok(())
    }

    /// Paths of a verified copy of `a`: one file, or its parts in order (the
    /// whole is checked across the parts without joining them on disk).
    pub fn get_all(&self, a: &Artifact, rep: &Reporter) -> Result<Vec<PathBuf>> {
        if a.parts.is_empty() {
            return Ok(vec![self.get(a, rep)?]);
        }
        let mut paths = Vec::new();
        for p in &a.parts {
            paths.push(self.get(p, rep)?);
        }
        let marker = self.marker(a);
        if fs::read_to_string(&marker).is_ok_and(|m| m.trim() == a.sha256) {
            return Ok(paths);
        }
        let mut mv = crate::archive::MultiVolume::open(&paths)?;
        let mut h = <sha2::Sha256 as sha2::Digest>::new();
        let mut buf = vec![0u8; util::CHUNK];
        let mut done = 0u64;
        loop {
            let n = mv.read(&mut buf)?;
            if n == 0 {
                break;
            }
            sha2::Digest::update(&mut h, &buf[..n]);
            done += n as u64;
            rep.progress(&format!("Checking {}", a.file), done, a.size);
        }
        let got = hex::encode(sha2::Digest::finalize(h));
        if done != a.size || got != a.sha256 {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!(
                    "{} (joined from {} parts) has sha256 {got}, expected {}",
                    a.file,
                    a.parts.len(),
                    a.sha256
                ),
            )
            .into());
        }
        util::write_atomic(&marker, a.sha256.as_bytes())?;
        Ok(paths)
    }

    /// Returns the path of a verified copy of `a`, fetching it if needed.
    pub fn get(&self, a: &Artifact, rep: &Reporter) -> Result<PathBuf> {
        let dest = self.dir.join(&a.file);
        if self.is_verified(a, &dest) {
            rep.info(format!("{} already downloaded", a.file));
            return Ok(dest);
        }
        // A user-supplied folder (files downloaded by hand or from a draft release).
        for dir in &self.imports {
            let cand = dir.join(&a.file);
            if fs::metadata(&cand).is_ok_and(|m| m.len() == a.size) {
                rep.info(format!("Importing {} from {}", a.file, dir.display()));
                let tmp = dest.with_extension("import");
                copy_with_progress(&cand, &tmp, a, rep)?;
                self.verify(a, &tmp, rep)?;
                fs::rename(&tmp, &dest)?;
                return Ok(dest);
            }
        }
        if dest.exists() && fs::metadata(&dest)?.len() == a.size {
            // Present but never verified (e.g. copied in by hand).
            if self.verify(a, &dest, rep).is_ok() {
                return Ok(dest);
            }
            fs::remove_file(&dest)?;
        }
        if a.urls.is_empty() {
            return Err(EngineError::new(
                Code::DownloadFailed,
                format!("{} has no download address", a.file),
            )
            .hint(
                a.note
                    .clone()
                    .unwrap_or_else(|| "Put the file in the import folder and try again.".into()),
            )
            .into());
        }
        let partial = self.dir.join(format!("{}.partial", a.file));
        let mut last_err = None;
        for url in &a.urls {
            for attempt in 1..=3 {
                match download(url, &partial, a, rep) {
                    Ok(()) => match self.verify(a, &partial, rep) {
                        Ok(()) => {
                            fs::rename(&partial, &dest)?;
                            return Ok(dest);
                        }
                        Err(e) => {
                            let _ = fs::remove_file(&partial);
                            last_err = Some(e);
                            break;
                        }
                    },
                    Err(e) => {
                        rep.warn(format!("{url}: {e:#} (attempt {attempt}/3)"));
                        last_err = Some(e);
                        std::thread::sleep(Duration::from_secs(2 * attempt));
                    }
                }
            }
        }
        let e = last_err.unwrap();
        if crate::error::code_of(&e).0 == Code::HashMismatch {
            return Err(e);
        }
        Err(EngineError::new(
            Code::DownloadFailed,
            format!("couldn't download {}: {e:#}", a.file),
        )
        .hint(a.note.clone().unwrap_or_else(|| {
            "Check the internet connection, or put the file in the import folder.".into()
        }))
        .into())
    }
}

fn copy_with_progress(src: &Path, dst: &Path, a: &Artifact, rep: &Reporter) -> Result<()> {
    let mut i = File::open(src)?;
    let mut o = File::create(dst)?;
    util::copy_exact(&mut i, &mut o, a.size, &mut |d| {
        rep.progress(&format!("Importing {}", a.file), d, a.size)
    })?;
    o.sync_all()?;
    Ok(())
}

/// Downloads `url` to `partial`, resuming from its current length.
fn download(url: &str, partial: &Path, a: &Artifact, rep: &Reporter) -> Result<()> {
    if let Some(local) = url.strip_prefix("file://") {
        return copy_with_progress(Path::new(local), partial, a, rep);
    }
    let have = fs::metadata(partial).map(|m| m.len()).unwrap_or(0);
    if have >= a.size && have > 0 {
        return Ok(());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(format!("thimsy-installer/{}", crate::VERSION))
        .build()
        .into();
    let mut req = agent.get(url);
    if have > 0 {
        req = req.header("Range", &format!("bytes={have}-"));
    }
    let resp = req.call().with_context(|| format!("connecting to {url}"))?;
    let status = resp.status().as_u16();
    let (mut file, mut done) = match status {
        206 => (OpenOptions::new().append(true).open(partial)?, have),
        200 => (File::create(partial)?, 0),
        416 if have == a.size => return Ok(()),
        _ => anyhow::bail!("server answered HTTP {status}"),
    };
    let mut body = resp.into_body().into_reader();
    let mut buf = vec![0u8; 1 << 20];
    let label = format!("Downloading {}", a.file);
    loop {
        let n = body.read(&mut buf).context("download interrupted")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        if done > a.size {
            anyhow::bail!("{} is larger than expected", a.file);
        }
        rep.progress(&label, done, a.size);
    }
    file.sync_all()?;
    if done != a.size {
        anyhow::bail!("download stopped at {done} of {} bytes", a.size);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::MemorySink;
    use std::sync::Arc;

    fn art(file: &str, data: &[u8], urls: Vec<String>) -> Artifact {
        Artifact {
            id: file.into(),
            file: file.into(),
            size: data.len() as u64,
            sha256: util::sha256_hex(data),
            urls,
            unpack: None,
            note: None,
            parts: vec![],
        }
    }

    #[test]
    fn split_artifacts_are_fetched_per_part_and_checked_whole() {
        let ws = tempfile::tempdir().unwrap();
        let imp = tempfile::tempdir().unwrap();
        let data: Vec<u8> = (0..30_000u32).map(|i| (i % 241) as u8).collect();
        let mut parts = Vec::new();
        for (i, c) in data.chunks(12_000).enumerate() {
            let name = format!("big.zst.{:03}", i + 1);
            std::fs::write(imp.path().join(&name), c).unwrap();
            parts.push(art(&name, c, vec![]));
        }
        let rep = Reporter::new(Arc::new(MemorySink::default()));
        let cache = Cache::new(ws.path().join("cache"), vec![imp.path().to_path_buf()]).unwrap();
        let mut whole = art("big.zst", &data, vec![]);
        whole.parts = parts;
        let paths = cache.get_all(&whole, &rep).unwrap();
        assert_eq!(paths.len(), 3);
        let mut joined = Vec::new();
        crate::archive::MultiVolume::open(&paths)
            .unwrap()
            .read_to_end(&mut joined)
            .unwrap();
        assert_eq!(joined, data);
        // Whole-file hash is enforced even when every part matches its own.
        whole.sha256 = "0".repeat(64);
        let cache2 = Cache::new(ws.path().join("cache2"), vec![imp.path().to_path_buf()]).unwrap();
        assert!(cache2.get_all(&whole, &rep).is_err());
    }

    #[test]
    fn import_then_cache_hit_and_bad_hash() {
        let ws = tempfile::tempdir().unwrap();
        let imp = tempfile::tempdir().unwrap();
        let data = b"hello thimsy".repeat(1000);
        std::fs::write(imp.path().join("a.bin"), &data).unwrap();
        let rep = Reporter::new(Arc::new(MemorySink::default()));
        let cache = Cache::new(ws.path().join("cache"), vec![imp.path().to_path_buf()]).unwrap();
        let a = art("a.bin", &data, vec![]);
        let p = cache.get(&a, &rep).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), data);
        // second call: cache hit even after the import folder is gone
        drop(imp);
        assert_eq!(cache.get(&a, &rep).unwrap(), p);
        // wrong hash from a file:// URL
        let src = ws.path().join("src.bin");
        std::fs::write(&src, b"other").unwrap();
        let mut bad = art("b.bin", b"other", vec![format!("file://{}", src.display())]);
        bad.sha256 = "0".repeat(64);
        let err = cache.get(&bad, &rep).unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch);
        // no URL and no import → download_failed with a hint
        let none = art("c.bin", b"x", vec![]);
        let err = cache.get(&none, &rep).unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::DownloadFailed);
    }
}
