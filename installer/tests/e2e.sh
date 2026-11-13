#!/usr/bin/env bash
# End-to-end test of the engine on real RG Cube firmware (no device needed):
# rebuilds the reference Thimsy PAC from the official GammaOS download plus the
# device pack and checks it byte for byte, then flashes it through a fake
# spd_dump. Usage: e2e.sh WORKSPACE PACK_DIR
#
# The fake spd_dump is found through THIMSY_TOOLS_DIR, which only engines
# built with the `test-hooks` cargo feature honour (release artifacts never
# are). So this script builds its own engine with that feature, in its own
# target folder (engine/target/test-hooks), unless ENGINE names one that was.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
if [ -z "${ENGINE:-}" ]; then
  (cd "$HERE/../engine" && cargo build -q --release --features test-hooks --target-dir target/test-hooks -p thimsy-installer)
  ENGINE="$HERE/../engine/target/test-hooks/release/thimsy-installer"
fi
WS="$1"; PACK="$2"
JOB="$WS/jobs/anbernic-rg-cube-heavy"
pass() { echo "PASS $*"; }
fail() { echo "FAIL $*"; exit 1; }

# 1. Space check before building; the plan's disk figure is the same one.
"$ENGINE" --json space --device anbernic-rg-cube --profile heavy --workspace "$WS" > "$WS/space.jsonl"
grep -q '"ok":true' "$WS/space.jsonl" && pass "space check" || fail "space check"
"$ENGINE" --json resolve --device anbernic-rg-cube --profile heavy > "$WS/plan.jsonl"
python3 -I -c "
import json, sys
s = json.load(open(sys.argv[1])); p = json.load(open(sys.argv[2]))
sys.exit(0 if s['need'] == p['disk_bytes'] and s['downloads'] == p['download_bytes'] else 1)" "$WS/space.jsonl" "$WS/plan.jsonl" \
  && pass "plan and space agree on the disk needed" || fail "plan disk_bytes differs from space need"
"$ENGINE" --json space --device anbernic-rg-cube --profile heavy --workspace "$WS" --import "$PACK" \
  | python3 -I -c "import json, sys; s = json.load(sys.stdin); sys.exit(0 if s['imported'] == s['downloads'] and s['ok'] else 1)" \
  && pass "space counts the picked files as not downloaded" || fail "space --import"

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
grep -q 'partition_list ../backup/run-[0-9-]*/device-partitions.xml read_part' "$WS/dry.jsonl" && pass "device partition table read first, into the run's backup folder" || fail "partition_list"
python3 -I -c "
import json, re, sys
plan = json.load(open(sys.argv[1]))
dry = [json.loads(l) for l in open(sys.argv[2]) if l.startswith('{')][-1]['commands'][0]
writes = re.findall(r'write_part (\S+)', dry)
erases = re.findall(r'erase_part (\S+)', dry)
sys.exit(0 if writes == plan['partitions_full'] and erases == plan['erases_full'] else 1)" "$WS/plan.jsonl" "$WS/dry.jsonl" \
  && pass "the plan lists exactly what a full install writes and erases" || fail "partitions_full/erases_full differ from the dry run"
"$ENGINE" --json flash --job "$JOB" --dry-run --mode update > "$WS/dry-update.jsonl"
n=$(grep -o 'write_part [a-z_]*' "$WS/dry-update.jsonl" | sort -u | wc -l)
[ "$n" = 5 ] && pass "update writes 5 partitions" || fail "update wrote $n partitions"
grep -q 'erase_part' "$WS/dry-update.jsonl" && fail "update must not erase" || pass "update keeps data"
for p in boot_a boot_b init_boot_a dtbo_b vbmeta_a vbmeta_system_b; do
  grep -q "read_part $p 0 0x[0-9a-f]* ../backup/run-[0-9-]*/check-$p.img" "$WS/dry-update.jsonl" || fail "update doesn't read back $p"
done
pass "update reads back the base it relies on (boot, init_boot, dtb, dtbo, vbmeta*)"

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

# 5e. An unpacked partition changed after unpacking (same size, the stamp
# still says this image): it's unpacked again, and the device gets the
# image's own bytes.
[ -f "$JOB/flash/super.img" ] && grep -q "$(python3 -I -c "import json, sys; print(json.load(open(sys.argv[1]))['sha256'])" "$JOB/job.json")" "$JOB/flash/.image-sha256" \
  || fail "expected this image's unpacked partitions in flash/"
python3 -I -c "
import sys
with open(sys.argv[1], 'r+b') as f:
    f.seek(4096); b = f.read(1); f.seek(4096); f.write(bytes([b[0] ^ 0xFF]))" "$JOB/flash/super.img"
[ "$(stat -c %s "$JOB/flash/super.img")" = "$SUPER_BYTES" ] || fail "super.img changed size"
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_HASH=1 THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/damaged.jsonl"
tail -1 "$WS/damaged.jsonl" | grep -q '"ok":true' && grep -q "super.img doesn't match the image; unpacking it again" "$WS/damaged.jsonl" \
  && pass "damaged unpacked partition detected and unpacked again" || fail "damaged partition not caught"
grep -q "^sha256 super $want\$" "$FAKE_SPD_LOG" && pass "flashed super equals the PAC entry after the repair" || fail "damaged super.img was flashed"

# 5f. Update mode on a device running the matching base: read back,
# compared, then only the update's partitions are written.
rm -f "$FAKE_SPD_LOG"
THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode update > "$WS/update.jsonl"
tail -1 "$WS/update.jsonl" | grep -q '"ok":true' && grep -q "runs the base this update needs" "$WS/update.jsonl" \
  && pass "update on a matching base" || fail "update on a matching base"
[ "$(grep '^write_part' "$FAKE_SPD_LOG" | cut -d' ' -f2 | sort | tr '\n' ' ')" = "fbootlogo logo super vendor_boot_a vendor_boot_b " ] \
  && ! grep -q '^erase_part' "$FAKE_SPD_LOG" && pass "update wrote only its 5 partitions" || fail "update writes"
RUNU=$(tail -1 "$WS/update.jsonl" | python3 -I -c "import json, sys; print(json.load(sys.stdin)['backup'])")
[ -f "$RUNU/device.json" ] && [ ! -e "$RUNU/check-boot_a.img" ] && python3 -I -c "
import hashlib, json, sys
d = json.load(open(sys.argv[1] + '/device.json'))
t = hashlib.sha256(open(sys.argv[1] + '/device-partitions.xml', 'rb').read()).hexdigest()
sys.exit(0 if d['device'] == 'anbernic-rg-cube' and d['partition_table_sha256'] == t and d['mode'] == 'update' else 1)" "$RUNU" \
  && pass "run folder records the device and its table; base images not kept" || fail "device.json"

# 5g. Update mode on a device running another base: refused before the
# first write, nothing written or erased.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_UPDATE_MISMATCH=dtbo_b THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode update > "$WS/update-mismatch.jsonl" \
  && fail "an update on another base must be refused" || true
grep -q '"code":"full_install_needed"' "$WS/update-mismatch.jsonl" && grep -q 'dtbo_b' "$WS/update-mismatch.jsonl" \
  && pass "another base: full_install_needed naming the partition" || fail "full_install_needed"
grep -q '^read_part dtbo_b' "$FAKE_SPD_LOG" && ! grep -q '^write_part\|^erase_part' "$FAKE_SPD_LOG" \
  && pass "nothing written or erased on another base" || fail "written on another base"

# 5h. The device never shows up in download mode: device_not_found, and
# not reported as a stopped flash.
rm -f "$FAKE_SPD_LOG"
FAKE_SPD_NO_DEVICE=1 THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/no-device.jsonl" \
  && fail "a missing device must fail" || true
grep -q '"code":"device_not_found"' "$WS/no-device.jsonl" && ! grep -q 'still recoverable' "$WS/no-device.jsonl" \
  && pass "device never answered: device_not_found" || fail "device_not_found"

# 5i. Another run holds the workspace: refused with busy, nothing done.
rm -f "$FAKE_SPD_LOG"
flock "$WS/.thimsy.lock" env THIMSY_TOOLS_DIR="$HERE/fake-tools" "$ENGINE" --json flash --job "$JOB" --mode full > "$WS/busy.jsonl" \
  && fail "a held workspace must be refused" || true
grep -q '"code":"busy"' "$WS/busy.jsonl" && [ ! -f "$FAKE_SPD_LOG" ] && pass "workspace in use: busy, spd_dump never ran" || fail "busy"
flock "$WS/.thimsy.lock" "$ENGINE" --json clean --workspace "$WS" --scratch > "$WS/busy-clean.jsonl" && fail "clean must wait for the lock" || true
grep -q '"code":"busy"' "$WS/busy-clean.jsonl" && [ -d "$JOB/flash" ] && pass "clean refused while the workspace is in use" || fail "busy clean"

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
[ ! -d "$JOB/flash" ] && [ "$n" = 8 ] && [ "$(stat -c %s "$RUN1/prodnv.img")" = 67108864 ] \
  && pass "clean --scratch keeps every backup" || fail "clean --scratch and backups ($n runs)"
# Backups are listed run by run (with what device.json says) and can be
# removed one at a time by path.
"$ENGINE" --json clean --workspace "$WS" --dry-run > "$WS/clean-list.jsonl"
python3 -I -c "
import json, sys
items = json.load(open(sys.argv[1]))['items']
runs = [i for i in items if i['kind'] == 'backup']
sys.exit(0 if len(runs) == 8 and any(i.get('info', {}).get('device') == 'anbernic-rg-cube' for i in runs) else 1)" "$WS/clean-list.jsonl" \
  && pass "clean lists each backup run" || fail "clean backup items"
"$ENGINE" --json clean --workspace "$WS" --path "$RUNU" > /dev/null && [ ! -d "$RUNU" ] && [ -f "$RUN1/prodnv.img" ] \
  && pass "clean --path removes one run only" || fail "clean --path"
"$ENGINE" --json clean --workspace "$WS" --path "$WS/space.jsonl" > "$WS/clean-bad.jsonl" && fail "clean --path must refuse other files" || true
grep -q '"code":"bad_input"' "$WS/clean-bad.jsonl" && [ -f "$WS/space.jsonl" ] && pass "clean --path refuses what it doesn't own" || fail "clean --path other file"
echo "ALL PASS"
