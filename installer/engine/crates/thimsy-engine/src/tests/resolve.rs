use crate::catalog::Catalog;
use crate::host::Host;
use crate::i18n;
use crate::resolve;

const ROOT: &str = "version: t\nrelease: R\nsystems: { heavy: sys }\nartifacts: [{ id: sys, file: s, size: 1, sha256: \"0000000000000000000000000000000000000000000000000000000000000000\" }]\n";
const PROFILE: &str = r#"id: heavy
name: Heavy
summary: s
emulators:
  - { id: a, name: A }
  - { id: ps2, name: PS2, requires: { android_api: 33, armv82: true } }
  - { id: v, name: V, requires: { vulkan: true } }
  - { id: big, name: Big, requires: { min_ram_gb: 16 } }
settings: { nano.theme: esde, shader.global: "off", fixed: profile }
editable: ["nano.*", "shader.global", "fixed"]
"#;

fn dev(api: u32, v82: bool, vulkan: bool, ram: u32) -> String {
    format!(
        r#"id: d
name: Dev
maker: M
soc: S
display: "1x1"
status: candidate
hardware: {{ android_api: {api}, armv82: {v82}, vulkan: {vulkan}, ram_gb: {ram}, gpu: g }}
support: {{ build: candidate, boot: candidate, install: {{ windows: guided, linux: none }} }}
settings: {{ fixed: device }}
guides: {{ unlock: {{ route: guided, steps: [{{ title: "Unlock it", body: "Steps" }}] }} }}
"#
    )
}

fn cat(d: &str) -> Catalog {
    Catalog::from_files(vec![
        ("catalog.yaml".into(), ROOT.into()),
        ("profiles/heavy.yaml".into(), PROFILE.into()),
        ("devices/d.yaml".into(), d.into()),
    ])
    .unwrap()
}

#[test]
fn hardware_requirements_exclude_items_with_reasons() {
    let c = cat(&dev(30, false, false, 4));
    let p = resolve::resolve(&c, "d", "heavy", &[], Host::Windows).unwrap();
    let ex: Vec<(&str, &str)> = p
        .excluded
        .iter()
        .map(|e| (e.id.as_str(), e.reason.as_str()))
        .collect();
    assert_eq!(
        ex,
        [
            ("ps2", "needs Android API 33"),
            ("v", "needs Vulkan"),
            ("big", "needs 16 GB RAM")
        ]
    );
    assert!(p.emulators.iter().find(|e| e.id == "a").unwrap().included);
    let c = cat(&dev(34, false, true, 16));
    let p = resolve::resolve(&c, "d", "heavy", &[], Host::Windows).unwrap();
    assert_eq!(p.excluded.len(), 1);
    assert_eq!(p.excluded[0].reason, "needs an ARMv8.2 CPU");
}

#[test]
fn settings_precedence_profile_device_user() {
    let c = cat(&dev(34, true, true, 8));
    let p = resolve::resolve(
        &c,
        "d",
        "heavy",
        &[
            ("nano.theme".into(), "minima".into()),
            ("fixed".into(), "user".into()),
        ],
        Host::Windows,
    )
    .unwrap();
    let get = |k: &str| p.settings.iter().find(|s| s.key == k).unwrap().clone();
    assert_eq!(get("nano.theme").value, "minima");
    assert_eq!(get("nano.theme").source, "user");
    assert_eq!(get("shader.global").source, "profile");
    // Device value wins over both profile and user, with an explanation.
    let f = get("fixed");
    assert_eq!((f.value.as_str(), f.source.as_str()), ("device", "device"));
    assert!(f.reason.unwrap().contains("not applied"));
    // Non-editable keys are refused.
    assert!(resolve::resolve(
        &c,
        "d",
        "heavy",
        &[("secret".into(), "1".into())],
        Host::Windows
    )
    .is_err());
}

#[test]
fn availability_host_and_journeys() {
    let c = cat(&dev(34, true, true, 8));
    let p = resolve::resolve(&c, "d", "heavy", &[], Host::Linux).unwrap();
    assert!(!p.available, "no source/build steps → not buildable");
    assert!(p.unavailable_reason.unwrap().contains("isn't supported"));
    assert!(p.host_message.unwrap().contains("Windows PC"));
    assert!(resolve::require_host(c.device("d").unwrap(), Host::Linux).is_err());
    assert!(resolve::require_host(c.device("d").unwrap(), Host::Windows).is_ok());
    assert!(p.journeys.first_time.unlock);
    assert!(!p.journeys.returning.unlock);
    assert_eq!(
        p.journeys.returning.modes,
        ["full"],
        "no update list → reinstall only"
    );
    assert!(resolve::resolve(&c, "d", "nope", &[], Host::Linux).is_err());
}

#[test]
fn guide_text_is_translated() {
    let mut files = vec![
        ("catalog.yaml".to_string(), ROOT.to_string()),
        ("profiles/heavy.yaml".to_string(), PROFILE.to_string()),
        ("devices/d.yaml".to_string(), dev(34, true, true, 8)),
        (
            "i18n/fr.yaml".to_string(),
            "\"Unlock it\": \"Déverrouillez-le\"\n\"needs Vulkan\": \"nécessite Vulkan\"\n"
                .to_string(),
        ),
    ];
    files.sort();
    let c = Catalog::from_files(files).unwrap();
    i18n::set_language("fr", &c.files).unwrap();
    let p = resolve::resolve(&c, "d", "heavy", &[], Host::Windows).unwrap();
    i18n::set_language("en", &c.files).unwrap();
    assert_eq!(p.guides.unlock.steps[0].title, "Déverrouillez-le");
    assert_eq!(
        p.guides.unlock.steps[0].body, "Steps",
        "untranslated text stays English"
    );
    assert!(i18n::set_language("xx", &c.files).is_err());
}
