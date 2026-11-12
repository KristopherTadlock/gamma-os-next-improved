"""zero_free_blocks.py and check_image_files.py, on small ext4 images made with mke2fs -d."""
import hashlib
import os
import subprocess

from tests import util

zfb = util.load_script("zero_free_blocks")
cif = util.load_script("check_image_files")

DUMPE2FS = """Filesystem volume name:   <none>
Block count:              64
Free blocks:              12
Block size:               4096

Group 0: (Blocks 0-31)
  Primary superblock at 0, Group descriptors at 1-1
  5 free blocks, 3 free inodes, 2 directories
  Free blocks: 10-12, 20, 30
  Free inodes: 14-16
Group 1: (Blocks 32-63)
  7 free blocks, 0 free inodes, 0 directories
  Free blocks: 40-46
  Free inodes:
Group 2: (Blocks 64-64)
  0 free blocks, 0 free inodes, 0 directories
  Free blocks:
  Free inodes:
"""


class FreeRanges(util.TestCase):
    def test_parse(self):
        self.assertEqual(zfb.free_ranges(DUMPE2FS), (4096, [(10, 12), (20, 20), (30, 30), (40, 46)]))

    def test_count_must_match(self):
        """L2: ranges that don't add up to the superblock's count stop the zeroing."""
        with self.assertRaises(SystemExit) as e:
            zfb.free_ranges(DUMPE2FS.replace("Free blocks:              12", "Free blocks:              13"))
        self.assertIn("the groups list 12 free blocks, the superblock 13", str(e.exception))


class Ext4Case(util.TestCase):
    def setUp(self):
        super().setUp()
        self.bin = util.need(self, "mke2fs", "debugfs", "dumpe2fs", "e2fsck")
        self.env = {"PATH": os.pathsep.join(self.bin + [os.environ["PATH"]])}
        self.root = self.path("root")
        self.files = {"system/bin/setup.sh": b"#!/system/bin/sh\necho hi\n", "system/etc/empty": b"",
                      "system/etc/big.bin": os.urandom(200 << 10), "system/etc/sp ace": b"s",
                      "system/etc/back\\slash": b"b"}
        for rel, data in self.files.items():
            self.write("root/" + rel, data)
        self.image = self.path("system.img")
        self.tool(["mke2fs", "-q", "-F", "-t", "ext4", "-b", "4096", "-I", "256", "-d", self.root, self.image, "8M"])

    def tool(self, cmd):
        return self.assertOk(util.run(cmd, env=self.env))

    def listing(self, names=None):
        """sha256sum output for the tree, as build.sh takes it in the mounted image."""
        names = sorted(self.files) if names is None else names
        out = subprocess.run(["sha256sum", "--", *["./" + n for n in names]], cwd=self.root,
                             capture_output=True, text=True, check=True).stdout
        return self.write("written.sha256", out)


class ZeroFreeBlocks(Ext4Case):
    def test_zeroes_only_free_blocks(self):
        dump = util.run(["dumpe2fs", self.image], env=dict(self.env, LC_ALL="C")).stdout
        block, ranges = zfb.free_ranges(dump)
        with open(self.image, "r+b") as f:
            for a, b in ranges:
                f.seek(a * block)
                f.write(b"\xaa" * ((b - a + 1) * block))
        self.tool(["e2fsck", "-fn", self.image])
        out = util.run_script("zero_free_blocks", self.image, env=self.env)
        self.assertOk(out)
        self.assertIn("zeroed %d free blocks" % sum(b - a + 1 for a, b in ranges), out.stdout)
        with open(self.image, "rb") as f:
            for a, b in ranges:
                f.seek(a * block)
                self.assertEqual(f.read((b - a + 1) * block).count(0), (b - a + 1) * block)
        self.tool(["e2fsck", "-fn", self.image])
        self.assertOk(util.run_script("check_image_files", self.image, self.listing(), env=self.env))

    def test_usage(self):
        self.assertFails(util.run_script("zero_free_blocks"), "zero_free_blocks.py IMAGE")


class CheckImageFiles(Ext4Case):
    def check(self, *args):
        return util.run_script("check_image_files", self.image, *args, env=self.env)

    def test_intact(self):
        proc = self.assertOk(self.check(self.listing()))
        self.assertIn("5 files the mods wrote read back intact", proc.stdout)
        # sha256sum escapes the backslash name; it is read back all the same.
        self.assertIn("\\" + hashlib.sha256(b"b").hexdigest(), util.slurp(self.listing(), text=True))

    def test_changed_file(self):
        self.write("root/system/bin/setup.sh", "changed after packing\n")
        self.assertFails(self.check(self.listing()), "1 of 5 files the mods wrote are not in the image",
                         "/system/bin/setup.sh")

    def test_missing_file(self):
        """L1: an empty file the image lacks used to pass (debugfs printed nothing, and exited 0)."""
        self.write("root/system/etc/new-empty", b"")
        proc = self.check(self.listing(["system/etc/new-empty"]))
        self.assertFails(proc, "/system/etc/new-empty (/system/etc/new-empty: File not found by ext2_lookup")

    def test_deleted_files(self):
        deleted = self.write("deleted.list", b"./system/etc/gone\0./system/etc/also gone\0")
        proc = self.assertOk(self.check(self.listing(), deleted))
        self.assertIn("2 deleted files are gone", proc.stdout)
        self.write("deleted.list", b"./system/etc/gone\0./system/etc/sp ace\0")
        self.assertFails(self.check(self.listing(), deleted), "1 of 2 files the mods deleted are still in the image",
                         "/system/etc/sp ace")

    def test_paths_must_exist(self):
        self.assertFails(self.check(self.path("nope.sha256")), "no file")
        listing = self.listing()
        self.assertFails(util.run_script("check_image_files", self.path("nope.img"), listing, env=self.env), "no file")
        self.assertFails(self.check(listing, self.path("nope.list")), "no file")

    def test_unreadable_lines(self):
        for line in ['%s  ./system/etc/"quoted"\n' % ("0" * 64), "%s  system/etc/x\n" % ("0" * 64),
                     "garbage\n", "\\%s  ./a\\qb\n" % ("0" * 64)]:
            self.assertFails(self.check(self.write("bad.sha256", line)), "can't check")

    def test_parse_sha256sum(self):
        digest = "ab" * 32
        self.assertEqual(cif.parse_sha256sum(f"{digest}  ./a b\n"), (digest, "./a b"))
        self.assertEqual(cif.parse_sha256sum(f"\\{digest}  ./a\\\\b\\nc\n"), (digest, "./a\\b\nc"))
        self.assertEqual(cif.parse_sha256sum(f"\\{digest}  ./a\\x\n"), (None, None))
        self.assertEqual(cif.parse_sha256sum(f"{digest} ./single-space\n"), (None, None))
