"""apps_list.py, tar_roots.py, apk_home.py and build_android_games.sh."""
import os

from tests import util

APPS = """# header
RetroArch|com.retroarch.aarch64|/system/etc/RetroArch_aarch64.apk||post_retroarch
Aurora Store|com.aurora.store|/system/etc/AuroraStore_4.6.2.apk||post_aurora
Firefox|org.mozilla.firefox_beta|/system/etc/fenix.apk|minmem:1300000|
"""


class AppsList(util.TestCase):
    def setUp(self):
        super().setUp()
        self.list = self.write("apps.list", APPS)

    def edit(self, *args):
        return util.run_script("apps_list", self.list, *args)

    def lines(self):
        return util.slurp(self.list, text=True).splitlines()

    def test_add(self):
        self.assertOk(self.edit("add", "Dolphin", "org.dolphinemu.dolphinemu", "/system/etc/dolphin.apk"))
        self.assertEqual(self.lines()[-1], "Dolphin|org.dolphinemu.dolphinemu|/system/etc/dolphin.apk||")
        self.assertOk(self.edit("add", "Fulguris", "net.slions", "/system/etc/f.apk", "maxmem:1300000"))
        self.assertEqual(self.lines()[-1], "Fulguris|net.slions|/system/etc/f.apk|maxmem:1300000|")
        self.assertOk(self.edit("add", "X", "x.y", "/x.apk", "", "post_x"))
        self.assertEqual(self.lines()[-1], "X|x.y|/x.apk||post_x")
        self.assertEqual(self.lines()[0], "# header")

    def test_add_twice_fails(self):
        before = self.lines()
        self.assertFails(self.edit("add", "Aurora", "com.aurora.store", "/a.apk"), "com.aurora.store is already listed")
        self.assertEqual(self.lines(), before)

    def test_hook_replace_source_remove(self):
        self.assertOk(self.edit("hook", "org.mozilla.firefox_beta", "post_ff"))
        self.assertEqual(self.lines()[3], "Firefox|org.mozilla.firefox_beta|/system/etc/fenix.apk|minmem:1300000|post_ff")
        self.assertOk(self.edit("replace", "org.mozilla.firefox_beta", "Brave", "com.brave.browser", "/system/etc/b.apk"))
        # The condition stays; the hook goes.
        self.assertEqual(self.lines()[3], "Brave|com.brave.browser|/system/etc/b.apk|minmem:1300000|")
        self.assertOk(self.edit("source", "/system/etc/AuroraStore_4.6.2.apk", "/system/etc/AuroraStore_4.8.4.apk"))
        self.assertEqual(self.lines()[2], "Aurora Store|com.aurora.store|/system/etc/AuroraStore_4.8.4.apk||post_aurora")
        self.assertOk(self.edit("remove", "com.retroarch.aarch64"))
        self.assertEqual(len(self.lines()), 3)
        self.assertNotIn("RetroArch", util.slurp(self.list, text=True))

    def test_must_match_one(self):
        before = self.lines()
        self.assertFails(self.edit("hook", "no.such", "h"), "0 apps match no.such")
        self.assertFails(self.edit("remove", "no.such"), "0 apps match no.such")
        self.assertFails(self.edit("source", "/nope.apk", "/x.apk"), "0 apps match /nope.apk")
        self.assertFails(self.edit("frob"), "unknown command frob")
        self.assertEqual(self.lines(), before)
        with open(self.list, "a") as f:
            f.write("Dup|com.aurora.store|/d.apk||\n")
        self.assertFails(self.edit("remove", "com.aurora.store"), "2 apps match com.aurora.store")

    def test_comments_are_not_apps(self):
        with open(self.list, "a") as f:
            f.write("#Old|com.old|/old.apk||\n")
        self.assertFails(self.edit("remove", "com.old"), "0 apps match com.old")


class TarRoots(util.TestCase):
    def test_roots(self):
        listing = "\n".join([
            "data/data/com.retroarch.aarch64/", "data/data/com.retroarch.aarch64/cores/",
            "data/data/com.retroarch.aarch64/cores/a.so", "sdcard/RetroArch/", "sdcard/RetroArch/system/", "sdcard/RetroArch/system/x",
            "sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg", ""])
        out = self.assertOk(util.run_script("tar_roots", input=listing)).stdout.split()
        self.assertEqual(out, ["data/data/com.retroarch.aarch64", "sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg",
                               "sdcard/RetroArch"])


XMLTREE = """N: android=http://schemas.android.com/apk/res/android
  E: manifest (line=2)
    A: android:versionCode(0x0101021b)=(type 0x10)0x35
    A: android:versionName(0x0101021c)="3.5.0" (Raw: "3.5.0")
    A: package="org.es_de.frontend" (Raw: "org.es_de.frontend")
    E: application (line=10)
      A: android:label(0x01010001)=@0x7f0e001c
      E: activity (line=12)
        A: android:name(0x01010003)=".SettingsActivity" (Raw: ".SettingsActivity")
        E: intent-filter (line=13)
          E: category (line=14)
            A: android:name(0x01010003)="android.intent.category.DEFAULT" (Raw: "android.intent.category.DEFAULT")
      E: activity (line=20)
        A: android:name(0x01010003)="org.es_de.frontend.MainActivity" (Raw: "org.es_de.frontend.MainActivity")
        E: intent-filter (line=21)
          E: action (line=22)
            A: android:name(0x01010003)="android.intent.action.MAIN" (Raw: "android.intent.action.MAIN")
          E: category (line=23)
            A: android:name(0x01010003)="android.intent.category.HOME" (Raw: "android.intent.category.HOME")
"""


class ApkHome(util.TestCase):
    def home(self, tree):
        apk = self.write("app.apk", b"PK")
        self.write("app.apk.xmltree", tree)
        shims = util.shim_dir(self.tmp, "aapt")
        return util.run_script("apk_home", apk, path_prepend=[shims])

    def test_home_activity(self):
        self.assertEqual(self.assertOk(self.home(XMLTREE)).stdout.split(),
                         ["org.es_de.frontend", "3.5.0", "org.es_de.frontend.MainActivity"])

    def test_relative_name_and_alias(self):
        tree = XMLTREE.replace('E: activity (line=20)', 'E: activity-alias (line=20)').replace(
            '"org.es_de.frontend.MainActivity" (Raw', '".Home" (Raw')
        self.assertEqual(self.assertOk(self.home(tree)).stdout.split()[2], "org.es_de.frontend.Home")

    def test_no_home(self):
        tree = XMLTREE.replace("android.intent.category.HOME", "android.intent.category.LAUNCHER")
        self.assertFails(self.home(tree), "home activity None")


class AndroidGamesKeystore(util.TestCase):
    def test_no_silent_new_key(self):
        """L11: a wiped cache must not sign the app with a new key unasked."""
        script = f"{util.SCRIPTS}/build_android_games.sh"
        env = {"ANDROID_JAR": self.write("android.jar", b"")}
        proc = util.run(["bash", script, self.path("out.apk"), self.path("cache/ks")], env=env)
        self.assertFails(proc, "no keystore", "NEW_KEYSTORE=1")
        self.assertFalse(os.path.exists(self.path("cache/ks")))
