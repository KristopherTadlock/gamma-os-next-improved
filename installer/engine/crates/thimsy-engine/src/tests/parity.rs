//! Parity with the Python tools the shell build uses (scripts/pactool.py and
//! scripts/lptool.py). Skipped when python3 isn't available.

use crate::lp::Super;
use crate::pac;
use crate::testutil::*;
use std::path::PathBuf;
use std::process::Command;

fn scripts() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../scripts");
    let ok = Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    (ok && p.join("lptool.py").exists()).then_some(p)
}

#[test]
fn lp_grow_and_replace_match_lptool() {
    let Some(dir) = scripts() else { return };
    let img = build_super(
        16 << 20,
        12 << 20,
        &[("system_a", 1, 2), ("odm_a", 3, 1), ("vendor_a", 4, 2)],
    );
    let t = tempfile::tempdir().unwrap();
    let ours = t.path().join("ours.img");
    let theirs = t.path().join("theirs.img");
    std::fs::write(&ours, &img).unwrap();
    std::fs::write(&theirs, &img).unwrap();
    let data = pattern(3 * (1 << 20) + 4321, 9);
    let image = t.path().join("sys.img");
    std::fs::write(&image, &data).unwrap();

    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&ours)
        .unwrap();
    let mut s = Super::new(&mut f, 0, img.len() as u64);
    s.grow("system_a", None).unwrap();
    s.replace(
        "system_a",
        &mut &data[..],
        data.len() as u64,
        &mut |_, _| {},
    )
    .unwrap();
    drop(f);

    let py = |args: &[&str]| {
        let o = Command::new("python3")
            .arg(dir.join("lptool.py"))
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    };
    py(&["grow", theirs.to_str().unwrap(), "system_a", "max"]);
    py(&[
        "replace",
        theirs.to_str().unwrap(),
        "system_a",
        image.to_str().unwrap(),
    ]);
    assert!(
        std::fs::read(&ours).unwrap() == std::fs::read(&theirs).unwrap(),
        "engine and lptool.py disagree"
    );
}

#[test]
fn pac_set_info_matches_pactool() {
    let Some(dir) = scripts() else { return };
    let bytes = build_pac(
        &[
            E("FDL", "f.bin", pattern(100, 1)),
            E("Super", "s.img", pattern(1000, 2)),
        ],
        "Old",
        "OLD",
    );
    let t = tempfile::tempdir().unwrap();
    let ours = t.path().join("ours.pac");
    let theirs = t.path().join("theirs.pac");
    std::fs::write(&ours, &bytes).unwrap();
    std::fs::write(&theirs, &bytes).unwrap();
    pac::set_info(&ours, "Thimsy_OS_v1.0a", "RG_Cube_Thimsy_OS").unwrap();
    let o = Command::new("python3")
        .arg(dir.join("pactool.py"))
        .args([
            "setinfo",
            theirs.to_str().unwrap(),
            "Thimsy_OS_v1.0a",
            "RG_Cube_Thimsy_OS",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(
        std::fs::read(&ours).unwrap() == std::fs::read(&theirs).unwrap(),
        "engine and pactool.py disagree"
    );
}

#[test]
fn crc_matches_the_c_helper() {
    let Some(_) = scripts() else { return };
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tools/crc16arc.c");
    let t = tempfile::tempdir().unwrap();
    let bin = t.path().join("crc16arc");
    let Ok(o) = Command::new("cc")
        .args(["-O2", "-o"])
        .arg(&bin)
        .arg(&src)
        .output()
    else {
        return;
    };
    if !o.status.success() {
        return;
    }
    let data = pattern(1 << 20, 3);
    let file = t.path().join("d.bin");
    std::fs::write(&file, &data).unwrap();
    let out = Command::new(&bin).arg(&file).arg("100").output().unwrap();
    let theirs = u16::from_str_radix(String::from_utf8_lossy(&out.stdout).trim(), 16).unwrap();
    assert_eq!(pac::crc16_arc(0, &data[100..]), theirs);
}
