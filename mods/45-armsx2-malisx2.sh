#!/usr/bin/env bash
# Pre-installs the MaliSX2 driver in ARMSX2 and selects it with the Vulkan
# renderer, so it's active on first launch. Runs after 40-added-apps.sh.
#
# Dolphin and hakuX only load custom drivers when /dev/kgsl-3d0 (an Adreno
# GPU) exists, so they can't use MaliSX2 on this device.
#
# Usage: 45-armsx2-malisx2.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
DRIVER_ZIP="malisx2-0.4.0.zip"
# The folder name ARMSX2 gives this release when it downloads it itself.
DRIVER_ID="armsx2libmali-v0.4.0-malisx2-0.4.0-63e307e63a"
PACK="armsx2-malisx2.tar.gz"
work="$(realpath -m "$2")/armsx2-malisx2"

app="data/data/com.armsx2"
rm -rf "$work"
mkdir -p "$work/tree/$app/files/drivers/$DRIVER_ID/cache" "$work/tree/$app/shared_prefs"
unzip -q "$ETC/$DRIVER_ZIP" meta.json libvulkan_malisx2.so -d "$work/tree/$app/files/drivers/$DRIVER_ID"
# ARMSX2 copies these plain preferences into its settings on first launch.
cat >"$work/tree/$app/shared_prefs/ARMSX2.xml" <<EOF
<?xml version='1.0' encoding='utf-8' standalone='yes' ?>
<map>
    <string name="customDriverId">$DRIVER_ID</string>
    <string name="renderer">vulkan</string>
</map>
EOF
find "$work/tree" -type d -exec chmod 771 {} +
find "$work/tree" -type f -exec chmod 660 {} +
# Only the app's own folders: entries for data/ or data/data/ would reset
# those directories' owner and mode when extracted.
(cd "$work/tree" && tar -czf "$work/$PACK" --owner=0 --group=0 "$app/files" "$app/shared_prefs")

sudo install -m 644 -o root -g root "$work/$PACK" "$ETC/$PACK"
sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$ETC/$PACK"

steps="$work/setup-steps.sh"
cat >"$steps" <<EOF
echo "Setting up the MaliSX2 driver in ARMSX2."
launcheruser=\$(stat -c "%U" /$app) && \\
launchergroup=\$(stat -c "%G" /$app) && \\
tar -xzf /system/etc/$PACK -C / && \\
chown -R \$launcheruser:\$launchergroup /$app/files /$app/shared_prefs && \\
restorecon -R /$app/files /$app/shared_prefs

EOF
sudo grep -q '^pm install /system/etc/ARMSX2-.*\.apk$' "$SETUP" || { echo "setup.sh doesn't install ARMSX2" >&2; exit 1; }
sudo awk -v steps="$steps" '
  { print }
  /^pm install \/system\/etc\/ARMSX2-.*\.apk$/ { print ""; while ((getline line < steps) > 0) print line }
' "$SETUP" >"$work/setup.sh"
# Writing through the existing file keeps its owner, mode, and SELinux label.
sudo cp --no-preserve=all "$work/setup.sh" "$SETUP"
rm -rf "$work"
echo "ARMSX2 will start with MaliSX2 ($DRIVER_ID) and the Vulkan renderer"
