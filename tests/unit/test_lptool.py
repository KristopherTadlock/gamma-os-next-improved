import hashlib
import os
import struct

from tests import util
from tests.fixtures import lp

lptool = util.load_script("lptool")
MiB = 1 << 20


def meta(path):
    with open(path, "rb") as f:
        return lptool.read_metadata(f)


class LpTool(util.TestCase):
    def setUp(self):
        super().setUp()
        self.super = self.path("super.img")
        self.layout = lp.build(self.super)

    def tool(self, *args):
        return util.run_script("lptool", *args)

    def region(self, offset, size):
        with open(self.super, "rb") as f:
            f.seek(offset)
            return f.read(size)

    def test_list(self):
        out = self.assertOk(self.tool("list", self.super)).stdout
        self.assertRegex(out, r"system_a\s+1048576\s+extents=1")
        self.assertRegex(out, r"vendor_a\s+1048576\s+extents=1")
        self.assertRegex(out, r"system_b\s+0\s+extents=0")

    def test_extract(self):
        out = self.path("system.img")
        self.assertOk(self.tool("extract", self.super, "system_a", out))
        self.assertEqual(util.slurp(out), b"S" * MiB)

    def test_extract_makes_a_new_file(self):
        out = self.write("system.img", b"old")
        os.link(out, self.path("holder"))
        self.assertOk(self.tool("extract", self.super, "vendor_a", out))
        self.assertEqual(util.slurp(out), b"V" * MiB)
        self.assertEqual(util.slurp(self.path("holder")), b"old")

    def test_extract_unknown(self):
        self.assertFails(self.tool("extract", self.super, "odm_a", self.path("o")),
                         "no partition 'odm_a'; have system_a, vendor_a, system_b")
        self.assertFalse(os.path.exists(self.path("o")))

    def test_replace_pads_with_zeros(self):
        before_meta = lp.metadata_slots(self.super)
        img = self.write("new.img", b"N" * 1000)
        self.assertOk(self.tool("replace", self.super, "system_a", img))
        off, size = self.layout["system_a"]
        self.assertEqual(self.region(off, size), b"N" * 1000 + bytes(size - 1000))
        self.assertEqual(self.region(*self.layout["vendor_a"]), b"V" * MiB)
        self.assertEqual(lp.metadata_slots(self.super), before_meta)

    def test_replace_too_big(self):
        before = util.slurp(self.super)
        img = self.write("new.img", b"N" * (MiB + 1))
        self.assertFails(self.tool("replace", self.super, "system_a", img), "system_a holds 1048576")
        self.assertEqual(util.slurp(self.super), before)

    def test_grow_max_is_capped_by_the_group(self):
        out = self.assertOk(self.tool("grow", self.super, "system_a", "max")).stdout
        # main_a may hold 8 MiB, and system_a and vendor_a already take 2.
        self.assertIn(f"grew system_a by {6 * MiB} bytes at offset {3 * MiB}", out)
        m = meta(self.super)
        parts = {lptool.name_of(p[0]): p for p in m["partitions"]}
        self.assertEqual(parts["system_a"][2:4], [0, 2])
        # The partitions after it point at their extents, now one further on.
        self.assertEqual(parts["vendor_a"][2:4], [2, 1])
        self.assertEqual(parts["system_b"][2:4], [3, 0])
        self.assertEqual(m["extents"][1], [6 * MiB // 512, 0, 3 * MiB // 512, 0])
        self.assertEqual(m["extents"][2][2], 2 * MiB // 512)
        # Every metadata copy is rewritten the same, with valid checksums.
        slots = lp.metadata_slots(self.super)
        self.assertEqual(len(set(slots)), 1)
        header_size, = struct.unpack_from("<I", slots[0], 8)
        header = bytearray(slots[0][:header_size])
        tables_size, = struct.unpack_from("<I", header, 44)
        self.assertEqual(hashlib.sha256(slots[0][header_size:header_size + tables_size]).digest(),
                         bytes(header[48:80]))
        stored = bytes(header[12:44])
        header[12:44] = bytes(32)
        self.assertEqual(hashlib.sha256(header).digest(), stored)
        # The grown partition is its old data, then the new extent.
        out = self.path("system.img")
        self.assertOk(self.tool("extract", self.super, "system_a", out))
        self.assertEqual(os.path.getsize(out), 7 * MiB)
        self.assertEqual(util.slurp(out)[:MiB], b"S" * MiB)
        self.assertOk(self.tool("extract", self.super, "vendor_a", out))
        self.assertEqual(util.slurp(out), b"V" * MiB)

    def test_grow_by_bytes_is_aligned(self):
        out = self.assertOk(self.tool("grow", self.super, "vendor_a", str(2 * MiB + 5000))).stdout
        self.assertIn(f"grew vendor_a by {2 * MiB} bytes", out)

    def test_grow_by_free_space(self):
        lp.build(self.super, group_max=0)
        out = self.assertOk(self.tool("grow", self.super, "system_a", "max")).stdout
        self.assertIn(f"grew system_a by {13 * MiB} bytes", out)

    def test_grow_with_no_room(self):
        lp.build(self.super, group_max=2 * MiB)
        before = util.slurp(self.super)
        self.assertFails(self.tool("grow", self.super, "system_a", "max"), "no free space to grow system_a")
        self.assertEqual(util.slurp(self.super), before)
        self.assertFails(self.tool("grow", self.super, "nope", "max"), "no partition 'nope'")

    def test_checksums_are_checked(self):
        with open(self.super, "r+b") as f:
            f.seek(lp.METADATA_OFFSET + 128 + 3)
            f.write(b"X")
        self.assertFails(self.tool("list", self.super), "metadata tables checksum mismatch")
        lp.build(self.super)
        with open(self.super, "r+b") as f:
            f.seek(lp.METADATA_OFFSET + 50)
            f.write(b"X")
        self.assertFails(self.tool("list", self.super), "metadata header checksum mismatch")

    def test_not_a_super_image(self):
        p = self.write("x.img", bytes(64 << 10))
        self.assertFails(self.tool("list", p), "bad geometry magic")
        self.assertFails(util.run_script("lptool"), "lptool.py list SUPER")
        self.assertFails(self.tool("frob", self.super), "lptool.py list SUPER")

    def test_non_linear_extent(self):
        with open(self.super, "r+b") as f:
            m = lptool.read_metadata(f)
            m["extents"][0][1] = 1
            lptool.write_metadata(f, m)
        self.assertFails(self.tool("extract", self.super, "system_a", self.path("o")), "non-linear extents")
