#!/usr/bin/env python3
"""Patches the RG Cube 1.2 vendor_dlkm: its DisplayPort driver (sprd-drm.ko) and the order its
modules load in.

  patch_vendor_dlkm.py [--load-leds] IN OUT

Applies sprd_drm.py's patch to the copy the kernel loads at boot. vendor_dlkm is a compressed
EROFS image, so it is rebuilt (erofs_image.py). vendor_dlkm has no verity.

The vendor's insmod-sh loads lib/modules/modules.load one module at a time, in the background,
from early in boot. Its first module, the Wi-Fi/Bluetooth base (wcn_bsp.ko), pulls in the PCIe
driver, which waits a second for a link the RG Cube does not have (nothing is on its PCIe), so the
display and GPU drivers the screen needs, the controls, touch and audio all waited behind it. The
radios and PCIe now load last, and the Goodix touch driver before the FocalTech one (they share
pins; see TOUCH_FIRST). The LED driver (cust-leds.ko, a one-second probe), moved out of the first
stage (patch_vendor_boot.py), where nothing else could run meanwhile, is not in this list: init loads
it in a process of its own beside it (system/core/rootdir/init.rc in the Nano patch). With a system
image that lacks that (--load-leds), it loads after the radios instead.

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
DEFERRED = "cust-leds.ko"   # from the first stage; init loads it beside this list (init.rc)
# Two touch drivers for the panels the RG Cube ships with, on the same I2C bus and the same reset and
# interrupt pins (the board's device tree). The FocalTech one first spent 1.3 s retrying a chip that
# is not there before it gave the pins up; with Goodix first, whichever panel is fitted takes the pins
# and the other driver fails at once.
TOUCH_FIRST, TOUCH_THEN = "gt9xx_ts.ko", "focaltech_ts.ko"


def reorder_modules(data, load_leds):
    mods = data.decode().split()
    if sorted(set(LAST) - set(mods)):
        sys.exit("vendor_dlkm: modules.load lacks %s" % sorted(set(LAST) - set(mods))[0])
    if DEFERRED in mods:
        sys.exit("vendor_dlkm: modules.load already loads %s" % DEFERRED)
    if TOUCH_FIRST not in mods or TOUCH_THEN not in mods:
        sys.exit("vendor_dlkm: modules.load lacks the touch drivers")
    order = [m for m in mods if m not in LAST and m != TOUCH_FIRST]
    order.insert(order.index(TOUCH_THEN), TOUCH_FIRST)
    order += [m for m in mods if m in LAST]
    if load_leds:
        order.append(DEFERRED)
    return ("\n".join(order) + "\n").encode()


def main():
    args = sys.argv[1:]
    load_leds = "--load-leds" in args
    args = [a for a in args if a != "--load-leds"]
    if len(args) != 2:
        sys.exit(__doc__)
    src, dst = args
    new = rebuild(src, "vendor_dlkm", {MODULE: patch_module,
                                       MODULE_LIST: lambda d: reorder_modules(d, load_leds)})
    with open(dst, "wb") as f:
        f.write(new)
    print("patched %s and %s -> %s" % (MODULE, MODULE_LIST, dst))


if __name__ == "__main__":
    main()
