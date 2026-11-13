"""Runs GammaOS's first-boot setup.sh in a sandbox: a chroot in a user namespace, with fake
Android commands that record what setup asked of them.

    box = Sandbox(tmpdir)                 # copies AOSP's gammaos/setup.sh
    box.apps(["Brave|com.brave.browser|/system/etc/Brave.apk|minmem:1300000|"])
    box.meminfo(1000000)                  # or None: no /proc/meminfo
    result = box.run()                    # result.log, result.calls, result.returncode

The fake pm records installs in /state/installed and makes /data/data/<package>; a package listed
by fail_install() never installs. sleep only logs, so retry backoffs take no time.
"""
import os
import subprocess

from tests import util

SETUP_SOURCE = os.path.join(util.AOSP, "gammaos", "setup.sh")
LOG = "data/data/org.lineageos.setupwizard/files/gammaos_setup.log"

LOGGER = '#!/bin/bash\necho "%s $*" >> /state/calls\n'
FAKES = {
    name: LOGGER % name
    for name in ("am", "appops", "chown", "cmd", "dmesg", "dumpsys", "mkswap", "nsenter",
                 "restorecon", "settings", "swapoff", "swapon", "sleep", "svc")
}
FAKES.update({
    "fallocate": "#!/bin/bash\nexit 1\n",
    "getprop": '#!/bin/bash\nf=/state/prop_$(echo "$1" | tr . _); [ -f "$f" ] && cat "$f"; exit 0\n',
    "setprop": '#!/bin/bash\necho "setprop $*" >> /state/calls; echo "$2" > /state/prop_$(echo "$1" | tr . _)\n',
    "pm": r'''#!/bin/bash
echo "pm $*" >> /state/calls
st=/state/installed
case "$1" in
  path) grep -qxF "$2" $st 2>/dev/null && echo "package:/data/app/$2/base.apk" || exit 1 ;;
  install)
    src="${@: -1}"
    pkg=$(grep -v '^#' /system/etc/gammaos/apps.list | awk -F'|' -v s="$src" '$3 == s { print $2 }')
    if grep -qxF "$pkg" /state/pm_fail 2>/dev/null; then
      echo "Failure [INSTALL_FAILED_INTERNAL_ERROR]"
      exit 1
    fi
    echo "$pkg" >> $st; mkdir -p "/data/data/$pkg"; echo Success ;;
  install-create) echo "Success: created install session [42]" ;;
  install-write|install-abandon|install-commit) echo Success ;;
esac
exit 0
''',
})


def available():
    return os.path.isfile(SETUP_SOURCE) and util.have_userns()


class Result:
    def __init__(self, returncode, log, calls, output):
        self.returncode, self.log, self.calls, self.output = returncode, log, calls, output

    def installs(self):
        """Source files `pm install` was run on, in order."""
        return [c.split()[-1] for c in self.calls if c.startswith("pm install ")]


class Sandbox:
    def __init__(self, parent, setup_text=None):
        self.root = os.path.join(parent, "root")
        r = self.root
        for d in ("system/bin", "system/etc/gammaos", "data/data/org.lineageos.setupwizard/files",
                  "data/system", "sdcard", "proc/sys/vm", "state", "tmp", "dev", "usr", "etc", "fake"):
            os.makedirs(os.path.join(r, d), exist_ok=True)
        self.binds = ["/usr", "/etc", "/dev"]
        for d in ("bin", "lib", "lib64", "sbin"):
            host = "/" + d
            if os.path.islink(host):
                os.symlink(os.readlink(host), os.path.join(r, d))
            elif os.path.isdir(host):
                os.makedirs(os.path.join(r, d))
                self.binds.append(host)
        os.symlink("/usr/bin/bash", os.path.join(r, "system/bin/sh"))
        if setup_text is None:
            setup_text = util.slurp(SETUP_SOURCE, text=True)
        self.set_setup(setup_text)
        for name, text in FAKES.items():
            self._write("fake/" + name, text, 0o755)
        self._write("proc/swaps", "Filename\tType\tSize\tUsed\tPriority\n")
        self._write("proc/sys/vm/drop_caches", "")
        self.meminfo(2000000)

    def _write(self, rel, text, mode=0o644):
        p = os.path.join(self.root, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as f:
            f.write(text)
        os.chmod(p, mode)
        return p

    def set_setup(self, text):
        self._write("system/bin/setup.sh", text, 0o755)

    def meminfo(self, kb):
        p = os.path.join(self.root, "proc/meminfo")
        if kb is None:
            if os.path.exists(p):
                os.remove(p)
        else:
            self._write("proc/meminfo", "MemTotal:       %d kB\nMemFree:          10000 kB\n" % kb)

    def apps(self, lines):
        """Writes apps.list from LINES ("name|package|source|condition|hook") and each source file."""
        self._write("system/etc/gammaos/apps.list", "# test list\n" + "".join(line + "\n" for line in lines))
        for line in lines:
            src = line.split("|")[2]
            if src:
                self._write(src.lstrip("/"), "apk")

    def installed(self, *packages):
        for pkg in packages:
            with open(os.path.join(self.root, "state/installed"), "a") as f:
                f.write(pkg + "\n")
            os.makedirs(os.path.join(self.root, "data/data", pkg), exist_ok=True)

    def fail_install(self, *packages):
        self._write("state/pm_fail", "".join(p + "\n" for p in packages))

    def completed(self):
        os.makedirs(os.path.join(self.root, "data/setupcompleted"), exist_ok=True)

    def run(self, timeout=60):
        mounts = " && ".join('mount --rbind %s "$1%s"' % (b, b) for b in self.binds)
        script = mounts + (' && exec chroot "$1" /usr/bin/env -i PATH=/fake:/usr/bin:/usr/sbin'
                           ' /system/bin/sh /system/bin/setup.sh')
        proc = subprocess.run(["unshare", "-Urm", "bash", "-c", script, "bash", self.root],
                              capture_output=True, text=True, timeout=timeout)

        def lines(rel):
            p = os.path.join(self.root, rel)
            return util.slurp(p, text=True).splitlines() if os.path.exists(p) else []
        log = "\n".join(lines(LOG))
        return Result(proc.returncode, log, lines("state/calls"), proc.stdout + proc.stderr)


def add_setup_steps(parent, setup_text, steps_text, nano=True, function="add_setup_steps"):
    """setup.sh text after the build's add_setup_steps (scripts/modlib.sh) inserted STEPS_TEXT,
    run with sudo shimmed, on a system folder in PARENT. Returns (new text, CompletedProcess).
    function="add_setup_functions" inserts post-install hook functions instead."""
    system = os.path.join(parent, "system")
    os.makedirs(os.path.join(system, "bin"), exist_ok=True)
    os.makedirs(os.path.join(system, "etc/gammaos"), exist_ok=True)
    if nano:
        with open(os.path.join(system, "etc/gammaos/apps.list"), "w") as f:
            f.write("# apps\n")
    setup = os.path.join(system, "bin/setup.sh")
    with open(setup, "w") as f:
        f.write(setup_text)
    steps = os.path.join(parent, "steps.sh")
    with open(steps, "w") as f:
        f.write(steps_text)
    shims = os.path.join(parent, "shims")
    if not os.path.isdir(shims):
        util.shim_dir(parent, "sudo")
    proc = util.run(["bash", "-c", 'set -euo pipefail; source "$1"; "$4" "$2" "$3"', "bash",
                     os.path.join(util.SCRIPTS, "modlib.sh"), system, steps, function], path_prepend=[shims])
    return util.slurp(setup, text=True), proc


def browser_steps():
    """The default-browser steps mods/15-update-apps.sh adds to setup.sh, taken from the mod."""
    text = util.slurp(os.path.join(util.MODS, "15-update-apps.sh"), text=True)
    start = text.index("cat >\"$CACHE/browser-steps.sh\" <<'STEPS'\n")
    body = text[text.index("\n", start) + 1:]
    return body[:body.index("\nSTEPS\n") + 1]
