#!/usr/bin/env bash
# Updates the non-emulator apps GammaOS Next installs on first boot, replaces
# Firefox with Brave, and removes Daijisho and MiXplorer, closed apps with no
# grant to redistribute them. Android's home is then Trebuchet, the launcher
# both bases ship.
#
# Kept as they are:
#   M64Plus FZ 3.0.335   newer builds are only on Google Play
#   DraStic r2.6.0.4a    the latest release
#   Magisk 30.6          the boot image is patched with this version; the app
#                        can update itself and the boot image after setup
#
# Brave is made the default browser on a fresh setup, so a choice made later
# survives setup re-running on a Nano/Android switch. Setup skips Brave on
# devices with 1.3 GB of RAM or less; on the 1.4.4 base those get Fulguris
# (MPL-2.0 / CPAL-1.0), a light browser with controller support, instead.
#
# Usage: 15-update-apps.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
CACHE="$(realpath -m "$2")/updated-apps"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
source "$(dirname "$0")/../scripts/modlib.sh"

# name|url|sha256|file name in /system/etc|file it replaces (a pattern)
APKS=(
  # IzzyOnDroid's reproducible build, signed with the developers' key like the
  # one it replaces. It only keeps recent versions, so this URL will expire.
  "Aurora Store 4.8.4|https://apt.izzysoft.de/fdroid/repo/com.aurora.store_76.apk|8a1ed9aa09631290da91cb793e0517b0f20dc70239ac94ae6682cd94f91a4bad|AuroraStore_4.8.4.apk|AuroraStore_*.apk"
  "Brave 1.99.55|https://github.com/brave/brave-browser/releases/download/v1.99.55/Bravearm64Universal.apk|c4ff5faf1c3828e6b438d10ca76b49be605604c4ebe055b2ebcc612106f0b997|Brave_1.99.55_arm64.apk|*fenix-*.apk"
)
# F-Droid's build. F-Droid moves older versions to its archive repo, so this
# URL will need updating for later builds.
FULGURIS_URL=https://f-droid.org/repo/net.slions.fulguris.full.fdroid_265.apk
FULGURIS_SHA256=72a95ef9111837e6a557cefac53744e9a8b4f8413fce199e485e249d04c886cc
FULGURIS_FILE=Fulguris_2.1.2.apk
FULGURIS_PKG=net.slions.fulguris.full.fdroid
LOW_MEM_KB=1300000
# Daijisho's app and data, and MiXplorer: 1.2's names, then 1.4.4's.
REMOVED=(daijisho412.apk.xz daijisho.tar.xz daijisho daijisho.tar.zst MiXplorer_v6.64.3-API29_B23090720.apk)

mkdir -p "$CACHE"
sudo cat "$SETUP" >"$CACHE/setup.sh"
sudo rm -rf "${REMOVED[@]/#/$ETC/}"

for entry in "${APKS[@]}"; do
  IFS='|' read -r name url sha file old <<<"$entry"
  [[ -f "$CACHE/$file" ]] || { curl -fsSL --retry 3 -o "$CACHE/$file.partial" "$url" && mv "$CACHE/$file.partial" "$CACHE/$file"; } || { echo "download failed: $url" >&2; exit 1; }
  echo "$sha  $CACHE/$file" | sha256sum -c --quiet || { rm -f "$CACHE/$file"; echo "checksum mismatch: $CACHE/$file (deleted from the cache)" >&2; exit 1; }
  matches=("$ETC"/$old)
  [[ ${#matches[@]} == 1 && -f "${matches[0]}" ]] || { echo "no single $old in $ETC" >&2; exit 1; }
  old="${matches[0]##*/}"
  # Renaming, then writing through, keeps the owner, mode, and SELinux label.
  sudo mv "$ETC/$old" "$ETC/$file"
  write_through "$CACHE/$file" "$ETC/$file"
  if is_nano_base "$SYSTEM"; then
    case "$file" in
      Brave_*) apps_list "$SYSTEM" replace org.mozilla.firefox_beta Brave com.brave.browser "/system/etc/$file" ;;
      *) apps_list "$SYSTEM" source "/system/etc/$old" "/system/etc/$file" ;;
    esac
  else
    sed -i "s#/system/etc/$old#/system/etc/$file#g" "$CACHE/setup.sh"
  fi
  echo "updated $name (replaces $old)"
done

default_browser() {
  cat >"$CACHE/browser-steps.sh" <<'STEPS'
if [ "${FRESH_SETUP:-1}" = 1 ]; then
    if pm path com.brave.browser >/dev/null 2>&1; then
        echo "Making Brave the default browser."
        cmd role add-role-holder --user 0 android.app.role.BROWSER com.brave.browser
    elif pm path net.slions.fulguris.full.fdroid >/dev/null 2>&1; then
        echo "Making Fulguris the default browser."
        cmd role add-role-holder --user 0 android.app.role.BROWSER net.slions.fulguris.full.fdroid
    fi
fi

STEPS
  add_setup_steps "$SYSTEM" "$CACHE/browser-steps.sh"
  echo "Brave (Fulguris on low-memory devices) is the default browser"
}

if is_nano_base "$SYSTEM"; then
  # The source patch drops them; a system image built without it still lists them.
  for pkg in com.magneticchen.daijishou com.mixplorer; do
    if sudo grep -qF "|$pkg|" "$ETC/gammaos/apps.list"; then
      apps_list "$SYSTEM" remove "$pkg"
    fi
  done
  # An image built without the source patch would ignore maxmem and install it everywhere.
  sudo grep -qF 'maxmem:*)' "$SETUP" || { echo "setup.sh has no maxmem condition" >&2; exit 1; }
  fulguris="$CACHE/$FULGURIS_FILE"
  [[ -f "$fulguris" ]] || { curl -fsSL --retry 3 -o "$fulguris.partial" "$FULGURIS_URL" && mv "$fulguris.partial" "$fulguris"; } || { echo "download failed: $FULGURIS_URL" >&2; exit 1; }
  echo "$FULGURIS_SHA256  $fulguris" | sha256sum -c --quiet || { rm -f "$fulguris"; echo "checksum mismatch: $fulguris (deleted from the cache)" >&2; exit 1; }
  install_system_file "$fulguris" "$ETC/$FULGURIS_FILE"
  if ! sudo grep -qF "|$FULGURIS_PKG|" "$ETC/gammaos/apps.list"; then
    apps_list "$SYSTEM" add Fulguris "$FULGURIS_PKG" "/system/etc/$FULGURIS_FILE" "maxmem:$LOW_MEM_KB"
  fi
  echo "added Fulguris for devices with $LOW_MEM_KB kB of RAM or less"
  default_browser
  exit 0
fi

python3 -I - "$CACHE/setup.sh" <<'EOF'
import sys

path = sys.argv[1]
setup = open(path).read()
old_daijisho = """echo "Installing Daijisho." && \\
mkdir -p /sdcard/daijisho && \\
cp /system/etc/daijisho412.apk.xz /sdcard/daijisho && \\
cd /sdcard/daijisho && \\
xz -d daijisho412.apk.xz

pm install /sdcard/daijisho/daijisho412.apk

cd /
rm -rf /sdcard/daijisho
launcheruser=$( stat -c "%U" /data/data/com.magneticchen.daijishou) && \\
launchergroup=$( stat -c "%G" /data/data/com.magneticchen.daijishou) && \\
tar -xJvf /system/etc/daijisho.tar.xz -P -C / && \\
chown -R $launcheruser:$launchergroup /data/data/com.magneticchen.daijishou

"""
daijisho_home = """cmd package set-home-activity com.magneticchen.daijishou/.app.HomeActivity
pm set-home-activity com.magneticchen.daijishou/.app.HomeActivity -user --user 0
"""
mixplorer = """echo "Installing MiXplorer."
pm install /system/etc/MiXplorer_v6.64.3-API29_B23090720.apk

"""
# Keep the "cd /": later steps use paths relative to / (system/etc/Toast.apk).
replacements = [
    (old_daijisho, "cd /\n\n"),
    (daijisho_home, ""),
    (mixplorer, ""),
    ('echo "Installing FireFox"\n', 'echo "Installing Brave."\n'),
]
for old, new in replacements:
    if setup.count(old) != 1:
        sys.exit(f"setup.sh doesn't contain exactly one {old!r}")
    setup = setup.replace(old, new)
open(path, "w").write(setup)
EOF
write_through "$CACHE/setup.sh" "$SETUP"
default_browser
