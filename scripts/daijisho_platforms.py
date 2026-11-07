#!/usr/bin/env python3
"""Adds Daijisho platforms from Daijisho's official platform definitions.

  daijisho_platforms.py DAIJISHOU_DB PLATFORMS_TXT CACHE_DIR

Each platform gets only the players PLATFORMS_TXT names, the first being its
default, and scans /sdcard/ROMs/<folder>. For a platform already in the
database, the named players are added to its list and the first becomes its
default.
"""

import json
import os
import sqlite3
import subprocess
import sys

BASE = "https://raw.githubusercontent.com/TapiocaFox/Daijishou/{commit}/platforms/{name}"
SYNC_URI = "content://com.android.externalstorage.documents/tree/primary%3AROMs%2F{folder}"


def read_config(path):
    commit, platforms = None, []
    for line in open(path):
        fields = line.split("#")[0].split()
        if not fields:
            continue
        if fields[0] == "commit":
            commit = fields[1]
        else:
            platforms.append((fields[0], fields[1], fields[2:]))
    if commit is None:
        sys.exit(f"{path}: no commit line")
    return commit, platforms


def definition(commit, name, cache):
    path = os.path.join(cache, f"{commit}-{name}")
    if not os.path.exists(path):
        url = BASE.format(commit=commit, name=name)
        subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", path, url], check=True)
    return json.load(open(path))


def main():
    db_path, config_path, cache = sys.argv[1:4]
    os.makedirs(cache, exist_ok=True)
    commit, platforms = read_config(config_path)
    con = sqlite3.connect(db_path)

    for name, folder, wanted in platforms:
        data = definition(commit, name, cache)
        p = data["platform"]
        existing = con.execute(
            "select id, player_id_list from PlatformEntity where unique_id = ?", (p["uniqueId"],)
        ).fetchone()

        players = {pl["uniqueId"]: pl for pl in data["playerList"]}
        player_ids = []
        for unique_id in wanted:
            if unique_id not in players:
                sys.exit(f"{name}: no player {unique_id}")
            pl = players[unique_id]
            if existing:
                found = con.execute(
                    "select pl.id from PlayerEntity pl, json_each(?) j"
                    " where pl.id = j.value and pl.unique_id = ?",
                    (existing[1], unique_id),
                ).fetchone()
                if found:
                    player_ids.append(found[0])
                    continue
            cur = con.execute(
                "insert into PlayerEntity (unique_id, player, name, description, am_start_arguments,"
                " kill_package_processes, kill_package_processes_warning, accepted_filename_regex, extra)"
                " values (?, 1, ?, ?, ?, ?, ?, ?, ?)",
                (pl["uniqueId"], pl["name"], pl.get("description"), pl["amStartArguments"],
                 int(pl.get("killPackageProcesses", False)),
                 int(pl.get("killPackageProcessesWarning", True)),
                 pl["acceptedFilenameRegex"], pl.get("extra", "")),
            )
            player_ids.append(cur.lastrowid)

        if existing:
            old = json.loads(existing[1])
            new_list = player_ids + [i for i in old if i not in player_ids]
            con.execute(
                "update PlatformEntity set default_player_id = ?, player_id_list = ? where id = ?",
                (player_ids[0], json.dumps(new_list, separators=(",", ":")), existing[0]),
            )
            print(f"{p['shortname']}: default is now {wanted[0]}")
            continue

        con.execute(
            "insert into PlatformEntity (unique_id, name, shortname, description, is_visible,"
            " scrape_preview_media, accept_filename_regex, default_player_id, player_id_list,"
            " scraper_source_list, box_art_aspect_ratio_id, use_custom_box_art_aspect_ratio,"
            " custom_box_art_aspect_ratio, screen_aspect_ratio_id, use_custom_screen_aspect_ratio,"
            " custom_screen_aspect_ratio, items_sync_tree_uri_list, retro_achievements_console_id_list,"
            " extra) values (?, ?, ?, ?, 1, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (p["uniqueId"], p["name"], p["shortname"], p.get("description"),
             p["acceptedFilenameRegex"], player_ids[0], json.dumps(player_ids, separators=(",", ":")),
             json.dumps(p.get("scraperSourceList", []), separators=(",", ":")),
             p.get("boxArtAspectRatioId", 0), int(p.get("useCustomBoxArtAspectRatio", False)),
             p.get("customBoxArtAspectRatio"), p.get("screenAspectRatioId", 0),
             int(p.get("useCustomScreenAspectRatio", False)), p.get("customScreenAspectRatio"),
             json.dumps([SYNC_URI.format(folder=folder)]),
             json.dumps(p.get("retroAchievementsConsoleIdList", [])), p.get("extra", "")),
        )
        print(f"{p['shortname']}: added '{p['name']}' scanning ROMs/{folder}, players: {', '.join(wanted)}")

    con.commit()
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()


if __name__ == "__main__":
    main()
