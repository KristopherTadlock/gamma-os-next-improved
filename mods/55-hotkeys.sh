#!/usr/bin/env bash
# Installs the universal hotkey profiles (hotkeys/*.json) where the framework
# and Nano read them, /system/etc/gammaos/hotkeys. HOTKEYS.md describes the
# format. A profile in /data/system/gammaos/hotkeys with the same id replaces a
# built-in one.
#
# Usage: 55-hotkeys.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"

if ! is_nano_base "$SYSTEM"; then
  echo "no GammaOS Nano in this base; skipping"
  exit 0
fi

dest="$SYSTEM/etc/gammaos/hotkeys"
sudo install -d -m 755 -o root -g root "$dest"
sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$dest"
for f in "$MODS"/hotkeys/*.json; do
  install_system_file "$f" "$dest/$(basename "$f")"
done
echo "installed $(ls "$MODS"/hotkeys/*.json | wc -l) hotkey profiles"
