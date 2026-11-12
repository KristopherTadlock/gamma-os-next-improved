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
source "$(dirname "$0")/../scripts/modlib.sh"

if is_nano_base "$SYSTEM"; then
  installed() { sudo grep -q '^[^#][^|]*|com\.armsx2|' "$ETC/gammaos/apps.list"; }
else
  installed() { sudo grep -q '^pm install /system/etc/ARMSX2-.*\.apk$' "$SETUP"; }
fi
if ! installed || [[ ! -f "$ETC/$DRIVER_ZIP" ]]; then
  echo "ARMSX2 or $DRIVER_ZIP isn't in added-apps.txt; skipping"
  exit 0
fi

app="data/data/com.armsx2"
rm -rf "$work"
mkdir -p "$work"
# The setup lines come first: when setup.sh already has them, this mod ran on
# the image before.
if is_nano_base "$SYSTEM"; then
  setup_lines="$work/hook.sh"
  cat >"$setup_lines" <<EOF
post_armsx2() {
    step "Setting up the MaliSX2 driver in ARMSX2."
    tar -xzf /system/etc/$PACK -C / && own_app_data com.armsx2 && \\
        restorecon -R /$app/files /$app/shared_prefs
}
EOF
else
  setup_lines="$work/setup-steps.sh"
  cat >"$setup_lines" <<EOF
echo "Setting up the MaliSX2 driver in ARMSX2."
launcheruser=\$(stat -c "%U" /$app) && \\
launchergroup=\$(stat -c "%G" /$app) && \\
tar -xzf /system/etc/$PACK -C / && \\
chown -R \$launcheruser:\$launchergroup /$app/files /$app/shared_prefs && \\
restorecon -R /$app/files /$app/shared_prefs

EOF
fi
if setup_has "$SYSTEM" "$setup_lines"; then
  rm -rf "$work"
  echo "ARMSX2 already starts with MaliSX2; left as it is"
  exit 0
fi

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

install_system_file "$work/$PACK" "$ETC/$PACK"

if is_nano_base "$SYSTEM"; then
  add_setup_functions "$SYSTEM" "$setup_lines"
  apps_list "$SYSTEM" hook com.armsx2 post_armsx2
else
  sudo awk -v steps="$setup_lines" -v marker="$(setup_marker "$setup_lines")" '
    { print }
    /^pm install \/system\/etc\/ARMSX2-.*\.apk$/ { print ""; print marker; while ((getline line < steps) > 0) print line }
  ' "$SETUP" >"$work/setup.sh"
  write_through "$work/setup.sh" "$SETUP"
fi
rm -rf "$work"
echo "ARMSX2 will start with MaliSX2 ($DRIVER_ID) and the Vulkan renderer"
