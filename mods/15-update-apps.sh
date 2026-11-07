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
#
# Usage: 15-update-apps.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
CACHE="$(realpath -m "$2")/updated-apps"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"

# name|url|sha256|file name in /system/etc|file name it replaces
APKS=(
  "Daijisho 1.5.0|https://github.com/TapiocaFox/Daijishou/releases/download/v1.5.0/416.apk|c09989b5d938f41cc1d53d2f38fa3b1261a40bc4d8dcf8367cce83353457df51|daijisho-1.5.0.apk|daijisho412.apk.xz"
  # IzzyOnDroid's reproducible build, signed with the developers' key like the
  # one it replaces. It only keeps recent versions, so this URL will expire.
  "Aurora Store 4.8.4|https://apt.izzysoft.de/fdroid/repo/com.aurora.store_76.apk|8a1ed9aa09631290da91cb793e0517b0f20dc70239ac94ae6682cd94f91a4bad|AuroraStore_4.8.4.apk|AuroraStore_4.6.2.apk"
  "Brave 1.99.55|https://github.com/brave/brave-browser/releases/download/v1.99.55/Bravearm64Universal.apk|c4ff5faf1c3828e6b438d10ca76b49be605604c4ebe055b2ebcc612106f0b997|Brave_1.99.55_arm64.apk|firefox-fenix-139.0.multi.android-arm64-v8a.apk"
)

mkdir -p "$CACHE"
sudo cat "$SETUP" >"$CACHE/setup.sh"

for entry in "${APKS[@]}"; do
  IFS='|' read -r name url sha file old <<<"$entry"
  [[ -f "$CACHE/$file" ]] || { curl -fsSL --retry 3 -o "$CACHE/$file.partial" "$url" && mv "$CACHE/$file.partial" "$CACHE/$file"; }
  echo "$sha  $CACHE/$file" | sha256sum -c --quiet
  # Renaming, then writing through, keeps the owner, mode, and SELinux label.
  sudo mv "$ETC/$old" "$ETC/$file"
  sudo cp --no-preserve=all "$CACHE/$file" "$ETC/$file"
  sed -i "s#/system/etc/$old#/system/etc/$file#g" "$CACHE/setup.sh"
  echo "updated $name (replaces $old)"
done

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
replacements = [
    (old_daijisho, "pm install /system/etc/daijisho-1.5.0.apk\n"),
    ('echo "Installing FireFox"\n', 'echo "Installing Brave."\n'),
]
for old, new in replacements:
    if setup.count(old) != 1:
        sys.exit(f"setup.sh doesn't contain exactly one {old!r}")
    setup = setup.replace(old, new)
open(path, "w").write(setup)
EOF
sudo cp --no-preserve=all "$CACHE/setup.sh" "$SETUP"
