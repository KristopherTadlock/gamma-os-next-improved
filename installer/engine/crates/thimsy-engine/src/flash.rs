//! Writing a finished build to the device with the bundled tool for its
//! family. Unisoc devices use `spd_dump` fed partition by partition from the
//! PAC, with the vendor's NV and calibration partitions backed up and never
//! written.

use crate::catalog::{Catalog, Device};
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use crate::job::{self, JobRecord};
use crate::pac::{self, Pac};
use crate::resolve;
use crate::tools::{self, Invocation};
use crate::util;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Fresh install: every flashable partition, data erased.
    Full,
    /// Only the partitions Thimsy changes; data kept.
    Update,
}

impl Mode {
    pub fn parse(s: &str) -> Result<Mode> {
        match s {
            "full" => Ok(Mode::Full),
            "update" => Ok(Mode::Update),
            _ => bail!("mode must be full or update"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlashOptions {
    pub job: PathBuf,
    pub mode: Mode,
    pub dry_run: bool,
    pub host: Host,
}

#[derive(Debug, Clone, Serialize)]
pub struct Write {
    pub entry: String,
    /// Position in the PAC (IDs can repeat, e.g. two `UBOOTLoader`s).
    pub index: usize,
    pub partition: String,
    pub bytes: u64,
}

/// What will happen, computed before anything touches the device.
#[derive(Debug, Clone, Serialize)]
pub struct SpdPlan {
    pub fdl: Vec<(String, u64)>,
    /// Partitions read off the device first, each at its full size.
    pub backups: Vec<Write>,
    pub writes: Vec<Write>,
    pub erases: Vec<String>,
    /// Device partitions (name, MiB) that must match the firmware's
    /// partition table before the first write: everything backed up,
    /// written or erased that the table lists.
    pub checks: Vec<(String, u64)>,
}

/// Device-unique data (NV, calibration, factory test results) that is never
/// written or erased, whatever a recipe or PAC says.
const PROTECTED: &[&str] = &["prodnv", "miscdata", "nvram", "nvdata", "persist", "calinv"];

/// What `spd_dump` prints when it wants an operation confirmed (`Answer
/// "yes" to confirm the "write partition" command: `, no newline).
pub const SPD_PROMPT: &str = "Answer \"yes\"";

/// Where `spd_dump partition_list` writes the device's table (in `flash/`).
pub const DEVICE_PARTITIONS: &str = "device-partitions.xml";

/// Records which image the files in `flash/` were unpacked from.
const STAMP: &str = ".image-sha256";

pub fn spd_plan(d: &Device, p: &Pac, mode: Mode) -> Result<SpdPlan> {
    let f = d.flash.as_ref().context("device has no flash recipe")?;
    let cfg = p.config()?;
    let pairs = pac::pair(p, &cfg)?;
    if cfg.partitions.is_empty() {
        bail!("the PAC config has no <Partitions> table, so the device's layout can't be checked");
    }
    // A misspelt name would silently protect nothing.
    for n in &f.never_write {
        if !p.entries.iter().any(|e| &e.id == n) {
            bail!("never_write names {n}, which isn't an entry in the PAC");
        }
    }
    let mut fdl = Vec::new();
    for id in &f.fdl {
        let c = cfg
            .file(id)
            .with_context(|| format!("PAC config has no {id}"))?;
        if c.base == 0 {
            bail!("{id} has no load address in the PAC config");
        }
        fdl.push((id.clone(), c.base));
    }
    let mut backups = Vec::new();
    let mut writes = Vec::new();
    for (e, c) in &pairs {
        let Some(block) = &c.block else { continue };
        if c.backup {
            // The whole partition, not just the size of the PAC's image.
            let part = cfg.partition(block).with_context(|| {
                format!("backup partition {block} isn't in the PAC's partition table")
            })?;
            if part.mib == pac::REST_OF_DISK {
                bail!("backup partition {block} has no fixed size");
            }
            backups.push(Write {
                entry: e.id.clone(),
                index: e.index,
                partition: block.clone(),
                bytes: part.mib << 20,
            });
            continue;
        }
        if e.size == 0
            || c.kind.starts_with("Erase")
            || f.fdl.contains(&e.id)
            || f.never_write.contains(&e.id)
        {
            continue;
        }
        let wanted = match mode {
            Mode::Full => true,
            Mode::Update => f.update.contains(&e.id),
        };
        if wanted {
            writes.push(Write {
                entry: e.id.clone(),
                index: e.index,
                partition: block.clone(),
                bytes: e.size,
            });
        }
    }
    if mode == Mode::Update {
        for u in &f.update {
            if !writes.iter().any(|w| &w.entry == u) {
                bail!("update entry {u} isn't a writable PAC entry");
            }
        }
    }
    let erases = if mode == Mode::Full {
        f.erase_full.clone()
    } else {
        Vec::new()
    };
    // Safety: nothing backed up or named like NV/calibration is written or
    // erased, whatever the recipe says.
    let backed_up: Vec<String> = backups
        .iter()
        .map(|b| b.partition.to_ascii_lowercase())
        .collect();
    let touched = writes
        .iter()
        .map(|w| ("write", &w.partition))
        .chain(erases.iter().map(|e| ("erase", e)));
    for (what, name) in touched {
        let n = name.to_ascii_lowercase();
        if PROTECTED.contains(&n.as_str()) || backed_up.contains(&n) {
            bail!("refusing to {what} protected partition {name}");
        }
    }
    let mut checks: Vec<(String, u64)> = Vec::new();
    let names = backups
        .iter()
        .chain(&writes)
        .map(|w| &w.partition)
        .chain(&erases);
    for name in names {
        // Boot-area partitions (splloader) aren't in the table.
        if let Some(t) = cfg.partition(name) {
            if !checks.iter().any(|(n, _)| n == name) {
                checks.push((name.clone(), t.mib));
            }
        }
    }
    Ok(SpdPlan {
        fdl,
        backups,
        writes,
        erases,
        checks,
    })
}

/// File name used while flashing: the device partition name (unique), or
/// the FDL's ID.
fn file_for(name: &str) -> String {
    format!("{}.img", name.to_ascii_lowercase())
}

pub fn spd_invocation(d: &Device, plan: &SpdPlan, backup_dir: &str) -> Invocation {
    let f = d.flash.as_ref().unwrap();
    let mut args = vec!["--wait".to_string(), f.wait_seconds.to_string()];
    args.extend(["keep_charge".into(), "1".into()]);
    for (id, base) in &plan.fdl {
        args.extend(["fdl".into(), file_for(id), format!("0x{base:x}")]);
    }
    // Read before the first question, checked before it's answered.
    args.extend(["partition_list".into(), DEVICE_PARTITIONS.into()]);
    for b in &plan.backups {
        args.extend([
            "read_part".into(),
            b.partition.clone(),
            "0".into(),
            format!("0x{:x}", b.bytes),
            format!("{backup_dir}/{}.img", b.partition),
        ]);
    }
    let mut confirms = 0;
    for w in &plan.writes {
        args.extend([
            "write_part".into(),
            w.partition.clone(),
            file_for(&w.partition),
        ]);
        confirms += 1;
    }
    for e in &plan.erases {
        args.extend(["erase_part".into(), e.clone()]);
        confirms += 1;
    }
    args.push("reset".into());
    Invocation {
        tool: "spd_dump".into(),
        args,
        stdin: vec!["yes".into(); confirms],
        answer_prompt: Some(SPD_PROMPT.into()),
    }
}

fn show_mib(mib: u64) -> String {
    if mib == pac::REST_OF_DISK {
        "the rest of the disk".into()
    } else {
        format!("{mib} MiB")
    }
}

/// Compares the device's partition table (from `spd_dump partition_list`)
/// with what the firmware expects; `wrong_device` on any difference.
pub fn check_device(checks: &[(String, u64)], device: &[pac::Partition]) -> Result<()> {
    // Size differences first: they say the most about what's connected.
    let mut diffs = Vec::new();
    let mut missing = Vec::new();
    for (name, mib) in checks {
        match device.iter().find(|p| &p.name == name) {
            None => missing.push(format!("{name} is missing")),
            Some(p) if p.mib != *mib => diffs.push(format!(
                "{name} is {} on the device, the firmware expects {}",
                show_mib(p.mib),
                show_mib(*mib)
            )),
            Some(_) => {}
        }
    }
    diffs.extend(missing);
    if diffs.is_empty() {
        return Ok(());
    }
    let more = diffs.len().saturating_sub(3);
    let mut msg = diffs.into_iter().take(3).collect::<Vec<_>>().join("; ");
    if more > 0 {
        msg.push_str(&format!("; and {more} more"));
    }
    Err(wrong_layout(format!(
        "the device's partition table doesn't match this firmware ({msg}); nothing was written"
    )))
}

fn wrong_layout(message: String) -> anyhow::Error {
    EngineError::new(Code::WrongDevice, message)
        .hint("This device's partition layout doesn't match this firmware: it's a different model, or it still runs stock firmware that needs repartitioning, which this version can't do yet.")
        .into()
}

/// Reads and checks the table `spd_dump` saved in `flash_dir`.
fn check_device_file(flash_dir: &Path, checks: &[(String, u64)]) -> Result<()> {
    let path = flash_dir.join(DEVICE_PARTITIONS);
    let text = fs::read_to_string(&path).map_err(|e| {
        EngineError::new(
            Code::ToolFailed,
            format!("spd_dump didn't save the device's partition table ({e}); nothing was written"),
        )
    })?;
    let table = pac::parse_partition_list(&text)
        .map_err(|e| wrong_layout(format!("{e:#}; nothing was written")))?;
    check_device(checks, &table)
}

/// The image must be exactly the one the build produced.
fn verify_image(rec: &JobRecord, rep: &Reporter) -> Result<()> {
    let len = fs::metadata(&rec.image)
        .with_context(|| format!("{} is missing", rec.image.display()))?
        .len();
    let sha = if len == rec.bytes {
        util::sha256_file(&rec.image, |d, t| rep.progress("Checking the image", d, t))?
    } else {
        String::new()
    };
    if len != rec.bytes || sha != rec.sha256 {
        return Err(EngineError::new(
            Code::HashMismatch,
            format!(
                "{} changed since it was built ({len} bytes, sha256 {sha}; expected {} bytes, sha256 {})",
                rec.image.display(),
                rec.bytes,
                rec.sha256
            ),
        )
        .hint("Delete the workspace's jobs folder and build again; report it if it persists.")
        .into());
    }
    Ok(())
}

/// Readies `flash/` for the image with this sha256: files unpacked from
/// another image (or by an engine that kept no record) are removed first.
fn prepare_flash_dir(flash_dir: &Path, sha256: &str) -> Result<()> {
    let stamp = flash_dir.join(STAMP);
    let same = fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == sha256);
    if !same && flash_dir.exists() {
        fs::remove_dir_all(flash_dir)
            .with_context(|| format!("removing old files in {}", flash_dir.display()))?;
    }
    fs::create_dir_all(flash_dir)?;
    if !same {
        util::write_atomic(&stamp, sha256.as_bytes())?;
    }
    Ok(())
}

/// A name in `dir` that doesn't exist yet: `stem.img`, else `stem-N.img`.
fn free_name(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.img"));
    if !first.exists() {
        return first;
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (0..)
        .map(|i| dir.join(format!("{stem}-{secs}-{i}.img")))
        .find(|p| !p.exists())
        .expect("a free name")
}

/// Moves the backups `spd_dump` read into `<job>/backup`, never replacing an
/// earlier one (that one may be from this device; this run's may be from
/// another). A file of the wrong size gets `.incomplete` added. Returns
/// what's missing or wrong.
fn keep_backups(plan: &SpdPlan, flash_dir: &Path, backup_dir: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for b in &plan.backups {
        let from = flash_dir
            .join("backup")
            .join(format!("{}.img", b.partition));
        let Ok(meta) = fs::metadata(&from) else {
            problems.push(format!("{} wasn't read", b.partition));
            continue;
        };
        fs::create_dir_all(backup_dir)?;
        let mut to = free_name(backup_dir, &b.partition);
        if meta.len() != b.bytes {
            problems.push(format!(
                "{} is {} bytes, expected {}",
                b.partition,
                meta.len(),
                b.bytes
            ));
            to = PathBuf::from(format!("{}.incomplete", to.display()));
        }
        fs::rename(&from, &to)?;
    }
    Ok(problems)
}

#[derive(Debug, Serialize)]
pub struct FlashResult {
    pub backup: Option<PathBuf>,
    pub commands: Vec<String>,
}

pub fn flash(cat: &Catalog, opts: &FlashOptions, rep: &Reporter) -> Result<FlashResult> {
    rep.stage("check", "Checking the build");
    let rec: JobRecord = job::load_job(&opts.job)?;
    if rec.catalog != cat.root.version {
        rep.warn(format!(
            "This build was made with catalog {}; the installer now has catalog {}. Build again if the device's recipe changed.",
            rec.catalog, cat.root.version
        ));
    }
    let d = cat.device(&rec.device)?;
    if let Err(e) = resolve::require_host(d, opts.host) {
        if !opts.dry_run {
            return Err(e);
        }
        rep.warn(format!("{e} (dry run continues)"));
    }
    let f = d.flash.as_ref().context("device has no flash recipe")?;
    if f.method != "spd" {
        return Err(EngineError::new(
            Code::UnsupportedHost,
            format!(
                "flashing method {} isn't available in this version",
                f.method
            ),
        )
        .into());
    }
    let p = Pac::open(&rec.image)?;
    let plan = spd_plan(d, &p, opts.mode)?;
    let flash_dir = opts.job.join("flash");
    let backup_dir = opts.job.join("backup");
    let inv = spd_invocation(d, &plan, "backup");
    rep.info(format!(
        "{} partitions to write ({}), {} to back up, {} to erase",
        plan.writes.len(),
        crate::util::human_bytes(plan.writes.iter().map(|w| w.bytes).sum()),
        plan.backups.len(),
        plan.erases.len()
    ));
    if opts.dry_run {
        rep.info(format!("Dry run: {}", inv.display()));
        return Ok(FlashResult {
            backup: None,
            commands: vec![inv.display()],
        });
    }
    verify_image(&rec, rep)?;

    rep.stage("driver", "Checking the USB driver");
    crate::drivers::ensure(d, opts.host, rep)?;

    rep.stage("prepare", "Preparing the partitions");
    // Backups an interrupted run read but never moved.
    let _ = keep_backups(&plan, &flash_dir, &backup_dir);
    prepare_flash_dir(&flash_dir, &rec.sha256)?;
    // Never mistake an earlier run's device table or backups for this one's.
    let _ = fs::remove_file(flash_dir.join(DEVICE_PARTITIONS));
    let _ = fs::remove_dir_all(flash_dir.join("backup"));
    // (file name, PAC entry) for everything spd_dump reads.
    let mut files: Vec<(String, &pac::Entry)> = Vec::new();
    for (id, _) in &plan.fdl {
        files.push((file_for(id), p.find(id)?));
    }
    for w in &plan.writes {
        files.push((file_for(&w.partition), &p.entries[w.index]));
    }
    let missing: u64 = files
        .iter()
        .filter(|(n, e)| !fs::metadata(flash_dir.join(n)).is_ok_and(|m| m.len() == e.size))
        .map(|(_, e)| e.size)
        .sum();
    let need = missing + missing / 20 + plan.backups.iter().map(|b| b.bytes).sum::<u64>();
    let free = fs4::available_space(&opts.job).unwrap_or(u64::MAX);
    if free < need {
        return Err(job::not_enough_space(&opts.job, need, free));
    }
    fs::create_dir_all(flash_dir.join("backup"))?;
    let total: u64 = files.iter().map(|(_, e)| e.size).sum();
    let mut done = 0;
    for (name, e) in &files {
        let out = flash_dir.join(name);
        if !fs::metadata(&out).is_ok_and(|m| m.len() == e.size) {
            // Complete-looking only once fully written.
            let part = flash_dir.join(format!("{name}.part"));
            p.extract(e, &part, &mut |n| {
                rep.progress("Preparing partitions", done + n, total)
            })?;
            fs::rename(&part, &out)?;
        }
        done += e.size;
    }

    rep.stage("wait_device", "Connect the device in download mode");
    // The device's own button steps, translated with its name filled in.
    let step = crate::resolve::trs(&d.guides.download_mode, &d.name)
        .into_iter()
        .next();
    rep.prompt(
        "enter_download_mode",
        step.as_ref()
            .map_or("Connect the device in download mode", |s| s.title.as_str()),
        step.as_ref().map_or("", |s| s.body.as_str()),
        None,
    );

    // The prompt stays up while spd_dump waits for the device; the install
    // stage starts when the device answers (its first BSL_REP_VER line).
    let mut connected = false;
    let write_total: u64 = plan.writes.iter().map(|w| w.bytes).sum();
    let mut written = 0u64;
    let names: Vec<(String, u64)> = plan
        .writes
        .iter()
        .map(|w| (w.partition.clone(), w.bytes))
        .collect();
    let on_line = |line: &str| {
        if !connected && (line.starts_with("BSL_REP_VER") || line.starts_with("load_partition")) {
            connected = true;
            rep.stage("flash", "Installing Thimsy");
        }
        // "load_partition: super, target: 0x..., written: 0x..."
        if let Some(rest) = line.strip_prefix("load_partition: ") {
            let name = rest.split(',').next().unwrap_or("").trim();
            if let Some((_, b)) = names.iter().find(|(n, _)| n == name) {
                written += b;
                rep.progress(&format!("Wrote {name}"), written, write_total);
            }
        }
    };
    // Nothing is confirmed until the device's partition table matches.
    let approve = |n: usize| {
        if n == 0 {
            check_device_file(&flash_dir, &plan.checks)?;
            rep.info("The device's partition table matches this firmware");
        }
        Ok(())
    };
    let ran = tools::run_answering(&inv, &flash_dir, rep, on_line, approve).map_err(|e| {
        if crate::error::code_of(&e).0 != Code::ToolFailed {
            return e;
        }
        let msg = e.to_string();
        if opts.host == Host::Windows && msg.contains("libusb_open_device failed") {
            return EngineError::new(Code::ToolFailed, msg)
                .hint("Download mode was never seen through WinUSB. Hold Back while you plug in the cable, and check in Device Manager or Zadig that Windows uses WinUSB for Unisoc download mode, not Unisoc's own driver.")
                .into();
        }
        e.context("flashing stopped; the device is still recoverable from download mode")
    });

    // Keep the NV/calibration backups with the job, not in the scratch
    // dir, whether or not the rest worked.
    let kept = keep_backups(&plan, &flash_dir, &backup_dir);
    if let Err(e) = ran {
        if let Err(k) = &kept {
            rep.warn(format!("couldn't keep the backups: {k:#}"));
        }
        return Err(e);
    }
    let problems = kept?;
    if !problems.is_empty() {
        return Err(EngineError::new(
            Code::ToolFailed,
            format!(
                "the device was installed, but its backup is incomplete: {}. The backed-up partitions were not written; keep {} and report this",
                problems.join("; "),
                backup_dir.display()
            ),
        )
        .into());
    }
    rep.stage("reboot", "Restarting the device");
    rep.info("The device restarts into Thimsy. The first boot takes a few minutes.");
    let _ = fs::remove_dir_all(&flash_dir);
    Ok(FlashResult {
        backup: Some(backup_dir),
        commands: vec![inv.display()],
    })
}

/// Automated bootloader unlock (fastboot devices only).
pub fn unlock(cat: &Catalog, device: &str, rep: &Reporter) -> Result<()> {
    let d = cat.device(device)?;
    match d.guides.unlock.route.as_str() {
        "none" => {
            rep.info(format!("{} doesn't need unlocking", d.name));
            Ok(())
        }
        "automated" => {
            // The device's own confirmation step (its last unlock step) when
            // the catalog has one: the keys differ between devices.
            let own = crate::resolve::trs(&d.guides.unlock.steps, &d.name).pop();
            rep.prompt(
                "confirm_unlock",
                own.as_ref()
                    .map_or("Confirm on the device", |s| s.title.as_str()),
                own.as_ref().map_or(
                    "Use the volume keys to choose Unlock, then press Power.",
                    |s| s.body.as_str(),
                ),
                None,
            );
            let inv = Invocation {
                tool: "fastboot".into(),
                args: vec!["flashing".into(), "unlock".into()],
                stdin: vec![],
                answer_prompt: None,
            };
            tools::run(&inv, Path::new("."), rep, |_| {})
        }
        _ => Err(EngineError::new(
            Code::UnlockGuided,
            format!("{} is unlocked with upstream's own steps", d.name),
        )
        .hint("Follow the unlock steps shown in the installer, then continue.")
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_confirms_every_write() {
        let cat = Catalog::bundled().unwrap();
        let d = cat.device("anbernic-rg-cube").unwrap();
        let plan = SpdPlan {
            fdl: vec![("FDL".into(), 0x65000800), ("FDL2".into(), 0xb4fffe00)],
            backups: vec![Write {
                entry: "ProdNV".into(),
                index: 2,
                partition: "prodnv".into(),
                bytes: 0x4000000,
            }],
            writes: vec![
                Write {
                    entry: "UBOOTLoader".into(),
                    index: 10,
                    partition: "uboot_a".into(),
                    bytes: 1,
                },
                Write {
                    entry: "UBOOTLoader".into(),
                    index: 11,
                    partition: "uboot_b".into(),
                    bytes: 1,
                },
                Write {
                    entry: "Super".into(),
                    index: 47,
                    partition: "super".into(),
                    bytes: 1,
                },
            ],
            erases: vec!["userdata".into()],
            checks: vec![],
        };
        let inv = spd_invocation(d, &plan, "backup");
        let s = inv.display();
        assert!(
            s.starts_with(
                "spd_dump --wait 300 keep_charge 1 fdl fdl.img 0x65000800 fdl fdl2.img 0xb4fffe00 partition_list device-partitions.xml read_part"
            ),
            "{s}"
        );
        assert!(s.contains("read_part prodnv 0 0x4000000 backup/prodnv.img"));
        assert!(s.contains("write_part uboot_a uboot_a.img write_part uboot_b uboot_b.img"));
        assert!(s.contains("write_part super super.img erase_part userdata reset"));
        assert_eq!(inv.stdin, vec!["yes"; 4]);
        // Answered one at a time, only when spd_dump asks.
        assert_eq!(inv.answer_prompt.as_deref(), Some("Answer \"yes\""));
    }

    #[test]
    fn device_table_must_match_what_the_firmware_expects() {
        let part = |n: &str, mib: u64| pac::Partition {
            name: n.into(),
            mib,
        };
        let checks = vec![
            ("prodnv".to_string(), 64),
            ("super".to_string(), 5600),
            ("userdata".to_string(), pac::REST_OF_DISK),
        ];
        let good = vec![
            part("prodnv", 64),
            part("splloader", 1),
            part("super", 5600),
            part("userdata", pac::REST_OF_DISK),
        ];
        check_device(&checks, &good).unwrap();
        // RG556-style layout: same USB ID, different table.
        let other = vec![
            part("prodnv", 64),
            part("super", 4096),
            part("cache", 64),
            part("userdata_x", pac::REST_OF_DISK),
        ];
        let err = check_device(&checks, &other).unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::WrongDevice);
        let msg = err.to_string();
        assert!(
            msg.contains("super is 4096 MiB on the device, the firmware expects 5600 MiB"),
            "{msg}"
        );
        assert!(msg.contains("userdata is missing"), "{msg}");
        assert!(crate::error::code_of(&err)
            .1
            .unwrap()
            .contains("partition layout"));
        // Only the first differences are listed.
        let many: Vec<(String, u64)> = (0..6).map(|i| (format!("p{i}"), 1)).collect();
        let msg = check_device(&many, &[]).unwrap_err().to_string();
        assert!(msg.contains("and 3 more"), "{msg}");
    }
}
