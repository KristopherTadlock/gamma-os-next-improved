#!/usr/bin/env python3
"""Draws the app icon: the Thimsy logo (tool/thimsy.svg, the boot screen's
logo) in white on the boot screen's background, Nano's "Midnight" #1a1a2e
(frameworks/base/cmds/gammaos-nano/bootscreen/NanoBootArt.h).

  python3 tool/make_icons.py        (from installer/app)

Writes the macOS AppIcon set, windows/runner/resources/app_icon.ico and
linux/runner/resources/app_icon.png. Standard library only: the logo is one
even-odd path of straight lines, drawn here with 16x16 supersampling and
rendered separately at every size (small sizes are not scaled-down copies).
"""
import os
import re
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
APP = os.path.dirname(HERE)
BG = (0x1A, 0x1A, 0x2E)
FG = (0xFF, 0xFF, 0xFF)


def supersampling(size):
    """Sub-scanlines per pixel: more for small icons, where edges matter most."""
    return 16 if size <= 128 else 8 if size <= 256 else 4


def load_polygons(path):
    d = re.search(r'\sd="([^"]+)"', open(path).read()).group(1)
    polys, cur = [], []
    for cmd, args in re.findall(r"([MLZ])([^MLZ]*)", d):
        nums = [float(v) for v in re.findall(r"-?[\d.]+", args)]
        if cmd == "M":
            if cur:
                polys.append(cur)
            cur = [(nums[0], nums[1])]
            nums = nums[2:]
            cmd = "L"  # extra pairs after M are lines
        if cmd == "L":
            cur += list(zip(nums[0::2], nums[1::2]))
        if cmd == "Z" and cur:
            polys.append(cur)
            cur = []
    if cur:
        polys.append(cur)
    return polys


def rounded_rect_coverage(x, y, x0, y0, x1, y1, r):
    """Coverage (0..1) of pixel (x, y) by a rounded rectangle, sampled."""
    n = 4
    hit = 0
    for sy in range(n):
        for sx in range(n):
            px, py = x + (sx + 0.5) / n, y + (sy + 0.5) / n
            if not (x0 <= px <= x1 and y0 <= py <= y1):
                continue
            cx = min(max(px, x0 + r), x1 - r)
            cy = min(max(py, y0 + r), y1 - r)
            if (px - cx) ** 2 + (py - cy) ** 2 <= r * r:
                hit += 1
    return hit / (n * n)


def render(polys, size, tile, radius, logo_frac):
    """RGBA bytes: a rounded tile (inset `tile` fraction of size, corner
    `radius` fraction of the tile) in BG, the logo scaled to `logo_frac` of
    the tile's width, centred."""
    xs = [x for p in polys for x, _ in p]
    ys = [y for p in polys for _, y in p]
    bx0, bx1, by0, by1 = min(xs), max(xs), min(ys), max(ys)
    t = size * tile
    t0 = (size - t) / 2
    scale = (t * logo_frac) / max(bx1 - bx0, by1 - by0)
    ox = size / 2 - (bx0 + bx1) / 2 * scale
    oy = size / 2 - (by0 + by1) / 2 * scale
    edges = []
    for p in polys:
        for (xa, ya), (xb, yb) in zip(p, p[1:] + p[:1]):
            xa, ya, xb, yb = xa * scale + ox, ya * scale + oy, xb * scale + ox, yb * scale + oy
            if ya != yb:
                edges.append((xa, ya, xb, yb))
    # Logo coverage by scanline with even-odd crossings.
    SS = supersampling(size)
    cov = [[0.0] * size for _ in range(size)]
    for sy in range(size * SS):
        y = (sy + 0.5) / SS
        xs_ = sorted(
            xa + (y - ya) * (xb - xa) / (yb - ya)
            for xa, ya, xb, yb in edges
            if (ya <= y < yb) or (yb <= y < ya)
        )
        row = cov[sy // SS]
        for a, b in zip(xs_[0::2], xs_[1::2]):
            # Exact horizontal coverage of [a, b) in pixel units, 1/SS tall.
            a, b = max(a, 0.0), min(b, float(size))
            if b <= a:
                continue
            for px in range(int(a), min(int(b) + 1, size)):
                lo, hi = max(a, px), min(b, px + 1)
                if hi > lo:
                    row[px] += (hi - lo) / SS
    out = bytearray()
    r = t * radius
    for y in range(size):
        for x in range(size):
            tile_a = rounded_rect_coverage(x, y, t0, t0, t0 + t, t0 + t, r)
            if tile_a == 0:
                out += b"\0\0\0\0"
                continue
            c = min(cov[y][x], 1.0)
            rgb = [round(BG[k] * (1 - c) + FG[k] * c) for k in range(3)]
            out += bytes(rgb) + bytes([round(255 * tile_a)])
    return bytes(out)


def png(size, rgba):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    raw = b"".join(b"\0" + rgba[y * size * 4:(y + 1) * size * 4] for y in range(size))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def ico(images):
    """ICO with PNG-encoded images (Windows Vista and later)."""
    head = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    dirs, data = b"", b""
    for size, blob in images:
        dirs += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(blob), offset + len(data))
        data += blob
    return head + dirs + data


def main():
    polys = load_polygons(os.path.join(HERE, "thimsy.svg"))
    # macOS: Apple's grid, an 824 px tile in 1024 with a ~185 px corner.
    mac = os.path.join(APP, "macos/Runner/Assets.xcassets/AppIcon.appiconset")
    for s in (16, 32, 64, 128, 256, 512, 1024):
        open(os.path.join(mac, f"app_icon_{s}.png"), "wb").write(png(s, render(polys, s, 824 / 1024, 0.225, 0.72)))
        print("macOS", s, file=sys.stderr)
    # Windows and Linux: a fuller tile.
    full = lambda s: render(polys, s, 0.94, 0.2, 0.74)
    images = [(s, png(s, full(s))) for s in (16, 24, 32, 48, 64, 128, 256)]
    open(os.path.join(APP, "windows/runner/resources/app_icon.ico"), "wb").write(ico(images))
    print("Windows ico", file=sys.stderr)
    os.makedirs(os.path.join(APP, "linux/runner/resources"), exist_ok=True)
    open(os.path.join(APP, "linux/runner/resources/app_icon.png"), "wb").write(png(256, full(256)))
    print("Linux png", file=sys.stderr)


if __name__ == "__main__":
    main()
