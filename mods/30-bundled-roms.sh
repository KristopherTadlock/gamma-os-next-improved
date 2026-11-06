#!/usr/bin/env bash
# Rebuilds the ROM pack installed on first boot from bundled-roms.txt.
#
# Usage: 30-bundled-roms.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

ETC="$1/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
work="$2/roms"

mkdir -p "$work"
python3 -I "$MODS/../scripts/bundled_roms.py" "$MODS/bundled-roms.txt" \
  "$ETC/roms.tar.xz" "$work/roms.tar.xz" "$work/cache" "$MODS/rom-licenses"
echo "roms.tar.xz: $(stat -c %s "$ETC/roms.tar.xz") -> $(stat -c %s "$work/roms.tar.xz") bytes"
sudo cp --no-preserve=all "$work/roms.tar.xz" "$ETC/roms.tar.xz"
rm "$work/roms.tar.xz"
