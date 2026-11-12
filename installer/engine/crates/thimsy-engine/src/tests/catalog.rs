use crate::catalog::{Catalog, InstallSupport};
use crate::host::Host;

const ROOT: &str = "version: t\nrelease: R\n";
const PROFILE: &str = "id: heavy\nname: Heavy\nsummary: s\n";
const DEV: &str = r#"id: d
name: D
maker: M
soc: S
display: "1x1"
status: candidate
hardware: { android_api: 34, armv82: true, vulkan: true, ram_gb: 8, gpu: g }
support: { build: candidate, boot: candidate, install: { windows: untested } }
guides: { unlock: { route: none } }
"#;

fn files(extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut v = vec![
        ("catalog.yaml".into(), ROOT.into()),
        ("profiles/heavy.yaml".into(), PROFILE.into()),
    ];
    v.extend(extra.iter().map(|(a, b)| (a.to_string(), b.to_string())));
    v
}

#[test]
fn bundled_catalog_is_consistent() {
    let c = Catalog::bundled().unwrap();
    for d in &c.devices {
        // Every device names every host explicitly or defaults to none.
        for h in [Host::Windows, Host::Macos, Host::Linux, Host::Android] {
            let _ = d.install_on(h);
        }
        // Android hosts can't flash in this version.
        assert_eq!(
            d.install_on(Host::Android),
            InstallSupport::None,
            "{}",
            d.id
        );
        // Buildable devices have a flash recipe and guides.
        if !d.build.is_empty() {
            assert!(d.flash.is_some(), "{} builds but can't flash", d.id);
            assert!(
                !d.guides.download_mode.is_empty(),
                "{} lacks download-mode steps",
                d.id
            );
            assert!(!d.guides.backup.is_empty(), "{} lacks backup steps", d.id);
            assert!(!d.expect.is_empty(), "{} has no reference hash", d.id);
        }
        // Devices this version can't flash somewhere must explain why on that host.
        if d.install_on(Host::Macos) == InstallSupport::None {
            assert!(
                crate::resolve::host_text(d, Host::Macos).contains("Windows"),
                "{}",
                d.id
            );
        }
    }
    // Profiles are ordered light → heavy and heavy is published.
    assert!(c.system_for("heavy").is_some());
    assert!(c.system_for("light").is_none());
}

#[test]
fn rejects_malformed_catalogs() {
    // Missing root.
    assert!(Catalog::from_files(vec![("profiles/heavy.yaml".into(), PROFILE.into())]).is_err());
    // Unknown field (typo protection).
    assert!(
        Catalog::from_files(files(&[("devices/d.yaml", &format!("{DEV}colour: red\n"))])).is_err()
    );
    // Duplicate device ids.
    assert!(
        Catalog::from_files(files(&[("devices/a.yaml", DEV), ("devices/b.yaml", DEV)])).is_err()
    );
    // systems → unknown profile / artifact.
    let bad_root = "version: t\nrelease: R\nsystems: { nope: x }\n";
    assert!(Catalog::from_files(vec![
        ("catalog.yaml".into(), bad_root.into()),
        ("profiles/heavy.yaml".into(), PROFILE.into())
    ])
    .is_err());
    let bad_root = "version: t\nrelease: R\nsystems: { heavy: missing }\n";
    assert!(Catalog::from_files(vec![
        ("catalog.yaml".into(), bad_root.into()),
        ("profiles/heavy.yaml".into(), PROFILE.into())
    ])
    .is_err());
    // Malformed sha256.
    let bad_root =
        "version: t\nrelease: R\nartifacts: [{ id: a, file: a, size: 1, sha256: xyz }]\n";
    assert!(Catalog::from_files(vec![("catalog.yaml".into(), bad_root.into())]).is_err());
    // Build steps naming an unknown artifact, or without source firmware.
    let with_build = format!("{DEV}build: [{{ op: pac_delta, entry: X, artifact: nope, in_sha256: a, out_sha256: b }}]\n");
    assert!(Catalog::from_files(files(&[("devices/d.yaml", &with_build)])).is_err());
    let with_fix = format!("{DEV}build: [{{ op: pac_fix_crc }}]\n");
    let err = Catalog::from_files(files(&[("devices/d.yaml", &with_fix)]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("no source firmware"), "{err}");
    // Unknown op.
    let with_op = format!("{DEV}build: [{{ op: rm_rf }}]\n");
    assert!(Catalog::from_files(files(&[("devices/d.yaml", &with_op)])).is_err());
}

#[test]
fn loads_from_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files(&[("devices/d.yaml", DEV)]) {
        let p = dir.path().join(&name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let c = Catalog::from_dir(dir.path()).unwrap();
    assert_eq!(
        c.device("d").unwrap().install_on(Host::Linux),
        InstallSupport::None,
        "unlisted host = none"
    );
    assert!(c.device("zz").is_err());
}
