#!/usr/bin/env python3
"""Prints an APK's package, version name, and home activity.

  apk_home.py APK

The home activity is the first activity (or alias) with an intent filter in
the android.intent.category.HOME category, as a full class name. Needs aapt.
"""

import re
import subprocess
import sys

ATTR_RE = re.compile(r'A: (?:android:)?(\w+)(?:\(0x[0-9a-f]+\))?="([^"]*)"')


def main():
    tree = subprocess.run(["aapt", "dump", "xmltree", sys.argv[1], "AndroidManifest.xml"],
                          check=True, capture_output=True, text=True).stdout
    pkg = version = home = None
    activity, activity_depth = None, None
    for line in tree.splitlines():
        depth = len(line) - len(line.lstrip())
        if " E: " in line:
            if activity_depth is not None and depth <= activity_depth:
                activity, activity_depth = None, None
            if re.search(r" E: activity(-alias)? ", line):
                activity, activity_depth = None, depth
            continue
        m = ATTR_RE.search(line)
        if not m:
            continue
        name, value = m.groups()
        if name == "package" and pkg is None:
            pkg = value
        elif name == "versionName" and version is None:
            version = value
        elif name == "name" and activity_depth is not None and activity is None:
            activity = value
        elif name == "name" and value == "android.intent.category.HOME" and home is None:
            home = activity
    if not (pkg and version and home):
        sys.exit(f"{sys.argv[1]}: package {pkg}, version {version}, home activity {home}")
    if home.startswith("."):
        home = pkg + home
    print(pkg, version, home)


if __name__ == "__main__":
    main()
