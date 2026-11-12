"""The packing half of build.sh on small images: a tree becomes an ext4 image (mke2fs -d), its free
blocks are zeroed and its files checked, it goes into a super image (lptool) and the super image
into a PAC (pactool), and every level is read back and compared."""
import os
import subprocess

from tests import util
from tests.fixtures import lp, pac
from tests.unit.test_pactool import build_crc_tool

MiB = 1 << 20


class Packing(util.TestCase):
    def setUp(self):
        super().setUp()
        self.bin = util.need(self, "mke2fs", "debugfs", "dumpe2fs", "e2fsck")
        self.env = {"PATH": os.pathsep.join(self.bin + [os.environ["PATH"]]), "CRC16ARC": build_crc_tool(self),
                    "LC_ALL": "C"}

    def ok(self, cmd):
        return self.assertOk(util.run(cmd, env=self.env))

    def script(self, name, *args):
        return self.assertOk(util.run_script(name, *args, env=self.env))

    def test_pipeline(self):
        files = {"system/build.prop": b"ro.thimsy.version=1.0\n", "system/bin/setup.sh": b"#!/system/bin/sh\n",
                 "system/etc/payload.tar.zst": os.urandom(300 << 10), "system/etc/empty": b"",
                 "system/etc/sub dir/name with spaces": b"spaces"}
        for rel, data in files.items():
            self.write("tree/" + rel, data)
        image = self.path("system_a.img")
        self.ok(["mke2fs", "-q", "-F", "-t", "ext4", "-b", "4096", "-d", self.path("tree"), image, "3M"])

        # Old contents in the free blocks, as deleted files leave them.
        _, ranges = util.load_script("zero_free_blocks").free_ranges(self.ok(["dumpe2fs", image]).stdout)
        with open(image, "r+b") as f:
            for a, b in ranges:
                f.seek(a * 4096)
                f.write(b"\xee" * ((b - a + 1) * 4096))
        self.script("zero_free_blocks", image)
        self.ok(["e2fsck", "-fn", image])
        listing = subprocess.run(["sha256sum", "--", *["./" + r for r in sorted(files)]], cwd=self.path("tree"),
                                 capture_output=True, text=True, check=True).stdout
        deleted = self.write("deleted.list", b"./system/etc/removed.apk\0")
        self.script("check_image_files", image, self.write("written.sha256", listing), deleted)

        # The super image: system_a grows into the free space, and takes the image.
        sup = self.path("super_full.img")
        lp.build(sup, partitions=[("system_a", 1, 1, b"S"), ("vendor_a", 1, 1, b"V")], group_max=6 * MiB,
                 device_size=8 * MiB)
        vendor_before = self.extract_partition(sup, "vendor_a")
        self.script("lptool", "grow", sup, "system_a", "max")
        old_system = self.extract_partition(sup, "system_a")
        self.assertEqual(len(old_system), 5 * MiB)
        self.script("lptool", "replace", sup, "system_a", image)
        self.assertEqual(self.extract_partition(sup, "system_a"),
                         util.slurp(image) + bytes(5 * MiB - os.path.getsize(image)))
        self.assertEqual(self.extract_partition(sup, "vendor_a"), vendor_before)

        # The PAC: the stock one with the new super image, its CRCs and its name.
        stock_super = self.write("stock_super.img", b"\x11" * os.path.getsize(sup))
        p = self.path("out.pac")
        pac.build(p, [("FDL", "fdl.bin", b"F" * 4000), ("Super", "super_full.img", util.slurp(stock_super)),
                      ("VENDORBOOT", "vendor_boot.img", b"B" * 777)], with_crcs=False)
        self.script("pactool", "replace", p, "super_full.img", sup)
        self.script("pactool", "fixcrc", p)
        self.script("pactool", "setinfo", p, "Thimsy_OS_vTEST", "RG_Cube_Thimsy_OS")
        self.script("pactool", "verify", p)
        listing_out = self.script("pactool", "list", p).stdout
        self.assertIn("version: Thimsy_OS_vTEST", listing_out)

        # Back out again, level by level.
        sup2 = self.path("super2.img")
        self.script("pactool", "extract", p, "Super", sup2)
        self.assertEqual(util.slurp(sup2), util.slurp(sup))
        fdl = self.path("fdl.bin")
        self.script("pactool", "extract", p, "fdl.bin", fdl)
        self.assertEqual(util.slurp(fdl), b"F" * 4000)
        system2 = self.path("system2.img")
        self.script("lptool", "extract", sup2, "system_a", system2)
        self.assertEqual(util.slurp(system2)[:os.path.getsize(image)], util.slurp(image))
        self.ok(["e2fsck", "-fn", system2])
        self.script("check_image_files", system2, self.path("written.sha256"), deleted)
        for rel, data in files.items():
            self.assertEqual(self.cat(system2, rel), data, rel)

    def cat(self, image, rel):
        proc = subprocess.run(["debugfs", "-R", f'cat "/{rel}"', image], capture_output=True,
                              env=dict(os.environ, **self.env), check=True)
        return proc.stdout

    def extract_partition(self, sup, name):
        out = self.path(name + ".out")
        self.script("lptool", "extract", sup, name, out)
        return util.slurp(out)
