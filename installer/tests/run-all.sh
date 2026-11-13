#!/usr/bin/env bash
# Every test of the installer: engine lint + unit/CLI tests, the real-firmware
# end-to-end test (when WORKSPACE and PACK_DIR are given), the maintainer
# tools' tests, and the app's analyzer + tests. Prints one summary line per
# step; when a step fails, its whole output, then stops.
# Usage: run-all.sh [WORKSPACE PACK_DIR]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

# step NAME SUMMARY_FILTER COMMAND...: runs COMMAND with its output in $LOG;
# on success prints SUMMARY_FILTER's view of it, on failure all of it.
step() {
  local name="$1" filter="$2"; shift 2
  if "$@" > "$LOG" 2>&1; then
    echo "$name: $(eval "$filter" < "$LOG")"
  else
    local rc=$?
    echo "== $name FAILED (exit $rc): $*" >&2
    cat "$LOG" >&2
    exit "$rc"
  fi
}
engine_summary='grep -E "^test result" | awk '"'"'{p+=$4; f+=$6} END {print p " passed, " f " failed"}'"'"''

cd "$HERE/../engine"
step "cargo fmt" 'echo ok' cargo fmt --check
step "clippy" 'echo ok' cargo clippy --locked --all-targets -q -- -D warnings
step "engine tests" "$engine_summary" cargo test --workspace --locked -q
step "engine tests (test hooks)" "$engine_summary" cargo test --workspace --locked --features test-hooks -q
# The engine as shipped: without the test-hooks feature.
step "engine build" 'echo ok' cargo build --release --locked -q
if [ $# -ge 2 ]; then
  rm -rf "$1/jobs"   # rebuild from the official firmware every time
  step "e2e" 'tail -1' "$HERE/e2e.sh" "$1" "$2"
fi
step "apply_pack tests" 'tail -1' python3 -I "$HERE/../tools/test_apply_pack.py"
cd "$HERE/../app"
step "flutter pub get" 'echo ok' flutter pub get --enforce-lockfile
step "flutter analyze" 'tail -1' flutter analyze --no-pub
step "flutter test" 'tail -1' flutter test --no-pub
