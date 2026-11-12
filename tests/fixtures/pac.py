"""Builds small Unisoc BP_R2 .pac files, laid out like the RG Cube's (see scripts/pactool.py)."""
import struct

HEADER_SIZE, ENTRY_SIZE = 2124, 2580
DEFAULT_ENTRIES = [("FDL", "fdl.bin", b"F" * 1000), ("Super", "super_full.img", b"S" * 5000),
                   ("VENDORBOOT", "vendor_boot.img", b"V" * 300), ("EraseX", "", b"")]


def u16(text, size):
    return text.encode("utf-16le").ljust(size, b"\0")


def crc16arc(data):
    crc = 0
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if crc & 1 else crc >> 1
    return crc


def build(path, entries=None, far_offset=None, version="BP_R2.0.1", with_crcs=True):
    """Writes a PAC with ENTRIES [(id, file, data)] (data b"" for an entry with no file).

    With FAR_OFFSET, the entry with id "Super" starts there (a sparse file past 4 GiB, say).
    With WITH_CRCS, the header and data CRCs are set as a flash tool expects. Returns
    {id: (offset, size)}."""
    entries = DEFAULT_ENTRIES if entries is None else entries
    hdr = bytearray(HEADER_SIZE)
    hdr[0:48] = u16(version, 48)
    hdr[564:564 + 512] = u16("Old_v1", 512)
    hdr[1104:1104 + 200] = u16("OldAlias", 200)
    struct.pack_into("<II", hdr, 1076, len(entries), HEADER_SIZE)
    off = HEADER_SIZE + ENTRY_SIZE * len(entries)
    table, blobs, layout = b"", [], {}
    for pid, fname, data in entries:
        if far_offset is not None and pid == "Super":
            off = far_offset
        e = bytearray(ENTRY_SIZE)
        e[4:516] = u16(pid, 512)
        e[516:1028] = u16(fname, 512)
        size, o = len(data), (off if data else 0)
        struct.pack_into("<III", e, 1532, size >> 32, o >> 32, size & 0xFFFFFFFF)
        struct.pack_into("<I", e, 1552, o & 0xFFFFFFFF)
        table += bytes(e)
        blobs.append((o, data))
        layout[pid] = (o, size)
        off += size
    with open(path, "w+b") as f:
        f.write(hdr)
        f.write(table)
        for o, data in blobs:
            if data:
                f.seek(o)
                f.write(data)
        if with_crcs:
            f.seek(HEADER_SIZE)
            crc2 = 0
            if far_offset is None:
                crc2 = crc16arc(f.read())
            struct.pack_into("<H", hdr, HEADER_SIZE - 2, crc2)
            struct.pack_into("<H", hdr, HEADER_SIZE - 4, crc16arc(hdr[:HEADER_SIZE - 4]))
            f.seek(0)
            f.write(hdr)
    return layout


def header(path):
    with open(path, "rb") as f:
        return f.read(HEADER_SIZE)


def crcs(path):
    """(header crc, data crc) stored in the PAC."""
    return struct.unpack_from("<HH", header(path), HEADER_SIZE - 4)
