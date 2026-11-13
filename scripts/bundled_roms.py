#!/usr/bin/env python3
"""Rebuilds the GammaOS ROM pack (roms.tar.xz) from mods/bundled-roms.txt.

  bundled_roms.py LIST IN_TAR OUT_TAR CACHE_DIR LICENSES_DIR LOCAL_DIR [MEDIA_LIST [nano]]

OUT_TAR is xz-compressed when its name ends in .xz, and a plain tar otherwise.

Keeps the pack's folders and systeminfo files, keeps only the "upstream" games
the list names, adds each "add" game after checking its sha256, copies each
"tree" folder out of an archive after checking the archive's sha256, adds each
"local" file from LOCAL_DIR, and creates each "folder" line's folder. A line with
base=nano is only for the 1.4.4 (Nano) base, base=1.2 only for the 1.2 base.
Every game gets its license file from LICENSES_DIR: "<path>.license.txt", a
"tree" folder's own LICENSE.txt, or a LICENSE.txt in the game's folder.

With MEDIA_LIST (pack-in-media.txt), each game's box art, screenshot, title
screen and marquee (downloaded and checked the same way, or from pack-in-art/
next to the list) are added with ES-DE game lists holding its details, in /sdcard/ES-DE where the ES-DE theme and ES-DE read them; with
"nano" as well, Nano's own box art index (/data/system/nano_scrape/index.json)
lists them too, for its other themes.

Downloaded files are cached in CACHE_DIR by sha256.
"""

import hashlib
import io
import json
import lzma
import os
import re
import subprocess
import sys
import tarfile
import zipfile
from xml.sax.saxutils import escape

ROMS = "sdcard/ROMs/"
ESDE = "sdcard/ES-DE/"
NANO_SCRAPE = "data/system/nano_scrape"
# Where Nano finds the files at run time (it scans /data/media/0 first).
DEVICE_ROMS = "/data/media/0/ROMs/"
DEVICE_ESDE = "/data/media/0/ES-DE/"
MEDIA_DIRS = {"cover": "covers", "screenshot": "screenshots", "titlescreen": "titlescreens", "marquee": "marquees"}
MEDIA_FIELDS = ("title", "developer", "publisher", "released", "genre", "players", "desc")
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
        if kind not in ("upstream", "add", "tree", "local") or "license" not in fields:
            sys.exit(f"{path}:{number}: can't parse: {line}")
        if kind in ("add", "tree") and not {"url", "sha256"} <= fields.keys():
            sys.exit(f"{path}:{number}: {kind} needs url= and sha256=")
        if kind == "tree" and "member" not in fields:
            sys.exit(f"{path}:{number}: tree needs member=")
        entries.append((kind, target, fields))
    return entries


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def download(url, cache):
    path = os.path.join(cache, "download-" + sha256(url.encode()))
    if not os.path.exists(path):
        subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", path + ".partial", url],
                       check=True)
        os.replace(path + ".partial", path)
    return path


def fetch(target, fields, cache):
    want = fields["sha256"]
    cached = os.path.join(cache, want)
    if os.path.exists(cached):
        data = open(cached, "rb").read()
        if sha256(data) == want:
            return data
    download_path = download(fields["url"], cache)
    raw = open(download_path, "rb").read()
    member = fields.get("member")
    if member is None:
        data = raw
    elif zipfile.is_zipfile(download_path):
        data = zipfile.ZipFile(download_path).read(member)
    else:
        with tarfile.open(download_path) as archive:
            name = next(n for n in archive.getnames() if n == member or n.endswith("/" + member))
            data = archive.extractfile(name).read()
    if sha256(data) != want:
        # Otherwise the bad download would be reused, and fail, on every later build.
        os.remove(download_path)
        sys.exit(f"{target}: sha256 is {sha256(data)}, expected {want} (download deleted from the cache)")
    with open(cached, "wb") as f:
        f.write(data)
    return data


def fetch_tree(target, fields, cache):
    """Returns {path: data} for the files under member= in the archive."""
    path = download(fields["url"], cache)
    if sha256(open(path, "rb").read()) != fields["sha256"]:
        os.remove(path)
        sys.exit(f"{target}: archive sha256 doesn't match (download deleted from the cache)")
    prefix = fields["member"].rstrip("/") + "/"
    archive = zipfile.ZipFile(path)
    files = {
        target.rstrip("/") + "/" + name[len(prefix):]: archive.read(name)
        for name in archive.namelist()
        if name.startswith(prefix) and not name.endswith("/")
    }
    if not files:
        sys.exit(f"{target}: nothing under {prefix} in the archive")
    return files


def parse_media(path, games):
    """Returns {game: {field: value}} from MEDIA_LIST, each image as {url, sha256, license}."""
    media, game = {}, None
    for number, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            game = line[1:-1]
            if game not in games:
                sys.exit(f"{path}:{number}: {game} isn't a game in the bundled list")
            media[game] = {}
            continue
        key, sep, value = (part.strip() for part in line.partition("="))
        if game is None or not sep or key not in MEDIA_FIELDS + tuple(MEDIA_DIRS):
            sys.exit(f"{path}:{number}: can't parse: {line}")
        if key in MEDIA_DIRS:
            fields = dict(pair.split("=", 1) for pair in re.split(r"\s{2,}", value))
            if "license" not in fields or not ({"url", "sha256"} <= fields.keys() or "file" in fields):
                sys.exit(f"{path}:{number}: {key} needs license= and url= with sha256=, or file=")
            value = fields
        media[game][key] = value
    return media


def image_ext(target, data):
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return ".png"
    if data.startswith(b"\xff\xd8\xff"):
        return ".jpg"
    sys.exit(f"{target}: not a PNG or JPEG image")


def media_files(media, cache, art_dir, nano):
    """Returns {tar path: data} for the ES-DE game lists and media, and Nano's index."""
    files, lists, index = {}, {}, []
    for game, info in sorted(media.items()):
        system, rel = game.split("/", 1)
        # ES-DE keeps a game's media under the same subfolders as the game (covers/Game/EBOOT.png
        # for psp/Game/EBOOT.PBP); a game in its own folder is also named after the folder.
        names = [os.path.splitext(rel)[0]] + ([rel.split("/")[0]] if "/" in rel else [])
        paths = {}
        for kind, folder in MEDIA_DIRS.items():
            if kind not in info:
                continue
            if "file" in info[kind]:
                data = open(os.path.join(art_dir, info[kind]["file"]), "rb").read()
            else:
                data = fetch(f"{game} {kind}", info[kind], cache)
            ext = image_ext(f"{game} {kind}", data)
            for name in names:
                files[f"{ESDE}downloaded_media/{system}/{folder}/{name}{ext}"] = data
            paths[kind] = f"{DEVICE_ESDE}downloaded_media/{system}/{folder}/{names[0]}{ext}"
        released = info.get("released", "")
        if released and not re.fullmatch(r"\d{4}(-\d{2}-\d{2})?", released):
            sys.exit(f"{game}: released must be YYYY or YYYY-MM-DD")
        lists.setdefault(system, []).append((rel, info))
        entry = {"rom": DEVICE_ROMS + game, "scraper": "pack-in", "when": 0}
        for key, field in (("cover", "box"), ("screenshot", "fan")):
            if key in paths:
                entry[field] = paths[key]
        for key, field in (("title", "title"), ("desc", "desc"), ("genre", "genre"), ("players", "players"),
                           ("released", "date"), ("developer", "dev"), ("publisher", "pub")):
            if info.get(key):
                entry[field] = info[key]
        index.append(entry)
    for system, games in lists.items():
        lines = ['<?xml version="1.0"?>', "<gameList>"]
        for rel, info in sorted(games):
            lines += ["\t<game>", f"\t\t<path>./{escape(rel)}</path>"]
            released = info.get("released", "")
            if released:
                released = (released.replace("-", "") + "0101")[:8] + "T000000"
            for key, tag, value in (("title", "name", info.get("title")), ("desc", "desc", info.get("desc")),
                                    ("released", "releasedate", released),
                                    ("developer", "developer", info.get("developer")),
                                    ("publisher", "publisher", info.get("publisher")),
                                    ("genre", "genre", info.get("genre")), ("players", "players", info.get("players"))):
                if value:
                    lines.append(f"\t\t<{tag}>{escape(value)}</{tag}>")
            lines.append("\t</game>")
        lines.append("</gameList>")
        files[f"{ESDE}gamelists/{system}/gamelist.xml"] = ("\n".join(lines) + "\n").encode()
    if nano:
        text = json.dumps({"version": 2, "items": index}, indent=2, ensure_ascii=False) + "\n"
        files[f"{NANO_SCRAPE}/index.json"] = text.encode()
    return files


def license_for(target, licenses, folder=False):
    candidates = [target + ".license.txt"] + [os.path.join(target, "LICENSE.txt")] * folder
    for candidate in candidates + [os.path.join(os.path.dirname(target), "LICENSE.txt")]:
        path = os.path.join(licenses, candidate)
        if os.path.exists(path):
            return candidate, open(path, "rb").read()
    sys.exit(f"{target}: no license file in {licenses}")


def main():
    list_path, in_tar, out_tar, cache, licenses, local = sys.argv[1:7]
    media_path = sys.argv[7] if len(sys.argv) > 7 else None
    nano = sys.argv[8:9] == ["nano"]
    os.makedirs(cache, exist_ok=True)
    base = "nano" if nano else "1.2"
    entries = [e for e in parse(list_path) if e[2].get("base", base) == base]
    upstream = {target for kind, target, _ in entries if kind == "upstream"}

    files = {}
    folders = [target for kind, target, _ in entries if kind == "folder"]
    entries = [entry for entry in entries if entry[0] != "folder"]
    for kind, target, fields in entries:
        if kind == "add":
            files[target] = fetch(target, fields, cache)
        elif kind == "tree":
            files.update(fetch_tree(target, fields, cache))
            target = target.rstrip("/")
        elif kind == "local":
            files[target] = open(os.path.join(local, target), "rb").read()
        name, text = license_for(target, licenses, kind == "tree")
        files[name] = text
    extra = {}
    if media_path:
        games = {target for kind, target, _ in entries} | upstream
        art_dir = os.path.join(os.path.dirname(media_path), "pack-in-art")
        extra = media_files(parse_media(media_path, games), cache, art_dir, nano)

    kept, removed = set(), []
    if out_tar.endswith(".xz"):
        sink = lzma.open(out_tar, "wb", preset=9 | lzma.PRESET_EXTREME)
    else:
        sink = open(out_tar, "wb")
    with tarfile.open(in_tar) as src, sink:
        with tarfile.open(fileobj=sink, mode="w", format=tarfile.GNU_FORMAT) as out:
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

            # ES-DE's folders are made like the ROMs folders. Nano's index is root's alone; only its
            # own folder is in the archive, so /data and /data/system keep their owners and modes.
            for path, data in sorted(extra.items()):
                nano_file = path.startswith(NANO_SCRAPE + "/")
                parts = path.split("/")
                for depth in range(2, len(parts)):
                    folder = "/".join(parts[:depth])
                    if folder in dirs or (nano_file and folder != NANO_SCRAPE):
                        continue
                    info = tarfile.TarInfo(folder)
                    for attr in ("type", "mode", "uid", "gid", "uname", "gname", "mtime"):
                        setattr(info, attr, getattr(dir_template, attr))
                    if nano_file:
                        info.mode, info.uid, info.gid, info.uname, info.gname = 0o700, 0, 0, "root", "root"
                    out.addfile(info)
                    dirs[folder] = info
                info = tarfile.TarInfo(path)
                for attr in ("mode", "uid", "gid", "uname", "gname", "mtime"):
                    setattr(info, attr, getattr(file_template, attr))
                if nano_file:
                    info.mode, info.uid, info.gid, info.uname, info.gname = 0o600, 0, 0, "root", "root"
                info.size = len(data)
                out.addfile(info, io.BytesIO(data))

    games = [target for _, target, _ in entries]
    print(f"bundled {len(games)} game files ({len(upstream)} from upstream), "
          f"removed {len(removed)} upstream files"
          + (f", added art and details for {len(parse_media(media_path, set(games) | upstream))} games"
             if media_path else ""))
    for rel in removed:
        print(f"  removed {rel}")


if __name__ == "__main__":
    main()
