//! The commands the app runs (docs/engine-protocol.md), exactly as the
//! `thimsy-installer` binary runs them: output lines go to `out`, events to
//! the reporter. Kept in the library so tests (and the protocol samples in
//! docs/protocol-samples) see the same lines the app does.

use crate::catalog::{Catalog, InstallSupport};
use crate::host::Host;
use crate::i18n::tr;
use crate::{detect, drivers, flash, job, resolve, Reporter};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Options every command takes.
#[derive(Debug, Clone, Default)]
pub struct Global {
    /// JSON Lines output for the app.
    pub json: bool,
    /// A catalog directory instead of the built-in one.
    pub catalog: Option<PathBuf>,
    /// Language code for messages.
    pub lang: String,
    /// Pretend to be another host (testing).
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Command {
    Catalog,
    Resolve {
        device: String,
        profile: String,
        set: Vec<(String, String)>,
    },
    Build {
        device: String,
        profile: String,
        workspace: PathBuf,
        import: Vec<PathBuf>,
        set: Vec<(String, String)>,
        delete_downloads: bool,
    },
    Space {
        device: String,
        profile: String,
        workspace: PathBuf,
        import: Vec<PathBuf>,
        delete_downloads: bool,
    },
    Clean {
        workspace: PathBuf,
        downloads: bool,
        image: bool,
        scratch: bool,
        backup: bool,
        paths: Vec<PathBuf>,
        dry_run: bool,
    },
    Detect {
        watch: bool,
    },
    Prepare {
        job: PathBuf,
        mode: String,
    },
    Flash {
        job: PathBuf,
        mode: String,
        dry_run: bool,
    },
    Unlock {
        device: String,
    },
    Drivers {
        device: String,
        install: bool,
    },
}

impl Command {
    /// Commands whose last line is always a `result` (`ok:false` after an
    /// error).
    pub fn streaming(&self) -> bool {
        matches!(
            self,
            Command::Build { .. }
                | Command::Prepare { .. }
                | Command::Flash { .. }
                | Command::Unlock { .. }
                | Command::Drivers { .. }
                | Command::Clean { .. }
        )
    }
}

/// Loads the catalog, activates the language and picks the host.
pub fn setup(g: &Global) -> Result<(Catalog, Host)> {
    let cat = match &g.catalog {
        Some(d) => Catalog::from_dir(d)?,
        None => Catalog::bundled()?,
    };
    let lang = if g.lang.is_empty() { "en" } else { &g.lang };
    crate::i18n::set_language(lang, &cat.files)?;
    let host = match &g.host {
        Some(h) => Host::parse(h).context("unknown host")?,
        None => Host::current(),
    };
    Ok((cat, host))
}

/// Runs one command. On failure the error event is emitted (and, for
/// streaming commands in JSON mode, a final `result` with `ok:false`);
/// returns the process exit code.
pub fn run(g: &Global, cmd: Command, rep: &Reporter, out: &mut dyn FnMut(Value)) -> i32 {
    let streaming = cmd.streaming();
    let drivers_for = match &cmd {
        Command::Drivers { device, .. } => Some(device.clone()),
        _ => None,
    };
    match dispatch(g, cmd, rep, out) {
        Ok(()) => 0,
        Err(e) => {
            rep.error(&e);
            if g.json && streaming {
                let mut result = json!({"type": "result", "ok": false});
                // A failed driver install still says where things stand.
                if let Some(device) = drivers_for {
                    if let Ok((cat, host)) = setup(g) {
                        if let Ok(d) = cat.device(&device) {
                            let st = drivers::status(d, host);
                            result["status"] = json!(st.status);
                            result["detail"] = json!(st.detail);
                        }
                    }
                }
                out(result);
            }
            1
        }
    }
}

fn dispatch(g: &Global, cmd: Command, rep: &Reporter, out: &mut dyn FnMut(Value)) -> Result<()> {
    let (cat, host) = setup(g)?;
    match cmd {
        Command::Catalog => {
            let devices: Vec<_> = cat
                .devices
                .iter()
                .map(|d| {
                    json!({
                        "id": d.id, "name": d.name, "maker": d.maker, "soc": d.soc, "display": d.display,
                        "status": d.status, "support": d.support, "blocks": d.blocks.iter().map(|n| tr(n)).collect::<Vec<_>>(),
                        "host_message": (d.install_on(host) == InstallSupport::None).then(|| resolve::host_text(d, host)),
                        "notes": d.notes.iter().map(|n| tr(n)).collect::<Vec<_>>(),
                        "buildable": d.source.is_some() && !d.build.is_empty(),
                    })
                })
                .collect();
            let profiles: Vec<_> = cat
                .profiles
                .iter()
                .map(|p| json!({"id": p.id, "name": tr(&p.name), "summary": tr(&p.summary), "available": cat.system_for(&p.id).is_some()}))
                .collect();
            out(
                json!({"type": "catalog", "version": cat.root.version, "release": cat.root.release,
                "engine": crate::VERSION, "host": host, "devices": devices, "profiles": profiles}),
            );
        }
        Command::Resolve {
            device,
            profile,
            set,
        } => {
            let plan = resolve::resolve(&cat, &device, &profile, &set, host)?;
            out(serde_json::to_value(plan)?);
        }
        Command::Build {
            device,
            profile,
            workspace,
            import,
            set,
            delete_downloads,
        } => {
            let opts = job::BuildOptions {
                device,
                profile,
                workspace,
                imports: import,
                overrides: set,
                host,
                delete_downloads,
            };
            let rec = job::build(&cat, &opts, rep)?;
            let dir = job::job_dir(&opts.workspace, &rec.device, &rec.profile);
            out(
                json!({"type": "result", "ok": true, "job": dir, "image": rec.image, "bytes": rec.bytes,
                "sha256": rec.sha256, "matches_reference": rec.matches_reference}),
            );
        }
        Command::Space {
            device,
            profile,
            workspace,
            import,
            delete_downloads,
        } => {
            let sc = job::check_space(
                &cat,
                &device,
                &profile,
                &workspace,
                &import,
                delete_downloads,
            )?;
            out(serde_json::to_value(sc)?);
        }
        Command::Clean {
            workspace,
            downloads,
            image,
            scratch,
            backup,
            paths,
            dry_run,
        } => {
            let mut kinds = Vec::new();
            for (on, k) in [
                (downloads, "downloads"),
                (image, "image"),
                (scratch, "scratch"),
                (backup, "backup"),
            ] {
                if on {
                    kinds.push(k);
                }
            }
            // What was there before cleaning.
            let items = job::cleanable(&cat, &workspace);
            let freed = if dry_run {
                0
            } else {
                job::clean(&cat, &workspace, &kinds, &paths)?
            };
            out(json!({"type": "result", "ok": true, "items": items, "freed": freed}));
        }
        Command::Detect { watch } => {
            let mut last = None;
            loop {
                let seen = detect::scan(&cat)?;
                if last.as_ref() != Some(&seen) {
                    out(json!({"type": "devices", "devices": seen}));
                    last = Some(seen);
                }
                if !watch {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
        }
        Command::Prepare { job, mode } => {
            let r = flash::prepare(&cat, &job, flash::Mode::parse(&mode)?, rep)?;
            out(json!({"type": "result", "ok": true, "files": r.files, "bytes": r.bytes}));
        }
        Command::Flash { job, mode, dry_run } => {
            let opts = flash::FlashOptions {
                job,
                mode: flash::Mode::parse(&mode)?,
                dry_run,
                host,
            };
            let r = flash::flash(&cat, &opts, rep)?;
            out(json!({"type": "result", "ok": true, "backup": r.backup, "commands": r.commands}));
        }
        Command::Unlock { device } => {
            flash::unlock(&cat, &device, rep)?;
            out(json!({"type": "result", "ok": true}));
        }
        Command::Drivers { device, install } => {
            let d = cat.device(&device)?;
            if install {
                drivers::install_for(d, host, rep)?;
            }
            let st = drivers::status(d, host);
            out(json!({"type": "result", "ok": true, "status": st.status, "detail": st.detail}));
        }
    }
    Ok(())
}
