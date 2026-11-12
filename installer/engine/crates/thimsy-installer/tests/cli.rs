//! The command line as the app sees it: JSON Lines shapes, exit codes and
//! error codes.

use serde_json::Value;
use std::process::Command;

fn run(args: &[&str]) -> (bool, Vec<Value>) {
    let out = Command::new(env!("CARGO_BIN_EXE_thimsy-installer"))
        .args(args)
        .output()
        .unwrap();
    let lines = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("bad JSON line {l:?}: {e}")))
        .collect();
    (out.status.success(), lines)
}

#[test]
fn catalog_lists_devices_profiles_and_host() {
    let (ok, l) = run(&["--json", "catalog"]);
    assert!(ok);
    assert_eq!(l.len(), 1, "one line");
    let c = &l[0];
    assert_eq!(c["type"], "catalog");
    let ids: Vec<&str> = c["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"anbernic-rg-cube"));
    let profiles: Vec<&str> = c["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(profiles, ["light", "medium", "heavy"]);
    assert!(["windows", "macos", "linux", "android"].contains(&c["host"].as_str().unwrap()));
}

#[test]
fn needs_windows_message_on_unsupported_hosts() {
    let (ok, l) = run(&["--json", "--host", "macos", "catalog"]);
    assert!(ok);
    let d = l[0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "anbernic-rg557")
        .unwrap();
    assert!(d["host_message"].as_str().unwrap().contains("Windows PC"));
    let cube = l[0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "anbernic-rg-cube")
        .unwrap();
    assert!(cube["host_message"].is_null());
}

#[test]
fn resolve_plan_shape() {
    let (ok, l) = run(&[
        "--json",
        "resolve",
        "--device",
        "anbernic-rg-cube",
        "--profile",
        "heavy",
    ]);
    assert!(ok);
    let p = &l[0];
    assert_eq!(p["type"], "plan");
    assert_eq!(p["available"], true);
    assert!(p["download_bytes"].as_u64().unwrap() > 4_000_000_000);
    for k in [
        "emulators",
        "apps",
        "cores",
        "excluded",
        "settings",
        "partitions",
    ] {
        assert!(p[k].is_array(), "{k}");
    }
    assert_eq!(
        p["journeys"]["returning"]["modes"],
        serde_json::json!(["update", "full"])
    );
    assert_eq!(p["guides"]["unlock"]["route"], "guided");
}

#[test]
fn errors_exit_nonzero_with_a_coded_event_and_result() {
    let ws = tempfile::tempdir().unwrap();
    let (ok, l) = run(&[
        "--json",
        "build",
        "--device",
        "anbernic-rg-cube",
        "--profile",
        "light",
        "--workspace",
        ws.path().to_str().unwrap(),
    ]);
    assert!(!ok);
    let err = l.iter().find(|e| e["type"] == "error").unwrap();
    assert_eq!(err["code"], "profile_unavailable");
    assert_eq!(l.last().unwrap()["type"], "result");
    assert_eq!(l.last().unwrap()["ok"], false);
    let (ok, l) = run(&[
        "--json",
        "resolve",
        "--device",
        "nope",
        "--profile",
        "heavy",
    ]);
    assert!(!ok);
    assert_eq!(l[0]["type"], "error");
}

#[test]
fn space_and_clean_on_an_empty_workspace() {
    let ws = tempfile::tempdir().unwrap();
    let w = ws.path().to_str().unwrap();
    let (ok, l) = run(&[
        "--json",
        "space",
        "--device",
        "anbernic-rg-cube",
        "--profile",
        "heavy",
        "--workspace",
        w,
    ]);
    assert!(ok);
    assert_eq!(l[0]["type"], "space");
    assert!(l[0]["need"].as_u64().unwrap() > 7_000_000_000);
    let (ok, l) = run(&["--json", "clean", "--workspace", w, "--dry-run"]);
    assert!(ok);
    assert_eq!(l[0]["items"], serde_json::json!([]));
}

#[test]
fn languages() {
    let (ok, l) = run(&["--json", "--lang", "ja", "catalog"]);
    assert!(ok);
    let heavy = l[0]["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "heavy")
        .unwrap();
    assert_ne!(heavy["summary"].as_str().unwrap(), "", "summary present");
    let (ok, _) = run(&["--json", "--lang", "zh_TW", "catalog"]);
    assert!(ok, "Flutter-style codes are accepted");
    let (ok, _) = run(&["--json", "--lang", "xx", "catalog"]);
    assert!(!ok);
}

#[test]
fn flash_without_a_build_is_an_error() {
    let ws = tempfile::tempdir().unwrap();
    let (ok, l) = run(&[
        "--json",
        "flash",
        "--job",
        ws.path().to_str().unwrap(),
        "--dry-run",
    ]);
    assert!(!ok);
    assert!(l.iter().any(|e| e["type"] == "error"));
}
