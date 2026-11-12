"""Runs mods/*.sh on fake /system trees, as root in a user namespace, with shims for sudo (runs its
command), setfattr (logs) and curl (fails loudly)."""
import os

from tests import util

TOOLS = ("unshare", "zstd", "xz", "unzip", "gzip", "tar", "awk")
CLEAN_ENV = ("ESDE_APK", "SYSTEM_IMG", "REFRESH_CORES", "ALLOW_BUNDLED_CORES", "NEW_KEYSTORE")


class ModCase(util.TestCase):
    """self.run_mod(NAME) runs mods/NAME*.sh on self.system with self.cache."""

    def setUp(self):
        super().setUp()
        if not util.have_userns():
            self.skipTest("no user namespaces (unshare -Ur)")
        util.need(self, *TOOLS)
        self.shims = util.shim_dir(self.tmp, "sudo", "setfattr", "curl", "sleep")
        self.log = self.path("shim.log")
        self.cache = self.path("cache")
        os.makedirs(self.cache)

    def mod_path(self, prefix):
        names = [n for n in os.listdir(util.MODS) if n.startswith(prefix) and n.endswith(".sh")]
        self.assertEqual(len(names), 1, prefix)
        return os.path.join(util.MODS, names[0])

    def run_mod(self, prefix, system=None, env=None, cache=None):
        full_env = {k: v for k, v in os.environ.items() if k not in CLEAN_ENV}
        full_env.update({"PATH": self.shims + os.pathsep + os.environ["PATH"], "SHIM_LOG": self.log,
                         "THIMSY_BUILD_VERSION": "Thimsy_OS_vTEST", "THIMSY_VERSION": "TEST"})
        full_env.update(env or {})
        cmd = ["unshare", "-Ur", "env", "-i"] + [f"{k}={v}" for k, v in full_env.items()] + \
              ["bash", self.mod_path(prefix), system or self.system, cache or self.cache]
        return util.run(cmd, env={}, timeout=300)

    def shim_calls(self, name):
        if not os.path.exists(self.log):
            return []
        return [c for c in util.slurp(self.log, text=True).splitlines() if c.startswith(name + " ")]
