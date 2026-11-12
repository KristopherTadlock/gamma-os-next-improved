#!/usr/bin/env bash
# Updates the emulators GammaOS Next installs on first boot from /system/etc.
#
# Usage: 10-update-emulators.sh SYSTEM_DIR CACHE_DIR
#   SYSTEM_DIR  the mounted image's /system directory
#   CACHE_DIR   where downloads are kept between builds
set -euo pipefail

SYSTEM="$1"
CACHE="$(realpath -m "$2")/emulators"
ETC="$SYSTEM/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"

# name|url|sha256|file name in /system/etc|file it replaces (a pattern)
APKS=(
  "RetroArch 1.22.2|https://buildbot.libretro.com/stable/1.22.2/android/RetroArch_aarch64.apk|7bd5d208dfe93cc8e2ea6c04608948ce1a045980f160a58ca2d0993aa20ad213|RetroArch_aarch64.apk|RetroArch_aarch64.apk"
  "Flycast 2.7|https://github.com/flyinghead/flycast/releases/download/v2.7/flycast-2.7.apk|1ac54d77b8283f6d58d077396d0916ec12c15b7e4ada9d957836a64590756c00|flycast-release.apk|flycast-release.apk"
  "PPSSPP 1.20.4|https://www.ppsspp.org/files/1_20_4/ppsspp.apk|dd702a31270ecd68db37afee311d6efe08b1d4da9601d2750ce1a73bbd316c3f|ppsspp_1.20.4.apk|ppsspp_*.apk"
)

# RetroArch cores and their assets have no stable releases; this takes the
# buildbot's nightlies, the same files RetroArch's online updater uses, pinned
# in retroarch-cores.lock by scripts/refresh_cores.sh. REFRESH_CORES=1 runs it
# first, to take the current ones; ALLOW_BUNDLED_CORES=1 lets a core that can't
# be downloaded as locked keep the base image's copy.
LOCK="$MODS/retroarch-cores.lock"
RA_DATA="data/data/com.retroarch.aarch64"
# C-BIOS (2-clause BSD), the free MSX BIOS: blueMSX runs MSX cartridge games
# with it when no MSX BIOS was installed. Disk games still need the real one.
CBIOS_URL="https://downloads.sourceforge.net/project/cbios/cbios/0.29/cbios-0.29a.zip"
CBIOS_SHA256="c585a2f4a10b397e460a95043d59ac79cf4442fbb45a6e14a1e27ef9febaed73"

mkdir -p "$CACHE"

fetch() {
  local url="$1" out="$2"
  [[ -f "$out" ]] || { curl -fsSL --retry 3 -o "$out.partial" "$url" && mv "$out.partial" "$out"; } || { echo "download failed: $url" >&2; exit 1; }
}

for entry in "${APKS[@]}"; do
  IFS='|' read -r name url sha file old <<<"$entry"
  fetch "$url" "$CACHE/$file"
  echo "$sha  $CACHE/$file" | sha256sum -c --quiet || { rm -f "$CACHE/$file"; echo "checksum mismatch: $CACHE/$file (deleted from the cache)" >&2; exit 1; }
  matches=("$ETC"/$old)
  [[ ${#matches[@]} == 1 && -f "${matches[0]}" ]] || { echo "no single $old in $ETC" >&2; exit 1; }
  old="${matches[0]##*/}"
  if [[ "$file" != "$old" ]]; then
    sudo mv "$ETC/$old" "$ETC/$file"
    if is_nano_base "$SYSTEM"; then
      apps_list "$SYSTEM" source "/system/etc/$old" "/system/etc/$file"
    else
      sudo sed "s#/system/etc/$old#/system/etc/$file#g" "$SYSTEM/bin/setup.sh" >"$CACHE/setup.sh"
      write_through "$CACHE/setup.sh" "$SYSTEM/bin/setup.sh"
    fi
  fi
  write_through "$CACHE/$file" "$ETC/$file"
  echo "updated $name"
done

work="$CACHE/retroarch"
rm -rf "$work"
mkdir -p "$work/tree" "$work/cores"
pack="$(payload_path "$ETC" retroarch)"
payload_to_tar "$pack" "$work/in.tar"
# setup.sh chowns these files after extracting them, so ownership is not kept.
tar -xpf "$work/in.tar" --no-same-owner -C "$work/tree"

if [[ "${REFRESH_CORES:-}" == 1 ]]; then
  bash "$MODS/../scripts/refresh_cores.sh" "$2"
fi
[[ -f "$LOCK" ]] || {
  echo "no $LOCK: run scripts/refresh_cores.sh (or the build with REFRESH_CORES=1) to pin the buildbot's current cores" >&2
  exit 1
}
cores_dir="$work/tree/$RA_DATA/cores"
mapfile -t wanted < <(wanted_cores "$MODS/retroarch-cores.txt")
((${#wanted[@]})) || { echo "retroarch-cores.txt lists no cores" >&2; exit 1; }
if ! diff <(printf '%s.zip\n' "${wanted[@]}" | sort) \
    <(awk '!/^#/ && $1 ~ /_libretro_android\.so\.zip$/ { print $1 }' "$LOCK" | sort) >/dev/null; then
  echo "retroarch-cores.txt and $LOCK list different cores: run scripts/refresh_cores.sh" >&2
  exit 1
fi
for so in "$cores_dir"/*_libretro_android.so; do
  if ! printf '%s\n' "${wanted[@]}" | grep -qxF "$(basename "$so")"; then
    rm "$so"
  fi
done

# Copies the locked file NAME to OUT: from the cache, or downloaded and checked
# against the lock. Fails when it can't.
fetch_locked() {
  local name="$1" out="$2" sha url cached
  read -r sha url < <(awk -v f="$name" '$1 == f { print $2, $3 }' "$LOCK") || true
  [[ -n "$url" ]] || { echo "$name has no download in $LOCK" >&2; return 1; }
  cached="$CACHE/buildbot/$sha"
  if ! echo "$sha  $cached" | sha256sum -c --quiet 2>/dev/null; then
    mkdir -p "$CACHE/buildbot"
    buildbot_get "$url" "$cached.download" || return 1
    if ! echo "$sha  $cached.download" | sha256sum -c --quiet; then
      rm -f "$cached.download"
      echo "$url is no longer the file in $LOCK: run scripts/refresh_cores.sh" >&2
      return 1
    fi
    mv "$cached.download" "$cached"
  fi
  cp "$cached" "$out"
}

updated=0
for core in "${wanted[@]}"; do
  so="$cores_dir/$core"
  if [[ "$(awk -v f="$core.zip" '$1 == f { print $2 }' "$LOCK")" == bundled ]]; then
    [[ -f "$so" ]] || { echo "unknown core ${core%_libretro_android.so}" >&2; exit 1; }
    echo "kept ${core%_libretro_android.so} (not on buildbot)"
    continue
  fi
  if ! fetch_locked "$core.zip" "$work/cores/$core.zip"; then
    if [[ "${ALLOW_BUNDLED_CORES:-}" != 1 || ! -f "$so" ]]; then
      echo "failed to get ${core%_libretro_android.so} (ALLOW_BUNDLED_CORES=1 keeps the base's copy)" >&2
      exit 1
    fi
    echo "WARNING: kept bundled ${core%_libretro_android.so} (download failed)" >&2
    continue
  fi
  unzip -p "$work/cores/$core.zip" "$core" >"$work/cores/$core"
  install -m 600 "$work/cores/$core" "$so"
  updated=$((updated + 1))
done
echo "shipping ${#wanted[@]} RetroArch cores, $updated updated to the $(sed -n 's/^# nightlies of //p' "$LOCK") nightlies"

fetch_locked info.zip "$work/info.zip"
unzip -oq "$work/info.zip" -d "$work/tree/$RA_DATA/info"
# The PPSSPP core's assets are only kept, refreshed, when that core ships.
rm -rf "$work/tree/sdcard/RetroArch/system/PPSSPP"
if [[ -f "$cores_dir/ppsspp_libretro_android.so" ]]; then
  fetch_locked PPSSPP.zip "$work/ppsspp.zip"
  unzip -oq "$work/ppsspp.zip" -d "$work/tree/sdcard/RetroArch/system"
  echo "updated RetroArch core info and PPSSPP core assets"
else
  echo "updated RetroArch core info; removed the PPSSPP core's assets (core not shipped)"
fi

# blueMSX's Auto machine falls back to these three when there is no "MSX2+"
# machine, for every game but a disk. Each folder gets its ROMs, laid out as the
# release's configs/blueMSX/README.txt says, and the license.
fetch "$CBIOS_URL" "$CACHE/cbios-0.29a.zip"
echo "$CBIOS_SHA256  $CACHE/cbios-0.29a.zip" | sha256sum -c --quiet ||
  { rm -f "$CACHE/cbios-0.29a.zip"; echo "C-BIOS download failed its checksum; run the build again" >&2; exit 1; }
rm -rf "$work/cbios"
unzip -q "$CACHE/cbios-0.29a.zip" -d "$work/cbios"
cb="$work/cbios/cbios-0.29a"
machines="$work/tree/sdcard/RetroArch/system/Machines"
for m in "MSX:msx1" "MSX2:msx2" "MSX2+:msx2+"; do
  dir="$machines/${m%%:*} - C-BIOS" gen="${m#*:}"
  mkdir -p "$dir"
  install -m 644 "$cb/configs/blueMSX/${m%%:*} - C-BIOS/config.ini" "$cb/roms/cbios_main_$gen.rom" \
    "$cb/roms/cbios_logo_$gen.rom" "$dir/"
  [[ "$gen" == msx1 ]] || install -m 644 "$cb/roms/cbios_sub.rom" "$dir/"
  [[ "$gen" != msx2+ ]] || install -m 644 "$cb/roms/cbios_music.rom" "$dir/"
  install -m 644 "$cb/doc/cbios.txt" "$dir/C-BIOS license.txt"
done
rm -rf "$work/cbios"
echo "added C-BIOS 0.29a for blueMSX"

# The built-in controller reports its face buttons by position
# (27-controller-layout.sh), so its RetroArch profile and player 1 binds, set
# for the buttons' print, swap A/B and X/Y back to the RetroPad's positions.
python3 -I - "$work/tree/$RA_DATA/autoconfig/android/retrogame_joypad.cfg" \
  "$work/tree/sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg" <<'PY'
import re, sys
# Android key codes: BUTTON_A 96, BUTTON_B 97, BUTTON_X 99, BUTTON_Y 100.
by_print = {"a": "96", "b": "97", "x": "99", "y": "100"}
by_position = {"a": "97", "b": "96", "x": "100", "y": "99"}
for path, prefix in zip(sys.argv[1:], ("input_", "input_player1_")):
    text = open(path).read()
    pattern = r'^(%s([abxy])_btn = ")(\d+)(")' % prefix
    binds = {m.group(2): m.group(3) for m in re.finditer(pattern, text, flags=re.M)}
    if len(re.findall(pattern, text, flags=re.M)) != 4 or len(binds) != 4:
        sys.exit("%s: %d face-button binds, expected 4" % (path, len(binds)))
    if binds == by_position:
        print("%s: face buttons already by position" % path)
        continue
    if binds != by_print:
        sys.exit("%s: unexpected face-button binds %s" % (path, binds))
    text = re.sub(pattern, lambda m: m.group(1) + by_position[m.group(2)] + m.group(4), text, flags=re.M)
    open(path, "w").write(text)
PY

# RetroArch's copies of the 4:3 and 8:7 Cropped shaders.
install -m 600 -t "$work/tree/$RA_DATA/shaders/shaders_slang" "$MODS"/retroarch-shaders/rg_cube_{4_3,8_7}_cropped.slang{,p}
echo "added the rg_cube_4_3_cropped and rg_cube_8_7_cropped shaders"

# Repack the same top-level paths as the original, so no entries for data/ or
# sdcard/ themselves are added; root would apply their owner and mode on extract.
tar -tf "$work/in.tar" | python3 -I "$MODS/../scripts/tar_roots.py" >"$work/roots"
# C-BIOS is new to the payload: its folder is added if no path above takes it in.
c="sdcard/RetroArch/system/Machines"
while [[ "$c" == */* ]] && ! grep -qxF "$c" "$work/roots"; do c="${c%/*}"; done
grep -qxF "$c" "$work/roots" || grep -q '^sdcard/RetroArch/system/Machines/' "$work/roots" ||
  echo "sdcard/RetroArch/system/Machines" >>"$work/roots"
(cd "$work/tree" && tar -cpf "$work/out.tar" --owner=u0_a188:10188 --group=u0_a188:10188 -T "$work/roots")
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
echo "${pack##*/}: $(stat -c %s "$pack") -> $(stat -c %s "$work/${pack##*/}") bytes"
write_through "$work/${pack##*/}" "$pack"
rm -rf "$work"
