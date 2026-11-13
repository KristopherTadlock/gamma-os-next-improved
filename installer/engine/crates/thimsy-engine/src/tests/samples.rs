//! The JSON Lines samples in installer/docs/protocol-samples: the real output
//! of `cli::run` (what the binary prints) for each case, with temporary paths
//! and times replaced by placeholders, log lines kept but not compared, and
//! only each progress label's final event kept (the others depend on
//! timing). `THIMSY_UPDATE_SAMPLES=1 cargo test samples` rewrites the files;
//! otherwise every line is compared with them field by field.
#![cfg(unix)]

use super::build::Fixture;
use crate::cli::{self, Command, Global};
use crate::events::{Event, Reporter, Sink};
use crate::{job, tools};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Events and output lines, in the order the app would read them.
#[derive(Default)]
struct Capture(Mutex<Vec<Value>>);

impl Sink for Capture {
    fn emit(&self, event: &Event) {
        self.0
            .lock()
            .unwrap()
            .push(serde_json::to_value(event).unwrap());
    }
}

fn global(catalog: Option<&Path>, host: &str) -> Global {
    Global {
        json: true,
        catalog: catalog.map(Path::to_path_buf),
        lang: "en".into(),
        host: Some(host.into()),
    }
}

fn run(g: &Global, cmd: Command) -> (i32, Vec<Value>) {
    let cap = Arc::new(Capture::default());
    let rep = Reporter::new(cap.clone());
    let sink = cap.clone();
    let code = cli::run(g, cmd, &rep, &mut |v| sink.0.lock().unwrap().push(v));
    let lines = std::mem::take(&mut *cap.0.lock().unwrap());
    (code, lines)
}

/// `run-YYYYMMDD-HHMMSS[-N]` → `run-<time>`, `YYYY-MM-DDTHH:MM:SSZ` → `<time>`.
fn mask_times(s: &str) -> String {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let stamp = b.len() >= i + 19
            && &b[i..i + 4] == b"run-"
            && digits(i + 4..i + 12)
            && b[i + 12] == b'-'
            && digits(i + 13..i + 19);
        let iso = b.len() >= i + 20
            && digits(i..i + 4)
            && b[i + 4] == b'-'
            && b[i + 10] == b'T'
            && b[i + 19] == b'Z'
            && digits(i + 11..i + 13);
        if stamp {
            out.push_str("run-<time>");
            i += 19;
            // `-1`, `-2`... when two runs start in the same second: timing,
            // not protocol.
            if b.get(i) == Some(&b'-') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                i += 1;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
            }
        } else if iso {
            out.push_str("<time>");
            i += 20;
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

fn normalise_value(v: &mut Value, paths: &[(String, &str)]) {
    match v {
        Value::String(s) => {
            let mut t = s.clone();
            for (from, to) in paths {
                t = t.replace(from.as_str(), to);
            }
            *s = mask_times(&t);
        }
        Value::Array(a) => a.iter_mut().for_each(|x| normalise_value(x, paths)),
        Value::Object(o) => o.values_mut().for_each(|x| normalise_value(x, paths)),
        _ => {}
    }
}

/// Placeholders for temporary paths, and only final progress events.
fn normalise(lines: Vec<Value>, paths: &[(&Path, &str)]) -> Vec<Value> {
    let mut subs: Vec<(String, &str)> = Vec::new();
    for (p, to) in paths {
        subs.push((p.display().to_string(), to));
        if let Ok(c) = std::fs::canonicalize(p) {
            subs.push((c.display().to_string(), to));
        }
    }
    subs.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    let mut out: Vec<Value> = Vec::new();
    for mut v in lines {
        if v["type"] == "progress" {
            let (done, total) = (v["done"].as_u64().unwrap(), v["total"].as_u64().unwrap());
            if total == 0 || done < total {
                continue;
            }
        }
        normalise_value(&mut v, &subs);
        // A label finishing twice in a row (e.g. checked again) shows once.
        if v["type"] == "progress" && out.last() == Some(&v) {
            continue;
        }
        out.push(v);
    }
    out
}

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/protocol-samples")
}

/// Writes (THIMSY_UPDATE_SAMPLES=1) or checks one sample file.
fn sample(name: &str, lines: &[Value]) {
    let path = samples_dir().join(name);
    let text: String = lines
        .iter()
        .map(|v| format!("{}\n", serde_json::to_string(v).unwrap()))
        .collect();
    if std::env::var_os("THIMSY_UPDATE_SAMPLES").is_some() {
        std::fs::create_dir_all(samples_dir()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; run THIMSY_UPDATE_SAMPLES=1 cargo test samples",
            path.display()
        )
    });
    let stale = format!(
        "{name} is out of date: run THIMSY_UPDATE_SAMPLES=1 cargo test samples and review the diff"
    );
    // Log lines are free text for the diagnostic bundle: present, not compared.
    let keep = |v: &&Value| v["type"] != "log";
    let have: Vec<Value> = have
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let want: Vec<&Value> = lines.iter().filter(keep).collect();
    let have: Vec<&Value> = have.iter().filter(keep).collect();
    assert_eq!(have.len(), want.len(), "{stale}");
    for (h, w) in have.iter().zip(&want) {
        let (Value::Object(h), Value::Object(w)) = (h, w) else {
            panic!("{stale}: not an object");
        };
        let mut hk: Vec<&String> = h.keys().collect();
        let mut wk: Vec<&String> = w.keys().collect();
        hk.sort();
        wk.sort();
        assert_eq!(hk, wk, "{stale}: fields of a {} line", w["type"]);
        for k in wk {
            assert_eq!(h[k], w[k], "{stale}: {k} of a {} line", w["type"]);
        }
    }
}

/// The fixture's catalog as files, for `--catalog`.
fn catalog_dir(fx: &Fixture) -> PathBuf {
    let dir = fx.dir.path().join("catalog");
    for (name, text) in &fx.catalog.files {
        let p = dir.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

fn fake_tools(env: &[(&str, &str)]) {
    let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fake-tools");
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = Some(fake));
    tools::test::ENV.with(|e| {
        *e.borrow_mut() = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    });
}

fn no_hooks() {
    tools::test::ENV.with(|e| e.borrow_mut().clear());
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = None);
    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = None);
    crate::disk::test::PROBE.with(|p| *p.borrow_mut() = None);
}

#[test]
fn samples_catalog_resolve_space_detect_drivers() {
    let g = global(None, "linux");
    let (code, l) = run(&g, Command::Catalog);
    assert_eq!(code, 0);
    sample("catalog.jsonl", &normalise(l, &[]));
    let (code, l) = run(
        &g,
        Command::Resolve {
            device: "anbernic-rg-cube".into(),
            profile: "heavy".into(),
            set: vec![],
        },
    );
    assert_eq!(code, 0);
    sample("resolve-rg-cube-heavy.jsonl", &normalise(l, &[]));

    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("Thimsy workspace");
    let space = || Command::Space {
        device: "anbernic-rg-cube".into(),
        profile: "heavy".into(),
        workspace: ws.clone(),
        import: vec![],
        delete_downloads: false,
    };
    for (fat32, name) in [(false, "space.jsonl"), (true, "space-fat32.jsonl")] {
        crate::disk::test::PROBE.with(|p| {
            *p.borrow_mut() = Some(Ok(crate::disk::Probe {
                free: 500_000_000_000,
                fat32,
            }))
        });
        let (code, l) = run(&g, space());
        assert_eq!(code, 0);
        assert_eq!(l[0]["ok"], !fat32);
        sample(name, &normalise(l, &[(&ws, "<workspace>")]));
    }

    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = Some(vec![(0x1782, 0x4d00, None)]));
    let (code, l) = run(&g, Command::Detect { watch: false });
    assert_eq!(code, 0);
    sample("detect.jsonl", &normalise(l, &[]));

    // The Windows driver install, declined at the UAC prompt.
    crate::drivers::test::ELEVATED_EXIT.with(|c| c.set(Some(1223)));
    let (code, l) = run(
        &global(None, "windows"),
        Command::Drivers {
            device: "anbernic-rg-cube".into(),
            install: true,
        },
    );
    crate::drivers::test::ELEVATED_EXIT.with(|c| c.set(None));
    assert_eq!(code, 1);
    assert_eq!(l.last().unwrap()["status"], "missing");
    sample("drivers-failed.jsonl", &normalise(l, &[]));
    no_hooks();
}

#[test]
fn samples_build_flash_and_errors() {
    let fx = super::build::good_fixture();
    let cat_dir = catalog_dir(&fx);
    let g = global(Some(&cat_dir), "linux");
    let ws = fx.dir.path().join("ws");
    let fixture_dir = fx.dir.path().to_path_buf();
    let paths = [
        (ws.as_path(), "<workspace>"),
        (fixture_dir.as_path(), "<fixture>"),
    ];
    let job = job::job_dir(&ws, "testdev", "heavy");
    let build = || Command::Build {
        device: "testdev".into(),
        profile: "heavy".into(),
        workspace: ws.clone(),
        import: vec![],
        set: vec![],
        delete_downloads: false,
    };
    let (code, l) = run(&g, build());
    assert_eq!(code, 0, "{l:#?}");
    sample("build.jsonl", &normalise(l, &paths));

    let flash = |mode: &str, dry_run: bool| Command::Flash {
        job: job.clone(),
        mode: mode.into(),
        dry_run,
    };
    let (code, l) = run(&g, flash("full", true));
    assert_eq!(code, 0);
    sample("flash-dry-run.jsonl", &normalise(l, &paths));

    // What the app runs in the background after the build: the install
    // after it skips the checks it already did.
    let (code, l) = run(
        &g,
        Command::Prepare {
            job: job.clone(),
            mode: "full".into(),
        },
    );
    assert_eq!(code, 0, "{l:#?}");
    sample("prepare.jsonl", &normalise(l, &paths));

    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = Some(vec![]));
    fake_tools(&[]);
    let (code, l) = run(&g, flash("full", false));
    assert_eq!(code, 0, "{l:#?}");
    sample("flash-full.jsonl", &normalise(l, &paths));

    // Another model on the same USB ID: refused before the first write.
    let parts = fx.dir.path().join("other-device.xml");
    std::fs::write(&parts, "<Partitions>\n    <Partition id=\"prodnv\" size=\"1\"/>\n    <Partition id=\"super\" size=\"10\"/>\n    <Partition id=\"userdata\" size=\"0xffffffff\"/>\n</Partitions>\n").unwrap();
    fake_tools(&[("FAKE_SPD_PARTS", &parts.display().to_string())]);
    let (code, l) = run(&g, flash("full", false));
    assert_eq!(code, 1);
    sample("error-wrong-device.jsonl", &normalise(l, &paths));

    // The device never shows up in download mode.
    fake_tools(&[("FAKE_SPD_NO_DEVICE", "1")]);
    let (code, l) = run(&g, flash("full", false));
    assert_eq!(code, 1);
    sample("error-device-not-found.jsonl", &normalise(l, &paths));

    // An update on a device that doesn't run the matching base.
    fake_tools(&[("FAKE_SPD_UPDATE_MISMATCH", "boot_a")]);
    let (code, l) = run(&g, flash("update", false));
    assert_eq!(code, 1);
    sample("error-full-install-needed.jsonl", &normalise(l, &paths));

    // Another run holds the workspace.
    let lock = job::lock_workspace(&ws).unwrap();
    let (code, l) = run(&g, build());
    drop(lock);
    assert_eq!(code, 1);
    sample("error-busy.jsonl", &normalise(l, &paths));

    // What clean can remove, backups listed per run.
    let (code, l) = run(
        &g,
        Command::Clean {
            workspace: ws.clone(),
            downloads: false,
            image: false,
            scratch: false,
            backup: false,
            paths: vec![],
            dry_run: true,
        },
    );
    assert_eq!(code, 0);
    sample("clean-dry-run.jsonl", &normalise(l, &paths));
    no_hooks();
}
