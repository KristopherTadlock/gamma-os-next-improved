#!/usr/bin/env bash
# Every test of the installer: engine lint + unit/CLI tests, the real-firmware
# end-to-end test (when WORKSPACE and PACK_DIR are given), and the app's
# analyzer + tests. Usage: run-all.sh [WORKSPACE PACK_DIR]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE/../engine"
cargo fmt --check
cargo clippy --all-targets -q -- -D warnings
cargo test --workspace -q 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6} END {print "engine tests: " p " passed, " f " failed"; exit f>0}'
cargo build --release -q
if [ $# -ge 2 ]; then
  rm -rf "$1/jobs"   # rebuild from the official firmware every time
  "$HERE/e2e.sh" "$1" "$2" | tail -1
fi
cd "$HERE/../app"
flutter analyze --no-pub 2>&1 | tail -1
flutter test --no-pub 2>&1 | tail -1
