#!/usr/bin/env python3
"""Removes Daijisho players that can't work on this build.

  daijisho_prune_players.py DAIJISHOU_DB RETROARCH_CORES_TXT

Drops the "RetroArch 32" and "RetroArch" players everywhere, since only the
64-bit RetroArch (com.retroarch.aarch64) is installed, and "RetroArch 64"
players whose core isn't in RETROARCH_CORES_TXT. Players for standalone apps
stay: users can install those apps. Fails if a platform's default player
would be removed.
"""

import json
import re
import sqlite3
import sys

PACKAGE_RE = re.compile(r"-n\s+([\w.]+)/")
# A path to the core, or (in newer definitions) just its name.
CORE_RE = re.compile(r"LIBRETRO\s+(?:\S*/)?([A-Za-z0-9_\-]+?)(?:_libretro_android\.so)?(?:\s|$)")
OTHER_RETROARCH = {"com.retroarch.ra32", "com.retroarch"}


def shipped_cores(path):
    cores = set()
    for line in open(path):
        fields = line.split("#")[0].split()
        if fields[:2] == ["D", "-"]:
            fields = fields[2:]
        if fields:
            cores.add(fields[0])
    return cores


def usable(args, cores):
    package = PACKAGE_RE.search(args or "")
    package = package.group(1) if package else None
    if package in OTHER_RETROARCH:
        return False
    if package == "com.retroarch.aarch64":
        core = CORE_RE.search(args)
        return bool(core) and core.group(1) in cores
    return True


def main():
    db_path, cores_path = sys.argv[1:3]
    cores = shipped_cores(cores_path)
    con = sqlite3.connect(db_path)
    ok = {pid for pid, args in con.execute("select id, am_start_arguments from PlayerEntity")
          if usable(args, cores)}

    removed = 0
    for platform_id, short, default, id_list in con.execute(
        "select id, shortname, default_player_id, player_id_list from PlatformEntity"
    ).fetchall():
        if default not in ok:
            sys.exit(f"{short}: the default player {default} would be removed")
        players = json.loads(id_list)
        kept = [p for p in players if p in ok]
        removed += len(players) - len(kept)
        con.execute("update PlatformEntity set player_id_list = ? where id = ?",
                    (json.dumps(kept, separators=(",", ":")), platform_id))

    # Delete the players no platform or game refers to any more.
    referenced = {default for default, in con.execute("select default_player_id from PlatformEntity")}
    for id_list, in con.execute("select player_id_list from PlatformEntity"):
        referenced.update(json.loads(id_list))
    referenced.update(p for p, in con.execute(
        "select custom_player_id from PlayableItemEntity where custom_player_id is not null"))
    for pid, in con.execute("select id from PlayerEntity").fetchall():
        if pid not in referenced:
            con.execute("delete from PlayerEntity where id = ?", (pid,))

    con.commit()
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()
    print(f"players: removed {removed} that need a missing core or another RetroArch")


if __name__ == "__main__":
    main()
