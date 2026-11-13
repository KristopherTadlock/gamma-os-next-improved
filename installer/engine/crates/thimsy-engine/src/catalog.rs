//! The device and profile catalog: what can be installed, from which inputs,
//! with which build steps and flashing method. Data only; no code.

use crate::host::Host;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/bundled_catalog.rs"));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    /// File name in the cache and on release pages.
    pub file: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub urls: Vec<String>,
    /// Set when the file is compressed and must be unpacked before use.
    #[serde(default)]
    pub unpack: Option<Unpack>,
    /// Shown when the file can't be downloaded automatically.
    #[serde(default)]
    pub note: Option<String>,
    /// Published in pieces (release assets are limited to 2 GiB): the
    /// artifact is the concatenation of these, and `size`/`sha256` describe
    /// the whole.
    #[serde(default)]
    pub parts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unpack {
    pub format: String, // "zstd"
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRoot {
    pub version: String,
    pub release: String,
    /// Profile id → shared (device-independent) system artifact id.
    #[serde(default)]
    pub systems: BTreeMap<String, String>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    #[serde(default)]
    pub android_api: Option<u32>,
    #[serde(default)]
    pub armv82: Option<bool>,
    #[serde(default)]
    pub vulkan: Option<bool>,
    #[serde(default)]
    pub min_ram_gb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub systems: Option<String>,
    #[serde(default)]
    pub requires: Requires,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Core {
    pub id: String,
    pub system: String,
    #[serde(default)]
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub emulators: Vec<ProfileItem>,
    #[serde(default)]
    pub apps: Vec<ProfileItem>,
    #[serde(default)]
    pub cores: Vec<Core>,
    /// Effective-setting key → value (Nano and emulator defaults).
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
    /// Setting keys the user may override (`*` suffix = prefix match).
    #[serde(default)]
    pub editable: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallSupport {
    Automated,
    Guided,
    Untested,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub build: String,
    pub boot: String,
    pub install: BTreeMap<Host, InstallSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbId {
    pub vid: u16,
    pub pid: u16,
    pub mode: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Identify {
    #[serde(default)]
    pub usb: Vec<UsbId>,
    #[serde(default)]
    pub adb_model: Option<String>,
    #[serde(default)]
    pub fastboot_product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub name: String,
    /// "archive7z" (volumes extracted with `password`) or "file".
    pub kind: String,
    #[serde(default)]
    pub password: Option<String>,
    pub parts: Vec<Artifact>,
    /// File inside the archive.
    pub contains: String,
    pub container: Container,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Container {
    pub format: String, // "pac"
    pub size: u64,
    pub sha256: String,
}

/// One build step. Typed; recipes can only combine these.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// Grow a logical partition of the `super` entry into free space.
    LpGrow { entry: String, partition: String },
    /// Write the profile's system image (or another artifact) to a logical partition.
    LpWrite {
        entry: String,
        partition: String,
        artifact: String,
    },
    /// Patch a logical partition with a delta artifact.
    LpDelta {
        entry: String,
        partition: String,
        artifact: String,
        in_sha256: String,
        out_sha256: String,
    },
    /// Patch a PAC entry with a delta artifact.
    PacDelta {
        entry: String,
        artifact: String,
        in_sha256: String,
        out_sha256: String,
    },
    /// Set the product strings flash tools show ({profile}, {release} expand).
    PacSetInfo { version: String, alias: String },
    /// Recompute the PAC data CRC.
    PacFixCrc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flash {
    /// "spd" (bundled spd_dump), "fastboot", "qdl", "spflash".
    pub method: String,
    /// PAC entries holding FDL1 and FDL2 (spd).
    #[serde(default)]
    pub fdl: Vec<String>,
    /// Entries written in update mode (keeps data).
    #[serde(default)]
    pub update: Vec<String>,
    /// Device partitions erased in full mode (fresh install).
    #[serde(default)]
    pub erase_full: Vec<String>,
    /// Extra entries never written, on top of `backup="1"` ones.
    #[serde(default)]
    pub never_write: Vec<String>,
    /// Seconds the tool waits for the device.
    #[serde(default = "default_wait")]
    pub wait_seconds: u32,
}
fn default_wait() -> u32 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub route: String, // automated | guided | none
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guides {
    #[serde(default)]
    pub backup: Vec<Step>,
    pub unlock: Unlock,
    #[serde(default)]
    pub download_mode: Vec<Step>,
    #[serde(default)]
    pub first_boot: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hardware {
    pub android_api: u32,
    pub armv82: bool,
    pub vulkan: bool,
    pub ram_gb: u32,
    pub gpu: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub maker: String,
    pub soc: String,
    pub display: String,
    pub status: String, // candidate | testing | supported
    pub hardware: Hardware,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub support: Support,
    /// Shown when this computer can't install the device.
    #[serde(default)]
    pub host_message: Option<String>,
    #[serde(default)]
    pub identify: Identify,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    /// Device-specific setting overrides (beat the profile).
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    /// Profile → the device pack as one archive (a tar of the system image,
    /// the patches and pack.json; see `bundle`). When present the build takes
    /// the system image and the artifacts from it instead of downloading
    /// them one by one.
    #[serde(default)]
    pub bundles: BTreeMap<String, Artifact>,
    #[serde(default)]
    pub build: Vec<Op>,
    /// Profile → expected output sha256, when known.
    #[serde(default)]
    pub expect: BTreeMap<String, String>,
    #[serde(default)]
    pub flash: Option<Flash>,
    /// What an already-unlocked device must be running for an update
    /// (keeps data) to be safe.
    #[serde(default)]
    pub update_requires: Option<String>,
    pub guides: Guides,
}

impl Device {
    pub fn install_on(&self, host: Host) -> InstallSupport {
        self.support
            .install
            .get(&host)
            .cloned()
            .unwrap_or(InstallSupport::None)
    }
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub root: CatalogRoot,
    pub profiles: Vec<Profile>,
    pub devices: Vec<Device>,
    /// Raw files, for translation tables (`i18n/<code>.yaml`).
    pub files: Vec<(String, String)>,
}

impl Catalog {
    /// The catalog built into this engine.
    pub fn bundled() -> Result<Catalog> {
        Self::from_files(
            BUNDLED
                .iter()
                .map(|(n, t)| (n.to_string(), t.to_string()))
                .collect(),
        )
    }

    /// A catalog directory (same layout as installer/catalog).
    pub fn from_dir(dir: &Path) -> Result<Catalog> {
        let mut files = Vec::new();
        fn walk(d: &Path, root: &Path, out: &mut Vec<(String, String)>) -> Result<()> {
            for e in std::fs::read_dir(d)? {
                let p = e?.path();
                if p.is_dir() {
                    walk(&p, root, out)?;
                } else if p.extension().is_some_and(|x| x == "yaml") {
                    let rel = p.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
                    out.push((rel, std::fs::read_to_string(&p)?));
                }
            }
            Ok(())
        }
        walk(dir, dir, &mut files)?;
        Self::from_files(files)
    }

    pub fn from_files(mut files: Vec<(String, String)>) -> Result<Catalog> {
        files.sort();
        let mut root = None;
        let mut profiles = Vec::new();
        let mut devices = Vec::new();
        for (name, text) in &files {
            if name == "catalog.yaml" {
                root = Some(
                    serde_yaml::from_str::<CatalogRoot>(text)
                        .with_context(|| format!("in {name}"))?,
                );
            } else if name.starts_with("profiles/") {
                profiles.push(
                    serde_yaml::from_str::<Profile>(text).with_context(|| format!("in {name}"))?,
                );
            } else if name.starts_with("devices/") {
                devices.push(
                    serde_yaml::from_str::<Device>(text).with_context(|| format!("in {name}"))?,
                );
            }
        }
        let root = root.context("catalog.yaml is missing")?;
        profiles.sort_by_key(|p| p.order);
        let cat = Catalog {
            root,
            profiles,
            devices,
            files: files.clone(),
        };
        cat.validate()?;
        Ok(cat)
    }

    pub fn device(&self, id: &str) -> Result<&Device> {
        self.devices.iter().find(|d| d.id == id).with_context(|| {
            let ids: Vec<_> = self.devices.iter().map(|d| d.id.as_str()).collect();
            format!("unknown device {id:?}; known: {}", ids.join(", "))
        })
    }

    pub fn profile(&self, id: &str) -> Result<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.id == id)
            .with_context(|| format!("unknown profile {id:?}"))
    }

    /// The shared artifact list plus a device's own.
    pub fn artifact(&self, device: &Device, id: &str) -> Result<Artifact> {
        self.root
            .artifacts
            .iter()
            .chain(device.artifacts.iter())
            .find(|a| a.id == id)
            .cloned()
            .with_context(|| format!("unknown artifact {id:?}"))
    }

    /// The system artifact for a profile, if published.
    pub fn system_for(&self, profile: &str) -> Option<&Artifact> {
        let id = self.root.systems.get(profile)?;
        self.root.artifacts.iter().find(|a| &a.id == id)
    }

    fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for d in &self.devices {
            if !seen.insert(&d.id) {
                bail!("duplicate device {}", d.id);
            }
            for op in &d.build {
                let art = match op {
                    Op::LpWrite { artifact, .. }
                    | Op::LpDelta { artifact, .. }
                    | Op::PacDelta { artifact, .. } => Some(artifact),
                    _ => None,
                };
                if let Some(a) = art {
                    if a != "@system" && self.artifact(d, a).is_err() {
                        bail!("device {}: build step names unknown artifact {a}", d.id);
                    }
                }
            }
            if !d.build.is_empty() && d.source.is_none() {
                bail!("device {} has build steps but no source firmware", d.id);
            }
            // Every profile a buildable device can build has a reference
            // hash, so no build is ever published unchecked.
            if !d.build.is_empty() {
                for p in self.root.systems.keys().chain(d.bundles.keys()) {
                    if !d.expect.contains_key(p) {
                        bail!(
                            "device {} can build the {p} profile but has no expect hash for it",
                            d.id
                        );
                    }
                }
            }
            for (p, b) in &d.bundles {
                self.profile(p)
                    .with_context(|| format!("device {}: bundle for unknown profile {p}", d.id))?;
                if b.unpack.is_some() {
                    bail!("device {}: bundle {} can't be compressed", d.id, b.file);
                }
            }
        }
        for (p, a) in &self.root.systems {
            self.profile(p)
                .with_context(|| format!("systems: unknown profile {p}"))?;
            if !self.root.artifacts.iter().any(|x| &x.id == a) {
                bail!("systems: profile {p} names unknown artifact {a}");
            }
        }
        for a in self
            .root
            .artifacts
            .iter()
            .chain(self.devices.iter().flat_map(|d| d.artifacts.iter()))
        {
            if a.sha256.len() != 64 || !a.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                bail!("artifact {} has a malformed sha256", a.id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_loads() {
        let c = Catalog::bundled().unwrap();
        assert!(c.device("anbernic-rg-cube").is_ok());
        assert_eq!(
            c.profiles.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["light", "medium", "heavy"]
        );
    }
}
