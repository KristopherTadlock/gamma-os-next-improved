//! Writing a finished build to the device with the bundled tool for its
//! family. Unisoc devices use `spd_dump` fed partition by partition from the
//! PAC, with the vendor's NV and calibration partitions backed up and never
//! written.

use crate::catalog::{Catalog, Device};
use crate::driver_watch::{DriverWatch, FollowEvent, SwitchFollow};
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::host::Host;
use crate::job::{self, JobRecord};
use crate::pac::{self, Pac};
use crate::resolve;
use crate::tools::{self, Invocation};
use crate::util;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// written, erased or read for `base` that the table lists.
    pub checks: Vec<(String, u64)>,
    /// Update mode: images the update relies on but doesn't write (recipe
    /// `update_check`), read back from the device (the PAC entry's size)
    /// and compared with the PAC before the first write.
    pub base: Vec<Write>,
}

/// Device-unique data (NV, calibration, factory test results) that is never
/// written or erased, whatever a recipe or PAC says.
const PROTECTED: &[&str] = &["prodnv", "miscdata", "nvram", "nvdata", "persist", "calinv"];

/// What `spd_dump` prints when it wants an operation confirmed (`Answer
/// "yes" to confirm the "write partition" command: `, no newline).
pub const SPD_PROMPT: &str = "Answer \"yes\"";

/// Where `spd_dump partition_list` writes the device's table (in the run's
/// backup folder).
pub const DEVICE_PARTITIONS: &str = "device-partitions.xml";

/// Written next to each run's backups: which device and partition table
/// they came from, and when.
pub const DEVICE_INFO: &str = "device.json";

/// File (in the run's backup folder) an update-mode base check reads.
fn base_file(partition: &str) -> String {
    format!("check-{}.img", partition.to_ascii_lowercase())
}

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
    // Every partition touched is checked against the device's table before
    // the first write, except the boot-area ones the recipe names as outside
    // it. Anything else missing from the table is refused.
    for n in &f.unchecked_partitions {
        if cfg.partition(n).is_some() {
            bail!("unchecked_partitions names {n}, which is in the PAC's partition table (and so is checked)");
        }
        let written_by_pac = pairs.iter().any(|(_, c)| {
            c.block
                .as_deref()
                .is_some_and(|b| b.eq_ignore_ascii_case(n))
        });
        if !written_by_pac {
            bail!("unchecked_partitions names {n}, which isn't a partition of any PAC entry");
        }
    }
    let mut checks: Vec<(String, u64)> = Vec::new();
    let names = backups
        .iter()
        .chain(&writes)
        .map(|w| &w.partition)
        .chain(&erases);
    for name in names {
        match cfg.partition(name) {
            Some(t) => {
                if !checks.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
                    checks.push((name.clone(), t.mib));
                }
            }
            None if f
                .unchecked_partitions
                .iter()
                .any(|u| u.eq_ignore_ascii_case(name)) => {}
            None => bail!(
                "{name} isn't in the PAC's partition table, so it can't be checked on the device before it's written or erased"
            ),
        }
    }
    if checks.is_empty() {
        bail!("nothing this flash touches is in the PAC's partition table, so the device can't be checked");
    }
    // What the plan showed the user is what gets written.
    let shown = match mode {
        Mode::Full => &f.writes_full,
        Mode::Update => &f.writes_update,
    };
    let lower = |v: &mut Vec<String>| {
        v.iter_mut().for_each(|n| *n = n.to_ascii_lowercase());
        v.sort();
    };
    let mut want: Vec<String> = shown.clone();
    let mut got: Vec<String> = writes.iter().map(|w| w.partition.clone()).collect();
    lower(&mut want);
    lower(&mut got);
    if want != got {
        let extra: Vec<&String> = got.iter().filter(|g| !want.contains(g)).collect();
        let absent: Vec<&String> = want.iter().filter(|w| !got.contains(w)).collect();
        bail!(
            "the recipe's list of partitions to write ({}) doesn't match this firmware: it would also write {:?} and not write {:?}",
            if mode == Mode::Full { "writes_full" } else { "writes_update" },
            extra,
            absent
        );
    }
    // Update mode relies on what it doesn't write: read it back first.
    let mut base = Vec::new();
    if mode == Mode::Update {
        for id in &f.update_check {
            let mut found = false;
            for (e, c) in pairs.iter().filter(|(e, _)| &e.id == id && e.size > 0) {
                let block = c
                    .block
                    .as_ref()
                    .with_context(|| format!("update_check entry {id} has no partition"))?;
                let t = cfg.partition(block).with_context(|| {
                    format!("update_check partition {block} isn't in the PAC's partition table")
                })?;
                if writes
                    .iter()
                    .any(|w| w.partition.eq_ignore_ascii_case(block))
                {
                    bail!("update_check names {id}, which the update writes");
                }
                if !checks.iter().any(|(n, _)| n.eq_ignore_ascii_case(block)) {
                    checks.push((block.clone(), t.mib));
                }
                base.push(Write {
                    entry: e.id.clone(),
                    index: e.index,
                    partition: block.clone(),
                    bytes: e.size,
                });
                found = true;
            }
            if !found {
                bail!("update_check names {id}, which isn't a PAC entry with data");
            }
        }
        if base.is_empty() {
            bail!("update mode has nothing to check the device's base with (update_check)");
        }
    }
    Ok(SpdPlan {
        fdl,
        backups,
        writes,
        erases,
        checks,
        base,
    })
}

/// File name used while flashing: the device partition name (unique), or
/// the FDL's ID.
fn file_for(name: &str) -> String {
    format!("{}.img", name.to_ascii_lowercase())
}

/// The `spd_dump` command line. The device's partition table, the backups
/// and the update-mode base images are read into `run_dir` (the run's
/// backup folder, relative to the flash folder it runs in).
pub fn spd_invocation(d: &Device, plan: &SpdPlan, run_dir: &str) -> Invocation {
    let f = d.flash.as_ref().unwrap();
    let mut args = vec!["--wait".to_string(), f.wait_seconds.to_string()];
    args.extend(["keep_charge".into(), "1".into()]);
    for (id, base) in &plan.fdl {
        args.extend(["fdl".into(), file_for(id), format!("0x{base:x}")]);
    }
    // Read before the first question, checked before it's answered.
    args.extend([
        "partition_list".into(),
        format!("{run_dir}/{DEVICE_PARTITIONS}"),
    ]);
    for b in &plan.backups {
        args.extend([
            "read_part".into(),
            b.partition.clone(),
            "0".into(),
            format!("0x{:x}", b.bytes),
            format!("{run_dir}/{}.img", b.partition),
        ]);
    }
    for b in &plan.base {
        args.extend([
            "read_part".into(),
            b.partition.clone(),
            "0".into(),
            format!("0x{:x}", b.bytes),
            format!("{run_dir}/{}", base_file(&b.partition)),
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
    if checks.is_empty() {
        bail!("there's nothing to compare the device's partition table with; nothing was written");
    }
    // Size differences first: they say the most about what's connected.
    let mut diffs = Vec::new();
    let mut missing = Vec::new();
    for (name, mib) in checks {
        match device.iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
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

/// Reads the table `spd_dump` saved in the run folder: (its text, parsed).
fn read_device_table(run_dir: &Path) -> Result<(String, Vec<pac::Partition>)> {
    let path = run_dir.join(DEVICE_PARTITIONS);
    let text = fs::read_to_string(&path).map_err(|e| {
        EngineError::new(
            Code::ToolFailed,
            format!("spd_dump didn't save the device's partition table ({e}); nothing was written"),
        )
    })?;
    let table = pac::parse_partition_list(&text)
        .map_err(|e| wrong_layout(format!("{e:#}; nothing was written")))?;
    Ok((text, table))
}

/// Records next to the run's backups which device and partition table
/// they came from, and when (so runs can be told apart).
fn write_device_info(run_dir: &Path, d: &Device, mode: Mode, table: &str) -> Result<()> {
    let info = serde_json::json!({
        "device": d.id,
        "device_name": d.name,
        "partition_table_sha256": util::sha256_hex(table.as_bytes()),
        "time": util::utc_iso(util::now_secs()),
        "mode": if mode == Mode::Full { "full" } else { "update" },
        "engine": crate::VERSION,
    });
    util::write_atomic(
        &run_dir.join(DEVICE_INFO),
        &serde_json::to_vec_pretty(&info)?,
    )
}

/// Update mode: every image the update relies on must be on the device
/// already, exactly as in the PAC; otherwise `full_install_needed`.
fn check_base(plan: &SpdPlan, run_dir: &Path, entries: &[job::EntryHash]) -> Result<()> {
    let mut differ = Vec::new();
    for b in &plan.base {
        let p = run_dir.join(base_file(&b.partition));
        let len = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        if len != b.bytes {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!(
                    "{} couldn't be read back from the device ({len} of {} bytes); nothing was written",
                    b.partition, b.bytes
                ),
            )
            .hint("Check the cable and that the device is still in its flashing mode, then try again.")
            .into());
        }
        let want = entry_hash(entries, b.index)?;
        if util::sha256_file(&p, |_, _| {})? != want {
            differ.push(b.partition.clone());
        }
    }
    if differ.is_empty() {
        return Ok(());
    }
    Err(EngineError::new(
        Code::FullInstallNeeded,
        format!(
            "the device's {} {} not match this firmware, so it doesn't run the base this update needs; nothing was written",
            differ.join(", "),
            if differ.len() == 1 { "does" } else { "do" }
        ),
    )
    .hint("This device doesn't run a matching Thimsy or GammaOS build, so it can't be updated. Use a full install instead (it erases the device).")
    .into())
}

fn entry_hash(entries: &[job::EntryHash], index: usize) -> Result<&str> {
    entries
        .iter()
        .find(|e| e.index == index)
        .map(|e| e.sha256.as_str())
        .with_context(|| format!("no checksum for PAC entry {index}"))
}

/// What the Windows `spd_dump` said while it waited for the device: it
/// opens it through WinUSB only, and while it waits says when the device
/// has another driver (Unisoc's own serial driver can't send the request
/// smartphone chipsets need) or that driver's port is in use, and why
/// libusb couldn't open it otherwise.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SpdConnect {
    /// The other driver the device has (`sprdvcom Ports COM5`).
    pub other_driver: Option<String>,
    /// That driver's serial port is open in another program (`COM5`), so
    /// Windows can't change its driver.
    pub port_busy: Option<String>,
    /// The device was there, but libusb couldn't open it: what spd_dump
    /// said.
    pub unusable: Option<String>,
}

impl SpdConnect {
    /// Takes in one line; returns the other driver it reports, if any.
    pub fn line(&mut self, line: &str) -> Option<String> {
        let l = line.trim();
        if l.starts_with("connected through ") {
            // Opened after all: whatever failed before doesn't matter.
            *self = SpdConnect::default();
        } else if l.starts_with("Windows has: ") {
            // What Windows has for the device changed: what spd_dump said
            // about the driver and its port before no longer holds (it
            // says it again, right after, if it still does).
            self.other_driver = None;
            self.port_busy = None;
        } else if let Some(found) = l
            .strip_prefix("download mode uses another driver (")
            .and_then(|r| r.rsplit_once("), not WinUSB"))
        {
            self.other_driver = Some(found.0.to_string());
            return self.other_driver.clone();
        } else if let Some(port) = l
            .strip_prefix("serial port ")
            .and_then(|r| r.strip_suffix(" is in use by another program"))
        {
            self.port_busy = Some(port.to_string());
        } else if l.starts_with("download mode is connected, but libusb can't open it") {
            self.unusable = Some(l.to_string());
        }
        None
    }
}

/// Windows, while waiting for the device: what to do after a line, given
/// what spd_dump said so far (`seen`), the other driver this line reported
/// (`found`), whether the helper that switches drivers runs (`helper`) and
/// whether a switch is under way (`switching`: the port may look busy then
/// because Windows is changing its driver).
#[derive(Debug)]
pub enum WaitAction {
    /// Keep waiting.
    Wait,
    /// Switch the device to WinUSB.
    Switch,
    /// Stop: the device can't be used as it is.
    Stop(anyhow::Error),
}

pub fn windows_wait_action(
    seen: &SpdConnect,
    found: Option<&str>,
    helper: bool,
    switching: bool,
) -> WaitAction {
    // Another program holds the port: Windows can't switch its driver.
    if seen.port_busy.is_some() && !switching {
        return WaitAction::Stop(windows_problem(seen).expect("port in use"));
    }
    // spd_dump reports no driver (or a problem) only once it has lasted,
    // so whatever it reports won't change by itself. Without the helper
    // (Windows had no other driver for the device when the install
    // started), there's nothing to switch it with.
    match found {
        Some(_) if helper => WaitAction::Switch,
        // Windows records the driver it gave the device, so the next try
        // starts the helper before the device is connected.
        Some(found) => WaitAction::Stop(
            EngineError::new(
                Code::DeviceNotFound,
                format!("the device is connected in download mode, but Windows gave it another driver ({found}) instead of WinUSB; nothing was written. Try again: the installer then gets ready to switch it before you connect it"),
            )
            .hint("Repeat the download-mode steps, then try again. If it still isn't found, check in Device Manager that Unisoc download mode uses the WinUSB driver.")
            .into(),
        ),
        None => WaitAction::Wait,
    }
}

/// Windows: why the device, though connected in download mode, can't be
/// used (as far as spd_dump said), or None.
fn windows_problem(seen: &SpdConnect) -> Option<anyhow::Error> {
    // Another program holds the device: its COM port, or WinUSB itself.
    let held = seen.port_busy.clone().or_else(|| {
        seen.unusable
            .as_ref()
            .filter(|u| u.contains("LIBUSB_ERROR_ACCESS") || u.contains("LIBUSB_ERROR_BUSY"))
            .map(|u| u.to_string())
    });
    if let Some(what) = held {
        return Some(
            EngineError::new(
                Code::DeviceNotFound,
                format!("the device is connected in download mode, but another program is using it ({what}); nothing was written"),
            )
            .hint("Close every other program that talks to the device (such as Unisoc's UnlockBootloader or ResearchDownload), then try again.")
            .into(),
        );
    }
    let why = match (&seen.other_driver, &seen.unusable) {
        (Some(found), _) => format!("Windows gives it another driver ({found}) instead of WinUSB"),
        (None, Some(why)) => format!("Windows gave it a driver the installer can't use ({why})"),
        _ => return None,
    };
    Some(
        EngineError::new(
            Code::DeviceNotFound,
            format!("the device is connected in download mode, but {why}; nothing was written"),
        )
        .hint("Repeat the download-mode steps, then try again. If it still isn't found, check in Device Manager that Unisoc download mode uses the WinUSB driver.")
        .into(),
    )
}

/// spd_dump exited without the device ever answering.
fn not_connected(d: &Device, host: Host, seen: &SpdConnect, e: anyhow::Error) -> anyhow::Error {
    let detail = format!("{e:#}");
    if host == Host::Windows {
        if let Some(e) = windows_problem(seen) {
            return e;
        }
    }
    let present = crate::detect::scan_ids()
        .unwrap_or_default()
        .iter()
        .any(|(vid, pid)| {
            d.identify
                .usb
                .iter()
                .any(|u| u.vid == *vid && u.pid == *pid)
        });
    if host == Host::Linux && present {
        return EngineError::new(
            Code::DeviceNotFound,
            format!("the device is connected in download mode, but this computer didn't let the installer open it ({detail}); nothing was written"),
        )
        .hint("Linux doesn't give the installer access to the device's USB connection yet. Set up USB access (it asks for your password once), then try again.")
        .into();
    }
    let err = EngineError::new(
        Code::DeviceNotFound,
        format!("the device never answered in download mode ({detail}); nothing was written"),
    );
    if host == Host::Windows {
        err.hint("Repeat the download-mode steps, then try again. If it still isn't found, unplug the cable, plug it into another USB port and try again.")
            .into()
    } else {
        err.hint("Repeat the download-mode steps and keep holding the button until installing starts, then try again.")
            .into()
    }
}

/// The image must be exactly the one the build produced. Returns each data
/// entry's sha256, read in the same pass.
fn verify_image(rec: &JobRecord, rep: &Reporter) -> Result<Vec<job::EntryHash>> {
    let len = fs::metadata(&rec.image)
        .with_context(|| format!("{} is missing", rec.image.display()))?
        .len();
    let (sha, entries) = if len == rec.bytes {
        job::hash_image(&rec.image, &mut |d, t| {
            rep.progress("Checking the image", d, t)
        })?
    } else {
        (String::new(), Vec::new())
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
    if !rec.entries.is_empty() && rec.entries != entries {
        return Err(EngineError::new(
            Code::HashMismatch,
            format!(
                "the partition checksums in {}'s job.json don't match the image",
                rec.image.display()
            ),
        )
        .hint("Delete the workspace's jobs folder and build again; report it if it persists.")
        .into());
    }
    Ok(entries)
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

/// Name of a new folder for one run's backups: `run-<UTC time>`.
fn backup_run_name() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("run-{}", util::utc_stamp(secs))
}

/// Creates a new, empty folder in `<job>/backup` for one run's backups
/// (`run-<UTC time>`, `-1`, `-2`... if taken): `spd_dump` reads straight
/// into it, outside the scratch folder, so nothing ever has to move,
/// replace or delete a backup.
fn new_backup_run(backup_root: &Path, suffix: &str) -> Result<PathBuf> {
    fs::create_dir_all(backup_root)
        .with_context(|| format!("creating {}", backup_root.display()))?;
    let base = format!("{}{suffix}", backup_run_name());
    for i in 0..1000 {
        let name = if i == 0 {
            base.clone()
        } else {
            format!("{base}-{i}")
        };
        let dir = backup_root.join(name);
        match fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(e).with_context(|| format!("creating {}", dir.display()));
            }
        }
    }
    bail!("no free backup folder name in {}", backup_root.display())
}

/// Backups an earlier engine version read into `<job>/flash/backup` and
/// didn't get to move are moved into a new `<job>/backup/run-…-recovered`
/// before anything clears the scratch folder. Fails (and the caller must
/// then leave `flash/` alone) if one can't be moved.
pub(crate) fn rescue_old_backups(job_dir: &Path) -> Result<()> {
    let old = job_dir.join("flash").join("backup");
    let Ok(rd) = fs::read_dir(&old) else {
        return Ok(());
    };
    let files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    if files.is_empty() {
        let _ = fs::remove_dir(&old);
        return Ok(());
    }
    let to = new_backup_run(&job_dir.join("backup"), "-recovered")?;
    for f in files {
        let name = f.file_name().context("backup without a name")?;
        fs::rename(&f, to.join(name)).with_context(|| {
            format!(
                "moving the unsaved backup {} to {}",
                f.display(),
                to.display()
            )
        })?;
    }
    let _ = fs::remove_dir(&old);
    Ok(())
}

/// The backups of this run: each must have been read at the partition's
/// full size before anything is confirmed.
fn check_backups(plan: &SpdPlan, run_dir: &Path) -> Result<()> {
    for b in &plan.backups {
        let p = run_dir.join(format!("{}.img", b.partition));
        let len = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        if len != b.bytes {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!(
                    "the backup of {} is incomplete ({len} of {} bytes); nothing was written",
                    b.partition, b.bytes
                ),
            )
            .hint("Check the cable and that the device is still in its flashing mode, then try again.")
            .into());
        }
    }
    Ok(())
}

/// After the run: what's missing or wrong in its backups. A file of the
/// wrong size gets `.incomplete` added (never over an existing file).
fn finish_backups(plan: &SpdPlan, run_dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for b in &plan.backups {
        let p = run_dir.join(format!("{}.img", b.partition));
        let Ok(meta) = fs::metadata(&p) else {
            problems.push(format!("{} wasn't read", b.partition));
            continue;
        };
        if meta.len() != b.bytes {
            problems.push(format!(
                "{} is {} bytes, expected {}",
                b.partition,
                meta.len(),
                b.bytes
            ));
            let to = run_dir.join(format!("{}.img.incomplete", b.partition));
            if !to.exists() {
                let _ = fs::rename(&p, &to);
            }
        }
    }
    problems
}

/// Lines `spd_dump` prints only once it's writing or erasing.
const WRITE_LINES: &[&str] = &["load_partition", "erase_partition", "write_partition"];

/// Follows `spd_dump`'s output while it flashes: progress, and the signs
/// that a write went wrong or ran without being confirmed.
pub struct SpdWatch {
    /// Planned (partition, bytes), and whether it was written completely.
    writes: Vec<(String, u64, bool)>,
    written: u64,
    total: u64,
}

impl SpdWatch {
    pub fn new(plan: &SpdPlan) -> SpdWatch {
        SpdWatch {
            writes: plan
                .writes
                .iter()
                .map(|w| (w.partition.clone(), w.bytes, false))
                .collect(),
            written: 0,
            total: plan.writes.iter().map(|w| w.bytes).sum(),
        }
    }

    /// Reads one output line. `checked`: whether the device's partition
    /// table has been checked (the first answer approved). Returns the
    /// partition just written with the bytes written so far and in total;
    /// fails when the run must stop at once.
    pub fn line(&mut self, line: &str, checked: bool) -> Result<Option<(String, u64, u64)>> {
        let l = line.trim();
        if l.contains("unexpected response") {
            return Err(write_failed(format!(
                "spd_dump got an unexpected response from the device ({l})"
            )));
        }
        if !checked && WRITE_LINES.iter().any(|w| l.starts_with(w)) {
            return Err(EngineError::new(
                Code::ToolFailed,
                format!("spd_dump wrote to the device without asking first, before its partition table was checked ({l})"),
            )
            .hint("Reinstall Thimsy Installer: its flashing tool isn't the version it expects.")
            .into());
        }
        let Some(rest) = l.strip_prefix("load_partition:") else {
            return Ok(None);
        };
        // "load_partition: super, target: 0x..., written: 0x..."
        let fields: Vec<&str> = rest.split(',').map(str::trim).collect();
        let num = |key: &str| {
            fields
                .iter()
                .find_map(|f| f.strip_prefix(key))
                .and_then(|v| pac::parse_num(v.trim()))
        };
        let name = fields.first().copied().unwrap_or_default();
        let (Some(target), Some(written)) = (num("target:"), num("written:")) else {
            return Err(write_failed(format!(
                "spd_dump's report can't be read ({l})"
            )));
        };
        let Some(w) = self
            .writes
            .iter_mut()
            .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        else {
            return Err(write_failed(format!(
                "spd_dump wrote {name}, which isn't in the plan"
            )));
        };
        if written != target {
            return Err(write_failed(format!(
                "writing {name} stopped after {written} of {target} bytes"
            )));
        }
        if target != w.1 {
            return Err(write_failed(format!(
                "spd_dump wrote {target} bytes to {name}; its image is {} bytes",
                w.1
            )));
        }
        if !w.2 {
            w.2 = true;
            self.written += target;
        }
        Ok(Some((name.to_string(), self.written, self.total)))
    }

    /// Once `spd_dump` has finished: every planned write must have been
    /// reported complete.
    pub fn finish(&self) -> Result<()> {
        let missing: Vec<&str> = self
            .writes
            .iter()
            .filter(|w| !w.2)
            .map(|w| w.0.as_str())
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(write_failed(format!(
            "spd_dump finished without reporting a complete write of {}",
            missing.join(", ")
        )))
    }
}

fn write_failed(message: String) -> anyhow::Error {
    EngineError::new(Code::ToolFailed, message)
        .hint("Check the cable and that the device is still in its flashing mode, then try again.")
        .into()
}

/// A real flash installs only the image this catalog expects for the
/// profile, not just whatever job.json describes.
fn check_reference(cat: &Catalog, d: &Device, rec: &JobRecord) -> Result<()> {
    let want = d.expect.get(&rec.profile);
    if want.is_some_and(|w| w.eq_ignore_ascii_case(&rec.sha256)) {
        return Ok(());
    }
    Err(EngineError::new(
        Code::HashMismatch,
        format!(
            "this build (sha256 {}) isn't the {} image catalog {} expects (sha256 {})",
            rec.sha256,
            rec.profile,
            cat.root.version,
            want.map_or("none", String::as_str)
        ),
    )
    .hint("This version of the installer expects a different build. Build again, then install.")
    .into())
}

#[derive(Debug, Serialize)]
pub struct FlashResult {
    pub backup: Option<PathBuf>,
    pub commands: Vec<String>,
}

/// Kept in `flash/`: what was last checked there (by `prepare` or an
/// install), the image and each unpacked file with its size and
/// modification time. A later run trusts a check only while the file is
/// unchanged; anything else is read and checked again.
pub const PREPARED: &str = "prepared.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct FileStamp {
    size: u64,
    modified_ns: u64,
}

impl FileStamp {
    fn of(path: &Path) -> Option<FileStamp> {
        let m = fs::metadata(path).ok()?;
        let t = m
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some(FileStamp {
            size: m.len(),
            modified_ns: u64::try_from(t.as_nanos()).ok()?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Prepared {
    image_sha256: String,
    image: Option<FileStamp>,
    entries: Vec<job::EntryHash>,
    /// Unpacked file → (its stamp when checked, its sha256).
    files: BTreeMap<String, (FileStamp, String)>,
}

/// The record in `flash_dir`, if it is for the image with this sha256.
fn load_prepared(flash_dir: &Path, sha256: &str) -> Prepared {
    fs::read(flash_dir.join(PREPARED))
        .ok()
        .and_then(|b| serde_json::from_slice::<Prepared>(&b).ok())
        .filter(|p| p.image_sha256 == sha256)
        .unwrap_or_default()
}

/// [verify_image], skipped when the image is unchanged since it was last
/// checked in full.
fn check_image(rec: &JobRecord, prev: &Prepared, rep: &Reporter) -> Result<Vec<job::EntryHash>> {
    let stamp = FileStamp::of(&rec.image);
    let unchanged = stamp.is_some()
        && prev.image == stamp
        && stamp.is_some_and(|s| s.size == rec.bytes)
        && prev.image_sha256 == rec.sha256
        && !prev.entries.is_empty()
        && (rec.entries.is_empty() || rec.entries == prev.entries);
    if unchanged {
        rep.info("The image is unchanged since it was checked");
        return Ok(prev.entries.clone());
    }
    verify_image(rec, rep)
}

/// Unpacks every file `spd_dump` will read into `flash_dir` and checks each
/// against its PAC entry (one that doesn't match is unpacked again, once).
/// `reads` is what the run will also read from the device, for the space
/// check. Records what was checked in [PREPARED].
#[allow(clippy::too_many_arguments)]
fn ready_files(
    rec: &JobRecord,
    p: &Pac,
    plan: &SpdPlan,
    flash_dir: &Path,
    job_dir: &Path,
    entries: &[job::EntryHash],
    reads: u64,
    rep: &Reporter,
) -> Result<()> {
    prepare_flash_dir(flash_dir, &rec.sha256)?;
    let prev = load_prepared(flash_dir, &rec.sha256);
    // An earlier engine read the device's table into the scratch folder.
    let _ = fs::remove_file(flash_dir.join(DEVICE_PARTITIONS));
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
    let need = missing + missing / 20 + reads;
    let free = crate::disk::probe(job_dir)
        .map_err(|e| {
            EngineError::new(
                Code::BadInput,
                format!("the free space in {} couldn't be checked ({e})", job_dir.display()),
            )
            .hint("The free space on this drive couldn't be checked. Choose another folder for the workspace.")
        })?
        .free;
    if free < need {
        return Err(job::not_enough_space(job_dir, need, free));
    }
    let total: u64 = files.iter().map(|(_, e)| e.size).sum();
    let mut done = 0;
    let unpack = |name: &str, e: &pac::Entry, done: u64| -> Result<()> {
        // Complete-looking only once fully written.
        let part = flash_dir.join(format!("{name}.part"));
        p.extract(e, &part, &mut |n| {
            rep.progress("Preparing partitions", done + n, total)
        })?;
        fs::rename(&part, flash_dir.join(name))?;
        Ok(())
    };
    for (name, e) in &files {
        if !fs::metadata(flash_dir.join(name)).is_ok_and(|m| m.len() == e.size) {
            unpack(name, e, done)?;
        }
        done += e.size;
    }
    // Every file spd_dump will write must be exactly its PAC entry, however
    // it got there: checked now, or unchanged since it was checked.
    let mut record = Prepared {
        image_sha256: rec.sha256.clone(),
        image: FileStamp::of(&rec.image),
        entries: entries.to_vec(),
        // Earlier checks of files this run doesn't use (the other mode's)
        // still hold while those files are unchanged.
        files: prev
            .files
            .iter()
            .filter(|(n, (s, _))| FileStamp::of(&flash_dir.join(n)) == Some(*s))
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect(),
    };
    let mut checked_bytes = 0;
    for (name, e) in &files {
        let want = entry_hash(entries, e.index)?;
        let path = flash_dir.join(name);
        let trusted = record
            .files
            .get(name)
            .is_some_and(|(s, sha)| sha == want && FileStamp::of(&path) == Some(*s));
        if !trusted {
            let hash = |path: &Path, at: u64| {
                util::sha256_file(path, |d, _| {
                    rep.progress("Checking the partitions", at + d, total)
                })
            };
            if hash(&path, checked_bytes)? != want {
                rep.warn(format!(
                    "{name} doesn't match the image; unpacking it again"
                ));
                unpack(name, e, 0)?;
                if hash(&path, checked_bytes)? != want {
                    return Err(EngineError::new(
                        Code::HashMismatch,
                        format!("{name} still doesn't match the image after unpacking it again; nothing was written"),
                    )
                    .hint("The drive the workspace is on may be failing. Choose a workspace on another drive and build again.")
                    .into());
                }
            }
            match FileStamp::of(&path) {
                Some(s) => {
                    record.files.insert(name.clone(), (s, want.to_string()));
                }
                None => {
                    record.files.remove(name);
                }
            }
        }
        checked_bytes += e.size;
        rep.progress("Checking the partitions", checked_bytes, total);
    }
    if let Err(e) = serde_json::to_vec(&record)
        .map_err(anyhow::Error::from)
        .and_then(|b| util::write_atomic(&flash_dir.join(PREPARED), &b))
    {
        // Only means the next run checks everything again.
        rep.warn(format!("couldn't record the checked files: {e:#}"));
    }
    Ok(())
}

/// The job, its device and the plan for `mode`: shared by [prepare] and
/// [flash].
fn load_for(
    cat: &Catalog,
    job_dir: &Path,
    mode: Mode,
    rep: &Reporter,
) -> Result<(JobRecord, Pac, SpdPlan)> {
    let rec: JobRecord = job::load_job(job_dir)?;
    if rec.catalog != cat.root.version {
        rep.warn(format!(
            "This build was made with catalog {}; the installer now has catalog {}. Build again if the device's recipe changed.",
            rec.catalog, cat.root.version
        ));
    }
    let d = cat.device(&rec.device)?;
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
    let plan = spd_plan(d, &p, mode)?;
    Ok((rec, p, plan))
}

#[derive(Debug, Serialize)]
pub struct PrepareResult {
    /// Files ready in the job's `flash` folder.
    pub files: usize,
    pub bytes: u64,
}

/// Gets a finished build ready to install in `mode`, without a device:
/// checks the image, then unpacks and checks every partition the install
/// will write. An install soon after skips that work if nothing changed.
pub fn prepare(cat: &Catalog, job_dir: &Path, mode: Mode, rep: &Reporter) -> Result<PrepareResult> {
    rep.stage("check", "Checking the build");
    let (rec, p, plan) = load_for(cat, job_dir, mode, rep)?;
    let d = cat.device(&rec.device)?;
    let _lock = job::lock_workspace(&job::workspace_of(job_dir))?;
    check_reference(cat, d, &rec)?;
    let flash_dir = job_dir.join("flash");
    let entries = check_image(&rec, &load_prepared(&flash_dir, &rec.sha256), rep)?;
    rep.stage("prepare", "Preparing the partitions");
    ready_files(&rec, &p, &plan, &flash_dir, job_dir, &entries, 0, rep)?;
    Ok(PrepareResult {
        files: plan.fdl.len() + plan.writes.len(),
        bytes: plan.writes.iter().map(|w| w.bytes).sum(),
    })
}

pub fn flash(cat: &Catalog, opts: &FlashOptions, rep: &Reporter) -> Result<FlashResult> {
    rep.stage("check", "Checking the build");
    let (rec, p, plan) = load_for(cat, &opts.job, opts.mode, rep)?;
    let d = cat.device(&rec.device)?;
    if let Err(e) = resolve::require_host(d, opts.host) {
        if !opts.dry_run {
            return Err(e);
        }
        rep.warn(format!("{e} (dry run continues)"));
    }
    let flash_dir = opts.job.join("flash");
    let backup_root = opts.job.join("backup");
    let run_name = backup_run_name();
    let inv = spd_invocation(d, &plan, &format!("../backup/{run_name}"));
    rep.info(format!(
        "{} partitions to write ({}), {} to back up, {} to erase, {} to compare",
        plan.writes.len(),
        crate::util::human_bytes(plan.writes.iter().map(|w| w.bytes).sum()),
        plan.backups.len(),
        plan.erases.len(),
        plan.base.len()
    ));
    if opts.dry_run {
        rep.info(format!("Dry run: {}", inv.display()));
        return Ok(FlashResult {
            backup: None,
            commands: vec![inv.display()],
        });
    }
    // One run at a time per workspace, for the whole flash.
    let _lock = job::lock_workspace(&job::workspace_of(&opts.job))?;
    check_reference(cat, d, &rec)?;
    let entries = check_image(&rec, &load_prepared(&flash_dir, &rec.sha256), rep)?;

    rep.stage("driver", "Checking the USB driver");
    crate::drivers::ensure_for_flash(d, opts.host, rep)?;

    rep.stage("prepare", "Preparing the partitions");
    rescue_old_backups(&opts.job)?;
    // Folders of runs stopped before they read anything (only empty ones
    // can be removed this way).
    for e in fs::read_dir(&backup_root).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with("run-") {
            let _ = fs::remove_dir(e.path());
        }
    }
    let reads: u64 = plan.backups.iter().chain(&plan.base).map(|b| b.bytes).sum();
    ready_files(&rec, &p, &plan, &flash_dir, &opts.job, &entries, reads, rep)?;
    // Windows with another driver for the device (Unisoc's own, which it
    // would get): the helper that switches it to WinUSB as it's connected,
    // started (one permission prompt) before the user is asked to connect.
    let switcher = if opts.host == Host::Windows && crate::driver_watch::needed(d) {
        rep.stage("driver", "Checking the USB driver");
        rep.prompt(
            "allow_driver_switch",
            "Allow the USB driver switch",
            "Windows asks for permission next. Click Yes so the installer can give your device the USB driver it needs as soon as you connect it. Don't connect it yet.",
            None,
        );
        Some(DriverWatch::start(d, rep)?)
    } else {
        None
    };
    let switcher = std::cell::RefCell::new(switcher);
    let run_dir = new_backup_run(&backup_root, "")?;
    let run_arg = format!(
        "../backup/{}",
        run_dir.file_name().unwrap().to_string_lossy()
    );
    let inv = spd_invocation(d, &plan, &run_arg);

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
    let connected = std::cell::Cell::new(false);
    let seen = std::cell::RefCell::new(SpdConnect::default());
    // Set once spd_dump opened the device (before it answers).
    let opened = std::cell::Cell::new(false);
    let mut watch = SpdWatch::new(&plan);
    // Set once the first answer is approved: the device's table, the
    // backups and (update mode) its base were checked. Any write before
    // that is fatal.
    let checked = std::cell::Cell::new(false);
    // Set when the engine stopped spd_dump with its own error, which is
    // reported as it is.
    let ours = std::cell::Cell::new(false);
    // How the helper's switches are going.
    let follow = std::cell::RefCell::new(SwitchFollow::default());
    let on_line = |line: &str| -> Result<()> {
        if !connected.get() {
            if line.starts_with("connected through ") {
                opened.set(true);
            }
            let found = seen.borrow_mut().line(line);
            if opts.host == Host::Windows {
                let mut sw = switcher.borrow_mut();
                let now = std::time::Instant::now();
                let switching = follow.borrow().switching(now);
                let action =
                    windows_wait_action(&seen.borrow(), found.as_deref(), sw.is_some(), switching);
                match action {
                    WaitAction::Wait => {}
                    WaitAction::Stop(e) => {
                        ours.set(true);
                        return Err(e);
                    }
                    WaitAction::Switch => {
                        // One at a time: the device may look different
                        // while a switch is under way.
                        if sw.as_mut().is_some_and(|w| w.request()) {
                            follow.borrow_mut().requested(now);
                            rep.info("Switching the device to WinUSB");
                        } else {
                            // Looked at again once the helper answers.
                            follow.borrow_mut().deferred();
                        }
                    }
                }
            }
        }
        if !connected.get()
            && (line.starts_with("BSL_REP_VER") || line.starts_with("load_partition"))
        {
            connected.set(true);
            rep.stage("flash", "Installing Thimsy");
        }
        if let Some((name, written, total)) = watch.line(line, checked.get())? {
            rep.progress_for("Wrote {partition}", &name, written, total);
        }
        Ok(())
    };
    // Nothing is confirmed until the device's partition table matches,
    // every backup was read in full and (update mode) the device holds the
    // base the update relies on.
    let approve = |n: usize| {
        if n == 0 {
            let (text, table) = read_device_table(&run_dir)?;
            if let Err(e) = write_device_info(&run_dir, d, opts.mode, &text) {
                rep.warn(format!(
                    "couldn't record which device the backup is from: {e:#}"
                ));
            }
            check_device(&plan.checks, &table)?;
            rep.info("The device's partition table matches this firmware");
            check_backups(&plan, &run_dir)?;
            if !plan.base.is_empty() {
                check_base(&plan, &run_dir, &entries)?;
                rep.info("The device runs the base this update needs");
            }
            checked.set(true);
        }
        Ok(())
    };
    // What the helper did, while the device isn't connected yet.
    let tick = || -> Result<()> {
        let mut sw = switcher.borrow_mut();
        let Some(sw) = sw.as_mut() else {
            return Ok(());
        };
        // Once spd_dump has the device, the switch is over.
        if connected.get() || opened.get() {
            return Ok(());
        }
        let status = match sw.step() {
            Ok(status) => status,
            Err(e) => {
                ours.set(true);
                return Err(e);
            }
        };
        let now = std::time::Instant::now();
        // A report that came while a switch was pending, if Windows still
        // gives the device another driver now.
        let other_now = seen.borrow().other_driver.is_some();
        if follow.borrow_mut().retry(sw.pending(), other_now, now) && sw.request() {
            follow.borrow_mut().requested(now);
            rep.info("Switching the device to WinUSB");
        }
        let events = follow.borrow_mut().next(status, now);
        for event in events {
            match event {
                FollowEvent::Switched { reboot } => {
                    rep.info("The device now uses WinUSB");
                    if reboot {
                        rep.warn("Windows says it needs a restart to finish switching the driver; if the device isn't found, restart the computer and try again");
                    }
                }
                // It left download mode first: the next time it's
                // connected, it's switched then.
                FollowEvent::LeftFirst => {
                    rep.info("The device left download mode before its driver could be switched");
                    rep.prompt(
                        "enter_download_mode",
                        step.as_ref()
                            .map_or("Connect the device in download mode", |s| s.title.as_str()),
                        step.as_ref().map_or("", |s| s.body.as_str()),
                        None,
                    );
                }
                FollowEvent::Failed(code) => {
                    ours.set(true);
                    return Err(EngineError::new(
                        Code::ToolFailed,
                        format!(
                            "Windows couldn't switch the device to WinUSB ({}); nothing was written",
                            crate::driver_watch::failure_text(code)
                        ),
                    )
                    .hint("Repeat the download-mode steps, then try again. If it still isn't found, check in Device Manager that Unisoc download mode uses the WinUSB driver.")
                    .into());
                }
                // Download mode has usually ended by the time the driver is
                // switched: the device must enter it again (the same USB
                // port keeps WinUSB).
                FollowEvent::Reconnect => {
                    let steps = step
                        .as_ref()
                        .map_or(String::new(), |s| format!("\n\n{}\n{}", s.title, s.body));
                    rep.prompt(
                        "reconnect_download_mode",
                        "Connect it again",
                        &format!(
                            "{}{steps}",
                            crate::i18n::tr("Windows now gives the device the driver the installer needs. Unplug the cable and turn the device off, then repeat the download-mode steps with the same USB port.")
                        ),
                        None,
                    );
                }
            }
        }
        Ok(())
    };
    let ran = tools::run_answering_ticking(&inv, &flash_dir, rep, on_line, approve, tick);
    // The helper is done either way.
    drop(switcher.into_inner());
    let connected = connected.get();
    let ran = ran.and_then(|()| watch.finish()).map_err(|e| {
        let code = crate::error::code_of(&e).0;
        if ours.get() {
            return e;
        }
        if code == Code::ToolFailed && !connected {
            // The device never answered: nothing happened on it.
            return not_connected(d, opts.host, &seen.borrow(), e);
        }
        if code != Code::ToolFailed || !checked.get() {
            // Refused or stopped before the first write was confirmed.
            return e;
        }
        e.context("flashing stopped; the device is still recoverable from download mode")
    });

    // The images read for the base check aren't backups.
    for b in &plan.base {
        let _ = fs::remove_file(run_dir.join(base_file(&b.partition)));
    }
    // The NV/calibration backups stay where spd_dump read them, in the
    // job's backup folder, whether or not the rest worked.
    let problems = finish_backups(&plan, &run_dir);
    if run_dir.join(DEVICE_PARTITIONS).exists() && !run_dir.join(DEVICE_INFO).exists() {
        if let Ok(text) = fs::read_to_string(run_dir.join(DEVICE_PARTITIONS)) {
            let _ = write_device_info(&run_dir, d, opts.mode, &text);
        }
    }
    // A run that never got to read anything leaves no empty folder.
    let _ = fs::remove_dir(&run_dir);
    ran?;
    if !problems.is_empty() {
        return Err(EngineError::new(
            Code::ToolFailed,
            format!(
                "the device was installed, but its backup is incomplete: {}. The backed-up partitions were not written; keep {} and report this",
                problems.join("; "),
                run_dir.display()
            ),
        )
        .into());
    }
    rep.stage("reboot", "Restarting the device");
    rep.info("The device restarts into Thimsy. The first boot takes a few minutes.");
    let _ = fs::remove_dir_all(&flash_dir);
    Ok(FlashResult {
        backup: run_dir.exists().then_some(run_dir),
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
            tools::run(&inv, Path::new("."), rep, |_| Ok(()))
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
            base: vec![],
        };
        let inv = spd_invocation(d, &plan, "backup");
        let s = inv.display();
        assert!(
            s.starts_with(
                "spd_dump --wait 300 keep_charge 1 fdl fdl.img 0x65000800 fdl fdl2.img 0xb4fffe00 partition_list backup/device-partitions.xml read_part"
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

    #[test]
    fn backup_runs_are_named_by_utc_time() {
        assert_eq!(util::utc_stamp(0), "19700101-000000");
        assert_eq!(util::utc_stamp(951_782_400), "20000229-000000");
        assert_eq!(util::utc_stamp(1_794_503_683), "20261112-171443");
        // Never reused: a second folder in the same second gets a suffix.
        let dir = tempfile::tempdir().unwrap();
        let a = new_backup_run(dir.path(), "").unwrap();
        let b = new_backup_run(dir.path(), "").unwrap();
        assert_ne!(a, b);
        assert!(a.is_dir() && b.is_dir());
    }

    #[test]
    fn device_check_ignores_case_and_refuses_an_empty_list() {
        let dev = vec![pac::Partition {
            name: "Super".into(),
            mib: 5600,
        }];
        check_device(&[("super".to_string(), 5600)], &dev).unwrap();
        // Nothing to compare: never "matches".
        let err = check_device(&[], &dev).unwrap_err();
        assert!(err.to_string().contains("nothing to compare"), "{err}");
    }

    fn plan_writing(parts: &[(&str, u64)]) -> SpdPlan {
        SpdPlan {
            fdl: vec![],
            backups: vec![],
            writes: parts
                .iter()
                .enumerate()
                .map(|(i, (p, b))| Write {
                    entry: p.to_uppercase(),
                    index: i,
                    partition: p.to_string(),
                    bytes: *b,
                })
                .collect(),
            erases: vec![],
            checks: vec![],
            base: vec![],
        }
    }

    #[test]
    fn spd_output_counts_only_complete_writes() {
        let plan = plan_writing(&[("uboot_a", 0x1000), ("super", 0x15e000000)]);
        let mut w = SpdWatch::new(&plan);
        assert!(w.line("file size : 0x1000", true).unwrap().is_none());
        let got = w
            .line(
                "load_partition: uboot_a, target: 0x1000, written: 0x1000",
                true,
            )
            .unwrap();
        assert_eq!(got, Some(("uboot_a".to_string(), 0x1000, 0x15e001000)));
        // Not every write was reported: spd_dump's exit status isn't enough.
        let err = w.finish().unwrap_err();
        assert!(err.to_string().contains("super"), "{err}");
        w.line(
            "load_partition: super, target: 0x15e000000, written: 0x15e000000",
            true,
        )
        .unwrap();
        w.finish().unwrap();
    }

    #[test]
    fn a_partial_write_stops_the_flash() {
        let plan = plan_writing(&[("super", 0x15e000000)]);
        // load_partition breaks off on an unexpected response, says how far
        // it got, and spd_dump may still exit 0.
        let mut w = SpdWatch::new(&plan);
        let err = w
            .line(
                "load_partition: super, target: 0x15e000000, written: 0x3000000",
                true,
            )
            .unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed);
        assert!(
            err.to_string()
                .contains("stopped after 50331648 of 5872025600 bytes"),
            "{err}"
        );
        let err = w.line("unexpected response (0x00fe)", true).unwrap_err();
        assert!(err.to_string().contains("unexpected response"), "{err}");
        // A full write of the wrong size, or of a partition not planned.
        let err = w
            .line("load_partition: super, target: 0x10, written: 0x10", true)
            .unwrap_err();
        assert!(err.to_string().contains("its image is"), "{err}");
        let err = w
            .line("load_partition: prodnv, target: 0x10, written: 0x10", true)
            .unwrap_err();
        assert!(err.to_string().contains("isn't in the plan"), "{err}");
        let err = w.line("load_partition: super, garbled", true).unwrap_err();
        assert!(err.to_string().contains("can't be read"), "{err}");
    }

    #[test]
    fn writing_before_the_device_was_checked_is_fatal() {
        let plan = plan_writing(&[("super", 0x10)]);
        let mut w = SpdWatch::new(&plan);
        // Reading is fine before the check.
        w.line(
            "dump_partition: prodnv+0x0, target: 0x4000000, read: 0x4000000",
            false,
        )
        .unwrap();
        w.line("BSL_REP_VER: \"Spreadtrum Boot Block version 1.1\"", false)
            .unwrap();
        let err = w
            .line("load_partition: super, target: 0x10, written: 0x10", false)
            .unwrap_err();
        assert!(err.to_string().contains("without asking first"), "{err}");
        assert!(w.line("erase_partition: userdata", false).is_err());
    }

    #[test]
    fn windows_says_why_the_device_couldnt_be_opened() {
        let cat = crate::catalog::Catalog::bundled().unwrap();
        let d = cat.device("anbernic-rg-cube").unwrap();
        let failed = || anyhow::anyhow!("spd_dump failed: libusb_open_device failed");
        let hint = |e: &anyhow::Error| {
            e.downcast_ref::<EngineError>()
                .and_then(|e| e.hint.clone())
                .unwrap_or_default()
        };
        // What the Windows spd_dump prints while it waits.
        let mut seen = SpdConnect::default();
        seen.line("Waiting for connection (300s)");
        seen.line("Windows has: 1782:4d00 sprdvcom Ports COM5");
        assert_eq!(seen, SpdConnect::default());
        seen.line("download mode uses another driver (sprdvcom Ports COM5), not WinUSB");
        assert_eq!(seen.other_driver.as_deref(), Some("sprdvcom Ports COM5"));
        // Still on that driver when spd_dump gave up.
        let e = not_connected(d, Host::Windows, &seen, failed());
        assert_eq!(crate::error::code_of(&e).0, Code::DeviceNotFound);
        assert!(
            e.to_string().contains(
                "Windows gives it another driver (sprdvcom Ports COM5) instead of WinUSB"
            ),
            "{e}"
        );
        // Any other driver spd_dump reports is switched.
        let mut fresh = SpdConnect::default();
        let found =
            fresh.line("download mode uses another driver (no driver USB problem 28), not WinUSB");
        assert_eq!(found.as_deref(), Some("no driver USB problem 28"));
        assert!(matches!(
            windows_wait_action(&fresh, found.as_deref(), true, false),
            WaitAction::Switch
        ));
        assert!(matches!(
            windows_wait_action(&fresh, None, true, false),
            WaitAction::Wait
        ));
        // Without the helper there's nothing to switch it with; the next
        // try starts it up front.
        match windows_wait_action(&fresh, found.as_deref(), false, false) {
            WaitAction::Stop(e) => {
                assert!(e.to_string().contains("Try again"), "{e}");
                assert!(
                    e.to_string()
                        .contains("another driver (no driver USB problem 28)"),
                    "{e}"
                );
                assert!(hint(&e).contains("uses the WinUSB driver"), "{}", hint(&e));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fresh.line("Windows has: 1782:4d00 WinUSB USBDevice"), None);
        assert!(hint(&e).contains("uses the WinUSB driver"), "{}", hint(&e));
        // Opened in the end.
        let mut opened = seen.clone();
        opened.line("connected through libusb (WinUSB)");
        assert_eq!(opened, SpdConnect::default());

        // Unisoc's tools hold the port: Windows can't switch its driver.
        let mut seen = SpdConnect::default();
        seen.line("serial port COM9 is in use by another program");
        assert_eq!(seen.port_busy.as_deref(), Some("COM9"));
        // That stops the wait at once, whatever else was said.
        assert!(matches!(
            windows_wait_action(&seen, Some("sprdvcom Ports COM9"), true, false),
            WaitAction::Stop(_)
        ));
        // A new state from spd_dump clears it (said again if it holds).
        let mut later = seen.clone();
        later.line("Windows has: 1782:4d00 WinUSB USBDevice");
        assert_eq!(later.port_busy, None);
        assert!(matches!(
            windows_wait_action(&later, None, true, false),
            WaitAction::Wait
        ));
        // Unless its driver is being switched: Windows may hold it then.
        assert!(matches!(
            windows_wait_action(&seen, None, true, true),
            WaitAction::Wait
        ));
        assert!(matches!(
            windows_wait_action(&seen, Some("sprdvcom Ports COM9"), true, true),
            WaitAction::Switch
        ));
        let e = not_connected(d, Host::Windows, &seen, failed());
        assert!(
            e.to_string().contains("another program is using it (COM9)"),
            "{e}"
        );
        assert!(hint(&e).contains("ResearchDownload"), "{}", hint(&e));
        // The same when another program holds the device through WinUSB.
        let mut seen = SpdConnect::default();
        seen.line("download mode is connected, but libusb can't open it : LIBUSB_ERROR_ACCESS");
        let e = not_connected(d, Host::Windows, &seen, failed());
        assert!(e.to_string().contains("another program is using it"), "{e}");
        assert!(hint(&e).contains("ResearchDownload"), "{}", hint(&e));

        let mut seen = SpdConnect::default();
        seen.line(
            "download mode is connected, but libusb can't open it : LIBUSB_ERROR_NOT_SUPPORTED",
        );
        assert!(seen.other_driver.is_none());
        let e = not_connected(d, Host::Windows, &seen, failed());
        assert!(e.to_string().contains("a driver the installer can't use (download mode is connected, but libusb can't open it : LIBUSB_ERROR_NOT_SUPPORTED)"), "{e}");
        assert!(hint(&e).contains("uses the WinUSB driver"), "{}", hint(&e));

        // Never seen at all: repeat the steps; WinUSB isn't a requirement.
        let e = not_connected(d, Host::Windows, &SpdConnect::default(), failed());
        assert!(e.to_string().contains("never answered"), "{e}");
        assert!(!hint(&e).contains("WinUSB"), "{}", hint(&e));
        // Other hosts never get the Windows driver advice.
        let e = not_connected(d, Host::Macos, &seen, failed());
        assert!(e.to_string().contains("never answered"), "{e}");
    }
}
