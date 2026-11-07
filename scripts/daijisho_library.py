#!/usr/bin/env python3
"""Makes Daijisho's game library match the games in the ROM pack.

  daijisho_library.py DAIJISHOU_DB ROMS_TAR

GammaOS ships Daijisho with its library already synced. This removes the
entries for games no longer in the pack and adds the pack's other games: the
files directly in each platform's ROMs folder, and EBOOT.PBP files one folder
down (named after their folder). It also makes every platform skip the
pack's license and systeminfo.txt files when the library is synced again.
"""

import hashlib
import json
import os
import sqlite3
import sys
import tarfile
import time
from urllib.parse import quote

ROMS = "sdcard/ROMs/"
TREE = "content://com.android.externalstorage.documents/tree/"
SKIP_PREFIX = r"(?!.*\.license\.txt$)(?!systeminfo\.txt$)"
SKIP = {"systeminfo.txt", "systems.txt", ".nomedia"}


def encode(path):
    # Android's Uri.encode leaves these characters as they are.
    return quote(path, safe="!~*'()")


def game_name(file_name):
    for suffix in (".p8.png",):
        if file_name.lower().endswith(suffix):
            return file_name[: -len(suffix)]
    return os.path.splitext(file_name)[0]


def pack_games(roms_tar):
    games = {}
    with tarfile.open(roms_tar) as tar:
        for info in tar:
            if not info.isfile() or not info.name.startswith(ROMS):
                continue
            parts = info.name[len(ROMS):].split("/")
            base = parts[-1]
            if base in SKIP or base.endswith(".license.txt") or base.startswith("."):
                continue
            if len(parts) == 2:
                name = game_name(base)
            elif len(parts) == 3 and base == "EBOOT.PBP":
                name = parts[1]
            else:
                continue
            md5 = hashlib.md5(tar.extractfile(info).read()).hexdigest()
            games["/".join(parts)] = (parts[0], name, md5)
    return games


def main():
    db_path, roms_tar = sys.argv[1:3]
    con = sqlite3.connect(db_path)
    games = pack_games(roms_tar)

    # ROMs folder -> (platform id, sync tree URI)
    folders = {}
    for platform_id, uris in con.execute("select id, items_sync_tree_uri_list from PlatformEntity"):
        for uri in json.loads(uris):
            if uri.startswith(TREE + "primary%3AROMs%2F"):
                folders[uri[len(TREE + "primary%3AROMs%2F"):]] = (platform_id, uri)

    wanted = {}
    for rel, (folder, name, md5) in games.items():
        if folder not in folders:
            sys.exit(f"no Daijisho platform scans ROMs/{folder} (for {rel})")
        platform_id, tree = folders[folder]
        uri = f"{tree}/document/{encode('primary:ROMs/' + rel)}"
        wanted[uri] = (platform_id, tree, name, md5)

    removed = 0
    for item_id, uri in con.execute("select id, uri from PlayableItemEntity").fetchall():
        if uri not in wanted:
            con.execute("delete from PlayableItemToGenrePairEntity where playable_item_id = ?", (item_id,))
            con.execute("delete from EventEntity where playable_item_id = ?", (item_id,))
            con.execute("delete from PlayableItemEntity where id = ?", (item_id,))
            removed += 1

    existing = {uri for uri, in con.execute("select uri from PlayableItemEntity")}
    now = int(time.time() * 1000)
    added = 0
    for uri, (platform_id, tree, name, md5) in sorted(wanted.items()):
        if uri in existing:
            continue
        cur = con.execute(
            "insert into PlayableItemEntity (is_singleton_disjoint_set,"
            " use_disjoint_set_primary_playable_item, playable_item_type, name, do_scrape_name,"
            " scraper_keyword_list, scrape_scraper_keywords, is_favorite, is_searchable, is_visible,"
            " attached_platform_id, uri, sync_tree_uri, md5, use_custom_player, play_count,"
            " added_date_timestamp, extra) values (1, 0, 1, ?, 1, ?, 1, 0, 1, 1, ?, ?, ?, ?, 0, 0, ?, '')",
            (name, json.dumps([name], ensure_ascii=False), platform_id, uri, tree, md5, now),
        )
        con.execute("update PlayableItemEntity set disjoint_set_primary_playable_item_id = id where id = ?",
                    (cur.lastrowid,))
        added += 1

    for platform_id, regex in con.execute("select id, accept_filename_regex from PlatformEntity").fetchall():
        if regex.startswith("^") and not regex.startswith("^" + SKIP_PREFIX):
            con.execute("update PlatformEntity set accept_filename_regex = ? where id = ?",
                        ("^" + SKIP_PREFIX + regex[1:], platform_id))

    con.commit()
    con.execute("pragma wal_checkpoint(truncate)")
    con.close()
    print(f"library: {len(wanted)} games ({added} added, {removed} removed)")


if __name__ == "__main__":
    main()
