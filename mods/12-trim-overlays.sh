#!/usr/bin/env bash
# Removes the Android look-and-feel overlays nothing needs: the icon packs, icon shapes and fonts of
# Android mode's Wallpaper & style screen, and Developer options' display cutout simulations (the
# RG Cube has no cutout). Android reads every overlay package at each boot (the package scan, the
# overlay config and the overlay manager), so 42 fewer shorten system_server's start. The
# navigation bar modes (the gesture setting uses them), the black theme and every device
# configuration overlay stay, so Android mode works as before with fewer style choices.
#
# Usage: 12-trim-overlays.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

OVERLAY="$1/product/overlay"
[[ -d "$OVERLAY" ]] || { echo "no product overlays; skipping"; exit 0; }
n=0
for o in "$OVERLAY"/IconPack* "$OVERLAY"/IconShape* "$OVERLAY"/Font* "$OVERLAY"/DisplayCutout*; do
  [[ -e "$o" ]] || continue
  sudo rm -rf "$o"
  n=$((n + 1))
done
echo "removed $n style overlays; $(ls "$OVERLAY" | wc -l) left"
