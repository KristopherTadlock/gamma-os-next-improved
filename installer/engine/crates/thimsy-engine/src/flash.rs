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
    pub backups: Vec<Write>,
    pub writes: Vec<Write>,
    pub erases: Vec<String>,
}

pub fn spd_plan(d: &Device, p: &Pac, mode: Mode) -> Result<SpdPlan> {
    let f = d.flash.as_ref().context("device has no flash recipe")?;
    let cfg = p.config()?;
    let pairs = pac::pair(p, &cfg)?;
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
            backups.push(Write {
                entry: e.id.clone(),
                index: e.index,
                partition: block.clone(),
                bytes: e.size,
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
    // Safety: nothing marked backup, nothing named like NV/calibration.
    for w in &writes {
        let n = w.partition.to_ascii_lowercase();
        if ["prodnv", "miscdata", "nvram", "nvdata", "persist", "calinv"].contains(&n.as_str())
            && !f.update.contains(&w.entry)
        {
            bail!("refusing to write protected partition {}", w.partition);
        }
    }
    let erases = if mode == Mode::Full {
        f.erase_full.clone()
    } else {
        Vec::new()
    };
    Ok(SpdPlan {
        fdl,
        backups,
        writes,
        erases,
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
    }
}

#[derive(Debug, Serialize)]
pub struct FlashResult {
    pub backup: Option<PathBuf>,
    pub commands: Vec<String>,
}

pub fn flash(cat: &Catalog, opts: &FlashOptions, rep: &Reporter) -> Result<FlashResult> {
    rep.stage("check", "Checking the build");
    let rec: JobRecord = job::load_job(&opts.job)?;
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

    rep.stage("driver", "Checking the USB driver");
    crate::drivers::ensure(d, opts.host, rep)?;

    rep.stage("prepare", "Preparing the partitions");
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
    fs::create_dir_all(&flash_dir)?;
    fs::create_dir_all(flash_dir.join("backup"))?;
    let total: u64 = files.iter().map(|(_, e)| e.size).sum();
    let mut done = 0;
    for (name, e) in &files {
        let out = flash_dir.join(name);
        if !fs::metadata(&out).is_ok_and(|m| m.len() == e.size) {
            p.extract(e, &out, &mut |n| {
                rep.progress("Preparing partitions", done + n, total)
            })?;
        }
        done += e.size;
    }

    rep.stage("wait_device", "Connect the device in download mode");
    let step = d.guides.download_mode.first();
    rep.prompt(
        "enter_download_mode",
        step.map_or("Connect the device in download mode", |s| s.title.as_str()),
        step.map_or("", |s| s.body.as_str()),
        None,
    );

    rep.stage("flash", "Installing Thimsy");
    let write_total: u64 = plan.writes.iter().map(|w| w.bytes).sum();
    let mut written = 0u64;
    let names: Vec<(String, u64)> = plan
        .writes
        .iter()
        .map(|w| (w.partition.clone(), w.bytes))
        .collect();
    tools::run(&inv, &flash_dir, rep, |line| {
        // "load_partition: super, target: 0x..., written: 0x..."
        if let Some(rest) = line.strip_prefix("load_partition: ") {
            let name = rest.split(',').next().unwrap_or("").trim();
            if let Some((_, b)) = names.iter().find(|(n, _)| n == name) {
                written += b;
                rep.progress(&format!("Wrote {name}"), written, write_total);
            }
        }
    })
    .map_err(|e| {
        if crate::error::code_of(&e).0 == Code::ToolFailed {
            e.context("flashing stopped; the device is still recoverable from download mode")
        } else {
            e
        }
    })?;

    // Keep the NV/calibration backups with the job, not in the scratch dir.
    fs::create_dir_all(&backup_dir)?;
    for b in &plan.backups {
        let from = flash_dir
            .join("backup")
            .join(format!("{}.img", b.partition));
        if from.exists() {
            fs::rename(&from, backup_dir.join(format!("{}.img", b.partition)))?;
        }
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
            rep.prompt(
                "confirm_unlock",
                "Confirm on the device",
                "Use the volume keys to choose Unlock, then press Power.",
                None,
            );
            let inv = Invocation {
                tool: "fastboot".into(),
                args: vec!["flashing".into(), "unlock".into()],
                stdin: vec![],
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
        };
        let inv = spd_invocation(d, &plan, "backup");
        let s = inv.display();
        assert!(
            s.starts_with(
                "spd_dump --wait 300 keep_charge 1 fdl fdl.img 0x65000800 fdl fdl2.img 0xb4fffe00"
            ),
            "{s}"
        );
        assert!(s.contains("read_part prodnv 0 0x4000000 backup/prodnv.img"));
        assert!(s.contains("write_part uboot_a uboot_a.img write_part uboot_b uboot_b.img"));
        assert!(s.contains("write_part super super.img erase_part userdata reset"));
        assert_eq!(inv.stdin, vec!["yes"; 4]);
    }
}
