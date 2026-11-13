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
}

/// Serves `data` at any path. Returns the base URL and a request counter.
fn serve(data: Vec<u8>, mode: Mode) -> (String, Arc<AtomicUsize>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    std::thread::spawn(move || {
        for (n, s) in l.incoming().enumerate() {
            let Ok(mut s) = s else { continue };
            h.fetch_add(1, Ordering::SeqCst);
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut range = None;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                    range = v.trim().trim_end_matches('-').parse::<usize>().ok();
                }
            }
            let mode = match mode {
                Mode::CutAfter(_) if n > 0 => Mode::Ranges, // only the first response is cut
                m => m,
            };
            let _ = match (mode, range) {
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
    (url, hits)
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
    let (url, _) = serve(data.clone(), Mode::Ranges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    // Leave the first 200 KB behind as if a previous run was interrupted.
    std::fs::write(ws.path().join("c/x.bin.partial"), &data[..200_000]).unwrap();
    let p = cache.get(&art(&data, &url), &rep()).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), data);
}

#[test]
fn restarts_when_the_server_ignores_range() {
    let data = pattern(100_000, 3);
    let (url, _) = serve(data.clone(), Mode::IgnoreRanges);
    let ws = tempfile::tempdir().unwrap();
    let cache = Cache::new(ws.path().join("c"), vec![]).unwrap();
    std::fs::write(ws.path().join("c/x.bin.partial"), &data[..50_000]).unwrap();
    let p = cache.get(&art(&data, &url), &rep()).unwrap();
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
