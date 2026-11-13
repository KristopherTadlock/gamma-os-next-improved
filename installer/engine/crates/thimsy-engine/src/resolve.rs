//! Device + profile + user overrides → one explicit plan (what ships, which
//! settings win and why, what gets downloaded and written).

use crate::catalog::{Catalog, Device, InstallSupport, Profile, ProfileItem, Requires, Step};
use crate::error::{Code, EngineError};
use crate::host::Host;
use crate::i18n::tr;
use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub systems: Option<String>,
    pub included: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Excluded {
    pub id: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoreOut {
    pub id: String,
    pub system: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub source: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnlockOut {
    pub route: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidesOut {
    pub backup: Vec<Step>,
    pub unlock: UnlockOut,
    pub download_mode: Vec<Step>,
    pub first_boot: Vec<Step>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Journey {
    pub title: String,
    pub body: String,
    pub unlock: bool,
    pub modes: Vec<String>,
    pub update_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Journeys {
    pub first_time: Journey,
    pub returning: Journey,
}

fn trs(steps: &[Step]) -> Vec<Step> {
    steps
        .iter()
        .map(|s| Step {
            title: tr(&s.title),
            body: tr(&s.body),
            link: s.link.clone(),
        })
        .collect()
}

fn journeys(d: &Device) -> Journeys {
    let needs_unlock = d.guides.unlock.route != "none";
    let can_update = d.flash.as_ref().is_some_and(|f| !f.update.is_empty());
    Journeys {
        first_time: Journey {
            title: tr("First time installing custom firmware"),
            body: if needs_unlock {
                tr("Your {device} still runs its original firmware. The installer walks you through unlocking it, then does a fresh install that erases the device.")
            } else {
                tr("Your {device} runs its original firmware. The installer does a fresh install that erases the device.")
            }
            .replace("{device}", &d.name),
            unlock: needs_unlock,
            modes: vec!["full".into()],
            update_note: None,
        },
        returning: Journey {
            title: tr("Already unlocked: install new firmware"),
            body: tr("Your {device} already runs custom firmware (it was unlocked before). Unlocking is skipped.").replace("{device}", &d.name),
            unlock: false,
            modes: if can_update { vec!["update".into(), "full".into()] } else { vec!["full".into()] },
            update_note: can_update.then(|| {
                tr(d.update_requires.as_deref().unwrap_or("Keeps your data. Only when the device already runs a compatible Thimsy or GammaOS build."))
            }),
        },
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub device: String,
    pub device_name: String,
    pub profile: String,
    pub profile_name: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub host: Host,
    pub install: InstallSupport,
    pub host_message: Option<String>,
    pub emulators: Vec<Item>,
    pub apps: Vec<Item>,
    pub cores: Vec<CoreOut>,
    pub excluded: Vec<Excluded>,
    pub settings: Vec<Setting>,
    pub download_bytes: u64,
    pub disk_bytes: u64,
    pub partitions: Vec<String>,
    pub wipes_data: bool,
    pub journeys: Journeys,
    pub guides: GuidesOut,
}

/// Why `req` isn't met by the device, if it isn't.
fn unmet(req: &Requires, d: &Device) -> Option<String> {
    let h = &d.hardware;
    if let Some(api) = req.android_api {
        if h.android_api < api {
            return Some(tr("needs Android API {n}").replace("{n}", &api.to_string()));
        }
    }
    if req.armv82 == Some(true) && !h.armv82 {
        return Some(tr("needs an ARMv8.2 CPU"));
    }
    if req.vulkan == Some(true) && !h.vulkan {
        return Some(tr("needs Vulkan"));
    }
    if let Some(r) = req.min_ram_gb {
        if h.ram_gb < r {
            return Some(tr("needs {n} GB RAM").replace("{n}", &r.to_string()));
        }
    }
    None
}

fn items(list: &[ProfileItem], d: &Device, excluded: &mut Vec<Excluded>) -> Vec<Item> {
    list.iter()
        .map(|it| {
            let why = unmet(&it.requires, d);
            if let Some(r) = &why {
                excluded.push(Excluded {
                    id: it.id.clone(),
                    name: it.name.clone(),
                    reason: r.clone(),
                });
            }
            Item {
                id: it.id.clone(),
                name: it.name.clone(),
                systems: it.systems.clone(),
                included: why.is_none(),
                reason: why,
            }
        })
        .collect()
}

fn editable(p: &Profile, key: &str) -> bool {
    p.editable.iter().any(|e| match e.strip_suffix('*') {
        Some(prefix) => key.starts_with(prefix),
        None => e == key,
    })
}

/// Disk use of a build + flash, in bytes.
#[derive(Debug, Clone, Serialize)]
pub struct Space {
    /// Everything that gets downloaded.
    pub downloads: u64,
    /// Peak during the build: downloads + the working image (the system
    /// image is streamed into it, never unpacked to disk), + 5% headroom.
    pub build_peak: u64,
    /// Extra space flashing needs (partitions unpacked from the image).
    pub flash_extra: u64,
    /// The finished image.
    pub image: u64,
}

pub fn space(cat: &Catalog, d: &Device, profile: &str) -> Space {
    let mut downloads = 0u64;
    let mut image = 0u64;
    if let Some(src) = &d.source {
        downloads += src.parts.iter().map(|a| a.size).sum::<u64>();
        image = src.container.size;
    }
    if let Some(b) = d.bundles.get(profile) {
        downloads += b.size;
    } else {
        if let Some(sys) = cat.system_for(profile) {
            downloads += sys.size;
        }
        downloads += d.artifacts.iter().map(|a| a.size).sum::<u64>();
    }
    let build_peak = downloads + image;
    Space {
        downloads,
        build_peak: build_peak + build_peak / 20,
        flash_extra: image + image / 20,
        image,
    }
}

/// Kept for the plan summary: (download bytes, peak disk bytes).
pub fn disk_estimate(cat: &Catalog, d: &Device, profile: &str) -> (u64, u64) {
    let s = space(cat, d, profile);
    (s.downloads, s.build_peak.max(s.image + s.flash_extra))
}

pub fn resolve(
    cat: &Catalog,
    device: &str,
    profile: &str,
    overrides: &[(String, String)],
    host: Host,
) -> Result<Plan> {
    let d = cat.device(device)?;
    let p = cat.profile(profile)?;
    let mut excluded = Vec::new();
    let emulators = items(&p.emulators, d, &mut excluded);
    let apps = items(&p.apps, d, &mut excluded);

    // Settings: profile, then device, then user (editable keys only).
    let mut eff: BTreeMap<String, Setting> = BTreeMap::new();
    for (k, v) in &p.settings {
        eff.insert(
            k.clone(),
            Setting {
                key: k.clone(),
                value: v.clone(),
                source: "profile".into(),
                reason: None,
            },
        );
    }
    for (k, v) in &d.settings {
        let reason = eff
            .get(k)
            .filter(|s| &s.value != v)
            .map(|s| format!("{} overrides the profile's {}", d.name, s.value));
        eff.insert(
            k.clone(),
            Setting {
                key: k.clone(),
                value: v.clone(),
                source: "device".into(),
                reason,
            },
        );
    }
    for (k, v) in overrides {
        if !editable(p, k) {
            return Err(EngineError::new(
                Code::BadInput,
                format!("setting {k} can't be changed in the {} profile", p.name),
            )
            .into());
        }
        if d.settings.contains_key(k) {
            // Device values are hardware constraints; keep them.
            if let Some(s) = eff.get_mut(k) {
                s.reason = Some(format!(
                    "{} needs {}; your choice {v} was not applied",
                    d.name, s.value
                ));
            }
            continue;
        }
        eff.insert(
            k.clone(),
            Setting {
                key: k.clone(),
                value: v.clone(),
                source: "user".into(),
                reason: None,
            },
        );
    }

    let system = cat.system_for(profile);
    let install = d.install_on(host);
    let (available, unavailable_reason) = if d.source.is_none() || d.build.is_empty() {
        (
            false,
            Some(
                tr("{device} isn't supported by this version of the installer yet.")
                    .replace("{device}", &d.name),
            ),
        )
    } else if system.is_none() {
        (
            false,
            Some(
                tr("The {profile} profile isn't published yet. Choose a profile marked available.")
                    .replace("{profile}", &tr(&p.name)),
            ),
        )
    } else {
        (true, None)
    };
    let host_message = (install == InstallSupport::None).then(|| host_text(d, host));

    let (download_bytes, disk_bytes) = disk_estimate(cat, d, profile);
    let partitions = d
        .flash
        .as_ref()
        .map(|f| f.update.clone())
        .unwrap_or_default();
    Ok(Plan {
        kind: "plan",
        device: d.id.clone(),
        device_name: d.name.clone(),
        profile: p.id.clone(),
        profile_name: tr(&p.name),
        available,
        unavailable_reason,
        host,
        install,
        host_message,
        emulators,
        apps,
        cores: p
            .cores
            .iter()
            .map(|c| CoreOut {
                id: c.id.clone(),
                system: c.system.clone(),
                default: c.default,
            })
            .collect(),
        excluded,
        settings: eff.into_values().collect(),
        download_bytes,
        disk_bytes,
        partitions,
        wipes_data: true,
        journeys: journeys(d),
        guides: GuidesOut {
            backup: trs(&d.guides.backup),
            unlock: UnlockOut {
                route: d.guides.unlock.route.clone(),
                steps: trs(&d.guides.unlock.steps),
            },
            download_mode: trs(&d.guides.download_mode),
            first_boot: trs(&d.guides.first_boot),
        },
    })
}

/// The "use Windows" message for a device this computer can't install.
pub fn host_text(d: &Device, host: Host) -> String {
    let t = d
        .host_message
        .as_deref()
        .unwrap_or("{device} can't be installed from {host}. Run Thimsy Installer on a Windows PC to install it.");
    tr(t)
        .replace("{device}", &d.name)
        .replace("{host}", host.display())
}

/// Fails with `unsupported_host` when this computer can't install the device.
pub fn require_host(d: &Device, host: Host) -> Result<()> {
    if d.install_on(host) == InstallSupport::None {
        return Err(EngineError::new(Code::UnsupportedHost, host_text(d, host))
            .hint("Use a Windows PC")
            .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rg_cube_plans() {
        let cat = Catalog::bundled().unwrap();
        let plan = resolve(&cat, "anbernic-rg-cube", "heavy", &[], Host::Linux).unwrap();
        assert!(plan.available, "{:?}", plan.unavailable_reason);
        assert!(plan.excluded.is_empty());
        assert!(plan.host_message.is_none());
        assert!(plan.journeys.first_time.unlock);
        assert!(!plan.journeys.returning.unlock);
        assert_eq!(plan.journeys.returning.modes, ["update", "full"]);
        let light = resolve(&cat, "anbernic-rg-cube", "light", &[], Host::Linux).unwrap();
        assert!(!light.available);
        // A user override of a device-fixed setting is reported, not applied.
        let p = resolve(
            &cat,
            "anbernic-rg-cube",
            "heavy",
            &[("nano.theme".into(), "minima".into())],
            Host::Linux,
        )
        .unwrap();
        let theme = p.settings.iter().find(|s| s.key == "nano.theme").unwrap();
        assert_eq!(theme.value, "minima");
        assert_eq!(theme.source, "user");
        assert!(resolve(
            &cat,
            "anbernic-rg-cube",
            "heavy",
            &[("not.editable".into(), "x".into())],
            Host::Linux
        )
        .is_err());
    }

    #[test]
    fn needs_windows_message() {
        let cat = Catalog::bundled().unwrap();
        let d = cat.device("anbernic-rg557").unwrap();
        let err = require_host(d, Host::Macos).unwrap_err();
        assert_eq!(crate::error::code_of(&err).0, Code::UnsupportedHost);
        assert!(err.to_string().contains("Windows"));
        let plan = resolve(&cat, "anbernic-rg557", "heavy", &[], Host::Macos).unwrap();
        assert!(plan.host_message.unwrap().contains("Windows"));
        // Android hosts can't flash anything in this version.
        let cube = cat.device("anbernic-rg-cube").unwrap();
        assert!(require_host(cube, Host::Android).is_err());
    }
}
