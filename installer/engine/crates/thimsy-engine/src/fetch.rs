//! Downloads with resume and sha256 checks, local imports, and the cache.

use crate::archive::{MultiVolume, Span};
use crate::bundle::Bundle;
use crate::catalog::Artifact;
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::util;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where verified files live: `<workspace>/cache/<file>`, with a
/// `<file>.ok` marker recording the hash it was verified against.
/// `imports` are folders to look in, or files the user picked.
pub struct Cache {
    pub dir: PathBuf,
    pub imports: Vec<PathBuf>,
    /// Opened device packs; their members are used in place.
    pub bundles: Vec<Bundle>,
    /// Longest wait for the server's answer or for the next piece of a
    /// download before the attempt counts as stalled (and is resumed).
    pub read_timeout: Duration,
    /// Pause before retrying a download that made no progress (times the
    /// attempt number).
    pub retry_delay: Duration,
}

/// A file called `name` of `size` bytes among `imports`: picked directly,
/// or inside an import folder.
pub fn find_in(imports: &[PathBuf], name: &str, size: u64) -> Option<PathBuf> {
    imports.iter().find_map(|imp| {
        let cand = if imp.is_dir() {
            imp.join(name)
        } else if imp.file_name().is_some_and(|n| n == name) {
            imp.clone()
        } else {
            return None;
        };
        fs::metadata(&cand)
            .is_ok_and(|m| m.is_file() && m.len() == size)
            .then_some(cand)
    })
}

/// Files the user picked (not folders), in the order given.
pub fn picked_files(imports: &[PathBuf]) -> Vec<&PathBuf> {
    imports.iter().filter(|p| p.is_file()).collect()
}

fn file_len(p: &Path) -> Option<u64> {
    fs::metadata(p)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len())
}

/// The unpacked file `name` of `size` bytes: by name in an import folder or
/// among the picked files, or a picked file of exactly that size under any
/// name (checked by sha256 once used).
pub fn find_unpacked(imports: &[PathBuf], name: &str, size: u64) -> Option<PathBuf> {
    find_in(imports, name, size).or_else(|| {
        picked_files(imports)
            .into_iter()
            .find(|p| file_len(p) == Some(size))
            .cloned()
    })
}

/// The device pack `a` from the user's files, used where it is: one picked
/// file of its size (any name), the file in an import folder, or its parts.
/// Parts are matched to picked files by name, then the rest by size in the
/// order they were picked, then by name in import folders. When picked
/// files stand for some parts but not all, that's an error (`bad_input`
/// naming what's missing): picked parts never mix with downloads.
pub fn local_bundle(imports: &[PathBuf], a: &Artifact) -> Result<Option<Vec<PathBuf>>> {
    let picked = picked_files(imports);
    if let Some(p) = picked.iter().find(|p| file_len(p) == Some(a.size)) {
        return Ok(Some(vec![(*p).clone()]));
    }
    if let Some(p) = find_in(imports, &a.file, a.size) {
        return Ok(Some(vec![p]));
    }
    if a.parts.is_empty() {
        return Ok(None);
    }
    let mut used = vec![false; picked.len()];
    let mut found: Vec<Option<PathBuf>> = vec![None; a.parts.len()];
    // By name among the picked files.
    for (i, part) in a.parts.iter().enumerate() {
        if let Some(k) = (0..picked.len()).find(|&k| {
            !used[k]
                && picked[k]
                    .file_name()
                    .is_some_and(|n| n == part.file.as_str())
                && file_len(picked[k]) == Some(part.size)
        }) {
            used[k] = true;
            found[i] = Some(picked[k].clone());
        }
    }
    // Renamed parts: by size, in the order picked.
    for (i, part) in a.parts.iter().enumerate() {
        if found[i].is_some() {
            continue;
        }
        if let Some(k) =
            (0..picked.len()).find(|&k| !used[k] && file_len(picked[k]) == Some(part.size))
        {
            used[k] = true;
            found[i] = Some(picked[k].clone());
        }
    }
    let from_picks = used.iter().any(|&u| u);
    // In import folders, by name.
    let folders: Vec<PathBuf> = imports.iter().filter(|p| p.is_dir()).cloned().collect();
    for (i, part) in a.parts.iter().enumerate() {
        if found[i].is_none() {
            found[i] = find_in(&folders, &part.file, part.size);
        }
    }
    if found.iter().all(Option::is_some) {
        return Ok(Some(found.into_iter().flatten().collect()));
    }
    if !from_picks {
        return Ok(None);
    }
    let missing: Vec<String> = a
        .parts
        .iter()
        .zip(&found)
        .filter(|(_, f)| f.is_none())
        .map(|(p, _)| format!("{} ({} bytes)", p.file, p.size))
        .collect();
    Err(EngineError::new(
        Code::BadInput,
        format!(
            "the device pack {} comes in {} parts and the files you chose don't include all of them; missing: {}",
            a.file,
            a.parts.len(),
            missing.join(", ")
        ),
    )
    .hint("Choose every part of the device pack together, then try again.")
    .into())
}

/// What `<file>.partial.json` records about a download in progress, so a
/// resume only ever continues the same file from the same server.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct PartialInfo {
    url: String,
    sha256: String,
    size: u64,
    etag: Option<String>,
    last_modified: Option<String>,
}

impl Cache {
    pub fn new(dir: PathBuf, imports: Vec<PathBuf>) -> Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Cache {
            dir,
            imports,
            bundles: Vec::new(),
            read_timeout: Duration::from_secs(60),
            retry_delay: Duration::from_secs(2),
        })
    }

    /// A file called `name` of `size` bytes among the imports: picked
    /// directly, or inside an import folder.
    pub fn find_import(&self, name: &str, size: u64) -> Option<PathBuf> {
        find_in(&self.imports, name, size)
    }

    fn bundle_member(&self, a: &Artifact) -> Option<&crate::bundle::Member> {
        self.bundles
            .iter()
            .find_map(|b| b.get(&a.file))
            .filter(|m| m.size == a.size)
    }

    /// Whether `a` comes from an opened device pack (nothing to delete).
    pub fn in_bundle(&self, a: &Artifact) -> bool {
        self.bundle_member(a).is_some()
    }

    fn check_spans(&self, a: &Artifact, spans: &[Span], what: &str, rep: &Reporter) -> Result<()> {
        let mut mv = MultiVolume::open_spans(spans)?;
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
            rep.progress_for("Checking {file}", &a.file, done, a.size);
        }
        let got = hex::encode(sha2::Digest::finalize(h));
        if done != a.size || got != a.sha256 {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!(
                    "{} ({what}) has sha256 {got}, expected {}",
                    a.file, a.sha256
                ),
            )
            .hint("The file is damaged or a different version. Delete it and download again.")
            .into());
        }
        Ok(())
    }

    /// A verified copy of the device pack `a`, used where it is: the user's
    /// files (see [`local_bundle`]), or else downloaded into the cache.
    pub fn get_in_place(&self, a: &Artifact, rep: &Reporter) -> Result<Vec<Span>> {
        if let Some(paths) = local_bundle(&self.imports, a)? {
            let spans = paths
                .iter()
                .map(|p| Span::whole(p))
                .collect::<Result<Vec<_>>>()?;
            rep.info(format!("Using {} from {}", a.file, spans[0].path.display()));
            self.check_spans(a, &spans, "picked file", rep)?;
            return Ok(spans);
        }
        self.get_all(a, rep)
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
            rep.progress_for("Checking {file}", &a.file, d, t)
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

    /// Byte ranges of a verified copy of `a`: inside an opened device pack,
    /// one file, or its parts in order (the whole is checked across the
    /// parts without joining them on disk).
    pub fn get_all(&self, a: &Artifact, rep: &Reporter) -> Result<Vec<Span>> {
        if let Some(m) = self.bundle_member(a) {
            let spans = m.spans.clone();
            self.check_spans(a, &spans, "in the device pack", rep)?;
            return Ok(spans);
        }
        if a.parts.is_empty() {
            return Ok(vec![Span::whole(&self.get(a, rep)?)?]);
        }
        let mut paths = Vec::new();
        for p in &a.parts {
            paths.push(self.get(p, rep)?);
        }
        let spans = paths
            .iter()
            .map(|p| Span::whole(p))
            .collect::<Result<Vec<_>>>()?;
        let marker = self.marker(a);
        if fs::read_to_string(&marker).is_ok_and(|m| m.trim() == a.sha256) {
            return Ok(spans);
        }
        let what = format!("joined from {} parts", a.parts.len());
        self.check_spans(a, &spans, &what, rep)?;
        util::write_atomic(&marker, a.sha256.as_bytes())?;
        Ok(spans)
    }

    /// Returns the path of a verified copy of `a`, fetching it if needed.
    pub fn get(&self, a: &Artifact, rep: &Reporter) -> Result<PathBuf> {
        let dest = self.dir.join(&a.file);
        if self.is_verified(a, &dest) {
            rep.info(format!("{} already downloaded", a.file));
            return Ok(dest);
        }
        let tmp = self.dir.join(format!("{}.import", a.file));
        // Inside an opened device pack.
        if let Some(m) = self.bundle_member(a) {
            {
                let mut i = MultiVolume::open_spans(&m.spans)?;
                let mut o = File::create(&tmp)?;
                util::copy_exact(&mut i, &mut o, a.size, &mut |d| {
                    rep.progress_for("Unpacking {file}", &a.file, d, a.size)
                })?;
            }
            self.verify(a, &tmp, rep)?;
            fs::rename(&tmp, &dest)?;
            return Ok(dest);
        }
        // A file the user picked or a folder of downloads (by hand or from a
        // draft release).
        if let Some(cand) = self.find_import(&a.file, a.size) {
            rep.info(format!("Importing {} from {}", a.file, cand.display()));
            copy_with_progress(&cand, &tmp, a, rep)?;
            self.verify(a, &tmp, rep)?;
            fs::rename(&tmp, &dest)?;
            return Ok(dest);
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
            .hint(a.note.as_deref().unwrap_or(
                "Download the file yourself, then choose it with Use a file I already downloaded.",
            ))
            .into());
        }
        let partial = self.dir.join(format!("{}.partial", a.file));
        let mut last_err = None;
        for url in &a.urls {
            // Attempts that got nothing; one that made progress doesn't
            // count (a long download may stall many times).
            let mut fruitless = 0u32;
            let mut tries = 0u32;
            let mut restarted = false;
            loop {
                tries += 1;
                let before = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
                match self.download(url, &partial, a, rep) {
                    Ok(resumed) => match self.verify(a, &partial, rep) {
                        Ok(()) => {
                            fs::rename(&partial, &dest)?;
                            let _ = fs::remove_file(sidecar(&partial));
                            return Ok(dest);
                        }
                        Err(e) => {
                            discard(&partial);
                            if resumed && !restarted {
                                // The file may have changed on the server
                                // between the pieces: once more from zero.
                                restarted = true;
                                rep.warn(format!(
                                    "{}: the resumed download doesn't match; downloading it again from the start",
                                    a.file
                                ));
                                continue;
                            }
                            last_err = Some(e);
                            break;
                        }
                    },
                    Err(e) => {
                        let missing = e.downcast_ref::<NotPublished>().is_some();
                        let after = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
                        if after <= before {
                            fruitless += 1;
                        }
                        rep.warn(format!(
                            "{url}: {e:#} (attempt {tries}; {fruitless} of 3 without progress)"
                        ));
                        last_err = Some(e);
                        if missing || fruitless >= 3 || tries >= 100 {
                            break; // asking again won't help
                        }
                        std::thread::sleep(self.retry_delay * fruitless.max(1));
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
        .hint(a.note.as_deref().unwrap_or(
            "Check the internet connection, or download the file yourself and choose it with Use a file I already downloaded.",
        ))
        .into())
    }

    /// Downloads `url` to `partial`, resuming from its current length when
    /// its `.partial.json` shows it's the same file from the same server
    /// (else starting over). Returns whether it continued earlier bytes.
    fn download(&self, url: &str, partial: &Path, a: &Artifact, rep: &Reporter) -> Result<bool> {
        if let Some(local) = url.strip_prefix("file://") {
            copy_with_progress(Path::new(local), partial, a, rep)?;
            return Ok(false);
        }
        let info_path = sidecar(partial);
        let mut info: PartialInfo = fs::read(&info_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut have = fs::metadata(partial).map(|m| m.len()).unwrap_or(0);
        let same = info.url == url && info.sha256 == a.sha256 && info.size == a.size;
        if have > 0 && (!same || have > a.size) {
            rep.info(format!(
                "Starting {} again: the partial download doesn't belong to this file",
                a.file
            ));
            discard(partial);
            have = 0;
        }
        if have == 0 || !same {
            info = PartialInfo {
                url: url.to_string(),
                sha256: a.sha256.clone(),
                size: a.size,
                etag: None,
                last_modified: None,
            };
        }
        if have == a.size {
            return Ok(true);
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(self.read_timeout))
            .timeout_per_read(Some(self.read_timeout))
            .http_status_as_error(false)
            .user_agent(format!("thimsy-installer/{}", crate::VERSION))
            .build()
            .into();
        // The bytes as stored: a compressed transfer can't be resumed by
        // byte offset.
        let mut req = agent.get(url).header("Accept-Encoding", "identity");
        if have > 0 {
            req = req.header("Range", &format!("bytes={have}-"));
            // Only continue if the server still has the same file.
            if let Some(v) = info.etag.as_ref().or(info.last_modified.as_ref()) {
                req = req.header("If-Range", v);
            }
        }
        let resp = req.call().with_context(|| format!("connecting to {url}"))?;
        let status = resp.status().as_u16();
        let header = |name: &str| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let etag = header("etag");
        let last_modified = header("last-modified");
        let resumed = match status {
            206 => {
                let range = header("content-range").unwrap_or_default();
                let changed = info.etag.is_some() && etag.is_some() && info.etag != etag;
                if content_range_start(&range) != Some((have, a.size)) || changed {
                    // Not the bytes asked for, or a different file now.
                    discard(partial);
                    anyhow::bail!(
                        "the server sent {range:?} for a resume at byte {have}; starting over"
                    );
                }
                true
            }
            200 => false,
            416 => {
                discard(partial);
                anyhow::bail!("the server can't resume at byte {have} (HTTP 416); starting over");
            }
            404 | 410 => return Err(NotPublished.into()),
            _ => anyhow::bail!("server answered HTTP {status}"),
        };
        if etag.is_some() || last_modified.is_some() || !resumed {
            info.etag = etag;
            info.last_modified = last_modified;
        }
        util::write_atomic(&info_path, &serde_json::to_vec(&info)?)?;
        let (mut file, mut done) = if resumed {
            (OpenOptions::new().append(true).open(partial)?, have)
        } else {
            (File::create(partial)?, 0)
        };
        let mut body = resp.into_body().into_reader();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = body.read(&mut buf).context("download interrupted")?;
            if n == 0 {
                break;
            }
            if done + n as u64 > a.size {
                drop(file);
                discard(partial);
                anyhow::bail!("{} is larger than expected", a.file);
            }
            file.write_all(&buf[..n])?;
            done += n as u64;
            rep.progress_for("Downloading {file}", &a.file, done, a.size);
        }
        file.sync_all()?;
        if done != a.size {
            anyhow::bail!("download stopped at {done} of {} bytes", a.size);
        }
        Ok(resumed)
    }
}

/// `<file>.partial.json` next to `<file>.partial`.
fn sidecar(partial: &Path) -> PathBuf {
    let mut s = partial.as_os_str().to_owned();
    s.push(".json");
    PathBuf::from(s)
}

/// Removes a partial download and what's recorded about it.
fn discard(partial: &Path) {
    let _ = fs::remove_file(partial);
    let _ = fs::remove_file(sidecar(partial));
}

/// (start, total) of a `Content-Range: bytes START-END/TOTAL` header.
fn content_range_start(v: &str) -> Option<(u64, u64)> {
    let rest = v.trim().strip_prefix("bytes ")?;
    let (range, total) = rest.split_once('/')?;
    let (start, _) = range.split_once('-')?;
    Some((start.trim().parse().ok()?, total.trim().parse().ok()?))
}

fn copy_with_progress(src: &Path, dst: &Path, a: &Artifact, rep: &Reporter) -> Result<()> {
    let mut i = File::open(src)?;
    let mut o = File::create(dst)?;
    util::copy_exact(&mut i, &mut o, a.size, &mut |d| {
        rep.progress_for("Importing {file}", &a.file, d, a.size)
    })?;
    o.sync_all()?;
    Ok(())
}

/// The server has no file at the address (HTTP 404): typically a release
/// that is still a draft. Not retried.
#[derive(Debug)]
struct NotPublished;

impl std::fmt::Display for NotPublished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the file isn't public at this address (HTTP 404); its release may still be a draft"
        )
    }
}

impl std::error::Error for NotPublished {}

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
        crate::archive::MultiVolume::open_spans(&paths)
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
