#!/usr/bin/env bash
# Builds out/RG_Cube_GammaOS_Next_Improved_v1.0.pac from the official release.
#
# Needs: curl, 7zz (or 7z), xz, zstd, unzip, e2fsck, resize2fs, setfattr, cc,
# python3, and sudo for loop-mounting the system image.
#
# Set SYSTEM_IMG=/path/to/system.img to use a GammaOS Next 1.4.4 system image
# built from source instead of the release's 1.2 system (the RG Cube's vendor,
# kernel and bootloader stay). That builds ..._Improved_v2.0.pac.
#
# Set ESDE_APK=/path/to/ES-DE.apk to replace Daijisho with ES-DE
# (mods/37-esde.sh, 1.2 base only); that also needs aapt.
#
# The vendor kernel's DisplayPort driver is patched (scripts/sprd_drm.py) in
# vendor_dlkm (scripts/patch_vendor_dlkm.py) and vendor_boot
# (scripts/patch_vendor_boot.py). That needs fsck.erofs, mkfs.erofs,
# unpack_bootimg, lz4 and avbtool on PATH, MKBOOTIMG naming mkbootimg (or
# mkbootimg.py) and AVB_KEY naming AOSP's testkey_rsa4096.pem.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$ROOT/work"
OUT="$ROOT/out"
NAME="RG_Cube_GammaOS_Next_Full_v1.2"
# Published in the upstream release notes when the build went public.
PASSWORD="f297133b935e68072bd77309645cca86ab24bd26847e42c8080db143157a090a"
if [[ -n "${SYSTEM_IMG:-}" ]]; then
  [[ -f "$SYSTEM_IMG" ]] || { echo "SYSTEM_IMG: no file $SYSTEM_IMG" >&2; exit 1; }
  SYSTEM_IMG="$(realpath "$SYSTEM_IMG")"
  VERSION="2.0"
else
  VERSION="1.0"
fi
# Firmware with the ES-DE app gets its own name, so it never replaces the
# regular build.
VARIANT=""
[[ -n "${ESDE_APK:-}" ]] && VARIANT="_ES-DE"
export IMPROVED_VERSION="GammaOS_Next_Improved_v$VERSION$VARIANT"
SEVENZIP="$(command -v 7zz || command -v 7z || true)"
[[ -n "$SEVENZIP" ]] || { echo "7-Zip (7zz or 7z) is required" >&2; exit 1; }
if [[ -n "${ESDE_APK:-}" ]]; then
  [[ -f "$ESDE_APK" ]] || { echo "ESDE_APK: no file $ESDE_APK" >&2; exit 1; }
  command -v aapt >/dev/null || { echo "aapt is required with ESDE_APK" >&2; exit 1; }
  ESDE_APK="$(realpath "$ESDE_APK")"
  export ESDE_APK
fi

for tool in fsck.erofs mkfs.erofs unpack_bootimg lz4 avbtool; do
  command -v "$tool" >/dev/null || { echo "$tool is required (DisplayPort driver patch)" >&2; exit 1; }
done
[[ -n "${MKBOOTIMG:-}" && -f "${AVB_KEY:-}" ]] || {
  echo "MKBOOTIMG and AVB_KEY are required (DisplayPort driver patch)" >&2; exit 1; }

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
if [[ -n "${SYSTEM_IMG:-}" ]]; then
  size="$(stat -c %s "$system")"
  (($(stat -c %s "$SYSTEM_IMG") <= size)) || { echo "$SYSTEM_IMG is larger than system_a ($size bytes)" >&2; exit 1; }
  cp "$SYSTEM_IMG" "$system"
  truncate -s "$size" "$system"
fi

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

pac="$OUT/RG_Cube_GammaOS_Next_Improved_v$VERSION$VARIANT.pac"
echo "writing $pac"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" system_a "$system"
dlkm="$WORK/images/vendor_dlkm_a.img"
python3 -I "$ROOT/scripts/lptool.py" extract "$super" vendor_dlkm_a "$dlkm"
python3 -I "$ROOT/scripts/patch_vendor_dlkm.py" "$dlkm" "$dlkm.patched"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" vendor_dlkm_a "$dlkm.patched"
rm -f "$dlkm" "$dlkm.patched"
cp "$WORK/pac/$NAME.pac" "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" super_full.img "$super"
vendor_boot="$WORK/images/vendor_boot.img"
python3 -I "$ROOT/scripts/pactool.py" extract "$WORK/pac/$NAME.pac" VENDORBOOT "$vendor_boot"
python3 -I "$ROOT/scripts/patch_vendor_boot.py" "$vendor_boot" "$vendor_boot.patched"
for slot in VENDORBOOT VENDORBOOT_B; do
  python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" "$slot" "$vendor_boot.patched"
done
rm -f "$vendor_boot" "$vendor_boot.patched"
python3 -I "$ROOT/scripts/pactool.py" fixcrc "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" setinfo "$pac.partial" "Improved_v$VERSION$VARIANT" RG_Cube_GammaOS_Next_Improved
mv "$pac.partial" "$pac"
rm -f "$super" "$system"
