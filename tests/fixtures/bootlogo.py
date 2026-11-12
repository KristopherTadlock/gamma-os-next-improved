"""Builds small RG Cube logo partitions (see scripts/bootlogo.py): "GZ", a picture count, 20
reserved bytes, a size per picture, then each picture as a gzip member of a square 24-bit BMP."""
import gzip
import io
import struct


def bmp(size, color):
    row = (size * 3 + 3) & ~3
    body = (bytes(color) * size).ljust(row, b"\0") * size + b"\0\0"
    hdr = b"BM" + struct.pack("<IHHI", 54 + len(body), 0, 0, 54)
    hdr += struct.pack("<IiiHHIIiiII", 40, size, size, 1, 24, 0, len(body), 2834, 2834, 0, 0)
    return hdr + body


def gz(name, data):
    out = io.BytesIO()
    with gzip.GzipFile(filename=name, mode="wb", fileobj=out, mtime=0) as g:
        g.write(data)
    return out.getvalue()


def build(path, size=6, count=5, padding=4096):
    """Writes a logo partition of COUNT SIZExSIZE pictures; returns the gzip members."""
    pics = [gz("%d.bmp" % (i + 1), bmp(size, (i, 2 * i, 3 * i))) for i in range(count)]
    blob = b"GZ" + struct.pack("<H", count) + b"\0" * 20
    blob += struct.pack("<%dI" % count, *map(len, pics)) + b"".join(pics)
    with open(path, "wb") as f:
        f.write(blob + b"\0" * padding)
    return pics


def read(path):
    """The gzip members of a logo partition."""
    with open(path, "rb") as f:
        d = f.read()
    n, = struct.unpack_from("<H", d, 2)
    sizes = struct.unpack_from("<%dI" % n, d, 24)
    off, out = 24 + 4 * n, []
    for s in sizes:
        out.append(d[off:off + s])
        off += s
    return out
