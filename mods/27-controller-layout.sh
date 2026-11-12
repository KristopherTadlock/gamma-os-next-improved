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
if python3 -I - "$work/in.tar" "$work/out.tar" <<'PY'
import io, re, sys, tarfile
# Android key codes: BUTTON_A 96, BUTTON_B 97, BUTTON_X 99, BUTTON_Y 100.
by_print = {"Circle": "96", "Cross": "97", "Square": "100", "Triangle": "99"}
by_position = {"Circle": "97", "Cross": "96", "Square": "99", "Triangle": "100"}
pattern = r"^((Circle|Cross|Square|Triangle) = 10-)(\d+)"
found = changed = False
with tarfile.open(sys.argv[1]) as src, tarfile.open(sys.argv[2], "w", format=src.format) as dst:
    for m in src.getmembers():
        data = src.extractfile(m).read() if m.isfile() else None
        if m.isfile() and m.name.lstrip("./") == "sdcard/PSP/SYSTEM/controls.ini":
            text = data.decode()
            binds = {g.group(2): g.group(3) for g in re.finditer(pattern, text, flags=re.M)}
            if len(re.findall(pattern, text, flags=re.M)) != 4 or len(binds) != 4:
                sys.exit("controls.ini: %d face-button binds, expected 4" % len(binds))
            if binds == by_print:
                text = re.sub(pattern, lambda g: g.group(1) + by_position[g.group(2)], text, flags=re.M)
                data = text.encode()
                m.size = len(data)
                changed = True
            elif binds != by_position:
                sys.exit("controls.ini: unexpected face-button binds %s" % binds)
            found = True
        dst.addfile(m, io.BytesIO(data) if data is not None else None)
if not found:
    sys.exit("no sdcard/PSP/SYSTEM/controls.ini in the PPSSPP payload")
if not changed:
    print("PPSSPP's controls are already by position")
    sys.exit(3)
PY
then
  tar_to_payload "$work/out.tar" "$work/${pack##*/}"
  write_through "$work/${pack##*/}" "$pack"
elif (($? != 3)); then
  exit 1
fi
echo "the built-in controller reports its face buttons by position; PPSSPP's controls follow"
