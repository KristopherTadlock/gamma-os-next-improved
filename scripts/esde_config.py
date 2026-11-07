#!/usr/bin/env python3
"""Writes ES-DE's settings and game lists for the ROM pack.

  esde_config.py ES_SYSTEMS_XML EMULATORS_TXT CORES_TXT ROMS_TAR OUT_DIR

OUT_DIR gets ES-DE/settings/es_settings.xml, ES-DE/gamelists/<system>/
gamelist.xml for every system with a pack-in game or an emulator from
EMULATORS_TXT (keeping the details of the games in the ROM pack's own game
lists), and ROMs/scummvm/<game>.scummvm for each Daijisho ScummVM
template (.dpt) in the ROM pack. Fails if an emulator label isn't one of the
system's ES-DE commands or names a RetroArch core CORES_TXT doesn't ship, and
if a system's ES-DE default needs a core that isn't shipped while another
command's core is.
"""

import os
import re
import sys
import tarfile
import xml.etree.ElementTree as ET
from xml.sax.saxutils import escape

ROMS = "sdcard/ROMs/"
GAMELISTS = "sdcard/ES-DE/gamelists/"
SETTINGS = [
    ("string", "ROMDirectory", "/storage/emulated/0/ROMs"),
    ("bool", "InputTouchOverlay", "false"),
]
CORE_RE = re.compile(r"/cores/(\w+)_libretro_android\.so")


def read_cores(path):
    cores = set()
    for line in open(path):
        line = line.split("#")[0].strip()
        if line:
            cores.add(line.removeprefix("D - ").strip())
    return cores


def read_emulators(path):
    emulators = {}
    for line in open(path):
        line = line.split("#")[0].strip()
        if line:
            system, label = line.split(None, 1)
            emulators[system] = label
    return emulators


def core_of(command):
    m = CORE_RE.search(command.text or "")
    return m.group(1) if m else None


def check_emulators(systems, emulators, cores, folders):
    errors = []
    for system, label in emulators.items():
        commands = {c.get("label"): c for c in systems.get(system, ET.Element("x")).iter("command")}
        if label not in commands:
            errors.append(f"{system}: ES-DE has no emulator {label!r}")
        elif core_of(commands[label]) not in (None, *cores):
            errors.append(f"{system}: {label!r} needs the {core_of(commands[label])} core, which isn't shipped")
    for system in sorted(folders):
        if system in emulators or system not in systems:
            continue
        commands = list(systems[system].iter("command"))
        default = core_of(commands[0])
        shipped = [c.get("label") for c in commands if core_of(c) in cores]
        if default is not None and default not in cores and shipped:
            errors.append(f"{system}: ES-DE's default needs the {default} core; add a line for one of {shipped}")
    if errors:
        sys.exit("esde-emulators.txt:\n  " + "\n  ".join(errors))


def game_name(filename):
    for ext in (".p8.png",):
        if filename.endswith(ext):
            return filename[: -len(ext)]
    return os.path.splitext(filename)[0]


def scan_roms(roms_tar, systems):
    """Returns {system: [(path, name, folderlink)]} and the ScummVM templates."""
    games, templates = {}, {}
    with tarfile.open(roms_tar) as tar:
        for m in tar:
            if not m.name.startswith(ROMS):
                continue
            parts = m.name[len(ROMS):].strip("/").split("/")
            if len(parts) < 2 or parts[0] not in systems:
                continue
            system, rel = parts[0], "/".join(parts[1:])
            extensions = systems[parts[0]].findtext("extension").split()
            if m.isfile() and rel.endswith(".dpt"):
                ids = re.findall(r"^\[scummvm_id\]\s*(\S+)", tar.extractfile(m).read().decode(), re.M)
                templates[game_name(rel)] = ids[0]
            elif m.isfile() and len(parts) == 2 and os.path.splitext(rel)[1] in extensions:
                games.setdefault(system, []).append((f"./{rel}", game_name(rel), None))
            elif m.isfile() and len(parts) == 3 and parts[2] == "EBOOT.PBP":
                # A PSP game folder: launch its EBOOT.PBP straight from the list.
                games.setdefault(system, []).append((f"./{parts[1]}", parts[1], parts[2]))
    return games, templates


def pack_details(roms_tar):
    """Returns {(system, path): {tag: text}} from the game lists in the ROM pack."""
    details = {}
    with tarfile.open(roms_tar) as tar:
        for m in tar:
            if not (m.isfile() and m.name.startswith(GAMELISTS) and m.name.endswith("/gamelist.xml")):
                continue
            system = m.name[len(GAMELISTS):].split("/")[0]
            for game in ET.fromstring(tar.extractfile(m).read()).iter("game"):
                details[(system, game.findtext("path"))] = {c.tag: c.text for c in game if c.tag != "path"}
    return details


def write_gamelist(path, label, entries, details):
    lines = ['<?xml version="1.0"?>']
    if label:
        lines += ["<alternativeEmulator>", f"\t<label>{escape(label)}</label>", "</alternativeEmulator>"]
    lines.append("<gameList>")
    for rel, name, link in sorted(entries):
        tag = "folder" if link else "game"
        extra = dict(details.get(f"{rel}/{link}" if link else rel, {}))
        name = extra.pop("name", None) or name
        lines += [f"\t<{tag}>", f"\t\t<path>{escape(rel)}</path>", f"\t\t<name>{escape(name)}</name>"]
        if link:
            lines.append(f"\t\t<folderlink>{escape(link)}</folderlink>")
        lines += [f"\t\t<{key}>{escape(text)}</{key}>" for key, text in extra.items() if text]
        lines.append(f"\t</{tag}>")
    lines.append("</gameList>")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")


def main():
    systems_xml, emulators_txt, cores_txt, roms_tar, out = sys.argv[1:6]
    systems = {s.findtext("name"): s for s in ET.parse(systems_xml).getroot().iter("system")}
    emulators = read_emulators(emulators_txt)
    cores = read_cores(cores_txt)
    with tarfile.open(roms_tar) as tar:
        folders = {m.name[len(ROMS):].split("/")[0] for m in tar if m.name.startswith(ROMS) and m.isdir()}
    check_emulators(systems, emulators, cores, folders)

    games, templates = scan_roms(roms_tar, systems)
    details = pack_details(roms_tar)
    for name in templates:
        games.setdefault("scummvm", []).append((f"./{name}.scummvm", name, None))
    for system in sorted(set(games) | set(emulators)):
        write_gamelist(f"{out}/ES-DE/gamelists/{system}/gamelist.xml", emulators.get(system), games.get(system, []),
                       {path: d for (sys_name, path), d in details.items() if sys_name == system})
    for name, scummvm_id in templates.items():
        os.makedirs(f"{out}/ROMs/scummvm", exist_ok=True)
        with open(f"{out}/ROMs/scummvm/{name}.scummvm", "w") as f:
            f.write(scummvm_id + "\n")

    os.makedirs(f"{out}/ES-DE/settings", exist_ok=True)
    with open(f"{out}/ES-DE/settings/es_settings.xml", "w") as f:
        f.write('<?xml version="1.0"?>\n')
        for kind, name, value in SETTINGS:
            f.write(f'<{kind} name="{name}" value="{value}" />\n')
    print(f"ES-DE: {sum(map(len, games.values()))} games, {len(emulators)} default emulators, "
          f"{len(templates)} ScummVM games")


if __name__ == "__main__":
    main()
