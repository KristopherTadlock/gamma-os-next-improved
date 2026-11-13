use crate::pac::{self, Config, Pac};
use crate::testutil::*;
use std::io::Write;

fn sample() -> Vec<u8> {
    build_pac(
        &[
            E("FDL", "fdl1.bin", pattern(1000, 1)),
            E("EraseUBOOTLOG", "", vec![]),
            E("UBOOTLoader", "uboot.img", pattern(3000, 2)),
            E("UBOOTLoader", "uboot.img", pattern(3000, 2)),
            E("Super", "super.img", pattern(5000, 3)),
            E(
                "",
                "cfg.xml",
                config_xml(&[("FDL", "FDL", None, 0x5500, false)]),
            ),
        ],
        "Test_v1",
        "TEST",
    )
}

fn write_tmp(data: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(data).unwrap();
    f
}

#[test]
fn opens_and_finds_entries() {
    let f = write_tmp(&sample());
    let p = Pac::open(f.path()).unwrap();
    assert_eq!(p.entries.len(), 6);
    assert_eq!(p.version(), "Test_v1");
    assert_eq!(p.alias(), "TEST");
    // Empty entries are skipped by find; IDs and file names both work.
    assert!(p.find("EraseUBOOTLOG").is_err());
    assert_eq!(p.find("super.img").unwrap().id, "Super");
    assert_eq!(
        p.read_entry(p.find("Super").unwrap()).unwrap(),
        pattern(5000, 3)
    );
    // Duplicate IDs keep their own positions.
    let ub: Vec<_> = p
        .entries
        .iter()
        .filter(|e| e.id == "UBOOTLoader")
        .map(|e| e.index)
        .collect();
    assert_eq!(ub, vec![2, 3]);
    assert_eq!(p.config().unwrap().file("FDL").unwrap().base, 0x5500);
}

#[test]
fn extract_and_replace_roundtrip() {
    let f = write_tmp(&sample());
    let p = Pac::open(f.path()).unwrap();
    let e = p.find("Super").unwrap().clone();
    let out = tempfile::NamedTempFile::new().unwrap();
    let mut seen = 0;
    p.extract(&e, out.path(), &mut |n| seen = n).unwrap();
    assert_eq!(seen, 5000);
    assert_eq!(std::fs::read(out.path()).unwrap(), pattern(5000, 3));
    let new = pattern(5000, 9);
    pac::replace(f.path(), &e, &new).unwrap();
    assert_eq!(Pac::open(f.path()).unwrap().read_entry(&e).unwrap(), new);
    assert!(
        pac::replace(f.path(), &e, &new[..10]).is_err(),
        "size must match exactly"
    );
}

#[test]
fn set_info_and_fix_crc_produce_valid_crcs() {
    let f = write_tmp(&sample());
    let p = Pac::open(f.path()).unwrap();
    let e = p.find("Super").unwrap().clone();
    pac::replace(f.path(), &e, &pattern(5000, 7)).unwrap();
    pac::set_info(f.path(), "Thimsy_heavy", "RG_Cube").unwrap();
    pac::fix_crc(f.path(), &mut |_, _| {}).unwrap();
    let data = std::fs::read(f.path()).unwrap();
    let (c1, c2) = pac::stored_crcs(f.path()).unwrap();
    assert_eq!(c1, pac::crc16_arc(0, &data[..2120]));
    assert_eq!(c2, pac::crc16_arc(0, &data[2124..]));
    let p = Pac::open(f.path()).unwrap();
    assert_eq!(p.version(), "Thimsy_heavy");
    assert_eq!(p.alias(), "RG_Cube");
    // A rebuilt PAC from the same inputs is identical (deterministic).
    let expect = build_pac(
        &[
            E("FDL", "fdl1.bin", pattern(1000, 1)),
            E("EraseUBOOTLOG", "", vec![]),
            E("UBOOTLoader", "uboot.img", pattern(3000, 2)),
            E("UBOOTLoader", "uboot.img", pattern(3000, 2)),
            E("Super", "super.img", pattern(5000, 7)),
            E(
                "",
                "cfg.xml",
                config_xml(&[("FDL", "FDL", None, 0x5500, false)]),
            ),
        ],
        "Thimsy_heavy",
        "RG_Cube",
    );
    // build_pac computes the same CRCs from scratch.
    assert_eq!(data.len(), expect.len());
    assert_eq!(data, expect);
}

#[test]
fn rejects_bad_input() {
    // Not a PAC.
    let f = write_tmp(&vec![0u8; 4096]);
    assert!(Pac::open(f.path()).is_err());
    // Too small.
    let f = write_tmp(b"tiny");
    assert!(Pac::open(f.path()).is_err());
    // Truncated: entry runs past the end of the file.
    let mut d = sample();
    d.truncate(d.len() - 100);
    let f = write_tmp(&d);
    let err = Pac::open(f.path()).unwrap_err().to_string();
    assert!(err.contains("truncated"), "{err}");
    // Absurd entry count.
    let mut d = sample();
    d[1076..1080].copy_from_slice(&100_000u32.to_le_bytes());
    let f = write_tmp(&d);
    assert!(Pac::open(f.path()).is_err());
    // Strings too long for their fields.
    let f = write_tmp(&sample());
    assert!(pac::set_info(f.path(), &"x".repeat(300), "a").is_err());
    assert!(pac::set_info(f.path(), "v", &"x".repeat(100)).is_err());
}

#[test]
fn pair_checks_order() {
    let f = write_tmp(&sample());
    let p = Pac::open(f.path()).unwrap();
    let good = Config::parse(
        std::str::from_utf8(&config_xml(&[
            ("FDL", "FDL", None, 0x5500, false),
            ("EraseUBOOTLOG", "EraseFlash2", Some("uboot_log"), 0, false),
            ("UBOOTLoader", "UBOOT_LOADER2", Some("uboot_a"), 0, false),
            ("UBOOTLoader", "UBOOT_LOADER2", Some("uboot_b"), 0, false),
            ("Super", "YAFFS_IMG2", Some("super"), 0, false),
        ]))
        .unwrap(),
    )
    .unwrap();
    let pairs = pac::pair(&p, &good).unwrap();
    assert_eq!(pairs[3].1.block.as_deref(), Some("uboot_b"));
    let bad = Config::parse(
        std::str::from_utf8(&config_xml(&[("Super", "X", Some("super"), 0, false)])).unwrap(),
    )
    .unwrap();
    assert!(pac::pair(&p, &bad).is_err());
    // Counts that differ are an error, not a silently shorter list: a
    // config that stops early (Super would get no partition)...
    let short = Config::parse(
        std::str::from_utf8(&config_xml(&[
            ("FDL", "FDL", None, 0x5500, false),
            ("EraseUBOOTLOG", "EraseFlash2", Some("uboot_log"), 0, false),
            ("UBOOTLoader", "UBOOT_LOADER2", Some("uboot_a"), 0, false),
        ]))
        .unwrap(),
    )
    .unwrap();
    let err = pac::pair(&p, &short).unwrap_err().to_string();
    assert!(
        err.contains("UBOOTLoader") && err.contains("no file"),
        "{err}"
    );
    // ...or lists more files than the PAC has entries.
    let mut files = vec![
        ("FDL", "FDL", None, 0x5500, false),
        ("EraseUBOOTLOG", "EraseFlash2", Some("uboot_log"), 0, false),
        ("UBOOTLoader", "UBOOT_LOADER2", Some("uboot_a"), 0, false),
        ("UBOOTLoader", "UBOOT_LOADER2", Some("uboot_b"), 0, false),
        ("Super", "YAFFS_IMG2", Some("super"), 0, false),
    ];
    files.extend([
        ("", "X", None, 0, false),
        ("Extra", "CODE2", Some("x"), 0, false),
    ]);
    let long = Config::parse(std::str::from_utf8(&config_xml(&files)).unwrap()).unwrap();
    let err = pac::pair(&p, &long).unwrap_err().to_string();
    assert!(err.contains("7 files but the PAC has only 6"), "{err}");
    assert!(Config::parse("<not xml").is_err());
    assert!(Config::parse("<BMAConfig/>").is_err(), "no files");
}

#[test]
fn parse_num_handles_hex_and_decimal() {
    assert_eq!(pac::parse_num("0x65000800"), Some(0x65000800));
    assert_eq!(pac::parse_num(" 0XFF "), Some(255));
    assert_eq!(pac::parse_num("42"), Some(42));
    assert_eq!(pac::parse_num("zz"), None);
}
