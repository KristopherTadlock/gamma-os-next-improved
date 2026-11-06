#!/usr/bin/env bash
# Downloads the official GammaOS Next v1.2 Full release for the Anbernic RG Cube.
set -euo pipefail

TAG="v.1.2.0-ANBERNICRGCUBE"
BASE="https://github.com/TheGammaSqueeze/GammaOSNext/releases/download/${TAG}"
NAME="RG_Cube_GammaOS_Next_Full_v1.2.7z"
DEST="${1:-work/download}"

mkdir -p "$DEST"
for part in 001 002 003 004 005; do
  file="${NAME}.${part}"
  if [[ -f "$DEST/$file" ]]; then
    echo "have $file"
    continue
  fi
  echo "fetching $file"
  curl -fL --retry 3 -o "$DEST/$file.partial" "$BASE/$file"
  mv "$DEST/$file.partial" "$DEST/$file"
done
