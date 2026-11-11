#!/usr/bin/env python3
"""Puts a precompiled SELinux policy matching SYSTEM on the odm partition.

  odm_sepolicy.py ODM_IN SYSTEM_IMG VENDOR_IMG ODM_OUT

At boot init loads /odm/etc/selinux/precompiled_sepolicy only when the hashes stored beside it
match the system's plat (and system_ext, product) policy hashes; otherwise it compiles the policy
from system, vendor and odm on every boot, about a second before anything else can start. The
RG Cube's odm carries the policy built for its 1.2 system. This compiles it the way init would
(system/core/init/selinux.cpp, CompileSplitPolicy: the same files, order and secilc flags) and
writes it, with the system's hashes, into a rebuilt odm (erofs_image.py).

SYSTEM_IMG and VENDOR_IMG are ext4 images (system as root, so its policy is under /system).
Needs on PATH: secilc (the AOSP host build's), debugfs, fsck.erofs and mkfs.erofs.
"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from erofs_image import rebuild  # noqa: E402

SEPOLICY_VERSION = "30"   # system/core/init/Android.bp, -DSEPOLICY_VERSION
PRECOMPILED = "etc/selinux/precompiled_sepolicy"


def dump(image, path, dest):
    """Copies directory PATH of an ext4 image to DEST; False when it has none."""
    os.makedirs(dest, exist_ok=True)
    subprocess.run(["debugfs", "-R", "rdump %s %s" % (path, dest), image], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    got = os.path.join(dest, os.path.basename(path))
    return got if os.path.isdir(got) else None


def first_line(path):
    with open(path) as f:
        return f.readline().strip()


def main():
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    odm_in, system_img, vendor_img, odm_out = sys.argv[1:]

    with tempfile.TemporaryDirectory() as tmp:
        def part(image, path, name):
            return dump(image, path, os.path.join(tmp, name))
        plat = part(system_img, "/system/etc/selinux", "plat")
        system_ext = part(system_img, "/system/system_ext/etc/selinux", "system_ext")
        product = part(system_img, "/system/product/etc/selinux", "product")
        vendor = part(vendor_img, "/etc/selinux", "vendor")
        if not plat or not vendor:
            sys.exit("odm_sepolicy: no SELinux policy in the system or vendor image")
        odm_tree = os.path.join(tmp, "odm")
        subprocess.run(["fsck.erofs", "--extract=" + odm_tree, "--no-preserve", odm_in],
                       check=True, stdout=subprocess.DEVNULL)
        odm = os.path.join(odm_tree, "etc/selinux")
        if not os.path.isfile(os.path.join(odm_tree, PRECOMPILED)):
            sys.exit("odm_sepolicy: the odm image has no %s to replace" % PRECOMPILED)

        def have(d, name):
            p = os.path.join(d, name) if d else None
            return p if p and os.path.isfile(p) else None

        vers = first_line(os.path.join(vendor, "plat_sepolicy_vers.txt"))
        vendor_cil = have(vendor, "vendor_sepolicy.cil")
        pub_cil = have(vendor, "plat_pub_versioned.cil")
        plat_mapping = os.path.join(plat, "mapping", vers + ".cil")
        if not vendor_cil or not pub_cil or not os.path.isfile(plat_mapping):
            sys.exit("odm_sepolicy: missing vendor policy or mapping %s.cil" % vers)
        out = os.path.join(tmp, "precompiled_sepolicy")
        args = ["secilc", os.path.join(plat, "plat_sepolicy.cil"),
                "-m", "-M", "true", "-G", "-N", "-c", SEPOLICY_VERSION,
                plat_mapping, "-o", out, "-f", os.devnull]
        args += [p for p in (
            have(os.path.join(plat, "mapping"), vers + ".compat.cil"),
            have(system_ext, "system_ext_sepolicy.cil"),
            have(system_ext and os.path.join(system_ext, "mapping"), vers + ".cil"),
            have(system_ext and os.path.join(system_ext, "mapping"), vers + ".compat.cil"),
            have(product, "product_sepolicy.cil"),
            have(product and os.path.join(product, "mapping"), vers + ".cil"),
            pub_cil, vendor_cil, have(odm, "odm_sepolicy.cil"),
        ) if p]
        subprocess.run(args, check=True)
        with open(out, "rb") as f:
            policy = f.read()
        plat_hash = first_line(os.path.join(plat, "plat_sepolicy_and_mapping.sha256"))

        # The hashes init compares, one per partition that has a policy: the image's file and
        # odm's copy must both exist or both be absent (FindPrecompiledSplitPolicy).
        changes = {PRECOMPILED: lambda _old: policy}
        for d, name in ((plat, "plat_sepolicy_and_mapping.sha256"),
                        (system_ext, "system_ext_sepolicy_and_mapping.sha256"),
                        (product, "product_sepolicy_and_mapping.sha256")):
            src = have(d, name)
            dst = "etc/selinux/precompiled_sepolicy." + name
            on_odm = os.path.isfile(os.path.join(odm_tree, dst))
            if bool(src) != on_odm:
                sys.exit("odm_sepolicy: %s is %s on odm but %s in the system image" % (
                    dst, "present" if on_odm else "absent", "present" if src else "absent"))
            if src:
                with open(src, "rb") as f:
                    data = f.read()
                changes[dst] = lambda _old, data=data: data

    new = rebuild(odm_in, "odm", changes)
    with open(odm_out, "wb") as f:
        f.write(new)
    print("odm: precompiled SELinux policy for plat %s, vendor %s (%d bytes)" % (
        plat_hash[:12], vers, len(policy)))


if __name__ == "__main__":
    main()
