#!/usr/bin/env bash
# Replaces Daijisho with ES-DE when the build is given ES-DE's APK
# (ESDE_APK=/path/to/ES-DE.apk). First-boot setup installs ES-DE, makes it the
# home app, and puts its settings and game lists in /sdcard/ES-DE: the ROMs
# folder, the emulator for each system from esde-emulators.txt, and the
# pack-in games. Beneath a Steel Sky's Daijisho template becomes the .scummvm
# file ES-DE launches. GammaOS's Magisk module for ES-DE theme downloads goes
# to /sdcard/Download for devices that need it. Runs after 30-bundled-roms.sh.
#
# Usage: 37-esde.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
MODS="$(cd "$(dirname "$0")" && pwd)"
VERSION="3.5.0"
SYSTEMS_URL="https://gitlab.com/es-de/emulationstation-de/-/raw/50e4b600ae533d772bae3ff880d11a09b05dbe84/resources/systems/android/es_systems.xml"
SYSTEMS_SHA256="c959dbe2bf773c8b4e9e3fc19b1a59d4c92f2f13d84f3b08e9aa1f1316723869"
FIX="MAGISK_ES-DE_fix.zip"
FIX_URL="https://github.com/TheGammaSqueeze/GammaOSNext/raw/b5800a5c42f14106c9e26f2641dcd07c9cdfc378/$FIX"
FIX_SHA256="2ac5a101d28a5372dfce0f1e74016d85529b28067a2a3c6e70dd4c752e231ebd"
cache="$(realpath -m "$2")/esde"
work="$cache/work"

if [[ -z "${ESDE_APK:-}" ]]; then
  echo "ESDE_APK isn't set; keeping Daijisho"
  exit 0
fi

rm -rf "$work"
mkdir -p "$work/tree"
fetch() {
  [[ -f "$cache/$3" ]] || { curl -fsSL --retry 3 -o "$cache/$3.partial" "$1" && mv "$cache/$3.partial" "$cache/$3"; }
  echo "$2  $cache/$3" | sha256sum -c --quiet
}
fetch "$SYSTEMS_URL" "$SYSTEMS_SHA256" es_systems.xml
fetch "$FIX_URL" "$FIX_SHA256" "$FIX"

read -r pkg version home < <(python3 -I "$MODS/../scripts/apk_home.py" "$ESDE_APK")
case "$pkg" in
  org.es_de.frontend | org.es_de.frontend.galaxy) ;;
  *) echo "$ESDE_APK isn't ES-DE (package $pkg)" >&2; exit 1 ;;
esac
if [[ "$version" != "$VERSION" ]]; then
  echo "$ESDE_APK is ES-DE $version; update VERSION and SYSTEMS_URL in $0 to match" >&2
  exit 1
fi

python3 -I "$MODS/../scripts/esde_config.py" "$cache/es_systems.xml" "$MODS/esde-emulators.txt" \
  "$MODS/retroarch-cores.txt" "$ETC/roms.tar.xz" "$work/tree"
(cd "$work/tree" && tar -cf "$work/esde.tar" --owner=0 --group=0 -- *)

sudo rm -f "$ETC"/daijisho*.apk "$ETC/daijisho.tar.xz" "$ETC/daijisho-wallpapers.tar"
add() {
  sudo install -m 644 -o root -g root "$1" "$ETC/$2"
  sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$ETC/$2"
}
add "$ESDE_APK" es-de.apk
add "$work/esde.tar" esde.tar
add "$cache/$FIX" "$FIX"

sudo cat "$SETUP" >"$work/setup.sh"
python3 -I - "$work/setup.sh" "$pkg" "$home" "$FIX" <<'PYEOF'
import re, sys
path, pkg, home, fix = sys.argv[1:5]
s = open(path).read()

daijisho = re.compile(r'echo "Installing Daijisho\." && \\\npm install /system/etc/daijisho[^\n]*\.apk\n'
                      r'cd /\n(?:[^\n]*com\.magneticchen\.daijishou[^\n]*\n|tar -xJvf /system/etc/daijisho\.tar\.xz[^\n]*\n)+')
s, n = daijisho.subn(f'echo "Installing ES-DE." && \\\npm install /system/etc/es-de.apk && \\\n'
                     f'appops set --uid {pkg} MANAGE_EXTERNAL_STORAGE allow\ncd /\n', s)
assert n == 1, "setup.sh: Daijisho install block not found"

home_lines = ("cmd package set-home-activity com.magneticchen.daijishou/.app.HomeActivity\n"
              "pm set-home-activity com.magneticchen.daijishou/.app.HomeActivity -user --user 0\n")
assert s.count(home_lines) == 1, "setup.sh: Daijisho home lines not found"
s = s.replace(home_lines, f"cmd package set-home-activity {pkg}/{home}\n"
                          f"pm set-home-activity {pkg}/{home} -user --user 0\n")

marker = 'echo "Granting read/write permissions to RetroArch."\n'
assert s.count(marker) == 1, "setup.sh: RetroArch permissions line not found"
steps = f'''echo "Setting up ES-DE."
tar -xf /system/etc/esde.tar -C /sdcard
for f in /sdcard/ROMs/scummvm/*.dpt; do
  [ -f "$f" ] || continue
  rm "$f"
  [ -f "$f.license.txt" ] && mv "$f.license.txt" "${{f%.dpt}}.scummvm.license.txt"
done
cp /system/etc/{fix} /sdcard/Download/
chown -R "$(stat -c %u:%g /sdcard/Download)" /sdcard/ES-DE /sdcard/ROMs/scummvm /sdcard/Download/{fix}

'''
s = s.replace(marker, steps + marker)
assert "daijisho" not in s.lower(), "setup.sh still mentions Daijisho"
open(path, "w").write(s)
PYEOF
# Writing through the existing file keeps its owner, mode, and SELinux label.
sudo cp --no-preserve=all "$work/setup.sh" "$SETUP"
rm -rf "$work"
echo "ES-DE $version ($pkg) replaces Daijisho; home activity $home"
