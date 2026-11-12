#!/usr/bin/env bash
# Downloads the official GammaOS Next v1.2 Full release for the Anbernic RG Cube
# and checks each part against the sha256 GitHub publishes for it.
set -euo pipefail

TAG="v.1.2.0-ANBERNICRGCUBE"
BASE="https://github.com/TheGammaSqueeze/GammaOSNext/releases/download/${TAG}"
NAME="RG_Cube_GammaOS_Next_Full_v1.2.7z"
DEST="${1:-work/download}"

# part sha256
PARTS=(
  "001 d7b3a4eee7c88c5129cd3070c0df605c87e19585c97a08ad6880c6e7567673a6"
  "002 bd51bba348030df62a22110a929468e17d83495d73a72df85ce9868b9b7c4ebf"
  "003 78cd87ba3308093581b53e17f512907495c3a0b96acb70f7c4d8dfc3de6e6c78"
  "004 eec844dab87e340f30a1e345af81cb4e35ac7036d02e534397460937462352a7"
  "005 71666923d21b2c6e03dab6105c73fe3f71e20abae3054566b324ee0bcb9e66bf"
)

mkdir -p "$DEST"
for entry in "${PARTS[@]}"; do
  read -r part sha <<<"$entry"
  file="${NAME}.${part}"
  if [[ -f "$DEST/$file" ]]; then
    echo "have $file"
  else
    echo "fetching $file"
    curl -fL --retry 3 -o "$DEST/$file.partial" "$BASE/$file"
    mv "$DEST/$file.partial" "$DEST/$file"
  fi
  echo "$sha  $DEST/$file" | sha256sum -c --quiet || { rm -f "$DEST/$file"; echo "checksum mismatch: $DEST/$file (deleted)" >&2; exit 1; }
done
