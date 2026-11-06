#!/usr/bin/env python3
"""Reads and edits Unisoc BP_R2 .pac firmware packages in place.

  pactool.py list PAC
  pactool.py extract PAC FILE OUT
  pactool.py replace PAC FILE IMAGE
  pactool.py fixcrc PAC

`replace` overwrites FILE's data and requires IMAGE to be exactly the same
size, so no offsets change. Run `fixcrc` afterwards to update the data CRC.
Set CRC16ARC to the path of the compiled tools/crc16arc.c helper.
"""

import os
import struct
import subprocess
import sys

HEADER_SIZE = 2124
ENTRY_SIZE = 2580
CRC2_OFFSET = HEADER_SIZE - 2
CHUNK = 64 << 20


def utf16(raw):
    return raw.decode("utf-16le").split("\0")[0]


def read_entries(f):
    f.seek(0)
    header = f.read(HEADER_SIZE)
    if utf16(header[:48]) != "BP_R2.0.1":
        sys.exit(f"unsupported pac version {utf16(header[:48])!r}")
    count, start = struct.unpack_from("<II", header, 1076)
    f.seek(start)
    entries = []
    for _ in range(count):
        raw = f.read(ENTRY_SIZE)
        hi_size, hi_off, lo_size = struct.unpack_from("<III", raw, 1532)
        lo_off, = struct.unpack_from("<I", raw, 1552)
        entries.append(
            {
                "id": utf16(raw[4:516]),
                "file": utf16(raw[516:1028]),
                "size": (hi_size << 32) | lo_size,
                "offset": (hi_off << 32) | lo_off,
            }
        )
    return entries


def find(entries, name):
    for e in entries:
        if name in (e["file"], e["id"]) and e["size"]:
            return e
    sys.exit(f"no file {name!r} in pac")


def copy(src, dst, length):
    while length:
        buf = src.read(min(length, CHUNK))
        if not buf:
            sys.exit("unexpected end of file")
        dst.write(buf)
        length -= len(buf)


def cmd_list(f):
    for e in read_entries(f):
        print(f"{e['id']:22s} {e['file']:32s} {e['size']:>12d}")


def cmd_extract(f, name, out_path):
    e = find(read_entries(f), name)
    f.seek(e["offset"])
    with open(out_path, "wb") as out:
        copy(f, out, e["size"])


def cmd_replace(f, name, image_path):
    e = find(read_entries(f), name)
    size = os.path.getsize(image_path)
    if size != e["size"]:
        sys.exit(f"{image_path} is {size} bytes; {name} must be {e['size']}")
    f.seek(e["offset"])
    with open(image_path, "rb") as img:
        copy(img, f, size)


def cmd_fixcrc(f):
    tool = os.environ.get("CRC16ARC", "crc16arc")
    out = subprocess.run(
        [tool, f.name, str(HEADER_SIZE)], check=True, capture_output=True, text=True
    )
    crc = int(out.stdout, 16)
    f.seek(CRC2_OFFSET)
    f.write(struct.pack("<H", crc))
    print(f"data crc {crc:04x}")


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    cmd, pac_path, *args = sys.argv[1:]
    mode = "r+b" if cmd in ("replace", "fixcrc") else "rb"
    commands = {
        "list": cmd_list,
        "extract": cmd_extract,
        "replace": cmd_replace,
        "fixcrc": cmd_fixcrc,
    }
    with open(pac_path, mode) as f:
        commands[cmd](f, *args)


if __name__ == "__main__":
    main()
