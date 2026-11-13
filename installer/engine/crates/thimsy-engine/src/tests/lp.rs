use crate::lp::{Extent, Super};
use crate::testutil::*;
use std::io::{Read, Seek, SeekFrom, Write};

const MIB: u64 = 1 << 20;

fn file_with(data: &[u8], prefix: usize) -> std::fs::File {
    let mut f = tempfile::tempfile().unwrap();
    f.write_all(&vec![0xCD; prefix]).unwrap();
    f.write_all(data).unwrap();
    f
}

#[test]
fn lists_partitions_and_extents() {
    let img = build_super(
        16 << 20,
        0,
        &[("system_a", 1, 4), ("system_b", 0, 0), ("odm_a", 5, 1)],
    );
    let mut f = file_with(&img, 0);
    let mut s = Super::new(&mut f, 0, img.len() as u64);
    let parts = s.partitions().unwrap();
    assert_eq!(parts.len(), 3);
    assert_eq!(
        parts[0].1,
        vec![Extent {
            offset: MIB,
            len: 4 * MIB
        }]
    );
    assert!(parts[1].1.is_empty());
    assert!(s.extents("nope").is_err());
    let m = s.metadata().unwrap();
    assert_eq!((m.slots, m.max_size), (2, 65536));
}

#[test]
fn grow_respects_device_end_group_limit_and_explicit_limit() {
    // Device 16 MiB; partitions end at 6 MiB → 10 MiB free.
    let img = build_super(16 << 20, 0, &[("system_a", 1, 4), ("odm_a", 5, 1)]);
    let mut f = file_with(&img, 0);
    assert_eq!(
        Super::new(&mut f, 0, img.len() as u64)
            .grow("system_a", None)
            .unwrap(),
        10 * MIB
    );

    // Group max 8 MiB with 5 used → 3 MiB.
    let img = build_super(16 << 20, 8 * MIB, &[("system_a", 1, 4), ("odm_a", 5, 1)]);
    let mut f = file_with(&img, 0);
    assert_eq!(
        Super::new(&mut f, 0, img.len() as u64)
            .grow("system_a", None)
            .unwrap(),
        3 * MIB
    );

    // Explicit limit, rounded down to alignment.
    let img = build_super(16 << 20, 0, &[("system_a", 1, 4), ("odm_a", 5, 1)]);
    let mut f = file_with(&img, 0);
    let mut s = Super::new(&mut f, 0, img.len() as u64);
    assert_eq!(s.grow("system_a", Some(2 * MIB + 5)).unwrap(), 2 * MIB);
    // The new extent sits after the last one; odm keeps its place.
    assert_eq!(
        s.extents("system_a").unwrap()[1],
        Extent {
            offset: 6 * MIB,
            len: 2 * MIB
        }
    );
    assert_eq!(
        s.extents("odm_a").unwrap(),
        vec![Extent {
            offset: 5 * MIB,
            len: MIB
        }]
    );

    // Full device: nothing to grow into.
    let img = build_super(8 << 20, 0, &[("system_a", 1, 7)]);
    let mut f = file_with(&img, 0);
    assert!(Super::new(&mut f, 0, img.len() as u64)
        .grow("system_a", None)
        .is_err());
}

#[test]
fn grow_keeps_every_metadata_slot_identical_and_valid() {
    let img = build_super(16 << 20, 0, &[("system_a", 1, 4), ("odm_a", 5, 1)]);
    let mut f = file_with(&img, 0);
    Super::new(&mut f, 0, img.len() as u64)
        .grow("system_a", None)
        .unwrap();
    let mut all = Vec::new();
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_to_end(&mut all).unwrap();
    let slot = |i: usize| &all[4096 * 3 + i * 65536..4096 * 3 + (i + 1) * 65536];
    for i in 1..4 {
        assert_eq!(slot(0), slot(i), "slot {i} differs");
    }
    // And it parses back with checksums intact.
    let mut s = Super::new(&mut f, 0, img.len() as u64);
    assert_eq!(s.capacity("system_a").unwrap(), 14 * MIB);
}

#[test]
fn replace_and_extract_across_two_extents_inside_a_container() {
    let img = build_super(16 << 20, 0, &[("system_a", 1, 2), ("odm_a", 3, 1)]);
    let prefix = 12345;
    let mut f = file_with(&img, prefix);
    let mut s = Super::new(&mut f, prefix as u64, img.len() as u64);
    s.grow("system_a", Some(2 * MIB)).unwrap();
    let data = pattern((3 * MIB + 777) as usize, 4);
    s.replace(
        "system_a",
        &mut &data[..],
        data.len() as u64,
        &mut |_, _| {},
    )
    .unwrap();
    let mut out = Vec::new();
    s.extract("system_a", &mut out, &mut |_, _| {}).unwrap();
    assert_eq!(out.len() as u64, 4 * MIB);
    assert_eq!(&out[..data.len()], &data[..]);
    assert!(
        out[data.len()..].iter().all(|&b| b == 0),
        "tail zero-filled"
    );
    // odm untouched, container prefix untouched.
    assert!(s.read_partition("odm_a").unwrap().iter().all(|&b| b == 0));
    let mut head = vec![0u8; prefix];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut head).unwrap();
    assert!(head.iter().all(|&b| b == 0xCD));
}

#[test]
fn rejects_corrupt_or_hostile_metadata() {
    let img = build_super(8 << 20, 0, &[("system_a", 1, 2)]);
    // Bad geometry magic.
    let mut bad = img.clone();
    bad[4096] ^= 1;
    let mut f = file_with(&bad, 0);
    assert!(Super::new(&mut f, 0, bad.len() as u64).metadata().is_err());
    // Header checksum.
    let mut bad = img.clone();
    bad[4096 * 3 + 20] ^= 1;
    let mut f = file_with(&bad, 0);
    assert!(Super::new(&mut f, 0, bad.len() as u64).metadata().is_err());
    // Extent beyond the end of the image (declared region smaller than the image).
    let mut f = file_with(&img, 0);
    assert!(Super::new(&mut f, 0, 2 << 20).partitions().is_err());
    // Implausible geometry.
    let mut bad = img.clone();
    bad[4096 + 40..4096 + 44].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut f = file_with(&bad, 0);
    assert!(Super::new(&mut f, 0, bad.len() as u64).metadata().is_err());
}

/// Extents and group tables from a crafted image: errors, never an
/// overflow (which panics with overflow checks on, as release builds have).
#[test]
fn crafted_extents_are_errors_not_overflows() {
    let img = build_super(16 << 20, 8 * MIB, &[("system_a", 1, 4), ("odm_a", 5, 1)]);
    for (sectors, target) in [(8, u64::MAX / 256), (u64::MAX / 2, 8), (u64::MAX, u64::MAX)] {
        let mut f = file_with(&img, 0);
        let mut s = Super::new(&mut f, 0, img.len() as u64);
        let mut m = s.metadata().unwrap();
        m.extents[0].sectors = sectors;
        m.extents[0].target_data = target;
        s.write_metadata_for_test(&m).unwrap();
        let started = std::time::Instant::now();
        assert!(s.partitions().is_err(), "{sectors} {target}");
        assert!(s.grow("odm_a", None).is_err(), "{sectors} {target}");
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
    // A partition whose extents run past the table (or wrap around).
    let mut f = file_with(&img, 0);
    let mut s = Super::new(&mut f, 0, img.len() as u64);
    let mut m = s.metadata().unwrap();
    m.partitions[0].first_extent = u32::MAX;
    m.partitions[0].extent_count = 2;
    s.write_metadata_for_test(&m).unwrap();
    assert!(s.partitions().is_err());
    assert!(s.grow("odm_a", None).is_err());
}
