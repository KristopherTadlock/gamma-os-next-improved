#!/usr/bin/env bash
# Makes the device boot into the GammaOS Nano home (1.4.4 base only), from the
# first boot, where Nano's own setup wizard runs first-boot setup. Its home
# theme is the ES-DE theme engine with the Colorful (Simplified) theme in its
# square layout, its settings screens have the Minima theme's look, its menu
# sounds are off, and its game systems are the ones in nano-systems.txt, with
# this project's default emulators. These are build.prop defaults, so choices
# made later in Nano, or "Boot Android" in the power menu, stick. Runs after
# 40-added-apps.sh.
#
# It also builds in the Android Games helper (tools/android-games), which lists
# the installed Android games as the Android Games system and starts them;
# building it needs the tools scripts/build_android_games.sh lists.
#
# Usage: 50-nano.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
# Colorful (Simplified) by ant, from Colorful by viking and faeran. CC BY-NC-SA 2.0.
THEME="colorful-simplified-es-de"
THEME_COMMIT="669e7f6a2de57c5ab69fd865e70d1cd357dbabbb"
THEME_URL="https://github.com/anthonycaccese/$THEME/archive/$THEME_COMMIT.zip"
THEME_SHA256="b36570e97d9233343872d8079c99bddc4116283c274785d086ca63be6eee3211"
GAMES_PKG="org.gammaosnextimproved.androidgames"
cache="$(realpath -m "$2")/nano"
work="$cache/work"

if ! is_nano_base "$SYSTEM"; then
  echo "no GammaOS Nano in this base; skipping"
  exit 0
fi

mkdir -p "$cache"
rm -rf "$work"
mkdir -p "$work"
zip="$cache/$THEME-$THEME_COMMIT.zip"
[[ -f "$zip" ]] || { curl -fsSL --retry 3 -o "$zip.partial" "$THEME_URL" && mv "$zip.partial" "$zip"; }
echo "$THEME_SHA256  $zip" | sha256sum -c --quiet

# Nano reads bundled theme sets from /system/etc/nano_esde_themes/<name>/. The
# README carries the license and credits.
unzip -q "$zip" -d "$work"
src="$work/$THEME-$THEME_COMMIT"
rm -f "$src/.gitignore"
dest="$ETC/nano_esde_themes/$THEME"
sudo rm -rf "$dest"
sudo cp -r --no-preserve=all "$src" "$dest"
sudo chown -R root:root "$dest"
sudo find "$dest" -type d -exec chmod 755 {} + -o -type f -exec chmod 644 {} +
sudo find "$dest" -exec setfattr -n security.selinux -v u:object_r:system_file:s0 {} +
echo "bundled $THEME ($(sudo du -sh "$dest" | cut -f1))"

bash "$MODS/../scripts/build_android_games.sh" "$work/AndroidGames.apk" "$cache/android-games.keystore"
app_dir="$SYSTEM/app/AndroidGames"
sudo install -d -m 755 -o root -g root "$app_dir"
sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$app_dir"
install_system_file "$work/AndroidGames.apk" "$app_dir/AndroidGames.apk"
# The apps setup installs are emulators and tools, not Android games.
sudo grep -v '^#' "$ETC/gammaos/apps.list" | cut -d'|' -f2 | grep . >"$work/android-games-exclude.txt"
install_system_file "$work/android-games-exclude.txt" "$ETC/gammaos/android-games-exclude.txt"

python3 -I "$MODS/../scripts/nano_systems.py" "$MODS/nano-systems.txt" "$ETC/nano_xmb/daijishou" \
  "$ETC/nano_xmb/icons_retroarch" "$ETC/gammaos/apps.list" "$MODS/retroarch-cores.txt" "$work/nano_systems.json" \
  "$GAMES_PKG"
install_system_file "$work/nano_systems.json" "$ETC/gammaos/nano_systems.json"

# Nano's setup wizard starts Nano before setup.sh runs, and Nano writes its
# built-in systems on its first start, so a fresh setup replaces them; Nano
# reloads the file when it changes.
cat >"$work/setup-steps.sh" <<EOF
if [ "\$FRESH_SETUP" = 1 ]; then
    step "Setting up the GammaOS Nano game systems."
    cp /system/etc/gammaos/nano_systems.json /data/system/nano_systems.json
    chown root:root /data/system/nano_systems.json
    chmod 644 /data/system/nano_systems.json
    restorecon /data/system/nano_systems.json
fi
step "Listing the installed Android games."
run pm grant $GAMES_PKG android.permission.READ_EXTERNAL_STORAGE
run pm grant $GAMES_PKG android.permission.WRITE_EXTERNAL_STORAGE
run am broadcast -a $GAMES_PKG.SYNC -n $GAMES_PKG/.SyncReceiver

EOF
add_setup_steps "$SYSTEM" "$work/setup-steps.sh"

sudo cat "$SYSTEM/build.prop" >"$work/build.prop"
if grep -qE '^persist\.(bootanim\.skip_nano|gammaos\.nano\.(ndstheme|esde\.|nav_sounds))' "$work/build.prop"; then
  echo "build.prop already sets Nano's boot, theme or sounds" >&2
  exit 1
fi
cat >>"$work/build.prop" <<EOF

# gamma-os-next-improved: boot into Nano, with the ES-DE theme engine, Minima's
# look for the settings and no menu sounds
persist.bootanim.skip_nano=0
persist.gammaos.nano.ndstheme=3
persist.gammaos.nano.esde.themeset=$THEME
persist.gammaos.nano.esde.aspectratio=1:1
persist.gammaos.nano.esde.colorscheme=colorful-dark
persist.gammaos.nano.esde.settingstheme=minima
persist.gammaos.nano.nav_sounds=0
EOF
write_through "$work/build.prop" "$SYSTEM/build.prop"

# The RG Cube's Quick Settings tiles (its vendor build.prop), plus the Force
# Mirror and Resize to External Display tiles, for Android's shade and Nano's
# Quick Settings. The product partition's build.prop is read after the
# vendor's, so these replace its list.
sudo cat "$SYSTEM/product/etc/build.prop" >"$work/product.prop"
cat >>"$work/product.prop" <<EOF

# gamma-os-next-improved: the RG Cube's Quick Settings tiles, with the HDMI ones
persist.gammaos.qs.override_default_tiles=1
persist.gammaos.qs.override_default_tiles_0=internet,bt,performance,fan,gammashader,gammargb,gammaeq,rotation
persist.gammaos.qs.override_default_tiles_1=deepsleepmode,externaldocking,externalmirror,externalresize,immersivemode
persist.gammaos.qs.override_default_tiles_2=abxy,dpadAnalogToggle,analogsensitivity,analogdeadzone,analogcalibration
persist.gammaos.qs.override_default_tiles_3=analogaxis,rightanalogaxis,retroarchmenubuttonoverride,mappingeditor
EOF
write_through "$work/product.prop" "$SYSTEM/product/etc/build.prop"
rm -rf "$work"
