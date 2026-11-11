#!/usr/bin/env python3
"""Patches the RG Cube 1.2 vendor_dlkm's DisplayPort driver (sprd-drm.ko).

  patch_vendor_dlkm.py IN OUT

Applies sprd_drm.py's patch to the copy the kernel loads at boot. vendor_dlkm is a compressed
EROFS image, so it is rebuilt (erofs_image.py). vendor_dlkm has no verity.

Needs on PATH: fsck.erofs and mkfs.erofs (erofs-utils 1.7 or later).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from erofs_image import rebuild  # noqa: E402
from sprd_drm import patch_module  # noqa: E402

MODULE = "lib/modules/sprd-drm.ko"


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    src, dst = sys.argv[1], sys.argv[2]
    new = rebuild(src, "vendor_dlkm", {MODULE: patch_module})
    with open(dst, "wb") as f:
        f.write(new)
    print("patched %s -> %s" % (MODULE, dst))


if __name__ == "__main__":
    main()
