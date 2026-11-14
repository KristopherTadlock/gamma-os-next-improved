"""mods/device-keys.json, and PhoneWindowManager's resolver for it (ThimsyDeviceKeys.java).

Nano's resolver (NanoDeviceKeys.cpp) is tested in AOSP (gammaos_nano_unit_tests) on the same
fixture cases the Java one runs here, so the two pick the same keys.
"""
import json
import os
import re
import shutil
import tempfile

from tests import util

TABLE = os.path.join(util.MODS, "device-keys.json")
POLICY = os.path.join(util.AOSP, "frameworks/base/services/core/java/com/android/server/policy")
ORG_JSON = os.path.join(util.AOSP, "libcore/json/src/main/java/org/json")
NANO_CASES = os.path.join(util.AOSP, "frameworks/base/cmds/gammaos-nano/test/fixtures/devkeys/cases.json")
CASES = os.path.join(util.TESTS, "fixtures/device_keys/cases.json")
CHECK = os.path.join(util.TESTS, "fixtures/device_keys/DeviceKeysCheck.java")
FIELDS = ("id", "function_linux", "function_android", "gamepad_only", "back_linux", "shared_back")

# The annotations libcore's org.json carries, so it compiles on its own.
TYPE_USE = ("@java.lang.annotation.Target({java.lang.annotation.ElementType.TYPE_USE, "
            "java.lang.annotation.ElementType.PARAMETER, java.lang.annotation.ElementType.METHOD, "
            "java.lang.annotation.ElementType.FIELD})\n")
STUBS = {
    "android/annotation/SystemApi.java":
        "package android.annotation;\n"
        "public @interface SystemApi { enum Client { PRIVILEGED_APPS, MODULE_LIBRARIES, SYSTEM_SERVER }\n"
        "  Client client() default Client.PRIVILEGED_APPS; }\n",
    "android/compat/annotation/UnsupportedAppUsage.java":
        "package android.compat.annotation;\n"
        "public @interface UnsupportedAppUsage { int maxTargetSdk() default 0; long trackingBug() default 0;\n"
        "  String implicitMember() default \"\"; String publicAlternatives() default \"\"; }\n",
    "libcore/util/NonNull.java": "package libcore.util;\n" + TYPE_USE + "public @interface NonNull {}\n",
    "libcore/util/Nullable.java": "package libcore.util;\n" + TYPE_USE + "public @interface Nullable {}\n",
}


def load_table():
    with open(TABLE) as f:
        return json.load(f)


def check_key(tc, key, where):
    tc.assertIsInstance(key, dict, where)
    tc.assertTrue(key.get("label"), where)
    tc.assertIsInstance(key.get("linux"), int, where)
    tc.assertTrue(0 <= key["linux"] <= 0x2FF, where)
    if "android" in key:   # absent: no Android keycode on the raw pad (Nano and drastic-nano only)
        tc.assertRegex(key["android"], r"^[A-Z][A-Z0-9_]*$", where)
        tc.assertFalse(key["android"].startswith("KEYCODE_"), where)
    tc.assertIn(key.get("gamepad_only", False), (True, False), where)


class Table(util.TestCase):
    def test_shape(self):
        doc = load_table()
        self.assertEqual(doc.get("version"), 1)
        for field in ("function", "back"):
            for key in doc["default"][field]:
                check_key(self, key, f"default {field}")
        ids = set()
        for entry in doc["devices"]:
            where = entry.get("id")
            self.assertRegex(where or "", r"^[a-z0-9-]+$")
            self.assertNotIn(where, ids)
            ids.add(where)
            self.assertTrue(entry.get("name"), where)
            self.assertTrue(entry.get("verified"), where)   # where the keys come from
            match = entry["match"]
            self.assertIsInstance(match, list, where)
            self.assertTrue(match, where)
            for alt in match:
                self.assertTrue(alt, where)
                for prop, want in alt.items():
                    self.assertRegex(prop, r"^ro\.[a-z0-9_.]+$", where)
                    for w in want if isinstance(want, list) else [want]:
                        self.assertIsInstance(w, str, where)
                        self.assertNotIn(w.rstrip("*"), ("", "*"), where)
                        self.assertNotIn("*", w[:-1], where)   # only a trailing prefix star
            for field in ("function", "back"):
                for key in entry.get(field, []):
                    check_key(self, key, f"{where} {field}")
            self.assertIn(entry.get("fallback", "none"), ("none", "back"), where)
            if entry.get("fallback") == "back":
                self.assertNotEqual(entry.get("back", doc["default"]["back"]), [], where)

    def test_rg_cube_keys_unchanged(self):
        """The Cube's own entry is the keys Thimsy always used: F10 from its pad, BUTTON_MODE, Back."""
        doc = load_table()
        cube = next(e for e in doc["devices"] if e["id"] == "anbernic-rgcube")
        self.assertIs(doc["devices"][0], cube)   # nothing can match before it
        self.assertEqual([(k["linux"], k["android"], k.get("gamepad_only", False)) for k in cube["function"]],
                         [(68, "F10", True), (316, "BUTTON_MODE", False)])
        self.assertEqual([(k["linux"], k["android"]) for k in cube["back"]], [(158, "BACK")])
        self.assertNotIn("fallback", cube)
        self.assertIn({"ro.product.vendor.model": "RGCUBE"}, cube["match"])
        # The Cube's key layout, which the entry relies on.
        kl = util.slurp(os.path.join(util.MODS, "keylayout/Vendor_484b_Product_1101.kl"), text=True)
        for line in ("key 68    F10", "key 316   BUTTON_MODE", "key 158   BACK"):
            self.assertRegex(kl, r"(?m)^" + r"\s+".join(map(re.escape, line.split())) + r"\s*$")


class KeyLayouts(util.TestCase):
    def test_gammapad_02fd_layout_maps_the_function_button(self):
        """A GammaPad virtual pad presenting as 045e:02fd must still deliver F10 (the Anbernic button)."""
        kl = os.path.join(util.AOSP, "frameworks/base/data/keyboards/Vendor_045e_Product_02fd.kl")
        if not os.path.isfile(kl):
            self.skipTest(f"no {kl}")
        self.assertRegex(util.slurp(kl, text=True), r"(?m)^key\s+68\s+F10\s*$")


class JavaResolver(util.TestCase):
    classes = None

    @classmethod
    def setUpClass(cls):
        cls.build_error = None
        if not (shutil.which("javac") and shutil.which("java")):
            cls.build_error = "javac isn't on PATH"
            return
        if not (os.path.isfile(os.path.join(POLICY, "ThimsyDeviceKeys.java")) and os.path.isdir(ORG_JSON)):
            cls.build_error = f"no AOSP tree at {util.AOSP}"
            return
        cls.work = tempfile.mkdtemp(prefix="thimsy-devkeys-")
        src = os.path.join(cls.work, "src")
        for rel, text in STUBS.items():
            os.makedirs(os.path.dirname(os.path.join(src, rel)), exist_ok=True)
            with open(os.path.join(src, rel), "w") as f:
                f.write(text)
        sources = [os.path.join(src, rel) for rel in STUBS]
        sources += [os.path.join(ORG_JSON, n) for n in sorted(os.listdir(ORG_JSON)) if n.endswith(".java")]
        sources += [os.path.join(POLICY, "ThimsyDeviceKeys.java"), CHECK]
        cls.classes = os.path.join(cls.work, "classes")
        os.makedirs(cls.classes)
        proc = util.run(["javac", "-nowarn", "-encoding", "UTF-8", "-d", cls.classes, *sources], timeout=300)
        if proc.returncode:
            raise AssertionError("javac failed:\n" + proc.stdout + proc.stderr)

    @classmethod
    def tearDownClass(cls):
        if getattr(cls, "work", None):
            util.rmtree(cls.work)

    def setUp(self):
        super().setUp()
        if self.build_error:
            self.skipTest(self.build_error)

    def resolve(self, cases_path):
        proc = self.assertOk(util.run(["java", "-cp", self.classes,
                                       "com.android.server.policy.DeviceKeysCheck", cases_path]))
        return json.loads(proc.stdout)["resolved"]

    def check_cases(self, cases_path):
        with open(cases_path) as f:
            cases = json.load(f)["cases"]
        got = self.resolve(cases_path)
        self.assertEqual(len(got), len(cases))
        for case, result in zip(cases, got):
            for field in FIELDS + tuple(f for f in ("back_android",) if f in case):
                self.assertEqual(result[field], case[field], f"{case['name']}: {field}")

    def test_same_cases_as_nano(self):
        if not os.path.isfile(NANO_CASES):
            self.skipTest(f"no {NANO_CASES}")
        self.check_cases(NANO_CASES)

    def test_back_release_same_as_nano(self):
        """Back acts on release: the decision PhoneWindowManager makes, on Nano's cases."""
        if not os.path.isfile(NANO_CASES):
            self.skipTest(f"no {NANO_CASES}")
        with open(NANO_CASES) as f:
            cases = json.load(f)["back_release"]
        proc = self.assertOk(util.run(["java", "-cp", self.classes,
                                       "com.android.server.policy.DeviceKeysCheck", NANO_CASES]))
        got = json.loads(proc.stdout)["back_release"]
        self.assertEqual(got, [c["want"] for c in cases])

    def test_the_table(self):
        self.check_cases(CASES)

    def test_no_table_is_the_rg_cube(self):
        p = self.write("cases.json", json.dumps({"table": None, "cases": [{"props": {}}]}))
        (r,) = self.resolve(p)
        self.assertEqual(r["id"], "builtin")
        self.assertEqual(r["function_android"], ["F10", "BUTTON_MODE"])
        self.assertEqual(r["gamepad_only"], [True, False])
        self.assertEqual(r["back_android"], ["BACK"])
        self.assertFalse(r["shared_back"])

    def test_malformed_table_is_the_rg_cube(self):
        for bad in ("", "{", "[]", "null"):
            self.write("t.json", bad)
            p = self.write("cases.json", json.dumps({"table": "t.json", "cases": [{"props": {"ro.product.vendor.model": "RG405M"}}]}))
            (r,) = self.resolve(p)
            self.assertEqual((r["id"], r["function_linux"], r["shared_back"]), ("builtin", [68, 316], False), bad)
