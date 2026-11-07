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
list="$MODS/bundled-roms.txt"
if is_nano_base "$1"; then
  # Daijisho's launch files (.dpt) do nothing without it; ScummVM lists its game itself.
  grep -v '\.dpt  ' "$list" >"$work/bundled-roms.txt"
  list="$work/bundled-roms.txt"
fi
payload_to_tar "$pack" "$work/in.tar"
python3 -I "$MODS/../scripts/bundled_roms.py" "$list" \
  "$work/in.tar" "$work/out.tar" "$work/cache" "$MODS/rom-licenses" "$MODS/rom-files"
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
echo "${pack##*/}: $(stat -c %s "$pack") -> $(stat -c %s "$work/${pack##*/}") bytes"
write_through "$work/${pack##*/}" "$pack"
rm -f "$work/in.tar" "$work/out.tar" "$work/${pack##*/}" "$work/bundled-roms.txt"
