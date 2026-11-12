# Engine protocol (app ⇄ `thimsy-installer`)

The Flutter app never talks to devices or parses firmware itself. It runs the
bundled engine binary, `thimsy-installer` (`thimsy-installer.exe` on Windows),
which sits next to the app executable (`<app dir>/engine/thimsy-installer`),
and reads **JSON Lines** from its stdout. One JSON object per line. Stderr is
free-form log text for the diagnostic bundle. Exit code 0 = success.

Every command accepts `--json`. Without it, output is human-readable.
Every command accepts `--lang CODE` (Thimsy's codes: `en ar da de el es fi fr
he it ja ko nb nl pl pt ru sv th tr zh-cn zh-tw`; Flutter forms such as
`zh_TW` are accepted). Stage titles, prompts, hints, guide steps, profile
text and the "use Windows" message come back translated; anything without a
translation stays English.

## Common event objects (streaming commands)

```json
{"type":"stage","stage":"fetch","title":"Downloading"}
{"type":"progress","stage":"fetch","label":"RG_Cube_GammaOS_Next_Full_v1.2.7z.001","done":123,"total":456}
{"type":"log","level":"info","message":"…"}
{"type":"prompt","id":"enter_download_mode","title":"…","body":"…","image":null}
{"type":"error","code":"device_not_found","message":"…","hint":"…"}
{"type":"result","ok":true, "...": "command-specific fields"}
```

- `done`/`total` are bytes (or steps when the label says so); `total` may be 0
  when unknown.
- `prompt` asks the user to do something physical (hold a key, plug in). The
  engine keeps going on its own once it sees the device; the app just shows the
  prompt until the next `stage`/`result`.
- The last line is always `result` (with `ok:false` after an `error`).

## Commands

### `catalog --json`
One line:
```json
{"type":"catalog","version":"0.1.0","devices":[{
  "id":"anbernic-rg-cube","name":"Anbernic RG Cube","maker":"Anbernic",
  "soc":"Unisoc T820","display":"720×720","status":"testing",
  "support":{"build":"verified","boot":"qualified",
             "install":{"windows":"automated","linux":"automated","macos":"untested","android":"none"}},
  "notes":["…"],"blocks":["…"]}],
 "profiles":[{"id":"light","name":"Light","summary":"…","available":false},
             {"id":"medium","name":"Medium","summary":"…","available":true},
             {"id":"heavy","name":"Heavy","summary":"…","available":false}],
 "host":"linux"}
```
`install` values: `automated`, `guided`, `untested`, `none`.

### `resolve --device ID --profile P [--set KEY=VALUE]... --json`
One line:
```json
{"type":"plan","device":"anbernic-rg-cube","profile":"medium","available":true,
 "unavailable_reason":null,
 "emulators":[{"id":"ppsspp","name":"PPSSPP","systems":"PSP","included":true,"reason":null}],
 "cores":[{"id":"mgba","system":"Game Boy Advance","default":true}],
 "apps":[{"id":"brave","name":"Brave","included":true,"reason":null}],
 "excluded":[{"id":"armsx2","name":"ARMSX2","reason":"needs ARMv8.2"}],
 "settings":[{"key":"nano.theme","value":"esde","source":"profile","reason":null}],
 "download_bytes":4800000000,"disk_bytes":19000000000,
 "partitions":["super","vendor_boot_a","vendor_boot_b","logo"],
 "wipes_data":true,
 "host_message":null,
 "journeys":{
   "first_time":{"title":"First time installing custom firmware","body":"…","unlock":true,"modes":["full"],"update_note":null},
   "returning":{"title":"Already unlocked: install new firmware","body":"…","unlock":false,"modes":["update","full"],
                "update_note":"Keeps your data. Only when …"}},
 "guides":{
   "backup":[{"title":"…","body":"…"}],
   "unlock":{"route":"automated|guided|none","steps":[{"title":"…","body":"…","link":null}]},
   "download_mode":[{"title":"…","body":"…"}],
   "first_boot":[{"title":"…","body":"…"}]}}
```

`host_message` is set when this computer can't install the device ("… Run
Thimsy Installer on a Windows PC to install it."); show it and offer only
build-and-export.

`journeys`: ask whether the device has had custom firmware before. First time
→ show the unlock step and flash with `--mode full`. Returning → skip
unlocking; offer `--mode update` (keeps data) and `--mode full`.

### `space --device ID --profile P --workspace DIR [--delete-downloads] --json`
Run before `build`; don't start the build when `ok` is false.
```json
{"type":"space","need":13114372066,"free":78826295296,"ok":true,"build_need":13114372066,"flash_need":12875989004,"cached":0,"downloads":4767661249}
```

### `clean --workspace DIR [--downloads] [--image] [--scratch] [--backup] [--dry-run] --json`
```json
{"type":"result","ok":true,"items":[{"kind":"downloads","path":"…","bytes":524288000}],"freed":0}
```
`backup` holds the device's NV/calibration backup; keep it unless the user insists.

### `build --device ID --profile P --workspace DIR [--import DIR] [--set K=V]... [--delete-downloads] --json`
Streams stages `plan`, `fetch`, `extract`, `assemble`, `validate`, `publish`.
Final:
```json
{"type":"result","ok":true,"job":"DIR/jobs/anbernic-rg-cube-medium","image":"DIR/jobs/…/out/RG_Cube_Thimsy_medium.pac","bytes":7722216910}
```
`--import DIR`: look in DIR for any needed file (matched by name and sha256)
before downloading.

### `detect --json [--watch]`
```json
{"type":"devices","devices":[{"vidpid":"1782:4d00","mode":"download","device":"anbernic-rg-cube","label":"Unisoc download mode"}]}
```
With `--watch`, a new line is printed whenever the list changes; runs until
killed.

### `flash --job DIR [--mode full|update] [--dry-run] --json`
`full` (default): fresh install, every flashable partition, user data erased;
NV/calibration partitions are backed up into the job and never written.
`update`: only the partitions Thimsy changes; data kept.
Streams stages `check`, `driver`, `prepare`, `wait_device`, `flash`, `reboot`,
with `prompt` events for physical steps. `--dry-run` prints every tool command
as `log` events and touches no device. Final:
```json
{"type":"result","ok":true,"backup":"DIR/…/backup"}
```

### `unlock --device ID --json`
For devices whose unlock is `automated` (fastboot `flashing unlock` with
on-device confirmation). For `guided` devices it returns
`{"type":"result","ok":false,...}` with code `unlock_guided`; the app shows
`guides.unlock` instead.

### `drivers --device ID [--install] --json`
```json
{"type":"result","ok":true,"status":"ok|missing|not_required","detail":"…"}
```
`--install` (Windows) installs WinUSB for the device's download-mode IDs; one
UAC prompt.

## Error codes
`device_not_found`, `wrong_device`, `download_failed`, `hash_mismatch`,
`not_enough_space`, `tool_missing`, `tool_failed`, `unsupported_host`,
`profile_unavailable`, `unlock_guided`, `cancelled`, `internal`.
