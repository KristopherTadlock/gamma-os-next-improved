#!/usr/bin/env python3
"""Points each Daijisho platform's default player at the cores marked "D - ".

  daijisho_defaults.py DAIJISHOU_DB RETROARCH_CORES_TXT

A platform keeps its default when that default is a standalone app, or a
RetroArch core that is marked as a default, or a shipped core with no marked
alternative. Otherwise it switches to its one "RetroArch 64" player whose core
is marked as a default.
"""

import re
import sqlite3
import sys

CORE_RE = re.compile(r"([A-Za-z0-9_\-]+)_libretro_android\.so")


def read_cores(path):
    shipped, defaults = set(), set()
    for line in open(path):
        fields = line.split("#")[0].split()
        if not fields:
            continue
        if fields[:2] == ["D", "-"]:
            defaults.add(fields[2])
            fields = fields[2:]
        shipped.add(fields[0])
    return shipped, defaults


def main():
    db_path, cores_path = sys.argv[1:3]
    shipped, defaults = read_cores(cores_path)
    con = sqlite3.connect(db_path)
    players = {}
    for pid, name, args in con.execute(
        "select id, name, am_start_arguments from PlayerEntity"
    ):
        core = CORE_RE.findall(args or "")
        is_ra64 = " - RetroArch 64 - " in name
        players[pid] = (name, core[0] if core else None, is_ra64)

    failed = False
    platforms = con.execute(
        "select id, shortname, default_player_id, player_id_list from PlatformEntity"
    ).fetchall()
    for platform_id, short, current, id_list in platforms:
        name, core, _ = players[current]
        options = [int(i) for i in re.findall(r"\d+", id_list)]
        candidates = [
            i for i in options if players[i][2] and players[i][1] in defaults
        ]
        if core is None or core in defaults or (core in shipped and not candidates):
            print(f"{short}: keeps {name}")
        elif len(candidates) == 1:
            new = candidates[0]
            con.execute(
                "update PlatformEntity set default_player_id = ? where id = ?",
                (new, platform_id),
            )
            print(f"{short}: {name} -> {players[new][0]}")
        else:
            failed = True
            found = ", ".join(players[i][1] for i in candidates) or "none"
            print(f"{short}: can't pick a default (marked cores: {found})", file=sys.stderr)

    if failed:
        sys.exit(1)
    con.commit()
    # Fold the write-ahead log into the database so only Daijishou.db is shipped.
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()


if __name__ == "__main__":
    main()
