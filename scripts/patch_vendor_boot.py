#!/usr/bin/env python3
"""Patches the RG Cube 1.2 vendor_boot: its DisplayPort driver (sprd-drm.ko) and its first-stage
module list.

  patch_vendor_boot.py IN OUT

Applies sprd_drm.py's patch to the copy recovery loads from the vendor ramdisk. The first stage no
longer loads the LED driver (cust-leds.ko): its probe takes a second, during which nothing else in
the boot could run. It loads later, in the background, from vendor_dlkm (patch_vendor_dlkm.py);
recovery still loads it from here (modules.load.recovery).

The ramdisk is otherwise byte-identical. The image is repacked and re-signed
with the AOSP test key it was signed with, keeping its salt and properties.

Needs on PATH: unpack_bootimg, lz4, avbtool. MKBOOTIMG names mkbootimg (or
mkbootimg.py) and AVB_KEY the key (external/avb/test/data/testkey_rsa4096.pem).
"""
import os
import shlex
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sprd_drm import patch_module  # noqa: E402

MODULE = "lib/modules/sprd-drm.ko"
FIRST_STAGE = "lib/modules/modules.load"
DEFERRED = "cust-leds.ko"
PARTITION_SIZE = 104857600


def run(*cmd, **kw):
    return subprocess.run(cmd, check=True, **kw)


def find_cpio_entry(cpio, name):
    """Offset and size of a newc cpio entry's data."""
    pos = 0
    while pos + 110 <= len(cpio):
        hdr = cpio[pos:pos + 110]
        if hdr[:6] not in (b"070701", b"070702"):
            sys.exit("vendor ramdisk: bad cpio header at %d" % pos)
        size = int(hdr[54:62], 16)
        namesize = int(hdr[94:102], 16)
        entry = cpio[pos + 110:pos + 110 + namesize - 1].decode()
        data = (pos + 110 + namesize + 3) & ~3
        if entry == name:
            return data, size
        if entry == "TRAILER!!!":
            break
        pos = (data + size + 3) & ~3
    sys.exit("vendor ramdisk: no %s" % name)


def replace_cpio_entry(cpio, name, data):
    """CPIO with the newc entry NAME's data replaced by DATA (its size may change)."""
    pos = 0
    while pos + 110 <= len(cpio):
        hdr = cpio[pos:pos + 110]
        if hdr[:6] not in (b"070701", b"070702"):
            sys.exit("vendor ramdisk: bad cpio header at %d" % pos)
        size = int(hdr[54:62], 16)
        namesize = int(hdr[94:102], 16)
        entry = cpio[pos + 110:pos + 110 + namesize - 1].decode()
        off = (pos + 110 + namesize + 3) & ~3
        end = (off + size + 3) & ~3
        if entry == name:
            pad = b"\0" * (((len(data) + 3) & ~3) - len(data))
            header = hdr[:54] + b"%08X" % len(data) + hdr[62:]
            return cpio[:pos] + header + cpio[pos + 110:off] + data + pad + cpio[end:]
        if entry == "TRAILER!!!":
            break
        pos = end
    sys.exit("vendor ramdisk: no %s" % name)


def drop_deferred(data):
    mods = data.decode().split()
    if DEFERRED not in mods:
        sys.exit("vendor ramdisk: the first stage does not load %s" % DEFERRED)
    return ("\n".join(m for m in mods if m != DEFERRED) + "\n").encode()


def footer_info(image):
    out = run("avbtool", "info_image", "--image", image, capture_output=True, text=True).stdout
    salt = props = None
    props = []
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("Salt:"):
            salt = line.split()[1]
        elif line.startswith("Prop:"):
            key, _, value = line[len("Prop:"):].strip().partition(" -> ")
            props.append((key, value.strip("'")))
    if not salt:
        sys.exit("vendor_boot: no hash footer salt")
    return salt, props


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    src, dst = sys.argv[1], sys.argv[2]
    mkbootimg = shlex.split(os.environ.get("MKBOOTIMG", "mkbootimg"))
    if mkbootimg[0].endswith(".py"):
        mkbootimg = [sys.executable] + mkbootimg
    key = os.environ.get("AVB_KEY")
    if not key or not os.path.isfile(key):
        sys.exit("AVB_KEY must name the AOSP testkey_rsa4096.pem")
    salt, props = footer_info(src)

    with tempfile.TemporaryDirectory() as tmp:
        unpacked = os.path.join(tmp, "unpacked")
        args = run("unpack_bootimg", "--boot_img", src, "--out", unpacked, "--format", "mkbootimg",
                   capture_output=True, text=True).stdout
        args = shlex.split(args)
        frags = [i for i, a in enumerate(args) if a == "--vendor_ramdisk_fragment"]
        if len(frags) != 1:
            sys.exit("vendor_boot: expected one ramdisk fragment")
        ramdisk = args[frags[0] + 1]

        cpio = run("lz4", "-dc", ramdisk, capture_output=True).stdout
        off, size = find_cpio_entry(cpio, MODULE)
        cpio = cpio[:off] + patch_module(cpio[off:off + size]) + cpio[off + size:]
        off, size = find_cpio_entry(cpio, FIRST_STAGE)
        cpio = replace_cpio_entry(cpio, FIRST_STAGE, drop_deferred(cpio[off:off + size]))
        cpio_path = os.path.join(tmp, "ramdisk.cpio")
        with open(cpio_path, "wb") as f:
            f.write(cpio)
        new_ramdisk = os.path.join(tmp, "ramdisk.lz4")
        run("lz4", "-l", "-12", "--favor-decSpeed", "-f", "-q", cpio_path, new_ramdisk)
        args[frags[0] + 1] = new_ramdisk

        run(*mkbootimg, *args, "--vendor_boot", dst)
        cmd = ["avbtool", "add_hash_footer", "--image", dst, "--partition_name", "vendor_boot",
               "--partition_size", str(PARTITION_SIZE), "--algorithm", "SHA256_RSA4096",
               "--key", key, "--salt", salt]
        for k, v in props:
            cmd += ["--prop", "%s:%s" % (k, v)]
        run(*cmd)

        # The result must unpack to the patched module.
        check = os.path.join(tmp, "check")
        run("unpack_bootimg", "--boot_img", dst, "--out", check, capture_output=True)
        cpio2 = run("lz4", "-dc", os.path.join(check, "vendor_ramdisk00"), capture_output=True).stdout
        if cpio2 != cpio:
            sys.exit("vendor_boot: repacked ramdisk does not match")
    print("patched %s and %s -> %s" % (MODULE, FIRST_STAGE, dst))


if __name__ == "__main__":
    main()
