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
    pub free: u64,
    pub ok: bool,
    pub build_need: u64,
    pub flash_need: u64,
    pub cached: u64,
    pub downloads: u64,
}

pub fn check_space(
    cat: &Catalog,
    device: &str,
    profile: &str,
    workspace: &Path,
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
    // New bytes the build writes: missing downloads + the working image.
    let build_need = sp.build_peak.saturating_sub(cached);
    // Flashing later unpacks partitions next to the finished image (which
    // doesn't exist yet, so it counts in full); without delete-downloads
    // the downloads still missing stay in the cache too.
    let after_build_kept = if delete_downloads {
        0
    } else {
        sp.downloads - cached
    };
    let flash_need = sp.image + sp.flash_extra + after_build_kept;
    let need = build_need.max(flash_need);
    let probe = if workspace.exists() {
        workspace.to_path_buf()
    } else {
        workspace
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    };
    let free = fs4::available_space(&probe).unwrap_or(u64::MAX);
    Ok(SpaceCheck {
        kind: "space",
        need,
        free,
        ok: free >= need,
        build_need,
        flash_need,
        cached,
        downloads: sp.downloads,
    })
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
        &mut |d, t| rep.progress("Unpacking", d, t),
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
    let unpacked = cache.find_import(&src.contains, src.container.size);
    let sc = check_space(
        cat,
        &d.id,
        &opts.profile,
        &opts.workspace,
        opts.delete_downloads,
    )?;
    rep.info(format!(
        "Needs about {} free in {}; {} available",
        util::human_bytes(sc.need),
        opts.workspace.display(),
        util::human_bytes(sc.free)
    ));
    if !sc.ok {
        return Err(not_enough(&opts.workspace, sc.need, sc.free));
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
                    rep.progress("Copying firmware", d, n)
                })?;
            }
            "archive7z" => {
                archive::extract_one(
                    &parts,
                    src.password.as_deref(),
                    &src.contains,
                    &tmp,
                    &mut |d, t| rep.progress(&format!("Unpacking {}", src.contains), d, t),
                )?;
            }
            "file" => {
                let mut i = File::open(&parts[0])?;
                let mut o = File::create(&tmp)?;
                let n = i.metadata()?.len();
                util::copy_exact(&mut i, &mut o, n, &mut |d| {
                    rep.progress("Copying firmware", d, n)
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
    let sha = util::sha256_file(&working, |d, t| {
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
}

fn dir_size(p: &Path) -> u64 {
    if p.is_file() {
        return fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    }
    fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// What `clean` could remove from a workspace.
pub fn cleanable(workspace: &Path) -> Vec<CleanItem> {
    let mut items = Vec::new();
    let cache = workspace.join("cache");
    if let Ok(rd) = fs::read_dir(&cache) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let kind = if name.ends_with(".partial") || name.ends_with(".import") {
                "scratch"
            } else {
                "downloads"
            };
            if name.ends_with(".ok") {
                continue;
            }
            items.push(CleanItem {
                kind,
                bytes: dir_size(&p),
                path: p,
            });
        }
    }
    if let Ok(rd) = fs::read_dir(workspace.join("jobs")) {
        for job in rd.flatten() {
            let j = job.path();
            for (sub, kind) in [
                ("work", "scratch"),
                ("flash", "scratch"),
                ("out", "image"),
                ("backup", "backup"),
                // Backups an earlier engine version left in the scratch
                // folder: never scratch (`clean` moves them to backup/).
                ("flash/backup", "backup"),
            ] {
                let p = j.join(sub);
                if p.exists() {
                    let mut bytes = dir_size(&p);
                    if sub == "flash" {
                        bytes -= dir_size(&p.join("backup")).min(bytes);
                    }
                    if bytes > 0 {
                        items.push(CleanItem {
                            kind,
                            path: p,
                            bytes,
                        });
                    }
                }
            }
        }
    }
    items
}

/// Removes the chosen kinds; returns bytes freed. Backups are removed only
/// when `backup` is asked for: removing a job's scratch folder first moves
/// any backup left in it to the job's backup folder.
pub fn clean(workspace: &Path, kinds: &[&str]) -> Result<u64> {
    let mut freed = 0;
    for it in cleanable(workspace) {
        if !kinds.contains(&it.kind) || !it.path.exists() {
            continue;
        }
        if it.kind == "scratch" && it.path.file_name().is_some_and(|n| n == "flash") {
            if let Some(job) = it.path.parent() {
                crate::flash::rescue_old_backups(job)?;
            }
        }
        if it.path.is_dir() {
            fs::remove_dir_all(&it.path)?
        } else {
            fs::remove_file(&it.path)?
        }
        if it.kind == "downloads" {
            let marker = PathBuf::from(format!("{}.ok", it.path.display()));
            let _ = fs::remove_file(marker);
        }
        if it.kind == "image" {
            // The job can no longer be flashed.
            let _ = fs::remove_file(it.path.parent().unwrap().join("job.json"));
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
                rep.progress(&format!("Writing {partition}"), d, t)
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
