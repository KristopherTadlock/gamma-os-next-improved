#!/usr/bin/env bash
# Rebuilds the ROM pack installed on first boot from bundled-roms.txt.
#
# Usage: 30-bundled-roms.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

ETC="$1/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
work="$2/roms"
pack="$(payload_path "$ETC" roms)"

mkdir -p "$work"
payload_to_tar "$pack" "$work/in.tar"
python3 -I "$MODS/../scripts/bundled_roms.py" "$MODS/bundled-roms.txt" \
  "$work/in.tar" "$work/out.tar" "$work/cache" "$MODS/rom-licenses" "$MODS/rom-files"
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
echo "${pack##*/}: $(stat -c %s "$pack") -> $(stat -c %s "$work/${pack##*/}") bytes"
write_through "$work/${pack##*/}" "$pack"
rm "$work/in.tar" "$work/out.tar" "$work/${pack##*/}"
