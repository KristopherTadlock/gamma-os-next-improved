#!/usr/bin/env bash
# Sets Daijisho's default player per platform from retroarch-cores.txt.
#
# Usage: 20-daijisho-defaults.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
work="$2/daijisho"
db="$work/tree/data/data/com.magneticchen.daijishou/databases/Daijishou.db"

rm -rf "$work"
mkdir -p "$work/tree"
# setup.sh chowns these files after extracting them, so ownership is not kept.
tar -xJpf "$ETC/daijisho.tar.xz" --no-same-owner -C "$work/tree"

python3 -I "$MODS/../scripts/daijisho_defaults.py" "$db" "$MODS/retroarch-cores.txt"

(cd "$work/tree" && tar -cpf - --owner=u0_a166:10166 --group=u0_a166:10166 data) |
  xz -9e -T0 >"$work/daijisho.tar.xz"
sudo cp --no-preserve=all "$work/daijisho.tar.xz" "$ETC/daijisho.tar.xz"
rm -rf "$work"
