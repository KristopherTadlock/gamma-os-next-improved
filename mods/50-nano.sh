#!/usr/bin/env bash
# Sets up the GammaOS Nano home (1.4.4 base only): its ES-DE theme engine as
# the home theme, with the Colorful (Simplified) theme in its square layout, and
# the game systems in nano-systems.txt with this project's default emulators.
# First-boot setup applies them, without overriding anything already chosen.
# The device still boots normal Android until "Boot Nano" is picked from the
# power menu, like GammaOS. Runs after 40-added-apps.sh.
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

python3 -I "$MODS/../scripts/nano_systems.py" "$MODS/nano-systems.txt" "$ETC/nano_xmb/daijishou" \
  "$ETC/nano_xmb/icons_retroarch" "$ETC/gammaos/apps.list" "$MODS/retroarch-cores.txt" "$work/nano_systems.json"
install_system_file "$work/nano_systems.json" "$ETC/gammaos/nano_systems.json"

cat >"$work/setup-steps.sh" <<EOF
step "Setting up the GammaOS Nano home."
if [ ! -f /data/system/nano_systems.json ]; then
    cp /system/etc/gammaos/nano_systems.json /data/system/nano_systems.json
    chown root:root /data/system/nano_systems.json
    chmod 644 /data/system/nano_systems.json
    restorecon /data/system/nano_systems.json
fi
nano_default() {
    [ -n "\$(getprop "\$1")" ] || setprop "\$1" "\$2"
}
nano_default persist.gammaos.nano.ndstheme 3
nano_default persist.gammaos.nano.esde.themeset $THEME
nano_default persist.gammaos.nano.esde.aspectratio 1:1
nano_default persist.gammaos.nano.esde.colorscheme colorful

EOF
add_setup_steps "$SYSTEM" "$work/setup-steps.sh"
rm -rf "$work"
