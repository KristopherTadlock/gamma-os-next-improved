//! Build jobs: resolve → fetch → extract → assemble → validate → publish,
//! journaled so an interrupted build resumes from the last finished stage.

use crate::catalog::{Artifact, Catalog, Device, Op};
use crate::error::{Code, EngineError};
use crate::events::Reporter;
use crate::fetch::Cache;
use crate::host::Host;
use crate::lp::Super;
use crate::resolve;
use crate::{archive, delta, pac, util};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub device: String,
    pub profile: String,
    pub workspace: PathBuf,
    pub imports: Vec<PathBuf>,
    pub overrides: Vec<(String, String)>,
    pub host: Host,
    /// Delete each download as soon as its last use is done.
    pub delete_downloads: bool,
}

/// Free-space check for a build, counting downloads already in the cache.
#[derive(Debug, Clone, Serialize)]
pub struct SpaceCheck {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub need: u64,
    /// Free bytes on the workspace's drive (0 when it couldn't be checked).
    pub free: u64,
    pub ok: bool,
    /// Why `ok` is false: `fat32`, `probe_failed` or `not_enough_space`.
    pub reason: Option<&'static str>,
    /// What to do about it (translated), when `ok` is false.
    pub hint: Option<String>,
    pub build_need: u64,
    pub flash_need: u64,
    /// Download bytes already in the cache.
    pub cached: u64,
    /// Download bytes the picked files (`--import`) stand in for.
    pub imported: u64,
    /// Every download this build uses, per the catalog.
    pub downloads: u64,
}

/// What the user's picked files and folders stand in for, in download
/// bytes: used where they are (never copied), or copied into the cache.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FromImports {
    pub in_place: u64,
    pub copied: u64,
}

/// The downloads `imports` make unnecessary, the way `build` uses them: a
/// device pack (or all its parts) used in place; the unpacked official
/// firmware, which makes its download parts unnecessary; any other file by
/// name and size, copied into the cache.
pub fn from_imports(cat: &Catalog, d: &Device, profile: &str, imports: &[PathBuf]) -> FromImports {
    let mut r = FromImports::default();
    if imports.is_empty() {
        return r;
    }
    let copied = |a: &Artifact| -> u64 {
        if a.parts.is_empty() {
            crate::fetch::find_in(imports, &a.file, a.size).map_or(0, |_| a.size)
        } else {
            a.parts
                .iter()
                .filter(|p| crate::fetch::find_in(imports, &p.file, p.size).is_some())
                .map(|p| p.size)
                .sum()
        }
    };
    if let Some(src) = &d.source {
        if crate::fetch::find_unpacked(imports, &src.contains, src.container.size).is_some() {
            r.in_place += src.parts.iter().map(|p| p.size).sum::<u64>();
        } else {
            r.copied += src.parts.iter().map(copied).sum::<u64>();
        }
    }
    match d.bundles.get(profile) {
        Some(b) => {
            if matches!(crate::fetch::local_bundle(imports, b), Ok(Some(_))) {
                r.in_place += b.size;
            }
        }
        None => {
            r.copied += d.artifacts.iter().map(copied).sum::<u64>();
            r.copied += cat.system_for(profile).map_or(0, copied);
        }
    }
    r
}

/// The largest single file a build of `profile` writes (FAT32 can't hold
/// one over 4 GiB).
fn largest_file(cat: &Catalog, d: &Device, profile: &str) -> u64 {
    let mut arts: Vec<&Artifact> = Vec::new();
    if let Some(src) = &d.source {
        arts.extend(src.parts.iter());
    }
    match d.bundles.get(profile) {
        Some(b) => arts.push(b),
        None => {
            arts.extend(d.artifacts.iter());
            arts.extend(cat.system_for(profile));
        }
    }
    let image = d.source.as_ref().map_or(0, |s| s.container.size);
    arts.iter()
        .flat_map(|a| {
            if a.parts.is_empty() {
                vec![a.size]
            } else {
                a.parts.iter().map(|p| p.size).collect()
            }
        })
        .chain([image])
        .max()
        .unwrap_or(0)
}

pub fn check_space(
    cat: &Catalog,
    device: &str,
    profile: &str,
    workspace: &Path,
    imports: &[PathBuf],
    delete_downloads: bool,
) -> Result<SpaceCheck> {
    let d = cat.device(device)?;
    let sp = resolve::space(cat, d, profile);
    let cache_dir = workspace.join("cache");
    // Exactly what `space` counts as downloads: with a device pack, the
    // system image and patches come from it, not from separate files.
    let mut arts: Vec<&Artifact> = Vec::new();
    if let Some(src) = &d.source {
        arts.extend(src.parts.iter());
    }
    match d.bundles.get(profile) {
        Some(b) => arts.push(b),
        None => {
            arts.extend(d.artifacts.iter());
            arts.extend(cat.system_for(profile));
        }
    }
    let cached: u64 = arts
        .iter()
        .flat_map(|a| {
            if a.parts.is_empty() {
                vec![*a]
            } else {
                a.parts.iter().collect()
            }
        })
        .filter(|a| fs::metadata(cache_dir.join(&a.file)).is_ok_and(|m| m.len() == a.size))
        .map(|a| a.size)
        .sum::<u64>()
        .min(sp.downloads);
    let imp = from_imports(cat, d, profile, imports);
    // Picked files used in place take no room; copies of them do (counted
    // like downloads, but not as cached).
    let in_place = imp.in_place.min(sp.downloads - cached);
    let (build_need, flash_need) = sp.need(cached, in_place, delete_downloads);
    let need = build_need.max(flash_need);
    let (free, reason, hint) = match crate::disk::probe(workspace) {
        Err(_) => (
            0,
            Some("probe_failed"),
            Some("The free space on this drive couldn't be checked. Choose another folder for the workspace."),
        ),
        Ok(p) if p.fat32 && largest_file(cat, d, profile) > crate::disk::FAT32_MAX_FILE => (
            p.free,
            Some("fat32"),
            Some("This drive is formatted as FAT32, which can't hold files over 4 GB. Choose a workspace on another drive (NTFS, exFAT, APFS or ext4)."),
        ),
        Ok(p) if p.free < need => (
            p.free,
            Some("not_enough_space"),
            Some("Choose a workspace on a drive with more free space (not FAT32), or free some space."),
        ),
        Ok(p) => (p.free, None, None),
    };
    Ok(SpaceCheck {
        kind: "space",
        need,
        free,
        ok: reason.is_none(),
        reason,
        hint: hint.map(crate::i18n::tr),
        build_need,
        flash_need,
        cached,
        imported: imp.in_place + imp.copied,
        downloads: sp.downloads,
    })
}

/// The error `build` fails with when `check_space` says no.
fn space_error(workspace: &Path, sc: &SpaceCheck) -> anyhow::Error {
    match sc.reason {
        Some("fat32") => EngineError::new(
            Code::BadInput,
            format!(
                "{} is on a FAT32 drive, which can't hold the {} firmware image (files over 4 GB)",
                workspace.display(),
                util::human_bytes(sc.need)
            ),
        )
        .hint("This drive is formatted as FAT32, which can't hold files over 4 GB. Choose a workspace on another drive (NTFS, exFAT, APFS or ext4).")
        .into(),
        Some("probe_failed") => EngineError::new(
            Code::BadInput,
            format!("the free space in {} couldn't be checked", workspace.display()),
        )
        .hint("The free space on this drive couldn't be checked. Choose another folder for the workspace.")
        .into(),
        _ => not_enough(workspace, sc.need, sc.free),
    }
}

fn not_enough(workspace: &Path, need: u64, free: u64) -> anyhow::Error {
    EngineError::new(
        Code::NotEnoughSpace,
        format!(
            "{} needs about {} free; only {} is available",
            workspace.display(),
            util::human_bytes(need),
            util::human_bytes(free)
        ),
    )
    .hint("Choose a workspace on a drive with more free space (not FAT32), or free some space.")
    .into()
}

pub(crate) fn not_enough_space(workspace: &Path, need: u64, free: u64) -> anyhow::Error {
    not_enough(workspace, need, free)
}

/// Held while a command changes a workspace (build, flash, clean): an
/// exclusive lock on `<workspace>/.thimsy.lock`, released when dropped
/// (or when the engine exits, however it ends).
#[derive(Debug)]
pub struct WorkspaceLock {
    _file: File,
}

/// Name of the lock file in a workspace.
pub const LOCK_FILE: &str = ".thimsy.lock";

/// Takes the workspace lock, or fails with `busy` when another run holds it.
pub fn lock_workspace(workspace: &Path) -> Result<WorkspaceLock> {
    fs::create_dir_all(workspace).with_context(|| format!("creating {}", workspace.display()))?;
    let path = workspace.join(LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    // A lock just released may still be held for a moment by a process
    // that was being started as it was dropped (fork copies descriptors
    // until exec closes them): retry briefly before calling it busy.
    let mut tries = 0;
    let got = loop {
        match fs4::FileExt::try_lock(&file) {
            Err(fs4::TryLockError::WouldBlock) if tries < 20 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            r => break r,
        }
    };
    match got {
        Ok(()) => Ok(WorkspaceLock { _file: file }),
        Err(fs4::TryLockError::WouldBlock) => Err(EngineError::new(
            Code::Busy,
            format!(
                "another Thimsy Installer task is using {}",
                workspace.display()
            ),
        )
        .hint("Another Thimsy Installer task is using this workspace. Wait for it to finish, or close the other installer window.")
        .into()),
        Err(fs4::TryLockError::Error(e)) => {
            Err(e).with_context(|| format!("locking {}", path.display()))
        }
    }
}

/// The workspace a job folder belongs to (`<workspace>/jobs/<job>`), or the
/// folder itself when it isn't laid out that way.
pub fn workspace_of(job: &Path) -> PathBuf {
    match job.parent() {
        Some(jobs) if jobs.file_name().is_some_and(|n| n == "jobs") => jobs
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| job.to_path_buf()),
        _ => job.to_path_buf(),
    }
}

/// Written to `<job>/job.json` when a build finishes; `flash` reads it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub engine: String,
    pub catalog: String,
    pub device: String,
    pub profile: String,
    pub overrides: Vec<(String, String)>,
    pub image: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub matches_reference: Option<bool>,
    /// sha256 of every PAC entry with data, so the partitions unpacked for
    /// flashing can be checked one by one before they're written (absent
    /// in jobs from earlier engines).
    #[serde(default)]
    pub entries: Vec<EntryHash>,
}

/// One PAC entry of a finished image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryHash {
    pub index: usize,
    pub id: String,
    pub size: u64,
    pub sha256: String,
}

/// The image's sha256 and each data entry's, in one pass over the file.
pub fn hash_image(
    path: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(String, Vec<EntryHash>)> {
    let p = pac::Pac::open(path)?;
    let with_data: Vec<&pac::Entry> = p.entries.iter().filter(|e| e.size > 0).collect();
    let ranges: Vec<(u64, u64)> = with_data.iter().map(|e| (e.offset, e.size)).collect();
    let (whole, parts) = util::sha256_with_ranges(path, &ranges, progress)?;
    let entries = with_data
        .iter()
        .zip(parts)
        .map(|(e, sha256)| EntryHash {
            index: e.index,
            id: e.id.clone(),
            size: e.size,
            sha256,
        })
        .collect();
    Ok((whole, entries))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    extracted: bool,
    assembling: bool,
}

/// The Thimsy build shipped with the installer: a `thimsy-build` folder next
/// to the engine, one level up (next to the app), or next to the macOS
/// `.app` bundle the engine is inside. Searched before downloading, like an
/// import folder.
pub fn bundled_build_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = std::env::var("THIMSY_BUILD_DIR") {
        out.push(PathBuf::from(dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        out.extend(build_dirs_near(&exe).into_iter().filter(|c| c.is_dir()));
    }
    out
}

/// Where a `thimsy-build` folder may sit for an engine at `exe`.
pub fn build_dirs_near(exe: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = exe.parent() {
        out.push(dir.join("thimsy-build"));
        out.push(dir.join("..").join("thimsy-build"));
    }
    let app = exe
        .ancestors()
        .skip(1)
        .find(|a| a.extension().is_some_and(|x| x.eq_ignore_ascii_case("app")));
    if let Some(folder) = app.and_then(Path::parent) {
        out.push(folder.join("thimsy-build"));
    }
    out
}

/// Files the build can use from the user's picks: the official firmware
/// (its download parts or the unpacked file), the device pack (or its
/// parts), the system image and the patches.
fn wanted_files<'a>(cat: &'a Catalog, d: &'a Device, profile: &str) -> Vec<(&'a str, u64)> {
    let mut out = Vec::new();
    let mut add = |a: &'a Artifact| {
        out.push((a.file.as_str(), a.size));
        out.extend(a.parts.iter().map(|p| (p.file.as_str(), p.size)));
    };
    if let Some(src) = &d.source {
        src.parts.iter().for_each(&mut add);
    }
    if let Some(b) = d.bundles.get(profile) {
        add(b);
    }
    if let Some(s) = cat.system_for(profile) {
        add(s);
    }
    d.artifacts.iter().for_each(&mut add);
    if let Some(src) = &d.source {
        out.push((src.contains.as_str(), src.container.size));
    }
    out
}

/// When the user picked files (not folders) and none of them is anything
/// this build uses, say so instead of quietly downloading everything.
fn check_picked_files(cat: &Catalog, d: &Device, profile: &str, imports: &[PathBuf]) -> Result<()> {
    let picked: Vec<&PathBuf> = imports.iter().filter(|p| !p.is_dir()).collect();
    if picked.is_empty() {
        return Ok(());
    }
    // Some parts of a split device pack picked but not all: say which are
    // missing (picked parts never mix with downloads).
    if let Some(b) = d.bundles.get(profile) {
        crate::fetch::local_bundle(imports, b)?;
    }
    let wanted = wanted_files(cat, d, profile);
    let useful = picked.iter().any(|p| {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string());
        let size = fs::metadata(p)
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        wanted
            .iter()
            .any(|(f, s)| name.as_deref() == Some(*f) || size == Some(*s))
    });
    if useful {
        return Ok(());
    }
    let names: Vec<String> = picked.iter().map(|p| p.display().to_string()).collect();
    let mut expected = Vec::new();
    if let Some(b) = d.bundles.get(profile) {
        expected.push(format!("the device pack {} ({} bytes)", b.file, b.size));
    }
    if let Some(src) = &d.source {
        expected.push(format!(
            "the official firmware {} ({} bytes) or its download parts ({})",
            src.contains,
            src.container.size,
            src.parts
                .iter()
                .map(|p| p.file.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Err(EngineError::new(
        Code::BadInput,
        format!(
            "none of the files you chose is used by this build ({}); expected {}",
            names.join(", "),
            expected.join(", or ")
        ),
    )
    .into())
}

/// Fetches and unpacks a device's official source firmware into `dest`
/// (maintainer tooling: the input of `dev pack`).
pub fn prepare_source(
    cat: &Catalog,
    device: &str,
    workspace: &Path,
    imports: &[PathBuf],
    rep: &Reporter,
) -> Result<PathBuf> {
    let d = cat.device(device)?;
    let src = d.source.as_ref().context("device has no source firmware")?;
    let cache = Cache::new(workspace.join("cache"), imports.to_vec())?;
    let mut parts = Vec::new();
    for p in &src.parts {
        parts.push(cache.get(p, rep)?);
    }
    let out_dir = workspace.join("source");
    fs::create_dir_all(&out_dir)?;
    let out = out_dir.join(&src.contains);
    if fs::metadata(&out).is_ok_and(|m| m.len() == src.container.size)
        && util::sha256_file(&out, |_, _| {})? == src.container.sha256
    {
        return Ok(out);
    }
    let tmp = out.with_extension("partial");
    archive::extract_one(
        &parts,
        src.password.as_deref(),
        &src.contains,
        &tmp,
        &mut |d, t| rep.progress_for("Unpacking {file}", &src.contains, d, t),
    )?;
    if util::sha256_file(&tmp, |_, _| {})? != src.container.sha256 {
        bail!(
            "unpacked {} doesn't match the catalog's sha256",
            src.contains
        );
    }
    fs::rename(&tmp, &out)?;
    Ok(out)
}

pub fn job_dir(workspace: &Path, device: &str, profile: &str) -> PathBuf {
    workspace.join("jobs").join(format!("{device}-{profile}"))
}

fn load_state(dir: &Path) -> State {
    fs::read(dir.join("state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_state(dir: &Path, s: &State) -> Result<()> {
    util::write_atomic(&dir.join("state.json"), &serde_json::to_vec_pretty(s)?)
}

pub fn output_name(d: &Device, profile: &str) -> String {
    format!("{}-thimsy-{profile}.pac", d.id)
}

pub fn build(cat: &Catalog, opts: &BuildOptions, rep: &Reporter) -> Result<JobRecord> {
    rep.stage("plan", "Checking the plan");
    // One run at a time per workspace, for the whole build.
    let _lock = lock_workspace(&opts.workspace)?;
    if !opts.overrides.is_empty() {
        // `resolve` shows them, but no build step applies them yet.
        return Err(EngineError::new(
            Code::BadInput,
            "custom settings (--set) aren't supported by this version yet; build without them",
        )
        .into());
    }
    let plan = resolve::resolve(cat, &opts.device, &opts.profile, &opts.overrides, opts.host)?;
    if !plan.available {
        return Err(EngineError::new(
            Code::ProfileUnavailable,
            plan.unavailable_reason.clone().unwrap_or_default(),
        )
        .into());
    }
    if let Some(msg) = &plan.host_message {
        rep.warn(format!(
            "{msg} The image will still be built so you can finish on another computer."
        ));
    }
    let d = cat.device(&opts.device)?;
    check_picked_files(cat, d, &opts.profile, &opts.imports)?;
    let src = d.source.as_ref().context("device has no source firmware")?;
    let system = cat.system_for(&opts.profile).cloned();
    let dir = job_dir(&opts.workspace, &d.id, &opts.profile);
    let work = dir.join("work");
    let out_dir = dir.join("out");
    fs::create_dir_all(&work)?;
    fs::create_dir_all(&out_dir)?;
    let final_path = out_dir.join(output_name(d, &opts.profile));
    let working = work.join(&src.contains);

    // Disk space: what's still missing must fit, with headroom. Checked
    // before anything is downloaded or written.
    let mut imports = opts.imports.clone();
    imports.extend(bundled_build_dirs());
    // The unpacked official firmware in an import folder saves downloading
    // and unpacking it (checked by size now, by sha256 once copied).
    let mut cache = Cache::new(opts.workspace.join("cache"), imports)?;
    let unpacked = crate::fetch::find_unpacked(&cache.imports, &src.contains, src.container.size);
    let sc = check_space(
        cat,
        &d.id,
        &opts.profile,
        &opts.workspace,
        &opts.imports,
        opts.delete_downloads,
    )?;
    rep.info(format!(
        "Needs about {} free in {}; {} available",
        util::human_bytes(sc.need),
        opts.workspace.display(),
        util::human_bytes(sc.free)
    ));
    if !sc.ok {
        return Err(space_error(&opts.workspace, &sc));
    }

    rep.stage("fetch", "Downloading");
    // The device pack as one archive: the system image and patches are read
    // from it in place.
    let bundle_spans = match d.bundles.get(&opts.profile) {
        Some(b) => {
            let spans = cache.get_in_place(b, rep)?;
            cache.bundles.push(crate::bundle::Bundle::index(&spans)?);
            Some((b.clone(), spans))
        }
        None => None,
    };
    let mut state = load_state(&dir);
    if state.assembling || !working.exists() {
        state.extracted = false;
    }
    // The official firmware's download parts, only when they'll be unpacked.
    let mut parts = Vec::new();
    if !state.extracted && unpacked.is_none() {
        for p in &src.parts {
            parts.push(cache.get(p, rep)?);
        }
    }
    let system_paths = match &system {
        Some(s) => Some(cache.get_all(s, rep)?),
        None => None,
    };
    let mut arts = std::collections::HashMap::new();
    for a in &d.artifacts {
        arts.insert(a.id.clone(), (a.clone(), cache.get(a, rep)?));
    }

    if !state.extracted {
        rep.stage("extract", "Unpacking the firmware");
        let tmp = working.with_extension("partial");
        let kind = if unpacked.is_some() {
            "unpacked"
        } else {
            src.kind.as_str()
        };
        match kind {
            "unpacked" => {
                let from = unpacked.as_ref().expect("checked above");
                rep.info(format!("Using {}", from.display()));
                let mut i = File::open(from)?;
                let mut o = File::create(&tmp)?;
                let n = src.container.size;
                util::copy_exact(&mut i, &mut o, n, &mut |d| {
                    rep.progress("Copying the firmware", d, n)
                })?;
            }
            "archive7z" => {
                archive::extract_one(
                    &parts,
                    src.password.as_deref(),
                    &src.contains,
                    &tmp,
                    &mut |d, t| rep.progress_for("Unpacking {file}", &src.contains, d, t),
                )?;
            }
            "file" => {
                let mut i = File::open(&parts[0])?;
                let mut o = File::create(&tmp)?;
                let n = i.metadata()?.len();
                util::copy_exact(&mut i, &mut o, n, &mut |d| {
                    rep.progress("Copying the firmware", d, n)
                })?;
            }
            k => bail!("unknown source kind {k}"),
        }
        let got = util::sha256_file(&tmp, |d, t| {
            rep.progress("Checking the unpacked firmware", d, t)
        })?;
        if got != src.container.sha256 {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!(
                    "the unpacked {} has sha256 {got}, expected {}",
                    src.contains, src.container.sha256
                ),
            )
            .into());
        }
        fs::rename(&tmp, &working)?;
        state = State {
            extracted: true,
            assembling: false,
        };
        save_state(&dir, &state)?;
        if opts.delete_downloads {
            for (a, p) in src.parts.iter().zip(&parts) {
                remove_download(&cache, a, p, rep);
            }
        }
    } else {
        rep.info("Firmware already unpacked");
    }

    rep.stage("assemble", "Building Thimsy for your device");
    state.assembling = true;
    save_state(&dir, &state)?;
    for (i, op) in d.build.iter().enumerate() {
        rep.info(format!(
            "Step {}/{}: {}",
            i + 1,
            d.build.len(),
            describe(op)
        ));
        run_op(
            op,
            &working,
            system.as_ref(),
            system_paths.as_deref(),
            &arts,
            &opts.profile,
            cat,
            rep,
        )
        .with_context(|| format!("build step {} ({})", i + 1, describe(op)))?;
    }

    if opts.delete_downloads {
        if let (Some(a), Some(ps)) = (&system, &system_paths) {
            if cache.in_bundle(a) {
                // Read from the device pack; removed with it below.
            } else if a.parts.is_empty() {
                remove_download(&cache, a, &ps[0].path, rep);
            } else {
                for (part, p) in a.parts.iter().zip(ps) {
                    remove_download(&cache, part, &p.path, rep);
                }
                let _ = fs::remove_file(cache.dir.join(format!("{}.ok", a.file)));
            }
        }
        for (a, p) in arts.values() {
            remove_download(&cache, a, p, rep);
        }
        // The device pack itself, if it was downloaded (never a picked file).
        if let Some((b, spans)) = &bundle_spans {
            let parts: Vec<&Artifact> = if b.parts.is_empty() {
                vec![b]
            } else {
                b.parts.iter().collect()
            };
            for (a, sp) in parts.into_iter().zip(spans) {
                if sp.path.starts_with(&cache.dir) {
                    remove_download(&cache, a, &sp.path, rep);
                }
            }
            let _ = fs::remove_file(cache.dir.join(format!("{}.ok", b.file)));
        }
    }

    rep.stage("validate", "Checking the result");
    let p = pac::Pac::open(&working)?;
    for op in &d.build {
        if let Op::LpWrite { entry, .. } | Op::LpGrow { entry, .. } = op {
            let e = p.find(entry)?;
            let mut f = File::open(&working)?;
            Super::new(&mut f, e.offset, e.size)
                .partitions()
                .context("super metadata after build")?;
        }
    }
    let bytes = fs::metadata(&working)?.len();
    let (sha, entries) = hash_image(&working, &mut |d, t| {
        rep.progress("Checksumming the image", d, t)
    })?;
    let matches = d.expect.get(&opts.profile).map(|want| want == &sha);
    match matches {
        Some(true) => rep.info("The image matches the reference build exactly"),
        Some(false) => {
            return Err(EngineError::new(
                Code::HashMismatch,
                format!("the built image (sha256 {sha}) doesn't match the reference build"),
            )
            .hint("Delete the workspace's jobs folder and build again; report it if it persists.")
            .into());
        }
        None => rep.info(format!("Image sha256 {sha}")),
    }

    rep.stage("publish", "Finishing");
    // Partitions unpacked from an earlier image must never be flashed with
    // this one.
    let flash_dir = dir.join("flash");
    crate::flash::rescue_old_backups(&dir)?;
    if flash_dir.exists() {
        fs::remove_dir_all(&flash_dir)
            .with_context(|| format!("removing {}", flash_dir.display()))?;
    }
    fs::rename(&working, &final_path)?;
    let rec = JobRecord {
        engine: crate::VERSION.into(),
        catalog: cat.root.version.clone(),
        device: d.id.clone(),
        profile: opts.profile.clone(),
        overrides: opts.overrides.clone(),
        image: final_path.clone(),
        bytes,
        sha256: sha,
        matches_reference: matches,
        entries,
    };
    util::write_atomic(&dir.join("job.json"), &serde_json::to_vec_pretty(&rec)?)?;
    fs::write(dir.join("plan.json"), serde_json::to_vec_pretty(&plan)?)?;
    save_state(&dir, &State::default())?;
    let _ = fs::remove_dir(&work);
    Ok(rec)
}

fn remove_download(cache: &Cache, a: &Artifact, p: &Path, rep: &Reporter) {
    if fs::remove_file(p).is_ok() {
        let _ = fs::remove_file(cache.dir.join(format!("{}.ok", a.file)));
        rep.info(format!(
            "Deleted {} ({}) to save space",
            a.file,
            util::human_bytes(a.size)
        ));
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanItem {
    pub kind: &'static str, // downloads | image | backup | scratch
    pub path: PathBuf,
    pub bytes: u64,
    /// For one run's backups: its `device.json` (which device and partition
    /// table they came from, and when), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

fn dir_size(p: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(p) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Every file name the catalog can put in a cache.
fn known_files(cat: &Catalog) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    let mut add = |a: &Artifact| {
        names.insert(a.file.clone());
        for p in &a.parts {
            names.insert(p.file.clone());
        }
    };
    cat.root.artifacts.iter().for_each(&mut add);
    for d in &cat.devices {
        d.artifacts.iter().for_each(&mut add);
        d.bundles.values().for_each(&mut add);
        if let Some(src) = &d.source {
            src.parts.iter().for_each(&mut add);
        }
    }
    names
}

/// What a cache entry called `name` is, if the engine put it there: a
/// download (`downloads`), a marker (`ok`, removed with its download) or
/// leftovers of an unfinished one (`scratch`). `None`: not ours, never
/// touched.
fn cache_kind(name: &str, known: &std::collections::HashSet<String>) -> Option<&'static str> {
    if known.contains(name) {
        return Some("downloads");
    }
    let base = name.strip_suffix(".tmp").unwrap_or(name);
    for (suffix, kind) in [
        (".partial.json", "scratch"),
        (".partial", "scratch"),
        (".import", "scratch"),
        (".ok", "ok"),
    ] {
        if let Some(stem) = base.strip_suffix(suffix) {
            if known.contains(stem) {
                return Some(if base != name { "scratch" } else { kind });
            }
        }
    }
    if base != name && known.contains(base) {
        return Some("scratch");
    }
    // Earlier engines named some scratch files `<stem>.import` / `.tmp`.
    let legacy = known.iter().any(|k| {
        let p = Path::new(k);
        p.with_extension("import").as_os_str() == name
            || p.with_extension("tmp").as_os_str() == name
    });
    legacy.then_some("scratch")
}

/// What `clean` could remove from a workspace: files in `cache/` the
/// catalog names (and their leftovers), and the folders of jobs named
/// `<device>-<profile>` for catalog devices and profiles. Nothing else in
/// the workspace is ever listed or removed.
pub fn cleanable(cat: &Catalog, workspace: &Path) -> Vec<CleanItem> {
    let mut items = Vec::new();
    let known = known_files(cat);
    let mut cache: Vec<PathBuf> = fs::read_dir(workspace.join("cache"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    cache.sort();
    for p in cache {
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let kind = match cache_kind(&name, &known) {
            Some("ok") | None => continue,
            Some(k) => k,
        };
        if fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir()) {
            continue;
        }
        items.push(CleanItem {
            kind,
            bytes: dir_size(&p),
            path: p,
            info: None,
        });
    }
    let mut jobs = Vec::new();
    for d in &cat.devices {
        for p in &cat.profiles {
            let j = job_dir(workspace, &d.id, &p.id);
            if fs::symlink_metadata(&j).is_ok_and(|m| m.is_dir()) {
                jobs.push(j);
            }
        }
    }
    jobs.sort();
    for j in jobs {
        for (sub, kind) in [("work", "scratch"), ("flash", "scratch"), ("out", "image")] {
            let p = j.join(sub);
            if !p.exists() {
                continue;
            }
            let mut bytes = dir_size(&p);
            if sub == "flash" {
                bytes -= dir_size(&p.join("backup")).min(bytes);
            }
            if bytes > 0 {
                items.push(CleanItem {
                    kind,
                    path: p,
                    bytes,
                    info: None,
                });
            }
        }
        // Each run's backups on their own, so the app can keep some.
        let mut runs: Vec<PathBuf> = fs::read_dir(j.join("backup"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        runs.sort();
        // Backups an earlier engine version left in the scratch folder:
        // never scratch (`clean --scratch` moves them to backup/).
        let legacy = j.join("flash").join("backup");
        if legacy.is_dir() {
            runs.push(legacy);
        }
        for p in runs {
            let bytes = dir_size(&p);
            if bytes == 0 {
                continue;
            }
            let info = fs::read(p.join(crate::flash::DEVICE_INFO))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
            items.push(CleanItem {
                kind: "backup",
                path: p,
                bytes,
                info,
            });
        }
    }
    items
}

/// Lexically the same path (`.`/`..` and separators normalised).
fn same_path(a: &Path, b: &Path) -> bool {
    fn norm(p: &Path) -> PathBuf {
        let p = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let mut out = PathBuf::new();
        for c in p.components() {
            match c {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    out.pop();
                }
                c => out.push(c),
            }
        }
        out
    }
    norm(a) == norm(b)
}

/// Removes the chosen kinds, and the items listed in `paths` (each must be
/// an item [`cleanable`] lists; otherwise nothing is removed and the
/// command fails with `bad_input`). Returns bytes freed. Backups are
/// removed only when `backup` is asked for or their path is listed:
/// removing a job's scratch folder first moves any backup left in it to
/// the job's backup folder. Holds the workspace lock throughout.
pub fn clean(cat: &Catalog, workspace: &Path, kinds: &[&str], paths: &[PathBuf]) -> Result<u64> {
    // Nothing to clean (and no lock file to create) in a missing workspace.
    let _lock = if workspace.is_dir() || !paths.is_empty() {
        Some(lock_workspace(workspace)?)
    } else {
        None
    };
    let items = cleanable(cat, workspace);
    for p in paths {
        if !items.iter().any(|it| same_path(&it.path, p)) {
            return Err(EngineError::new(
                Code::BadInput,
                format!(
                    "{} isn't something clean can remove (see clean --dry-run); nothing was removed",
                    p.display()
                ),
            )
            .into());
        }
    }
    let mut freed = 0;
    for it in items {
        let listed = paths.iter().any(|p| same_path(&it.path, p));
        if !(kinds.contains(&it.kind) || listed) || !it.path.exists() {
            continue;
        }
        if it.kind == "scratch" && it.path.file_name().is_some_and(|n| n == "flash") {
            if let Some(job) = it.path.parent() {
                crate::flash::rescue_old_backups(job)?;
            }
        }
        if fs::symlink_metadata(&it.path)?.is_dir() {
            fs::remove_dir_all(&it.path)?
        } else {
            fs::remove_file(&it.path)?
        }
        if it.kind == "downloads" {
            let marker = PathBuf::from(format!("{}.ok", it.path.display()));
            let _ = fs::remove_file(marker);
        }
        if it.kind == "scratch" {
            // A partial download goes with what's recorded about it.
            let side = PathBuf::from(format!("{}.json", it.path.display()));
            if it.path.extension().is_some_and(|x| x == "partial") {
                let _ = fs::remove_file(side);
            }
        }
        if it.kind == "image" {
            // The job can no longer be flashed.
            let _ = fs::remove_file(it.path.parent().unwrap().join("job.json"));
        }
        if it.kind == "backup" {
            if let Some(parent) = it.path.parent() {
                let _ = fs::remove_dir(parent); // only if now empty
            }
        }
        freed += it.bytes;
    }
    Ok(freed)
}

pub fn load_job(dir: &Path) -> Result<JobRecord> {
    let raw = fs::read(dir.join("job.json"))
        .with_context(|| format!("{} has no finished build", dir.display()))?;
    Ok(serde_json::from_slice(&raw)?)
}

fn describe(op: &Op) -> String {
    match op {
        Op::LpGrow { partition, .. } => format!("grow {partition}"),
        Op::LpWrite { partition, .. } => format!("write {partition}"),
        Op::LpDelta { partition, .. } => format!("patch {partition}"),
        Op::PacDelta { entry, .. } => format!("patch {entry}"),
        Op::PacSetInfo { .. } => "set product name".into(),
        Op::PacFixCrc => "update checksums".into(),
    }
}

/// Reader that hashes what passes through it.
struct HashingReader<R> {
    inner: R,
    hasher: Sha256,
    count: u64,
}
impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hasher.update(&buf[..n]);
        self.count += n as u64;
        Ok(n)
    }
}

#[allow(clippy::too_many_arguments)]
fn run_op(
    op: &Op,
    pac_path: &Path,
    system: Option<&Artifact>,
    system_paths: Option<&[crate::archive::Span]>,
    arts: &std::collections::HashMap<String, (Artifact, PathBuf)>,
    profile: &str,
    cat: &Catalog,
    rep: &Reporter,
) -> Result<()> {
    let art = |id: &str| -> Result<&(Artifact, PathBuf)> {
        arts.get(id)
            .with_context(|| format!("artifact {id} not fetched"))
    };
    match op {
        Op::LpGrow { entry, partition } => {
            let e = pac::Pac::open(pac_path)?.find(entry)?.clone();
            let mut f = OpenOptions::new().read(true).write(true).open(pac_path)?;
            let added = Super::new(&mut f, e.offset, e.size).grow(partition, None)?;
            rep.info(format!("{partition} grew by {}", util::human_bytes(added)));
        }
        Op::LpWrite {
            entry,
            partition,
            artifact,
        } => {
            let (a, paths) = if artifact == "@system" {
                (
                    system.context("no system image for this profile")?.clone(),
                    system_paths.context("system not fetched")?.to_vec(),
                )
            } else {
                let (a, p) = art(artifact)?.clone();
                (a, vec![crate::archive::Span::whole(&p)?])
            };
            let e = pac::Pac::open(pac_path)?.find(entry)?.clone();
            let mut f = OpenOptions::new().read(true).write(true).open(pac_path)?;
            let mut sup = Super::new(&mut f, e.offset, e.size);
            let file = crate::archive::MultiVolume::open_spans(&paths)?;
            let (size, want, mut reader): (u64, String, HashingReader<Box<dyn Read>>) =
                match &a.unpack {
                    Some(u) if u.format == "zstd" => {
                        let dec = zstd::stream::read::Decoder::new(file)?;
                        (
                            u.size,
                            u.sha256.clone(),
                            HashingReader {
                                inner: Box::new(dec),
                                hasher: Sha256::new(),
                                count: 0,
                            },
                        )
                    }
                    Some(u) => bail!("unknown compression {}", u.format),
                    None => (
                        a.size,
                        a.sha256.clone(),
                        HashingReader {
                            inner: Box::new(file),
                            hasher: Sha256::new(),
                            count: 0,
                        },
                    ),
                };
            sup.replace(partition, &mut reader, size, &mut |d, t| {
                rep.progress_for("Writing {partition}", partition, d, t)
            })?;
            let got = hex::encode(reader.hasher.finalize());
            if reader.count != size || got != want {
                return Err(EngineError::new(
                    Code::HashMismatch,
                    format!("{} unpacked to sha256 {got}, expected {want}", a.file),
                )
                .into());
            }
        }
        Op::LpDelta {
            entry,
            partition,
            artifact,
            in_sha256,
            out_sha256,
        } => {
            let (_, path) = art(artifact)?;
            let patch = fs::read(path)?;
            let e = pac::Pac::open(pac_path)?.find(entry)?.clone();
            let mut f = OpenOptions::new().read(true).write(true).open(pac_path)?;
            let mut sup = Super::new(&mut f, e.offset, e.size);
            let cur = sup.read_partition(partition)?;
            let new = delta::apply(&cur, &patch, Some(in_sha256), Some(out_sha256), partition)?;
            sup.replace(partition, &mut &new[..], new.len() as u64, &mut |_, _| {})?;
        }
        Op::PacDelta {
            entry,
            artifact,
            in_sha256,
            out_sha256,
        } => {
            let (_, path) = art(artifact)?;
            let patch = fs::read(path)?;
            let p = pac::Pac::open(pac_path)?;
            // Every entry with this ID or file name (e.g. both slots).
            let e = p
                .entries
                .iter()
                .find(|x| x.size > 0 && (&x.id == entry || &x.file == entry))
                .context("no such entry")?
                .clone();
            let cur = p.read_entry(&e)?;
            let new = delta::apply(&cur, &patch, Some(in_sha256), Some(out_sha256), entry)?;
            pac::replace(pac_path, &e, &new)?;
        }
        Op::PacSetInfo { version, alias } => {
            let expand = |s: &str| {
                s.replace("{profile}", profile)
                    .replace("{release}", &cat.root.release)
            };
            pac::set_info(pac_path, &expand(version), &expand(alias))?;
        }
        Op::PacFixCrc => {
            let crc = pac::fix_crc(pac_path, &mut |d, t| {
                rep.progress("Updating checksums", d, t)
            })?;
            rep.info(format!("Data CRC {crc:04x}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_bundled_build_next_to_the_engine_the_app_or_the_mac_app() {
        let dirs = build_dirs_near(Path::new(
            "/Applications/Thimsy Installer.app/Contents/MacOS/engine/thimsy-installer",
        ));
        assert!(
            dirs.contains(&PathBuf::from("/Applications/thimsy-build")),
            "{dirs:?}"
        );
        assert_eq!(
            dirs[0],
            PathBuf::from("/Applications/Thimsy Installer.app/Contents/MacOS/engine/thimsy-build")
        );
        // Windows/Linux layout: next to the engine or one level up only.
        let dirs = build_dirs_near(Path::new("/opt/thimsy/engine/thimsy-installer"));
        assert_eq!(
            dirs,
            [
                PathBuf::from("/opt/thimsy/engine/thimsy-build"),
                PathBuf::from("/opt/thimsy/engine/../thimsy-build"),
            ]
        );
    }
}
