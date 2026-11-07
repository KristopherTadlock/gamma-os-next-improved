#!/usr/bin/env python3
"""Gives each Daijisho platform without a wallpaper the one in WALLPAPERS_DIR.

  daijisho_wallpapers.py DAIJISHOU_DB APP_FILES_DIR WALLPAPERS_DIR

WALLPAPERS_DIR holds <platform shortname>.png. Fails if a platform is left
without a wallpaper.
"""

import os
import shutil
import sqlite3
import sys

DEVICE_DIR = "/data/user/0/com.magneticchen.daijishou/files/platform_wallpapers"


def main():
    db_path, files_dir, wallpapers = sys.argv[1:4]
    con = sqlite3.connect(db_path)
    missing = []
    for platform_id, shortname, path in con.execute(
        "select id, shortname, wallpaper_path from PlatformEntity order by id"
    ).fetchall():
        if path and os.path.exists(os.path.join(files_dir, "platform_wallpapers", os.path.basename(path))):
            continue
        png = os.path.join(wallpapers, shortname + ".png")
        if not os.path.exists(png):
            missing.append(shortname)
            continue
        os.makedirs(os.path.join(files_dir, "platform_wallpapers"), exist_ok=True)
        dest = os.path.join(files_dir, "platform_wallpapers", f"{platform_id}.png")
        shutil.copyfile(png, dest)
        os.chmod(dest, 0o600)
        con.execute("update PlatformEntity set wallpaper_path = ? where id = ?",
                    (f"{DEVICE_DIR}/{platform_id}.png", platform_id))
        print(f"{shortname}: wallpaper added")
    if missing:
        sys.exit(f"no wallpaper for: {', '.join(missing)}")
    con.commit()
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()


if __name__ == "__main__":
    main()
