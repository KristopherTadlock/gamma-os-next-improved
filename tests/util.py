"""Shared helpers for the Thimsy build tests (standard library only).

THIMSY_TEST_MODE (set by tests/run.sh) picks the tests that run: "quick" (the default), "slow" or
"all". A test or class decorated with @slow runs in "slow" and "all"; every other test in "quick"
and "all".
"""

import hashlib
import importlib.util
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPTS = os.path.join(ROOT, "scripts")
MODS = os.path.join(ROOT, "mods")
TESTS = os.path.join(ROOT, "tests")
SHIMS = os.path.join(TESTS, "shims")
MODE = os.environ.get("THIMSY_TEST_MODE", "quick")
AOSP = os.path.expanduser(os.environ.get("AOSP", "~/gammaos-src"))
AOSP_HOST_BIN = os.path.join(AOSP, "out/host/linux-x86/bin")
# The real build cache: tests that need its downloads copy what they use and never write to it.
TEST_CACHE = os.environ.get("THIMSY_TEST_CACHE")


def slow(obj):
    obj.thimsy_slow = True
    return obj


class TestCase(unittest.TestCase):
    """A test case with the quick/slow selection and a scratch directory (self.tmp)."""

    def setUp(self):
        method = getattr(self, self._testMethodName)
        is_slow = getattr(method, "thimsy_slow", False) or getattr(type(self), "thimsy_slow", False)
        if is_slow and MODE not in ("slow", "all"):
            self.skipTest("slow test (tests/run.sh slow)")
        if not is_slow and MODE == "slow":
            self.skipTest("quick test")
        self.tmp = tempfile.mkdtemp(prefix="thimsy-test-")
        self.addCleanup(rmtree, self.tmp)

    def path(self, *parts):
        return os.path.join(self.tmp, *parts)

    def write(self, rel, data, mode=None):
        """Writes DATA (str or bytes) to self.tmp/REL, making its folders; returns the path."""
        p = self.path(rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data.encode() if isinstance(data, str) else data)
        if mode is not None:
            os.chmod(p, mode)
        return p

    def assertFails(self, proc, *needles):
        """PROC exited non-zero and its stderr (or stdout) says each of NEEDLES."""
        out = proc.stdout + proc.stderr
        self.assertNotEqual(proc.returncode, 0, f"expected a failure, got:\n{out}")
        for needle in needles:
            self.assertIn(needle, out)

    def assertOk(self, proc):
        self.assertEqual(proc.returncode, 0, f"exit {proc.returncode}:\n{proc.stdout}{proc.stderr}")
        return proc


def rmtree(path):
    """rm -rf, including folders a test left without write permission."""
    def fix(func, p, _exc):
        os.chmod(os.path.dirname(p), 0o755)
        if os.path.isdir(p) and not os.path.islink(p):
            os.chmod(p, 0o755)
        func(p)
    if os.path.lexists(path):
        shutil.rmtree(path, onerror=fix)


def load_script(name):
    """Imports scripts/NAME.py as a module, without running its main()."""
    spec = importlib.util.spec_from_file_location("thimsy_" + name, os.path.join(SCRIPTS, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run(cmd, env=None, cwd=None, input=None, timeout=120, path_prepend=()):
    """Runs CMD; returns the CompletedProcess with text stdout and stderr."""
    full_env = dict(os.environ)
    full_env.update(env or {})
    if path_prepend:
        full_env["PATH"] = os.pathsep.join(list(path_prepend) + [full_env.get("PATH", "")])
    return subprocess.run(cmd, env=full_env, cwd=cwd, input=input, timeout=timeout,
                          capture_output=True, text=True, errors="replace")


def run_script(name, *args, **kw):
    """Runs scripts/NAME.py with ARGS the way build.sh does (python3 -I)."""
    return run([sys.executable, "-I", os.path.join(SCRIPTS, name + ".py"), *map(str, args)], **kw)


def which(tool):
    """TOOL on PATH, or in the AOSP host build's bin folder; None when neither has it."""
    return shutil.which(tool) or shutil.which(tool, path=AOSP_HOST_BIN)


def need(testcase, *tools):
    """Skips TESTCASE unless every tool is found; returns a PATH prefix list for the ones found."""
    dirs = []
    for tool in tools:
        found = which(tool)
        if not found:
            testcase.skipTest(f"{tool} isn't on PATH or in {AOSP_HOST_BIN}")
        d = os.path.dirname(found)
        if d not in dirs:
            dirs.append(d)
    return dirs


_userns = None


def have_userns():
    global _userns
    if _userns is None:
        try:
            _userns = subprocess.run(["unshare", "-Ur", "true"], capture_output=True).returncode == 0
        except OSError:
            _userns = False
    return _userns


def shim_dir(parent, *names, extra=None):
    """A folder in PARENT holding the named shims from tests/shims (and EXTRA {name: script})."""
    d = os.path.join(parent, "shims")
    os.makedirs(d, exist_ok=True)
    for name in names:
        os.symlink(os.path.join(SHIMS, name), os.path.join(d, name))
    for name, text in (extra or {}).items():
        p = os.path.join(d, name)
        with open(p, "w") as f:
            f.write(text)
        os.chmod(p, 0o755)
    return d


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def manifest(top):
    """{relative path: (kind, sha256 or link target, mode)} for everything under TOP."""
    out = {}
    for dirpath, dirnames, filenames in os.walk(top):
        for name in sorted(dirnames + filenames):
            p = os.path.join(dirpath, name)
            rel = os.path.relpath(p, top)
            st = os.lstat(p)
            if stat.S_ISLNK(st.st_mode):
                out[rel] = ("link", os.readlink(p), 0)
            elif stat.S_ISDIR(st.st_mode):
                out[rel] = ("dir", "", stat.S_IMODE(st.st_mode))
            else:
                out[rel] = ("file", sha256_file(p), stat.S_IMODE(st.st_mode))
    return out


def manifest_diff(a, b):
    """Readable differences between two manifests."""
    lines = []
    for rel in sorted(set(a) | set(b)):
        if a.get(rel) != b.get(rel):
            lines.append(f"{rel}: {a.get(rel)} -> {b.get(rel)}")
    return "\n".join(lines)


def slurp(path, text=False):
    """The contents of PATH (bytes, or str with TEXT)."""
    with open(path, "r" if text else "rb") as f:
        return f.read()
