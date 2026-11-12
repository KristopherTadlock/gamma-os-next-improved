"""Builds newc ("070701") cpio archives, as in a vendor ramdisk."""


def entry(name, data, magic=b"070701", mode=0o100644, ino=1):
    namez = name.encode() + b"\0"
    fields = [ino, mode, 0, 0, 1, 0, len(data), 0, 0, 0, 0, len(namez), 0]
    hdr = magic + b"".join(b"%08X" % v for v in fields)
    out = hdr + namez
    out += b"\0" * (-len(out) % 4)
    out += data + b"\0" * (-len(data) % 4)
    return out


def build(files, magic=b"070701"):
    """A cpio archive of FILES [(name, data)], with its trailer."""
    out = b"".join(entry(name, data, magic, ino=i + 1) for i, (name, data) in enumerate(files))
    return out + entry("TRAILER!!!", b"", magic, mode=0, ino=0)


def members(cpio):
    """[(name, data)] of a newc archive, up to its trailer."""
    pos, out = 0, []
    while True:
        size = int(cpio[pos + 54:pos + 62], 16)
        namesize = int(cpio[pos + 94:pos + 102], 16)
        name = cpio[pos + 110:pos + 110 + namesize - 1].decode()
        data = (pos + 110 + namesize + 3) & ~3
        if name == "TRAILER!!!":
            return out
        out.append((name, cpio[data:data + size]))
        pos = (data + size + 3) & ~3
