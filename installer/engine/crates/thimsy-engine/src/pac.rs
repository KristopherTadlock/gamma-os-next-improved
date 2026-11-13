//! Unisoc `BP_R2` .pac firmware packages: read entries and the embedded XML
//! config, edit entries in place, set the product strings and fix the CRCs.
//! A port of `scripts/pactool.py` and `tools/crc16arc.c`.

use crate::util::{self, CHUNK};
use anyhow::{bail, Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const HEADER_SIZE: u64 = 2124;
pub const ENTRY_SIZE: usize = 2580;
const CRC1_OFFSET: usize = HEADER_SIZE as usize - 4;
const CRC2_OFFSET: u64 = HEADER_SIZE - 2;
/// (offset, length in UTF-16 units) of the version and alias strings.
const VERSION_FIELD: (usize, usize) = (564, 256);
const ALIAS_FIELD: (usize, usize) = (1104, 100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub index: usize,
    pub id: String,
    pub file: String,
    pub size: u64,
    pub offset: u64,
}

#[derive(Debug)]
pub struct Pac {
    pub path: PathBuf,
    pub header: Vec<u8>,
    pub entries: Vec<Entry>,
    pub len: u64,
}

impl Pac {
    pub fn open(path: &Path) -> Result<Pac> {
        let mut f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let len = f.metadata()?.len();
        if len < HEADER_SIZE {
            bail!("{} is too small to be a PAC", path.display());
        }
        let header = util::read_at(&mut f, 0, HEADER_SIZE as usize)?;
        let magic = util::utf16_field(&header[..48]);
        if magic != "BP_R2.0.1" {
            bail!("unsupported PAC version {magic:?} in {}", path.display());
        }
        let count = util::u32_at(&header, 1076) as usize;
        let start = util::u32_at(&header, 1080) as u64;
        if count > 4096 {
            bail!("PAC claims {count} entries");
        }
        let table_len = (count * ENTRY_SIZE) as u64;
        if util::add(start, table_len)? > len {
            bail!("PAC entry table is truncated");
        }
        let table = util::read_at(&mut f, start, count * ENTRY_SIZE)?;
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let raw = &table[i * ENTRY_SIZE..(i + 1) * ENTRY_SIZE];
            let hi_size = util::u32_at(raw, 1532) as u64;
            let hi_off = util::u32_at(raw, 1536) as u64;
            let lo_size = util::u32_at(raw, 1540) as u64;
            let lo_off = util::u32_at(raw, 1552) as u64;
            let e = Entry {
                index: i,
                id: util::utf16_field(&raw[4..516]),
                file: util::utf16_field(&raw[516..1028]),
                size: (hi_size << 32) | lo_size,
                offset: (hi_off << 32) | lo_off,
            };
            if e.size > 0 && util::add(e.offset, e.size)? > len {
                bail!("PAC is truncated: {} ends past the end of the file", e.file);
            }
            entries.push(e);
        }
        Ok(Pac {
            path: path.to_path_buf(),
            header,
            entries,
            len,
        })
    }

    pub fn version(&self) -> String {
        field(&self.header, VERSION_FIELD)
    }
    pub fn alias(&self) -> String {
        field(&self.header, ALIAS_FIELD)
    }

    /// First entry with data whose file name or ID is `name`.
    pub fn find(&self, name: &str) -> Result<&Entry> {
        self.entries
            .iter()
            .find(|e| e.size > 0 && (e.file == name || e.id == name))
            .with_context(|| format!("no entry {name:?} in {}", self.path.display()))
    }

    pub fn read_entry(&self, e: &Entry) -> Result<Vec<u8>> {
        let mut f = File::open(&self.path)?;
        util::read_at(&mut f, e.offset, usize::try_from(e.size)?)
    }

    pub fn extract(&self, e: &Entry, out: &Path, progress: &mut dyn FnMut(u64)) -> Result<()> {
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(e.offset))?;
        let mut o = File::create(out).with_context(|| format!("creating {}", out.display()))?;
        util::copy_exact(&mut f, &mut o, e.size, progress)?;
        o.sync_all()?;
        Ok(())
    }

    /// The flash-tool XML config packed as the last `.xml` entry.
    pub fn config(&self) -> Result<Config> {
        let e = self
            .entries
            .iter()
            .rev()
            .find(|e| e.size > 0 && e.file.to_ascii_lowercase().ends_with(".xml"))
            .context("PAC has no XML config entry")?;
        let raw = self.read_entry(e)?;
        Config::parse(&String::from_utf8_lossy(&raw))
    }
}

fn field(header: &[u8], (offset, len): (usize, usize)) -> String {
    util::utf16_field(&header[offset..offset + len * 2])
}

/// Overwrites an entry's data in place; `data` must be exactly its size.
pub fn replace(path: &Path, e: &Entry, data: &[u8]) -> Result<()> {
    if data.len() as u64 != e.size {
        bail!("{} must be {} bytes, got {}", e.file, e.size, data.len());
    }
    let mut f = OpenOptions::new().write(true).open(path)?;
    util::write_at(&mut f, e.offset, data)?;
    Ok(())
}

/// Sets the product version and alias flash tools show, and the header CRC.
pub fn set_info(path: &Path, version: &str, alias: &str) -> Result<()> {
    let pac = Pac::open(path)?;
    let mut header = pac.header.clone();
    for ((offset, len), text) in [(VERSION_FIELD, version), (ALIAS_FIELD, alias)] {
        let raw: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        if raw.len() >= len * 2 {
            bail!("{text:?} is longer than {} characters", len - 1);
        }
        header[offset..offset + len * 2].fill(0);
        header[offset..offset + raw.len()].copy_from_slice(&raw);
    }
    let crc = crc16_arc(0, &header[..CRC1_OFFSET]);
    header[CRC1_OFFSET..CRC1_OFFSET + 2].copy_from_slice(&crc.to_le_bytes());
    let mut f = OpenOptions::new().write(true).open(path)?;
    util::write_at(&mut f, 0, &header[..CRC2_OFFSET as usize])?;
    Ok(())
}

/// Recomputes the data CRC (CRC-16/ARC of everything after the header).
pub fn fix_crc(path: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<u16> {
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let len = f.metadata()?.len();
    f.seek(SeekFrom::Start(HEADER_SIZE))?;
    let total = len - HEADER_SIZE;
    let mut buf = vec![0u8; CHUNK];
    let mut crc = 0u16;
    let mut done = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        crc = crc16_arc(crc, &buf[..n]);
        done += n as u64;
        progress(done, total);
    }
    util::write_at(&mut f, CRC2_OFFSET, &crc.to_le_bytes())?;
    f.flush()?;
    Ok(crc)
}

/// Reads the stored (header, data) CRCs.
pub fn stored_crcs(path: &Path) -> Result<(u16, u16)> {
    let mut f = File::open(path)?;
    let h = util::read_at(&mut f, 0, HEADER_SIZE as usize)?;
    Ok((
        util::u16_at(&h, CRC1_OFFSET),
        util::u16_at(&h, CRC2_OFFSET as usize),
    ))
}

#[allow(clippy::needless_range_loop)]
static CRC_TABLE: std::sync::LazyLock<[[u16; 256]; 8]> = std::sync::LazyLock::new(|| {
    let mut t = [[0u16; 256]; 8];
    for i in 0..256 {
        let mut c = i as u16;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xA001
            } else {
                c >> 1
            };
        }
        t[0][i] = c;
    }
    for i in 0..256 {
        for k in 1..8 {
            let prev = t[k - 1][i];
            t[k][i] = (prev >> 8) ^ t[0][(prev & 0xff) as usize];
        }
    }
    t
});

/// CRC-16/ARC (reflected, poly 0x8005, init 0), continuing from `crc`.
/// Slicing-by-8 for speed on multi-GB packages.
pub fn crc16_arc(mut crc: u16, data: &[u8]) -> u16 {
    let t = &*CRC_TABLE;
    let (chunks, rest) = data.as_chunks::<8>();
    for c in chunks {
        let lo = crc ^ u16::from_le_bytes([c[0], c[1]]);
        crc = t[7][(lo & 0xff) as usize]
            ^ t[6][(lo >> 8) as usize]
            ^ t[5][c[2] as usize]
            ^ t[4][c[3] as usize]
            ^ t[3][c[4] as usize]
            ^ t[2][c[5] as usize]
            ^ t[1][c[6] as usize]
            ^ t[0][c[7] as usize];
    }
    for &b in rest {
        crc = (crc >> 8) ^ t[0][((crc ^ b as u16) & 0xff) as usize];
    }
    crc
}

/// One `<File>` of the flash-tool scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgFile {
    pub id: String,
    pub kind: String,
    /// Device partition name (`<Block id=...>`); FDLs have none.
    pub block: Option<String>,
    pub base: u64,
    pub flag: u32,
    pub check_flag: u32,
    /// `backup="1"`: device-unique data the vendor tool preserves.
    pub backup: bool,
}

/// One `<Partition id=… size=…>` of a partition table: the PAC config's
/// (what the firmware expects) or the one `spd_dump partition_list` reads
/// from the device. Size in MiB; `REST_OF_DISK` for the last partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub mib: u64,
}

/// Size of a partition that takes the rest of the disk (`0xFFFFFFFF`).
pub const REST_OF_DISK: u64 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub product: String,
    pub files: Vec<CfgFile>,
    /// The `<Partitions>` table the firmware expects on the device.
    pub partitions: Vec<Partition>,
}

impl Config {
    pub fn parse(xml: &str) -> Result<Config> {
        let doc = roxmltree::Document::parse(xml).context("parsing PAC XML config")?;
        let mut cfg = Config::default();
        if let Some(p) = doc.descendants().find(|n| n.has_tag_name("Product")) {
            cfg.product = p.attribute("name").unwrap_or_default().to_string();
        }
        let text = |n: roxmltree::Node, tag: &str| -> String {
            n.children()
                .find(|c| c.has_tag_name(tag))
                .and_then(|c| c.text())
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        for f in doc.descendants().filter(|n| n.has_tag_name("File")) {
            let block = f.children().find(|c| c.has_tag_name("Block"));
            let base_s = block.map(|b| text(b, "Base")).unwrap_or_default();
            let base = parse_num(&base_s).unwrap_or(0);
            cfg.files.push(CfgFile {
                id: text(f, "ID"),
                kind: text(f, "Type"),
                block: block.and_then(|b| b.attribute("id")).map(str::to_string),
                base,
                flag: text(f, "Flag").parse().unwrap_or(0),
                check_flag: text(f, "CheckFlag").parse().unwrap_or(0),
                backup: f.attribute("backup") == Some("1"),
            });
        }
        if cfg.files.is_empty() {
            bail!("PAC XML config lists no files");
        }
        if let Some(t) = doc.descendants().find(|n| n.has_tag_name("Partitions")) {
            cfg.partitions = read_partitions(t)?;
        }
        Ok(cfg)
    }

    pub fn file(&self, id: &str) -> Option<&CfgFile> {
        self.files.iter().find(|f| f.id == id)
    }

    /// The table's partition called `name`, whatever the case.
    pub fn partition(&self, name: &str) -> Option<&Partition> {
        self.partitions
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

fn read_partitions(table: roxmltree::Node) -> Result<Vec<Partition>> {
    if let Some(e) = table.children().find(|c| c.has_tag_name("error")) {
        bail!(
            "the partition table is unusable: {}",
            e.text().unwrap_or_default().trim()
        );
    }
    let mut out = Vec::new();
    for p in table.children().filter(|c| c.has_tag_name("Partition")) {
        let name = p.attribute("id").unwrap_or_default();
        let size = p.attribute("size").unwrap_or_default();
        let mib = parse_num(size)
            .with_context(|| format!("partition {name:?} has a bad size {size:?}"))?;
        if name.is_empty() {
            bail!("a partition has no name");
        }
        out.push(Partition {
            name: name.to_string(),
            mib,
        });
    }
    Ok(out)
}

/// The device's partition table as `spd_dump partition_list FILE` writes
/// it: `<Partitions>` of `<Partition id="…" size="MiB"/>`, the last one
/// `0xffffffff`, and an `<error>` element when the sizes aren't whole MiB.
pub fn parse_partition_list(xml: &str) -> Result<Vec<Partition>> {
    let doc = roxmltree::Document::parse(xml).context("parsing the device's partition list")?;
    let t = doc
        .descendants()
        .find(|n| n.has_tag_name("Partitions"))
        .context("the device's partition list has no <Partitions>")?;
    read_partitions(t)
}

pub fn parse_num(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// Pairs PAC entries with their config files. Entries and `<File>`s appear in
/// the same order (IDs can repeat, e.g. two `UBOOTLoader`s), so pair by
/// position and check the IDs agree. The only entries without a `<File>` are
/// the ones with no ID after the last (the XML config itself).
pub fn pair<'a>(pac: &'a Pac, cfg: &'a Config) -> Result<Vec<(&'a Entry, &'a CfgFile)>> {
    if cfg.files.len() > pac.entries.len() {
        bail!(
            "the PAC config lists {} files but the PAC has only {} entries",
            cfg.files.len(),
            pac.entries.len()
        );
    }
    if let Some(e) = pac.entries[cfg.files.len()..]
        .iter()
        .find(|e| !e.id.is_empty())
    {
        bail!(
            "PAC entry {} ({}) has no file in the PAC config ({} entries, {} files)",
            e.index,
            e.id,
            pac.entries.len(),
            cfg.files.len()
        );
    }
    let mut out = Vec::new();
    for (e, f) in pac.entries.iter().zip(cfg.files.iter()) {
        if e.id != f.id {
            bail!(
                "PAC entry {} ({}) doesn't match config file {}",
                e.index,
                e.id,
                f.id
            );
        }
        out.push((e, f));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_bitwise() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 31 % 251) as u8).collect();
        let mut slow = 0u16;
        for &b in &data {
            slow ^= b as u16;
            for _ in 0..8 {
                slow = if slow & 1 != 0 {
                    (slow >> 1) ^ 0xA001
                } else {
                    slow >> 1
                };
            }
        }
        assert_eq!(crc16_arc(0, &data), slow);
        // Streaming in odd-sized pieces gives the same answer.
        let mut c = 0;
        for piece in data.chunks(777) {
            c = crc16_arc(c, piece);
        }
        assert_eq!(c, slow);
        // CRC-16/ARC check value.
        assert_eq!(crc16_arc(0, b"123456789"), 0xBB3D);
    }

    #[test]
    fn parses_config() {
        let xml = r#"<BMAConfig><ProductList><Product name="ums9620_2h10"/></ProductList>
        <SchemeList><Scheme name="x">
          <File><ID>FDL</ID><Type>FDL</Type><Block><Base>0x65000800</Base></Block><Flag>1</Flag><CheckFlag>1</CheckFlag></File>
          <File backup="1"><ID>ProdNV</ID><Type>CODE2</Type><Block id="prodnv"><Base>0x0</Base></Block><Flag>1</Flag><CheckFlag>2</CheckFlag></File>
        </Scheme></SchemeList></BMAConfig>"#;
        let c = Config::parse(xml).unwrap();
        assert_eq!(c.product, "ums9620_2h10");
        assert_eq!(c.file("FDL").unwrap().base, 0x65000800);
        assert!(c.file("ProdNV").unwrap().backup);
        assert_eq!(c.file("ProdNV").unwrap().block.as_deref(), Some("prodnv"));
        assert!(c.partitions.is_empty());
    }

    #[test]
    fn parses_partition_tables() {
        // The PAC config's table (MiB, last one the rest of the disk).
        let xml = r#"<BMAConfig><ProductList><Product name="p"><Partitions>
          <!-- size unit is MBytes -->
          <Partition id="prodnv" size="64"/><Partition id="userdata" size="0xFFFFFFFF"/>
        </Partitions></Product></ProductList><SchemeList><Scheme name="x">
          <File><ID>FDL</ID><Type>FDL</Type><Block><Base>0x5500</Base></Block></File>
        </Scheme></SchemeList></BMAConfig>"#;
        let c = Config::parse(xml).unwrap();
        assert_eq!(c.partition("prodnv").unwrap().mib, 64);
        assert_eq!(c.partition("userdata").unwrap().mib, REST_OF_DISK);
        // spd_dump's partition_list file, exactly as partition_list() prints it.
        let dev = "<Partitions>\n    <Partition id=\"prodnv\" size=\"64\"/>\n    <Partition id=\"userdata\" size=\"0xffffffff\"/>\n</Partitions>\n";
        assert_eq!(parse_partition_list(dev).unwrap(), c.partitions);
        // Wrong unit size: spd_dump says not to use the table.
        let bad = "<Partitions>\n    <Partition id=\"prodnv\" size=\"64\"/>\n<error>Wrong unit size, do not use this table!</error>\n</Partitions>\n";
        let err = parse_partition_list(bad).unwrap_err().to_string();
        assert!(err.contains("Wrong unit size"), "{err}");
        assert!(parse_partition_list("not xml").is_err());
    }
}
