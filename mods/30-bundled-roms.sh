#!/usr/bin/env bash
# Rebuilds the ROM pack installed on first boot from bundled-roms.txt, with the
# games' box art, screenshots and details from pack-in-media.txt.
#
# Usage: 30-bundled-roms.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

ETC="$1/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
work="$2/roms"
pack="$(payload_path "$ETC" roms)"

mkdir -p "$work"
list="$MODS/bundled-roms.txt"
nano=()
if is_nano_base "$1"; then
  # Daijisho's launch files (.dpt) do nothing without it; ScummVM lists its game itself.
  grep -v '\.dpt  ' "$list" >"$work/bundled-roms.txt"
  list="$work/bundled-roms.txt"
  nano=(nano)
fi
payload_to_tar "$pack" "$work/in.tar"
python3 -I "$MODS/../scripts/bundled_roms.py" "$list" \
  "$work/in.tar" "$work/out.tar" "$work/cache" "$MODS/rom-licenses" "$MODS/rom-files" \
  "$MODS/pack-in-media.txt" "${nano[@]}"
tar_to_payload "$work/out.tar" "$work/${pack##*/}"
echo "${pack##*/}: $(stat -c %s "$pack") -> $(stat -c %s "$work/${pack##*/}") bytes"
write_through "$work/${pack##*/}" "$pack"
rm -f "$work/in.tar" "$work/out.tar" "$work/${pack##*/}" "$work/bundled-roms.txt"

if is_nano_base "$1"; then
  # The ROM pack brings Nano's box art index for the pack-in games. Nano may
  # have started on an empty one already, so it's told to read it again.
  cat >"$work/setup-steps.sh" <<'EOF2'
if [ "$FRESH_SETUP" = 1 ] && [ -f /data/system/nano_scrape/index.json ]; then
    restorecon -R /data/system/nano_scrape
    setprop sys.gammaos.nano.scrape_reload "pack-ins-$(date +%s)"
fi

EOF2
  add_setup_steps "$1" "$work/setup-steps.sh"
  rm "$work/setup-steps.sh"
fi
