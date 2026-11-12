#!/usr/bin/env bash
# Replaces Android's boot animation, GammaOS's cube logo, with the Thimsy logo
# fading in on Nano's Midnight background (/system/product/media/bootanimation.zip,
# shown when the console boots into full Android). mods/bootanimation/bootanimation.zip
# is drawn from the logo by tools/bootanimation/make_bootanimation.py, which also
# writes the same animation into an AOSP tree. The Thimsy patch can only carry
# text, so this mod puts it in the image whatever source tree built it.
#
# Usage: 58-bootanimation.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"

zip="$MODS/bootanimation/bootanimation.zip"
replaced=0
for dest in "$SYSTEM/product/media/bootanimation.zip" "$SYSTEM/media/bootanimation.zip"; do
  # bootanimation-dark.zip is a link to it.
  if sudo test -f "$dest" && ! sudo test -L "$dest"; then
    write_through "$zip" "$dest"
    echo "replaced ${dest#"$SYSTEM"}"
    replaced=1
  fi
done
for dark in "$SYSTEM/product/media/bootanimation-dark.zip" "$SYSTEM/media/bootanimation-dark.zip"; do
  if sudo test -f "$dark" && ! sudo test -L "$dark"; then
    write_through "$zip" "$dark"
    echo "replaced ${dark#"$SYSTEM"}"
  fi
done
if ((!replaced)); then
  if is_nano_base "$SYSTEM"; then
    echo "no bootanimation.zip in the 1.4.4 system" >&2
    exit 1
  fi
  echo "no bootanimation.zip in this system; skipping"
fi
