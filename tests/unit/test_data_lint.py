"""The data the build reads: mods/*.txt, the lock, hotkeys, the boot animation."""
import glob
import json
import os
import re
import zipfile

from tests import util

br = util.load_script("bundled_roms")
esde = util.load_script("esde_config")
ns = util.load_script("nano_systems")
M = util.MODS
SHA = re.compile(r"[0-9a-f]{64}")
CATALOG = os.path.join(util.AOSP, "frameworks/base/cmds/gammaos-nano/ps3xmb/daijishou")


def added_apps():
    """[(line number, kind, file, fields)] of added-apps.txt, split the way 40-added-apps.sh does."""
    out = []
    for n, line in enumerate(util.slurp(f"{M}/added-apps.txt", text=True).splitlines(), 1):
        if not line.replace(" ", "") or line.startswith("#"):
            continue
        kind, file, *pairs = re.split(r" {2,}", line)
        out.append((n, kind, file, dict(p.split("=", 1) for p in pairs)))
    return out


def wanted_cores_awk(path):
    proc = util.run(["bash", "-c", 'source "$1"; wanted_cores "$2"', "-", f"{util.SCRIPTS}/modlib.sh", path])
    if proc.returncode:
        raise AssertionError(proc.stderr)
    return proc.stdout.splitlines()


class Lists(util.TestCase):
    def test_every_list_has_a_parser(self):
        # A new list needs a check here too.
        self.assertEqual(sorted(os.path.basename(p) for p in glob.glob(f"{M}/*.txt")),
                         ["added-apps.txt", "bundled-roms.txt", "esde-emulators.txt", "nano-systems.txt",
                          "pack-in-media.txt", "retroarch-cores.txt"])

    def test_bundled_roms(self):
        entries = br.parse(f"{M}/bundled-roms.txt")
        self.assertTrue(entries)
        for kind, target, fields in entries:
            if kind == "folder":
                continue
            br.license_for(target.rstrip("/"), f"{M}/rom-licenses", kind == "tree")
            self.assertIn(fields.get("base", "nano"), ("nano", "1.2"), target)
            if kind == "local":
                self.assertTrue(os.path.isfile(f"{M}/rom-files/{target}"), target)
            if "sha256" in fields:
                self.assertRegex(fields["sha256"], SHA, target)
            if "url" in fields:
                self.assertRegex(fields["url"], r"^https://", target)

    def test_pack_in_media(self):
        games = {t for k, t, _ in br.parse(f"{M}/bundled-roms.txt") if k != "folder"}
        media = br.parse_media(f"{M}/pack-in-media.txt", games)
        self.assertTrue(media)
        for game, info in media.items():
            for kind in br.MEDIA_DIRS:
                image = info.get(kind)
                if not image:
                    continue
                if "file" in image:
                    path = f"{M}/pack-in-art/{image['file']}"
                    self.assertTrue(os.path.isfile(path), path)
                    br.image_ext(path, util.slurp(path)[:8])
                else:
                    self.assertRegex(image["sha256"], SHA, game)
            released = info.get("released", "")
            self.assertRegex(released, r"^(\d{4}(-\d{2}-\d{2})?)?$", game)

    def test_added_apps(self):
        apps = added_apps()
        self.assertTrue(apps)
        for n, kind, file, f in apps:
            where = f"added-apps.txt:{n}"
            self.assertIn(kind, ("apk", "file"), where)
            self.assertRegex(f.get("sha256", ""), SHA, where)
            self.assertRegex(f.get("url", ""), r"^https://", where)
            self.assertIn("source", f, where)
            self.assertNotIn("/", file, where)
            if kind == "apk":
                self.assertTrue({"name", "package"} <= f.keys(), where)
                self.assertTrue(file.endswith(".apk"), where)
            else:
                self.assertIn("dest", f, where)
            if f.get("game") == "1":
                self.assertEqual(kind, "apk", f"{where}: game=1 is only for apps")
        # 50-nano.sh finds the games with this pattern.
        games = [f["package"] for _, kind, _, f in apps if f.get("game") == "1"]
        pattern = [m for m in util.slurp(f"{M}/added-apps.txt", text=True).splitlines()
                   if re.match(r"^apk .*  game=1( |$)", m)]
        self.assertEqual(len(pattern), len(games))

    def test_retroarch_cores(self):
        path = f"{M}/retroarch-cores.txt"
        py_esde, py_nano = esde.read_cores(path), ns.read_cores(path)
        awk = wanted_cores_awk(path)
        self.assertTrue(awk)
        self.assertEqual(py_esde, py_nano)
        self.assertEqual(sorted(c + "_libretro_android.so" for c in py_esde), sorted(awk))
        self.assertEqual(len(awk), len(set(awk)), "a core is listed twice")
        for core in awk:
            self.assertRegex(core, r"^[A-Za-z0-9_\-]+_libretro_android\.so$")

    def test_core_parsers_agree_on_edge_cases(self):
        p = self.write("cores.txt", "# c\n\n  mgba   # x\nD - sameboy # y\nD -   snes9x\n\t fceumm\n# D - gone\n")
        awk = wanted_cores_awk(p)
        self.assertEqual(sorted(awk), sorted(c + "_libretro_android.so" for c in esde.read_cores(p)))
        self.assertEqual(esde.read_cores(p), ns.read_cores(p))
        self.assertEqual(sorted(awk), ["fceumm_libretro_android.so", "mgba_libretro_android.so",
                                       "sameboy_libretro_android.so", "snes9x_libretro_android.so"])

    def test_cores_lock(self):
        lock = f"{M}/retroarch-cores.lock"
        if not os.path.exists(lock):
            self.skipTest("no retroarch-cores.lock (scripts/refresh_cores.sh writes it)")
        rows = [line.split() for line in util.slurp(lock, text=True).splitlines() if line and not line.startswith("#")]
        names = [r[0] for r in rows]
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual(sorted(n for n in names if n.endswith(".so.zip")),
                         sorted(c + ".zip" for c in wanted_cores_awk(f"{M}/retroarch-cores.txt")),
                         "retroarch-cores.txt changed since the lock: run scripts/refresh_cores.sh")
        self.assertIn("info.zip", names)
        self.assertIn("PPSSPP.zip", names)
        for row in rows:
            if row[1] == "bundled":
                self.assertEqual(len(row), 2, row)
            else:
                self.assertRegex(row[1], SHA, row[0])
                self.assertEqual(len(row), 3, row)
                self.assertRegex(row[2], r"^https://buildbot\.libretro\.com/.*/" + re.escape(row[0]) + "$")
        self.assertRegex(util.slurp(lock, text=True), r"(?m)^# nightlies of \d{4}-\d{2}-\d{2}$")

    def test_esde_emulators(self):
        emulators = esde.read_emulators(f"{M}/esde-emulators.txt")
        self.assertTrue(emulators)
        for system, label in emulators.items():
            self.assertRegex(system, r"^[a-z0-9-]+$")
            self.assertTrue(label.strip())

    def test_nano_systems(self):
        systems = list(ns.parse(f"{M}/nano-systems.txt"))
        self.assertTrue(systems)
        dirs = [d for _, d, _ in systems]
        self.assertEqual(len(dirs), len(set(dirs)))
        for n, _, f in systems:
            where = f"nano-systems.txt:{n}"
            self.assertTrue({"name", "short", "icon"} <= f.keys(), where)
            self.assertRegex(f["icon"], r"^(builtin|retroarch):\S+$", where)
            if "player" not in f:
                self.assertTrue({"package", "intent", "exts"} <= f.keys(), where)
                self.assertTrue(all(e.startswith(".") for e in f["exts"].split(",")), where)

    def test_nano_players_in_catalog(self):
        if not os.path.isdir(CATALOG):
            self.skipTest(f"no Nano catalog at {CATALOG}")
        catalog = ns.read_catalog(CATALOG)
        for n, _, f in ns.parse(f"{M}/nano-systems.txt"):
            if "player" not in f:
                continue
            self.assertIn(f["player"], catalog, f"nano-systems.txt:{n}")
            player, platform_regex = catalog[f["player"]]
            exts = ns.exts_from_regex(player.get("acceptedFilenameRegex", "")) or ns.exts_from_regex(platform_regex)
            self.assertTrue(any(exts.split(",")), f"nano-systems.txt:{n}: no file extensions")


class Assets(util.TestCase):
    def test_hotkeys(self):
        ids = set()
        paths = sorted(glob.glob(f"{M}/hotkeys/*.json"))
        self.assertTrue(paths)
        for path in paths:
            with open(path) as f:
                profile = json.load(f)
            self.assertIsInstance(profile, dict, path)
            self.assertIn("id", profile, path)
            self.assertNotIn(profile["id"], ids, path)
            ids.add(profile["id"])

    def test_bootanimation(self):
        with zipfile.ZipFile(f"{M}/bootanimation/bootanimation.zip") as z:
            infos = z.infolist()
            # Android maps the frames straight out of the zip: they must be stored, not deflated.
            self.assertEqual([i.filename for i in infos if i.compress_type != zipfile.ZIP_STORED], [])
            desc = z.read("desc.txt").decode().splitlines()
            names = {i.filename for i in infos}
        width, height, fps = map(int, desc[0].split()[:3])
        self.assertTrue(width > 0 and height > 0 and fps > 0)
        parts = [line.split() for line in desc[1:] if line.strip()]
        self.assertTrue(parts)
        for p in parts:
            self.assertIn(p[0], ("p", "c"), p)
            self.assertTrue(p[1].isdigit() and p[2].isdigit(), p)
            self.assertTrue(any(n.startswith(p[3] + "/") and n.endswith(".png") for n in names), p)
            if len(p) > 4:
                self.assertRegex(p[4], r"^#[0-9a-fA-F]{6}$")

    def test_files_the_mods_install(self):
        for rel in ("keylayout/Vendor_484b_Product_1101.kl",
                    "retroarch-shaders/rg_cube_4_3_cropped.slang", "retroarch-shaders/rg_cube_4_3_cropped.slangp",
                    "retroarch-shaders/rg_cube_8_7_cropped.slang", "retroarch-shaders/rg_cube_8_7_cropped.slangp"):
            self.assertTrue(os.path.isfile(f"{M}/{rel}"), rel)

    def test_mod_scripts(self):
        for path in sorted(glob.glob(f"{M}/*.sh")) + sorted(glob.glob(f"{util.SCRIPTS}/*.sh")):
            self.assertOk(util.run(["bash", "-n", path]))
            if not path.endswith("/modlib.sh"):   # sourced, not run
                self.assertIn("\nset -euo pipefail\n", util.slurp(path, text=True), path)
