"""The RG Cube 1.2 vendor kernel's DisplayPort driver (sprd-drm.ko) and its patch.

Plugging in a monitor made the vendor composer set the largest mode the monitor
lists. For any 3840x2160 mode the driver goes into bypass and reads the plane's
framebuffer format, but the composer's hotplug commit has no framebuffer, so
the kernel panicked; and 4K at 60 Hz needs more than the Cube's two DP lanes.

sprd_dp_connector_mode_valid:
  - rejects modes with a pixel clock above 4K at 30 Hz (about 297 MHz);
  - keeps CEA 1080p60 the only preferred mode, as the vendor driver does, so
    the composer sets 1080p at 60 Hz when a monitor connects; the monitor's own
    preferred mode becomes user-defined instead of being dropped, since the
    vendor composer lists only preferred and user-defined modes.
sprd_dp_encoder_mode_set:
  - treats a missing framebuffer in bypass as RGB instead of reading through it.

The module is in vendor_dlkm (loaded at boot) and in vendor_boot (recovery).
"""
import hashlib
import struct
import sys

MODULE_SHA256 = "4d60c3c021bdd779cd55502b73fbe849eb2e773f7b9671dbd843444d8ce5f932"
TEXT_OFFSET = 0x30000   # .text in the module file

# (address in .text, original words, patched words)
PATCHES = [
    # mode_valid: clock > 0x49000 kHz -> MODE_CLOCK_HIGH; keep the type's preferred bit.
    (0x8cdc, [0xb9403668, 0x36180068, 0x121c7908, 0xb9003668],
             [0xb9403a6a, 0x7141255f, 0x540007a8, 0xb9403668]),
    # mode_valid: every mode but CEA 1080p60 (other CEA codes, non-CEA modes, a
    # 1080p at another clock, 640x480 at another clock) goes to the stub below,
    # which makes a preferred mode user-defined, so the composer still lists it
    # and CEA 1080p60 stays the only preferred mode.
    (0x8d04, [0x54000a81], [0x54000461]),   # b.ne stub
    (0x8d18, [0x540009e1], [0x540003c1]),   # b.ne stub
    (0x8d88, [0x54000661], [0x54000041]),   # b.ne stub
    # CEA 640x480 skips its log line (its space holds the stub) and stays
    # user-defined; then the stub.
    (0x8d8c, [0xb940b663, 0x29479a65, 0x2948a667, 0x294a2e6a, 0x294b366c, 0xb940666e],
             [0x1400002f,               # b    0x8e48: type |= user-defined
              0xb9403668,               # ldr  w8, [x19, #0x34]   (type)
              0x36180608,               # tbz  w8, #3, return     (not preferred)
              0x121c7908,               # and  w8, w8, #~preferred
              0x321b0108,               # orr  w8, w8, #user-defined
              0x1400002c]),             # b    0x8e50: store type, return
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


def patch_module(ko):
    if hashlib.sha256(ko).hexdigest() != MODULE_SHA256:
        sys.exit("sprd-drm.ko is not the 1.2 module this patch is for")
    ko = bytearray(ko)
    for addr, old, new in PATCHES:
        off = TEXT_OFFSET + addr
        n = len(old)
        if list(struct.unpack_from("<%dI" % n, ko, off)) != old:
            sys.exit("unexpected code at 0x%x" % addr)
        struct.pack_into("<%dI" % n, ko, off, *new)
    return bytes(ko)
