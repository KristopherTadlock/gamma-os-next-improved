#!/usr/bin/env python3
"""Writes GammaOS Nano's game systems file from mods/nano-systems.txt.

  nano_systems.py NANO_SYSTEMS_TXT CATALOG_DIR ICONS_DIR APPS_LIST RETROARCH_CORES_TXT OUT_JSON [PACKAGE...]

Resolves each system's player in Nano's emulator catalog (CATALOG_DIR, the
Daijisho platform definitions) the way Nano's emulator picker applies one: a
RetroArch 64 player runs its core in Nano, any other player starts its app
with the player's am start arguments. Fails when a core isn't in
RETROARCH_CORES_TXT, an app isn't in APPS_LIST or the PACKAGEs (apps built
into the image), an icon doesn't exist, or a system has no file extensions, so
the list can't drift from what the image ships.
"""

import json
import os
import re
import sys

CORE_RE = re.compile(r"-e LIBRETRO (?:\S*/)?([A-Za-z0-9_\-]+?)(?:_libretro_android\.so)?(?:\s|$)")


def parse(path):
    for n, line in enumerate(open(path), 1):
        if not line.strip() or line.startswith("#"):
            continue
        rom_dir, *pairs = re.split(r" {2,}", line.strip())
        fields = dict(p.split("=", 1) for p in pairs)
        yield n, rom_dir, fields


def read_catalog(path):
    players = {}
    for name in os.listdir(path):
        if not name.endswith(".json"):
            continue
        definition = json.load(open(os.path.join(path, name)))
        platform_regex = definition["platform"].get("acceptedFilenameRegex", "")
        for player in definition["playerList"]:
            players[player["uniqueId"]] = (player, platform_regex)
    return players


def exts_from_regex(regex):
    """The first (?:a|b|c) group as ".a,.b,.c", like Nano's extsFromRegex."""
    m = re.search(r"\(\?:([^)]*)\)", regex)
    if not m:
        return ""
    return ",".join("." + e for e in m.group(1).split("|") if e and "\\" not in e)


def read_cores(path):
    cores = set()
    for line in open(path):
        fields = line.split("#")[0].split()
        if fields[:2] == ["D", "-"]:
            fields = fields[2:]
        if fields:
            cores.add(fields[0])
    return cores


def main():
    txt, catalog_dir, icons_dir, apps_list, cores_txt, out, *builtin = sys.argv[1:]
    catalog = read_catalog(catalog_dir)
    cores = read_cores(cores_txt)
    packages = {line.split("|")[1] for line in open(apps_list)
                if line.strip() and not line.startswith("#")} | set(builtin)

    systems, errors = [], []
    for order, (n, rom_dir, f) in enumerate(parse(txt)):
        system = {
            "id": rom_dir, "builtin": True, "enabled": True, "order": order,
            "displayName": f["name"], "shortname": f["short"], "romDir": rom_dir,
            "launchArgs": "", "scanSources": [],
        }
        if "player" in f:
            if f["player"] not in catalog:
                errors.append(f"line {n}: no player {f['player']} in Nano's catalog")
                continue
            player, platform_regex = catalog[f["player"]]
            args = " ".join(player["amStartArguments"].split())
            core = CORE_RE.search(args)
            if "com.retroarch.aarch64/" in args and core:
                system.update(launchType="libretro-core", package="", intentTemplate="",
                              coreSo=core.group(1) + "_libretro_android.so")
                if core.group(1) not in cores:
                    errors.append(f"line {n}: core {core.group(1)} isn't in retroarch-cores.txt")
            else:
                package = re.search(r"-n (\S+?)/", args).group(1)
                system.update(launchType="custom-package", package=package, coreSo="",
                              intentTemplate=args)
            exts = exts_from_regex(player.get("acceptedFilenameRegex", "")) or exts_from_regex(platform_regex)
        else:
            system.update(launchType="custom-package", package=f["package"], coreSo="",
                          intentTemplate=f["intent"])
            exts = f["exts"]
        if system["package"] and system["package"] not in packages:
            errors.append(f"line {n}: {system['package']} isn't installed by apps.list")
        system["extensions"] = exts.split(",")
        if not any(system["extensions"]):
            # Nano would list no games for the system.
            errors.append(f"line {n}: no file extensions for {rom_dir}")

        kind, _, icon = f["icon"].partition(":")
        if kind == "retroarch" and not os.path.exists(os.path.join(icons_dir, icon + ".png")):
            errors.append(f"line {n}: no icon {icon}.png")
        elif kind not in ("builtin", "retroarch"):
            errors.append(f"line {n}: unknown icon {f['icon']}")
        system["icon"] = {"ref": f["icon"], "tintR": 1, "tintG": 1, "tintB": 1}
        systems.append(system)

    if errors:
        sys.exit("\n".join(errors))
    json.dump({"version": 1, "systems": systems}, open(out, "w"), indent=2)
    print(f"Nano: {len(systems)} game systems")


if __name__ == "__main__":
    main()
