//! Downloads against a tiny local HTTP server: resume with Range, servers
//! that ignore Range, errors, and size limits.

use crate::catalog::Artifact;
use crate::events::{MemorySink, Reporter};
use crate::fetch::Cache;
use crate::testutil::pattern;
use crate::util::sha256_hex;
use crate::Code;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Mode {
    Ranges,
    IgnoreRanges,
    CutAfter(usize),
    NotFound,
    /// The first response sends this much, then goes quiet (connection
    /// left open); later ones serve ranges.
    StallAfter(usize),
    /// Resumes are answered from the wrong offset.
    WrongRange,
}

/// Serves `data` at any path. Returns the base URL and a request counter.
fn serve(data: Vec<u8>, mode: Mode) -> (String, Arc<AtomicUsize>) {
    let (url, hits, _) = serve_versions(vec![data], mode);
    (url, hits)
}

/// Like [`serve`], serving `versions[n]` to the nth request (the last one
/// from then on): a file that changes on the server. Also returns every
/// request's header lines.
fn serve_versions(
    versions: Vec<Vec<u8>>,
    mode: Mode,
) -> (String, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    std::thread::spawn(move || {
        for (n, s) in l.incoming().enumerate() {
            let Ok(mut s) = s else { continue };
            h.fetch_add(1, Ordering::SeqCst);
            let data = versions[n.min(versions.len() - 1)].clone();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut range = None;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                seen2.lock().unwrap().push(line.trim().to_ascii_lowercase());
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                    range = v.trim().trim_end_matches('-').parse::<usize>().ok();
                }
            }
            let mode = match mode {
                // Only the first response is cut or stalls.
                Mode::CutAfter(_) if n > 0 => Mode::Ranges,
                Mode::StallAfter(_) if n > 0 => Mode::Ranges,
                Mode::StallAfter(cut) => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        data.len()
                    );
                    let _ = s
                        .write_all(head.as_bytes())
                        .and_then(|_| s.write_all(&data[..cut]));
                    let _ = s.flush();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(20));
                        drop(s);
                    });
                    continue;
                }
                m => m,
            };
            let _ = match (mode, range) {
                (Mode::WrongRange, Some(_)) => {
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 0-{}/{}\r\nConnection: close\r\n\r\n",
                        data.len(),
                        data.len() - 1,
                        data.len()
                    );
                    s.write_all(head.as_bytes())
                        .and_then(|_| s.write_all(&data))
                }
                (Mode::NotFound, _) => s.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
                (Mode::Ranges, Some(from)) | (Mode::CutAfter(_), Some(from)) => {
                    let body = &data[from..];
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        from,
                        data.len() - 1,
                        data.len()
                    );
                    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body))
                }
                (Mode::CutAfter(cut), None) => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        data.len()
                    );
                    s.write_all(head.as_bytes())
                        .and_then(|_| s.write_all(&data[..cut]))
                }
                _ => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        data.len()
                    );
                    s.write_all(head.as_bytes())
                        .and_then(|_| s.write_all(&data))
                }
            };
        }
    });
    (url, hits, seen)
}

fn art(data: &[u8], url: &str) -> Artifact {
    Artifact {
        id: "x".into(),
        file: "x.bin".into(),
        size: data.len() as u64,
        sha256: sha256_hex(data),
        urls: vec![format!("{url}/x.bin")],
        unpack: None,
        note: None,
        parts: vec![],
    }
}

fn rep() -> Reporter {
    Reporter::new(Arc::new(MemorySink::default()))
}

/// A cache that gives up on a quiet server quickly and retries at once.
fn quick_cache(dir: &std::path::Path) -> Cache {
    let mut c = Cache::new(dir.to_path_buf(), vec![]).unwrap();
    c.read_timeout = std::time::Duration::from_millis(300);
    c.retry_delay = std::time::Duration::from_millis(10);
    c
}

/// `<file>.partial.json` as the engine writes it for `url`.
fn sidecar_for(a: &Artifact, url: &str) -> String {
    serde_json::json!({"url": url, "sha256": a.sha256, "size": a.size, "etag": null, "last_modified": null})
        .to_string()
}

#[test]
fn a_stalled_download_times_out_and_resumes() {
    let data = pattern(400_000, 8);
    let (url, hits) = serve(data.clone(), Mode::StallAfter(100_000));
    let ws = tempfile::tempdir().unwrap();
    let cache = quick_cache(&ws.path().join("c"));
    let started = std::time::Instant::now();
    let p = cache.get(&art(&data, &url), &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), data);
    assert_eq!(hits.load(Ordering::SeqCst), 2, "one stall, one resume");
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn a_file_that_changed_between_pieces_is_downloaded_again_once() {
    let old = pattern(300_000, 9);
    let new = pattern(300_000, 10);
    // The first request gets the old file, cut off; later ones the new one.
    let mut cut = old.clone();
    cut.truncate(120_000);
    let (url, hits, seen) = serve_versions(vec![old, new.clone()], Mode::CutAfter(120_000));
    let ws = tempfile::tempdir().unwrap();
    let cache = quick_cache(&ws.path().join("c"));
    let p = cache.get(&art(&new, &url), &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), new);
    // Cut, resumed (wrong mix, refused), then once more from the start.
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    let seen = seen.lock().unwrap();
    assert!(
        seen.iter().any(|l| l == "accept-encoding: identity"),
        "{seen:?}"
    );
    assert!(!ws.path().join("c/x.bin.partial.json").exists());
}

#[test]
fn a_stale_or_foreign_partial_download_starts_over() {
    let data = pattern(200_000, 11);
    let (url, hits) = serve(data.clone(), Mode::Ranges);
    let ws = tempfile::tempdir().unwrap();
    let dir = ws.path().join("c");
    let cache = quick_cache(&dir);
    let a = art(&data, &url);
    // Longer than the file (an older, bigger version), recorded as ours.
    std::fs::write(dir.join("x.bin.partial"), pattern(250_000, 12)).unwrap();
    std::fs::write(dir.join("x.bin.partial.json"), sidecar_for(&a, &a.urls[0])).unwrap();
    assert_eq!(std::fs::read(cache.get(&a, &rep()).unwrap()).unwrap(), data);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    // A partial without a record of where it came from isn't resumed.
    std::fs::remove_file(dir.join("x.bin")).unwrap();
    std::fs::remove_file(dir.join("x.bin.ok")).unwrap();
    std::fs::write(dir.join("x.bin.partial"), pattern(50_000, 13)).unwrap();
    assert_eq!(std::fs::read(cache.get(&a, &rep()).unwrap()).unwrap(), data);
}

#[test]
fn a_resume_answered_from_the_wrong_offset_starts_over() {
    let data = pattern(200_000, 14);
    let (url, _) = serve(data.clone(), Mode::WrongRange);
    let ws = tempfile::tempdir().unwrap();
    let dir = ws.path().join("c");
    let cache = quick_cache(&dir);
    let a = art(&data, &url);
    std::fs::write(dir.join("x.bin.partial"), &data[..80_000]).unwrap();
    std::fs::write(dir.join("x.bin.partial.json"), sidecar_for(&a, &a.urls[0])).unwrap();
    assert_eq!(std::fs::read(cache.get(&a, &rep()).unwrap()).unwrap(), data);
}

#[test]
fn downloads_and_verifies() {
    let data = pattern(300_000, 1);
    let (url, _) = serve(data.clone(), Mode::Ranges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    let p = cache.get(&art(&data, &url), &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), data);
}

#[test]
fn resumes_a_partial_download_with_range() {
    let data = pattern(500_000, 2);
    let (url, _, seen) = serve_versions(vec![data.clone()], Mode::Ranges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    // Leave the first 200 KB behind as if a previous run was interrupted.
    let a = art(&data, &url);
    std::fs::write(ws.path().join("c/x.bin.partial"), &data[..200_000]).unwrap();
    std::fs::write(
        ws.path().join("c/x.bin.partial.json"),
        sidecar_for(&a, &a.urls[0]),
    )
    .unwrap();
    let p = cache.get(&a, &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), data);
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .any(|l| l == "range: bytes=200000-"));
}

#[test]
fn restarts_when_the_server_ignores_range() {
    let data = pattern(100_000, 3);
    let (url, _) = serve(data.clone(), Mode::IgnoreRanges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    let a = art(&data, &url);
    std::fs::write(ws.path().join("c/x.bin.partial"), &data[..50_000]).unwrap();
    std::fs::write(
        ws.path().join("c/x.bin.partial.json"),
        sidecar_for(&a, &a.urls[0]),
    )
    .unwrap();
    let p = cache.get(&a, &rep()).unwrap();
    assert_eq!(
        std::fs::read(p).unwrap(),
        data,
        "a 200 reply must replace, not append"
    );
}

#[test]
fn retries_a_cut_connection_and_resumes() {
    let data = pattern(400_000, 4);
    let (url, hits) = serve(data.clone(), Mode::CutAfter(150_000));
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    let p = cache.get(&art(&data, &url), &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), data);
    assert!(hits.load(Ordering::SeqCst) >= 2);
}

#[test]
fn http_errors_become_download_failed_with_a_hint() {
    let data = pattern(1000, 5);
    let (url, hits) = serve(data.clone(), Mode::NotFound);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    let mut a = art(&data, &url);
    a.note = Some("Get it from the release page.".into());
    let started = std::time::Instant::now();
    let err = cache.get(&a, &rep()).unwrap_err();
    let (code, hint) = crate::error::code_of(&err);
    assert_eq!(code, Code::DownloadFailed);
    assert_eq!(hint.as_deref(), Some("Get it from the release page."));
    // A missing file (a draft release) says so and isn't retried.
    assert!(
        format!("{err:#}").contains("may still be a draft"),
        "{err:#}"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn wrong_content_is_a_hash_mismatch_and_leaves_no_file() {
    let data = pattern(2000, 6);
    let (url, _) = serve(pattern(2000, 7), Mode::Ranges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    let err = cache.get(&art(&data, &url), &rep()).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch);
    assert!(!ws.path().join("c/x.bin").exists());
    assert!(!ws.path().join("c/x.bin.partial").exists());
}
