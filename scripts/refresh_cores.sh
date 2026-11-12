#!/usr/bin/env bash
# Pins the RetroArch cores and assets the build takes from the libretro
# buildbot. Downloads the current nightly of every core in
# mods/retroarch-cores.txt, the core info files (info.zip) and the PPSSPP
# core's assets (PPSSPP.zip) into the build cache, and writes each file's
# sha256 and URL to mods/retroarch-cores.lock, which mods/10-update-emulators.sh
# checks every download against. A core the buildbot doesn't have is locked as
# "bundled": the base image's own copy ships.
#
# The buildbot keeps only its latest nightly of each core, so once one is
# rebuilt there, a build without these files in its cache fails until this is
# run again (or the build is run with REFRESH_CORES=1, which runs it first).
#
# Usage: refresh_cores.sh [CACHE_DIR]   (the build's cache; default work/cache)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/modlib.sh"
CACHE="$(realpath -m "${1:-$ROOT/work/cache}")/emulators/buildbot"
LOCK="$ROOT/mods/retroarch-cores.lock"
CORES_URL="https://buildbot.libretro.com/nightly/android/latest/arm64-v8a"
ASSETS=(
  "info.zip https://buildbot.libretro.com/assets/frontend/info.zip"
  "PPSSPP.zip https://buildbot.libretro.com/assets/system/PPSSPP.zip"
)

mkdir -p "$CACHE"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mapfile -t wanted < <(wanted_cores "$ROOT/mods/retroarch-cores.txt")
((${#wanted[@]})) || { echo "retroarch-cores.txt lists no cores" >&2; exit 1; }
buildbot_get "$CORES_URL/.index-extended" "$tmp/index.txt"

# Downloads URL into the cache under its sha256, which it leaves in PINNED.
pin() {
  buildbot_get "$1" "$tmp/file" || exit 1
  PINNED="$(sha256sum "$tmp/file" | cut -d' ' -f1)"
  mv "$tmp/file" "$CACHE/$PINNED"
}

lines=()
newest=""
for core in "${wanted[@]}"; do
  # Index lines are "date crc32 file".
  date="$(awk -v f="$core.zip" '$3 == f { print $1 }' "$tmp/index.txt")"
  if [[ -z "$date" ]]; then
    lines+=("$core.zip  bundled")
    echo "${core%_libretro_android.so}: not on the buildbot; the base's own core ships"
    continue
  fi
  pin "$CORES_URL/$core.zip"
  lines+=("$core.zip  $PINNED  $CORES_URL/$core.zip")
  [[ "$date" > "$newest" ]] && newest="$date"
  echo "${core%_libretro_android.so}: $date"
done
for entry in "${ASSETS[@]}"; do
  read -r name url <<<"$entry"
  pin "$url"
  lines+=("$name  $PINNED  $url")
  echo "$name: pinned"
done

{
  echo "# The RetroArch cores and assets mods/10-update-emulators.sh takes from the"
  echo "# libretro buildbot: file, sha256 (or \"bundled\": the base image's own core"
  echo "# ships) and URL. Written by scripts/refresh_cores.sh; don't edit."
  echo "# nightlies of $newest"
  printf '%s\n' "${lines[@]}"
} >"$LOCK.partial"
mv "$LOCK.partial" "$LOCK"
echo "wrote $LOCK: ${#wanted[@]} cores, nightlies up to $newest"
