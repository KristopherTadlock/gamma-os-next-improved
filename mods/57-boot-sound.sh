#!/usr/bin/env bash
# Downloads the ES-DE home's startup sound and installs it where Nano plays it,
# /system/etc/nano_xmb/audio/esde_boot.mp3 (Nano plays its first two seconds as
# the ES-DE system view appears after a cold boot). "Computer Startup Sound
# Effect" by Dragon Studio, from Pixabay under the Pixabay Content License; it
# is fetched here rather than kept in this repository.
#
# Usage: 57-boot-sound.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"

URL="https://cdn.pixabay.com/download/audio/2025/03/13/audio_41e6aeedf3.mp3?filename=dragon-studio-computer-startup-sound-effect-312870.mp3"
SHA256="52d6b9545aa131150457845dd8ccbf585ba6e7c5c5e13a5ec5bf84812b416f5e"

dest="$SYSTEM/etc/nano_xmb/audio"
if ! is_nano_base "$SYSTEM" || [[ ! -d "$dest" ]]; then
  echo "no GammaOS Nano audio folder in this base; skipping"
  exit 0
fi

cache="$2/boot-sound"
mkdir -p "$cache"
cached="$cache/$SHA256.mp3"
if [[ ! -f "$cached" ]]; then
  curl -fsSL --retry 3 -A "Mozilla/5.0" -e "https://pixabay.com/" -o "$cached.partial" "$URL"
  if ! echo "$SHA256  $cached.partial" | sha256sum -c --quiet; then
    rm -f "$cached.partial"
    echo "the downloaded startup sound does not match its checksum" >&2
    exit 1
  fi
  mv "$cached.partial" "$cached"
fi
echo "$SHA256  $cached" | sha256sum -c --quiet || { rm -f "$cached"; echo "checksum mismatch: $cached (deleted from the cache)" >&2; exit 1; }
install_system_file "$cached" "$dest/esde_boot.mp3"
echo "installed the ES-DE startup sound"
