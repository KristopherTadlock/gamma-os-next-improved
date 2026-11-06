#!/usr/bin/env python3
"""Reads and edits raw (non-sparse) Android super images in place.

  lptool.py list SUPER
  lptool.py extract SUPER PARTITION OUT
  lptool.py replace SUPER PARTITION IMAGE

`replace` writes IMAGE over the partition's existing extents, so IMAGE must
not be larger than the partition. The partition metadata is left untouched.
"""

import struct
import sys

GEOMETRY_OFFSET = 4096
GEOMETRY_MAGIC = 0x616C4467
METADATA_OFFSET = 4096 * 3
HEADER_MAGIC = 0x414C5030
SECTOR = 512
CHUNK = 64 << 20


def read_partitions(f):
    f.seek(GEOMETRY_OFFSET)
    magic, = struct.unpack("<I", f.read(4))
    if magic != GEOMETRY_MAGIC:
        sys.exit("not a raw super image (bad geometry magic)")

    f.seek(METADATA_OFFSET)
    header = f.read(256)
    magic, _major, _minor, header_size = struct.unpack_from("<IHHI", header, 0)
    if magic != HEADER_MAGIC:
        sys.exit("bad metadata header magic")
    # Table descriptors (offset, count, entry size) follow the two checksums.
    descs = struct.unpack_from("<12I", header, 80)
    part_off, part_n, part_sz, ext_off, ext_n, ext_sz = descs[:6]

    f.seek(METADATA_OFFSET + header_size)
    tables = f.read(max(part_off + part_n * part_sz, ext_off + ext_n * ext_sz))

    extents = []
    for i in range(ext_n):
        sectors, kind, start, _source = struct.unpack_from(
            "<QIQI", tables, ext_off + i * ext_sz
        )
        extents.append((kind, start * SECTOR, sectors * SECTOR))

    partitions = {}
    for i in range(part_n):
        raw = tables[part_off + i * part_sz : part_off + (i + 1) * part_sz]
        name = raw[:36].split(b"\0")[0].decode()
        first, count = struct.unpack_from("<II", raw, 40)
        partitions[name] = extents[first : first + count]
    return partitions


def get(partitions, name):
    if name not in partitions:
        sys.exit(f"no partition {name!r}; have {', '.join(partitions)}")
    exts = partitions[name]
    if any(kind != 0 for kind, _, _ in exts):
        sys.exit(f"{name} has non-linear extents")
    return exts


def cmd_list(f):
    for name, exts in read_partitions(f).items():
        size = sum(length for _, _, length in exts)
        print(f"{name:24s} {size:>12d}  extents={len(exts)}")


def cmd_extract(f, name, out_path):
    with open(out_path, "wb") as out:
        for _, offset, length in get(read_partitions(f), name):
            f.seek(offset)
            while length:
                buf = f.read(min(length, CHUNK))
                out.write(buf)
                length -= len(buf)


def cmd_replace(f, name, image_path):
    exts = get(read_partitions(f), name)
    capacity = sum(length for _, _, length in exts)
    with open(image_path, "rb") as img:
        img.seek(0, 2)
        size = img.tell()
        if size > capacity:
            sys.exit(f"{image_path} is {size} bytes; {name} holds {capacity}")
        img.seek(0)
        for _, offset, length in exts:
            f.seek(offset)
            while length:
                buf = img.read(min(length, CHUNK)) or b"\0" * min(length, CHUNK)
                f.write(buf)
                length -= len(buf)


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    cmd, super_path, *args = sys.argv[1:]
    mode = "r+b" if cmd == "replace" else "rb"
    with open(super_path, mode) as f:
        {"list": cmd_list, "extract": cmd_extract, "replace": cmd_replace}[cmd](
            f, *args
        )


if __name__ == "__main__":
    main()
