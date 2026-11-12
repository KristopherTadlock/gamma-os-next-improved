#!/usr/bin/env python3
"""Draws Android's boot animation from the Thimsy logo.

  make_bootanimation.py LOGO_SVG ZIP [SOURCE_DIR]

LOGO_SVG is the Thimsy logo (frameworks/base/cmds/gammaos-nano/bootscreen/thimsy.svg
in the AOSP tree). Writes ZIP, the bootanimation.zip mods/58-bootanimation.sh
installs (mods/bootanimation/bootanimation.zip). With SOURCE_DIR (the AOSP
tree's vendor/lineage/bootanimation), also writes the bootanimation.tar and
desc.txt its Android.mk builds the same zip from.

The logo, white, fades in over half a second in the middle of Nano's Midnight
background (#1a1a2e, the boot screen's colour), then stays until Android has
started. Like the GammaOS animation it replaces, the frames are drawn at twice
the 480x480 square desc.txt asks for, and the logo is half the square's height.

Needs only Python 3: the logo is one path of straight lines (M, L and Z, filled
even-odd), which this rasterises itself. The output is the same on every run.
"""

import io
import re
import struct
import sys
import tarfile
import zipfile
import zlib

SIZE = 960            # frame size in pixels
DRAWN = 480           # the square desc.txt draws the frames into
LOGO_HEIGHT = 0.5     # of the frame, as the GammaOS cube
BACKGROUND = (0x1A, 0x1A, 0x2E)
FOREGROUND = (0xFF, 0xFF, 0xFF)
FPS = 20
FADE_FRAMES = 10
SUBSAMPLES = 16       # sub-scanlines per pixel row
LEVELS = 200          # coverage levels; mogrify -colors 250 then changes nothing
DESC = (f"{DRAWN} {DRAWN} {FPS}\n"
        f"p 1 0 part0 #{'%02x%02x%02x' % BACKGROUND}\n"
        f"p 0 0 part1 #{'%02x%02x%02x' % BACKGROUND}\n")


def read_polygons(svg):
    text = open(svg, encoding="utf-8").read()
    paths = re.findall(r'<path\b[^>]*\bd="([^"]*)"', text)
    if len(paths) != 1:
        sys.exit(f"{svg}: expected one path, found {len(paths)}")
    polygons, current = [], []
    for cmd, args in re.findall(r"([A-Za-z])([^A-Za-z]*)", paths[0]):
        nums = [float(n) for n in re.findall(r"-?\d*\.?\d+(?:[eE][-+]?\d+)?", args)]
        if cmd in "ML":
            if cmd == "M" and current:
                polygons.append(current)
                current = []
            current += list(zip(nums[0::2], nums[1::2]))
        elif cmd in "Zz":
            if current:
                polygons.append(current)
            current = []
        else:
            sys.exit(f"{svg}: unsupported path command {cmd}")
    if current:
        polygons.append(current)
    return polygons


def coverage(polygons):
    """Even-odd coverage of each pixel, 0..1, row by row."""
    xs = [x for p in polygons for x, _ in p]
    ys = [y for p in polygons for _, y in p]
    scale = SIZE * LOGO_HEIGHT / (max(ys) - min(ys))
    cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    edges = []
    for p in polygons:
        pts = [((x - cx) * scale + SIZE / 2, (y - cy) * scale + SIZE / 2) for x, y in p]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:] + pts[:1]):
            if y0 == y1:
                continue
            if y0 > y1:
                x0, y0, x1, y1 = x1, y1, x0, y0
            edges.append((y0, y1, x0, (x1 - x0) / (y1 - y0)))
    edges.sort()
    rows, active, nxt = [], [], 0
    for row in range(SIZE):
        cov = [0.0] * (SIZE + 1)
        run = [0] * (SIZE + 1)
        for k in range(SUBSAMPLES):
            y = row + (k + 0.5) / SUBSAMPLES
            while nxt < len(edges) and edges[nxt][0] <= y:
                active.append(edges[nxt])
                nxt += 1
            active = [e for e in active if e[1] > y]
            hits = sorted(x0 + (y - y0) * slope for y0, _, x0, slope in active if y0 <= y)
            for xa, xb in zip(hits[0::2], hits[1::2]):
                xa, xb = max(xa, 0.0), min(xb, float(SIZE))
                if xb <= xa:
                    continue
                ia, ib = int(xa), min(int(xb), SIZE - 1)
                if ia == ib:
                    cov[ia] += xb - xa
                else:
                    cov[ia] += ia + 1 - xa
                    run[ia + 1] += 1
                    run[ib] -= 1
                    cov[ib] += xb - ib
        full, out = 0, []
        for x in range(SIZE):
            full += run[x]
            out.append(min(1.0, (cov[x] + full) / SUBSAMPLES))
        rows.append(out)
    return rows


def png(indices, palette):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    raw = b"".join(b"\0" + bytes(r) for r in indices)
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 3, 0, 0, 0))
            + chunk(b"PLTE", b"".join(bytes(c) for c in palette))
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))


def frames(rows):
    levels = [[round(c * LEVELS) for c in r] for r in rows]
    def frame(alpha):
        palette = [tuple(round(b + (f - b) * alpha * q / LEVELS) for b, f in zip(BACKGROUND, FOREGROUND))
                   for q in range(LEVELS + 1)]
        return png(levels, palette)
    fade = []
    for i in range(FADE_FRAMES):
        t = (i + 1) / FADE_FRAMES
        fade.append(frame(t * t * (3 - 2 * t)))
    full = fade[-1]
    return [(f"part0/{i + 1:09d}.png", f) for i, f in enumerate(fade)] + [("part1/000000001.png", full)]


def write_zip(path, files):
    def entry(name):
        info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_STORED
        info.external_attr = (0o40755 if name.endswith("/") else 0o100644) << 16
        return info
    with zipfile.ZipFile(path, "w") as z:
        z.writestr(entry("desc.txt"), DESC)
        for part in ("part0/", "part1/"):
            z.writestr(entry(part), b"")
            for name, data in files:
                if name.startswith(part):
                    z.writestr(entry(name), data)


def write_tar(path, files):
    def entry(name, size=0):
        info = tarfile.TarInfo(name)
        info.mtime = 0
        info.uname = info.gname = "root"
        if size is None:
            info.type, info.mode = tarfile.DIRTYPE, 0o755
        else:
            info.size, info.mode = size, 0o644
        return info
    with tarfile.open(path, "w", format=tarfile.USTAR_FORMAT) as t:
        for part in ("part0", "part1"):
            t.addfile(entry(part, None))
            for name, data in files:
                if name.startswith(part + "/"):
                    t.addfile(entry(name, len(data)), io.BytesIO(data))


def main():
    if len(sys.argv) not in (3, 4):
        sys.exit(__doc__.split("\n\n")[1])
    files = frames(coverage(read_polygons(sys.argv[1])))
    write_zip(sys.argv[2], files)
    if len(sys.argv) == 4:
        write_tar(f"{sys.argv[3]}/bootanimation.tar", files)
        with open(f"{sys.argv[3]}/desc.txt", "w") as f:
            f.write(DESC)


if __name__ == "__main__":
    main()
