#!/usr/bin/env python3
"""Puts a new boot picture in the RG Cube's logo partition (BootLogo, logo.img).

  bootlogo.py LOGO_IN FRAME.rgba LOGO_OUT

The bootloader shows picture 4 of this partition from power-on until Android draws. The
partition holds "GZ", a 16-bit picture count, 20 reserved bytes and a 32-bit size per picture,
then each picture as a gzip member (named <n>.bmp) of a 720x720 24-bit BMP; the rest is padding.
Pictures 1-3 are the charging screens and are kept as they are.

FRAME.rgba is the screen as it should look (720x720 RGBA, row 0 at the top), such as Nano's boot
screen drawn by AOSP's nano-bootlogo host tool. It is stored a quarter turn clockwise, as the
panel scans and as the stock picture is, in a BMP laid out like the stock ones.
"""
import gzip
import io
import struct
import sys

BOOT_PICTURE = 3   # 0-based: 4.bmp


def pictures(d):
    if d[:2] != b"GZ":
        sys.exit("bootlogo: not a GZ logo partition")
    n = struct.unpack_from("<H", d, 2)[0]
    sizes = struct.unpack_from("<%dI" % n, d, 24)
    off, out = 24 + 4 * n, []
    for s in sizes:
        out.append(d[off:off + s])
        off += s
    if n <= BOOT_PICTURE or off > len(d):
        sys.exit("bootlogo: unexpected logo partition layout")
    return out


def bmp(frame, size):
    """FRAME (size x size RGBA, top row first), a quarter turn clockwise, as a bottom-up BGR BMP."""
    row = (size * 3 + 3) & ~3
    body = bytearray(row * size)
    for y in range(size):              # stored row y (from the top) ...
        dst = (size - 1 - y) * row     # ... is BMP row size-1-y (bottom-up)
        for x in range(size):
            # a clockwise quarter turn: stored (x, y) shows upright pixel (y, size-1-x)
            p = ((size - 1 - x) * size + y) * 4
            body[dst + x * 3:dst + x * 3 + 3] = bytes((frame[p + 2], frame[p + 1], frame[p]))
    body += b"\0\0"                    # as the stock pictures
    hdr = b"BM" + struct.pack("<IHHI", 54 + len(body), 0, 0, 54)
    hdr += struct.pack("<IiiHHIIiiII", 40, size, size, 1, 24, 0, len(body), 2834, 2834, 0, 0)
    return hdr + bytes(body)


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    logo_in, frame_path, logo_out = sys.argv[1:]
    d = open(logo_in, "rb").read()
    pics = pictures(d)
    old = gzip.decompress(pics[BOOT_PICTURE])
    w, h = struct.unpack_from("<ii", old, 18)
    frame = open(frame_path, "rb").read()
    if w != h or len(frame) != w * h * 4:
        sys.exit("bootlogo: the frame must be %dx%d RGBA" % (w, abs(h)))
    out = io.BytesIO()
    with gzip.GzipFile(filename="%d.bmp" % (BOOT_PICTURE + 1), mode="wb", fileobj=out, mtime=0) as g:
        g.write(bmp(frame, w))
    pics[BOOT_PICTURE] = out.getvalue()
    blob = b"GZ" + struct.pack("<H", len(pics)) + b"\0" * 20
    blob += struct.pack("<%dI" % len(pics), *map(len, pics)) + b"".join(pics)
    if len(blob) > len(d):
        sys.exit("bootlogo: the pictures do not fit the partition")
    with open(logo_out, "wb") as f:
        f.write(blob + b"\0" * (len(d) - len(blob)))


if __name__ == "__main__":
    main()
