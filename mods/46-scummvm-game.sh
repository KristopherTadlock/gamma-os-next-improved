#!/usr/bin/env bash
# Adds Beneath a Steel Sky to ScummVM's game list as "sky", the ID its
# ROMs/scummvm/Beneath a Steel Sky.scummvm file starts it with. Runs after
# 30-bundled-roms.sh and 40-added-apps.sh.
#
# On the 1.4.4 (Nano) base the game's files are in the ROM pack, in
# ROMs/scummvm/Beneath a Steel Sky/, where RetroArch's ScummVM core reads them
# too; that base lets every app read shared storage, so ScummVM's entry points
# there. First-boot setup writes ScummVM's settings file only when it has none,
# so a setup re-run keeps the user's own.
#
# On 1.2 the game is installed in ScummVM's own storage instead, as ScummVM
# may not read /sdcard/ROMs there without the user granting access.
#
# Usage: 46-scummvm-game.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
URL="https://downloads.scummvm.org/frs/extras/Beneath%20a%20Steel%20Sky/bass-cd-1.2.zip"
SHA256="53209b9400eab6fd7fa71518b2f357c8de75cfeaa5ba57024575ab79cc974593"
cache="$(realpath -m "$2")/scummvm"
work="$cache/work"
app="data/data/org.scummvm.scummvm"
source "$(dirname "$0")/../scripts/modlib.sh"

if is_nano_base "$SYSTEM"; then
  apk="$(sudo awk -F'|' '$1 !~ /^#/ && $2 == "org.scummvm.scummvm" { print $3 }' "$ETC/gammaos/apps.list")"
else
  apk="$(sudo sed -n 's#^pm install \(/system/etc/scummvm-.*\.apk\)$#\1#p' "$SETUP")"
fi
if [[ -z "$apk" ]]; then
  echo "ScummVM isn't in added-apps.txt; skipping"
  exit 0
fi
# ScummVM writes its version in its settings file: 2026.3.0 for scummvm-2026.3.0.apk.
version="$(basename "$apk" .apk)"
version="${version#scummvm-}"

# Prints ScummVM's settings file, with the game at path $1.
scummvm_ini() {
  cat <<EOF
[scummvm]
versioninfo=$version

[sky]
description=Beneath a Steel Sky (CD/DOS/English)
engineid=sky
gameid=sky
extra=CD
language=en
platform=pc
path=$1
EOF
}

rm -rf "$work"
mkdir -p "$work"

if is_nano_base "$SYSTEM"; then
  PACK="scummvm.tar.zst"
  GAME="sdcard/ROMs/scummvm/Beneath a Steel Sky"
  cat >"$work/hook.sh" <<EOF
post_scummvm() {
    # Only when ScummVM has no settings yet: a re-run keeps the user's games and options.
    [ -e /$app/files/scummvm.ini ] && return 0
    step "Adding Beneath a Steel Sky to ScummVM."
    extract_archive /system/etc/$PACK && own_app_data org.scummvm.scummvm && \\
        restorecon -R /$app/files
}
EOF
  if setup_has "$SYSTEM" "$work/hook.sh"; then
    rm -rf "$work"
    echo "ScummVM already lists Beneath a Steel Sky; left as it is"
    exit 0
  fi
  payload_to_tar "$(payload_path "$ETC" roms)" "$work/roms.tar"
  tar -tf "$work/roms.tar" >"$work/roms.list"
  rm "$work/roms.tar"
  if ! grep -qxF -e "$GAME/sky.dsk" "$work/roms.list" ||
     ! grep -qxF -e "$GAME.scummvm" "$work/roms.list"; then
    rm -rf "$work"
    echo "the ROM pack has no Beneath a Steel Sky (bundled-roms.txt); ScummVM is left to set itself up"
    exit 0
  fi

  mkdir -p "$work/tree/$app/files"
  scummvm_ini "/storage/emulated/0/${GAME#sdcard/}" >"$work/tree/$app/files/scummvm.ini"
  chmod 771 "$work/tree/$app/files"
  chmod 660 "$work/tree/$app/files/scummvm.ini"
  # Only the app's files folder: entries for data/ or data/data/ would reset
  # those directories' owner and mode when extracted.
  (cd "$work/tree" && tar -cf "$work/pack.tar" --owner=0 --group=0 "$app/files")
  tar_to_payload "$work/pack.tar" "$work/$PACK"
  install_system_file "$work/$PACK" "$ETC/$PACK"
  add_setup_functions "$SYSTEM" "$work/hook.sh"
  apps_list "$SYSTEM" hook org.scummvm.scummvm post_scummvm
  echo "ScummVM will list Beneath a Steel Sky from /${GAME#sdcard/}"
  rm -rf "$work"
  exit 0
fi

PACK="scummvm-bass.tar.xz"
GAME_DIR="games/Beneath a Steel Sky"
# The setup lines come first: when setup.sh already has them, this mod ran on
# the image before.
cat >"$work/setup-steps.sh" <<EOF
echo "Adding Beneath a Steel Sky to ScummVM."
launcheruser=\$(stat -c "%U" /$app) && \\
launchergroup=\$(stat -c "%G" /$app) && \\
tar -xJf /system/etc/$PACK -C / && \\
chown -R \$launcheruser:\$launchergroup /$app/files && \\
restorecon -R /$app/files

EOF
if setup_has "$SYSTEM" "$work/setup-steps.sh"; then
  rm -rf "$work"
  echo "ScummVM already lists Beneath a Steel Sky; left as it is"
  exit 0
fi

zip="$cache/bass-cd-1.2.zip"
[[ -f "$zip" ]] || { curl -fsSL --retry 3 -o "$zip.partial" "$URL" && mv "$zip.partial" "$zip"; } || { echo "download failed: $URL" >&2; exit 1; }
echo "$SHA256  $zip" | sha256sum -c --quiet || { rm -f "$zip"; echo "checksum mismatch: $zip (deleted from the cache)" >&2; exit 1; }

mkdir -p "$work/tree/$app/files/$GAME_DIR"
# The license asks for the readme to stay with the game, so it ships unchanged.
unzip -q -j "$zip" -d "$work/tree/$app/files/$GAME_DIR"
scummvm_ini "/data/user/0/org.scummvm.scummvm/files/$GAME_DIR" >"$work/tree/$app/files/scummvm.ini"
find "$work/tree" -type d -exec chmod 771 {} +
find "$work/tree" -type f -exec chmod 660 {} +
(cd "$work/tree" && tar -cf - --owner=0 --group=0 "$app/files") | xz -9 -T0 >"$work/$PACK"

install_system_file "$work/$PACK" "$ETC/$PACK"
sudo awk -v steps="$work/setup-steps.sh" -v marker="$(setup_marker "$work/setup-steps.sh")" '
  { print }
  /^pm install \/system\/etc\/scummvm-.*\.apk$/ { print ""; print marker; while ((getline line < steps) > 0) print line }
' "$SETUP" >"$work/setup.sh"
write_through "$work/setup.sh" "$SETUP"
echo "ScummVM will list Beneath a Steel Sky ($(stat -c %s "$work/$PACK") bytes compressed)"
rm -rf "$work"
