#!/usr/bin/env bash
# Updates the non-emulator apps GammaOS Next installs on first boot, and
# replaces Firefox with Brave.
#
# Kept as they are:
#   MiXplorer 6.64.3     newer builds are only attached to its XDA thread,
#                        which can't be downloaded from a script
#   M64Plus FZ 3.0.335   newer builds are only on Google Play
#   DraStic r2.6.0.4a    the latest release
#   Magisk 30.6          the boot image is patched with this version; the app
#                        can update itself and the boot image after setup
#   Daijisho 1.8.1       on the 1.4.4 base, which already ships it
#
# Brave is made the default browser on a fresh setup, so a choice made later
# survives setup re-running on a Nano/Android switch.
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
if ! is_nano_base "$SYSTEM"; then
  APKS+=("Daijisho 1.5.0|https://github.com/TapiocaFox/Daijishou/releases/download/v1.5.0/416.apk|c09989b5d938f41cc1d53d2f38fa3b1261a40bc4d8dcf8367cce83353457df51|daijisho-1.5.0.apk|daijisho412.apk.xz")
fi

mkdir -p "$CACHE"
sudo cat "$SETUP" >"$CACHE/setup.sh"

for entry in "${APKS[@]}"; do
  IFS='|' read -r name url sha file old <<<"$entry"
  [[ -f "$CACHE/$file" ]] || { curl -fsSL --retry 3 -o "$CACHE/$file.partial" "$url" && mv "$CACHE/$file.partial" "$CACHE/$file"; }
  echo "$sha  $CACHE/$file" | sha256sum -c --quiet
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
if [ "${FRESH_SETUP:-1}" = 1 ] && pm path com.brave.browser >/dev/null 2>&1; then
    echo "Making Brave the default browser."
    cmd role add-role-holder --user 0 android.app.role.BROWSER com.brave.browser
fi

STEPS
  add_setup_steps "$SYSTEM" "$CACHE/browser-steps.sh"
  echo "Brave is the default browser"
}

if is_nano_base "$SYSTEM"; then
  default_browser
  exit 0
fi

# Daijisho was shipped xz-compressed and unpacked to /sdcard before installing.
python3 -I - "$CACHE/setup.sh" <<'EOF'
import sys

path = sys.argv[1]
setup = open(path).read()
old_daijisho = """mkdir -p /sdcard/daijisho && \\
cp /system/etc/daijisho-1.5.0.apk /sdcard/daijisho && \\
cd /sdcard/daijisho && \\
xz -d daijisho412.apk.xz

pm install /sdcard/daijisho/daijisho412.apk

cd /
rm -rf /sdcard/daijisho
"""
# Keep the "cd /": later steps use paths relative to / (system/etc/Toast.apk).
replacements = [
    (old_daijisho, "pm install /system/etc/daijisho-1.5.0.apk\ncd /\n"),
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
