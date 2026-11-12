# Helpers shared by mods/*.sh, for both bases: the RG Cube's GammaOS Next 1.2,
# and GammaOS Next 1.4.4 built from source (Nano). Sourced, not run.
#
# 1.2 packs its first-boot payloads as .tar.xz (some .tar.gz) and lists every
# install step in setup.sh. 1.4.4 packs them as .tar.zst and installs the apps
# in /system/etc/gammaos/apps.list, with a post-install hook per app.

# True when SYSTEM_DIR is the 1.4.4 (Nano) base.
is_nano_base() {
  [[ -f "$1/etc/gammaos/apps.list" ]]
}

# Path of payload NAME (for example "roms") in ETC_DIR, whatever its compression.
payload_path() {
  local p
  for p in "$1/$2.tar.zst" "$1/$2.tar.xz" "$1/$2.tar.gz"; do
    [[ -f "$p" ]] && { echo "$p"; return; }
  done
  echo "no payload $2 in $1" >&2
  return 1
}

# Decompresses payload SRC to the plain tar OUT.
payload_to_tar() {
  case "$1" in
    *.zst) zstd -q -dc "$1" >"$2" ;;
    *.xz) xz -dc "$1" >"$2" ;;
    *.gz) gzip -dc "$1" >"$2" ;;
    *) echo "unknown payload $1" >&2; return 1 ;;
  esac
}

# Compresses the plain tar TAR to OUT, with the compression OUT's name says.
tar_to_payload() {
  case "$2" in
    *.zst) zstd -q -19 -T0 -f "$1" -o "$2" ;;
    *.xz) xz -9e -T0 -c "$1" >"$2" ;;
    *.gz) gzip -9 -c "$1" >"$2" ;;
    *) echo "unknown payload $2" >&2; return 1 ;;
  esac
}

# Copies SRC over the image file DEST. Writing through the existing file keeps
# its owner, mode, and SELinux label.
write_through() {
  sudo cp --no-preserve=all "$1" "$2"
}

# Installs SRC as the new image file DEST, readable by all and labelled like the
# rest of /system.
install_system_file() {
  sudo install -m 644 -o root -g root "$1" "$2"
  sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$2"
}

# Inserts the shell lines in STEPS_FILE into first-boot setup, after the apps
# are installed and before setup finishes. Their output goes to the setup log.
add_setup_steps() {
  local system="$1" steps="$2" setup="$1/bin/setup.sh" marker
  if is_nano_base "$system"; then
    marker='mkdir -p /data/setupcompleted'
  else
    marker='echo "Granting permissions to applications."'
  fi
  sudo grep -qxF "$marker" "$setup" || { echo "setup.sh has no line: $marker" >&2; return 1; }
  sudo awk -v steps="$steps" -v marker="$marker" '
    $0 == marker { while ((getline line < steps) > 0) print line }
    { print }
  ' "$setup" >"$steps.setup"
  write_through "$steps.setup" "$setup"
  rm "$steps.setup"
}

# 1.4.4 only: adds the shell functions in FUNCS_FILE to setup.sh, next to its own
# post-install hooks, so apps.list can name them.
add_setup_functions() {
  local setup="$1/bin/setup.sh" funcs="$2"
  local marker='step "Starting configuration of the Thimsy system..."'
  sudo grep -qxF "$marker" "$setup" || marker='step "Starting configuration of the GammaOS system..."'
  sudo grep -qxF "$marker" "$setup" || { echo "setup.sh has no line: $marker" >&2; return 1; }
  sudo awk -v funcs="$funcs" -v marker="$marker" '
    $0 == marker { while ((getline line < funcs) > 0) print line; print "" }
    { print }
  ' "$setup" >"$funcs.setup"
  write_through "$funcs.setup" "$setup"
  rm "$funcs.setup"
}

# 1.4.4 only: edits apps.list with scripts/apps_list.py (see its usage).
apps_list() {
  local system="$1"; shift
  local list="$system/etc/gammaos/apps.list" tmp
  tmp="$(mktemp)"
  sudo cat "$list" >"$tmp"
  python3 -I "$(dirname "${BASH_SOURCE[0]}")/apps_list.py" "$tmp" "$@"
  write_through "$tmp" "$list"
  rm "$tmp"
}
