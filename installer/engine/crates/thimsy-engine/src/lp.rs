//! Raw (non-sparse) Android `super` images (liblp metadata), edited in place,
//! possibly as a region inside a larger file such as a PAC. A port of
//! `scripts/lptool.py`.

use crate::util::{self, CHUNK};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

const GEOMETRY_OFFSET: u64 = 4096;
const GEOMETRY_MAGIC: u32 = 0x616C_4467;
const METADATA_OFFSET: u64 = 4096 * 3;
const HEADER_MAGIC: u32 = 0x414C_5030;
pub const SECTOR: u64 = 512;

const PARTITION_SIZE: usize = 52; // name[36], attributes, first extent, extent count, group
const EXTENT_SIZE: usize = 24; // sectors u64, target type u32, target data u64, target source u32
const GROUP_SIZE: usize = 48; // name[36], flags u32, maximum size u64
const DEVICE_SIZE: usize = 64; // first logical sector u64, alignment, offset, size u64, name[36], flags

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionRow {
    pub name: [u8; 36],
    pub attributes: u32,
    pub first_extent: u32,
    pub extent_count: u32,
    pub group: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtentRow {
    pub sectors: u64,
    pub target_type: u32,
    pub target_data: u64,
    pub target_source: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    pub name: [u8; 36],
    pub flags: u32,
    pub maximum_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRow {
    pub first_logical_sector: u64,
    pub alignment: u32,
    pub alignment_offset: u32,
    pub size: u64,
    pub name: [u8; 36],
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub max_size: u32,
    pub slots: u32,
    pub block_size: u32,
    header: Vec<u8>,
    pub partitions: Vec<PartitionRow>,
    pub extents: Vec<ExtentRow>,
    pub groups: Vec<GroupRow>,
    pub devices: Vec<DeviceRow>,
}

/// One linear extent of a logical partition, as a byte range of the super image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub offset: u64,
    pub len: u64,
}

/// A super image living at `base..base+len` of `file`.
pub struct Super<'f> {
    pub file: &'f mut File,
    pub base: u64,
    pub len: u64,
}

fn name36(raw: &[u8]) -> [u8; 36] {
    raw[..36].try_into().unwrap()
}

pub fn name_str(raw: &[u8; 36]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(36);
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

#[cfg(test)]
impl Metadata {
    pub fn for_test(
        max_size: u32,
        slots: u32,
        partitions: Vec<PartitionRow>,
        extents: Vec<ExtentRow>,
        groups: Vec<GroupRow>,
        devices: Vec<DeviceRow>,
    ) -> Metadata {
        let mut header = vec![0u8; 128];
        header[0..4].copy_from_slice(&HEADER_MAGIC.to_le_bytes());
        header[4..6].copy_from_slice(&10u16.to_le_bytes());
        header[8..12].copy_from_slice(&128u32.to_le_bytes());
        Metadata {
            max_size,
            slots,
            block_size: 4096,
            header,
            partitions,
            extents,
            groups,
            devices,
        }
    }
}

impl<'f> Super<'f> {
    #[cfg(test)]
    pub fn write_metadata_for_test(&mut self, meta: &Metadata) -> Result<()> {
        self.write_metadata(meta)
    }

    pub fn new(file: &'f mut File, base: u64, len: u64) -> Self {
        Super { file, base, len }
    }

    fn read(&mut self, off: u64, n: usize) -> Result<Vec<u8>> {
        if off + n as u64 > self.len {
            bail!("read past the end of the super image");
        }
        util::read_at(self.file, self.base + off, n)
    }

    fn write(&mut self, off: u64, data: &[u8]) -> Result<()> {
        if off + data.len() as u64 > self.len {
            bail!("write past the end of the super image");
        }
        util::write_at(self.file, self.base + off, data)
    }

    pub fn metadata(&mut self) -> Result<Metadata> {
        let geometry = self.read(GEOMETRY_OFFSET, 4096)?;
        if util::u32_at(&geometry, 0) != GEOMETRY_MAGIC {
            bail!("not a raw super image (bad geometry magic)");
        }
        let max_size = util::u32_at(&geometry, 40);
        let slots = util::u32_at(&geometry, 44);
        let block_size = util::u32_at(&geometry, 48);
        if max_size == 0 || max_size > 1 << 20 || slots == 0 || slots > 8 {
            bail!("implausible super geometry (metadata {max_size} bytes, {slots} slots)");
        }
        let raw = self.read(METADATA_OFFSET, max_size as usize)?;
        if util::u32_at(&raw, 0) != HEADER_MAGIC {
            bail!("bad super metadata header magic");
        }
        let header_size = util::u32_at(&raw, 8) as usize;
        if !(128..=raw.len()).contains(&header_size) {
            bail!("bad super metadata header size {header_size}");
        }
        let header = raw[..header_size].to_vec();
        let tables_size = util::u32_at(&header, 44) as usize;
        if header_size + tables_size > raw.len() {
            bail!("super metadata tables overflow their slot");
        }
        let tables = &raw[header_size..header_size + tables_size];
        let mut check = header.clone();
        check[12..44].fill(0);
        if Sha256::digest(&check).as_slice() != &header[12..44] {
            bail!("super metadata header checksum mismatch");
        }
        if Sha256::digest(tables).as_slice() != &header[48..80] {
            bail!("super metadata tables checksum mismatch");
        }
        let desc = |i: usize| -> Result<(usize, usize, usize)> {
            let o = util::u32_at(&header, 80 + i * 12) as usize;
            let c = util::u32_at(&header, 84 + i * 12) as usize;
            let s = util::u32_at(&header, 88 + i * 12) as usize;
            if o.checked_add(c.checked_mul(s).context("table overflow")?)
                .context("table overflow")?
                > tables.len()
            {
                bail!("super metadata table {i} out of range");
            }
            Ok((o, c, s))
        };
        let rows = |i: usize, want: usize| -> Result<Vec<&[u8]>> {
            let (o, c, s) = desc(i)?;
            if s < want {
                bail!("super metadata table {i} entries are too small");
            }
            Ok((0..c).map(|k| &tables[o + k * s..o + k * s + s]).collect())
        };
        let partitions = rows(0, PARTITION_SIZE)?
            .into_iter()
            .map(|r| PartitionRow {
                name: name36(r),
                attributes: util::u32_at(r, 36),
                first_extent: util::u32_at(r, 40),
                extent_count: util::u32_at(r, 44),
                group: util::u32_at(r, 48),
            })
            .collect();
        let extents = rows(1, EXTENT_SIZE)?
            .into_iter()
            .map(|r| ExtentRow {
                sectors: util::u64_at(r, 0),
                target_type: util::u32_at(r, 8),
                target_data: util::u64_at(r, 12),
                target_source: util::u32_at(r, 20),
            })
            .collect();
        let groups = rows(2, GROUP_SIZE)?
            .into_iter()
            .map(|r| GroupRow {
                name: name36(r),
                flags: util::u32_at(r, 36),
                maximum_size: util::u64_at(r, 40),
            })
            .collect();
        let devices = rows(3, DEVICE_SIZE)?
            .into_iter()
            .map(|r| DeviceRow {
                first_logical_sector: util::u64_at(r, 0),
                alignment: util::u32_at(r, 8),
                alignment_offset: util::u32_at(r, 12),
                size: util::u64_at(r, 16),
                name: name36(&r[24..60]),
                flags: util::u32_at(r, 60),
            })
            .collect();
        Ok(Metadata {
            max_size,
            slots,
            block_size,
            header,
            partitions,
            extents,
            groups,
            devices,
        })
    }

    fn write_metadata(&mut self, meta: &Metadata) -> Result<()> {
        let mut tables = Vec::new();
        let mut descs = Vec::new();
        let mut push = |rows: Vec<Vec<u8>>, size: usize, tables: &mut Vec<u8>| {
            descs.push((tables.len() as u32, rows.len() as u32, size as u32));
            for r in rows {
                tables.extend_from_slice(&r);
            }
        };
        push(
            meta.partitions
                .iter()
                .map(|p| {
                    let mut r = p.name.to_vec();
                    for v in [p.attributes, p.first_extent, p.extent_count, p.group] {
                        r.extend_from_slice(&v.to_le_bytes());
                    }
                    r
                })
                .collect(),
            PARTITION_SIZE,
            &mut tables,
        );
        push(
            meta.extents
                .iter()
                .map(|e| {
                    let mut r = e.sectors.to_le_bytes().to_vec();
                    r.extend_from_slice(&e.target_type.to_le_bytes());
                    r.extend_from_slice(&e.target_data.to_le_bytes());
                    r.extend_from_slice(&e.target_source.to_le_bytes());
                    r
                })
                .collect(),
            EXTENT_SIZE,
            &mut tables,
        );
        push(
            meta.groups
                .iter()
                .map(|g| {
                    let mut r = g.name.to_vec();
                    r.extend_from_slice(&g.flags.to_le_bytes());
                    r.extend_from_slice(&g.maximum_size.to_le_bytes());
                    r
                })
                .collect(),
            GROUP_SIZE,
            &mut tables,
        );
        push(
            meta.devices
                .iter()
                .map(|d| {
                    let mut r = d.first_logical_sector.to_le_bytes().to_vec();
                    r.extend_from_slice(&d.alignment.to_le_bytes());
                    r.extend_from_slice(&d.alignment_offset.to_le_bytes());
                    r.extend_from_slice(&d.size.to_le_bytes());
                    r.extend_from_slice(&d.name);
                    r.extend_from_slice(&d.flags.to_le_bytes());
                    r
                })
                .collect(),
            DEVICE_SIZE,
            &mut tables,
        );
        let mut header = meta.header.clone();
        for (i, (o, c, s)) in descs.iter().enumerate() {
            header[80 + i * 12..84 + i * 12].copy_from_slice(&o.to_le_bytes());
            header[84 + i * 12..88 + i * 12].copy_from_slice(&c.to_le_bytes());
            header[88 + i * 12..92 + i * 12].copy_from_slice(&s.to_le_bytes());
        }
        header[44..48].copy_from_slice(&(tables.len() as u32).to_le_bytes());
        header[48..80].copy_from_slice(&Sha256::digest(&tables));
        header[12..44].fill(0);
        let sum = Sha256::digest(&header);
        header[12..44].copy_from_slice(&sum);

        let mut blob = header;
        blob.extend_from_slice(&tables);
        if blob.len() > meta.max_size as usize {
            bail!("super metadata no longer fits in its slot");
        }
        blob.resize(meta.max_size as usize, 0);
        // Primary slots, then backup slots, all identical.
        for slot in 0..(meta.slots * 2) as u64 {
            self.write(METADATA_OFFSET + slot * meta.max_size as u64, &blob)?;
        }
        Ok(())
    }

    /// Partition name → linear extents.
    pub fn partitions(&mut self) -> Result<Vec<(String, Vec<Extent>)>> {
        let meta = self.metadata()?;
        let mut out = Vec::new();
        for p in &meta.partitions {
            let first = p.first_extent as usize;
            let count = p.extent_count as usize;
            let rows = meta
                .extents
                .get(first..first + count)
                .with_context(|| format!("{} points past the extent table", name_str(&p.name)))?;
            let mut exts = Vec::new();
            for e in rows {
                if e.target_type != 0 {
                    bail!("{} has non-linear extents", name_str(&p.name));
                }
                let ext = Extent {
                    offset: e.target_data * SECTOR,
                    len: e.sectors * SECTOR,
                };
                if ext.offset + ext.len > self.len {
                    bail!(
                        "{} extends past the end of the super image",
                        name_str(&p.name)
                    );
                }
                exts.push(ext);
            }
            out.push((name_str(&p.name), exts));
        }
        Ok(out)
    }

    pub fn extents(&mut self, name: &str) -> Result<Vec<Extent>> {
        let parts = self.partitions()?;
        let names: Vec<String> = parts.iter().map(|(n, _)| n.clone()).collect();
        parts
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e)
            .with_context(|| format!("no partition {name:?}; have {}", names.join(", ")))
    }

    pub fn capacity(&mut self, name: &str) -> Result<u64> {
        Ok(self.extents(name)?.iter().map(|e| e.len).sum())
    }

    /// Reads a whole logical partition into memory (small partitions only).
    pub fn read_partition(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for e in self.extents(name)? {
            out.extend_from_slice(&self.read(e.offset, usize::try_from(e.len)?)?);
        }
        Ok(out)
    }

    /// Copies a logical partition to `dst`.
    pub fn extract(
        &mut self,
        name: &str,
        dst: &mut dyn Write,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<()> {
        let exts = self.extents(name)?;
        let total: u64 = exts.iter().map(|e| e.len).sum();
        let mut done = 0;
        for e in exts {
            self.file.seek(SeekFrom::Start(self.base + e.offset))?;
            util::copy_exact(self.file, dst, e.len, &mut |n| progress(done + n, total))?;
            done += e.len;
        }
        Ok(())
    }

    /// Writes `size` bytes from `src` over the partition's extents, then
    /// zero-fills the rest of the partition (as lptool.py's `replace`).
    pub fn replace(
        &mut self,
        name: &str,
        src: &mut dyn Read,
        size: u64,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<()> {
        let exts = self.extents(name)?;
        let capacity: u64 = exts.iter().map(|e| e.len).sum();
        if size > capacity {
            bail!("image is {size} bytes; {name} holds {capacity}");
        }
        let mut left_in_src = size;
        let mut done = 0;
        let mut buf = vec![0u8; CHUNK];
        for e in exts {
            self.file.seek(SeekFrom::Start(self.base + e.offset))?;
            let mut left = e.len;
            while left > 0 {
                let n = (left as usize).min(buf.len());
                let from_src = (left_in_src as usize).min(n);
                if from_src > 0 {
                    src.read_exact(&mut buf[..from_src])
                        .context("image ended early")?;
                    left_in_src -= from_src as u64;
                }
                buf[from_src..n].fill(0);
                self.file.write_all(&buf[..n])?;
                left -= n as u64;
                done += n as u64;
                progress(done, capacity);
            }
        }
        Ok(())
    }

    /// Gives `name` one more extent in the free space after the last extent,
    /// as large as the device and group allow (or `limit`), and rewrites all
    /// metadata slots. Returns the bytes added.
    pub fn grow(&mut self, name: &str, limit: Option<u64>) -> Result<u64> {
        let mut meta = self.metadata()?;
        let index = meta
            .partitions
            .iter()
            .position(|p| name_str(&p.name) == name)
            .with_context(|| format!("no partition {name:?}"))?;
        let (first, count, group) = {
            let p = &meta.partitions[index];
            (p.first_extent, p.extent_count, p.group)
        };
        if meta.devices.len() != 1 {
            bail!("only single-device super images are supported");
        }
        let d = meta.devices[0].clone();
        let alignment = d.alignment as u64;
        let align_offset = d.alignment_offset as u64;
        if alignment == 0 {
            bail!("super device alignment is zero");
        }
        let end = meta
            .extents
            .iter()
            .map(|e| (e.target_data + e.sectors) * SECTOR)
            .chain([d.first_logical_sector * SECTOR])
            .max()
            .unwrap();
        let start = (end - align_offset).div_ceil(alignment) * alignment + align_offset;
        let mut size = d.size.saturating_sub(start);
        let max_group = meta
            .groups
            .get(group as usize)
            .context("bad group index")?
            .maximum_size;
        if max_group > 0 {
            let in_group: u64 = meta
                .partitions
                .iter()
                .filter(|p| p.group == group)
                .flat_map(|p| (p.first_extent..p.first_extent + p.extent_count).map(|i| i as usize))
                .map(|i| meta.extents[i].sectors * SECTOR)
                .sum();
            size = size.min(max_group.saturating_sub(in_group));
        }
        if let Some(l) = limit {
            size = size.min(l);
        }
        size -= size % alignment;
        if size == 0 {
            bail!("no free space to grow {name}");
        }
        let at = (first + count) as usize;
        meta.extents.insert(
            at,
            ExtentRow {
                sectors: size / SECTOR,
                target_type: 0,
                target_data: start / SECTOR,
                target_source: 0,
            },
        );
        meta.partitions[index].extent_count = count + 1;
        for (i, p) in meta.partitions.iter_mut().enumerate() {
            if i != index && p.first_extent >= first + count {
                p.first_extent += 1;
            }
        }
        self.write_metadata(&meta)?;
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a tiny valid super image with one group and two partitions.
    pub fn tiny_super() -> Vec<u8> {
        let len = 8 << 20;
        let mut img = vec![0u8; len];
        let max_size: u32 = 65536;
        let slots: u32 = 2;
        // geometry (only the fields we read)
        img[4096..4100].copy_from_slice(&GEOMETRY_MAGIC.to_le_bytes());
        img[4096 + 40..4096 + 44].copy_from_slice(&max_size.to_le_bytes());
        img[4096 + 44..4096 + 48].copy_from_slice(&slots.to_le_bytes());
        img[4096 + 48..4096 + 52].copy_from_slice(&4096u32.to_le_bytes());
        let name = |s: &str| {
            let mut n = [0u8; 36];
            n[..s.len()].copy_from_slice(s.as_bytes());
            n
        };
        let meta = Metadata {
            max_size,
            slots,
            block_size: 4096,
            header: {
                let mut h = vec![0u8; 128];
                h[0..4].copy_from_slice(&HEADER_MAGIC.to_le_bytes());
                h[4..6].copy_from_slice(&10u16.to_le_bytes());
                h[8..12].copy_from_slice(&128u32.to_le_bytes());
                h
            },
            partitions: vec![
                PartitionRow {
                    name: name("system_a"),
                    attributes: 1,
                    first_extent: 0,
                    extent_count: 1,
                    group: 1,
                },
                PartitionRow {
                    name: name("odm_a"),
                    attributes: 1,
                    first_extent: 1,
                    extent_count: 1,
                    group: 1,
                },
            ],
            extents: vec![
                ExtentRow {
                    sectors: 2048,
                    target_type: 0,
                    target_data: 2048,
                    target_source: 0,
                }, // 1 MiB @ 1 MiB
                ExtentRow {
                    sectors: 2048,
                    target_type: 0,
                    target_data: 4096,
                    target_source: 0,
                }, // 1 MiB @ 2 MiB
            ],
            groups: vec![
                GroupRow {
                    name: name("default"),
                    flags: 0,
                    maximum_size: 0,
                },
                GroupRow {
                    name: name("main"),
                    flags: 0,
                    maximum_size: 5 << 20,
                },
            ],
            devices: vec![DeviceRow {
                first_logical_sector: 2048,
                alignment: 1 << 20,
                alignment_offset: 0,
                size: len as u64,
                name: name("super"),
                flags: 0,
            }],
        };
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(&img).unwrap();
        Super::new(&mut f, 0, len as u64)
            .write_metadata(&meta)
            .unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        let mut out = Vec::new();
        f.read_to_end(&mut out).unwrap();
        out
    }

    #[test]
    fn grow_and_replace_inside_a_larger_file() {
        let img = tiny_super();
        let mut f = tempfile::tempfile().unwrap();
        let base = 4096u64; // pretend the super lives inside a container
        f.write_all(&vec![0xAB; base as usize]).unwrap();
        f.write_all(&img).unwrap();
        let mut s = Super::new(&mut f, base, img.len() as u64);
        assert_eq!(s.capacity("system_a").unwrap(), 1 << 20);
        let added = s.grow("system_a", None).unwrap();
        // group max 5 MiB, 2 MiB used → 3 MiB; device free after 3 MiB is 5 MiB.
        assert_eq!(added, 3 << 20);
        assert_eq!(s.capacity("system_a").unwrap(), 4 << 20);
        assert_eq!(
            s.extents("odm_a").unwrap(),
            vec![Extent {
                offset: 2 << 20,
                len: 1 << 20
            }]
        );

        let data: Vec<u8> = (0..1_500_000u32).map(|i| (i % 253) as u8).collect();
        s.replace(
            "system_a",
            &mut &data[..],
            data.len() as u64,
            &mut |_, _| {},
        )
        .unwrap();
        let back = s.read_partition("system_a").unwrap();
        assert_eq!(&back[..data.len()], &data[..]);
        assert!(back[data.len()..].iter().all(|&b| b == 0));
        // The prefix before the super region is untouched.
        let mut head = vec![0u8; base as usize];
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_exact(&mut head).unwrap();
        assert!(head.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn rejects_oversized_image() {
        let img = tiny_super();
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(&img).unwrap();
        let mut s = Super::new(&mut f, 0, img.len() as u64);
        let data = vec![1u8; (1 << 20) + 1];
        assert!(s
            .replace("odm_a", &mut &data[..], data.len() as u64, &mut |_, _| {})
            .is_err());
    }

    #[test]
    fn rejects_corrupt_metadata() {
        let mut img = tiny_super();
        img[METADATA_OFFSET as usize + 200] ^= 0xFF; // inside the tables
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(&img).unwrap();
        assert!(Super::new(&mut f, 0, img.len() as u64).metadata().is_err());
    }
}
