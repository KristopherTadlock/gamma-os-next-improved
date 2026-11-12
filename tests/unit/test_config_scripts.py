"""esde_config.py, nano_systems.py and odm_sepolicy.py."""
import contextlib
import io
import json
import os
import sys
import tarfile
import xml.etree.ElementTree as ET
from unittest import mock

from tests import util

esde = util.load_script("esde_config")
ns = util.load_script("nano_systems")
odm = util.load_script("odm_sepolicy")

ES_SYSTEMS = """<?xml version="1.0"?>
<systemList>
  <system><name>gb</name><extension>.gb .GB .zip</extension>
    <command label="Gambatte">%EMULATOR_RETROARCH% -e LIBRETRO /data/data/com.retroarch.aarch64/cores/gambatte_libretro_android.so</command>
    <command label="SameBoy">%EMULATOR_RETROARCH% -e LIBRETRO /data/data/com.retroarch.aarch64/cores/sameboy_libretro_android.so</command>
  </system>
  <system><name>psp</name><extension>.iso .cso .pbp</extension>
    <command label="PPSSPP (Standalone)">%EMULATOR_PPSSPP% %ROM%</command>
  </system>
  <system><name>nes</name><extension>.nes .zip</extension>
    <command label="FCEUmm">%EMULATOR_RETROARCH% -e LIBRETRO /data/data/com.retroarch.aarch64/cores/fceumm_libretro_android.so</command>
    <command label="Mesen">%EMULATOR_RETROARCH% -e LIBRETRO /data/data/com.retroarch.aarch64/cores/mesen_libretro_android.so</command>
  </system>
</systemList>
"""


def exits(testcase, func, *args):
    with testcase.assertRaises(SystemExit) as e, contextlib.redirect_stdout(io.StringIO()):
        func(*args)
    return str(e.exception)


def add(tar, name, data=None, mode=0o660):
    info = tarfile.TarInfo(name)
    if data is None:
        info.type, info.mode = tarfile.DIRTYPE, 0o2770
        tar.addfile(info)
    else:
        info.size, info.mode = len(data), mode
        tar.addfile(info, io.BytesIO(data))


class EsdeConfig(util.TestCase):
    def systems(self):
        return {s.findtext("name"): s for s in ET.fromstring(ES_SYSTEMS).iter("system")}

    def test_parsers(self):
        p = self.write("cores.txt", "# x\nmgba   # GBA\nD - sameboy  # GB\n\n")
        self.assertEqual(esde.read_cores(p), {"mgba", "sameboy"})
        p = self.write("emu.txt", "# x\ngb   SameBoy\npsp  PPSSPP (Standalone)\n")
        self.assertEqual(esde.read_emulators(p), {"gb": "SameBoy", "psp": "PPSSPP (Standalone)"})

    def test_check_emulators(self):
        s = self.systems()
        esde.check_emulators(s, {"gb": "SameBoy", "psp": "PPSSPP (Standalone)"}, {"sameboy"}, {"gb", "psp"})
        self.assertIn("gb: ES-DE has no emulator 'mGBA'", exits(self, esde.check_emulators, s, {"gb": "mGBA"}, set(), set()))
        self.assertIn("needs the sameboy core, which isn't shipped",
                      exits(self, esde.check_emulators, s, {"gb": "SameBoy"}, {"mgba"}, set()))
        # nes's default (FCEUmm) isn't shipped, but Mesen is: the list must pick one.
        self.assertIn("nes: ES-DE's default needs the fceumm core; add a line for one of ['Mesen']",
                      exits(self, esde.check_emulators, s, {}, {"mesen"}, {"nes"}))
        esde.check_emulators(s, {}, {"fceumm"}, {"nes"})
        esde.check_emulators(s, {}, set(), {"nes", "n64"})

    def test_main(self):
        roms = self.path("roms.tar")
        with tarfile.open(roms, "w") as t:
            for d in ("sdcard/ROMs", "sdcard/ROMs/gb", "sdcard/ROMs/psp", "sdcard/ROMs/psp/Game", "sdcard/ROMs/nes"):
                add(t, d)
            add(t, "sdcard/ROMs/gb/Tobu & Tobu.gb", b"G")
            add(t, "sdcard/ROMs/gb/readme.txt", b"R")
            add(t, "sdcard/ROMs/psp/Game/EBOOT.PBP", b"P")
            add(t, "sdcard/ES-DE/gamelists/gb/gamelist.xml",
                b"<gameList><game><path>./Tobu &amp; Tobu.gb</path><name>Tobu Tobu</name>"
                b"<developer>Dev</developer></game></gameList>")
        out = self.path("out")
        proc = util.run_script("esde_config", self.write("es_systems.xml", ES_SYSTEMS),
                               self.write("emu.txt", "gb  SameBoy\n"), self.write("cores.txt", "D - sameboy\nfceumm\n"),
                               roms, out)
        self.assertOk(proc)
        self.assertIn("ES-DE: 2 games, 1 default emulators", proc.stdout)
        gb = util.slurp(f"{out}/ES-DE/gamelists/gb/gamelist.xml", text=True)
        self.assertIn("<alternativeEmulator>\n\t<label>SameBoy</label>", gb)
        self.assertIn("<path>./Tobu &amp; Tobu.gb</path>\n\t\t<name>Tobu Tobu</name>", gb)
        self.assertIn("<developer>Dev</developer>", gb)
        self.assertNotIn("readme", gb)
        psp = util.slurp(f"{out}/ES-DE/gamelists/psp/gamelist.xml", text=True)
        self.assertIn("<folder>\n\t\t<path>./Game</path>\n\t\t<name>Game</name>\n\t\t<folderlink>EBOOT.PBP</folderlink>", psp)
        self.assertFalse(os.path.exists(f"{out}/ES-DE/gamelists/nes"))
        self.assertIn('<string name="ROMDirectory" value="/storage/emulated/0/ROMs" />',
                      util.slurp(f"{out}/ES-DE/settings/es_settings.xml", text=True))


RA_ARGS = ("-n com.retroarch.aarch64/com.retroarch.browser.retroactivity.RetroActivityFuture "
           "-e ROM {file.path} -e LIBRETRO /data/data/com.retroarch.aarch64/cores/CORE_libretro_android.so "
           "-e CONFIGFILE /storage/emulated/0/Android/data/com.retroarch.aarch64/files/retroarch.cfg")


class NanoSystems(util.TestCase):
    def setUp(self):
        super().setUp()
        self.catalog = self.path("catalog")
        self.platform("gb.json", r"^.*\.(?:gb|zip)$", [
            {"uniqueId": "ra.sameboy", "amStartArguments": RA_ARGS.replace("CORE", "sameboy"),
             "acceptedFilenameRegex": r"^.*\.(?:gb|gbc|zip|7z)$"},
            {"uniqueId": "ra.gambatte", "amStartArguments": RA_ARGS.replace("CORE", "gambatte")}])
        self.platform("nds.json", r"^.*\.(?:nds|zip)$", [
            {"uniqueId": "drastic", "amStartArguments": "-n com.dsemu.drastic/.DraSticActivity\n  -d {file.uri}"}])
        self.platform("odd.json", r"^.*$", [
            {"uniqueId": "noexts", "amStartArguments": "-n com.dsemu.drastic/.X -d {file.uri}",
             "acceptedFilenameRegex": r"^.*\.(?:\w+)$"}])
        os.makedirs(self.path("icons"))
        self.write("icons/gb.png", b"png")
        self.apps = self.write("apps.list", "# x\nDraStic|com.dsemu.drastic|/system/etc/d.apk||\n")
        self.cores = self.write("cores.txt", "D - sameboy  # GB\nmgba\n")
        self.out = self.path("out.json")

    def platform(self, name, regex, players):
        definition = {"platform": {"acceptedFilenameRegex": regex}, "playerList": players}
        self.write(f"catalog/{name}", json.dumps(definition))

    def systems(self, text, *builtin):
        return util.run_script("nano_systems", self.write("systems.txt", text), self.catalog, self.path("icons"),
                               self.apps, self.cores, self.out, *builtin)

    def test_helpers(self):
        self.assertEqual(ns.exts_from_regex(r"^.*\.(?:gb|gbc|z\.ip|)$"), ".gb,.gbc")
        self.assertEqual(ns.exts_from_regex("^.*$"), "")
        self.assertEqual(ns.read_cores(self.cores), {"sameboy", "mgba"})
        self.assertEqual(list(ns.parse(self.write("s.txt", "# c\n\ngb  name=GB  short=gb\n"))),
                         [(3, "gb", {"name": "GB", "short": "gb"})])

    def test_systems(self):
        proc = self.systems("gb  name=Game Boy  short=GB  player=ra.sameboy  icon=retroarch:gb\n"
                            "nds  name=DS  short=NDS  player=drastic  icon=builtin:nds\n"
                            "android  name=Android Games  short=AG  package=org.thimsy.androidgames  "
                            "intent=-n org.thimsy.androidgames/.Main  exts=.apk  icon=builtin:android\n",
                            "org.thimsy.androidgames")
        self.assertOk(proc)
        out = json.loads(util.slurp(self.out))
        gb, nds, android = out["systems"]
        self.assertEqual((gb["launchType"], gb["coreSo"], gb["extensions"]),
                         ("libretro-core", "sameboy_libretro_android.so", [".gb", ".gbc", ".zip", ".7z"]))
        self.assertEqual((nds["launchType"], nds["package"], nds["extensions"]),
                         ("custom-package", "com.dsemu.drastic", [".nds", ".zip"]))
        self.assertEqual(nds["intentTemplate"], "-n com.dsemu.drastic/.DraSticActivity -d {file.uri}")
        self.assertEqual((android["package"], android["extensions"], android["order"]), ("org.thimsy.androidgames", [".apk"], 2))

    def test_errors(self):
        for line, message in [
            ("gb  name=G  short=G  player=nope  icon=builtin:gb", "no player nope in Nano's catalog"),
            ("gb  name=G  short=G  player=ra.gambatte  icon=builtin:gb", "core gambatte isn't in retroarch-cores.txt"),
            ("x  name=X  short=X  package=com.other  intent=-n com.other/.A  exts=.x  icon=builtin:x",
             "com.other isn't installed by apps.list"),
            ("gb  name=G  short=G  player=ra.sameboy  icon=retroarch:missing", "no icon missing.png"),
            ("gb  name=G  short=G  player=ra.sameboy  icon=other:gb", "unknown icon other:gb"),
            # L9: a system Nano would list no games for.
            ("odd  name=O  short=O  player=noexts  icon=builtin:x", "line 1: no file extensions for odd"),
            ("x  name=X  short=X  package=com.dsemu.drastic  intent=-n com.dsemu.drastic/.A  exts=  icon=builtin:x",
             "line 1: no file extensions for x"),
        ]:
            proc = self.systems(line + "\n")
            self.assertFails(proc, message)
            self.assertFalse(os.path.exists(self.out), line)


class OdmSepolicy(util.TestCase):
    """main() with shims for debugfs, fsck.erofs and secilc that record what they are given."""

    def setUp(self):
        super().setUp()
        fs = self.path("fs")
        sel = "system.img/system/etc/selinux"
        for rel, text in [(f"{sel}/plat_sepolicy.cil", "plat"), (f"{sel}/mapping/30.0.cil", "map"),
                          (f"{sel}/mapping/30.0.compat.cil", "compat"),
                          (f"{sel}/plat_sepolicy_and_mapping.sha256", "abc123def4567890\n"),
                          ("system.img/system/system_ext/etc/selinux/system_ext_sepolicy.cil", "se"),
                          ("system.img/system/system_ext/etc/selinux/mapping/30.0.cil", "semap"),
                          ("system.img/system/system_ext/etc/selinux/system_ext_sepolicy_and_mapping.sha256", "se-hash\n"),
                          ("vendor.img/etc/selinux/plat_sepolicy_vers.txt", "30.0\n"),
                          ("vendor.img/etc/selinux/vendor_sepolicy.cil", "vendor"),
                          ("vendor.img/etc/selinux/plat_pub_versioned.cil", "pub"),
                          ("odm.img/etc/selinux/precompiled_sepolicy", "old policy"),
                          ("odm.img/etc/selinux/precompiled_sepolicy.plat_sepolicy_and_mapping.sha256", "old\n"),
                          ("odm.img/etc/selinux/precompiled_sepolicy.system_ext_sepolicy_and_mapping.sha256", "old\n"),
                          ("odm.img/etc/selinux/odm_sepolicy.cil", "odmcil")]:
            self.write(f"fs/{rel}", text)
        self.argv = self.path("secilc.argv")
        self.bin = util.shim_dir(self.tmp, extra={
            "debugfs": """#!/bin/sh
set -- $2 "$3"
[ "$1" = rdump ] || exit 1
src="$FAKE_FS/$(basename "$4")$2"
[ -d "$src" ] && cp -r "$src" "$3/"
exit 0
""",
            "fsck.erofs": """#!/bin/sh
dir=${1#--extract=}
mkdir -p "$dir" && cp -r "$FAKE_FS/$(basename "$3")/." "$dir/"
""",
            "secilc": """#!/bin/sh
printf '%s\\n' "$@" >"$SECILC_ARGV"
while [ $# -gt 0 ]; do [ "$1" = -o ] && echo "NEW POLICY" >"$2"; shift; done
"""})
        self.env = {"PATH": self.bin + os.pathsep + os.environ["PATH"], "FAKE_FS": fs, "SECILC_ARGV": self.argv}

    def run_main(self):
        captured = {}

        def rebuild(image, label, changes):
            captured.update({path: func(b"old") for path, func in changes.items()}, label=label, image=image)
            return b"ODM IMAGE"
        out = self.path("odm.new")
        with mock.patch.dict(os.environ, self.env), mock.patch.object(odm, "rebuild", rebuild), \
                mock.patch.object(sys, "argv", ["odm_sepolicy.py", self.path("odm.img"), self.path("system.img"),
                                                self.path("vendor.img"), out]), \
                contextlib.redirect_stdout(io.StringIO()):
            odm.main()
        return captured, out

    def test_compiles_like_init(self):
        captured, out = self.run_main()
        self.assertEqual(util.slurp(out), b"ODM IMAGE")
        self.assertEqual(captured.pop("label"), "odm")
        self.assertEqual(captured.pop("image"), self.path("odm.img"))
        self.assertEqual(captured, {
            "etc/selinux/precompiled_sepolicy": b"NEW POLICY\n",
            "etc/selinux/precompiled_sepolicy.plat_sepolicy_and_mapping.sha256": b"abc123def4567890\n",
            "etc/selinux/precompiled_sepolicy.system_ext_sepolicy_and_mapping.sha256": b"se-hash\n"})
        args = util.slurp(self.argv, text=True).splitlines()
        tmpdir = args[0][:args[0].index("/plat/selinux/")]
        rel = [os.path.relpath(a, tmpdir) if a.startswith(tmpdir) else a for a in args]
        self.assertEqual(rel, [
            "plat/selinux/plat_sepolicy.cil", "-m", "-M", "true", "-G", "-N", "-c", "30",
            "plat/selinux/mapping/30.0.cil", "-o", "precompiled_sepolicy", "-f", os.devnull,
            "plat/selinux/mapping/30.0.compat.cil",
            "system_ext/selinux/system_ext_sepolicy.cil", "system_ext/selinux/mapping/30.0.cil",
            "vendor/selinux/plat_pub_versioned.cil", "vendor/selinux/vendor_sepolicy.cil",
            "odm/etc/selinux/odm_sepolicy.cil"])

    def test_hash_files_must_pair(self):
        os.remove(self.path("fs/odm.img/etc/selinux/precompiled_sepolicy.system_ext_sepolicy_and_mapping.sha256"))
        with self.assertRaises(SystemExit) as e:
            self.run_main()
        self.assertIn("system_ext_sepolicy_and_mapping.sha256 is absent on odm but present in the system image",
                      str(e.exception))

    def test_missing_policy(self):
        util.rmtree(self.path("fs/vendor.img"))
        with self.assertRaises(SystemExit) as e:
            self.run_main()
        self.assertIn("no SELinux policy in the system or vendor image", str(e.exception))
