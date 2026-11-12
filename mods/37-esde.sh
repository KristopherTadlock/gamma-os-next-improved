#!/usr/bin/env bash
# Adds ES-DE as the home app when the build is given ES-DE's APK
# (ESDE_APK=/path/to/ES-DE.apk). First-boot setup installs ES-DE, makes it the
# home app, and puts its settings and game lists in /sdcard/ES-DE: the ROMs
# folder, the emulator for each system from esde-emulators.txt, and the
# pack-in games. GammaOS's Magisk module for ES-DE theme downloads goes to
# /sdcard/Download for devices that need it. Runs after 15-update-apps.sh,
# which removes Daijisho, and 30-bundled-roms.sh.
#
# Usage: 37-esde.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
VERSION="3.5.0"
SYSTEMS_URL="https://gitlab.com/es-de/emulationstation-de/-/raw/50e4b600ae533d772bae3ff880d11a09b05dbe84/resources/systems/android/es_systems.xml"
SYSTEMS_SHA256="c959dbe2bf773c8b4e9e3fc19b1a59d4c92f2f13d84f3b08e9aa1f1316723869"
FIX="MAGISK_ES-DE_fix.zip"
FIX_URL="https://github.com/TheGammaSqueeze/GammaOSNext/raw/b5800a5c42f14106c9e26f2641dcd07c9cdfc378/$FIX"
FIX_SHA256="2ac5a101d28a5372dfce0f1e74016d85529b28067a2a3c6e70dd4c752e231ebd"
cache="$(realpath -m "$2")/esde"
work="$cache/work"

if [[ -z "${ESDE_APK:-}" ]]; then
  echo "ESDE_APK isn't set; skipping"
  exit 0
fi
if [[ -f "$ETC/gammaos/apps.list" ]]; then
  echo "ESDE_APK only works with the 1.2 base; 1.4.4's Nano home has its own ES-DE theme engine" >&2
  exit 1
fi

rm -rf "$work"
mkdir -p "$work/tree"
fetch() {
  [[ -f "$cache/$3" ]] || { curl -fsSL --retry 3 -o "$cache/$3.partial" "$1" && mv "$cache/$3.partial" "$cache/$3"; } || { echo "download failed: $1" >&2; exit 1; }
  echo "$2  $cache/$3" | sha256sum -c --quiet || { rm -f "$cache/$3"; echo "checksum mismatch: $cache/$3 (deleted from the cache)" >&2; exit 1; }
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

add() {
  sudo install -m 644 -o root -g root "$1" "$ETC/$2"
  sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$ETC/$2"
}
add "$ESDE_APK" es-de.apk
add "$work/esde.tar" esde.tar
add "$cache/$FIX" "$FIX"

sudo cat "$SETUP" >"$work/setup.sh"
python3 -I - "$work/setup.sh" "$pkg" "$home" "$FIX" <<'PYEOF'
import sys
path, pkg, home, fix = sys.argv[1:5]
s = open(path).read()

aurora = 'echo "Installing Aurora Store." && \\\n'
assert s.count(aurora) == 1, "setup.sh: Aurora Store install not found"
s = s.replace(aurora, f'echo "Installing ES-DE." && \\\npm install /system/etc/es-de.apk && \\\n'
                      f'appops set --uid {pkg} MANAGE_EXTERNAL_STORAGE allow\n\n' + aurora)

grants = 'echo "Granting permissions to applications."\n'
assert s.count(grants) == 1, "setup.sh: permissions line not found"
s = s.replace(grants, grants + f"cmd package set-home-activity {pkg}/{home}\n"
                               f"pm set-home-activity {pkg}/{home} -user --user 0\n")

marker = 'echo "Granting read/write permissions to RetroArch."\n'
assert s.count(marker) == 1, "setup.sh: RetroArch permissions line not found"
steps = f'''echo "Setting up ES-DE."
tar -xf /system/etc/esde.tar -C /sdcard
cp /system/etc/{fix} /sdcard/Download/
chown -R "$(stat -c %u:%g /sdcard/Download)" /sdcard/ES-DE /sdcard/Download/{fix}

'''
s = s.replace(marker, steps + marker)
open(path, "w").write(s)
PYEOF
write_through "$work/setup.sh" "$SETUP"
rm -rf "$work"
echo "ES-DE $version ($pkg) is the home app; home activity $home"
