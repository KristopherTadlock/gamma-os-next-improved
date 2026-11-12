"""GammaOS's first-boot setup.sh, run in a chroot with fake Android commands (fixtures/setup_sandbox)."""
from tests import util
from tests.fixtures import setup_sandbox
from tests.fixtures.setup_sandbox import Sandbox

BRAVE = "Brave|com.brave.browser|/system/etc/Brave_1.99.55_arm64.apk|minmem:1300000|"
FULGURIS = "Fulguris|net.slions.fulguris.full.fdroid|/system/etc/Fulguris_2.1.2.apk|maxmem:1300000|"
ANCHOR = "mkdir -p /data/setupcompleted"


class SetupSandboxCase(util.TestCase):
    def setUp(self):
        super().setUp()
        if not util.have_userns():
            self.skipTest("no user namespaces (unshare -Ur)")
        if not setup_sandbox.available():
            self.skipTest(f"no {setup_sandbox.SETUP_SOURCE}")
        self.source = util.slurp(setup_sandbox.SETUP_SOURCE, text=True)

    def sandbox(self, name="box", setup_text=None):
        return Sandbox(self.path(name), setup_text)

    def run_ok(self, box):
        result = box.run()
        self.assertEqual(result.returncode, 0, result.output + result.log)
        self.assertIn("setup.sh exited with 0", result.log)
        return result


class MemoryConditions(SetupSandboxCase):
    def test_minmem_and_maxmem(self):
        # MemTotal (kB, None: no /proc/meminfo) -> (Brave installed, Fulguris installed)
        cases = {1000000: (False, True), 1300000: (False, True), 1300001: (True, False),
                 3900000: (True, False), None: (True, False)}
        for kb, (brave, fulguris) in cases.items():
            with self.subTest(meminfo=kb):
                box = self.sandbox(f"box-{kb}")
                box.apps([BRAVE, FULGURIS])
                box.meminfo(kb)
                result = self.run_ok(box)
                installs = result.installs()
                self.assertEqual("/system/etc/Brave_1.99.55_arm64.apk" in installs, brave, result.log)
                self.assertEqual("/system/etc/Fulguris_2.1.2.apk" in installs, fulguris, result.log)
                if not brave:
                    self.assertIn("Skipping Brave (low-memory device", result.log)
                if not fulguris:
                    self.assertIn("Skipping Fulguris (only for devices with 1300000 kB of RAM or less)",
                                  result.log)


class DefaultBrowser(SetupSandboxCase):
    ROLE = "cmd role add-role-holder --user 0 android.app.role.BROWSER "

    def setup_with_browser_steps(self):
        text, proc = setup_sandbox.add_setup_steps(self.path("build"), self.source,
                                                   setup_sandbox.browser_steps())
        self.assertOk(proc)
        return text

    def test_browser_by_memory_and_rerun(self):
        text = self.setup_with_browser_steps()
        for name, kb, completed, want in (("big", 2000000, False, "com.brave.browser"),
                                          ("small", 1000000, False, "net.slions.fulguris.full.fdroid"),
                                          ("rerun", 2000000, True, None)):
            with self.subTest(name):
                box = self.sandbox(name, text)
                box.apps([BRAVE, FULGURIS])
                box.meminfo(kb)
                if completed:
                    box.completed()
                    box.installed("com.brave.browser")
                result = self.run_ok(box)
                roles = [c for c in result.calls if c.startswith(self.ROLE)]
                self.assertEqual(roles, [self.ROLE + want] if want else [], result.log)


class ReRun(SetupSandboxCase):
    def test_setupcompleted_keeps_roms_and_installed_apps(self):
        box = self.sandbox()
        box.apps([BRAVE, "Aurora Store|com.aurora.store|/system/etc/AuroraStore.apk||"])
        box.installed("com.brave.browser")
        box.completed()
        result = self.run_ok(box)
        self.assertIn("Re-run detected (/data/setupcompleted exists)", result.log)
        self.assertNotIn("Extracting default ROMs.", result.log)
        self.assertIn("Brave already installed, skipping install.", result.log)
        self.assertEqual(result.installs(), ["/system/etc/AuroraStore.apk"])

    def test_fresh_setup_extracts_roms(self):
        box = self.sandbox()
        box.apps([BRAVE])
        result = self.run_ok(box)
        self.assertIn("Extracting default ROMs.", result.log)
        self.assertNotIn("Re-run detected", result.log)


class InstallFailures(SetupSandboxCase):
    def test_pm_failure_is_retried_then_skipped(self):
        box = self.sandbox()
        box.apps(["Aurora Store|com.aurora.store|/system/etc/AuroraStore.apk||post_aurora",
                  "Flycast|com.flycast.emulator|/system/etc/flycast.apk||"])
        box.fail_install("com.aurora.store")
        result = self.run_ok(box)
        self.assertEqual(result.installs().count("/system/etc/AuroraStore.apk"), 3, result.log)
        self.assertIn("installing /system/etc/AuroraStore.apk failed (rc=1), attempt 1 of 3", result.log)
        self.assertIn("attempt 2 of 3", result.log)
        self.assertIn("Aurora Store install failed, skipping its post-install step.", result.log)
        # post_aurora would extract its payload, and warn that this image has none.
        self.assertNotIn("aurorastore.tar.zst", result.log)
        self.assertIn("/system/etc/flycast.apk", result.installs())
        self.assertIn("sleep 3", result.calls)
        self.assertIn("sleep 6", result.calls)

    def test_unknown_hook(self):
        box = self.sandbox()
        box.apps(["Flycast|com.flycast.emulator|/system/etc/flycast.apk||post_nothing"])
        result = self.run_ok(box)
        self.assertIn("warning: unknown post-install hook 'post_nothing' for Flycast.", result.log)


class InsertedSteps(SetupSandboxCase):
    STEPS = ('if [ -e /data/setupcompleted ]; then echo "THIMSY-STEP AFTER"; '
             'else echo "THIMSY-STEP BEFORE"; fi\n\n')

    def test_steps_run_before_setup_completes(self):
        text, proc = setup_sandbox.add_setup_steps(self.path("build"), self.source, self.STEPS)
        self.assertOk(proc)
        lines = text.splitlines()
        steps = self.STEPS.splitlines()
        self.assertEqual(lines.count(ANCHOR), 1)
        i = lines.index(ANCHOR)
        self.assertEqual(lines[i - len(steps):i], steps)
        self.assertTrue(lines[i - len(steps) - 1].startswith("# thimsy: "), lines[i - len(steps) - 1])
        # Everything else is as it was.
        self.assertEqual(lines[:i - len(steps) - 1] + lines[i:], self.source.splitlines())

        again, proc = setup_sandbox.add_setup_steps(self.path("build"), text, self.STEPS)
        self.assertOk(proc)
        self.assertEqual(again, text)

        box = self.sandbox(setup_text=text)
        box.apps([BRAVE])
        result = self.run_ok(box)
        self.assertIn("THIMSY-STEP BEFORE", result.log)
        self.assertNotIn("THIMSY-STEP AFTER", result.log)
