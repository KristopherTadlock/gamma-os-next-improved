#!/usr/bin/env python3
"""Reads and edits raw (non-sparse) Android super images in place.

  lptool.py list SUPER
  lptool.py extract SUPER PARTITION OUT
  lptool.py replace SUPER PARTITION IMAGE
  lptool.py grow SUPER PARTITION max|BYTES

`replace` writes IMAGE over the partition's existing extents, so IMAGE must
not be larger than the partition. The partition metadata is left untouched.

`grow` gives PARTITION one more extent in the free space after the last
partition, as large as the free space and the partition's group allow (or
BYTES, if smaller), and rewrites every metadata slot.
"""

import hashlib
import struct
import sys

GEOMETRY_OFFSET = 4096
GEOMETRY_MAGIC = 0x616C4467
METADATA_OFFSET = 4096 * 3
HEADER_MAGIC = 0x414C5030
SECTOR = 512
CHUNK = 64 << 20

PARTITION = struct.Struct("<36sIIII")  # name, attributes, first extent, extent count, group
EXTENT = struct.Struct("<QIQI")  # sectors, target type, target data (sector), target source
GROUP = struct.Struct("<36sIQ")  # name, flags, maximum size
DEVICE = struct.Struct("<QIIQ36sI")  # first logical sector, alignment, offset, size, name, flags


def read_metadata(f):
    f.seek(GEOMETRY_OFFSET)
    geometry = f.read(4096)
    magic, = struct.unpack_from("<I", geometry, 0)
    if magic != GEOMETRY_MAGIC:
        sys.exit("not a raw super image (bad geometry magic)")
    max_size, slots, block_size = struct.unpack_from("<III", geometry, 40)

    f.seek(METADATA_OFFSET)
    raw = f.read(max_size)
    magic, _major, _minor, header_size = struct.unpack_from("<IHHI", raw, 0)
    if magic != HEADER_MAGIC:
        sys.exit("bad metadata header magic")
    header = bytearray(raw[:header_size])
    tables_size, = struct.unpack_from("<I", header, 44)
    tables = raw[header_size : header_size + tables_size]

    check = bytearray(header)
    check[12:44] = bytes(32)
    if hashlib.sha256(check).digest() != header[12:44]:
        sys.exit("metadata header checksum mismatch")
    if hashlib.sha256(tables).digest() != header[48:80]:
        sys.exit("metadata tables checksum mismatch")

    # Table descriptors (offset, count, entry size) follow the two checksums.
    descs = struct.unpack_from("<12I", header, 80)

    def table(index, layout):
        offset, count, size = descs[index * 3 : index * 3 + 3]
        return [list(layout.unpack_from(tables, offset + i * size)) for i in range(count)]

    return {
        "max_size": max_size,
        "slots": slots,
        "block_size": block_size,
        "header": header,
        "partitions": table(0, PARTITION),
        "extents": table(1, EXTENT),
        "groups": table(2, GROUP),
        "devices": table(3, DEVICE),
    }


def write_metadata(f, meta):
    tables = b"".join(
        b"".join(layout.pack(*row) for row in meta[key])
        for key, layout in (("partitions", PARTITION), ("extents", EXTENT),
                            ("groups", GROUP), ("devices", DEVICE))
    )
    header = meta["header"]
    offset = 0
    for index, (key, layout) in enumerate((("partitions", PARTITION), ("extents", EXTENT),
                                           ("groups", GROUP), ("devices", DEVICE))):
        struct.pack_into("<III", header, 80 + index * 12, offset, len(meta[key]), layout.size)
        offset += len(meta[key]) * layout.size
    struct.pack_into("<I", header, 44, len(tables))
    header[48:80] = hashlib.sha256(tables).digest()
    header[12:44] = bytes(32)
    header[12:44] = hashlib.sha256(header).digest()

    blob = bytes(header) + tables
    if len(blob) > meta["max_size"]:
        sys.exit("metadata no longer fits in its slot")
    blob += bytes(meta["max_size"] - len(blob))
    # Primary slots, then backup slots, all identical.
    for slot in range(meta["slots"] * 2):
        f.seek(METADATA_OFFSET + slot * meta["max_size"])
        f.write(blob)


def name_of(raw):
    return raw.split(b"\0")[0].decode()


def read_partitions(f):
    meta = read_metadata(f)
    extents = [(kind, start * SECTOR, sectors * SECTOR) for sectors, kind, start, _ in meta["extents"]]
    return {
        name_of(name): extents[first : first + count]
        for name, _attrs, first, count, _group in meta["partitions"]
    }


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
                if not buf:
                    sys.exit(f"{name} extends past the end of the super image")
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


def cmd_grow(f, name, amount):
    meta = read_metadata(f)
    parts = meta["partitions"]
    index = next((i for i, p in enumerate(parts) if name_of(p[0]) == name), None)
    if index is None:
        sys.exit(f"no partition {name!r}")
    _, _, first, count, group = parts[index]
    if len(meta["devices"]) != 1:
        sys.exit("only single-device super images are supported")
    first_sector, alignment, align_offset, device_size, _, _ = meta["devices"][0]

    # Free space after the last extent, aligned like the existing extents.
    end = max([first_sector * SECTOR] + [(s + n) * SECTOR for n, _, s, _ in meta["extents"]])
    start = -(-(end - align_offset) // alignment) * alignment + align_offset
    size = device_size - start

    max_group = meta["groups"][group][2]
    if max_group:
        in_group = sum(
            meta["extents"][e][0] * SECTOR
            for p in parts if p[4] == group
            for e in range(p[2], p[2] + p[3])
        )
        size = min(size, max_group - in_group)
    if amount != "max":
        size = min(size, int(amount))
    size -= size % alignment
    if size <= 0:
        sys.exit(f"no free space to grow {name}")

    meta["extents"].insert(first + count, [size // SECTOR, 0, start // SECTOR, 0])
    parts[index][3] = count + 1
    for p in parts:
        if p is not parts[index] and p[2] >= first + count:
            p[2] += 1
    write_metadata(f, meta)
    print(f"grew {name} by {size} bytes at offset {start}")


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    cmd, super_path, *args = sys.argv[1:]
    mode = "r+b" if cmd in ("replace", "grow") else "rb"
    commands = {"list": cmd_list, "extract": cmd_extract, "replace": cmd_replace, "grow": cmd_grow}
    with open(super_path, mode) as f:
        commands[cmd](f, *args)


if __name__ == "__main__":
    main()
