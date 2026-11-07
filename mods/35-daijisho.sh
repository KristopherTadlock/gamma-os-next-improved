#!/usr/bin/env bash
# Adds the platforms in daijisho-platforms.txt to Daijisho, sets each
# platform's default player from retroarch-cores.txt and removes players for
# cores that aren't shipped, gives platforms without a wallpaper one from
# daijisho-wallpapers.txt, and makes the game library match the ROM pack.
# Wallpapers for platforms not in Daijisho (systems that need a BIOS) are
# copied to /sdcard/Daijisho/Wallpapers/ on first boot. Runs after
# 30-bundled-roms.sh.
#
# Usage: 35-daijisho.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
SPARE_PACK="daijisho-wallpapers.tar"
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
python3 -I "$MODS/../scripts/daijisho_prune_players.py" "$db" "$MODS/retroarch-cores.txt"
python3 -I "$MODS/../scripts/daijisho_wallpapers.py" "$db" "$app/files" "$MODS/daijisho-wallpapers.txt" \
  "$2/daijisho-wallpapers" "$work/spare"
python3 -I "$MODS/../scripts/daijisho_library.py" "$db" "$ETC/roms.tar.xz"

# Repack the same top-level paths as the original, so no entries for data/
# itself are added; root would apply their owner and mode on extract.
tar -tJf "$ETC/daijisho.tar.xz" | python3 -I "$MODS/../scripts/tar_roots.py" >"$work/roots"
(cd "$work/tree" && tar -cpf - --owner=u0_a166:10166 --group=u0_a166:10166 -T "$work/roots") |
  xz -9e -T0 >"$work/daijisho.tar.xz"
sudo cp --no-preserve=all "$work/daijisho.tar.xz" "$ETC/daijisho.tar.xz"

if [[ -n "$(ls -A "$work/spare")" ]]; then
  # Files only, so extracting doesn't change the folder's owner or mode.
  (cd "$work/spare" && tar -cf "$work/$SPARE_PACK" --owner=0 --group=0 -- *)
  sudo install -m 644 -o root -g root "$work/$SPARE_PACK" "$ETC/$SPARE_PACK"
  sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$ETC/$SPARE_PACK"
  cat >"$work/setup-steps.sh" <<EOF
echo "Copying wallpapers for Daijisho platforms that need a BIOS."
mkdir -p /sdcard/Daijisho/Wallpapers
tar -xf /system/etc/$SPARE_PACK -C /sdcard/Daijisho/Wallpapers
chown -R "\$(stat -c %u:%g /sdcard/Download)" /sdcard/Daijisho

EOF
  marker='echo "Granting permissions to applications."'
  sudo grep -qxF "$marker" "$SETUP" || { echo "setup.sh has no line: $marker" >&2; exit 1; }
  sudo awk -v steps="$work/setup-steps.sh" -v marker="$marker" '
    $0 == marker { while ((getline line < steps) > 0) print line }
    { print }
  ' "$SETUP" >"$work/setup.sh"
  # Writing through the existing file keeps its owner, mode, and SELinux label.
  sudo cp --no-preserve=all "$work/setup.sh" "$SETUP"
fi
rm -rf "$work"
