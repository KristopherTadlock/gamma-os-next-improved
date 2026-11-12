#!/usr/bin/env bash
# Adds the apps in added-apps.txt to /system/etc and to first-boot setup.
#
# Usage: 40-added-apps.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
ETC="$SYSTEM/etc"
MODS="$(cd "$(dirname "$0")" && pwd)"
source "$MODS/../scripts/modlib.sh"
CACHE="$2/apps"
mkdir -p "$CACHE"

# Setup lines for the new apps: on 1.2 all of them, on 1.4.4 (which installs
# apps from apps.list) the copied files.
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
  echo "${f[sha256]}  $cached" | sha256sum -c --quiet || { rm -f "$cached"; echo "checksum mismatch: $cached (deleted from the cache)" >&2; exit 1; }

  install_system_file "$cached" "$ETC/$file"

  case "$kind" in
    apk)
      if is_nano_base "$SYSTEM"; then
        apps_list "$SYSTEM" add "${f[name]}" "${f[package]}" "/system/etc/$file"
      else
        printf 'echo "Installing %s."\npm install /system/etc/%s\n\n' "${f[name]}" "$file" >>"$steps"
      fi
      ;;
    file)
      # Owned like the folders Android creates there, so the user can manage it.
      printf 'mkdir -p "/sdcard/%s"\ncp /system/etc/%s "/sdcard/%s/"\nchown -R "$(stat -c %%u:%%g /sdcard/Download)" "/sdcard/%s"\n\n' \
        "${f[dest]}" "$file" "${f[dest]}" "${f[dest]}" >>"$steps"
      ;;
    *)
      echo "unknown kind $kind in added-apps.txt" >&2
      exit 1
      ;;
  esac
  echo "added $file"
  unset f
done <"$MODS/added-apps.txt"

[[ -s "$steps" ]] && add_setup_steps "$SYSTEM" "$steps"
rm "$steps"
