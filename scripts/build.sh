#!/usr/bin/env bash
# Builds out/RG_Cube_Thimsy_OS_v1.0.pac from the official release.
#
# Needs: curl, 7zz (or 7z), xz, zstd, unzip, e2fsck, resize2fs, dumpe2fs,
# setfattr, flock, cc, python3, and sudo for loop-mounting the system image.
# sudo asks for its password once, at the start; the build keeps it valid
# until it ends.
#
# Runs tests/run.sh quick first; SKIP_TESTS=1 skips that.
#
# The RetroArch cores are the buildbot nightlies pinned in
# mods/retroarch-cores.lock (see scripts/refresh_cores.sh). REFRESH_CORES=1
# pins the current ones first; ALLOW_BUNDLED_CORES=1 lets a core that can't be
# downloaded as pinned keep the base image's copy.
#
# Set SYSTEM_IMG=/path/to/system.img to use a GammaOS Next 1.4.4 system image
# built from source instead of the release's 1.2 system (the RG Cube's vendor,
# kernel and bootloader stay). That builds ..._v1.0a.pac.
#
# Set ESDE_APK=/path/to/ES-DE.apk to make ES-DE the home app
# (mods/37-esde.sh, 1.2 base only); that also needs aapt.
#
# The vendor kernel's DisplayPort driver is patched (scripts/sprd_drm.py) in
# vendor_dlkm (scripts/patch_vendor_dlkm.py) and vendor_boot
# (scripts/patch_vendor_boot.py). That needs fsck.erofs, mkfs.erofs,
# unpack_bootimg, lz4 and avbtool on PATH, MKBOOTIMG naming mkbootimg (or
# mkbootimg.py) and AVB_KEY naming AOSP's testkey_rsa4096.pem.
#
# odm gets a precompiled SELinux policy matching the system image
# (scripts/odm_sepolicy.py), so init does not compile one at every boot. That
# needs secilc (from the AOSP host build) and debugfs on PATH.
#
# With SYSTEM_IMG, the bootloader's boot picture (BootLogo, and the fastboot-mode
# copy Fastboot_Logo) becomes Nano's boot screen without its status line
# (scripts/bootlogo.py), so the screen from
# power-on carries straight into Nano's. That needs nano-bootlogo (from the AOSP
# host build) on PATH; BOOT_LOGO_SVG and BOOT_LOGO_FONT default to the logo and
# Roboto in the AOSP tree MKBOOTIMG is in (or AOSP=<tree>).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$ROOT/work"
OUT="$ROOT/out"
if [[ "${SKIP_TESTS:-}" != 1 ]]; then
  "$ROOT/tests/run.sh" quick || { echo "the quick tests failed (SKIP_TESTS=1 skips them)" >&2; exit 1; }
fi
mkdir -p "$WORK"
# Two builds at once would share work/: its images, mount point and cache.
exec 9>"$WORK/.lock"
flock -n 9 || { echo "another build is using $WORK" >&2; exit 1; }
NAME="RG_Cube_GammaOS_Next_Full_v1.2"
# Published in the upstream release notes when the build went public.
PASSWORD="f297133b935e68072bd77309645cca86ab24bd26847e42c8080db143157a090a"
if [[ -n "${SYSTEM_IMG:-}" ]]; then
  [[ -f "$SYSTEM_IMG" ]] || { echo "SYSTEM_IMG: no file $SYSTEM_IMG" >&2; exit 1; }
  SYSTEM_IMG="$(realpath "$SYSTEM_IMG")"
  VERSION="1.0a"
else
  VERSION="1.0"
fi
# Firmware with the ES-DE app gets its own name, so it never replaces the
# regular build.
VARIANT=""
[[ -n "${ESDE_APK:-}" ]] && VARIANT="_ES-DE"
export THIMSY_BUILD_VERSION="Thimsy_OS_v$VERSION$VARIANT"
export THIMSY_VERSION="$VERSION"
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
for tool in secilc debugfs; do
  command -v "$tool" >/dev/null || { echo "$tool is required (precompiled SELinux policy)" >&2; exit 1; }
done
mkbootimg="${MKBOOTIMG:-}"; mkbootimg="${mkbootimg%% *}"
[[ -n "$mkbootimg" && ( -f "$mkbootimg" || -n "$(command -v "$mkbootimg")" ) && -f "${AVB_KEY:-}" ]] || {
  echo "MKBOOTIMG and AVB_KEY are required (DisplayPort driver patch)" >&2; exit 1; }

if [[ -n "${SYSTEM_IMG:-}" ]]; then
  AOSP="${AOSP:-${mkbootimg%/system/tools/mkbootimg/mkbootimg.py}}"
  BOOT_LOGO_SVG="${BOOT_LOGO_SVG:-$AOSP/frameworks/base/cmds/gammaos-nano/bootscreen/thimsy.svg}"
  BOOT_LOGO_FONT="${BOOT_LOGO_FONT:-$AOSP/external/roboto-fonts/Roboto-Regular.ttf}"
  command -v nano-bootlogo >/dev/null || { echo "nano-bootlogo is required (boot picture)" >&2; exit 1; }
  [[ -f "$BOOT_LOGO_SVG" && -f "$BOOT_LOGO_FONT" ]] || {
    echo "BOOT_LOGO_SVG and BOOT_LOGO_FONT are required (boot picture)" >&2; exit 1; }
fi

mkdir -p "$WORK/bin" "$WORK/images" "$WORK/mnt" "$OUT"
super="$WORK/images/super_full.img"
system="$WORK/images/system_a.img"
mnt="$WORK/mnt/system"

# A long step (7-Zip, xz) can outlast sudo's timestamp, leaving a later sudo
# waiting for a password nobody types; this keeps it valid until the build ends.
sudo -v
(exec 9>&-; while sleep 60 && kill -0 $$ 2>/dev/null; do sudo -n -v 2>/dev/null || exit; done) &
keepalive=$!
mounted=""

# umount returns once this mount is gone, but a copy of it in another mount
# namespace keeps the filesystem alive, and the image file can then be packed
# (or the next build started) before its last writes reach it.
wait_for_loop() {
  for _ in $(seq 120); do
    [[ -z "$(sudo losetup -j "$system")" ]] && return 0
    sleep 1
  done
  echo "the system image is still in use after unmounting" >&2
  return 1
}
cleanup() {
  if [[ -n "$mounted" ]]; then
    sudo umount "$mnt" 2>/dev/null || true
    wait_for_loop || true
  fi
  kill "$keepalive" 2>/dev/null || true
}
trap cleanup EXIT

# A loop device left on an image by an earlier build would keep writing to it.
if mountpoint -q "$mnt"; then
  echo "$mnt is still mounted from an earlier build; unmount it first" >&2
  exit 1
fi
for img in "$system" "$super"; do
  if [[ -e "$img" && -n "$(sudo losetup -j "$img")" ]]; then
    echo "$img is still attached to a loop device (sudo losetup -j $img); detach it first" >&2
    exit 1
  fi
done
export CRC16ARC="$WORK/bin/crc16arc"
cc -O2 -o "$CRC16ARC" "$ROOT/tools/crc16arc.c"

"$ROOT/scripts/fetch.sh" "$WORK/download"
base_pac="$WORK/pac/$NAME.pac"
if [[ ! -f "$base_pac" ]]; then
  # Extract next to the final name, so an interrupted extraction is redone.
  rm -rf "$WORK/pac.partial"
  "$SEVENZIP" x -y -o"$WORK/pac.partial" -p"$PASSWORD" "$WORK/download/$NAME.7z.001" >/dev/null
  mkdir -p "$WORK/pac"
  rm -f "$base_pac.sha256"
  mv "$WORK/pac.partial/$NAME.pac" "$base_pac"
  rm -rf "$WORK/pac.partial"
fi
# The PAC's own CRCs are checked on its first use and its sha256 recorded;
# later builds check that, so a PAC changed on disk is never built from.
if [[ -f "$base_pac.sha256" ]]; then
  (cd "$(dirname "$base_pac")" && sha256sum -c --quiet "$NAME.pac.sha256") ||
    { echo "$base_pac changed since its first use; delete it to extract it again" >&2; exit 1; }
else
  python3 -I "$ROOT/scripts/pactool.py" verify "$base_pac"
  (cd "$(dirname "$base_pac")" && sha256sum "$NAME.pac") >"$base_pac.sha256.partial"
  mv "$base_pac.sha256.partial" "$base_pac.sha256"
fi

echo "extracting system image"
# New files, not the old ones overwritten: a stale loop device keeps the old inode.
rm -f "$super" "$system"
python3 -I "$ROOT/scripts/pactool.py" extract "$base_pac" super_full.img "$super"
# Give the system partition the unallocated space at the end of super.
python3 -I "$ROOT/scripts/lptool.py" grow "$super" system_a max
python3 -I "$ROOT/scripts/lptool.py" extract "$super" system_a "$system"
if [[ -n "${SYSTEM_IMG:-}" ]]; then
  size="$(stat -c %s "$system")"
  (($(stat -c %s "$SYSTEM_IMG") <= size)) || { echo "$SYSTEM_IMG is larger than system_a ($size bytes)" >&2; exit 1; }
  rm -f "$system"
  cp "$SYSTEM_IMG" "$system"
  truncate -s "$size" "$system"
fi

# The image is built with deduplicated (shared) blocks, which the kernel only
# mounts read-only. Grow the filesystem to fill its partition, then unshare.
# e2fsck exits 1 when it changed the filesystem, which is expected here.
e2fsck -fy "$system" >/dev/null || (($? == 1))
resize2fs "$system" "$(($(stat -c %s "$system") / 4096))"
e2fsck -fy -E unshare_blocks "$system" >/dev/null || (($? == 1))

mkdir -p "$mnt"
sudo mount -o loop "$system" "$mnt"
mounted=1
stamp="$WORK/mnt/mods.stamp"
touch "$stamp"
before="$WORK/mnt/files-before.list"
(cd "$mnt" && sudo find . -xdev -type f -print0) >"$before"
for mod in "$ROOT"/mods/*.sh; do
  echo "== $(basename "$mod")"
  bash "$mod" "$mnt/system" "$WORK/cache"
done
df -h "$mnt" | tail -1
written="$WORK/mnt/written.sha256"
(cd "$mnt" && sudo find . -xdev -type f -cnewer "$stamp" -print0 | sudo xargs -0 -r sha256sum) >"$written"
deleted="$WORK/mnt/deleted.list"
LC_ALL=C comm -z -23 <(LC_ALL=C sort -z "$before") <(cd "$mnt" && sudo find . -xdev -type f -print0 | LC_ALL=C sort -z) >"$deleted"
sudo sync -f "$mnt"
sudo umount "$mnt"
mounted=""
wait_for_loop
e2fsck -fn "$system"
# Deleted and replaced files leave their old contents in the free blocks, which
# the PAC carries and 7-Zip compresses (about 500 MB of the download).
python3 -I "$ROOT/scripts/zero_free_blocks.py" "$system"
e2fsck -fn "$system"
python3 -I "$ROOT/scripts/check_image_files.py" "$system" "$written" "$deleted"

pac="$OUT/RG_Cube_Thimsy_OS_v$VERSION$VARIANT.pac"
echo "writing $pac"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" system_a "$system"
dlkm="$WORK/images/vendor_dlkm_a.img"
python3 -I "$ROOT/scripts/lptool.py" extract "$super" vendor_dlkm_a "$dlkm"
# Only the Nano patch's init.rc loads the LED driver beside the module list.
leds=()
[[ -n "${SYSTEM_IMG:-}" ]] || leds=(--load-leds)
python3 -I "$ROOT/scripts/patch_vendor_dlkm.py" "${leds[@]}" "$dlkm" "$dlkm.patched"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" vendor_dlkm_a "$dlkm.patched"
rm -f "$dlkm" "$dlkm.patched"
odm="$WORK/images/odm_a.img"
vendor="$WORK/images/vendor_a.img"
python3 -I "$ROOT/scripts/lptool.py" extract "$super" odm_a "$odm"
python3 -I "$ROOT/scripts/lptool.py" extract "$super" vendor_a "$vendor"
python3 -I "$ROOT/scripts/odm_sepolicy.py" "$odm" "$system" "$vendor" "$odm.patched"
python3 -I "$ROOT/scripts/lptool.py" replace "$super" odm_a "$odm.patched"
rm -f "$odm" "$odm.patched" "$vendor"
cp "$base_pac" "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" super_full.img "$super"
vendor_boot="$WORK/images/vendor_boot.img"
python3 -I "$ROOT/scripts/pactool.py" extract "$base_pac" VENDORBOOT "$vendor_boot"
python3 -I "$ROOT/scripts/patch_vendor_boot.py" "$vendor_boot" "$vendor_boot.patched"
for slot in VENDORBOOT VENDORBOOT_B; do
  python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" "$slot" "$vendor_boot.patched"
done
rm -f "$vendor_boot" "$vendor_boot.patched"
if [[ -n "${SYSTEM_IMG:-}" ]]; then
  logo="$WORK/images/logo.img"
  python3 -I "$ROOT/scripts/pactool.py" extract "$base_pac" logo.img "$logo"
  nano-bootlogo "$BOOT_LOGO_SVG" "$BOOT_LOGO_FONT" 720 720 "$logo.rgba"
  python3 -I "$ROOT/scripts/bootlogo.py" "$logo" "$logo.rgba" "$logo.new"
  python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" logo.img "$logo.new"
  # Fastboot mode shows its own copy of the stock pictures (Anbernic's logo); give it the same.
  python3 -I "$ROOT/scripts/pactool.py" replace "$pac.partial" fbootlogo.img "$logo.new"
  rm -f "$logo" "$logo.rgba" "$logo.new"
fi
python3 -I "$ROOT/scripts/pactool.py" fixcrc "$pac.partial"
python3 -I "$ROOT/scripts/pactool.py" setinfo "$pac.partial" "Thimsy_OS_v$VERSION$VARIANT" RG_Cube_Thimsy_OS
mv "$pac.partial" "$pac"
rm -f "$super" "$system"
