#!/usr/bin/env python3
"""Tests for apply_pack.py (stdlib only): run it on a temporary copy of the
catalog for two profiles and check neither wipes the other.

  python3 installer/tools/test_apply_pack.py
"""
import json, os, re, shutil, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOOL = os.path.join(HERE, "apply_pack.py")
CATALOG = os.path.join(os.path.dirname(HERE), "catalog")
DEVICE = "anbernic-rg-cube"


def sha(c):
    return c * 64


def flow_to_json(text):
    """The flow mappings apply_pack writes (bare keys, JSON values) as JSON."""
    return json.loads(re.sub(r'([{,]\s*)([A-Za-z_][\w-]*):', r'\1"\2":', text))


def top_block(text, name):
    """The indented lines of the top-level block `name:`."""
    m = re.search(rf"^{name}:[ \t]*\n((?:[ \t]+\S.*\n)*)", text, flags=re.M)
    return m.group(1) if m else None


def pack(profile, c, ops):
    return {
        "reference_sha256": sha(c),
        "source_sha256": sha("5"),
        "system": {
            "id": f"thimsy-system-test-{profile}",
            "file": f"thimsy-system-test-{profile}.img.zst",
            "size": 1000,
            "sha256": sha(c),
            "unpack": {"size": 4000, "sha256": sha("e")},
        },
        "artifacts": [
            {"id": "rgcube-odm-a-delta", "file": "rgcube-odm-a-delta.bsdiff", "size": 10, "sha256": sha("a")},
        ],
        "build": ops,
    }


def bundle(profile, c):
    name = f"Thimsy-test-{DEVICE}-{profile}.tar"
    return {"id": name, "file": name, "size": 2000, "sha256": sha(c)}


# Build steps with text the regex replacement must not mangle: backslashes
# (`\1`, `\g<0>`, `\u` are special in re.sub templates) and non-ASCII.
TRICKY = [
    {"artifact": "@system", "entry": "Super", "op": "lp_write", "partition": "system_a"},
    {"alias": "RG_Cube_Thimsy_OS", "op": "pac_set_info", "version": "Thimsy\\1 \\g<0> \\u00e9 C:\\new\\path"},
    {"op": "pac_set_info", "alias": "Thimsy — Ünïcødé 日本語 😀", "version": "v1"},
    {"op": "pac_fix_crc"},
]


class ApplyPackTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="apply-pack-")
        self.cat = os.path.join(self.tmp, "catalog")
        shutil.copytree(CATALOG, self.cat)
        self.dev_yaml = os.path.join(self.cat, "devices", f"{DEVICE}.yaml")
        self.cat_yaml = os.path.join(self.cat, "catalog.yaml")

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def apply(self, profile, c, ops, with_bundle=True):
        d = os.path.join(self.tmp, profile)
        os.makedirs(d)
        pj = os.path.join(d, "pack.json")
        with open(pj, "w", encoding="utf-8") as f:
            json.dump(pack(profile, c, ops), f, ensure_ascii=False)
        args = [sys.executable, "-I", TOOL]
        if with_bundle:
            bj = os.path.join(d, "bundle.json")
            with open(bj, "w", encoding="utf-8") as f:
                json.dump(bundle(profile, c), f)
            args += ["--bundle", bj]
        args += [pj, self.dev_yaml, self.cat_yaml, profile, "installer-vtest"]
        r = subprocess.run(args, capture_output=True, text=True,
                           env={**os.environ, "LC_ALL": "C", "PYTHONIOENCODING": "ascii"})
        self.assertEqual(r.returncode, 0, r.stderr)

    def read(self, path):
        with open(path, encoding="utf-8") as f:
            return f.read()

    def test_heavy_then_medium_keeps_both(self):
        self.apply("heavy", "1", TRICKY)
        self.apply("medium", "2", TRICKY)
        dev, cat = self.read(self.dev_yaml), self.read(self.cat_yaml)

        exp = flow_to_json(re.search(r"^expect: (\{.*\})$", dev, flags=re.M).group(1))
        self.assertEqual(exp, {"heavy": sha("1"), "medium": sha("2")})

        bundles = top_block(dev, "bundles")
        entries = dict(re.findall(r"^  (\w+): (\{.*\})$", bundles, flags=re.M))
        self.assertEqual(set(entries), {"heavy", "medium"})
        self.assertEqual(flow_to_json(entries["heavy"])["sha256"], sha("1"))
        self.assertEqual(flow_to_json(entries["medium"])["sha256"], sha("2"))
        self.assertEqual(len(re.findall(r"^bundles:", dev, flags=re.M)), 1)

        systems = dict(re.findall(r"^  (\w+): (\S+)$", top_block(cat, "systems"), flags=re.M))
        self.assertEqual(systems, {"heavy": "thimsy-system-test-heavy", "medium": "thimsy-system-test-medium"})
        for p in ("heavy", "medium"):
            self.assertEqual(len(re.findall(rf"^  - id: thimsy-system-test-{p}$", cat, flags=re.M)), 1)
        # The system image already in the catalog is kept too.
        self.assertIn("  - id: thimsy-system-1.0a-heavy\n", cat)

    def test_build_steps_keep_backslashes_and_non_ascii(self):
        self.apply("heavy", "1", TRICKY)
        dev = self.read(self.dev_yaml)
        steps = [flow_to_json(l[4:]) for l in top_block(dev, "build").splitlines()]
        self.assertEqual(steps, TRICKY)
        self.assertIn("Ünïcødé 日本語 😀", dev)  # written as is, not escaped
        # The rest of the device file is untouched.
        self.assertIn('display: "720×720"', dev)
        self.assertIn("flash:\n  method: spd\n", dev)
        self.assertEqual(len(re.findall(r"^build:", dev, flags=re.M)), 1)

    def test_reapplying_a_profile_replaces_only_its_entries(self):
        self.apply("heavy", "1", TRICKY)
        self.apply("medium", "2", TRICKY)
        shutil.rmtree(os.path.join(self.tmp, "heavy"))
        self.apply("heavy", "3", TRICKY)
        dev, cat = self.read(self.dev_yaml), self.read(self.cat_yaml)
        exp = flow_to_json(re.search(r"^expect: (\{.*\})$", dev, flags=re.M).group(1))
        self.assertEqual(exp, {"heavy": sha("3"), "medium": sha("2")})
        self.assertEqual(len(re.findall(r"^  heavy: ", top_block(dev, "bundles"), flags=re.M)), 1)
        self.assertEqual(len(re.findall(r"^  heavy: ", top_block(cat, "systems"), flags=re.M)), 1)

    def test_without_bundle_lists_download_addresses(self):
        self.apply("medium", "2", TRICKY, with_bundle=False)
        dev, cat = self.read(self.dev_yaml), self.read(self.cat_yaml)
        arts = [flow_to_json(l[4:]) for l in top_block(dev, "artifacts").splitlines()]
        self.assertEqual(arts[0]["urls"], [
            "https://github.com/KristopherTadlock/thimsy-os/releases/download/installer-vtest/rgcube-odm-a-delta.bsdiff"])
        # The existing heavy bundle is left alone.
        self.assertIn("  heavy: {", top_block(dev, "bundles"))
        self.assertIn("thimsy-system-test-medium.img.zst\"]", cat)

    def test_device_without_pack_sections_gets_them(self):
        other = os.path.join(self.cat, "devices", "anbernic-rg557.yaml")
        self.dev_yaml = other
        before = self.read(other)
        self.assertIsNone(re.search(r"^(expect|build|bundles):", before, flags=re.M))
        self.apply("medium", "2", TRICKY)
        dev = self.read(other)
        self.assertTrue(dev.startswith(before.split("\nflash:")[0]))
        self.assertEqual(flow_to_json(top_block(dev, "bundles").strip()[len("medium: "):])["sha256"], sha("2"))
        self.assertEqual([flow_to_json(l[4:]) for l in top_block(dev, "build").splitlines()], TRICKY)
        self.assertRegex(dev, r"(?m)^expect:\n  medium: \"2{64}\"\n")


if __name__ == "__main__":
    unittest.main()
