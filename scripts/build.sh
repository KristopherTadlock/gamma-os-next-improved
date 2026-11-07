#!/usr/bin/env bash
# Builds out/RG_Cube_GammaOS_Next_Improved_v1.0.pac from the official release.
#
# Needs: curl, 7zz (or 7z), xz, unzip, e2fsck, resize2fs, setfattr, cc,
# python3, and sudo for loop-mounting the system image.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$ROOT/work"
OUT="$ROOT/out"
NAME="RG_Cube_GammaOS_Next_Full_v1.2"
# Published in the upstream release notes when the build went public.
PASSWORD="f297133b935e68072bd77309645cca86ab24bd26847e42c8080db143157a090a"
SEVENZIP="$(command -v 7zz || command -v 7z || true)"
[[ -n "$SEVENZIP" ]] || { echo "7-Zip (7zz or 7z) is required" >&2; exit 1; }

mkdir -p "$WORK/bin" "$WORK/images" "$WORK/mnt" "$OUT"
export CRC16ARC="$WORK/bin/crc16arc"
cc -O2 -o "$CRC16ARC" "$ROOT/tools/crc16arc.c"

"$ROOT/scripts/fetch.sh" "$WORK/download"
if [[ ! -f "$WORK/pac/$NAME.pac" ]]; then
  # Extract next to the final name, so an interrupted extraction is redone.
  rm -rf "$WORK/pac.partial"
  "$SEVENZIP" x -y -o"$WORK/pac.partial" -p"$PASSWORD" "$WORK/download/$NAME.7z.001" >/dev/null
  mkdir -p "$WORK/pac"
  mv "$WORK/pac.partial/$NAME.pac" "$WORK/pac/$NAME.pac"
  rm -rf "$WORK/pac.partial"
fi

echo "extracting system image"
super="$WORK/images/super_full.img"
system="$WORK/images/system_a.img"
python3 -I "$ROOT/scripts/pactool.py" extract "$WORK/pac/$NAME.pac" super_full.img "$super"
# Give the system partition the unallocated space at the end of super.
python3 -I "$ROOT/scripts/lptool.py" grow "$super" system_a max
python3 -I "$ROOT/scripts/lptool.py" extract "$super" system_a "$system"

# The image is built with deduplicated (shared) blocks, which the kernel only
# mounts read-only. Grow the filesystem to fill its partition, then unshare.
# e2fsck exits 1 when it changed the filesystem, which is expected here.
e2fsck -fy "$system" >/dev/null || (($? == 1))
resize2fs "$system" "$(($(stat -c %s "$system") / 4096))"
e2fsck -fy -E unshare_blocks "$system" >/dev/null || (($? == 1))

mnt="$WORK/mnt/system"
mkdir -p "$mnt"
sudo mount -o loop "$system" "$mnt"
trap 'sudo umount "$mnt" 2>/dev/null || true' EXIT
for mod in "$ROOT"/mods/*.sh; do
  echo "== $(basename "$mod")"
  bash "$mod" "$mnt/system" "$WORK/cache"
done
df -h "$mnt" | tail -1
sudo umount "$mnt"
trap - EXIT
e2fsck -fn "$system"

echo "writing $OUT/RG_Cube_GammaOS_Next_Improved_v1.0.pac"
pac="$OUT/RG_Cube_GammaOS_Next_Improved_v1.0.pac"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" system_a "$system"
cp "$WORK/pac/$NAME.pac" "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" super_full.img "$super"
python3 -I "$ROOT/scripts/pactool.py" fixcrc "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" setinfo "$pac.partial" Improved_v1.0 RG_Cube_GammaOS_Next_Improved
mv "$pac.partial" "$pac"
rm -f "$super" "$system"
