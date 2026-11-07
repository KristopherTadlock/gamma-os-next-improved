#!/usr/bin/env python3
"""Prints the top-level paths of a tar listing read from stdin.

A path is top-level when the listing has no entry for its parent directory.
Repacking only these paths reproduces the archive's structure without adding
entries for parent directories (such as data/ or sdcard/) that the original
never contained.
"""

import sys

names = {line.rstrip("\n").rstrip("/") for line in sys.stdin if line.strip()}
for name in sorted(names):
    if name.rpartition("/")[0] not in names:
        print(name)
