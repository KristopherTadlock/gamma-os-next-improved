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
SPARE_PACK="daijisho-wallpapers.tar"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
work="$(realpath -m "$2")/daijisho"
app="$work/tree/data/data/com.magneticchen.daijishou"
db="$app/databases/Daijishou.db"

if [[ -n "${ESDE_APK:-}" ]]; then
  echo "ES-DE replaces Daijisho; skipping"
  exit 0
fi

rm -rf "$work"
mkdir -p "$work/tree"
pack="$(payload_path "$ETC" daijisho)"
payload_to_tar "$pack" "$work/in.tar"
payload_to_tar "$(payload_path "$ETC" roms)" "$work/roms.tar"
# setup.sh chowns these files after extracting them, so ownership is not kept.
tar -xpf "$work/in.tar" --no-same-owner -C "$work/tree"

python3 -I "$MODS/../scripts/daijisho_platforms.py" "$db" "$MODS/daijisho-platforms.txt" "$2/daijisho-platforms"
python3 -I "$MODS/../scripts/daijisho_defaults.py" "$db" "$MODS/retroarch-cores.txt"
python3 -I "$MODS/../scripts/daijisho_prune_players.py" "$db" "$MODS/retroarch-cores.txt"
python3 -I "$MODS/../scripts/daijisho_wallpapers.py" "$db" "$app/files" "$MODS/daijisho-wallpapers.txt" \
  "$2/daijisho-wallpapers" "$work/spare"
python3 -I "$MODS/../scripts/daijisho_library.py" "$db" "$work/roms.tar"

# Repack the same top-level paths as the original, so no entries for data/
# itself are added; root would apply their owner and mode on extract.
tar -tf "$work/in.tar" | python3 -I "$MODS/../scripts/tar_roots.py" >"$work/roots"
(cd "$work/tree" && tar -cpf "$work/out.tar" --owner=u0_a166:10166 --group=u0_a166:10166 -T "$work/roots")
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
write_through "$work/${pack##*/}" "$pack"

if [[ -n "$(ls -A "$work/spare")" ]]; then
  # Files only, so extracting doesn't change the folder's owner or mode.
  (cd "$work/spare" && tar -cf "$work/$SPARE_PACK" --owner=0 --group=0 -- *)
  install_system_file "$work/$SPARE_PACK" "$ETC/$SPARE_PACK"
  cat >"$work/setup-steps.sh" <<EOF
echo "Copying wallpapers for Daijisho platforms that need a BIOS."
mkdir -p /sdcard/Daijisho/Wallpapers
tar -xf /system/etc/$SPARE_PACK -C /sdcard/Daijisho/Wallpapers
chown -R "\$(stat -c %u:%g /sdcard/Download)" /sdcard/Daijisho

EOF
  add_setup_steps "$SYSTEM" "$work/setup-steps.sh"
fi
rm -rf "$work"
