#!/usr/bin/env bash
# For each published RG Cube firmware release: download it, check its
# SHA256SUMS, make a device pack (shared system image + deltas) from the
# official GammaOS 1.2 firmware, rebuild with the installer, and check the
# result is byte-identical to the published PAC.
#
#   verify-drafts.sh WORKDIR SOURCE_PAC TAG...
# WORKDIR/ws/cache may already hold the GammaOS download (reused).
# WORKDIR/results.tsv gets one line per release (rewritten each run); exits
# non-zero when any release FAILs.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ENGINE="${ENGINE:-$HERE/../engine/target/release/thimsy-installer}"
SEVENZ="${SEVENZ:-7zz}"
REPO="${REPO:-KristopherTadlock/thimsy-os}"
WORK="$1"; SRC="$2"; shift 2
mkdir -p "$WORK"; RESULTS="$WORK/results.tsv"
: > "$RESULTS"   # this run's results only
for TAG in "$@"; do
  D="$WORK/$TAG"; rm -rf "$D"; mkdir -p "$D/dl" "$D/pac"
  echo "== $TAG $(date +%T)"
  if ! gh release download "$TAG" -R "$REPO" -D "$D/dl" -p '*.7z.*' -p 'SHA256SUMS' >/dev/null 2>&1; then
    echo -e "$TAG\tSKIP\tdownload failed" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  # SHA256SUMS lists the 7z volumes and, in some releases, the PAC inside
  # them: check the volumes now and the PAC once extracted. A release
  # without it can't be checked, so it fails.
  if [ ! -f "$D/dl/SHA256SUMS" ]; then
    echo -e "$TAG\tFAIL\tno SHA256SUMS published" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  mv "$D/dl/SHA256SUMS" "$D/SHA256SUMS"
  first=$(ls "$D/dl"/*.7z.001 2>/dev/null | head -1)
  if [ -z "$first" ]; then echo -e "$TAG\tSKIP\tno 7z volumes" | tee -a "$RESULTS"; rm -rf "$D"; continue; fi
  # Every volume must be listed and match (not just the ones listed).
  unlisted=""
  for vol in "$D/dl"/*.7z.*; do
    awk -v f="$(basename "$vol")" '{n=$2; sub(/^\*/, "", n)} n==f {found=1} END {exit !found}' "$D/SHA256SUMS" || unlisted="$unlisted $(basename "$vol")"
  done
  if [ -n "$unlisted" ] || ! (cd "$D/dl" && sha256sum -c --quiet --ignore-missing ../SHA256SUMS); then
    echo -e "$TAG\tFAIL\tpublished files don't match SHA256SUMS${unlisted:+ (not listed:$unlisted)}" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  # Only the PAC, into a new empty folder (an archive's other files, or
  # paths, never land anywhere else).
  rm -rf "$D/pac"; mkdir "$D/pac"
  "$SEVENZ" x -y -r "$first" -o"$D/pac" '*.pac' >/dev/null || { echo -e "$TAG\tFAIL\textract" | tee -a "$RESULTS"; rm -rf "$D"; continue; }
  rm -rf "$D/dl"
  mapfile -t pacs < <(find "$D/pac" -type f -name '*.pac')
  if [ "${#pacs[@]}" -ne 1 ]; then
    echo -e "$TAG\tFAIL\texpected one PAC in the archive, found ${#pacs[@]}" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  REF="${pacs[0]}"
  REF_SHA=$(sha256sum "$REF" | cut -d' ' -f1)
  # Names may carry sha256sum's binary-mode marker: "HASH *file".
  LISTED=$(awk -v f="$(basename "$REF")" '{n=$2; sub(/^\*/, "", n)} n==f {print $1}' "$D/SHA256SUMS" 2>/dev/null)
  if [ -n "$LISTED" ] && [ "$LISTED" != "$REF_SHA" ]; then
    echo -e "$TAG\tFAIL\textracted PAC doesn't match SHA256SUMS" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  if ! "$ENGINE" dev pack --source "$SRC" --reference "$REF" --out "$D/pack" --device rgcube --system-name "thimsy-system-$TAG" > "$D/pack.log" 2>&1; then
    echo -e "$TAG\tFAIL\tpack: $(tail -1 "$D/pack.log")" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  rm -f "$REF"
  cp -r "$HERE/../catalog" "$D/catalog"
  # Build from this release's loose pack files: drop the device's published
  # bundles, or the engine would want (and download) the 1.0a archive. This
  # copy only builds heavy: other profiles' entries may go (--replace-others).
  python3 -I - "$D/catalog/devices/anbernic-rg-cube.yaml" <<'PY'
import re, sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
s = re.sub(r"^bundles:[ \t]*\n(?:[ \t]+\S.*\n|[ \t]*\n(?=[ \t]+\S))*\n?", "", s, flags=re.M)
open(p, "w", encoding="utf-8").write(s)
PY
  if grep -q '^bundles:' "$D/catalog/devices/anbernic-rg-cube.yaml" ||
     ! python3 -I "$HERE/../tools/apply_pack.py" --replace-others "$D/pack/pack.json" "$D/catalog/devices/anbernic-rg-cube.yaml" "$D/catalog/catalog.yaml" heavy "$TAG" > "$D/apply.log" 2>&1; then
    echo -e "$TAG\tFAIL\tapply_pack: $(tail -1 "$D/apply.log" 2>/dev/null)" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  # Keep only the published-size parts, when apply_pack split the image
  # into parts (it doesn't for a small one: then the image is what's used).
  for img in "$D/pack"/thimsy-system-*.img.zst; do
    [ -f "$img" ] && [ -f "$img.001" ] && rm -f "$img"
  done
  WS="$WORK/ws"; rm -rf "$WS/jobs"
  if ! "$ENGINE" --catalog "$D/catalog" --json build --device anbernic-rg-cube --profile heavy --workspace "$WS" --import "$D/pack" > "$D/build.jsonl" 2>"$D/build.err"; then
    echo -e "$TAG\tFAIL\tbuild: $(grep '"type":"error"' "$D/build.jsonl" | tail -1)" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  OUT=$(ls "$WS"/jobs/*/out/*.pac)
  OUT_SHA=$(sha256sum "$OUT" | cut -d' ' -f1)
  SYS=$(python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["system"]["size"])' "$D/pack/pack.json")
  DELTAS=$(du -cb "$D"/pack/*.bsdiff | tail -1 | cut -f1)
  if [ "$OUT_SHA" = "$REF_SHA" ]; then
    echo -e "$TAG\tPASS\t$REF_SHA\tsystem=$SYS deltas=$DELTAS" | tee -a "$RESULTS"
  else
    echo -e "$TAG\tFAIL\tbuilt $OUT_SHA, published $REF_SHA" | tee -a "$RESULTS"
  fi
  # Keep the per-release packs small for the record; drop the big files.
  rm -rf "$WS/jobs" "$D/pack"/thimsy-system-* "$WS/cache"/thimsy-system-*
  find "$WS/cache" -name 'rgcube-*' -delete
done
echo "== done"; cat "$RESULTS"
if grep -q "$(printf '\tFAIL\t')" "$RESULTS"; then
  echo "== some releases FAILED" >&2
  exit 1
fi
