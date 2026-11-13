//! A complete synthetic device: official firmware as an encrypted split 7z
//! holding a PAC with a super image, a shared system image (zstd) and deltas.
//! Exercises resolve → fetch → extract → assemble → validate → publish, then
//! flash planning and cleanup.

use crate::catalog::Catalog;
use crate::events::{Event, MemorySink, Reporter};
use crate::host::Host;
use crate::job::{self, BuildOptions};
use crate::lp::Super;
use crate::pac::{self, Pac};
use crate::testutil::*;
use crate::util::sha256_hex;
use crate::{delta, flash, Code};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MIB: u64 = 1 << 20;

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub catalog: Catalog,
    pub system: Vec<u8>,
    pub vendor_new: Vec<u8>,
    pub odm_new: Vec<u8>,
    pub src_pac: Vec<u8>,
    pub expected_sha: String,
}

/// Builds the fixture. `expect` = Some(sha) to set the recipe's reference hash.
pub fn fixture(expect_override: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    // Source PAC: FDLs, NV (backup), vendor_boot, super, persist, userdata, config.
    let mut sup = build_super(
        12 << 20,
        10 * MIB,
        &[("system_a", 1, 3), ("odm_a", 4, 1), ("vendor_a", 5, 1)],
    );
    let odm_old = pattern(MIB as usize, 20);
    sup[(4 * MIB) as usize..(5 * MIB) as usize].copy_from_slice(&odm_old);
    let vendor_old = pattern(64 * 1024, 30);
    let entries = vec![
        E("FDL", "fdl1.bin", pattern(2048, 1)),
        E("FDL2", "fdl2.bin", pattern(4096, 2)),
        E("ProdNV", "prodnv.img", pattern(8192, 3)),
        E("VENDORBOOT", "vendor_boot.img", vendor_old.clone()),
        E("VENDORBOOT_B", "vendor_boot_b.img", vendor_old.clone()),
        E("Super", "super.img", sup),
        E("Persist", "persist.img", pattern(4096, 4)),
        E("UserData", "userdata.img", pattern(1024, 5)),
        E("EraseMetadata", "", vec![]),
        E(
            "",
            "cfg.xml",
            config_xml(&[
                ("FDL", "FDL", None, 0x5500, false),
                ("FDL2", "NAND_FDL", None, 0x9efffe00, false),
                ("ProdNV", "CODE2", Some("prodnv"), 0, true),
                ("VENDORBOOT", "CODE2", Some("vendor_boot_a"), 0, false),
                ("VENDORBOOT_B", "CODE2", Some("vendor_boot_b"), 0, false),
                ("Super", "YAFFS_IMG2", Some("super"), 0, false),
                ("Persist", "YAFFS_IMG2", Some("persist"), 0, false),
                ("UserData", "YAFFS_IMG2", Some("userdata"), 0, false),
                ("EraseMetadata", "EraseFlash2", Some("metadata"), 0, false),
            ]),
        ),
    ];
    let src_pac = build_pac(&entries, "Stock_1.0", "TESTDEV");
    let src_sha = sha256_hex(&src_pac);
    let parts = build_7z(d, "fw", "firmware.pac", &src_pac, "pw123", 3);

    // Thimsy pieces.
    let system = pattern((3 * MIB + MIB / 2) as usize, 40); // needs the grow
    let sys_path = d.join("thimsy-system.img.zst");
    std::fs::write(&sys_path, zstd::encode_all(&system[..], 3).unwrap()).unwrap();
    let mut vendor_new = vendor_old.clone();
    vendor_new[100..200].fill(0xEE);
    let vb_delta = d.join("vb.bsdiff");
    std::fs::write(&vb_delta, delta::make(&vendor_old, &vendor_new).unwrap()).unwrap();
    let mut odm_new = odm_old.clone();
    odm_new[5000..6000].fill(0x11);
    let odm_delta = d.join("odm.bsdiff");
    std::fs::write(&odm_delta, delta::make(&odm_old, &odm_new).unwrap()).unwrap();

    let parts_yaml: Vec<String> = parts
        .iter()
        .enumerate()
        .map(|(i, p)| format!("    - {}", artifact_yaml(&format!("p{i}"), p)))
        .collect();
    let sys_bytes = std::fs::read(&sys_path).unwrap();
    let mk_catalog = |expect: &str| {
        let root = format!(
            "version: test\nrelease: Thimsy Test\nsystems: {{ heavy: sys }}\nartifacts:\n  - {{ id: sys, file: thimsy-system.img.zst, size: {}, sha256: \"{}\", urls: [\"{}\"], unpack: {{ format: zstd, size: {}, sha256: \"{}\" }} }}\n",
            sys_bytes.len(),
            sha256_hex(&sys_bytes),
            file_url(&sys_path),
            system.len(),
            sha256_hex(&system)
        );
        let device = format!(
            r#"id: testdev
name: Test Device
maker: Test
soc: Unisoc T820
display: "720×720"
status: testing
hardware: {{ android_api: 34, armv82: true, vulkan: true, ram_gb: 8, gpu: test }}
support: {{ build: verified, boot: qualified, install: {{ windows: automated, linux: automated, macos: none, android: none }} }}
identify: {{ usb: [{{ vid: 0x1782, pid: 0x4d00, mode: download, label: "Unisoc download mode" }}] }}
source:
  id: src
  name: Stock
  kind: archive7z
  password: pw123
  contains: firmware.pac
  container: {{ format: pac, size: {}, sha256: "{}" }}
  parts:
{}
artifacts:
  - {}
  - {}
build:
  - {{ op: pac_delta, entry: VENDORBOOT, artifact: vb, in_sha256: "{vo}", out_sha256: "{vn}" }}
  - {{ op: pac_delta, entry: VENDORBOOT_B, artifact: vb, in_sha256: "{vo}", out_sha256: "{vn}" }}
  - {{ op: lp_grow, entry: Super, partition: system_a }}
  - {{ op: lp_write, entry: Super, partition: system_a, artifact: "@system" }}
  - {{ op: lp_delta, entry: Super, partition: odm_a, artifact: odm, in_sha256: "{oo}", out_sha256: "{on}" }}
  - {{ op: pac_set_info, version: "Thimsy_{{profile}}", alias: "TESTDEV" }}
  - {{ op: pac_fix_crc }}
expect: {{ heavy: "{expect}" }}
flash:
  method: spd
  fdl: [FDL, FDL2]
  update: [Super, VENDORBOOT, VENDORBOOT_B]
  erase_full: [userdata, metadata]
  never_write: [UserData, Persist]
guides:
  download_mode: [{{ title: "Hold Volume Down", body: "Plug in the cable." }}]
  unlock: {{ route: guided, steps: [] }}
"#,
            src_pac.len(),
            src_sha,
            parts_yaml.join("\n"),
            artifact_yaml("vb", &vb_delta),
            artifact_yaml("odm", &odm_delta),
            vo = sha256_hex(&vendor_old),
            vn = sha256_hex(&vendor_new),
            oo = sha256_hex(&odm_old),
            on = sha256_hex(&odm_new),
        );
        let profiles = ["light", "heavy"]
            .iter()
            .enumerate()
            .map(|(i, p)| (format!("profiles/{p}.yaml"), format!("id: {p}\nname: {p}\nsummary: s\norder: {i}\neditable: [\"nano.theme\"]\nsettings: {{ nano.theme: esde }}\n")));
        let mut files = vec![
            ("catalog.yaml".to_string(), root),
            ("devices/testdev.yaml".to_string(), device),
        ];
        files.extend(profiles);
        Catalog::from_files(files).unwrap()
    };
    // First pass with a placeholder expectation to learn the output hash.
    let placeholder = "0".repeat(64);
    let catalog = mk_catalog(expect_override.unwrap_or(&placeholder));
    let expected_sha = expect_override.map(str::to_string).unwrap_or_default();
    Fixture {
        dir,
        catalog,
        system,
        vendor_new,
        odm_new,
        src_pac,
        expected_sha,
    }
}

fn quiet() -> (Reporter, Arc<MemorySink>) {
    let sink = Arc::new(MemorySink::default());
    (Reporter::new(sink.clone()), sink)
}

fn opts(ws: &Path, delete_downloads: bool) -> BuildOptions {
    BuildOptions {
        device: "testdev".into(),
        profile: "heavy".into(),
        workspace: ws.to_path_buf(),
        imports: vec![],
        overrides: vec![],
        host: Host::Linux,
        delete_downloads,
    }
}

/// Builds once with a wrong expectation to read the real hash, then returns a
/// fixture whose recipe expects it.
fn good_fixture() -> Fixture {
    let probe = fixture(None);
    let ws = probe.dir.path().join("probe-ws");
    let (rep, _) = quiet();
    let err = job::build(&probe.catalog, &opts(&ws, false), &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
    let msg = err.to_string();
    let sha = msg.split("sha256 ").nth(1).unwrap()[..64].to_string();
    fixture(Some(&sha))
}

#[test]
fn builds_the_expected_image_and_records_the_job() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, sink) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    assert_eq!(rec.sha256, fx.expected_sha);
    assert_eq!(rec.matches_reference, Some(true));
    assert!(rec.image.ends_with("testdev-thimsy-heavy.pac"));

    // Contents: deltas applied to both slots, system written into the grown
    // partition, odm patched, strings and CRCs set.
    let p = Pac::open(&rec.image).unwrap();
    assert_eq!(
        p.read_entry(p.find("VENDORBOOT").unwrap()).unwrap(),
        fx.vendor_new
    );
    assert_eq!(
        p.read_entry(p.find("VENDORBOOT_B").unwrap()).unwrap(),
        fx.vendor_new
    );
    assert_eq!(p.version(), "Thimsy_heavy");
    let e = p.find("Super").unwrap();
    let mut f = std::fs::File::open(&rec.image).unwrap();
    let mut s = Super::new(&mut f, e.offset, e.size);
    let sys = s.read_partition("system_a").unwrap();
    assert_eq!(&sys[..fx.system.len()], &fx.system[..]);
    assert_eq!(s.read_partition("odm_a").unwrap(), fx.odm_new);
    let data = std::fs::read(&rec.image).unwrap();
    let (c1, c2) = pac::stored_crcs(&rec.image).unwrap();
    assert_eq!(c1, pac::crc16_arc(0, &data[..2120]));
    assert_eq!(c2, pac::crc16_arc(0, &data[2124..]));

    // Stages in order, and the scratch folder is gone.
    let stages: Vec<String> = sink
        .0
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            Event::Stage { stage, .. } => Some(stage.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        stages,
        ["plan", "fetch", "extract", "assemble", "validate", "publish"]
    );
    let job_dir = job::job_dir(&ws, "testdev", "heavy");
    assert!(!job_dir.join("work").exists());
    assert_eq!(job::load_job(&job_dir).unwrap().sha256, rec.sha256);

    // Rebuilding gives the same bytes (deterministic) and reuses the cache.
    let (rep2, sink2) = quiet();
    let rec2 = job::build(&fx.catalog, &opts(&ws, false), &rep2).unwrap();
    assert_eq!(rec2.sha256, rec.sha256);
    assert!(sink2.0.lock().unwrap().iter().any(
        |e| matches!(e, Event::Log { message, .. } if message.contains("already downloaded"))
    ));
}

#[test]
fn delete_downloads_frees_the_cache_and_clean_removes_the_rest() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, true), &rep).unwrap();
    let cache: Vec<PathBuf> = std::fs::read_dir(ws.join("cache"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert!(cache.is_empty(), "cache should be empty, has {cache:?}");
    let items = job::cleanable(&ws);
    assert!(items.iter().any(|i| i.kind == "image"));
    assert_eq!(job::clean(&ws, &["scratch"]).unwrap(), 0);
    assert!(rec.image.exists(), "image survives a scratch-only clean");
    let freed = job::clean(&ws, &["image"]).unwrap();
    assert_eq!(freed, rec.bytes);
    assert!(!rec.image.exists());
    assert!(
        job::load_job(&job::job_dir(&ws, "testdev", "heavy")).is_err(),
        "job can't be flashed without its image"
    );
}

#[test]
fn interrupted_assembly_reextracts_instead_of_reapplying_steps() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    // Simulate a crash in the middle of assembling the next build: state says
    // assembling, and the working PAC has already been modified.
    let dir = job::job_dir(&ws, "testdev", "heavy");
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let out = job::load_job(&dir).unwrap().image;
    std::fs::rename(&out, dir.join("work").join("firmware.pac")).unwrap();
    std::fs::write(
        dir.join("state.json"),
        r#"{"extracted":true,"assembling":true}"#,
    )
    .unwrap();
    let (rep, sink) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    assert_eq!(
        rec.sha256, fx.expected_sha,
        "a half-assembled image must not be reused"
    );
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, Event::Stage { stage, .. } if stage == "extract")));
}

#[test]
fn unpacked_official_firmware_in_an_import_folder_skips_the_download() {
    let fx = good_fixture();
    let imp = fx.dir.path().join("import");
    std::fs::create_dir_all(&imp).unwrap();
    std::fs::write(imp.join("firmware.pac"), &fx.src_pac).unwrap();
    // The archive is gone: only the unpacked firmware can satisfy the build.
    for i in 1..=3 {
        let _ = std::fs::remove_file(fx.dir.path().join(format!("fw.7z.{i:03}")));
    }
    let mut o = opts(&fx.dir.path().join("ws"), false);
    o.imports = vec![imp.clone()];
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &o, &rep).unwrap();
    assert_eq!(rec.sha256, fx.expected_sha);

    // A same-sized but different file is caught by the hash check.
    let mut bad = fx.src_pac.clone();
    bad[3000] ^= 0xFF;
    std::fs::write(imp.join("firmware.pac"), &bad).unwrap();
    let mut o = opts(&fx.dir.path().join("ws2"), false);
    o.imports = vec![imp];
    let (rep, _) = quiet();
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
}

#[test]
fn device_pack_archive_supplies_the_system_image_and_patches() {
    let mut fx = good_fixture();
    let d = fx.dir.path().to_path_buf();
    // One archive holding the system image and both patches, then nothing
    // else: the individual files are gone and have no download address.
    let files = ["thimsy-system.img.zst", "vb.bsdiff", "odm.bsdiff"].map(|f| d.join(f));
    let refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
    let entry =
        crate::bundle::make(&d.join("pack.tar"), &refs, u64::MAX, 1, &mut |_, _| {}).unwrap();
    for f in &files {
        std::fs::remove_file(f).unwrap();
    }
    for a in fx.catalog.root.artifacts.iter_mut() {
        a.urls.clear();
    }
    let dev = &mut fx.catalog.devices[0];
    for a in dev.artifacts.iter_mut() {
        a.urls.clear();
    }
    let mut bundle: crate::catalog::Artifact = serde_json::from_value(entry).unwrap();
    bundle.urls = vec![file_url(&d.join("pack.tar"))];
    dev.bundles.insert("heavy".into(), bundle.clone());

    // Downloaded (file:// here), then deleted afterwards with the rest.
    let ws = d.join("ws");
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, true), &rep).unwrap();
    assert_eq!(rec.sha256, fx.expected_sha);
    assert!(!ws.join("cache/pack.tar").exists());
    assert!(!ws.join("cache/thimsy-system.img.zst").exists());

    // A picked file under any name is used in place and never deleted.
    let picked = d.join("my download.tar");
    std::fs::rename(d.join("pack.tar"), &picked).unwrap();
    let mut o = opts(&d.join("ws2"), true);
    o.imports = vec![picked.clone()];
    let (rep, _) = quiet();
    assert_eq!(
        job::build(&fx.catalog, &o, &rep).unwrap().sha256,
        fx.expected_sha
    );
    assert!(picked.exists());
    assert!(!d.join("ws2/cache/my download.tar").exists());

    // Split into parts in an import folder.
    let bytes = std::fs::read(&picked).unwrap();
    let parts_dir = d.join("parts");
    std::fs::create_dir_all(&parts_dir).unwrap();
    bundle.parts = bytes
        .chunks(bytes.len() / 3 + 1)
        .enumerate()
        .map(|(i, c)| {
            let p = parts_dir.join(format!("pack.tar.{:03}", i + 1));
            std::fs::write(&p, c).unwrap();
            serde_yaml::from_str(&artifact_yaml(&format!("pack.tar.{:03}", i + 1), &p)).unwrap()
        })
        .collect();
    bundle.urls.clear();
    fx.catalog.devices[0].bundles.insert("heavy".into(), bundle);
    let mut o = opts(&d.join("ws3"), false);
    o.imports = vec![parts_dir.clone()];
    let (rep, _) = quiet();
    assert_eq!(
        job::build(&fx.catalog, &o, &rep).unwrap().sha256,
        fx.expected_sha
    );

    // A damaged picked file is refused, not silently replaced.
    let mut bad = bytes.clone();
    let mid = bad.len() / 2;
    bad[mid] ^= 0xFF;
    std::fs::write(&picked, &bad).unwrap();
    let mut o = opts(&d.join("ws4"), false);
    o.imports = vec![picked];
    let (rep, _) = quiet();
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
}

#[test]
fn tampered_inputs_are_rejected() {
    let fx = good_fixture();
    // Corrupt one archive volume: the download check catches it.
    let part = fx.dir.path().join("fw.7z.002");
    let mut b = std::fs::read(&part).unwrap();
    b[10] ^= 0xFF;
    std::fs::write(&part, &b).unwrap();
    let (rep, _) = quiet();
    let err = job::build(&fx.catalog, &opts(&fx.dir.path().join("ws"), false), &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
}

#[test]
fn not_enough_space_is_reported_before_anything_is_written() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, false).unwrap();
    assert!(sc.ok);
    assert!(sc.need >= sc.downloads);
    assert_eq!(sc.cached, 0);
    let with_delete = job::check_space(&fx.catalog, "testdev", "heavy", &ws, true).unwrap();
    assert!(with_delete.flash_need <= sc.flash_need);
    // The message for the app.
    let e = job::not_enough_space(&ws, 10_000_000_000, 1_000);
    assert_eq!(crate::error::code_of(&e).0, Code::NotEnoughSpace);
    assert!(e.to_string().contains("10.00 GB"));
}

#[test]
fn flash_plan_backs_up_nv_and_never_writes_protected_partitions() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    let d = fx.catalog.device("testdev").unwrap();
    let p = Pac::open(&rec.image).unwrap();
    let full = flash::spd_plan(d, &p, flash::Mode::Full).unwrap();
    let written: Vec<&str> = full.writes.iter().map(|w| w.partition.as_str()).collect();
    assert_eq!(written, ["vendor_boot_a", "vendor_boot_b", "super"]);
    assert_eq!(
        full.backups
            .iter()
            .map(|b| b.partition.as_str())
            .collect::<Vec<_>>(),
        ["prodnv"]
    );
    assert_eq!(full.erases, ["userdata", "metadata"]);
    assert_eq!(
        full.fdl,
        vec![
            ("FDL".to_string(), 0x5500),
            ("FDL2".to_string(), 0x9efffe00)
        ]
    );
    let upd = flash::spd_plan(d, &p, flash::Mode::Update).unwrap();
    assert!(upd.erases.is_empty());
    assert_eq!(upd.writes.len(), 3);

    // A recipe that forgets to exclude persist is refused.
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().never_write = vec!["UserData".into()];
    let err = flash::spd_plan(&bad, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string().contains("protected partition persist"),
        "{err}"
    );
    // An update list naming an unknown entry is refused.
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().update = vec!["Nope".into()];
    assert!(flash::spd_plan(&bad, &p, flash::Mode::Update).is_err());

    // Hosts: macOS is "none" for this device → unsupported_host, except dry runs.
    let o = flash::FlashOptions {
        job: job::job_dir(&ws, "testdev", "heavy"),
        mode: flash::Mode::Full,
        dry_run: false,
        host: Host::Macos,
    };
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::UnsupportedHost);
    let o = flash::FlashOptions { dry_run: true, ..o };
    let r = flash::flash(&fx.catalog, &o, &rep).unwrap();
    assert!(r.commands[0].contains("write_part super super.img"));
}

#[test]
fn unavailable_profile_is_refused() {
    let fx = good_fixture();
    let (rep, _) = quiet();
    let mut o = opts(&fx.dir.path().join("ws"), false);
    o.profile = "light".into();
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ProfileUnavailable);
}
