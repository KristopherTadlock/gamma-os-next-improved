"""The mods that need no downloads, on both bases, each run twice: the second run must change nothing,
or refuse loudly and leave the tree as it was."""
import os
import re
import tarfile

from tests import util
from tests.e2e.modrun import ModCase
from tests.fixtures import systree

OFFLINE = ("05", "12", "25", "27", "37", "45", "55", "58")


class OfflineMods(ModCase):
    def build(self, nano):
        top = self.path("nano" if nano else "v12")
        util.rmtree(top)
        self.system = systree.build(os.path.join(top, "system"), nano)
        return self.system

    def setup_sh(self):
        return util.slurp(f"{self.system}/bin/setup.sh", text=True)

    # --- what each mod does

    def test_05_version(self):
        for nano in (True, False):
            self.build(nano)
            self.assertOk(self.run_mod("05", env={"THIMSY_BUILD_VERSION": "Thimsy_OS_v9.9", "THIMSY_VERSION": "9.9"}))
            prop = util.slurp(f"{self.system}/build.prop", text=True)
            self.assertIn("\nro.lineage.version=Thimsy_OS_v9.9\nro.lineage.display.version=Thimsy_OS_v9.9\n", prop)
            self.assertIn("ro.build.user=gammaos\nro.build.host=localhost\n", prop)
            self.assertTrue(prop.endswith("ro.thimsy.version=9.9\n"))
            self.assertIn("ro.product.system.model=RG Cube", prop)

    def test_12_overlays(self):
        self.build(True)
        proc = self.assertOk(self.run_mod("12"))
        self.assertIn("removed 4 style overlays; 2 left", proc.stdout)
        self.assertEqual(sorted(os.listdir(f"{self.system}/product/overlay")),
                         ["NavigationBarModeGestural", "ThemedIconsOverlay"])
        util.rmtree(f"{self.system}/product/overlay")
        self.assertIn("no product overlays; skipping", self.assertOk(self.run_mod("12")).stdout)

    def test_25_usb_mtp(self):
        for nano, anchor in ((True, "mkdir -p /data/setupcompleted"),
                             (False, 'echo "Granting permissions to applications."')):
            self.build(nano)
            before = self.setup_sh()
            self.assertOk(self.run_mod("25"))
            after = self.setup_sh()
            steps = ('if [ "${FRESH_SETUP:-1}" = 1 ]; then\n'
                     '    nsenter -t 1 -m -- svc usb setScreenUnlockedFunctions mtp\nfi\n\n')
            marker = re.search(r"^# thimsy: 25-usb-mtp\.sh [0-9a-f]{16}$", after, re.M)
            self.assertTrue(marker, after)
            self.assertEqual(after, before.replace(anchor + "\n", marker.group(0) + "\n" + steps + anchor + "\n"))

    def test_27_controller_layout(self):
        for nano in (True, False):
            self.build(nano)
            self.assertOk(self.run_mod("27"))
            kl = f"{self.system}/product/usr/keylayout/Vendor_484b_Product_1101.kl"
            self.assertEqual(util.slurp(kl), util.slurp(f"{util.MODS}/keylayout/Vendor_484b_Product_1101.kl"))
            self.assertEqual(os.stat(kl).st_mode & 0o777, 0o644)
            self.assertIn(f"setfattr -n security.selinux -v u:object_r:system_file:s0 {kl}", self.shim_calls("setfattr"))
            pack = f"{self.system}/etc/" + ("ppsspp.tar.zst" if nano else "ppsspp.tar.xz")
            files = systree.read_payload(pack)
            controls = files["sdcard/PSP/SYSTEM/controls.ini"].decode()
            self.assertIn("Circle = 10-97\nCross = 10-96\nSquare = 10-99\nTriangle = 10-100\n", controls)
            self.assertIn("Up = 10-19\n", controls)
            self.assertEqual(files["sdcard/PSP/SYSTEM/ppsspp.ini"], b"[General]\n")
            self.assertEqual(set(files), {"sdcard/PSP", "sdcard/PSP/SYSTEM", "sdcard/PSP/SYSTEM/controls.ini",
                                          "sdcard/PSP/SYSTEM/ppsspp.ini", "data/data/org.ppsspp.ppsspp"})

    def test_27_unexpected_binds(self):
        self.build(True)
        pack = f"{self.system}/etc/ppsspp.tar.zst"
        files = systree.read_payload(pack)
        files["sdcard/PSP/SYSTEM/controls.ini"] = systree.CONTROLS_INI.replace("Cross = 10-97", "Cross = 10-23").encode()
        systree.payload(pack, files)
        before = util.manifest(self.system)
        self.assertFails(self.run_mod("27"), "controls.ini: unexpected face-button binds")
        after = util.manifest(self.system)
        self.assertEqual(after.pop("product/usr/keylayout/Vendor_484b_Product_1101.kl")[0], "file")
        for rel in ("product/usr", "product/usr/keylayout"):
            after.pop(rel)
        self.assertEqual(after, before)

    def test_37_esde_skipped_without_apk(self):
        for nano in (True, False):
            self.build(nano)
            before = util.manifest(self.system)
            self.assertIn("ESDE_APK isn't set; skipping", self.assertOk(self.run_mod("37")).stdout)
            self.assertEqual(util.manifest(self.system), before)

    def test_37_esde_errors(self):
        apk = self.write("ES-DE.apk", b"PK")
        self.build(True)
        before = util.manifest(self.system)
        self.assertFails(self.run_mod("37", env={"ESDE_APK": apk}), "ESDE_APK only works with the 1.2 base")
        self.assertEqual(util.manifest(self.system), before)
        # On 1.2 it needs ES-DE's systems file: with no network, it stops at the download.
        self.build(False)
        before = util.manifest(self.system)
        self.assertFails(self.run_mod("37", env={"ESDE_APK": apk}), "download failed: https://gitlab.com/")
        self.assertTrue(self.shim_calls("curl"))
        self.assertEqual(util.manifest(self.system), before)

    def test_45_malisx2_nano(self):
        self.build(True)
        self.assertOk(self.run_mod("45"))
        pack = f"{self.system}/etc/armsx2-malisx2.tar.gz"
        with tarfile.open(pack) as t:
            members = {m.name: m for m in t}
            prefs = t.extractfile("data/data/com.armsx2/shared_prefs/ARMSX2.xml").read().decode()
        driver = "data/data/com.armsx2/files/drivers/armsx2libmali-v0.4.0-malisx2-0.4.0-63e307e63a"
        self.assertIn(f"{driver}/libvulkan_malisx2.so", members)
        self.assertIn(f"{driver}/meta.json", members)
        self.assertNotIn(f"{driver}/README.md", members)
        self.assertFalse([n for n in members if n in ("data", "data/data")])
        self.assertEqual(members[f"{driver}/meta.json"].mode, 0o660)
        self.assertEqual((members[f"{driver}/meta.json"].uid, members[driver].mode), (0, 0o771))
        self.assertIn('<string name="renderer">vulkan</string>', prefs)
        setup = self.setup_sh()
        hook = setup.index("post_armsx2() {")
        self.assertRegex(setup[:hook], r"# thimsy: 45-armsx2-malisx2\.sh [0-9a-f]{16}\n$")
        self.assertLess(hook, setup.index('step "Starting configuration of the Thimsy system..."'))
        self.assertIn("ARMSX2 PS2 emulator|com.armsx2|/system/etc/ARMSX2-2.9.2.apk||post_armsx2",
                      util.slurp(f"{self.system}/etc/gammaos/apps.list", text=True))

    def test_45_malisx2_12(self):
        self.build(False)
        self.assertOk(self.run_mod("45"))
        setup = self.setup_sh()
        self.assertRegex(setup, r"pm install /system/etc/ARMSX2-2\.9\.2\.apk\n\n# thimsy: 45-armsx2-malisx2\.sh "
                                r"[0-9a-f]{16}\necho \"Setting up the MaliSX2 driver in ARMSX2\.\"\n")
        self.assertTrue(os.path.isfile(f"{self.system}/etc/armsx2-malisx2.tar.gz"))

    def test_45_skips_without_armsx2(self):
        self.build(True)
        os.remove(f"{self.system}/etc/malisx2-0.4.0.zip")
        before = util.manifest(self.system)
        self.assertIn("skipping", self.assertOk(self.run_mod("45")).stdout)
        self.assertEqual(util.manifest(self.system), before)

    def test_55_hotkeys(self):
        self.build(True)
        self.assertOk(self.run_mod("55"))
        dest = f"{self.system}/etc/gammaos/hotkeys"
        want = sorted(n for n in os.listdir(f"{util.MODS}/hotkeys") if n.endswith(".json"))
        self.assertEqual(sorted(os.listdir(dest)), want)
        for name in want:
            self.assertEqual(util.slurp(f"{dest}/{name}"), util.slurp(f"{util.MODS}/hotkeys/{name}"))
        self.build(False)
        before = util.manifest(self.system)
        self.assertIn("skipping", self.assertOk(self.run_mod("55")).stdout)
        self.assertEqual(util.manifest(self.system), before)

    def test_58_bootanimation(self):
        new = util.slurp(f"{util.MODS}/bootanimation/bootanimation.zip")
        for nano, media in ((True, "product/media"), (False, "media")):
            self.build(nano)
            self.assertOk(self.run_mod("58"))
            self.assertEqual(util.slurp(f"{self.system}/{media}/bootanimation.zip"), new)
            # The dark one is a link to it, and stays one.
            self.assertEqual(os.readlink(f"{self.system}/{media}/bootanimation-dark.zip"), "bootanimation.zip")
        self.build(True)
        util.rmtree(f"{self.system}/product/media")
        self.assertFails(self.run_mod("58"), "no bootanimation.zip in the 1.4.4 system")
        self.build(False)
        util.rmtree(f"{self.system}/media")
        self.assertIn("skipping", self.assertOk(self.run_mod("58")).stdout)

    def test_write_through_refuses_links(self):
        """L10: a link in the image would make write_through write the build machine's own file."""
        self.build(True)
        prop = util.slurp(f"{self.system}/build.prop")
        outside = self.write("outside.prop", prop)
        os.remove(f"{self.system}/build.prop")
        os.symlink(outside, f"{self.system}/build.prop")
        self.assertFails(self.run_mod("05"), "build.prop is a symbolic link")
        self.assertEqual(util.slurp(outside), prop)

    # --- idempotence

    def test_second_run_changes_nothing(self):
        for nano in (True, False):
            for mod in OFFLINE:
                with self.subTest(mod=mod, base="1.4.4" if nano else "1.2"):
                    self.build(nano)
                    self.assertOk(self.run_mod(mod))
                    first = util.manifest(self.system)
                    proc = self.run_mod(mod)
                    second = util.manifest(self.system)
                    self.assertEqual(second, first, util.manifest_diff(first, second))
                    if proc.returncode:
                        self.assertTrue(proc.stderr.strip(), f"{mod} failed without saying why")
                    util.rmtree(os.path.dirname(self.system))
