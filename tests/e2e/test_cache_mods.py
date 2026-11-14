"""The mods that need downloads, run offline from a copy of the real build cache.

Set THIMSY_TEST_CACHE to the build's cache (work/cache) and run tests/run.sh slow. Each test copies
the cache folders its mod uses (cp --reflink=auto, cheap on btrfs) and never writes to the real one;
curl is a shim that fails, so a file missing from the cache fails the test instead of downloading."""
import json
import os
import re
import subprocess
import tarfile

from tests import util
from tests.e2e.modrun import ModCase
from tests.fixtures import setup_sandbox, systree

NANO_SRC = os.path.join(util.AOSP, "frameworks/base/cmds/gammaos-nano/ps3xmb")


@util.slow
class CacheMods(ModCase):
    def setUp(self):
        super().setUp()
        if not util.TEST_CACHE or not os.path.isdir(util.TEST_CACHE):
            self.skipTest("set THIMSY_TEST_CACHE to the build's work/cache")
        self.system = systree.build(self.path("nano", "system"), True)
        apps = f"{self.system}/etc/gammaos/apps.list"
        # The base as 40-added-apps.sh finds it: ARMSX2 isn't listed yet.
        lines = [a for a in util.slurp(apps, text=True).splitlines(True) if "|com.armsx2|" not in a]
        lines += ["Aurora Store|com.aurora.store|/system/etc/AuroraStore_4.6.2.apk||post_aurora\n",
                  "Firefox|org.mozilla.firefox_beta|/system/etc/fenix-148.0b9.multi.android-arm64-v8a.apk|minmem:1300000|\n"]
        with open(apps, "w") as f:
            f.writelines(lines)
        os.remove(f"{self.system}/etc/ARMSX2-2.9.2.apk")
        for name in ("AuroraStore_4.6.2.apk", "fenix-148.0b9.multi.android-arm64-v8a.apk", "daijisho.tar.zst"):
            systree.write(f"{self.system}/etc/{name}", name)

    def seed(self, *folders):
        for folder in folders:
            src = os.path.join(util.TEST_CACHE, folder)
            if not os.path.isdir(src):
                self.skipTest(f"no {src}")
            subprocess.run(["cp", "-r", "--reflink=auto", src, os.path.join(self.cache, folder)], check=True)

    def apps(self):
        return util.slurp(f"{self.system}/etc/gammaos/apps.list", text=True)

    def setup_sh(self):
        return util.slurp(f"{self.system}/bin/setup.sh", text=True)

    def test_15_update_apps(self):
        self.seed("updated-apps")
        self.assertOk(self.run_mod("15"))
        apps = self.apps()
        self.assertIn("Aurora Store|com.aurora.store|/system/etc/AuroraStore_4.8.4.apk||post_aurora", apps)
        self.assertIn("Brave|com.brave.browser|/system/etc/Brave_1.99.55_arm64.apk|minmem:1300000|", apps)
        self.assertIn("Fulguris|net.slions.fulguris.full.fdroid|/system/etc/Fulguris_2.1.2.apk|maxmem:1300000|", apps)
        self.assertNotIn("firefox", apps)
        self.assertFalse(os.path.exists(f"{self.system}/etc/daijisho.tar.zst"))
        self.assertEqual(util.sha256_file(f"{self.system}/etc/Brave_1.99.55_arm64.apk"),
                         "c4ff5faf1c3828e6b438d10ca76b49be605604c4ebe055b2ebcc612106f0b997")
        self.assertIn("cmd role add-role-holder --user 0 android.app.role.BROWSER com.brave.browser", self.setup_sh())
        self.assertEqual(self.shim_calls("curl"), [])

    def test_40_45_46_added_apps(self):
        self.seed("apps", "scummvm")
        self.assertOk(self.run_mod("40"))
        apps = self.apps()
        for line in util.slurp(f"{util.MODS}/added-apps.txt", text=True).splitlines():
            if line.startswith("apk "):
                file = line.split()[1]
                package = re.search(r"  package=(\S+)", line).group(1)
                self.assertIn(f"|{package}|/system/etc/{file}||", apps)
                self.assertTrue(os.path.isfile(f"{self.system}/etc/{file}"), file)
        setup = self.setup_sh()
        # L8: a file is copied to /sdcard on a fresh setup only.
        self.assertRegex(setup, r'if \[ "\$\{FRESH_SETUP:-1\}" = 1 \]; then\n    mkdir -p "/sdcard/GPU Drivers"\n'
                                r'    cp /system/etc/malisx2-0\.4\.0\.zip "/sdcard/GPU Drivers/"\n')
        self.assertLess(setup.index('mkdir -p "/sdcard/GPU Drivers"'), setup.index("mkdir -p /data/setupcompleted"))
        self.assertOk(self.run_mod("45"))
        self.assertIn("|com.armsx2|/system/etc/ARMSX2-2.9.2.apk||post_armsx2", self.apps())
        bass = "sdcard/ROMs/scummvm/Beneath a Steel Sky"
        systree.payload(f"{self.system}/etc/roms.tar.zst", {bass: None, f"{bass}/sky.dsk": b"dsk",
                                                            f"{bass}.scummvm": b"sky\n"})
        self.assertOk(self.run_mod("46"))
        self.assertIn("|org.scummvm.scummvm|/system/etc/scummvm-2026.3.0.apk||post_scummvm", self.apps())
        ini = systree.read_payload(f"{self.system}/etc/scummvm.tar.zst")["data/data/org.scummvm.scummvm/files/scummvm.ini"]
        self.assertIn(b"path=/storage/emulated/0/ROMs/scummvm/Beneath a Steel Sky\n", ini)
        # A second run of 45 and 46 changes nothing; 40 refuses (its apps are listed).
        before = util.manifest(self.system)
        self.assertOk(self.run_mod("45"))
        self.assertOk(self.run_mod("46"))
        self.assertEqual(util.manifest(self.system), before)
        self.assertFails(self.run_mod("40"), "is already listed")
        self.assertEqual(self.shim_calls("curl"), [])

    def test_46_scummvm_12(self):
        """1.2 keeps the game in ScummVM's own storage, from the cached zip."""
        self.seed("scummvm")
        self.system = systree.build(self.path("v12", "system"), False)
        self.assertOk(self.run_mod("46"))
        with tarfile.open(f"{self.system}/etc/scummvm-bass.tar.xz") as t:
            names = t.getnames()
            ini = t.extractfile("data/data/org.scummvm.scummvm/files/scummvm.ini").read().decode()
        game = "data/data/org.scummvm.scummvm/files/games/Beneath a Steel Sky"
        self.assertTrue({f"{game}/sky.dsk", f"{game}/sky.dnr", f"{game}/readme.txt"} <= set(names))
        self.assertIn("versioninfo=2026.3.0\n", ini)
        self.assertIn("path=/data/user/0/org.scummvm.scummvm/files/games/Beneath a Steel Sky\n", ini)
        setup = util.slurp(f"{self.system}/bin/setup.sh", text=True)
        self.assertRegex(setup, r"pm install /system/etc/scummvm-2026\.3\.0\.apk\n\n# thimsy: 46-scummvm-game\.sh "
                                r"[0-9a-f]{16}\nif \[ ! -e /data/data/org\.scummvm\.scummvm/files/scummvm\.ini \]; "
                                r"then\necho \"Adding Beneath a Steel Sky to ScummVM\.\"\n")
        before = util.manifest(self.system)
        self.assertOk(self.run_mod("46"))
        self.assertEqual(util.manifest(self.system), before)
        self.assertEqual(self.shim_calls("curl"), [])
        if not util.have_userns():
            self.skipTest("no user namespaces for the setup sandbox")
        pack = util.slurp(f"{self.system}/etc/scummvm-bass.tar.xz")
        ini = "data/data/org.scummvm.scummvm/files/scummvm.ini"
        for rerun in (False, True):
            with self.subTest(rerun=rerun):
                box = setup_sandbox.Sandbox(self.path("rerun" if rerun else "fresh"), setup)
                with open(os.path.join(box.root, "system/etc/scummvm-bass.tar.xz"), "wb") as f:
                    f.write(pack)
                box.installed("org.scummvm.scummvm")
                if rerun:
                    os.makedirs(os.path.join(box.root, os.path.dirname(ini)))
                    with open(os.path.join(box.root, ini), "w") as f:
                        f.write("[scummvm]\nmine=1\n")
                result = box.run()
                self.assertEqual(result.returncode, 0, result.output)
                got = util.slurp(os.path.join(box.root, ini), text=True)
                if rerun:
                    self.assertEqual(got, "[scummvm]\nmine=1\n")
                    self.assertNotIn("Adding Beneath a Steel Sky to ScummVM.", result.output)
                    self.assertFalse(os.path.exists(os.path.join(box.root, os.path.dirname(ini), "games")))
                else:
                    self.assertIn("path=/data/user/0/org.scummvm.scummvm/files/games/Beneath a Steel Sky\n", got)
                    self.assertIn("Adding Beneath a Steel Sky to ScummVM.", result.output)
                    self.assertIn("restorecon -R /data/data/org.scummvm.scummvm/files", result.calls)

    def test_57_boot_sound(self):
        self.seed("boot-sound")
        self.assertOk(self.run_mod("57"))
        self.assertEqual(util.sha256_file(f"{self.system}/etc/nano_xmb/audio/esde_boot.mp3"),
                         "52d6b9545aa131150457845dd8ccbf585ba6e7c5c5e13a5ec5bf84812b416f5e")

    def test_30_bundled_roms(self):
        self.seed("roms")
        br = util.load_script("bundled_roms")
        entries = br.parse(f"{util.MODS}/bundled-roms.txt")
        files = {"sdcard/ROMs": None}
        for kind, target, _ in entries:
            system = target.split("/")[0]
            files[f"sdcard/ROMs/{system}"] = None
            files[f"sdcard/ROMs/{system}/systeminfo.txt"] = b"info"
            if kind == "upstream":
                files[f"sdcard/ROMs/{target}"] = b"upstream game"
        files["sdcard/ROMs/gb/Removed Game.gb"] = b"no grant"
        systree.payload(f"{self.system}/etc/roms.tar.zst", files)
        proc = self.assertOk(self.run_mod("30"))
        self.assertIn("removed gb/Removed Game.gb", proc.stdout)
        out = systree.read_payload(f"{self.system}/etc/roms.tar.zst")
        names = set(out)
        self.assertNotIn("sdcard/ROMs/gb/Removed Game.gb", names)
        for kind, target, _ in entries:
            if kind in ("add", "local"):
                self.assertIn(f"sdcard/ROMs/{target}", names)
        # Beneath a Steel Sky's files, beside the .scummvm file ScummVM and its RetroArch core open.
        bass = "sdcard/ROMs/scummvm/Beneath a Steel Sky"
        self.assertEqual(out[f"{bass}.scummvm"], b"sky\n")
        for name in ("sky.dsk", "sky.dnr", "sky.cpt", "readme.txt", "LICENSE.txt"):
            self.assertIn(f"{bass}/{name}", names)
        index = json.loads(out["data/system/nano_scrape/index.json"])
        self.assertTrue(index["items"])
        self.assertIn("setprop sys.gammaos.nano.scrape_reload", self.setup_sh())
        self.assertEqual(self.shim_calls("curl"), [])

    def test_50_nano(self):
        for tool in ("javac", "keytool", "aapt2", "d8", "zipalign", "apksigner", "zip"):
            if not util.which(tool):
                self.skipTest(f"{tool} isn't on PATH (scripts/build_android_games.sh)")
        if not os.environ.get("ANDROID_JAR") or not os.path.isdir(NANO_SRC):
            self.skipTest("needs ANDROID_JAR and the Nano source tree")
        self.seed("nano")
        etc = f"{self.system}/etc"
        subprocess.run(["cp", "-r", f"{NANO_SRC}/daijishou", f"{etc}/nano_xmb/daijishou"], check=True)
        icons = os.path.join(NANO_SRC, "icons_retroarch")
        if os.path.isdir(icons):
            subprocess.run(["cp", "-r", icons, f"{etc}/nano_xmb/icons_retroarch"], check=True)
        with open(f"{etc}/gammaos/apps.list", "a") as f:
            for _, _, fields in util.load_script("nano_systems").parse(f"{util.MODS}/nano-systems.txt"):
                if "package" in fields:
                    f.write(f"X|{fields['package']}|/system/etc/x.apk||\n")
        proc = self.run_mod("50", env={"ANDROID_JAR": os.environ["ANDROID_JAR"]})
        self.assertOk(proc)
        prop = util.slurp(f"{self.system}/build.prop", text=True)
        self.assertIn("persist.gammaos.nano.esde.themeset=colorful-simplified-es-de", prop)
        self.assertTrue(os.path.isfile(f"{etc}/gammaos/nano_systems.json"))
        # A second run refuses before it changes anything.
        before = util.manifest(self.system)
        self.assertFails(self.run_mod("50", env={"ANDROID_JAR": os.environ["ANDROID_JAR"]}), "build.prop already sets")
        self.assertEqual(util.manifest(self.system), before)

    def test_10_emulators_from_the_lock(self):
        """The RetroArch cores come from the lock's cached files, checked, with no download."""
        lock = f"{util.MODS}/retroarch-cores.lock"
        if not os.path.exists(lock):
            self.skipTest("no retroarch-cores.lock")
        self.seed("emulators")
        ra = "data/data/com.retroarch.aarch64"
        cfg = "".join(f'input_{k}_btn = "{v}"\n' for k, v in (("a", 96), ("b", 97), ("x", 99), ("y", 100)))
        files = {ra: None, f"{ra}/cores": None, f"{ra}/cores/fake08_libretro_android.so": b"bundled fake08",
                 f"{ra}/cores/dropped_libretro_android.so": b"not wanted", f"{ra}/info": None,
                 f"{ra}/autoconfig": None, f"{ra}/autoconfig/android": None,
                 f"{ra}/autoconfig/android/retrogame_joypad.cfg": cfg.encode(),
                 f"{ra}/shaders": None, f"{ra}/shaders/shaders_slang": None,
                 "sdcard/RetroArch": None, "sdcard/RetroArch/system": None,
                 "sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg":
                     cfg.replace("input_", "input_player1_").encode()}
        systree.payload(f"{self.system}/etc/retroarch.tar.zst", files)
        for name in ("flycast-release.apk", "ppsspp_1.20.3.apk"):
            systree.write(f"{self.system}/etc/{name}", name)
        with open(f"{self.system}/etc/gammaos/apps.list", "a") as f:
            f.write("Flycast|com.flycast.emulator|/system/etc/flycast-release.apk||post_flycast\n")
        self.assertOk(self.run_mod("10"))
        out = systree.read_payload(f"{self.system}/etc/retroarch.tar.zst")
        cores = sorted(n.rsplit("/", 1)[1] for n in out if n.startswith(f"{ra}/cores/") and n.endswith(".so"))
        wanted = [row.split()[0][:-4] for row in util.slurp(lock, text=True).splitlines()
                  if not row.startswith("#") and row.split()[0].endswith(".so.zip")]
        self.assertEqual(cores, sorted(w for w in wanted if w.endswith(".so")))
        self.assertEqual(out[f"{ra}/cores/fake08_libretro_android.so"], b"bundled fake08")
        self.assertIn('input_a_btn = "97"', out[f"{ra}/autoconfig/android/retrogame_joypad.cfg"].decode())
        self.assertIn("sdcard/RetroArch/system/Machines/MSX2 - C-BIOS/cbios_main_msx2.rom", out)
        # The ScummVM core lists the ROM pack's Beneath a Steel Sky; neither the core nor its data ships.
        self.assertIn(b"path=/storage/emulated/0/ROMs/scummvm/Beneath a Steel Sky\n",
                      out["sdcard/RetroArch/system/scummvm.ini"])
        self.assertFalse([n for n in out if n.endswith("scummvm_libretro_android.so") or n.startswith("sdcard/RetroArch/system/scummvm/")])
        self.assertEqual(self.shim_calls("curl"), [])
        self.assertIn("|/system/etc/ppsspp_1.20.4.apk|", self.apps())

    def test_10_core_not_as_locked(self):
        """A core the cache lacks and the buildbot can't give stops the build, unless ALLOW_BUNDLED_CORES=1."""
        lock = f"{util.MODS}/retroarch-cores.lock"
        if not os.path.exists(lock):
            self.skipTest("no retroarch-cores.lock")
        row = next(r.split() for r in util.slurp(lock, text=True).splitlines() if r.startswith("mgba_"))
        self.seed("emulators")
        os.remove(os.path.join(self.cache, "emulators/buildbot", row[1]))
        ra = "data/data/com.retroarch.aarch64"
        cfg = "".join(f'input_{k}_btn = "{v}"\n' for k, v in (("a", 96), ("b", 97), ("x", 99), ("y", 100)))
        files = {ra: None, f"{ra}/cores": None, f"{ra}/cores/mgba_libretro_android.so": b"bundled mgba",
                 f"{ra}/cores/fake08_libretro_android.so": b"bundled fake08", f"{ra}/info": None,
                 f"{ra}/autoconfig/android/retrogame_joypad.cfg": cfg.encode(), f"{ra}/shaders/shaders_slang": None,
                 "sdcard/RetroArch/system": None,
                 "sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg":
                     cfg.replace("input_", "input_player1_").encode()}
        systree.payload(f"{self.system}/etc/retroarch.tar.zst", files)
        for name in ("flycast-release.apk", "ppsspp_1.20.3.apk"):
            systree.write(f"{self.system}/etc/{name}", name)
        self.assertFails(self.run_mod("10"), "failed to get mgba (ALLOW_BUNDLED_CORES=1 keeps the base's copy)")
        self.assertTrue(self.shim_calls("curl"))
        proc = self.assertOk(self.run_mod("10", env={"ALLOW_BUNDLED_CORES": "1"}))
        self.assertIn("WARNING: kept bundled mgba", proc.stderr)
        out = systree.read_payload(f"{self.system}/etc/retroarch.tar.zst")
        self.assertEqual(out[f"{ra}/cores/mgba_libretro_android.so"], b"bundled mgba")
