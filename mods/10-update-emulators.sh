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
# buildbot's current nightlies, the same files RetroArch's online updater uses.
CORES_URL="https://buildbot.libretro.com/nightly/android/latest/arm64-v8a"
INFO_URL="https://buildbot.libretro.com/assets/frontend/info.zip"
PPSSPP_ASSETS_URL="https://buildbot.libretro.com/assets/system/PPSSPP.zip"
RA_DATA="data/data/com.retroarch.aarch64"

mkdir -p "$CACHE"

fetch() {
  local url="$1" out="$2"
  [[ -f "$out" ]] || { curl -fsSL --retry 3 -o "$out.partial" "$url" && mv "$out.partial" "$out"; }
}

for entry in "${APKS[@]}"; do
  IFS='|' read -r name url sha file old <<<"$entry"
  fetch "$url" "$CACHE/$file"
  echo "$sha  $CACHE/$file" | sha256sum -c --quiet
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
  # Writing through the existing file keeps its owner, mode, and SELinux label.
  sudo cp --no-preserve=all "$CACHE/$file" "$ETC/$file"
  echo "updated $name"
done

work="$CACHE/retroarch"
rm -rf "$work"
mkdir -p "$work/tree" "$work/cores"
pack="$(payload_path "$ETC" retroarch)"
payload_to_tar "$pack" "$work/in.tar"
# setup.sh chowns these files after extracting them, so ownership is not kept.
tar -xpf "$work/in.tar" --no-same-owner -C "$work/tree"

fetch "$CORES_URL/.index-extended" "$work/index.txt"
cores_dir="$work/tree/$RA_DATA/cores"
# Lines are "core" or "D - core", where D marks the platform's default core.
mapfile -t wanted < <(sed 's/#.*//' "$MODS/retroarch-cores.txt" |
  awk 'NF { core = ($1 == "D" && $2 == "-") ? $3 : $1; print core "_libretro_android.so" }')
for so in "$cores_dir"/*_libretro_android.so; do
  if ! printf '%s\n' "${wanted[@]}" | grep -qxF "$(basename "$so")"; then
    rm "$so"
  fi
done

updated=0
for core in "${wanted[@]}"; do
  so="$cores_dir/$core"
  if ! grep -q " $core.zip$" "$work/index.txt"; then
    [[ -f "$so" ]] || { echo "unknown core ${core%_libretro_android.so}" >&2; exit 1; }
    echo "kept ${core%_libretro_android.so} (not on buildbot)"
    continue
  fi
  # The buildbot briefly 404s while it replaces a nightly, so retry those too.
  for attempt in 1 2 3 4 5; do
    curl -fsSL --retry 3 -o "$work/cores/$core.zip" "$CORES_URL/$core.zip" && break
    rm -f "$work/cores/$core.zip"
    sleep $((attempt * 10))
  done
  if [[ ! -f "$work/cores/$core.zip" ]]; then
    [[ -f "$so" ]] || { echo "failed to download $core" >&2; exit 1; }
    echo "WARNING: kept bundled ${core%_libretro_android.so} (download failed)" >&2
    continue
  fi
  unzip -p "$work/cores/$core.zip" "$core" >"$work/cores/$core"
  install -m 600 "$work/cores/$core" "$so"
  updated=$((updated + 1))
done
echo "shipping ${#wanted[@]} RetroArch cores, $updated updated to $(awk '{print $1}' "$work/index.txt" | sort | tail -1) nightlies"

curl -fsSL --retry 3 -o "$work/info.zip" "$INFO_URL"
unzip -oq "$work/info.zip" -d "$work/tree/$RA_DATA/info"
# The PPSSPP core's assets are only kept, refreshed, when that core ships.
rm -rf "$work/tree/sdcard/RetroArch/system/PPSSPP"
if [[ -f "$cores_dir/ppsspp_libretro_android.so" ]]; then
  curl -fsSL --retry 3 -o "$work/ppsspp.zip" "$PPSSPP_ASSETS_URL"
  unzip -oq "$work/ppsspp.zip" -d "$work/tree/sdcard/RetroArch/system"
  echo "updated RetroArch core info and PPSSPP core assets"
else
  echo "updated RetroArch core info; removed the PPSSPP core's assets (core not shipped)"
fi

# RetroArch's copies of the 4:3 and 8:7 Cropped shaders, which skip RetroArch.
install -m 600 -t "$work/tree/$RA_DATA/shaders/shaders_slang" "$MODS"/retroarch-shaders/rg_cube_{4_3,8_7}_cropped.slang{,p}
echo "added the rg_cube_4_3_cropped and rg_cube_8_7_cropped shaders"

# Repack the same top-level paths as the original, so no entries for data/ or
# sdcard/ themselves are added; root would apply their owner and mode on extract.
tar -tf "$work/in.tar" | python3 -I "$MODS/../scripts/tar_roots.py" >"$work/roots"
(cd "$work/tree" && tar -cpf "$work/out.tar" --owner=u0_a188:10188 --group=u0_a188:10188 -T "$work/roots")
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
echo "${pack##*/}: $(stat -c %s "$pack") -> $(stat -c %s "$work/${pack##*/}") bytes"
write_through "$work/${pack##*/}" "$pack"
rm -rf "$work"
