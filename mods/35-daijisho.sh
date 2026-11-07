#!/usr/bin/env bash
# Adds the platforms in daijisho-platforms.txt to Daijisho, sets each
# platform's default player from retroarch-cores.txt, gives platforms without
# a wallpaper one from daijisho-wallpapers/, and makes the game library match
# the ROM pack. Runs after 30-bundled-roms.sh.
#
# Usage: 35-daijisho.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
work="$(realpath -m "$2")/daijisho"
app="$work/tree/data/data/com.magneticchen.daijishou"
db="$app/databases/Daijishou.db"

rm -rf "$work"
mkdir -p "$work/tree"
# setup.sh chowns these files after extracting them, so ownership is not kept.
tar -xJpf "$ETC/daijisho.tar.xz" --no-same-owner -C "$work/tree"

python3 -I "$MODS/../scripts/daijisho_platforms.py" "$db" "$MODS/daijisho-platforms.txt" "$2/daijisho-platforms"
python3 -I "$MODS/../scripts/daijisho_defaults.py" "$db" "$MODS/retroarch-cores.txt"
python3 -I "$MODS/../scripts/daijisho_wallpapers.py" "$db" "$app/files" "$MODS/daijisho-wallpapers"
python3 -I "$MODS/../scripts/daijisho_library.py" "$db" "$ETC/roms.tar.xz"

# Repack the same top-level paths as the original, so no entries for data/
# itself are added; root would apply their owner and mode on extract.
tar -tJf "$ETC/daijisho.tar.xz" | python3 -I "$MODS/../scripts/tar_roots.py" >"$work/roots"
(cd "$work/tree" && tar -cpf - --owner=u0_a166:10166 --group=u0_a166:10166 -T "$work/roots") |
  xz -9e -T0 >"$work/daijisho.tar.xz"
sudo cp --no-preserve=all "$work/daijisho.tar.xz" "$ETC/daijisho.tar.xz"
rm -rf "$work"
