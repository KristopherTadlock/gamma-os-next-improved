import gzip
import os
import random
import struct

from tests import util
from tests.fixtures import bootlogo as logo


class BootLogo(util.TestCase):
    SIZE = 6

    def setUp(self):
        super().setUp()
        self.logo = self.path("logo.img")
        self.pics = logo.build(self.logo, size=self.SIZE)
        self.out = self.path("logo.new")

    def frame(self, pixels=None):
        """A SIZExSIZE RGBA frame, black but for PIXELS {(row, col): (r, g, b)}."""
        f = bytearray(self.SIZE * self.SIZE * 4)
        for (r, c), rgb in (pixels or {}).items():
            p = (r * self.SIZE + c) * 4
            f[p:p + 4] = bytes(rgb) + b"\xff"
        return self.write("frame.rgba", bytes(f))

    def stored_pixels(self, bmp_data):
        """{(row from the top, col): (r, g, b)} of the BMP's non-black pixels."""
        w, h = struct.unpack_from("<ii", bmp_data, 18)
        row = (w * 3 + 3) & ~3
        out = {}
        for y in range(h):
            base = 54 + (h - 1 - y) * row
            for x in range(w):
                b, g, r = bmp_data[base + 3 * x:base + 3 * x + 3]
                if (r, g, b) != (0, 0, 0):
                    out[(y, x)] = (r, g, b)
        return out

    def test_only_the_boot_picture_changes(self):
        self.assertOk(util.run_script("bootlogo", self.logo, self.frame(), self.out))
        new = logo.read(self.out)
        self.assertEqual(len(new), 5)
        self.assertEqual(new[:3], self.pics[:3])
        self.assertEqual(new[4], self.pics[4])
        self.assertNotEqual(new[3], self.pics[3])
        self.assertEqual(os.path.getsize(self.out), os.path.getsize(self.logo))
        # The gzip member keeps its name, as the stock ones have.
        self.assertTrue(new[3][3] & 8)
        self.assertEqual(new[3][10:16], b"4.bmp\0")

    def test_rotation_and_layout(self):
        red, green = (255, 0, 0), (0, 255, 0)
        s = self.SIZE
        # The top-left pixel goes to the top-right, a quarter turn clockwise; the bottom-left one
        # to the top-left.
        self.assertOk(util.run_script("bootlogo", self.logo, self.frame({(0, 0): red, (s - 1, 0): green}),
                                      self.out))
        data = gzip.decompress(logo.read(self.out)[3])
        self.assertEqual(self.stored_pixels(data), {(0, s - 1): red, (0, 0): green})
        old = gzip.decompress(self.pics[3])
        self.assertEqual(len(data), len(old))
        self.assertEqual(data[:54], old[:54])
        self.assertEqual(data[-2:], b"\0\0")

    def test_errors(self):
        self.assertFails(util.run_script("bootlogo", self.write("bad", b"XX" + bytes(100)), self.frame(), self.out),
                         "not a GZ logo partition")
        self.assertFails(util.run_script("bootlogo", self.logo, self.write("f", bytes(10)), self.out),
                         "the frame must be 6x6 RGBA")
        logo.build(self.path("three.img"), size=self.SIZE, count=3)
        self.assertFails(util.run_script("bootlogo", self.path("three.img"), self.frame(), self.out),
                         "unexpected logo partition layout")
        self.assertFails(util.run_script("bootlogo", self.logo), "bootlogo.py LOGO_IN")

    def test_must_fit(self):
        logo.build(self.logo, size=32, padding=0)
        rnd = random.Random(1)
        frame = self.write("noise.rgba", bytes(rnd.getrandbits(8) for _ in range(32 * 32 * 4)))
        self.assertFails(util.run_script("bootlogo", self.logo, frame, self.out), "do not fit the partition")
        self.assertFalse(os.path.exists(self.out))
