#!/usr/bin/env python3
"""Zeroes the free blocks of an ext4 image file.

  zero_free_blocks.py IMAGE

Deleted and replaced files leave their old contents in the free blocks, which the PAC carries and
7-Zip compresses (about 500 MB of the download for the system image). `e2fsck -E discard` would
punch them out, but the e2fsck AOSP builds for the host ignores discard on an image file, and the
release build puts that one first on PATH. This reads the free ranges from dumpe2fs and writes
zeros over them. Run it on an unmounted, clean filesystem.
"""
import os
import re
import subprocess
import sys

CHUNK = 1 << 20


def free_ranges(dumpe2fs):
    """(block size, [(first, last)] free block ranges) from dumpe2fs output.

    Fails unless the ranges add up to the superblock's free block count, so a line this misreads
    can't leave blocks out, or zero blocks in use."""
    block = int(re.search(r"^Block size:\s+(\d+)", dumpe2fs, re.M).group(1))
    free = int(re.search(r"^Free blocks:\s+(\d+)", dumpe2fs, re.M).group(1))
    ranges = []
    for line in re.findall(r"^[ \t]+Free blocks: (.+)$", dumpe2fs, re.M):
        for part in line.split(","):
            a, _, b = part.strip().partition("-")
            ranges.append((int(a), int(b or a)))
    listed = sum(b - a + 1 for a, b in ranges)
    if listed != free:
        sys.exit(f"zero_free_blocks: the groups list {listed} free blocks, the superblock {free}")
    return block, ranges


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    image = sys.argv[1]
    out = subprocess.run(["dumpe2fs", image], capture_output=True, text=True, check=True,
                         env={**os.environ, "LC_ALL": "C"}).stdout
    block, ranges = free_ranges(out)
    zero = bytes(CHUNK)
    total = 0
    with open(image, "r+b") as f:
        for a, b in ranges:
            f.seek(a * block)
            left = (b - a + 1) * block
            total += left
            while left:
                n = min(left, CHUNK)
                f.write(zero[:n])
                left -= n
    print("zeroed %d free blocks (%d MB) in %s" % (total // block, total >> 20, image))


if __name__ == "__main__":
    main()
