// On Windows the app starts the engine with pipes; the GUI subsystem keeps a
// console window from flashing up. Run from a terminal, it attaches to it.
#![cfg_attr(windows, windows_subsystem = "windows")]

//! `thimsy-installer`: the engine the Thimsy Installer app runs in the
//! background (see installer/docs/engine-protocol.md). Also usable directly.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use thimsy_engine::{cli, job, pac, pack, Reporter};

#[derive(Parser)]
#[command(
    name = "thimsy-installer",
    version,
    about = "Builds and installs Thimsy OS on supported handhelds"
)]
struct Cli {
    /// JSON Lines output for the app.
    #[arg(long, global = true)]
    json: bool,
    /// Use a catalog directory instead of the built-in one.
    #[arg(long, global = true)]
    catalog: Option<PathBuf>,
    /// Language for messages (Thimsy codes: en ar da de el es fi fr he it ja ko nb nl pl pt ru sv th tr zh-cn zh-tw).
    #[arg(long, global = true, default_value = "en")]
    lang: String,
    /// Pretend to be another host (testing).
    #[arg(long, global = true, hide = true)]
    host: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List devices and profiles.
    Catalog,
    /// Show what a device + profile install contains.
    Resolve {
        #[arg(long)]
        device: String,
        #[arg(long)]
        profile: String,
        #[arg(long = "set", value_parser = parse_kv)]
        set: Vec<(String, String)>,
    },
    /// Download and build the firmware image.
    Build {
        #[arg(long)]
        device: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long = "import")]
        import: Vec<PathBuf>,
        #[arg(long = "set", value_parser = parse_kv)]
        set: Vec<(String, String)>,
        /// Delete each download once it's used.
        #[arg(long)]
        delete_downloads: bool,
    },
    /// Check free space for a build before starting it.
    Space {
        #[arg(long)]
        device: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        workspace: PathBuf,
        /// Files or folders the build will use instead of downloading.
        #[arg(long = "import")]
        import: Vec<PathBuf>,
        #[arg(long)]
        delete_downloads: bool,
    },
    /// Remove downloads, images, scratch files or backups from a workspace.
    Clean {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        downloads: bool,
        #[arg(long)]
        image: bool,
        #[arg(long)]
        scratch: bool,
        #[arg(long)]
        backup: bool,
        /// Remove only these items (paths `clean --dry-run` listed).
        #[arg(long)]
        path: Vec<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// List connected devices and flashing modes.
    Detect {
        #[arg(long)]
        watch: bool,
    },
    /// Install a finished build on the device.
    Flash {
        #[arg(long)]
        job: PathBuf,
        /// full (fresh install, erases data) or update (keeps data). Required:
        /// the choice is the user's, never a default.
        #[arg(long, value_parser = ["full", "update"])]
        mode: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Unlock the bootloader (devices with an automated unlock).
    Unlock {
        #[arg(long)]
        device: String,
    },
    /// USB driver (Windows) or USB access (Linux) status / install.
    Drivers {
        #[arg(long)]
        device: String,
        #[arg(long)]
        install: bool,
    },
    /// Maintainer tools.
    #[command(subcommand)]
    Dev(Dev),
}

#[derive(Subcommand)]
enum Dev {
    /// List a PAC's entries.
    PacList { pac: PathBuf },
    /// List the logical partitions of a PAC's super entry.
    LpList {
        pac: PathBuf,
        #[arg(long, default_value = "Super")]
        entry: String,
    },
    /// Make a device pack from official firmware and a reference build.
    Pack {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        reference: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        device: String,
        #[arg(long)]
        system_name: String,
    },
    /// Write a device pack archive (tar) from FILES; split into 1 GB parts
    /// if it's over GitHub's 2 GiB asset limit. Prints its catalog entry.
    Bundle {
        #[arg(long)]
        out: PathBuf,
        /// Split above this many bytes (default: just under 2 GiB).
        #[arg(long)]
        max_asset: Option<u64>,
        #[arg(long)]
        part_size: Option<u64>,
        files: Vec<PathBuf>,
    },
    /// List the files in a device pack archive (or its parts, in order).
    BundleList { files: Vec<PathBuf> },
    /// Download and unpack a device's official source firmware.
    Source {
        #[arg(long)]
        device: String,
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long = "import")]
        import: Vec<PathBuf>,
    },
    /// Validate a catalog directory.
    CheckCatalog,
    /// Write the English source strings (catalog/i18n/en.yaml).
    I18nStrings {
        #[arg(long)]
        src: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| "expected KEY=VALUE".into())
}

fn out(json_mode: bool, v: serde_json::Value) {
    if json_mode {
        println!("{v}");
    } else {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }
}

#[cfg(windows)]
fn attach_console() {
    // SAFETY: plain Win32 call; failure (no parent console) is fine.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn main() {
    #[cfg(windows)]
    attach_console();
    let cli = Cli::parse();
    // Notice the app dying even while nothing is written to it.
    thimsy_engine::tools::watch_parent();
    let rep = if cli.json {
        Reporter::json()
    } else {
        Reporter::text()
    };
    let g = cli::Global {
        json: cli.json,
        catalog: cli.catalog,
        lang: cli.lang,
        host: cli.host,
    };
    let json_mode = g.json;
    let mut print = |v: serde_json::Value| out(json_mode, v);
    let cmd = match cli.cmd {
        Cmd::Dev(dev) => {
            if let Err(e) = dev_command(&g, dev, &rep) {
                rep.error(&e);
                std::process::exit(1);
            }
            return;
        }
        Cmd::Catalog => cli::Command::Catalog,
        Cmd::Resolve {
            device,
            profile,
            set,
        } => cli::Command::Resolve {
            device,
            profile,
            set,
        },
        Cmd::Build {
            device,
            profile,
            workspace,
            import,
            set,
            delete_downloads,
        } => cli::Command::Build {
            device,
            profile,
            workspace,
            import,
            set,
            delete_downloads,
        },
        Cmd::Space {
            device,
            profile,
            workspace,
            import,
            delete_downloads,
        } => cli::Command::Space {
            device,
            profile,
            workspace,
            import,
            delete_downloads,
        },
        Cmd::Clean {
            workspace,
            downloads,
            image,
            scratch,
            backup,
            path,
            dry_run,
        } => cli::Command::Clean {
            workspace,
            downloads,
            image,
            scratch,
            backup,
            paths: path,
            dry_run,
        },
        Cmd::Detect { watch } => cli::Command::Detect { watch },
        Cmd::Flash { job, mode, dry_run } => cli::Command::Flash { job, mode, dry_run },
        Cmd::Unlock { device } => cli::Command::Unlock { device },
        Cmd::Drivers { device, install } => cli::Command::Drivers { device, install },
    };
    let code = cli::run(&g, cmd, &rep, &mut print);
    std::process::exit(code);
}

fn dev_command(g: &cli::Global, dev: Dev, rep: &Reporter) -> Result<()> {
    let (cat, _) = cli::setup(g)?;
    match dev {
        Dev::PacList { pac: path } => {
            let p = pac::Pac::open(&path)?;
            println!("version: {}\nalias: {}", p.version(), p.alias());
            for e in &p.entries {
                println!("{:22} {:32} {:>12}", e.id, e.file, e.size);
            }
        }
        Dev::LpList { pac: path, entry } => {
            let p = pac::Pac::open(&path)?;
            let e = p.find(&entry)?;
            let mut f = std::fs::File::open(&path)?;
            for (name, exts) in
                thimsy_engine::lp::Super::new(&mut f, e.offset, e.size).partitions()?
            {
                let size: u64 = exts.iter().map(|x| x.len).sum();
                println!("{name:24} {size:>12}  extents={}", exts.len());
            }
        }
        Dev::Pack {
            source,
            reference,
            out: o,
            device,
            system_name,
        } => {
            let r = pack::make(&source, &reference, &o, &device, &system_name, rep)?;
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        Dev::Bundle {
            out: o,
            max_asset,
            part_size,
            files,
        } => {
            let refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();
            let e = thimsy_engine::bundle::make(
                &o,
                &refs,
                max_asset.unwrap_or(thimsy_engine::bundle::MAX_ASSET),
                part_size.unwrap_or(thimsy_engine::bundle::PART),
                &mut |d, t| rep.progress("Writing the device pack", d, t),
            )?;
            println!("{}", serde_json::to_string_pretty(&e)?);
        }
        Dev::BundleList { files } => {
            let spans = files
                .iter()
                .map(|p| thimsy_engine::archive::Span::whole(p))
                .collect::<Result<Vec<_>>>()?;
            let b = thimsy_engine::bundle::Bundle::index(&spans)?;
            for (name, m) in &b.members {
                println!("{:>12}  {name}", m.size);
            }
        }
        Dev::I18nStrings { src, out: path } => {
            let strings = thimsy_engine::i18n::source_strings(&cat, &src)?;
            let mut body = String::from(
                "# Source strings (English): thimsy-installer dev i18n-strings. Don't edit.\n",
            );
            for s in &strings {
                let q = serde_json::to_string(s)?;
                body.push_str(&format!("{q}: {q}\n"));
            }
            std::fs::write(&path, body)?;
            println!("{} strings -> {}", strings.len(), path.display());
        }
        Dev::Source {
            device,
            workspace,
            import,
        } => {
            let p = job::prepare_source(&cat, &device, &workspace, &import, rep)?;
            println!("{}", p.display());
        }
        Dev::CheckCatalog => {
            println!(
                "catalog {} ok: {} devices, {} profiles",
                cat.root.version,
                cat.devices.len(),
                cat.profiles.len()
            );
        }
    }
    Ok(())
}
