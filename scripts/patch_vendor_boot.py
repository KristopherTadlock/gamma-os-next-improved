#!/usr/bin/env python3
"""Patches the RG Cube 1.2 vendor_boot's DisplayPort driver (sprd-drm.ko).

  patch_vendor_boot.py IN OUT

Plugging in a monitor made the vendor composer set the largest mode the monitor
lists. For any 3840x2160 mode the driver goes into bypass and reads the plane's
framebuffer format, but the composer's hotplug commit has no framebuffer, so
the kernel panicked; and 4K at 60 Hz needs more than the Cube's two DP lanes.

sprd_dp_connector_mode_valid:
  - rejects modes with a pixel clock above 4K at 30 Hz (about 297 MHz);
  - keeps the monitor's preferred mode preferred, instead of clearing it and
    preferring CEA 1080p60 only, so the composer picks the monitor's own mode
    when it fits.
sprd_dp_encoder_mode_set:
  - treats a missing framebuffer in bypass as RGB instead of reading through it.

The ramdisk is otherwise byte-identical. The image is repacked and re-signed
with the AOSP test key it was signed with, keeping its salt and properties.

Needs on PATH: unpack_bootimg, lz4, avbtool. MKBOOTIMG names mkbootimg (or
mkbootimg.py) and AVB_KEY the key (external/avb/test/data/testkey_rsa4096.pem).
"""
import hashlib
import os
import shlex
import struct
import subprocess
import sys
import tempfile

MODULE = "lib/modules/sprd-drm.ko"
MODULE_SHA256 = "4d60c3c021bdd779cd55502b73fbe849eb2e773f7b9671dbd843444d8ce5f932"
TEXT_OFFSET = 0x30000   # .text in the module file
PARTITION_SIZE = 104857600

# (address in .text, original words, patched words)
PATCHES = [
    # mode_valid: clock > 0x49000 kHz -> MODE_CLOCK_HIGH; keep the type's preferred bit.
    (0x8cdc, [0xb9403668, 0x36180068, 0x121c7908, 0xb9003668],
             [0xb9403a6a, 0x7141255f, 0x540007a8, 0xb9403668]),
    # mode_valid: CEA 1080p60 no longer made preferred.
    (0x8d74, [0x321d0108], [0xd503201f]),
    # mode_valid: the CEA 4K60 branch (now unreachable) becomes the MODE_CLOCK_HIGH return.
    (0x8dd8, [0xb9403a69, 0x52820a0a, 0x72a0012a, 0x6b0a013f, 0x54000361, 0x39431289],
             [0xa9447bfd, 0xa9454ff4, 0xf85f8e5e, 0x528001e0, 0x910183ff, 0xd65f03c0]),
    # encoder_mode_set: NULL framebuffer in bypass -> RGB, 8 bpc.
    (0x90f4, [0xf94042c8, 0x528ac9c9, 0x72a64629, 0xf9426d08, 0xf9400908, 0xf9402508,
              0xb9400108, 0x6b09011f, 0x54000100, 0x52860a09, 0x72a60629, 0x6b09011f,
              0x540000c1, 0x52800028, 0x52800149, 0x14000005, 0x52800028, 0x14000002,
              0x2a1f03e8, 0x52800109],
             [0xf94042c8, 0xf9426d08, 0xf9400908, 0xb40001e8, 0xf9402508, 0xb940010a,
              0x52800028, 0x528ac9c9, 0x72a64629, 0x6b09015f, 0x54000120, 0x52860a09,
              0x72a60629, 0x6b09015f, 0x54000081, 0x52800149, 0x14000004, 0xd503201f,
              0x2a1f03e8, 0x52800109]),
]


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


def patch_module(ko):
    if hashlib.sha256(ko).hexdigest() != MODULE_SHA256:
        sys.exit("%s is not the 1.2 module this patch is for" % MODULE)
    ko = bytearray(ko)
    for addr, old, new in PATCHES:
        off = TEXT_OFFSET + addr
        n = len(old)
        if list(struct.unpack_from("<%dI" % n, ko, off)) != old:
            sys.exit("unexpected code at 0x%x" % addr)
        struct.pack_into("<%dI" % n, ko, off, *new)
    return bytes(ko)


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
    print("patched %s -> %s" % (MODULE, dst))


if __name__ == "__main__":
    main()
