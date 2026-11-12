#!/usr/bin/env python3
"""Checks that an ext4 image file holds the files the mods wrote, and not the ones they deleted.

usage: check_image_files.py IMAGE SHA256_LIST [DELETED_LIST]

SHA256_LIST is sha256sum output taken in the mounted image before it was
unmounted, with paths relative to the image root ("./system/bin/setup.sh").
Each file is read back from IMAGE itself with debugfs, so a write that never
reached the image file (the image was packed before the kernel flushed it)
fails the build instead of shipping a broken file. DELETED_LIST holds the
NUL-separated paths (same form) of the files the mods removed; each must be
gone from IMAGE.
"""

import hashlib
import os
import re
import subprocess
import sys

BANNER = re.compile(rb"^debugfs \d[\d.]* \(.*\)$")
NOT_FOUND = b"File not found by ext2_lookup"


def debugfs(image, request, sink):
    """Runs one debugfs request, feeding its output to SINK; returns its messages but the banner.

    debugfs exits 0 even when the request fails, so its messages are the only sign of that."""
    proc = subprocess.Popen(["debugfs", "-R", request, image],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    for chunk in iter(lambda: proc.stdout.read(1 << 20), b""):
        sink(chunk)
    stderr = proc.stderr.read()
    if proc.wait():
        sys.exit(f"check_image_files: debugfs exited {proc.returncode} on {request!r}")
    return [line for line in stderr.splitlines() if line.strip() and not BANNER.match(line)]


def image_sha256(image, path):
    """(sha256 of PATH in IMAGE, None), or (None, debugfs's message) when it can't be read."""
    digest = hashlib.sha256()
    errors = debugfs(image, f'cat "{path}"', digest.update)
    if errors:
        return None, errors[0].decode(errors="replace").strip()
    return digest.hexdigest(), None


def in_image(image, path):
    errors = debugfs(image, f'stat "{path}"', lambda _: None)
    if any(NOT_FOUND in line for line in errors):
        return False
    if errors:
        sys.exit(f"check_image_files: {path}: {errors[0].decode(errors='replace')}")
    return True


def parse_sha256sum(line):
    """(digest, path) of a sha256sum line, undoing its escaping of names with \\ or a newline."""
    line = line.rstrip("\n")
    escaped = line.startswith("\\")
    if escaped:
        line = line[1:]
    want, sep, path = line.partition("  ")
    if not sep or not re.fullmatch(r"[0-9a-f]{64}", want):
        return None, None
    if escaped:
        parts = re.split(r"(\\.?)", path)
        unescape = {"\\\\": "\\", "\\n": "\n", "\\r": "\r"}
        if any(p.startswith("\\") and p not in unescape for p in parts):
            return None, None
        path = "".join(unescape.get(p, p) for p in parts)
    return want, path


def checkable(path):
    # debugfs takes the path in double quotes, and can't be given one inside them.
    return path.startswith("./") and '"' not in path and "\n" not in path


def main():
    if len(sys.argv) not in (3, 4):
        sys.exit(__doc__)
    image, listing = sys.argv[1:3]
    deleted_list = sys.argv[3] if len(sys.argv) == 4 else None
    for path in filter(None, (image, listing, deleted_list)):
        if not os.path.isfile(path):
            sys.exit(f"check_image_files: no file {path}")

    checked, bad = 0, []
    with open(listing, encoding="utf-8", errors="surrogateescape") as f:
        for line in f:
            want, path = parse_sha256sum(line)
            if not path or not checkable(path):
                sys.exit(f"check_image_files: can't check {line!r}")
            got, error = image_sha256(image, path[1:])
            if got != want:
                bad.append(path[1:] + (f" ({error})" if error else ""))
            checked += 1

    deleted, present = 0, []
    if deleted_list:
        with open(deleted_list, "rb") as f:
            names = [n.decode("utf-8", "surrogateescape") for n in f.read().split(b"\0") if n]
        for path in names:
            if not checkable(path):
                sys.exit(f"check_image_files: can't check deleted {path!r}")
            if in_image(image, path[1:]):
                present.append(path[1:])
            deleted += 1

    if bad:
        print(f"{len(bad)} of {checked} files the mods wrote are not in the image as written:",
              file=sys.stderr)
        for path in bad[:50]:
            print(f"  {path}", file=sys.stderr)
    if present:
        print(f"{len(present)} of {deleted} files the mods deleted are still in the image:",
              file=sys.stderr)
        for path in present[:50]:
            print(f"  {path}", file=sys.stderr)
    if bad or present:
        sys.exit(1)
    print(f"{checked} files the mods wrote read back intact from the image"
          + (f"; {deleted} deleted files are gone" if deleted_list else ""))


if __name__ == "__main__":
    main()
