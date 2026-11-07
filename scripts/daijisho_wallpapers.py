#!/usr/bin/env python3
"""Gives each Daijisho platform without a wallpaper the one WALLPAPERS_TXT lists.

  daijisho_wallpapers.py DAIJISHOU_DB APP_FILES_DIR WALLPAPERS_TXT CACHE_DIR SPARE_DIR

Downloaded wallpapers are checked against their sha256 and cached in
CACHE_DIR; local ones are relative to WALLPAPERS_TXT's folder. The pack's
license is saved next to the wallpapers. "spare" wallpapers, for platforms
not in the database, go to SPARE_DIR with another copy of the license. Fails
if a platform is left without a wallpaper.
"""

import hashlib
import os
import sqlite3
import subprocess
import sys

DEVICE_DIR = "/data/user/0/com.magneticchen.daijishou/files/platform_wallpapers"


def read_list(path):
    base, license, wallpapers, spares = None, None, {}, {}
    for line in open(path):
        fields = line.split("#")[0].split()
        if not fields:
            continue
        if fields[0] == "base":
            base = fields[1]
        elif fields[0] == "license":
            license = (fields[1], fields[2])
        elif fields[0] == "spare":
            spares[fields[1]] = (base + fields[2], fields[3])
        elif len(fields) == 3:
            wallpapers[fields[0]] = (base + fields[1], fields[2])
        else:
            wallpapers[fields[0]] = (os.path.join(os.path.dirname(path), fields[1]), None)
    return license, wallpapers, spares


def load(source, sha256, cache):
    if sha256 is None:
        return open(source, "rb").read()
    cached = os.path.join(cache, sha256)
    if not os.path.exists(cached):
        subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", cached + ".partial", source], check=True)
        os.replace(cached + ".partial", cached)
    data = open(cached, "rb").read()
    if hashlib.sha256(data).hexdigest() != sha256:
        sys.exit(f"{source}: sha256 mismatch")
    return data


def main():
    db_path, files_dir, list_path, cache, spare_dir = sys.argv[1:6]
    os.makedirs(cache, exist_ok=True)
    license, wallpapers, spares = read_list(list_path)
    con = sqlite3.connect(db_path)
    missing = []
    for platform_id, shortname, path in con.execute(
        "select id, shortname, wallpaper_path from PlatformEntity order by id"
    ).fetchall():
        if path and os.path.exists(os.path.join(files_dir, "platform_wallpapers", os.path.basename(path))):
            continue
        if shortname not in wallpapers:
            missing.append(shortname)
            continue
        os.makedirs(os.path.join(files_dir, "platform_wallpapers"), exist_ok=True)
        dest = os.path.join(files_dir, "platform_wallpapers", f"{platform_id}.png")
        with open(dest, "wb") as f:
            f.write(load(*wallpapers[shortname], cache))
        os.chmod(dest, 0o600)
        con.execute("update PlatformEntity set wallpaper_path = ? where id = ?",
                    (f"{DEVICE_DIR}/{platform_id}.png", platform_id))
        print(f"{shortname}: wallpaper added")
    if missing:
        sys.exit(f"no wallpaper for: {', '.join(missing)}")
    if license:
        dest = os.path.join(files_dir, "platform_wallpapers", "LICENSE.txt")
        with open(dest, "wb") as f:
            f.write(load(*license, cache))
        os.chmod(dest, 0o600)

    os.makedirs(spare_dir, exist_ok=True)
    for name, source in spares.items():
        with open(os.path.join(spare_dir, name), "wb") as f:
            f.write(load(*source, cache))
    if spares and license:
        with open(os.path.join(spare_dir, "LICENSE.txt"), "wb") as f:
            f.write(load(*license, cache))
    if spares:
        print(f"spare wallpapers: {', '.join(spares)}")
    con.commit()
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()


if __name__ == "__main__":
    main()
