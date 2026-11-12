//! The real RG Cube recipe against the bundled catalog (no firmware needed).
use crate::catalog::Catalog;
use crate::detect;
use crate::flash::{self, Mode};

#[test]
fn rg_cube_recipe_never_writes_device_unique_partitions() {
    let c = Catalog::bundled().unwrap();
    let d = c.device("anbernic-rg-cube").unwrap();
    let f = d.flash.as_ref().unwrap();
    for p in ["Persist", "calinv", "isedata", "UserData"] {
        assert!(
            f.never_write.iter().any(|x| x == p),
            "{p} must be in never_write"
        );
    }
    assert_eq!(
        f.update,
        ["Super", "VENDORBOOT", "VENDORBOOT_B", "BootLogo"]
    );
    assert_eq!(f.erase_full, ["userdata", "metadata"]);
    assert!(
        Mode::parse("full").is_ok() && Mode::parse("update").is_ok() && Mode::parse("x").is_err()
    );
}

#[test]
fn unlock_routes() {
    let c = Catalog::bundled().unwrap();
    let rep =
        crate::events::Reporter::new(std::sync::Arc::new(crate::events::MemorySink::default()));
    let err = flash::unlock(&c, "anbernic-rg-cube", &rep).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, crate::Code::UnlockGuided);
    flash::unlock(&c, "anbernic-rg557", &rep).unwrap(); // route none
}

#[test]
fn detect_names_generic_modes() {
    let c = Catalog::bundled().unwrap();
    assert_eq!(
        detect::classify(&c, 0x18d1, 0x4ee0, None).unwrap().mode,
        "fastboot"
    );
    assert_eq!(
        detect::classify(&c, 0x0e8d, 0x2000, None).unwrap().mode,
        "preloader"
    );
    let adb = detect::classify(&c, 0x1234, 0x0001, Some("RG Cube")).unwrap();
    assert_eq!(
        (adb.mode.as_str(), adb.device.as_deref()),
        ("adb", Some("anbernic-rg-cube"))
    );
}
