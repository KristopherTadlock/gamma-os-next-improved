#!/usr/bin/env bash
# Makes File Transfer (MTP) the default USB mode (Android's Developer options →
# Default USB configuration), so a computer sees the device's storage when it
# is plugged in. The device has no secure lock screen, so the default applies
# from boot and at every plug-in. Only a fresh setup sets it, so a choice made
# later survives setup re-running on a Nano/Android switch (1.2 has no
# FRESH_SETUP and runs setup once). svc needs init's mount namespace to start
# app_process.
#
# Usage: 25-usb-mtp.sh SYSTEM_DIR CACHE_DIR
set -euo pipefail

SYSTEM="$1"
source "$(dirname "$0")/../scripts/modlib.sh"
steps="$(mktemp)"
trap 'rm -f "$steps"' EXIT

cat >"$steps" <<'EOF'
if [ "${FRESH_SETUP:-1}" = 1 ]; then
    nsenter -t 1 -m -- svc usb setScreenUnlockedFunctions mtp
fi

EOF
add_setup_steps "$SYSTEM" "$steps"
echo "File Transfer (MTP) is the default USB mode"
