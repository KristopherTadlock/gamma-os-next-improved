"""Rebuilds a compressed EROFS partition image with some files' contents replaced.

rebuild(IN, LABEL, CHANGES) carries every file over with its owner, mode and SELinux label (read
from IN), through a PAX tarball that mkfs.erofs builds the image from, with IN's block size,
build time and UUID. CHANGES maps a path in the image (no leading slash) to a function that
takes the file's contents and returns the new contents; the paths must exist. The result must
use no filesystem feature IN does not, must hold the same files with the same metadata, and is
padded to IN's size. Directories, regular files and symlinks only.

Needs on PATH: fsck.erofs and mkfs.erofs (erofs-utils 1.7 or later).
"""
import io
import os
import struct
import subprocess
import sys
import tarfile
import tempfile

EROFS_MAGIC = 0xE0F5E1E2
XATTR_PREFIXES = {1: "user.", 4: "trusted.", 6: "security."}


class Erofs:
    """Just enough of an EROFS reader for inode metadata, xattrs and directories."""

    def __init__(self, data):
        self.d = data
        (magic,) = struct.unpack_from("<I", data, 1024)
        if magic != EROFS_MAGIC:
            sys.exit("not an EROFS image")
        self.compat, self.blkbits = struct.unpack_from("<IB", data, 1024 + 8)
        (self.root_nid,) = struct.unpack_from("<H", data, 1024 + 14)
        self.build_time, = struct.unpack_from("<Q", data, 1024 + 24)
        self.meta_blkaddr, self.xattr_blkaddr = struct.unpack_from("<II", data, 1024 + 40)
        self.uuid = data[1024 + 48:1024 + 64]
        (self.incompat,) = struct.unpack_from("<I", data, 1024 + 80)
        self.bs = 1 << self.blkbits

    def inode(self, nid):
        off = self.meta_blkaddr * self.bs + nid * 32
        fmt, xicount, mode = struct.unpack_from("<HHH", self.d, off)
        extended = fmt & 1
        layout = (fmt >> 1) & 7
        if extended:
            size, = struct.unpack_from("<Q", self.d, off + 8)
            raw, = struct.unpack_from("<I", self.d, off + 16)
            uid, gid = struct.unpack_from("<II", self.d, off + 24)
            isize = 64
        else:
            size, = struct.unpack_from("<I", self.d, off + 8)
            raw, = struct.unpack_from("<I", self.d, off + 16)
            uid, gid = struct.unpack_from("<HH", self.d, off + 24)
            isize = 32
        xsize = 0 if xicount == 0 else 12 + (xicount - 1) * 4
        return dict(off=off, isize=isize, xsize=xsize, mode=mode, uid=uid, gid=gid,
                    size=size, layout=layout, raw=raw)

    def _entry(self, pos):
        name_len, index, value_size = struct.unpack_from("<BBH", self.d, pos)
        name = self.d[pos + 4:pos + 4 + name_len].decode()
        value = self.d[pos + 4 + name_len:pos + 4 + name_len + value_size]
        if index not in XATTR_PREFIXES:
            sys.exit("unsupported xattr namespace %d" % index)
        return XATTR_PREFIXES[index] + name, value, 4 + name_len + value_size

    def xattrs(self, ino):
        out = {}
        if not ino["xsize"]:
            return out
        base = ino["off"] + ino["isize"]
        shared = self.d[base + 4]
        for i in range(shared):
            sid, = struct.unpack_from("<I", self.d, base + 12 + 4 * i)
            name, value, _ = self._entry(self.xattr_blkaddr * self.bs + sid * 4)
            out[name] = value
        pos, end = base + 12 + 4 * shared, base + ino["xsize"]
        while pos < end:
            name, value, n = self._entry(pos)
            out[name] = value
            pos += (n + 3) & ~3
        return out

    def data(self, ino):
        """A directory's entries or a symlink's target (flat layouts only)."""
        if ino["layout"] == 0:     # flat plain
            start = ino["raw"] * self.bs
            return self.d[start:start + ino["size"]]
        if ino["layout"] == 2:     # flat inline: full blocks, then the tail after the inode
            full = ino["size"] // self.bs * self.bs
            head = self.d[ino["raw"] * self.bs:ino["raw"] * self.bs + full] if full else b""
            tail_start = ino["off"] + ino["isize"] + ino["xsize"]
            return head + self.d[tail_start:tail_start + ino["size"] - full]
        sys.exit("unsupported inode layout %d" % ino["layout"])

    def walk(self, nid=None, path=""):
        """(path, nid) for every inode, parents first; the root is ""."""
        nid = self.root_nid if nid is None else nid
        yield path, nid
        ino = self.inode(nid)
        if (ino["mode"] & 0o170000) != 0o040000:
            return
        data = self.data(ino)
        for blk in range(0, len(data), self.bs):
            block = data[blk:blk + self.bs]
            first_nameoff, = struct.unpack_from("<H", block, 8)
            count = first_nameoff // 12
            for i in range(count):
                child, nameoff, _ = struct.unpack_from("<QHB", block, i * 12)
                end = struct.unpack_from("<H", block, (i + 1) * 12 + 8)[0] if i + 1 < count else len(block)
                name = block[nameoff:end].split(b"\0")[0].decode()
                if name in (".", ".."):
                    continue
                yield from self.walk(child, path + "/" + name if path else name)



S_IFMT, S_IFDIR, S_IFREG, S_IFLNK = 0o170000, 0o040000, 0o100000, 0o120000


def rebuild(src, label, changes):
    """IN's image with CHANGES applied, padded to IN's size, as bytes."""
    with open(src, "rb") as f:
        image = f.read()
    fs = Erofs(image)
    entries = list(fs.walk())
    paths = {p for p, _ in entries}
    missing = sorted(set(changes) - paths)
    if missing:
        sys.exit("%s: no /%s to change" % (label, missing[0]))

    with tempfile.TemporaryDirectory() as tmp:
        tree = os.path.join(tmp, "tree")
        subprocess.run(["fsck.erofs", "--extract=" + tree, "--no-preserve", src], check=True,
                       stdout=subprocess.DEVNULL)
        extracted = {""}
        for root, dirs, files in os.walk(tree):
            rel = os.path.relpath(root, tree)
            for name in dirs + files:
                extracted.add(name if rel == "." else os.path.join(rel, name))
        if extracted != paths:
            sys.exit("%s: could not read its directory tree" % label)

        def contents(path):
            with open(os.path.join(tree, path), "rb") as f:
                data = f.read()
            return changes[path](data) if path in changes else data

        tar_path = os.path.join(tmp, label + ".tar")
        with tarfile.open(tar_path, "w", format=tarfile.PAX_FORMAT) as tar:
            for path, nid in entries:
                ino = fs.inode(nid)
                kind = ino["mode"] & S_IFMT
                info = tarfile.TarInfo(path or ".")
                info.uid, info.gid = ino["uid"], ino["gid"]
                info.mode = ino["mode"] & 0o7777
                info.mtime = fs.build_time
                info.pax_headers = {"SCHILY.xattr." + k: v.decode("utf-8", "surrogateescape")
                                    for k, v in fs.xattrs(ino).items()}
                if kind == S_IFDIR:
                    info.type = tarfile.DIRTYPE
                    tar.addfile(info)
                elif kind == S_IFREG:
                    data = contents(path)
                    info.size = len(data)
                    tar.addfile(info, io.BytesIO(data))
                elif kind == S_IFLNK and path not in changes:
                    info.type = tarfile.SYMTYPE
                    info.linkname = fs.data(ino).decode()
                    tar.addfile(info)
                else:
                    sys.exit("%s: unsupported file type: /%s" % (label, path))

        uuid = fs.uuid.hex()
        uuid = "-".join((uuid[:8], uuid[8:12], uuid[12:16], uuid[16:20], uuid[20:]))
        built = os.path.join(tmp, "built.img")
        # The block size would otherwise follow the build host's page size; the kernel needs IN's.
        subprocess.run(["mkfs.erofs", "-b%d" % fs.bs, "-zlz4hc", "-E^xattr-name-filter",
                        "-T%d" % fs.build_time, "-U" + uuid,
                        "--tar=f", built, tar_path], check=True, stdout=subprocess.DEVNULL)
        with open(built, "rb") as f:
            new = f.read()

        # Same features, same files and metadata, and only CHANGES changed.
        nfs = Erofs(new)
        if nfs.bs != fs.bs:
            sys.exit("rebuilt %s has a different block size" % label)
        if (nfs.incompat & ~fs.incompat) or (nfs.compat & ~fs.compat):
            sys.exit("rebuilt %s uses EROFS features the original does not" % label)
        if len(new) > len(image):
            sys.exit("rebuilt %s is larger than the partition" % label)
        nentries = list(nfs.walk())
        if sorted(p for p, _ in nentries) != sorted(paths):
            sys.exit("rebuilt %s has different files" % label)
        old_meta = {p: fs.inode(n) for p, n in entries}
        for path, nid in nentries:
            a, b = old_meta[path], nfs.inode(nid)
            if (a["mode"], a["uid"], a["gid"]) != (b["mode"], b["uid"], b["gid"]) or \
                    fs.xattrs(a) != nfs.xattrs(b):
                sys.exit("rebuilt %s: metadata differs for /%s" % (label, path))
            if (a["mode"] & S_IFMT) == S_IFLNK and fs.data(a) != nfs.data(b):
                sys.exit("rebuilt %s: /%s points elsewhere" % (label, path))
        check = os.path.join(tmp, "check")
        subprocess.run(["fsck.erofs", "--extract=" + check, "--no-preserve", built], check=True,
                       stdout=subprocess.DEVNULL)
        for path, nid in entries:
            if (fs.inode(nid)["mode"] & S_IFMT) != S_IFREG:
                continue
            with open(os.path.join(check, path), "rb") as f:
                if f.read() != contents(path):
                    sys.exit("rebuilt %s: /%s does not match" % (label, path))

    return new + b"\0" * (len(image) - len(new))
