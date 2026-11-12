import os
import shutil
import struct
import subprocess
import tempfile

from tests import util
from tests.fixtures import pac

pactool = util.load_script("pactool")


def build_crc_tool(testcase):
    """Compiles tools/crc16arc.c into a scratch folder; returns its path."""
    cc = shutil.which("cc")
    if not cc:
        testcase.skipTest("no C compiler")
    d = tempfile.mkdtemp(prefix="thimsy-crc-")
    testcase.addCleanup(util.rmtree, d)
    out = os.path.join(d, "crc16arc")
    subprocess.run([cc, "-O2", "-Wall", "-Werror", "-o", out, os.path.join(util.ROOT, "tools/crc16arc.c")],
                   check=True)
    return out


class Crc16Arc(util.TestCase):
    def test_check_value_python(self):
        self.assertEqual(pactool.crc16arc(b"123456789"), 0xBB3D)
        self.assertEqual(pac.crc16arc(b"123456789"), 0xBB3D)

    def test_check_value_c(self):
        tool = build_crc_tool(self)
        out = util.run([tool], input="123456789")
        self.assertEqual(self.assertOk(out).stdout.strip(), "bb3d")
        p = self.write("f", b"xx123456789")
        self.assertEqual(self.assertOk(util.run([tool, p, "2"])).stdout.strip(), "bb3d")

    def test_c_reports_read_errors(self):
        tool = build_crc_tool(self)
        self.assertFails(util.run([tool, self.tmp, "0"]))
        self.assertFails(util.run([tool, self.path("missing")]), "missing")


class PacTool(util.TestCase):
    def setUp(self):
        super().setUp()
        self.pac = self.path("t.pac")
        self.layout = pac.build(self.pac)
        self.env = {"CRC16ARC": build_crc_tool(self)}

    def tool(self, *args):
        return util.run_script("pactool", *args, env=self.env)

    def read_entry(self, pid):
        off, size = self.layout[pid]
        with open(self.pac, "rb") as f:
            f.seek(off)
            return f.read(size)

    def test_list(self):
        out = self.assertOk(self.tool("list", self.pac)).stdout
        self.assertIn("version: Old_v1", out)
        self.assertIn("alias: OldAlias", out)
        self.assertRegex(out, r"Super\s+super_full.img\s+5000")
        self.assertRegex(out, r"EraseX\s+0")

    def test_extract_by_file_and_id(self):
        for name in ("super_full.img", "Super"):
            out = self.path(name + ".out")
            self.assertOk(self.tool("extract", self.pac, name, out))
            self.assertEqual(util.slurp(out), b"S" * 5000)

    def test_extract_makes_a_new_file(self):
        # A loop device (or any other holder) of the old file must not see the new contents.
        out = self.write("old.img", b"old contents")
        os.link(out, self.path("holder"))
        self.assertOk(self.tool("extract", self.pac, "fdl.bin", out))
        self.assertEqual(util.slurp(out), b"F" * 1000)
        self.assertEqual(util.slurp(self.path("holder")), b"old contents")

    def test_extract_errors(self):
        self.assertFails(self.tool("extract", self.pac, "nothere.img", self.path("o")), "no file 'nothere.img'")
        # An entry without data (an erase step) has nothing to extract.
        self.assertFails(self.tool("extract", self.pac, "EraseX", self.path("o")), "no file 'EraseX'")

    def test_replace(self):
        before = self.read_entry("FDL"), self.read_entry("VENDORBOOT")
        img = self.write("new.img", b"N" * 5000)
        self.assertOk(self.tool("replace", self.pac, "super_full.img", img))
        self.assertEqual(self.read_entry("Super"), b"N" * 5000)
        self.assertEqual((self.read_entry("FDL"), self.read_entry("VENDORBOOT")), before)
        self.assertEqual(os.path.getsize(self.pac), self.layout["VENDORBOOT"][0] + 300)

    def test_replace_size_must_match(self):
        before = util.slurp(self.pac)
        for size in (4999, 5001):
            img = self.write("bad.img", b"N" * size)
            self.assertFails(self.tool("replace", self.pac, "Super", img), f"is {size} bytes; Super must be 5000")
        self.assertEqual(util.slurp(self.pac), before)

    def test_bad_version(self):
        pac.build(self.pac, version="BP_R1.0.0")
        self.assertFails(self.tool("list", self.pac), "unsupported pac version 'BP_R1.0.0'")

    def test_truncated(self):
        with open(self.pac, "r+b") as f:
            f.truncate(self.layout["VENDORBOOT"][0] + 10)
        self.assertFails(self.tool("list", self.pac), "vendor_boot.img ends past the end of the file")
        with open(self.pac, "r+b") as f:
            f.truncate(pac.HEADER_SIZE + 100)
        self.assertFails(self.tool("list", self.pac), "pac entry table is truncated")

    def test_usage(self):
        self.assertFails(util.run_script("pactool"), "pactool.py list PAC")
        self.assertFails(self.tool("frobnicate", self.pac), "pactool.py list PAC")

    def test_setinfo(self):
        data_before = util.slurp(self.pac)[pac.HEADER_SIZE:]
        crc2 = pac.crcs(self.pac)[1]
        self.assertOk(self.tool("setinfo", self.pac, "Thimsy_OS_v1.0", "RG_Cube_Thimsy_OS"))
        out = self.assertOk(self.tool("list", self.pac)).stdout
        self.assertIn("version: Thimsy_OS_v1.0", out)
        self.assertIn("alias: RG_Cube_Thimsy_OS", out)
        hdr = pac.header(self.pac)
        crc1, new_crc2 = pac.crcs(self.pac)
        self.assertEqual(crc1, pac.crc16arc(hdr[:pac.HEADER_SIZE - 4]))
        self.assertEqual(new_crc2, crc2)
        self.assertEqual(util.slurp(self.pac)[pac.HEADER_SIZE:], data_before)
        # A shorter value leaves nothing of the old one behind.
        self.assertOk(self.tool("setinfo", self.pac, "v2", "a"))
        self.assertNotEqual(hdr[564 + 4:564 + 512], bytes(508))
        self.assertEqual(pac.header(self.pac)[564 + 4:564 + 512], bytes(508))

    def test_setinfo_too_long(self):
        before = util.slurp(self.pac)
        self.assertFails(self.tool("setinfo", self.pac, "v" * 256, "a"), "longer than 255 characters")
        self.assertFails(self.tool("setinfo", self.pac, "v", "a" * 100), "longer than 99 characters")
        self.assertEqual(util.slurp(self.pac), before)

    def test_fixcrc_and_verify(self):
        self.assertOk(self.tool("verify", self.pac))
        img = self.write("new.img", b"N" * 5000)
        self.assertOk(self.tool("replace", self.pac, "Super", img))
        self.assertFails(self.tool("verify", self.pac), "data crc")
        out = self.assertOk(self.tool("fixcrc", self.pac)).stdout
        want = pac.crc16arc(util.slurp(self.pac)[pac.HEADER_SIZE:])
        self.assertEqual(out.strip(), f"data crc {want:04x}")
        self.assertEqual(pac.crcs(self.pac)[1], want)
        self.assertOk(self.tool("verify", self.pac))
        self.assertOk(self.tool("setinfo", self.pac, "Thimsy", "Alias"))
        self.assertOk(self.tool("verify", self.pac))

    def test_verify_header_crc(self):
        with open(self.pac, "r+b") as f:
            f.seek(600)
            f.write(b"X")
        self.assertFails(self.tool("verify", self.pac), "header crc")

    def test_verify_needs_the_crc_tool(self):
        self.env["CRC16ARC"] = self.path("no-such-tool")
        self.assertFails(self.tool("verify", self.pac))


@util.slow
class PacToolPast4GiB(util.TestCase):
    """64-bit sizes and offsets: Super past 4 GiB in a sparse file."""

    def test_far_entry(self):
        p = self.path("big.pac")
        far = (4 << 30) + 123
        layout = pac.build(p, far_offset=far, with_crcs=False)
        self.assertEqual(layout["Super"][0], far)
        self.assertGreater(os.path.getsize(p), 4 << 30)
        out = self.path("super.img")
        self.assertOk(util.run_script("pactool", "extract", p, "Super", out))
        self.assertEqual(util.slurp(out), b"S" * 5000)
        self.assertOk(util.run_script("pactool", "replace", p, "Super", self.write("n", b"N" * 5000)))
        with open(p, "rb") as f:
            f.seek(far)
            self.assertEqual(f.read(5000), b"N" * 5000)
            f.seek(layout["VENDORBOOT"][0])
            self.assertEqual(f.read(300), b"V" * 300)
        with open(p, "rb") as f:
            entries = pactool.read_entries(f)
        self.assertEqual([e["offset"] for e in entries if e["id"] == "Super"], [far])
        # The table entry's high words hold the offset's top bits.
        with open(p, "rb") as f:
            f.seek(pac.HEADER_SIZE + pac.ENTRY_SIZE)
            raw = f.read(pac.ENTRY_SIZE)
        self.assertEqual(struct.unpack_from("<II", raw, 1532), (0, 1))
