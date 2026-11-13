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

/// The fixture firmware's partition table (MiB). NV is bigger than its image
/// in the PAC: backups read the whole partition.
pub const TABLE: &[(&str, u64)] = &[
    ("prodnv", 1),
    ("vendor_boot_a", 1),
    ("vendor_boot_b", 1),
    ("boot_a", 1),
    ("super", 12),
    ("persist", 1),
    ("metadata", 1),
    ("userdata", pac::REST_OF_DISK),
];

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
        E("BOOT", "boot.img", pattern(8192, 6)),
        E("Super", "super.img", sup),
        E("Persist", "persist.img", pattern(4096, 4)),
        E("UserData", "userdata.img", pattern(1024, 5)),
        E("EraseMetadata", "", vec![]),
        E(
            "",
            "cfg.xml",
            config_xml_with_table(
                &[
                    ("FDL", "FDL", None, 0x5500, false),
                    ("FDL2", "NAND_FDL", None, 0x9efffe00, false),
                    ("ProdNV", "CODE2", Some("prodnv"), 0, true),
                    ("VENDORBOOT", "CODE2", Some("vendor_boot_a"), 0, false),
                    ("VENDORBOOT_B", "CODE2", Some("vendor_boot_b"), 0, false),
                    ("BOOT", "CODE2", Some("boot_a"), 0, false),
                    ("Super", "YAFFS_IMG2", Some("super"), 0, false),
                    ("Persist", "YAFFS_IMG2", Some("persist"), 0, false),
                    ("UserData", "YAFFS_IMG2", Some("userdata"), 0, false),
                    ("EraseMetadata", "EraseFlash2", Some("metadata"), 0, false),
                ],
                TABLE,
            ),
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
  update_check: [BOOT]
  writes_full: [vendor_boot_a, vendor_boot_b, boot_a, super]
  writes_update: [vendor_boot_a, vendor_boot_b, super]
  erase_full: [userdata, metadata]
  never_write: [UserData, Persist]
guides:
  download_mode: [{{ title: "Hold Back", body: "Turn the {{device}} off, hold Back and plug in the cable." }}]
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
pub fn good_fixture() -> Fixture {
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
    let items = job::cleanable(&fx.catalog, &ws);
    assert!(items.iter().any(|i| i.kind == "image"));
    assert_eq!(job::clean(&fx.catalog, &ws, &["scratch"], &[]).unwrap(), 0);
    assert!(rec.image.exists(), "image survives a scratch-only clean");
    let freed = job::clean(&fx.catalog, &ws, &["image"], &[]).unwrap();
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
    let d = fx.catalog.device("testdev").unwrap();
    let sp = crate::resolve::space(&fx.catalog, d, "heavy");
    let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], false).unwrap();
    assert!(sc.ok);
    assert!(sc.need >= sc.downloads);
    assert_eq!(sc.cached, 0);
    // The plan's figure is the same as `space`'s for an empty workspace.
    let plan = crate::resolve::resolve(&fx.catalog, "testdev", "heavy", &[], Host::Linux).unwrap();
    assert_eq!(plan.disk_bytes, sc.need);
    assert_eq!(plan.download_bytes, sc.downloads);
    let with_delete = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], true).unwrap();
    assert!(with_delete.flash_need <= sc.flash_need);
    // Flashing needs the image (not built yet), the unpacked partitions and,
    // when downloads are kept, the downloads still missing; the build needs
    // its peak minus what's cached. Nothing is subtracted twice.
    std::fs::create_dir_all(ws.join("cache")).unwrap();
    std::fs::copy(fx.dir.path().join("fw.7z.001"), ws.join("cache/fw.7z.001")).unwrap();
    for delete in [false, true] {
        let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], delete).unwrap();
        assert_eq!(
            sc.cached,
            std::fs::metadata(ws.join("cache/fw.7z.001")).unwrap().len()
        );
        let kept = if delete { 0 } else { sc.downloads - sc.cached };
        assert_eq!(
            sc.flash_need,
            sp.image + sp.flash_extra + kept,
            "delete={delete}"
        );
        assert!(
            sc.need >= sp.image + sp.flash_extra + kept,
            "delete={delete}"
        );
        let fresh = sc.downloads - sc.cached;
        assert_eq!(sc.build_need, fresh + sp.image + (fresh + sp.image) / 20);
        assert!(sc.need >= sc.build_need, "delete={delete}");
        if !delete {
            assert!(sc.need >= sp.image + sp.flash_extra + (sc.downloads - sc.cached));
        }
    }
    // With a device pack, the system image and patches come from it: copies
    // of them in the cache don't count as already downloaded.
    let mut cat = fx.catalog.clone();
    let mut pack = d.artifacts[0].clone();
    pack.file = "pack.tar".into();
    pack.size = 123_456;
    cat.devices[0].bundles.insert("heavy".into(), pack);
    std::fs::copy(
        fx.dir.path().join("thimsy-system.img.zst"),
        ws.join("cache/thimsy-system.img.zst"),
    )
    .unwrap();
    let sc = job::check_space(&cat, "testdev", "heavy", &ws, &[], false).unwrap();
    assert_eq!(
        sc.cached,
        std::fs::metadata(ws.join("cache/fw.7z.001")).unwrap().len()
    );
    // The message for the app.
    let e = job::not_enough_space(&ws, 10_000_000_000, 1_000);
    assert_eq!(crate::error::code_of(&e).0, Code::NotEnoughSpace);
    assert!(e.to_string().contains("10.00 GB"));
}

#[test]
fn picked_files_that_the_build_cant_use_are_refused() {
    let fx = good_fixture();
    let other = fx.dir.path().join("holiday.mp4");
    std::fs::write(&other, b"not firmware").unwrap();
    let mut o = opts(&fx.dir.path().join("ws"), false);
    o.imports = vec![other];
    let (rep, _) = quiet();
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::BadInput, "{err:#}");
    let msg = err.to_string();
    assert!(
        msg.contains("holiday.mp4") && msg.contains("firmware.pac"),
        "{msg}"
    );
    assert!(
        !fx.dir.path().join("ws/cache/fw.7z.001").exists(),
        "nothing downloaded"
    );
    // Settings overrides can't be applied yet: refused, not ignored.
    let mut o = opts(&fx.dir.path().join("ws"), false);
    o.overrides = vec![("nano.theme".into(), "minima".into())];
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::BadInput, "{err:#}");
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
    assert_eq!(
        written,
        ["vendor_boot_a", "vendor_boot_b", "boot_a", "super"]
    );
    assert!(full.base.is_empty(), "a full install checks no base");
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
    // An update reads back what it relies on: boot, at its image's size,
    // and checks the device's table has it.
    assert_eq!(upd.base.len(), 1);
    assert_eq!(upd.base[0].partition, "boot_a");
    assert_eq!(upd.base[0].bytes, 8192);
    assert!(upd.checks.contains(&("boot_a".to_string(), 1)));
    // What the plan shows is what's written: a recipe whose list differs
    // from the firmware is refused.
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().writes_full.pop();
    let err = flash::spd_plan(&bad, &p, flash::Mode::Full).unwrap_err();
    assert!(err.to_string().contains("writes_full"), "{err}");
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().update_check = vec!["VENDORBOOT".into()];
    let err = flash::spd_plan(&bad, &p, flash::Mode::Update).unwrap_err();
    assert!(err.to_string().contains("which the update writes"), "{err}");
    // The plan lists the same partitions before anything is built.
    let plan = crate::resolve::resolve(&fx.catalog, "testdev", "heavy", &[], Host::Linux).unwrap();
    let names = |w: &[flash::Write]| w.iter().map(|w| w.partition.clone()).collect::<Vec<_>>();
    assert_eq!(plan.partitions_full, names(&full.writes));
    assert_eq!(plan.partitions_update, names(&upd.writes));
    assert_eq!(plan.erases_full, full.erases);

    // Backups read the whole partition (1 MiB), not the PAC image (8 KiB).
    assert_eq!(full.backups[0].bytes, MIB);
    // Everything touched is checked against the device's table.
    assert_eq!(
        full.checks,
        [
            ("prodnv".to_string(), 1),
            ("vendor_boot_a".to_string(), 1),
            ("vendor_boot_b".to_string(), 1),
            ("boot_a".to_string(), 1),
            ("super".to_string(), 12),
            ("userdata".to_string(), pac::REST_OF_DISK),
            ("metadata".to_string(), 1),
        ]
    );
    // Erasing NV is refused too, whatever the case.
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().erase_full = vec!["userdata".into(), "ProdNV".into()];
    let err = flash::spd_plan(&bad, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string().contains("erase protected partition ProdNV"),
        "{err}"
    );
    // Listing persist for update doesn't make it writable.
    let mut bad = d.clone();
    bad.flash.as_mut().unwrap().update.push("Persist".into());
    bad.flash.as_mut().unwrap().never_write = vec!["UserData".into()];
    let err = flash::spd_plan(&bad, &p, flash::Mode::Update).unwrap_err();
    assert!(
        err.to_string()
            .contains("write protected partition persist"),
        "{err}"
    );
    // A misspelt never_write name is refused.
    let mut bad = d.clone();
    bad.flash
        .as_mut()
        .unwrap()
        .never_write
        .push("Persits".into());
    let err = flash::spd_plan(&bad, &p, flash::Mode::Full).unwrap_err();
    assert!(err.to_string().contains("Persits"), "{err}");
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

    // A build made with another catalog version is flagged.
    let mut rec2 = job::load_job(&o.job).unwrap();
    rec2.catalog = "older".into();
    std::fs::write(o.job.join("job.json"), serde_json::to_vec(&rec2).unwrap()).unwrap();
    let (rep, sink) = quiet();
    flash::flash(&fx.catalog, &o, &rep).unwrap();
    assert!(sink.0.lock().unwrap().iter().any(|e| matches!(e,
        Event::Log { level, message } if level == "warn" && message.contains("catalog older"))));
}

#[test]
fn every_partition_touched_must_be_checkable_on_the_device() {
    let dir = tempfile::tempdir().unwrap();
    let mk = |name: &str, table: &[(&str, u64)]| -> Pac {
        let entries = vec![
            E("FDL", "fdl1.bin", pattern(2048, 1)),
            E("FDL2", "fdl2.bin", pattern(4096, 2)),
            E("SPLLoader", "spl.img", pattern(4096, 3)),
            E("Boot", "boot.img", pattern(4096, 4)),
            E(
                "",
                "cfg.xml",
                config_xml_with_table(
                    &[
                        ("FDL", "FDL", None, 0x5500, false),
                        ("FDL2", "NAND_FDL", None, 0x9efffe00, false),
                        ("SPLLoader", "UBOOT_LOADER2", Some("splloader"), 0, false),
                        ("Boot", "CODE2", Some("BOOT_A"), 0, false),
                    ],
                    table,
                ),
            ),
        ];
        let path = dir.path().join(format!("{name}.pac"));
        std::fs::write(&path, build_pac(&entries, "v", "a")).unwrap();
        Pac::open(&path).unwrap()
    };
    let cat = Catalog::bundled().unwrap();
    let mut d = cat.device("anbernic-rg-cube").unwrap().clone();
    let set = |d: &mut crate::catalog::Device, unchecked: &[&str]| {
        let f = d.flash.as_mut().unwrap();
        f.fdl = vec!["FDL".into(), "FDL2".into()];
        f.update.clear();
        f.update_check.clear();
        f.writes_full = vec!["splloader".into(), "boot_a".into()];
        f.erase_full.clear();
        f.never_write.clear();
        f.unchecked_partitions = unchecked.iter().map(|s| s.to_string()).collect();
    };
    let p = mk("normal", &[("boot_a", 1), ("userdata", pac::REST_OF_DISK)]);
    // The table names partitions in another case: still checked.
    set(&mut d, &["SPLLOADER"]);
    let plan = flash::spd_plan(&d, &p, flash::Mode::Full).unwrap();
    assert_eq!(plan.checks, [("BOOT_A".to_string(), 1)]);
    assert_eq!(plan.writes.len(), 2);
    // A partition missing from the table is refused unless the recipe
    // names it as unchecked.
    set(&mut d, &[]);
    let err = flash::spd_plan(&d, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string()
            .contains("splloader isn't in the PAC's partition table"),
        "{err}"
    );
    // The list only names partitions outside the table, written by the PAC.
    set(&mut d, &["splloader", "boot_a"]);
    let err = flash::spd_plan(&d, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string()
            .contains("boot_a, which is in the PAC's partition table"),
        "{err}"
    );
    set(&mut d, &["splloader", "splloader_bak"]);
    let err = flash::spd_plan(&d, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string()
            .contains("splloader_bak, which isn't a partition"),
        "{err}"
    );
    // Nothing left to check: refused, never an empty check that "matches".
    let p = mk("other", &[("other", 1), ("userdata", pac::REST_OF_DISK)]);
    set(&mut d, &["splloader", "boot_a"]);
    let err = flash::spd_plan(&d, &p, flash::Mode::Full).unwrap_err();
    assert!(
        err.to_string().contains("nothing this flash touches"),
        "{err}"
    );
}

#[test]
fn an_unpacked_firmware_isnt_downloaded_again() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    // An earlier run unpacked the official firmware, then stopped.
    let dir = job::job_dir(&ws, "testdev", "heavy");
    std::fs::create_dir_all(dir.join("work")).unwrap();
    std::fs::write(dir.join("work").join("firmware.pac"), &fx.src_pac).unwrap();
    std::fs::write(
        dir.join("state.json"),
        r#"{"extracted":true,"assembling":false}"#,
    )
    .unwrap();
    // Its download is gone (deleted to save space): fetching it would fail.
    for i in 1..=3 {
        std::fs::remove_file(fx.dir.path().join(format!("fw.7z.{i:03}"))).unwrap();
    }
    let (rep, sink) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    assert_eq!(rec.sha256, fx.expected_sha);
    assert!(sink.0.lock().unwrap().iter().any(
        |e| matches!(e, Event::Log { message, .. } if message == "Firmware already unpacked")
    ));
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

/// Flashes the fixture through the fake spd_dump (installer/tests/fake-tools).
#[cfg(unix)]
#[test]
fn flash_checks_the_image_the_device_and_the_backups() {
    use crate::tools;
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    let job_dir = job::job_dir(&ws, "testdev", "heavy");
    let flash_dir = job_dir.join("flash");
    let backup_dir = job_dir.join("backup");
    let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fake-tools");
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = Some(fake));
    let log = fx.dir.path().join("spd.log");
    let fake_env = |extra: Option<(&str, String)>| {
        let _ = std::fs::remove_file(&log);
        let mut env = vec![
            ("FAKE_SPD_LOG".to_string(), log.display().to_string()),
            ("FAKE_SPD_HASH".to_string(), "1".to_string()),
        ];
        env.extend(extra.map(|(k, v)| (k.to_string(), v)));
        tools::test::ENV.with(|e| *e.borrow_mut() = env);
    };
    let o = flash::FlashOptions {
        job: job_dir.clone(),
        mode: flash::Mode::Full,
        dry_run: false,
        host: Host::Linux,
    };
    let p = Pac::open(&rec.image).unwrap();
    let super_sha = sha256_hex(&p.read_entry(p.find("Super").unwrap()).unwrap());

    // Files left by a flash of an older image, the same size as this one's:
    // thrown away, not flashed.
    std::fs::create_dir_all(&flash_dir).unwrap();
    std::fs::write(flash_dir.join(".image-sha256"), "0".repeat(64)).unwrap();
    std::fs::write(flash_dir.join("super.img"), vec![0xAA; 12 << 20]).unwrap();
    fake_env(None);
    let (rep, sink) = quiet();
    let r = flash::flash(&fx.catalog, &o, &rep).unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains(&format!("sha256 super {super_sha}")),
        "super must come from this image:\n{text}"
    );
    // The device's table is read into this run's backup folder.
    assert!(text.contains("partition_list ../backup/run-"), "{text}");
    assert!(text.contains("erase_part metadata"));
    assert!(!flash_dir.exists(), "scratch removed after a good flash");
    // Read straight into a folder of its own in the job's backup folder.
    let first_run = r.backup.clone().unwrap();
    assert_eq!(first_run.parent(), Some(backup_dir.as_path()));
    assert!(first_run
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("run-"));
    // The backup is the whole partition.
    assert_eq!(
        std::fs::metadata(first_run.join("prodnv.img"))
            .unwrap()
            .len(),
        MIB
    );
    let runs = |dir: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        v.sort();
        v
    };
    assert!(sink.0.lock().unwrap().iter().any(|e| matches!(e,
        Event::Log { message, .. } if message.contains("partition table matches"))));

    // Another device (or stock layout): super is a different size. Nothing
    // is confirmed, so nothing is written or erased.
    let parts = fx.dir.path().join("other-device.xml");
    let mut table = String::from("<Partitions>\n");
    for (i, (name, mib)) in TABLE.iter().enumerate() {
        let mib = if *name == "super" { 10 } else { *mib };
        let size = if i + 1 == TABLE.len() {
            "0xffffffff".to_string()
        } else {
            mib.to_string()
        };
        table.push_str(&format!("    <Partition id=\"{name}\" size=\"{size}\"/>\n"));
    }
    table.push_str("</Partitions>\n");
    std::fs::write(&parts, table).unwrap();
    fake_env(Some(("FAKE_SPD_PARTS", parts.display().to_string())));
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::WrongDevice, "{err:#}");
    assert!(
        err.to_string().contains("super is 10 MiB on the device"),
        "{err}"
    );
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        !text.contains("write_part") && !text.contains("erase_part"),
        "{text}"
    );
    // Its backup is kept, next to (not over) the first one.
    let kept = runs(&backup_dir);
    assert_eq!(kept.len(), 2, "both backups kept: {kept:?}");
    for run in &kept {
        assert_eq!(
            std::fs::metadata(run.join("prodnv.img")).unwrap().len(),
            MIB
        );
    }

    // spd_dump says the device's table is unusable (sizes in the wrong
    // unit): refused like a different device.
    let bad_table = fx.dir.path().join("error-table.xml");
    std::fs::write(&bad_table, "<Partitions>\n    <Partition id=\"prodnv\" size=\"1\"/>\n<error>Wrong unit size, do not use this table!</error>\n</Partitions>\n").unwrap();
    fake_env(Some(("FAKE_SPD_PARTS", bad_table.display().to_string())));
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::WrongDevice, "{err:#}");
    assert!(err.to_string().contains("Wrong unit size"), "{err}");
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        !text.contains("write_part") && !text.contains("erase_part"),
        "{text}"
    );

    // A backup read short: nothing is confirmed, and the file is kept,
    // marked incomplete.
    fake_env(Some(("FAKE_SPD_SHORT_READ", "prodnv".to_string())));
    let before = runs(&backup_dir);
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed, "{err:#}");
    assert!(
        format!("{err:#}").contains("backup of prodnv is incomplete"),
        "{err:#}"
    );
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        !text.contains("write_part") && !text.contains("erase_part"),
        "{text}"
    );
    let new: Vec<PathBuf> = runs(&backup_dir)
        .into_iter()
        .filter(|r| !before.contains(r))
        .collect();
    assert_eq!(new.len(), 1, "{new:?}");
    assert_eq!(
        std::fs::metadata(new[0].join("prodnv.img.incomplete"))
            .unwrap()
            .len(),
        MIB / 2
    );
    assert!(!new[0].join("prodnv.img").exists());
    for run in &before {
        assert_eq!(
            std::fs::metadata(run.join("prodnv.img")).unwrap().len(),
            MIB
        );
    }

    // The device stops answering halfway through a write: spd_dump says so
    // and carries on (exit 0), but the flash stops there and fails.
    fake_env(Some(("FAKE_SPD_SHORT_WRITE", "vendor_boot_b".to_string())));
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed, "{err:#}");
    assert!(
        format!("{err:#}").contains("unexpected response"),
        "{err:#}"
    );
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains("write_part vendor_boot_b vendor_boot_b.img short"),
        "{text}"
    );
    assert!(
        !text.contains("write_part super") && !text.contains("erase_part"),
        "nothing after the broken write:\n{text}"
    );

    // An spd_dump that writes without asking (old or forked): stopped at
    // its first write, and an error even though it would exit 0.
    fake_env(Some(("FAKE_SPD_NO_PROMPT", "1".to_string())));
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed, "{err:#}");
    assert!(
        format!("{err:#}").contains("without asking first"),
        "{err:#}"
    );
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(text.matches("write_part").count() <= 1, "{text}");
    assert!(!text.contains("erase_part"), "{text}");

    // A build that isn't the image the catalog now expects (the installer
    // was updated since): refused before the device is touched.
    let newer = fixture(Some(&"1".repeat(64)));
    fake_env(None);
    let (rep, _) = quiet();
    let err = flash::flash(&newer.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
    assert!(err.to_string().contains("catalog test expects"), "{err}");
    assert!(!log.exists(), "spd_dump never ran");

    // Backups an earlier engine left unmoved in the scratch folder survive
    // a scratch clean (moved to the backup folder), and a backup clean is
    // the only thing that removes backups.
    let legacy = flash_dir.join("backup");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("prodnv.img"), vec![7u8; 4096]).unwrap();
    let items = job::cleanable(&fx.catalog, &ws);
    assert!(items
        .iter()
        .any(|i| i.kind == "backup" && i.path == legacy && i.bytes == 4096));
    assert!(items
        .iter()
        .any(|i| i.kind == "scratch" && i.path == flash_dir));
    let backups_before = runs(&backup_dir).len();
    job::clean(&fx.catalog, &ws, &["scratch"], &[]).unwrap();
    assert!(!flash_dir.exists());
    let after = runs(&backup_dir);
    assert_eq!(after.len(), backups_before + 1);
    let recovered = after
        .iter()
        .find(|r| r.to_string_lossy().ends_with("-recovered"))
        .unwrap();
    assert_eq!(
        std::fs::read(recovered.join("prodnv.img")).unwrap(),
        vec![7u8; 4096]
    );
    // Each run is its own item, with what its device.json says.
    let items = job::cleanable(&fx.catalog, &ws);
    assert!(items
        .iter()
        .any(|i| i.kind == "backup" && i.path == *recovered));
    assert!(items.iter().any(|i| i.kind == "backup"
        && i.path == first_run
        && i.info.as_ref().is_some_and(|v| v["device"] == "testdev")));

    // Publishing a new image clears the old unpacked partitions, but not
    // a backup left among them.
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(flash_dir.join("super.img"), b"old").unwrap();
    std::fs::write(legacy.join("miscdata.img"), b"nv").unwrap();
    let (rep, _) = quiet();
    job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    assert!(!flash_dir.exists(), "build must remove <job>/flash");
    assert!(runs(&backup_dir)
        .iter()
        .any(|r| std::fs::read(r.join("miscdata.img")).is_ok_and(|b| b == b"nv")));

    // The image changed after the build (same size): refused before the
    // device is touched.
    let mut bytes = std::fs::read(&rec.image).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&rec.image, &bytes).unwrap();
    fake_env(None);
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::HashMismatch, "{err:#}");
    assert!(!log.exists(), "spd_dump never ran");
    tools::test::ENV.with(|e| e.borrow_mut().clear());
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = None);
}

/// Update mode's base check, a device that never shows up, a damaged
/// unpacked partition, and the workspace lock, through the fake spd_dump.
#[cfg(unix)]
#[test]
fn flash_checks_the_base_the_connection_and_each_partition() {
    use crate::tools;
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    let rec = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    let job_dir = job::job_dir(&ws, "testdev", "heavy");
    let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fake-tools");
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = Some(fake));
    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = Some(vec![]));
    let log = fx.dir.path().join("spd.log");
    let fake_env = |extra: &[(&str, &str)]| {
        let _ = std::fs::remove_file(&log);
        let mut env = vec![
            ("FAKE_SPD_LOG".to_string(), log.display().to_string()),
            ("FAKE_SPD_HASH".to_string(), "1".to_string()),
        ];
        env.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        tools::test::ENV.with(|e| *e.borrow_mut() = env);
    };
    let o = |mode| flash::FlashOptions {
        job: job_dir.clone(),
        mode,
        dry_run: false,
        host: Host::Linux,
    };
    let p = Pac::open(&rec.image).unwrap();
    let entry_sha = |id: &str| sha256_hex(&p.read_entry(p.find(id).unwrap()).unwrap());
    let written = |text: &str| text.contains("write_part") || text.contains("erase_part");

    // An update on a device running the matching base: boot_a is read back
    // at its image's size and compared, then the update is written.
    fake_env(&[]);
    let (rep, sink) = quiet();
    let r = flash::flash(&fx.catalog, &o(flash::Mode::Update), &rep).unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains("read_part boot_a 0x2000 ../backup/run-"),
        "{text}"
    );
    assert!(text.contains("write_part super") && !text.contains("write_part boot_a"));
    assert!(sink.0.lock().unwrap().iter().any(|e| matches!(e,
        Event::Log { message, .. } if message.contains("runs the base this update needs"))));
    // What was read for the check isn't kept with the backups; the run says
    // which device and table it came from.
    let run = r.backup.unwrap();
    assert!(!run.join("check-boot_a.img").exists());
    let info: serde_json::Value =
        serde_json::from_slice(&std::fs::read(run.join("device.json")).unwrap()).unwrap();
    assert_eq!(info["device"], "testdev");
    assert_eq!(info["mode"], "update");
    let table = std::fs::read(run.join("device-partitions.xml")).unwrap();
    assert_eq!(info["partition_table_sha256"], sha256_hex(&table));

    // Another base: refused before the first write, nothing written.
    fake_env(&[("FAKE_SPD_UPDATE_MISMATCH", "boot_a")]);
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o(flash::Mode::Update), &rep).unwrap_err();
    assert_eq!(
        crate::error::code_of(&err).0,
        Code::FullInstallNeeded,
        "{err:#}"
    );
    assert!(err.to_string().contains("boot_a"), "{err}");
    assert!(!format!("{err:#}").contains("still recoverable"), "{err:#}");
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(!written(&text), "{text}");

    // The device never answers in download mode: device_not_found, and no
    // "flashing stopped" wording (nothing was written).
    fake_env(&[("FAKE_SPD_NO_DEVICE", "1")]);
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o(flash::Mode::Full), &rep).unwrap_err();
    let (code, hint) = crate::error::code_of(&err);
    assert_eq!(code, Code::DeviceNotFound, "{err:#}");
    assert!(hint.unwrap().contains("Repeat the download-mode steps"));
    assert!(!format!("{err:#}").contains("still recoverable"), "{err:#}");
    // On Linux, with the device connected but not openable: USB access.
    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = Some(vec![(0x1782, 0x4d00, None)]));
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o(flash::Mode::Full), &rep).unwrap_err();
    let (code, hint) = crate::error::code_of(&err);
    assert_eq!(code, Code::DeviceNotFound, "{err:#}");
    assert!(hint.unwrap().contains("USB connection"), "{err:#}");
    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = Some(vec![]));
    // A write that breaks off after the first confirmed one: "recoverable".
    fake_env(&[("FAKE_SPD_SHORT_WRITE", "vendor_boot_b")]);
    let (rep, _) = quiet();
    let err = flash::flash(&fx.catalog, &o(flash::Mode::Full), &rep).unwrap_err();
    assert!(format!("{err:#}").contains("still recoverable"), "{err:#}");

    // An unpacked partition changed after unpacking (same size, the stamp
    // still says this image): unpacked again before it's written.
    let flash_dir = job_dir.join("flash");
    let sup = flash_dir.join("super.img");
    let mut bytes = std::fs::read(&sup).unwrap();
    bytes[1000] ^= 0xFF;
    std::fs::write(&sup, &bytes).unwrap();
    fake_env(&[]);
    let (rep, sink) = quiet();
    flash::flash(&fx.catalog, &o(flash::Mode::Full), &rep).unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains(&format!("sha256 super {}", entry_sha("Super"))),
        "{text}"
    );
    assert!(sink.0.lock().unwrap().iter().any(|e| matches!(e,
        Event::Log { level, message } if level == "warn" && message.contains("super.img doesn't match"))));
    // job.json records every data entry's checksum.
    let rec = job::load_job(&job_dir).unwrap();
    let sup_entry = rec.entries.iter().find(|e| e.id == "Super").unwrap();
    assert_eq!(sup_entry.sha256, entry_sha("Super"));

    // Another run holds the workspace: flash, build and clean are refused.
    let lock = job::lock_workspace(&ws).unwrap();
    fake_env(&[]);
    let (rep, _) = quiet();
    let busy = |e: anyhow::Error| crate::error::code_of(&e).0 == Code::Busy;
    assert!(busy(
        flash::flash(&fx.catalog, &o(flash::Mode::Full), &rep).unwrap_err()
    ));
    assert!(!log.exists(), "spd_dump never ran");
    assert!(busy(
        job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap_err()
    ));
    assert!(busy(
        job::clean(&fx.catalog, &ws, &["scratch"], &[]).unwrap_err()
    ));
    // A dry run changes nothing and doesn't need the lock.
    let dry = flash::FlashOptions {
        dry_run: true,
        ..o(flash::Mode::Full)
    };
    flash::flash(&fx.catalog, &dry, &rep).unwrap();
    drop(lock);
    job::clean(&fx.catalog, &ws, &["scratch"], &[]).unwrap();
    tools::test::ENV.with(|e| e.borrow_mut().clear());
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = None);
    crate::detect::test::DEVICES.with(|d| *d.borrow_mut() = None);
}

#[test]
fn clean_removes_only_what_the_engine_made() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let (rep, _) = quiet();
    job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap();
    let cache = ws.join("cache");
    // Ours: a known download, its marker, scratch of known names.
    for (name, data) in [
        ("thimsy-system.img.zst.partial", &b"part"[..]),
        ("thimsy-system.img.zst.partial.json", b"{}"),
        ("vb.bsdiff.import", b"x"),
        ("vb.bsdiff.ok.tmp", b"x"),
    ] {
        std::fs::write(cache.join(name), data).unwrap();
    }
    // Not ours: the user's files in the cache and the workspace, a job
    // folder for no catalog device.
    std::fs::write(cache.join("my-notes.txt"), b"keep").unwrap();
    std::fs::write(cache.join("holiday.partial"), b"keep").unwrap();
    std::fs::write(ws.join("readme.txt"), b"keep").unwrap();
    let other_job = ws.join("jobs/someone-else");
    std::fs::create_dir_all(other_job.join("out")).unwrap();
    std::fs::write(other_job.join("out/x.pac"), b"keep").unwrap();
    let items = job::cleanable(&fx.catalog, &ws);
    let listed = |p: &Path| items.iter().any(|i| i.path == p);
    assert!(listed(&cache.join("vb.bsdiff")));
    assert!(listed(&cache.join("vb.bsdiff.import")));
    assert!(listed(&cache.join("thimsy-system.img.zst.partial.json")));
    assert!(
        !listed(&cache.join("vb.bsdiff.ok")),
        "markers go with their download"
    );
    for p in [
        cache.join("my-notes.txt"),
        cache.join("holiday.partial"),
        other_job.join("out"),
    ] {
        assert!(!listed(&p), "{} listed", p.display());
    }
    // Clean by path: only listed items, and only ones clean knows.
    let err = job::clean(&fx.catalog, &ws, &[], &[cache.join("my-notes.txt")]).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::BadInput);
    let freed = job::clean(&fx.catalog, &ws, &[], &[cache.join("vb.bsdiff")]).unwrap();
    assert!(freed > 0);
    assert!(!cache.join("vb.bsdiff").exists() && !cache.join("vb.bsdiff.ok").exists());
    assert!(cache.join("odm.bsdiff").exists(), "only the listed path");
    // Everything clean may remove, at once.
    job::clean(
        &fx.catalog,
        &ws,
        &["downloads", "scratch", "image", "backup"],
        &[],
    )
    .unwrap();
    for keep in [
        cache.join("my-notes.txt"),
        cache.join("holiday.partial"),
        ws.join("readme.txt"),
        other_job.join("out/x.pac"),
    ] {
        assert!(keep.exists(), "{} was removed", keep.display());
    }
    assert!(!cache.join("thimsy-system.img.zst.partial").exists());
    assert!(!cache.join("vb.bsdiff.import").exists());
    assert!(!cache.join("odm.bsdiff").exists());
}

#[test]
fn space_counts_picked_files_and_refuses_fat32_and_unknown_drives() {
    let fx = good_fixture();
    let ws = fx.dir.path().join("ws");
    let d = fx.catalog.device("testdev").unwrap();
    let sp = crate::resolve::space(&fx.catalog, d, "heavy");
    let empty = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], false).unwrap();
    // The unpacked official firmware picked: its download parts aren't
    // needed at all.
    let pac = fx.dir.path().join("my firmware.pac");
    std::fs::write(&pac, &fx.src_pac).unwrap();
    let sc = job::check_space(
        &fx.catalog,
        "testdev",
        "heavy",
        &ws,
        std::slice::from_ref(&pac),
        false,
    )
    .unwrap();
    let parts: u64 = d
        .source
        .as_ref()
        .unwrap()
        .parts
        .iter()
        .map(|p| p.size)
        .sum();
    assert_eq!(sc.imported, parts);
    assert_eq!(sc.build_need, sp.need(0, parts, false).0);
    assert!(sc.need < empty.need);
    // A folder holding the downloads: copied into the cache, so they still
    // take room but aren't downloaded.
    let sc = job::check_space(
        &fx.catalog,
        "testdev",
        "heavy",
        &ws,
        &[fx.dir.path().to_path_buf()],
        false,
    )
    .unwrap();
    assert_eq!(sc.imported, sc.downloads);
    assert_eq!(sc.need, empty.need);

    // FAT32 can't hold the image; a drive that can't be checked is never
    // assumed to have room.
    let probe = |r: Result<crate::disk::Probe, String>| {
        crate::disk::test::PROBE.with(|p| *p.borrow_mut() = Some(r))
    };
    let mut cat = fx.catalog.clone();
    cat.devices[0].source.as_mut().unwrap().container.size = 5 << 30;
    probe(Ok(crate::disk::Probe {
        free: u64::MAX / 2,
        fat32: true,
    }));
    let sc = job::check_space(&cat, "testdev", "heavy", &ws, &[], false).unwrap();
    assert!(!sc.ok);
    assert_eq!(sc.reason, Some("fat32"));
    assert!(sc.hint.as_deref().unwrap().contains("FAT32"));
    // Small enough for FAT32: fine there.
    let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], false).unwrap();
    assert!(sc.ok, "{sc:?}");
    probe(Err("permission denied".into()));
    let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], false).unwrap();
    assert!(!sc.ok);
    assert_eq!((sc.free, sc.reason), (0, Some("probe_failed")));
    let (rep, _) = quiet();
    let err = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::BadInput, "{err:#}");
    probe(Ok(crate::disk::Probe {
        free: 1000,
        fat32: false,
    }));
    let sc = job::check_space(&fx.catalog, "testdev", "heavy", &ws, &[], false).unwrap();
    assert_eq!(sc.reason, Some("not_enough_space"));
    let err = job::build(&fx.catalog, &opts(&ws, false), &rep).unwrap_err();
    assert_eq!(
        crate::error::code_of(&err).0,
        Code::NotEnoughSpace,
        "{err:#}"
    );
    crate::disk::test::PROBE.with(|p| *p.borrow_mut() = None);
}

#[test]
fn renamed_device_pack_parts_are_matched_and_missing_ones_named() {
    let mut fx = good_fixture();
    let d = fx.dir.path().to_path_buf();
    let files = ["thimsy-system.img.zst", "vb.bsdiff", "odm.bsdiff"].map(|f| d.join(f));
    let refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
    let entry =
        crate::bundle::make(&d.join("pack.tar"), &refs, u64::MAX, 1, &mut |_, _| {}).unwrap();
    for f in &files {
        std::fs::remove_file(f).unwrap();
    }
    let bytes = std::fs::read(d.join("pack.tar")).unwrap();
    let mut bundle: crate::catalog::Artifact = serde_json::from_value(entry).unwrap();
    // Split into parts with download addresses (which must not be used).
    let parts_dir = d.join("published");
    std::fs::create_dir_all(&parts_dir).unwrap();
    let chunk = bytes.len() / 3 + 1;
    bundle.parts = bytes
        .chunks(chunk)
        .enumerate()
        .map(|(i, c)| {
            let p = parts_dir.join(format!("pack.tar.{:03}", i + 1));
            std::fs::write(&p, c).unwrap();
            serde_yaml::from_str(&artifact_yaml(&format!("pack.tar.{:03}", i + 1), &p)).unwrap()
        })
        .collect();
    fx.catalog.devices[0]
        .bundles
        .insert("heavy".into(), bundle.clone());
    // The user's copies, renamed (the browser added " (1)").
    let mine: Vec<PathBuf> = (1..=3)
        .map(|i| {
            let p = d.join(format!("download ({i}).bin"));
            std::fs::copy(parts_dir.join(format!("pack.tar.{i:03}")), &p).unwrap();
            p
        })
        .collect();
    let mut o = opts(&d.join("ws"), false);
    o.imports = mine.clone();
    let (rep, _) = quiet();
    assert_eq!(
        job::build(&fx.catalog, &o, &rep).unwrap().sha256,
        fx.expected_sha
    );
    // One part named as published, the others renamed: still matched.
    std::fs::copy(&mine[1], d.join("pack.tar.002")).unwrap();
    let mut o = opts(&d.join("ws2"), false);
    o.imports = vec![mine[0].clone(), d.join("pack.tar.002"), mine[2].clone()];
    assert_eq!(
        job::build(&fx.catalog, &o, &rep).unwrap().sha256,
        fx.expected_sha
    );
    // A part missing: bad_input naming it, and nothing downloaded.
    let mut o = opts(&d.join("ws3"), false);
    o.imports = vec![mine[0].clone(), mine[2].clone()];
    let err = job::build(&fx.catalog, &o, &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::BadInput, "{err:#}");
    assert!(err.to_string().contains("pack.tar.002"), "{err}");
    assert!(!d.join("ws3/cache/pack.tar.002").exists());
    assert!(!d.join("ws3/cache/pack.tar.002.partial").exists());
}
