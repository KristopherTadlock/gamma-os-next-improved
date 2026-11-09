#!/usr/bin/env bash
# Makes the RG Cube's built-in controller report its face buttons by position,
# as an Xbox controller does, so games map them to the right places: the
# bottom button (printed B) is A, the right one (printed A) is B, the left one
# (printed Y) is X and the top one (printed X) is Y. A PlayStation game's Cross
# is then the bottom button. Android looks in /product before /vendor, so this
# layout replaces the vendor's, which reports the buttons by their print. Nano
# reads the buttons directly, so its menus keep A to confirm.
#
# Usage: 27-controller-layout.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"

dir="$SYSTEM/product/usr/keylayout"
for d in "$SYSTEM/product/usr" "$dir"; do
  if [[ ! -d "$d" ]]; then
    sudo install -d -m 755 -o root -g root "$d"
    sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$d"
  fi
done
install_system_file "$MODS/keylayout/Vendor_484b_Product_1101.kl" "$dir/Vendor_484b_Product_1101.kl"
echo "the built-in controller reports its face buttons by position"
