#!/usr/bin/env bash
# End-to-end test of the engine on real RG Cube firmware (no device needed):
# rebuilds the reference Thimsy PAC from the official GammaOS download plus the
# device pack and checks it byte for byte, then flashes it through a fake
# spd_dump. Usage: e2e.sh WORKSPACE PACK_DIR
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ENGINE="${ENGINE:-$HERE/../engine/target/release/thimsy-installer}"
WS="$1"; PACK="$2"
JOB="$WS/jobs/anbernic-rg-cube-heavy"
pass() { echo "PASS $*"; }
fail() { echo "FAIL $*"; exit 1; }

# 1. Space check before building.
"$ENGINE" --json space --device anbernic-rg-cube --profile heavy --workspace "$WS" | grep -q '"ok":true' && pass "space check" || fail "space check"

# 2. Build and compare with the reference.
"$ENGINE" --json build --device anbernic-rg-cube --profile heavy --workspace "$WS" --import "$PACK" > "$WS/build.jsonl"
tail -1 "$WS/build.jsonl" | grep -q '"matches_reference":true' && pass "build matches reference byte for byte" || fail "build"
grep -q '"type":"progress"' "$WS/build.jsonl" && pass "progress events" || fail "progress events"

# 3. Flash dry runs (full and update).
"$ENGINE" --json flash --job "$JOB" --dry-run > "$WS/dry.jsonl"
grep -q 'write_part super super.img' "$WS/dry.jsonl" && pass "full dry run writes super" || fail "full dry run"
grep -q 'write_part prodnv' "$WS/dry.jsonl" && fail "prodnv must never be written" || pass "prodnv not written"
grep -q 'read_part prodnv' "$WS/dry.jsonl" && pass "prodnv backed up" || fail "prodnv backup"
"$ENGINE" --json flash --job "$JOB" --dry-run --mode update > "$WS/dry-update.jsonl"
n=$(grep -o 'write_part [a-z_]*' "$WS/dry-update.jsonl" | sort -u | wc -l)
[ "$n" = 4 ] && pass "update writes 4 partitions" || fail "update wrote $n partitions"
grep -q 'erase_part' "$WS/dry-update.jsonl" && fail "update must not erase" || pass "update keeps data"

# 4. Flash through the fake spd_dump.
export FAKE_SPD_LOG="$WS/spd.log"; rm -f "$FAKE_SPD_LOG"
THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/flash.jsonl"
tail -1 "$WS/flash.jsonl" | grep -q '"ok":true' && pass "flash (fake spd_dump)" || fail "flash"
grep -q '"type":"prompt","id":"enter_download_mode"' "$WS/flash.jsonl" && pass "download-mode prompt" || fail "prompt"
[ "$(grep -c '^write_part' "$FAKE_SPD_LOG")" -ge 50 ] && pass "all partitions written" || fail "write count"
grep -q '^write_part super .* 5872025600$' "$FAKE_SPD_LOG" && pass "super written at full size" || fail "super size"
[ -f "$JOB/backup/prodnv.img" ] && pass "NV backup kept with the job" || fail "backup"
[ ! -d "$JOB/flash" ] && pass "flash scratch cleaned up" || fail "scratch left behind"

# 5. Host checks.
"$ENGINE" --host android --json flash --job "$JOB" > "$WS/android.jsonl" 2>&1 && fail "android flash should fail" || true
grep -q '"code":"unsupported_host"' "$WS/android.jsonl" && pass "android: unsupported_host" || fail "android host"
"$ENGINE" --host macos --json resolve --device anbernic-rg557 --profile heavy | grep -q 'Windows PC' && pass "macOS: needs Windows message" || fail "needs windows"

# 6. Clean (dry run lists items; image kept).
"$ENGINE" --json clean --workspace "$WS" --dry-run | grep -q '"kind":"image"' && pass "clean lists the image" || fail "clean"
echo "ALL PASS"
