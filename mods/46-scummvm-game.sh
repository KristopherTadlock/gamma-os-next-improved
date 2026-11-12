#!/usr/bin/env bash
# Installs Beneath a Steel Sky into ScummVM's own storage and adds it to
# ScummVM's game list as "sky", the ID ES-DE launches it with
# (ROMs/scummvm/Beneath a Steel Sky.scummvm). ScummVM can't read /sdcard/ROMs
# until the user grants it access, so the game can't live there. Runs after
# 40-added-apps.sh.
#
# Usage: 46-scummvm-game.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
URL="https://downloads.scummvm.org/frs/extras/Beneath%20a%20Steel%20Sky/bass-cd-1.2.zip"
SHA256="53209b9400eab6fd7fa71518b2f357c8de75cfeaa5ba57024575ab79cc974593"
PACK="scummvm-bass.tar.xz"
GAME_DIR="games/Beneath a Steel Sky"
cache="$(realpath -m "$2")/scummvm"
work="$cache/work"
source "$(dirname "$0")/../scripts/modlib.sh"

if is_nano_base "$SYSTEM"; then
  installed() { sudo grep -q '^[^#][^|]*|org\.scummvm\.scummvm|' "$ETC/gammaos/apps.list"; }
else
  installed() { sudo grep -q '^pm install /system/etc/scummvm-.*\.apk$' "$SETUP"; }
fi
if ! installed; then
  echo "ScummVM isn't in added-apps.txt; skipping"
  exit 0
fi

mkdir -p "$cache"
zip="$cache/bass-cd-1.2.zip"
[[ -f "$zip" ]] || { curl -fsSL --retry 3 -o "$zip.partial" "$URL" && mv "$zip.partial" "$zip"; } || { echo "download failed: $URL" >&2; exit 1; }
echo "$SHA256  $zip" | sha256sum -c --quiet || { rm -f "$zip"; echo "checksum mismatch: $zip (deleted from the cache)" >&2; exit 1; }

app="data/data/org.scummvm.scummvm"
rm -rf "$work"
mkdir -p "$work"
# The setup lines come first: when setup.sh already has them, this mod ran on
# the image before.
if is_nano_base "$SYSTEM"; then
  setup_lines="$work/hook.sh"
  cat >"$setup_lines" <<EOF
post_scummvm() {
    step "Adding Beneath a Steel Sky to ScummVM."
    tar -xJf /system/etc/$PACK -C / && own_app_data org.scummvm.scummvm && \\
        restorecon -R /$app/files
}
EOF
else
  setup_lines="$work/setup-steps.sh"
  cat >"$setup_lines" <<EOF
echo "Adding Beneath a Steel Sky to ScummVM."
launcheruser=\$(stat -c "%U" /$app) && \\
launchergroup=\$(stat -c "%G" /$app) && \\
tar -xJf /system/etc/$PACK -C / && \\
chown -R \$launcheruser:\$launchergroup /$app/files && \\
restorecon -R /$app/files

EOF
fi
if setup_has "$SYSTEM" "$setup_lines"; then
  rm -rf "$work"
  echo "ScummVM already lists Beneath a Steel Sky; left as it is"
  exit 0
fi

mkdir -p "$work/tree/$app/files/$GAME_DIR"
# The license asks for the readme to stay with the game, so it ships unchanged.
unzip -q -j "$zip" -d "$work/tree/$app/files/$GAME_DIR"
cat >"$work/tree/$app/files/scummvm.ini" <<EOF
[scummvm]
versioninfo=2026.3.0

[sky]
description=Beneath a Steel Sky (CD/DOS/English)
engineid=sky
gameid=sky
extra=CD
language=en
platform=pc
path=/data/user/0/org.scummvm.scummvm/files/$GAME_DIR
EOF
find "$work/tree" -type d -exec chmod 771 {} +
find "$work/tree" -type f -exec chmod 660 {} +
# Only the app's files folder: entries for data/ or data/data/ would reset
# those directories' owner and mode when extracted.
(cd "$work/tree" && tar -cf - --owner=0 --group=0 "$app/files") | xz -9 -T0 >"$work/$PACK"

install_system_file "$work/$PACK" "$ETC/$PACK"

if is_nano_base "$SYSTEM"; then
  add_setup_functions "$SYSTEM" "$setup_lines"
  apps_list "$SYSTEM" hook org.scummvm.scummvm post_scummvm
else
  sudo awk -v steps="$setup_lines" -v marker="$(setup_marker "$setup_lines")" '
    { print }
    /^pm install \/system\/etc\/scummvm-.*\.apk$/ { print ""; print marker; while ((getline line < steps) > 0) print line }
  ' "$SETUP" >"$work/setup.sh"
  write_through "$work/setup.sh" "$SETUP"
fi
echo "ScummVM will list Beneath a Steel Sky ($(stat -c %s "$work/$PACK") bytes compressed)"
rm -rf "$work"
