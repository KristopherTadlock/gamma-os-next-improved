#!/usr/bin/env bash
# Adds the apps in added-apps.txt to /system/etc and to first-boot setup.
#
# Usage: 40-added-apps.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
SETUP="$SYSTEM/bin/setup.sh"
MODS="$(cd "$(dirname "$0")" && pwd)"
CACHE="$2/apps"
mkdir -p "$CACHE"

# Setup lines for the new apps, inserted before setup grants app permissions.
steps="$CACHE/setup-steps.sh"
: >"$steps"

while IFS= read -r line; do
  [[ -z "${line// }" || "$line" == \#* ]] && continue
  IFS=$'\t' read -r kind file rest <<<"$(sed -E 's/ {2,}/\t/g' <<<"$line")"
  declare -A f=()
  while IFS= read -r pair; do
    f["${pair%%=*}"]="${pair#*=}"
  done < <(tr '\t' '\n' <<<"$rest")

  cached="$CACHE/${f[sha256]}"
  if [[ ! -f "$cached" ]]; then
    curl -fsSL --retry 3 -o "$cached.partial" "${f[url]}"
    mv "$cached.partial" "$cached"
  fi
  echo "${f[sha256]}  $cached" | sha256sum -c --quiet

  sudo install -m 644 -o root -g root "$cached" "$ETC/$file"
  sudo setfattr -n security.selinux -v u:object_r:system_file:s0 "$ETC/$file"

  case "$kind" in
    apk)
      printf 'echo "Installing %s."\npm install /system/etc/%s\n\n' "${f[name]}" "$file" >>"$steps"
      ;;
    file)
      printf 'mkdir -p "/sdcard/%s"\ncp /system/etc/%s "/sdcard/%s/"\n\n' "${f[dest]}" "$file" "${f[dest]}" >>"$steps"
      ;;
    *)
      echo "unknown kind $kind in added-apps.txt" >&2
      exit 1
      ;;
  esac
  echo "added $file"
  unset f
done <"$MODS/added-apps.txt"

marker='echo "Granting permissions to applications."'
sudo grep -qxF "$marker" "$SETUP" || { echo "setup.sh has no line: $marker" >&2; exit 1; }
sudo awk -v steps="$steps" -v marker="$marker" '
  $0 == marker { while ((getline line < steps) > 0) print line }
  { print }
' "$SETUP" >"$CACHE/setup.sh"
# Writing through the existing file keeps its owner, mode, and SELinux label.
sudo cp --no-preserve=all "$CACHE/setup.sh" "$SETUP"
