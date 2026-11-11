#!/usr/bin/env bash
# Sets the version Settings > About phone shows to this release.
#
# Usage: 05-version.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

PROP="$1/build.prop"
VERSION="Thimsy_OS_v1.0"
work="$(realpath -m "$2")/version"

mkdir -p "$work"
for key in ro.lineage.version ro.lineage.display.version; do
  sudo grep -qx "$key=GammaOS_Next_v1.2" "$PROP" || { echo "build.prop has no line: $key=GammaOS_Next_v1.2" >&2; exit 1; }
done
sudo sed -E "s/^(ro\.lineage(\.display)?\.version)=GammaOS_Next_v1\.2$/\1=$VERSION/" "$PROP" >"$work/build.prop"
# Writing through the existing file keeps its owner, mode, and SELinux label.
sudo cp --no-preserve=all "$work/build.prop" "$PROP"
rm -rf "$work"
echo "version: $VERSION"
