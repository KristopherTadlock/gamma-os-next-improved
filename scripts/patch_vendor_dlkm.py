#!/usr/bin/env python3
"""Patches the RG Cube 1.2 vendor_dlkm: its DisplayPort driver (sprd-drm.ko) and the order its
modules load in.

  patch_vendor_dlkm.py IN OUT

Applies sprd_drm.py's patch to the copy the kernel loads at boot. vendor_dlkm is a compressed
EROFS image, so it is rebuilt (erofs_image.py). vendor_dlkm has no verity.

The vendor's insmod-sh loads lib/modules/modules.load one module at a time, in the background,
from early in boot. Its first module, the Wi-Fi/Bluetooth base (wcn_bsp.ko), pulls in the PCIe
driver, which waits a second for a link the RG Cube does not have (nothing is on its PCIe), so the
display and GPU drivers the screen needs, the controls, touch and audio all waited behind it. The
radios and PCIe now load last, and the LED driver (cust-leds.ko, a one-second probe) after them,
moved out of the first stage (patch_vendor_boot.py), where nothing else could run meanwhile.

Needs on PATH: fsck.erofs and mkfs.erofs (erofs-utils 1.7 or later).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from erofs_image import rebuild  # noqa: E402
from sprd_drm import patch_module  # noqa: E402

MODULE = "lib/modules/sprd-drm.ko"
MODULE_LIST = "lib/modules/modules.load"
# The radios and what they depend on (modules.dep), loaded last.
LAST = ["wcn_bsp.ko", "sprd_wlan_combo.ko", "sprdbt_tty.ko", "sprd_fm.ko", "gnss_common_ctl_all.ko",
        "gnss_dbg.ko", "gnss_pmnotify_ctl.ko", "pcie-sprd-misc.ko", "pcie-sprd.ko"]
DEFERRED = "cust-leds.ko"   # from the first stage; its probe takes a second


def reorder_modules(data):
    mods = data.decode().split()
    if sorted(set(LAST) - set(mods)):
        sys.exit("vendor_dlkm: modules.load lacks %s" % sorted(set(LAST) - set(mods))[0])
    if DEFERRED in mods:
        sys.exit("vendor_dlkm: modules.load already loads %s" % DEFERRED)
    order = [m for m in mods if m not in LAST] + [m for m in mods if m in LAST] + [DEFERRED]
    return ("\n".join(order) + "\n").encode()


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    src, dst = sys.argv[1], sys.argv[2]
    new = rebuild(src, "vendor_dlkm", {MODULE: patch_module, MODULE_LIST: reorder_modules})
    with open(dst, "wb") as f:
        f.write(new)
    print("patched %s and %s -> %s" % (MODULE, MODULE_LIST, dst))


if __name__ == "__main__":
    main()
