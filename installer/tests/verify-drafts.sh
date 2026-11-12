#!/usr/bin/env bash
# For each published RG Cube firmware release: download it, check its
# SHA256SUMS, make a device pack (shared system image + deltas) from the
# official GammaOS 1.2 firmware, rebuild with the installer, and check the
# result is byte-identical to the published PAC.
#
#   verify-drafts.sh WORKDIR SOURCE_PAC TAG...
# WORKDIR/ws/cache may already hold the GammaOS download (reused).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ENGINE="${ENGINE:-$HERE/../engine/target/release/thimsy-installer}"
SEVENZ="${SEVENZ:-7zz}"
REPO="${REPO:-KristopherTadlock/thimsy-os}"
WORK="$1"; SRC="$2"; shift 2
mkdir -p "$WORK"; RESULTS="$WORK/results.tsv"
for TAG in "$@"; do
  D="$WORK/$TAG"; rm -rf "$D"; mkdir -p "$D/dl" "$D/pac"
  echo "== $TAG $(date +%T)"
  if ! gh release download "$TAG" -R "$REPO" -D "$D/dl" -p '*.7z.*' -p 'SHA256SUMS' >/dev/null 2>&1; then
    echo -e "$TAG\tSKIP\tdownload failed" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  # SHA256SUMS lists the 7z volumes and, in some releases, the PAC inside
  # them: check the volumes now and the PAC once extracted.
  [ -f "$D/dl/SHA256SUMS" ] && mv "$D/dl/SHA256SUMS" "$D/SHA256SUMS"
  if [ -f "$D/SHA256SUMS" ] && ! (cd "$D/dl" && sha256sum -c --quiet --ignore-missing ../SHA256SUMS); then
    echo -e "$TAG\tFAIL\tpublished files don't match SHA256SUMS" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  first=$(ls "$D/dl"/*.7z.001 2>/dev/null | head -1)
  if [ -z "$first" ]; then echo -e "$TAG\tSKIP\tno 7z volumes" | tee -a "$RESULTS"; rm -rf "$D"; continue; fi
  "$SEVENZ" x -y "$first" -o"$D/pac" >/dev/null || { echo -e "$TAG\tFAIL\textract" | tee -a "$RESULTS"; rm -rf "$D"; continue; }
  rm -rf "$D/dl"
  REF=$(ls "$D/pac"/*.pac | head -1)
  REF_SHA=$(sha256sum "$REF" | cut -d' ' -f1)
  LISTED=$(awk -v f="$(basename "$REF")" '$2==f {print $1}' "$D/SHA256SUMS" 2>/dev/null)
  if [ -n "$LISTED" ] && [ "$LISTED" != "$REF_SHA" ]; then
    echo -e "$TAG\tFAIL\textracted PAC doesn't match SHA256SUMS" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  if ! "$ENGINE" dev pack --source "$SRC" --reference "$REF" --out "$D/pack" --device rgcube --system-name "thimsy-system-$TAG" > "$D/pack.log" 2>&1; then
    echo -e "$TAG\tFAIL\tpack: $(tail -1 "$D/pack.log")" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  rm -f "$REF"
  cp -r "$HERE/../catalog" "$D/catalog"
  python3 "$HERE/../tools/apply_pack.py" "$D/pack/pack.json" "$D/catalog/devices/anbernic-rg-cube.yaml" "$D/catalog/catalog.yaml" heavy "$TAG" >/dev/null
  rm -f "$D/pack"/thimsy-system-*.img.zst   # keep only the published-size parts
  WS="$WORK/ws"; rm -rf "$WS/jobs"
  if ! "$ENGINE" --catalog "$D/catalog" --json build --device anbernic-rg-cube --profile heavy --workspace "$WS" --import "$D/pack" > "$D/build.jsonl" 2>"$D/build.err"; then
    echo -e "$TAG\tFAIL\tbuild: $(grep '"type":"error"' "$D/build.jsonl" | tail -1)" | tee -a "$RESULTS"; rm -rf "$D"; continue
  fi
  OUT=$(ls "$WS"/jobs/*/out/*.pac)
  OUT_SHA=$(sha256sum "$OUT" | cut -d' ' -f1)
  SYS=$(python3 -c "import json;print(json.load(open('$D/pack/pack.json'))['system']['size'])")
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
