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

# 3. Flash dry runs (full and update). --mode is required.
"$ENGINE" --json flash --job "$JOB" --dry-run > /dev/null 2>&1 && fail "flash without --mode must fail" || pass "--mode required"
"$ENGINE" --json flash --job "$JOB" --dry-run --mode full > "$WS/dry.jsonl"
grep -q 'write_part super super.img' "$WS/dry.jsonl" && pass "full dry run writes super" || fail "full dry run"
grep -q 'write_part prodnv' "$WS/dry.jsonl" && fail "prodnv must never be written" || pass "prodnv not written"
grep -q 'read_part prodnv 0 0x4000000 ' "$WS/dry.jsonl" && pass "prodnv backed up at its full size" || fail "prodnv backup"
grep -q 'partition_list device-partitions.xml read_part' "$WS/dry.jsonl" && pass "device partition table read first" || fail "partition_list"
"$ENGINE" --json flash --job "$JOB" --dry-run --mode update > "$WS/dry-update.jsonl"
n=$(grep -o 'write_part [a-z_]*' "$WS/dry-update.jsonl" | sort -u | wc -l)
[ "$n" = 5 ] && pass "update writes 5 partitions" || fail "update wrote $n partitions"
grep -q 'erase_part' "$WS/dry-update.jsonl" && fail "update must not erase" || pass "update keeps data"

# 4. Flash through the fake spd_dump. Leftovers of an older image in the
# scratch folder (same size, stale stamp) must be thrown away, not flashed.
SUPER_BYTES=5872025600
mkdir -p "$JOB/flash"
echo 0000 > "$JOB/flash/.image-sha256"
truncate -s "$SUPER_BYTES" "$JOB/flash/super.img"
export FAKE_SPD_LOG="$WS/spd.log"; rm -f "$FAKE_SPD_LOG"
FAKE_SPD_HASH=1 THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/flash.jsonl"
tail -1 "$WS/flash.jsonl" | grep -q '"ok":true' && pass "flash (fake spd_dump)" || fail "flash"
grep -q '"type":"prompt","id":"enter_download_mode"' "$WS/flash.jsonl" && pass "download-mode prompt" || fail "prompt"
grep -q '"enter_download_mode".*Back button' "$WS/flash.jsonl" && pass "prompt says hold Back" || fail "prompt wording"
grep -q '"label":"Checking the image"' "$WS/flash.jsonl" && pass "image checked against the build" || fail "image check"
grep -q "partition table matches this firmware" "$WS/flash.jsonl" && pass "device partition table checked" || fail "device check"
# The prompt stays up while spd_dump waits: the install stage starts only
# once the device answers (spd_dump logs that on stderr).
p=$(grep -n '"id":"enter_download_mode"' "$WS/flash.jsonl" | head -1 | cut -d: -f1)
v=$(grep -n 'BSL_REP_VER' "$WS/flash.jsonl" | head -1 | cut -d: -f1)
f=$(grep -n '"type":"stage","stage":"flash"' "$WS/flash.jsonl" | head -1 | cut -d: -f1)
[ -n "$p" ] && [ -n "$v" ] && [ -n "$f" ] && [ "$p" -lt "$v" ] && [ "$v" -lt "$f" ] \
  && pass "install stage starts when the device answers" || fail "stage order (prompt $p, device $v, flash $f)"
python3 -I -c "
import json, sys
ev = [json.loads(l) for l in open(sys.argv[1]) if l.startswith('{')]
w = [e for e in ev if e.get('type') == 'progress' and e.get('label', '').startswith('Wrote')]
sys.exit(0 if w and w[-1]['done'] == w[-1]['total'] else 1)" "$WS/flash.jsonl" \
  && pass "write progress reaches 100%" || fail "write progress"
[ "$(grep -c '^write_part' "$FAKE_SPD_LOG")" -ge 50 ] && pass "all partitions written" || fail "write count"
grep -q "^write_part super .* $SUPER_BYTES\$" "$FAKE_SPD_LOG" && pass "super written at full size" || fail "super size"
want=$(python3 -I -c "
import hashlib, json, struct, sys
image = json.load(open(sys.argv[1]))['image']
f = open(image, 'rb'); h = f.read(2124)
count, start = struct.unpack_from('<II', h, 1076)
f.seek(start); t = f.read(count * 2580)
for i in range(count):
    e = t[i * 2580:(i + 1) * 2580]
    if e[4:516].decode('utf-16-le').split('\0')[0] == 'Super':
        size = struct.unpack_from('<I', e, 1540)[0] | (struct.unpack_from('<I', e, 1532)[0] << 32)
        off = struct.unpack_from('<I', e, 1552)[0] | (struct.unpack_from('<I', e, 1536)[0] << 32)
        f.seek(off); d = hashlib.sha256()
        while size:
            b = f.read(min(size, 1 << 22)); d.update(b); size -= len(b)
        print(d.hexdigest()); break" "$JOB/job.json")
grep -q "^sha256 super $want\$" "$FAKE_SPD_LOG" && pass "leftover same-size super.img replaced by this image's" || fail "stale super.img was flashed"
RUN1=$(tail -1 "$WS/flash.jsonl" | python3 -I -c "import json, sys; print(json.load(sys.stdin)['backup'])")
case "$RUN1" in "$JOB/backup/run-"*) pass "backups read into their own run folder";; *) fail "backup folder $RUN1";; esac
[ "$(stat -c %s "$RUN1/prodnv.img")" = 67108864 ] && pass "prodnv backup is the whole 64 MiB partition" || fail "prodnv backup size"
[ "$(stat -c %s "$RUN1/miscdata.img")" = 1048576 ] && pass "miscdata backup is the whole 1 MiB partition" || fail "miscdata backup size"
[ ! -d "$JOB/flash" ] && pass "flash scratch cleaned up" || fail "scratch left behind"
# Backup runs that read something (a run cut short before it reads stays empty).
runs() { ls "$JOB"/backup/run-*/prodnv.img* 2>/dev/null | wc -l; }

# 5. A device whose partition table differs (another model on the same USB
# ID, or a stock layout): refused before the first write, nothing written.
cat > "$WS/other-device.xml" <<'XML'
<Partitions>
    <Partition id="prodnv" size="64"/>
    <Partition id="miscdata" size="1"/>
    <Partition id="super" size="4096"/>
    <Partition id="userdata" size="0xffffffff"/>
</Partitions>
XML
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_PARTS="$WS/other-device.xml" THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/wrong.jsonl" \
  && fail "a different partition table must be refused" || true
grep -q '"code":"wrong_device"' "$WS/wrong.jsonl" && pass "different partition table: wrong_device" || fail "wrong_device code"
grep -q 'super is 4096 MiB on the device' "$WS/wrong.jsonl" && pass "wrong_device names the difference" || fail "wrong_device message"
grep -q '^read_part prodnv' "$FAKE_SPD_LOG" && ! grep -q '^write_part\|^erase_part' "$FAKE_SPD_LOG" \
  && pass "nothing written or erased on the wrong device" || fail "wrong device was written"
[ "$(runs)" = 2 ] && [ "$(stat -c %s "$RUN1/prodnv.img")" = 67108864 ] \
  && [ "$(stat -c %s "$(ls "$JOB"/backup/run-*/prodnv.img | grep -v "^$RUN1/")")" = 67108864 ] \
  && pass "its backup kept beside the first, not over it" || fail "backups after wrong device"

# 5b. The device stops answering halfway through a partition: spd_dump logs
# it and would carry on (exit 0); the flash stops there and fails.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_SHORT_WRITE=logo THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/short-write.jsonl" \
  && fail "a partial write must fail the flash" || true
grep -q '"code":"tool_failed"' "$WS/short-write.jsonl" && grep -q 'unexpected response' "$WS/short-write.jsonl" \
  && pass "partial write: tool_failed" || fail "partial write code"
grep -q '^write_part logo .* short$' "$FAKE_SPD_LOG" && [ "$(grep '^write_part' "$FAKE_SPD_LOG" | tail -1 | cut -d' ' -f2)" = logo ] \
  && ! grep -q '^erase_part' "$FAKE_SPD_LOG" && pass "nothing written after the partial write" || fail "kept writing after a partial write"

# 5c. An spd_dump that writes without asking (old or forked): stopped at its
# first write, never treated as a checked flash.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_NO_PROMPT=1 THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/no-prompt.jsonl" \
  && fail "a tool that never asks must fail" || true
grep -q '"code":"tool_failed"' "$WS/no-prompt.jsonl" && grep -q 'without asking first' "$WS/no-prompt.jsonl" \
  && pass "tool that never asks: tool_failed" || fail "no-prompt code"
[ "$(grep -c '^write_part' "$FAKE_SPD_LOG")" -le 1 ] && ! grep -q '^erase_part' "$FAKE_SPD_LOG" \
  && pass "tool that never asks stopped at its first write" || fail "no-prompt tool kept writing"

# 5d. A backup read short: nothing is confirmed; the file is kept, marked
# incomplete, and the earlier backups are untouched.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_SHORT_READ=prodnv THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/short-read.jsonl" \
  && fail "an incomplete backup must stop the flash" || true
grep -q 'backup of prodnv is incomplete' "$WS/short-read.jsonl" && pass "short backup refused" || fail "short backup message"
! grep -q '^write_part\|^erase_part' "$FAKE_SPD_LOG" && pass "nothing written with an incomplete backup" || fail "written with an incomplete backup"
[ "$(runs)" = 5 ] && [ "$(stat -c %s "$JOB"/backup/run-*/prodnv.img.incomplete)" = 33554432 ] \
  && [ "$(stat -c %s "$RUN1/prodnv.img")" = 67108864 ] && pass "incomplete backup kept as .incomplete" || fail "incomplete backup files"

# 6. The engine killed while spd_dump waits for the device (what the app's
# Cancel does): spd_dump dies with it and nothing is written. The "device"
# connects after 8 s: the log is read well after that, so a surviving
# spd_dump would have had time to write.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_WAIT=8 THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/killed.jsonl" &
engine=$!
for _ in $(seq 1 600); do grep -q 'Waiting for connection' "$WS/killed.jsonl" 2>/dev/null && break; sleep 0.5; done
pgrep -f "$HERE/fake-tools/spd_dump" > /dev/null && pass "spd_dump waiting for the device" || fail "spd_dump never started"
kill -TERM "$engine"; wait "$engine" 2>/dev/null || true
sleep 1
pgrep -f "$HERE/fake-tools/spd_dump" > /dev/null && { pkill -f "$HERE/fake-tools/spd_dump"; fail "spd_dump outlived the engine"; } || pass "spd_dump died with the engine"
sleep 12
[ -f "$FAKE_SPD_LOG" ] && ! grep -q '^fdl\|^write_part\|^erase_part' "$FAKE_SPD_LOG" \
  && pass "nothing written after the engine died (checked past the device's connect time)" || fail "written after the engine died"

# 7. Host checks.
"$ENGINE" --host android --json flash --job "$JOB" --mode full > "$WS/android.jsonl" 2>&1 && fail "android flash should fail" || true
grep -q '"code":"unsupported_host"' "$WS/android.jsonl" && pass "android: unsupported_host" || fail "android host"
"$ENGINE" --host macos --json resolve --device anbernic-rg557 --profile heavy | grep -q 'Windows PC' && pass "macOS: needs Windows message" || fail "needs windows"

# 8. Clean (dry run lists items; image kept). Scratch cleaning never touches
# backups.
"$ENGINE" --json clean --workspace "$WS" --dry-run | grep -q '"kind":"image"' && pass "clean lists the image" || fail "clean"
"$ENGINE" --json clean --workspace "$WS" --scratch > "$WS/clean.jsonl"
n=$(runs)
[ ! -d "$JOB/flash" ] && [ "$n" = 5 ] && [ "$(stat -c %s "$RUN1/prodnv.img")" = 67108864 ] \
  && pass "clean --scratch keeps every backup" || fail "clean --scratch and backups ($n runs)"
echo "ALL PASS"
