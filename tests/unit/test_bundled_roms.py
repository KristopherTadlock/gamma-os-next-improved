import contextlib
import hashlib
import io
import json
import os
import shutil
import tarfile
import zipfile

from tests import util

br = util.load_script("bundled_roms")
PNG = b"\x89PNG\r\n\x1a\n" + b"png data"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def exits(testcase, func, *args):
    """Runs FUNC(*ARGS), which must sys.exit(); returns its message."""
    with testcase.assertRaises(SystemExit) as e, contextlib.redirect_stdout(io.StringIO()):
        func(*args)
    return str(e.exception)


class Parse(util.TestCase):
    def test_entries(self):
        p = self.write("list.txt", "\n".join([
            "# comment", "",
            "upstream  gb/Up.gb  license=CC0",
            "add  nes/Game.nes  license=MIT  url=https://example.org/g.nes  sha256=" + "a" * 64,
            "tree  pico8/  license=CC0  url=https://example.org/t.zip  sha256=" + "b" * 64 + "  member=carts",
            "local  scummvm/G.scummvm  license=GPL",
            "folder  n64",
        ]) + "\n")
        entries = br.parse(p)
        self.assertEqual([(k, t) for k, t, _ in entries],
                         [("upstream", "gb/Up.gb"), ("add", "nes/Game.nes"), ("tree", "pico8/"),
                          ("local", "scummvm/G.scummvm"), ("folder", "n64")])
        self.assertEqual(entries[2][2]["member"], "carts")

    def test_errors(self):
        for line, message in [
            ("upstream  gb/Up.gb", "can't parse"),
            ("bogus  x  license=CC0", "can't parse"),
            ("add  x  license=CC0  url=u", "add needs url= and sha256="),
            ("tree  x/  license=CC0  url=u  sha256=s", "tree needs member="),
        ]:
            p = self.write("list.txt", line + "\n")
            self.assertIn(message, exits(self, br.parse, p))


class Fetch(util.TestCase):
    def setUp(self):
        super().setUp()
        if not shutil.which("curl"):
            self.skipTest("no curl")
        self.cache = self.path("cache")
        os.makedirs(self.cache)

    def url(self, rel):
        return "file://" + self.path(rel)

    def test_fetch_and_cache(self):
        self.write("dl/game.bin", b"the real rom")
        fields = {"url": self.url("dl/game.bin"), "sha256": sha(b"the real rom")}
        self.assertEqual(br.fetch("gb/x.gb", fields, self.cache), b"the real rom")
        os.remove(self.path("dl/game.bin"))
        # Served from the cache by its sha256 now.
        self.assertEqual(br.fetch("gb/x.gb", fields, self.cache), b"the real rom")

    def test_bad_download_is_not_kept(self):
        """M4: a download that fails its checksum is deleted, so the next build fetches it again."""
        self.write("dl/game.bin", b"an error page")
        fields = {"url": self.url("dl/game.bin"), "sha256": sha(b"the real rom")}
        self.assertIn("(download deleted from the cache)", exits(self, br.fetch, "gb/x.gb", fields, self.cache))
        self.assertEqual(os.listdir(self.cache), [])
        self.write("dl/game.bin", b"the real rom")
        self.assertEqual(br.fetch("gb/x.gb", fields, self.cache), b"the real rom")

    def test_member_of_archives(self):
        with zipfile.ZipFile(self.path("dl.zip"), "w") as z:
            z.writestr("pkg/sub/game.nes", b"zipped rom")
        fields = {"url": self.url("dl.zip"), "sha256": sha(b"zipped rom"), "member": "pkg/sub/game.nes"}
        self.assertEqual(br.fetch("nes/g.nes", fields, self.cache), b"zipped rom")
        with tarfile.open(self.path("dl.tar.gz"), "w:gz") as t:
            info = tarfile.TarInfo("top/dir/game.sms")
            info.size = 9
            t.addfile(info, io.BytesIO(b"tared rom"))
        fields = {"url": self.url("dl.tar.gz"), "sha256": sha(b"tared rom"), "member": "dir/game.sms"}
        self.assertEqual(br.fetch("sms/g.sms", fields, self.cache), b"tared rom")
        # A member that fails its checksum takes its archive download with it.
        fields = {"url": self.url("dl.zip"), "sha256": "0" * 64, "member": "pkg/sub/game.nes"}
        exits(self, br.fetch, "nes/g.nes", fields, self.cache)
        self.assertFalse([n for n in os.listdir(self.cache) if n.startswith("download-")
                          and util.sha256_file(os.path.join(self.cache, n)) == util.sha256_file(self.path("dl.zip"))])

    def test_tree(self):
        with zipfile.ZipFile(self.path("t.zip"), "w") as z:
            z.writestr("repo/carts/a.p8.png", b"A")
            z.writestr("repo/carts/sub/b.p8.png", b"B")
            z.writestr("repo/carts/sub/", b"")
            z.writestr("repo/other.txt", b"O")
        good = util.sha256_file(self.path("t.zip"))
        files = br.fetch_tree("pico8/", {"url": self.url("t.zip"), "sha256": good, "member": "repo/carts/"}, self.cache)
        self.assertEqual(files, {"pico8/a.p8.png": b"A", "pico8/sub/b.p8.png": b"B"})
        missing = {"url": self.url("t.zip"), "sha256": good, "member": "repo/none"}
        self.assertIn("nothing under repo/none/", exits(self, br.fetch_tree, "pico8/", missing, self.cache))

    def test_bad_tree_archive_is_not_kept(self):
        with zipfile.ZipFile(self.path("t.zip"), "w") as z:
            z.writestr("repo/carts/a.p8.png", b"A")
        fields = {"url": self.url("t.zip"), "sha256": "0" * 64, "member": "repo/carts"}
        self.assertIn("archive sha256 doesn't match", exits(self, br.fetch_tree, "pico8/", fields, self.cache))
        self.assertEqual(os.listdir(self.cache), [])


class Media(util.TestCase):
    def test_parse_media(self):
        p = self.write("media.txt", "\n".join([
            "[gb/Game.gb]", "title = Game", "released = 2001", "cover = file=gb/cover.png  license=CC0",
            "screenshot = url=https://example.org/s.png  sha256=" + "c" * 64 + "  license=CC0"]) + "\n")
        media = br.parse_media(p, {"gb/Game.gb"})
        self.assertEqual(media["gb/Game.gb"]["title"], "Game")
        self.assertEqual(media["gb/Game.gb"]["cover"], {"file": "gb/cover.png", "license": "CC0"})
        for text, message in [("[gb/Other.gb]\n", "isn't a game in the bundled list"),
                              ("title = orphan\n", "can't parse"),
                              ("[gb/Game.gb]\nbogus = 1\n", "can't parse"),
                              ("[gb/Game.gb]\ncover = file=x.png\n", "needs license=")]:
            self.write("media.txt", text)
            self.assertIn(message, exits(self, br.parse_media, p, {"gb/Game.gb"}))

    def test_image_ext(self):
        self.assertEqual(br.image_ext("t", PNG), ".png")
        self.assertEqual(br.image_ext("t", b"\xff\xd8\xff\xe0jpeg"), ".jpg")
        self.assertIn("not a PNG or JPEG", exits(self, br.image_ext, "t", b"GIF89a"))


class Main(util.TestCase):
    def setUp(self):
        super().setUp()
        if not shutil.which("curl"):
            self.skipTest("no curl")
        self.in_tar = self.path("in.tar")
        with tarfile.open(self.in_tar, "w", format=tarfile.GNU_FORMAT) as t:
            for name in ("sdcard/ROMs", "sdcard/ROMs/gb", "sdcard/ROMs/nes"):
                info = tarfile.TarInfo(name)
                info.type, info.mode, info.uid, info.gid = tarfile.DIRTYPE, 0o2770, 1023, 1023
                t.addfile(info)
            for name, data in (("sdcard/ROMs/gb/Up.gb", b"UP"), ("sdcard/ROMs/gb/Gone.gb", b"GONE"),
                               ("sdcard/ROMs/gb/systeminfo.txt", b"info")):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.uid, info.gid = len(data), 0o660, 1023, 1023
                t.addfile(info, io.BytesIO(data))
        self.write("dl/new.nes", b"NEW")
        self.write("licenses/gb/Up.gb.license.txt", "up license")
        self.write("licenses/nes/LICENSE.txt", "nes license")
        self.write("licenses/scummvm/G.scummvm.license.txt", "g license")
        self.write("local/scummvm/G.scummvm", "sky")
        self.write("pack-in-art/gb/cover.png", PNG)
        self.list = self.write("list.txt", "\n".join([
            "upstream  gb/Up.gb  license=CC0",
            f"add  nes/New.nes  license=MIT  url=file://{self.path('dl/new.nes')}  sha256={sha(b'NEW')}",
            "local  scummvm/G.scummvm  license=GPL",
            "folder  n64/sub"]) + "\n")
        self.media = self.write("pack-in-media.txt", "[gb/Up.gb]\ntitle = Up & Away\nreleased = 1999-05-01\n"
                                "cover = file=gb/cover.png  license=CC0\n")

    def build(self, *extra):
        out = self.path("out.tar")
        proc = util.run_script("bundled_roms", self.list, self.in_tar, out, self.path("cache"),
                               self.path("licenses"), self.path("local"), *extra)
        return proc, out

    def test_rebuilds_the_pack(self):
        proc, out = self.build(self.media, "nano")
        self.assertOk(proc)
        self.assertIn("removed sdcard/ROMs/gb/Gone.gb".replace("sdcard/ROMs/", ""), proc.stdout)
        with tarfile.open(out) as t:
            members = {m.name: m for m in t}
            data = {m.name: t.extractfile(m).read() for m in t if m.isfile()}
        self.assertEqual(data["sdcard/ROMs/gb/Up.gb"], b"UP")
        self.assertEqual(data["sdcard/ROMs/nes/New.nes"], b"NEW")
        self.assertEqual(data["sdcard/ROMs/scummvm/G.scummvm"], b"sky")
        self.assertEqual(data["sdcard/ROMs/gb/systeminfo.txt"], b"info")
        self.assertEqual(data["sdcard/ROMs/gb/Up.gb.license.txt"], b"up license")
        self.assertEqual(data["sdcard/ROMs/nes/LICENSE.txt"], b"nes license")
        self.assertNotIn("sdcard/ROMs/gb/Gone.gb", members)
        for d in ("sdcard/ROMs/scummvm", "sdcard/ROMs/n64", "sdcard/ROMs/n64/sub"):
            self.assertTrue(members[d].isdir(), d)
            self.assertEqual((members[d].mode, members[d].uid), (0o2770, 1023))
        self.assertEqual(members["sdcard/ROMs/nes/New.nes"].mode, 0o660)
        self.assertEqual(data["sdcard/ES-DE/downloaded_media/gb/covers/Up.png"], PNG)
        gamelist = data["sdcard/ES-DE/gamelists/gb/gamelist.xml"].decode()
        self.assertIn("<name>Up &amp; Away</name>", gamelist)
        self.assertIn("<releasedate>19990501T000000</releasedate>", gamelist)
        index = json.loads(data["data/system/nano_scrape/index.json"])
        self.assertEqual(index["items"][0]["box"], "/data/media/0/ES-DE/downloaded_media/gb/covers/Up.png")
        nano = members["data/system/nano_scrape/index.json"]
        self.assertEqual((nano.mode, nano.uid, nano.uname), (0o600, 0, "root"))
        self.assertNotIn("data", members)
        self.assertNotIn("data/system", members)

    def test_missing_upstream_game(self):
        with open(self.list, "a") as f:
            f.write("upstream  gb/Absent.gb  license=CC0\n")
        self.write("licenses/gb/Absent.gb.license.txt", "x")
        self.assertFails(self.build()[0], "upstream games not in the pack: gb/Absent.gb")

    def test_missing_license(self):
        os.remove(self.path("licenses/nes/LICENSE.txt"))
        self.assertFails(self.build()[0], "nes/New.nes: no license file")
