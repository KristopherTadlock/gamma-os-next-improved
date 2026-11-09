#!/usr/bin/env bash
# Makes the RG Cube's built-in controller report its face buttons by position,
# as an Xbox controller does, so games map them to the right places: the
# bottom button (printed B) is A, the right one (printed A) is B, the left one
# (printed Y) is X and the top one (printed X) is Y. A PlayStation game's Cross
# is then the bottom button. Android looks in /product before /vendor, so this
# layout replaces the vendor's, which reports the buttons by their print. Nano
# reads the buttons directly, so its menus keep A to confirm. Controller Enable
# (gammapad) reads only the vendor's layout, so with it on apps get the print.
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

# PPSSPP's controls, set for the buttons' print, move to their positions: Cross
# on the bottom button, Circle on the right one, Square on the left and Triangle
# on the top. (10-update-emulators.sh does the same for RetroArch's binds.)
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
pack="$(payload_path "$SYSTEM/etc" ppsspp)"
payload_to_tar "$pack" "$work/in.tar"
python3 -I - "$work/in.tar" "$work/out.tar" <<'PY'
import io, re, sys, tarfile
swap = {"96": "97", "97": "96", "99": "100", "100": "99"}
found = False
with tarfile.open(sys.argv[1]) as src, tarfile.open(sys.argv[2], "w", format=src.format) as dst:
    for m in src.getmembers():
        data = src.extractfile(m).read() if m.isfile() else None
        if m.isfile() and m.name.lstrip("./") == "sdcard/PSP/SYSTEM/controls.ini":
            text, n = re.subn(r"^((?:Circle|Cross|Square|Triangle) = 10-)(\d+)",
                              lambda g: g.group(1) + swap.get(g.group(2), g.group(2)),
                              data.decode(), flags=re.M)
            if n != 4:
                sys.exit("controls.ini: %d face-button binds, expected 4" % n)
            data = text.encode()
            m.size = len(data)
            found = True
        dst.addfile(m, io.BytesIO(data) if data is not None else None)
if not found:
    sys.exit("no sdcard/PSP/SYSTEM/controls.ini in the PPSSPP payload")
PY
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
write_through "$work/${pack##*/}" "$pack"
echo "the built-in controller reports its face buttons by position; PPSSPP's controls follow"
