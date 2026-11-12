#!/usr/bin/env bash
# Makes the device boot into the GammaOS Nano home (1.4.4 base only), from the
# first boot, where Nano's own setup wizard runs first-boot setup. Its home
# theme is the ES-DE theme engine with the Colorful (Simplified) theme, its
# aspect ratio picked automatically, its settings screens have the Minima theme's look on a
# Midnight background, its menu sounds are off, and its game systems are the
# ones in nano-systems.txt, with this project's default emulators. The stick
# LEDs follow the screen's colour, the performance mode is Max Performance,
# and Nano's Slide Behaviour settings are hidden (the RG Cube does not slide).
# A connected monitor is the main screen (External as Primary on, Force Mirror
# off).
# These are build.prop defaults, so choices made later in Nano, or "Boot
# Android" in the power menu, stick. Runs after 40-added-apps.sh.
#
# The theme's Gilroy and Akrobat fonts, and Nano's Akrobat, are replaced by OFL
# fonts (Urbanist, Barlow Semi Condensed), and its Quake picture is left out.
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
# OFL fonts for the ES-DE theme: Urbanist 1.330 (its metrics match Gilroy's)
# and Barlow Semi Condensed 1.422 (close to Akrobat's).
URBANIST_URL="https://github.com/coreyhu/Urbanist/releases/download/1.330/Urbanist-fonts.zip"
URBANIST_SHA256="4b808f025f020d8801bb55a7c3ea855e934527be70bf451ab4205e62bfb8fb60"
URBANIST_OFL_URL="https://raw.githubusercontent.com/coreyhu/Urbanist/cdcd01ab1efda2cdc7ae5453209de0f06ab1a7b4/OFL.txt"
URBANIST_OFL_SHA256="502f01e7ab22276fa50b68f6e4b9db36d2928e08de227484ec5f430cb0d9defa"
BARLOW_URL="https://raw.githubusercontent.com/jpt/barlow/697600fcece1685b0ba409eb82868546def9a84e"
BARLOW_FILES=(
  "fonts/ttf/BarlowSemiCondensed-Medium.ttf ef5c39de3ac1764a24c7495098672ccc034e39679ccd65d76eed752da59e11e1"
  "fonts/ttf/BarlowSemiCondensed-SemiBold.ttf 5358c566b2039988c68b4ea2af05270da055110a5d23fbd686d5a11fbecf4e03"
  "fonts/ttf/BarlowSemiCondensed-Bold.ttf bd434e949d128dd0e5fab17f60e70fcb367bff1c3e0506812c8bd734e001b356"
  "OFL.txt 186d750eb496a4c17a76385f82be6aea2ac1cf2de074a811d63786cf374ea73f"
)
GAMES_PKG="org.thimsy.androidgames"
cache="$(realpath -m "$2")/nano"
work="$cache/work"

if ! is_nano_base "$SYSTEM"; then
  echo "no GammaOS Nano in this base; skipping"
  exit 0
fi

# Both property files get a block appended below; a second run on the same
# image would add another.
if sudo grep -qE '^(persist\.(bootanim\.skip_nano|gammaos\.nano\.(ndstheme|esde\.|nav_sounds|minima\.bg)|gammaos\.performance_mode|gammargb\.control|gammaos\.ext\.(primary|force_mirror))|ro\.gammaos\.nano\.no_slide)' "$SYSTEM/build.prop"; then
  echo "build.prop already sets Nano's boot, theme, sounds, LEDs, performance mode or external display" >&2
  exit 1
fi
if sudo grep -q '^persist\.gammaos\.qs\.override_default_tiles' "$SYSTEM/product/etc/build.prop"; then
  echo "the product build.prop already sets the Quick Settings tiles" >&2
  exit 1
fi

mkdir -p "$cache/fonts"
rm -rf "$work"
mkdir -p "$work"
fetch() {
  [[ -f "$3" ]] || { curl -fsSL --retry 3 -o "$3.partial" "$1" && mv "$3.partial" "$3"; } || { echo "download failed: $1" >&2; exit 1; }
  echo "$2  $3" | sha256sum -c --quiet || { rm -f "$3"; echo "checksum mismatch: $3 (deleted from the cache)" >&2; exit 1; }
}
zip="$cache/$THEME-$THEME_COMMIT.zip"
fetch "$THEME_URL" "$THEME_SHA256" "$zip"
fetch "$URBANIST_URL" "$URBANIST_SHA256" "$cache/fonts/Urbanist-fonts-1.330.zip"
fetch "$URBANIST_OFL_URL" "$URBANIST_OFL_SHA256" "$cache/fonts/Urbanist-OFL.txt"
for entry in "${BARLOW_FILES[@]}"; do
  read -r path sha <<<"$entry"
  name="$(basename "$path")"
  [[ "$name" == OFL.txt ]] && name="Barlow-OFL.txt"
  fetch "$BARLOW_URL/$path" "$sha" "$cache/fonts/$name"
done

# Nano reads bundled theme sets from /system/etc/nano_esde_themes/<name>/. The
# README carries the license and credits.
unzip -q "$zip" -d "$work"
src="$work/$THEME-$THEME_COMMIT"
rm -f "$src/.gitignore"

# Everything shipped must be free to redistribute. Gilroy is commercial and
# Akrobat's free-font EULA forbids redistributing it, so OFL fonts replace them,
# with their licenses. quake.webp is a Quake Champions render (id Software);
# the Quake system falls back to the theme's _default.webp.
fonts="$src/_inc/fonts"
rm "$fonts"/Gilroy-{Light,Regular,SemiBold,Bold}.ttf "$fonts/Akrobat-Bold.ttf" "$src/_inc/systems/images/quake.webp"
for weight in Light Regular SemiBold Bold; do
  unzip -q -j "$cache/fonts/Urbanist-fonts-1.330.zip" "Urbanist-fonts/fonts/ttf/Urbanist-$weight.ttf" -d "$fonts"
done
cp "$cache/fonts/BarlowSemiCondensed-Bold.ttf" "$cache/fonts/Urbanist-OFL.txt" "$cache/fonts/Barlow-OFL.txt" "$fonts/"
python3 -I - "$src" <<'PYEOF'
import glob, os, sys
src = sys.argv[1]
path = os.path.join(src, "theme.xml")
s = open(path, encoding="utf-8").read()
for old, new in [("Gilroy-Light", "Urbanist-Light"), ("Gilroy-Regular", "Urbanist-Regular"),
                 ("Gilroy-SemiBold", "Urbanist-SemiBold"), ("Gilroy-Bold", "Urbanist-Bold"),
                 ("Akrobat-Bold", "BarlowSemiCondensed-Bold")]:
    old, new = f"./_inc/fonts/{old}.ttf", f"./_inc/fonts/{new}.ttf"
    assert s.count(old) == 1, f"theme.xml: {old} not found once"
    s = s.replace(old, new)
open(path, "w", encoding="utf-8").write(s)
for xml in glob.glob(os.path.join(src, "**", "*.xml"), recursive=True):
    t = open(xml, encoding="utf-8").read()
    assert "Gilroy" not in t and "Akrobat" not in t and "quake.webp" not in t, f"{xml} still names a removed file"
PYEOF
cat >>"$src/README.md" <<'EOF'

## Changes in Thimsy OS

[Thimsy OS](https://github.com/KristopherTadlock/thimsy-os) ships this theme
with these changes, under the same CC BY-NC-SA 2.0 license:

- The Gilroy font (Radomir Tinkov, commercial) is replaced by
  [Urbanist](https://github.com/coreyhu/Urbanist) 1.330 (Light, Regular,
  SemiBold, Bold), Copyright 2024 The Urbanist Project Authors, SIL Open Font
  License 1.1 (`_inc/fonts/Urbanist-OFL.txt`).
- The help bar's Akrobat Bold (Fontfabric, whose free-font license does not
  allow redistributing it) is replaced by
  [Barlow](https://github.com/jpt/barlow) Semi Condensed Bold 1.422, Copyright
  2017 The Barlow Project Authors, SIL Open Font License 1.1
  (`_inc/fonts/Barlow-OFL.txt`).
- `theme.xml` names the new font files.
- `_inc/systems/images/quake.webp`, a Quake Champions render (id Software),
  is removed; the Quake system shows the theme's default image
  (`_inc/systems/images/_default.webp`) until a replacement is made.
EOF

dest="$ETC/nano_esde_themes/$THEME"
sudo rm -rf "$dest"
sudo cp -r --no-preserve=all "$src" "$dest"
sudo chown -R root:root "$dest"
sudo find "$dest" -type d -exec chmod 755 {} + -o -type f -exec chmod 644 {} +
sudo find "$dest" -exec setfattr -n security.selinux -v u:object_r:system_file:s0 {} +
echo "bundled $THEME ($(sudo du -sh "$dest" | cut -f1))"

# Nano draws ES-DE theme text that has no fontPath in Akrobat, from these two
# paths. Barlow Semi Condensed Medium and SemiBold take their place.
xmb_fonts="$ETC/nano_xmb/fonts"
for pair in "akrobat-regular.ttf BarlowSemiCondensed-Medium.ttf" "akrobat-semibold.ttf BarlowSemiCondensed-SemiBold.ttf"; do
  read -r old new <<<"$pair"
  if sudo test -f "$xmb_fonts/$old"; then
    write_through "$cache/fonts/$new" "$xmb_fonts/$old"
    replaced_akrobat=1
  fi
done
if [[ -n "${replaced_akrobat:-}" ]]; then
  install_system_file "$cache/fonts/Barlow-OFL.txt" "$xmb_fonts/Barlow-OFL.txt"
  echo "replaced Nano's Akrobat with Barlow Semi Condensed"
fi

bash "$MODS/../scripts/build_android_games.sh" "$work/AndroidGames.apk" "$cache/android-games.keystore"
app_dir="$SYSTEM/app/AndroidGames"
sudo install -d -m 755 -o root -g root "$app_dir"
sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$app_dir"
install_system_file "$work/AndroidGames.apk" "$app_dir/AndroidGames.apk"
# The apps setup installs are emulators and tools, not Android games, except
# the added apps marked game=1.
{ grep -E '^apk .*  game=1( |$)' "$MODS/added-apps.txt" || true; } |
  sed -nE 's/.*(^| )package=([^ ]+).*/\2/p' >"$work/added-games.txt"
sudo grep -v '^#' "$ETC/gammaos/apps.list" | cut -d'|' -f2 | grep . | grep -vxFf "$work/added-games.txt" \
  >"$work/android-games-exclude.txt"
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
    step "Setting up the Nano game systems."
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
cat >>"$work/build.prop" <<EOF

# Thimsy OS: boot into Nano, with the ES-DE theme engine, Minima's
# look on Midnight for the settings, no menu sounds and no slide settings
persist.bootanim.skip_nano=0
persist.gammaos.nano.ndstheme=3
persist.gammaos.nano.esde.themeset=$THEME
persist.gammaos.nano.esde.colorscheme=colorful-dark
persist.gammaos.nano.esde.settingstheme=minima
persist.gammaos.nano.minima.bg=1a1a2e
persist.gammaos.nano.nav_sounds=0
ro.gammaos.nano.no_slide=1
# The stick LEDs on, following the screen's colour (the vendor sets the Follow
# effect, but nothing turns GammaRGB on), and Max Performance
persist.gammargb.control=on
persist.gammaos.performance_mode=max
# No global display shader until one is chosen
persist.gammaos.shader.enable=0
# A connected monitor is the main screen (the console's screen takes its size and
# its backlight goes off), not a mirror of the console's screen
persist.gammaos.ext.primary=1
persist.gammaos.ext.force_mirror=false
EOF
write_through "$work/build.prop" "$SYSTEM/build.prop"

# The RG Cube's Quick Settings tiles (its vendor build.prop), plus the Force
# Mirror and Resize to External Display tiles, for Android's shade and Nano's
# Quick Settings. The product partition's build.prop is read after the
# vendor's, so these replace its list.
sudo cat "$SYSTEM/product/etc/build.prop" >"$work/product.prop"
cat >>"$work/product.prop" <<EOF

# Thimsy OS: the RG Cube's Quick Settings tiles, with the HDMI ones
persist.gammaos.qs.override_default_tiles=1
persist.gammaos.qs.override_default_tiles_0=internet,bt,performance,fan,gammashader,gammargb,gammaeq,rotation
persist.gammaos.qs.override_default_tiles_1=deepsleepmode,externaldocking,externalmirror,externalresize,immersivemode
persist.gammaos.qs.override_default_tiles_2=abxy,dpadAnalogToggle,analogsensitivity,analogdeadzone,analogcalibration
persist.gammaos.qs.override_default_tiles_3=analogaxis,rightanalogaxis,retroarchmenubuttonoverride,mappingeditor
EOF
write_through "$work/product.prop" "$SYSTEM/product/etc/build.prop"
rm -rf "$work"
