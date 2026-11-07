#!/usr/bin/env python3
"""Edits GammaOS 1.4.4's first-boot app list (/system/etc/gammaos/apps.list).

  apps_list.py LIST add NAME PACKAGE SOURCE [CONDITION [HOOK]]
  apps_list.py LIST hook PACKAGE HOOK
  apps_list.py LIST replace PACKAGE NAME NEW_PACKAGE SOURCE
  apps_list.py LIST source OLD_SOURCE NEW_SOURCE

Each app is a line "name|package|source|condition|hook". `add` appends an app,
`hook` sets an app's post-install hook, `replace` swaps an app for another
(keeping its condition and dropping its hook), and `source` points an app at a
renamed file. Every command fails unless it changes exactly one line.
"""

import sys


def main():
    path, cmd, *args = sys.argv[1:]
    lines = open(path).read().splitlines()

    def apps():
        for i, line in enumerate(lines):
            if line.strip() and not line.startswith("#"):
                yield i, (line.split("|") + [""] * 5)[:5]

    def only(matches, what):
        if len(matches) != 1:
            sys.exit(f"apps.list: {len(matches)} apps match {what}")
        return matches[0]

    if cmd == "add":
        name, package, source = args[:3]
        condition, hook = (args[3:] + ["", ""])[:2]
        if [i for i, f in apps() if f[1] == package]:
            sys.exit(f"apps.list: {package} is already listed")
        lines.append("|".join([name, package, source, condition, hook]))
    elif cmd == "hook":
        package, hook = args
        i, f = only([(i, f) for i, f in apps() if f[1] == package], package)
        f[4] = hook
        lines[i] = "|".join(f)
    elif cmd == "replace":
        package, name, new_package, source = args
        i, f = only([(i, f) for i, f in apps() if f[1] == package], package)
        lines[i] = "|".join([name, new_package, source, f[3], ""])
    elif cmd == "source":
        old, new = args
        i, f = only([(i, f) for i, f in apps() if f[2] == old], old)
        f[2] = new
        lines[i] = "|".join(f)
    else:
        sys.exit(f"unknown command {cmd}")
    open(path, "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
