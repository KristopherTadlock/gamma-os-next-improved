#!/usr/bin/env python3
"""Checks that an ext4 image file holds the files the mods wrote.

usage: check_image_files.py IMAGE SHA256_LIST

SHA256_LIST is sha256sum output taken in the mounted image before it was
unmounted, with paths relative to the image root ("./system/bin/setup.sh").
Each file is read back from IMAGE itself with debugfs, so a write that never
reached the image file (the image was packed before the kernel flushed it)
fails the build instead of shipping a broken file.
"""

import hashlib
import subprocess
import sys


def image_sha256(image, path):
    proc = subprocess.Popen(["debugfs", "-R", f'cat "{path}"', image],
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    digest = hashlib.sha256()
    for chunk in iter(lambda: proc.stdout.read(1 << 20), b""):
        digest.update(chunk)
    proc.wait()
    return digest.hexdigest()


def main():
    image, listing = sys.argv[1:3]
    checked, bad = 0, []
    with open(listing, encoding="utf-8", errors="surrogateescape") as f:
        for line in f:
            want, _, path = line.rstrip("\n").partition("  ")
            if not path.startswith("./") or '"' in path:
                sys.exit(f"check_image_files: can't check {line!r}")
            if image_sha256(image, path[1:]) != want:
                bad.append(path[1:])
            checked += 1
    if bad:
        print(f"{len(bad)} of {checked} files the mods wrote are not in the image as written:",
              file=sys.stderr)
        for path in bad[:50]:
            print(f"  {path}", file=sys.stderr)
        sys.exit(1)
    print(f"{checked} files the mods wrote read back intact from the image")


if __name__ == "__main__":
    main()
