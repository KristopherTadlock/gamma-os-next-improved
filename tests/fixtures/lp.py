"""Builds small raw (non-sparse) Android super images (see scripts/lptool.py)."""
import hashlib
import struct

SECTOR, ALIGN = 512, 1 << 20
GEOMETRY_OFFSET, METADATA_OFFSET = 4096, 4096 * 3
PARTITION = struct.Struct("<36sIIII")
EXTENT = struct.Struct("<QIQI")
GROUP = struct.Struct("<36sIQ")
DEVICE = struct.Struct("<QIIQ36sI")


def build(path, partitions=None, group_max=8 << 20, device_size=16 << 20, slots=2, max_size=4096):
    """Writes a super image. PARTITIONS is [(name, group index, size in MiB, fill byte)], laid out
    one after another from the first aligned sector, each one extent; group 0 is "default"
    (no limit), group 1 "main_a" (GROUP_MAX). Returns {name: (offset, size)}."""
    if partitions is None:
        partitions = [("system_a", 1, 1, b"S"), ("vendor_a", 1, 1, b"V"), ("system_b", 0, 0, b"")]
    first_lba = (METADATA_OFFSET + max_size * slots * 2) // SECTOR
    start = -(-first_lba * SECTOR // ALIGN) * ALIGN
    parts, exts, layout, fills = [], [], {}, []
    for name, group, mib, fill in partitions:
        size = mib * ALIGN
        if size:
            parts.append((name.encode(), 0, len(exts), 1, group))
            exts.append((size // SECTOR, 0, start // SECTOR, 0))
            layout[name] = (start, size)
            fills.append((start, fill * size))
            start += size
        else:
            parts.append((name.encode(), 0, len(exts), 0, group))
            layout[name] = (0, 0)
    groups = [(b"default", 0, 0), (b"main_a", 0, group_max)]
    devices = [(first_lba, ALIGN, 0, device_size, b"super", 0)]
    tabs = [b"".join(layout_.pack(*row) for row in rows)
            for layout_, rows in ((PARTITION, parts), (EXTENT, exts), (GROUP, groups), (DEVICE, devices))]
    tables = b"".join(tabs)
    hdr = bytearray(128)
    struct.pack_into("<IHHI", hdr, 0, 0x414C5030, 10, 0, 128)
    struct.pack_into("<I", hdr, 44, len(tables))
    hdr[48:80] = hashlib.sha256(tables).digest()
    off = 0
    for i, (tab, layout_) in enumerate(zip(tabs, (PARTITION, EXTENT, GROUP, DEVICE))):
        struct.pack_into("<III", hdr, 80 + 12 * i, off, len(tab) // layout_.size, layout_.size)
        off += len(tab)
    hdr[12:44] = hashlib.sha256(bytes(hdr)).digest()
    geo = bytearray(4096)
    struct.pack_into("<II", geo, 0, 0x616C4467, 52)
    struct.pack_into("<III", geo, 40, max_size, slots, 4096)
    img = bytearray(device_size)
    img[GEOMETRY_OFFSET:GEOMETRY_OFFSET + 4096] = geo
    img[GEOMETRY_OFFSET + 4096:GEOMETRY_OFFSET + 8192] = geo
    blob = (bytes(hdr) + tables).ljust(max_size, b"\0")
    for s in range(slots * 2):
        img[METADATA_OFFSET + s * max_size:METADATA_OFFSET + (s + 1) * max_size] = blob
    for offset, data in fills:
        img[offset:offset + len(data)] = data
    with open(path, "wb") as f:
        f.write(img)
    return layout


def metadata_slots(path, slots=2, max_size=4096):
    """The raw bytes of every metadata slot (primary then backup)."""
    with open(path, "rb") as f:
        f.seek(METADATA_OFFSET)
        return [f.read(max_size) for _ in range(slots * 2)]
