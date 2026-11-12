#!/usr/bin/env bash
# Turns a Thimsy firmware built by scripts/build.sh into the installer's
# "Thimsy build": the shared system image and the device's patches against the
# official firmware, never a whole firmware. Checks the installer rebuilds the
# exact same PAC from them, then lays them out for shipping.
#
#   make-thimsy-build.sh [options] THIMSY_PAC OUT_DIR
#
#   --device ID       catalog device (default anbernic-rg-cube)
#   --profile NAME    light | medium | heavy (default heavy)
#   --tag TAG         installer release the downloads point at
#                     (default installer-v<catalog version>)
#   --system-name ID  shared system image id (default thimsy-system-<release>-<profile>)
#   --source PAC      the official firmware; downloaded from its upstream
#                     release (the catalog's source) when not given
#   --apply           also write the result into installer/catalog, so the
#                     next engine build embeds it
#
# OUT_DIR gets:
#   thimsy-build/   the system image (split under 2 GiB) and the device's
#                   .bsdiff patches, and pack.json (what `dev pack` found).
#                   Ship it next to the app (the engine looks
#                   in <app>/thimsy-build) or upload it to the release TAG.
#   catalog/        the catalog with this build written in
#
# Needs: the engine (cargo build --release in installer/engine, or ENGINE=...)
# and python3. About 3 x the PAC size of free space while it runs.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
INSTALLER="$(dirname "$HERE")"
ENGINE="${ENGINE:-$INSTALLER/engine/target/release/thimsy-installer}"
DEVICE=anbernic-rg-cube PROFILE=heavy TAG="" SYSTEM_NAME="" SOURCE="" APPLY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --device) DEVICE="$2"; shift 2 ;;
    --profile) PROFILE="$2"; shift 2 ;;
    --tag) TAG="$2"; shift 2 ;;
    --system-name) SYSTEM_NAME="$2"; shift 2 ;;
    --source) SOURCE="$2"; shift 2 ;;
    --apply) APPLY=1; shift ;;
    -h|--help) sed -n '2,/^set -e/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 0 ;;
    -*) echo "unknown option $1" >&2; exit 2 ;;
    *) break ;;
  esac
done
[[ $# -eq 2 ]] || { echo "usage: $0 [options] THIMSY_PAC OUT_DIR (--help)" >&2; exit 2; }
PAC="$(realpath "$1")"; OUT="$2"
[[ -x "$ENGINE" ]] || { echo "no engine at $ENGINE (cargo build --release in installer/engine)" >&2; exit 1; }
[[ -f "$PAC" ]] || { echo "no file $PAC" >&2; exit 1; }
DEVICE_YAML="$INSTALLER/catalog/devices/$DEVICE.yaml"
[[ -f "$DEVICE_YAML" ]] || { echo "no catalog device $DEVICE" >&2; exit 1; }

version=$(sed -n 's/^version: *"\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' "$INSTALLER/catalog/catalog.yaml")
release=$(sed -n 's/^release: *"\{0,1\}Thimsy \([^"]*\)"\{0,1\}$/\1/p' "$INSTALLER/catalog/catalog.yaml")
TAG="${TAG:-installer-v$version}"
SYSTEM_NAME="${SYSTEM_NAME:-thimsy-system-$release-$PROFILE}"
short=$(sed -n 's/^short: *"\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' "$DEVICE_YAML")
short="${short:-$(echo "$DEVICE" | sed 's/^[a-z]*-//; s/-//g')}"

mkdir -p "$OUT"; OUT="$(realpath "$OUT")"
WORK="$OUT/.work"; rm -rf "$WORK" "$OUT/thimsy-build" "$OUT/catalog"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

if [[ -z "$SOURCE" ]]; then
  echo "== downloading the official firmware for $DEVICE"
  SOURCE=$("$ENGINE" dev source --device "$DEVICE" --workspace "$WORK/source")
fi
SOURCE="$(realpath "$SOURCE")"

echo "== making the device pack (system image + patches)"
"$ENGINE" dev pack --source "$SOURCE" --reference "$PAC" --out "$OUT/thimsy-build" \
  --device "$short" --system-name "$SYSTEM_NAME"

echo "== writing it into the catalog"
cp -r "$INSTALLER/catalog" "$OUT/catalog"
(cd "$OUT/thimsy-build" && python3 -I "$HERE/apply_pack.py" "$OUT/thimsy-build/pack.json" \
  "$OUT/catalog/devices/$DEVICE.yaml" "$OUT/catalog/catalog.yaml" "$PROFILE" "$TAG")
# apply_pack splits a system image over 1.9 GB into parts; ship the parts only.
system_file=$(python3 -I -c "import json,sys;print(json.load(open(sys.argv[1]))['system']['file'])" "$OUT/thimsy-build/pack.json")
ls "$OUT/thimsy-build/$system_file".0* >/dev/null 2>&1 && rm -f "$OUT/thimsy-build/$system_file"
"$ENGINE" --catalog "$OUT/catalog" dev check-catalog

echo "== rebuilding with the installer and comparing"
want=$(sha256sum "$PAC" | cut -d' ' -f1)
"$ENGINE" --catalog "$OUT/catalog" build --device "$DEVICE" --profile "$PROFILE" \
  --workspace "$WORK/ws" --import "$OUT/thimsy-build" --import "$(dirname "$SOURCE")"
got=$(sha256sum "$WORK"/ws/jobs/*/out/*.pac | cut -d' ' -f1)
if [[ "$got" != "$want" ]]; then
  echo "MISMATCH: the installer built $got, $PAC is $want" >&2; exit 1
fi
echo "byte-identical: $want"

if [[ $APPLY -eq 1 ]]; then
  cp "$OUT/catalog/catalog.yaml" "$INSTALLER/catalog/catalog.yaml"
  cp "$OUT/catalog/devices/$DEVICE.yaml" "$DEVICE_YAML"
  echo "== written into $INSTALLER/catalog (rebuild the engine to embed it)"
fi
du -sh "$OUT/thimsy-build"
ls -l "$OUT/thimsy-build"
