#!/usr/bin/env bash
# Sets the version Settings > About phone shows to this release, records it as
# ro.thimsy.version for Nano's System Information, and replaces the build
# machine's user and host names, which a source build records.
#
# Usage: 05-version.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

PROP="$1/build.prop"
VERSION="${THIMSY_BUILD_VERSION:-Thimsy_OS_v1.0}"
THIMSY="${THIMSY_VERSION:-1.0}"
work="$(realpath -m "$2")/version"
source "$(dirname "$0")/../scripts/modlib.sh"

mkdir -p "$work"
sudo cat "$PROP" >"$work/build.prop"
for key in ro.lineage.version ro.lineage.display.version; do
  grep -qE "^$key=GammaOS_Next_(Full_)?v[0-9.]+$" "$work/build.prop" ||
    { echo "build.prop has no GammaOS Next $key" >&2; exit 1; }
done
sed -i -E \
  -e "s/^(ro\.lineage(\.display)?\.version)=GammaOS_Next_(Full_)?v[0-9.]+$/\1=$VERSION/" \
  -e "s/^ro\.build\.user=.*/ro.build.user=gammaos/" \
  -e "s/^ro\.build\.host=.*/ro.build.host=localhost/" \
  -e "/^ro\.thimsy\.version=/d" \
  "$work/build.prop"
echo "ro.thimsy.version=$THIMSY" >>"$work/build.prop"
write_through "$work/build.prop" "$PROP"
rm -rf "$work"
echo "version: $VERSION (ro.thimsy.version=$THIMSY)"
