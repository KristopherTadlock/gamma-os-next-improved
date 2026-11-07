#!/usr/bin/env python3
"""Rebuilds the GammaOS ROM pack (roms.tar.xz) from mods/bundled-roms.txt.

  bundled_roms.py LIST IN_TAR OUT_TAR CACHE_DIR LICENSES_DIR

Keeps the pack's folders and systeminfo files, keeps only the "upstream" games
the list names, adds each "add" game after checking its sha256, and creates
each "folder" line's folder. Every game
gets its license file from LICENSES_DIR, either "<path>.license.txt" or a
LICENSE.txt in the game's folder.

Downloaded files are cached in CACHE_DIR by sha256.
"""

import hashlib
import io
import lzma
import os
import re
import subprocess
import sys
import tarfile
import zipfile

ROMS = "sdcard/ROMs/"
PACK_FILES = re.compile(r"(^|/)(systeminfo\.txt|systems\.txt|\.nomedia)$")


def parse(path):
    entries = []
    for number, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.rstrip("\n")
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        kind, target, *pairs = re.split(r"\s{2,}", line.strip())
        fields = dict(pair.split("=", 1) for pair in pairs)
        if kind == "folder":
            entries.append((kind, target, fields))
            continue
        if kind not in ("upstream", "add") or "license" not in fields:
            sys.exit(f"{path}:{number}: can't parse: {line}")
        if kind == "add" and not {"url", "sha256"} <= fields.keys():
            sys.exit(f"{path}:{number}: add needs url= and sha256=")
        entries.append((kind, target, fields))
    return entries


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def fetch(target, fields, cache):
    want = fields["sha256"]
    cached = os.path.join(cache, want)
    if os.path.exists(cached):
        data = open(cached, "rb").read()
        if sha256(data) == want:
            return data
    url = fields["url"]
    download = os.path.join(cache, "download-" + sha256(url.encode()))
    if not os.path.exists(download):
        subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", download + ".partial", url],
                       check=True)
        os.replace(download + ".partial", download)
    raw = open(download, "rb").read()
    member = fields.get("member")
    if member is None:
        data = raw
    elif zipfile.is_zipfile(download):
        data = zipfile.ZipFile(download).read(member)
    else:
        with tarfile.open(download) as archive:
            name = next(n for n in archive.getnames() if n == member or n.endswith("/" + member))
            data = archive.extractfile(name).read()
    if sha256(data) != want:
        sys.exit(f"{target}: sha256 is {sha256(data)}, expected {want}")
    with open(cached, "wb") as f:
        f.write(data)
    return data


def license_for(target, licenses):
    for candidate in (target + ".license.txt", os.path.join(os.path.dirname(target), "LICENSE.txt")):
        path = os.path.join(licenses, candidate)
        if os.path.exists(path):
            return candidate, open(path, "rb").read()
    sys.exit(f"{target}: no license file in {licenses}")


def main():
    list_path, in_tar, out_tar, cache, licenses = sys.argv[1:6]
    os.makedirs(cache, exist_ok=True)
    entries = parse(list_path)
    upstream = {target for kind, target, _ in entries if kind == "upstream"}

    files = {}
    folders = [target for kind, target, _ in entries if kind == "folder"]
    entries = [entry for entry in entries if entry[0] != "folder"]
    for kind, target, fields in entries:
        if kind == "add":
            files[target] = fetch(target, fields, cache)
        name, text = license_for(target, licenses)
        files[name] = text

    kept, removed = set(), []
    with tarfile.open(in_tar) as src, lzma.open(out_tar, "wb", preset=9 | lzma.PRESET_EXTREME) as xz:
        with tarfile.open(fileobj=xz, mode="w", format=tarfile.GNU_FORMAT) as out:
            dirs, file_template = {}, None
            for info in src:
                rel = info.name[len(ROMS):] if info.name.startswith(ROMS) else None
                if info.isdir():
                    out.addfile(info)
                    dirs[info.name.rstrip("/")] = info
                elif rel in upstream or PACK_FILES.search(info.name):
                    out.addfile(info, src.extractfile(info))
                    kept.add(rel)
                    file_template = file_template or info
                else:
                    removed.append(rel or info.name)

            missing = upstream - kept
            if missing:
                sys.exit(f"upstream games not in the pack: {', '.join(sorted(missing))}")

            dir_template = dirs[ROMS.rstrip("/")]
            for rel, data in sorted(files.items()) + [(folder + "/", None) for folder in folders]:
                parts = (ROMS + rel).split("/")
                for depth in range(2, len(parts)):
                    folder = "/".join(parts[:depth])
                    if folder not in dirs:
                        info = tarfile.TarInfo(folder)
                        for attr in ("type", "mode", "uid", "gid", "uname", "gname", "mtime"):
                            setattr(info, attr, getattr(dir_template, attr))
                        out.addfile(info)
                        dirs[folder] = info
                if data is None:
                    continue
                info = tarfile.TarInfo(ROMS + rel)
                for attr in ("mode", "uid", "gid", "uname", "gname", "mtime"):
                    setattr(info, attr, getattr(file_template, attr))
                info.size = len(data)
                out.addfile(info, io.BytesIO(data))

    games = [target for _, target, _ in entries]
    print(f"bundled {len(games)} game files ({len(upstream)} from upstream), "
          f"removed {len(removed)} upstream files")
    for rel in removed:
        print(f"  removed {rel}")


if __name__ == "__main__":
    main()
