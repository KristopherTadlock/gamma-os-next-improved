//! Fixture builders for tests: synthetic PACs, super images, 7z archives and
//! catalogs, so every module is tested without real firmware.
#![cfg(test)]

use crate::lp::{self, Super};
use crate::pac::{self, ENTRY_SIZE, HEADER_SIZE};
use crate::util::sha256_hex;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub fn utf16(s: &str, units: usize) -> Vec<u8> {
    let mut v: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    v.resize(units * 2, 0);
    v
}

/// One PAC entry: (id, file name, data).
pub struct E<'a>(pub &'a str, pub &'a str, pub Vec<u8>);

/// Builds a BP_R2 PAC with valid header and data CRCs.
pub fn build_pac(entries: &[E], version: &str, alias: &str) -> Vec<u8> {
    let mut header = vec![0u8; HEADER_SIZE as usize];
    header[..48].copy_from_slice(&utf16("BP_R2.0.1", 24));
    header[564..564 + 512].copy_from_slice(&utf16(version, 256));
    header[1104..1104 + 200].copy_from_slice(&utf16(alias, 100));
    let start = HEADER_SIZE as u32;
    header[1076..1080].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    header[1080..1084].copy_from_slice(&start.to_le_bytes());
    let mut table = Vec::new();
    let mut data = Vec::new();
    let mut offset = HEADER_SIZE + (entries.len() * ENTRY_SIZE) as u64;
    for E(id, file, d) in entries {
        let mut raw = vec![0u8; ENTRY_SIZE];
        raw[4..516].copy_from_slice(&utf16(id, 256));
        raw[516..1028].copy_from_slice(&utf16(file, 256));
        let size = d.len() as u64;
        let off = if size == 0 { 0 } else { offset };
        raw[1532..1536].copy_from_slice(&((size >> 32) as u32).to_le_bytes());
        raw[1536..1540].copy_from_slice(&((off >> 32) as u32).to_le_bytes());
        raw[1540..1544].copy_from_slice(&(size as u32).to_le_bytes());
        raw[1552..1556].copy_from_slice(&(off as u32).to_le_bytes());
        table.extend_from_slice(&raw);
        data.extend_from_slice(d);
        offset += size;
    }
    let mut out = header;
    out.extend_from_slice(&table);
    out.extend_from_slice(&data);
    let crc1 = pac::crc16_arc(0, &out[..HEADER_SIZE as usize - 4]);
    out[HEADER_SIZE as usize - 4..HEADER_SIZE as usize - 2].copy_from_slice(&crc1.to_le_bytes());
    let crc2 = pac::crc16_arc(0, &out[HEADER_SIZE as usize..]);
    out[HEADER_SIZE as usize - 2..HEADER_SIZE as usize].copy_from_slice(&crc2.to_le_bytes());
    out
}

/// A PAC config XML for the given (id, type, block, base, backup) files.
pub fn config_xml(files: &[(&str, &str, Option<&str>, u64, bool)]) -> Vec<u8> {
    let mut x = String::from("<?xml version=\"1.0\"?>\n<BMAConfig><ProductList><Product name=\"test\"/></ProductList><SchemeList><Scheme name=\"test\">\n");
    for (id, kind, block, base, backup) in files {
        let b = match block {
            Some(n) => format!("<Block id=\"{n}\"><Base>0x{base:x}</Base><Size>0x0</Size></Block>"),
            None => format!("<Block><Base>0x{base:x}</Base><Size>0x0</Size></Block>"),
        };
        let attr = if *backup { " backup=\"1\"" } else { "" };
        x.push_str(&format!("<File{attr}><ID>{id}</ID><IDAlias>{id}</IDAlias><Type>{kind}</Type>{b}<Flag>1</Flag><CheckFlag>0</CheckFlag></File>\n"));
    }
    x.push_str("</Scheme></SchemeList></BMAConfig>\n");
    x.into_bytes()
}

pub fn name36(s: &str) -> [u8; 36] {
    let mut n = [0u8; 36];
    n[..s.len()].copy_from_slice(s.as_bytes());
    n
}

/// A raw super image: `len` bytes, 1 MiB alignment, one group (`max` bytes,
/// 0 = unlimited) and the given (name, extent offset MiB, size MiB) partitions.
pub fn build_super(len: usize, group_max: u64, parts: &[(&str, u64, u64)]) -> Vec<u8> {
    let mut img = vec![0u8; len];
    let max_size: u32 = 65536;
    let slots: u32 = 2;
    img[4096..4100].copy_from_slice(&0x616C_4467u32.to_le_bytes());
    img[4096 + 40..4096 + 44].copy_from_slice(&max_size.to_le_bytes());
    img[4096 + 44..4096 + 48].copy_from_slice(&slots.to_le_bytes());
    img[4096 + 48..4096 + 52].copy_from_slice(&4096u32.to_le_bytes());
    let mib = 1u64 << 20;
    let mut partitions = Vec::new();
    let mut extents = Vec::new();
    for (name, at, size) in parts {
        let first = extents.len() as u32;
        let count = if *size == 0 { 0 } else { 1 };
        if *size > 0 {
            extents.push(lp::ExtentRow {
                sectors: size * mib / 512,
                target_type: 0,
                target_data: at * mib / 512,
                target_source: 0,
            });
        }
        partitions.push(lp::PartitionRow {
            name: name36(name),
            attributes: 1,
            first_extent: first,
            extent_count: count,
            group: 1,
        });
    }
    let meta = lp::Metadata::for_test(
        max_size,
        slots,
        partitions,
        extents,
        vec![
            lp::GroupRow {
                name: name36("default"),
                flags: 0,
                maximum_size: 0,
            },
            lp::GroupRow {
                name: name36("main"),
                flags: 0,
                maximum_size: group_max,
            },
        ],
        vec![lp::DeviceRow {
            first_logical_sector: 2048,
            alignment: 1 << 20,
            alignment_offset: 0,
            size: len as u64,
            name: name36("super"),
            flags: 0,
        }],
    );
    let mut f = tempfile::tempfile().unwrap();
    f.write_all(&img).unwrap();
    Super::new(&mut f, 0, len as u64)
        .write_metadata_for_test(&meta)
        .unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    f.read_to_end(&mut out).unwrap();
    out
}

pub fn pattern(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2654435761).wrapping_add(1);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x & 0xff) as u8
        })
        .collect()
}

/// Writes an encrypted 7z holding `file_name` → `data`, split into `parts`.
pub fn build_7z(
    dir: &Path,
    stem: &str,
    file_name: &str,
    data: &[u8],
    password: &str,
    parts: usize,
) -> Vec<PathBuf> {
    let src_dir = dir.join(format!("{stem}-src"));
    std::fs::create_dir_all(&src_dir).unwrap();
    let src = src_dir.join(file_name);
    std::fs::write(&src, data).unwrap();
    let archive = dir.join(format!("{stem}.7z"));
    let mut w = sevenz_rust2::ArchiveWriter::create(&archive).unwrap();
    w.set_content_methods(vec![
        sevenz_rust2::encoder_options::AesEncoderOptions::new(password.into()).into(),
        sevenz_rust2::EncoderMethod::LZMA2.into(),
    ]);
    w.push_source_path(&src, |_| true).unwrap();
    w.finish().unwrap();
    let bytes = std::fs::read(&archive).unwrap();
    std::fs::remove_file(&archive).unwrap();
    let chunk = bytes.len().div_ceil(parts);
    bytes
        .chunks(chunk)
        .enumerate()
        .map(|(i, c)| {
            let p = dir.join(format!("{stem}.7z.{:03}", i + 1));
            std::fs::write(&p, c).unwrap();
            p
        })
        .collect()
}

/// A `file://` URL usable inside double-quoted YAML on every OS.
pub fn file_url(path: &Path) -> String {
    format!("file://{}", path.display().to_string().replace('\\', "/"))
}

pub fn artifact_yaml(id: &str, path: &Path) -> String {
    let data = std::fs::read(path).unwrap();
    format!(
        "{{ id: {id}, file: {}, size: {}, sha256: \"{}\", urls: [\"{}\"] }}",
        path.file_name().unwrap().to_string_lossy(),
        data.len(),
        sha256_hex(&data),
        file_url(path)
    )
}
