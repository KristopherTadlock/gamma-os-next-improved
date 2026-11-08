# Universal hotkeys

Applies to the GammaOS Next 1.4.4 build for the RG Cube (branch `nano-144`).

Status: implemented. The per-emulator bindings come from each emulator's source code and are not yet tested on a device (see [Testing](#testing)).

## What it does

Hold the **Anbernic button** in a game and press another button:

| Action | Default chord | Kind |
|---|---|---|
| Save state | Anbernic + R1 | one shot |
| Load state | Anbernic + L1 | one shot |
| Fast forward | Anbernic + R2 | toggle |
| Rewind | Anbernic + L2 | toggle |
| Quit game | Anbernic + Start, held 1 s | one shot |

- R is "forward" (save, fast forward) and L is "back" (load, rewind).
- Quit needs a 1 s hold so it can't happen by accident.
- The buttons can be changed under **Settings → Gamepad Settings → Hotkeys**.
- The Anbernic button on its own opens the Quick Menu when it is **released**, over a game and in Nano's own menus alike.

Each emulator has its own hotkeys, or none. A per-emulator **profile** turns the five actions into whatever that emulator understands. Adding an emulator means adding a profile file, with no code change.

## How it works

```
 pad --> PhoneWindowManager.interceptKeyBeforeQueueing
            |  chord detector (Anbernic held + button)
            v
         GammaHotkeyController (system_server)
            |  foreground package --> profile --> driver
            +--> inject a keyboard key (InputManager)
            +--> RetroArch network command (UDP 127.0.0.1:55355)
            +--> intent
            +--> the system exit (long-press-Back path) / force-stop

 Nano (startup and every return to the front)
            +--> profile "provision" rules: bind the keys in each emulator's settings files

 drastic-nano (Nano's DS player): its own chord detector, calling the player directly
```

### Chord detector (PhoneWindowManager)

It runs while a game is in front (`app_launched`, overlay down, not in a drastic-nano session, screen on):

1. **Anbernic down:** swallowed and armed. Nothing opens yet.
2. **A gamepad button or D-pad press while armed:**
   - it is a chord: the button runs its hotkey;
   - the button's repeats and key-up are swallowed, even if the Anbernic button is released first.

   Buttons with no hotkey are swallowed too, so the game never sees half a chord.
3. **Anbernic up:**
   - after a chord: nothing more;
   - alone: the overlay rises on the Quick Menu.

   A hold of 10 s or longer is the emergency recovery instead, and opens nothing.
4. **Quit:** the quit button must stay down for 1 s (*Hold to Quit*). Releasing it earlier cancels.
5. **Order of presses:** only Anbernic first, then the button, is a chord. A button already held when Anbernic goes down passes through untouched.
6. **A lost key-up:** if the Anbernic button's key-up goes missing, a later chord checks that it is physically down before acting.

Nano's own menus follow the same rule. The Quick Menu toggles on release, and a button pressed while the Anbernic button is held cancels it and is ignored.

### GammaHotkeyController (system_server)

`frameworks/base/services/core/java/com/android/server/policy/GammaHotkeyController.java`. It:

- finds the foreground package and its profile, falling back to the `"*"` profile;
- runs the action's driver;
- re-reads profiles when a file in either folder changes;
- keeps the toggle state for the app in front.

**Held keys never get stuck.** A `key_hold` key is released when:
- it is toggled off;
- another hold starts;
- the Quick Menu opens;
- quit runs;
- another app comes to the front;
- the game exits;
- the screen goes off.

There are no on-screen messages from the hotkeys. This build skips SystemUI, so system toasts can't show over a game. The emulators show their own messages: RetroArch, PPSSPP, Flycast and drastic-nano all report saves and loads.

### Drivers

| Driver | What it does |
|---|---|
| `key` | Taps a keyboard key (virtual keyboard, `SOURCE_KEYBOARD`). `"mods": ["CTRL", "SHIFT", "ALT"]` puts the modifiers on the event, and also presses and releases the modifier keys around it |
| `key_toggle` | Taps a key the emulator treats as a toggle |
| `key_hold` | Key down when toggled on, key up when toggled off |
| `retroarch_cmd` | Sends `"command"` (e.g. `SAVE_STATE`) as a UDP datagram to `"port"` (default 55355). RetroArch needs `network_cmd_enable = "true"` |
| `intent` | `"action"`, `"package"`, `"class"`, `"extras"`. A broadcast unless `"broadcast": false`, which starts an activity instead |
| `exit` | The system exit used by a long press of Back. RetroArch and DraStic get Esc and save before closing; drastic-nano gets its quit channel; standalone apps are stopped (unless *Keep Running in Background* is set) and Nano returns |
| `force_stop` | Stops the app and returns to Nano |
| `none` | Not available; logged |
| `native` | Only in the drastic-nano profile, which documents what Nano's DS player does itself |

Any action can add `"then": "<driver>"` with `"timeout_ms"`. If the app is still in front after the timeout, that driver runs. Graceful quits use it: the emulator's own quit key, then `exit`.

### Profiles

Profiles are JSON files, one per emulator:

- **Built-in:** `/system/etc/gammaos/hotkeys/<id>.json`, from `mods/hotkeys/` (installed by `mods/55-hotkeys.sh`).
- **User or override:** `/data/system/gammaos/hotkeys/<id>.json`. A file with the same `id` replaces the built-in one; a new id adds an emulator.
- **Fallback:** the profile with `"packages": ["*"]` (`fallback.json`) covers every other app. It supports quit only.

```json
{
  "id": "ppsspp",
  "name": "PPSSPP",
  "packages": ["org.ppsspp.ppsspp", "org.ppsspp.ppssppgold"],
  "note": "Shown in Settings > Hotkeys > Emulator Support.",
  "actions": {
    "save_state":   {"driver": "key", "key": "F2"},
    "load_state":   {"driver": "key", "key": "F4"},
    "fast_forward": {"driver": "key_hold", "key": "F6"},
    "rewind":       {"driver": "key", "key": "F7"},
    "quit":         {"driver": "exit"}
  },
  "provision": [
    {"file": "${sdcard}/PSP/SYSTEM/controls.ini", "format": "ini", "section": "ControlMapping",
     "mode": "append", "separator": ",",
     "values": {"Save State": "1-132,0-132", "Load State": "1-134,0-134"}}
  ]
}
```

- **Key names:** Android `KeyEvent` names without `KEYCODE_` (`F2`, `SPACE`, `R`).
- **Shared keys:** the profiles use F2 save, F4 load, F6 fast forward, F7 rewind and F8 quit, except RetroArch, which keeps its own defaults.

### Provisioning

Nano applies every profile's `provision` rules on a background thread when it starts and each time it comes back to the front, one run at a time. It skips emulators with a running process, which may write their settings back when they exit (so an emulator kept running in the background is bound once it has been closed), and leaves a file the emulator changed while Nano was editing it for the next run. Every launch starts from Nano, so the bindings are in place before the emulator reads them, and they come back if an emulator rewrites its file. With Universal Hotkeys off, nothing is provisioned.

**Rules:**
- Only files that already exist are edited (the emulator has run once). A rule with `"template"` creates the file from it when the emulator's folder exists.
- `${sdcard}` is `/storage/emulated/0`, which keeps file ownership right on shared storage. An edited file keeps its owner and mode; a file made from a template takes its folder's owner, mode 0660.
- A value can be a string or a number. Files are replaced through a temporary file that is synced first.

**Modes, for each value:**

| Mode | Behaviour |
|---|---|
| `add` (default) | Sets a missing or empty setting |
| `append` | Like `add`, and also adds the value to a setting bound to something else, joined with `"separator"`. Use it where the emulator takes several bindings for one action |
| `ifUnset` | Only sets a missing setting |
| `set` | Always sets it |

**Formats:**

| Format | File shape | Behaviour |
|---|---|---|
| `ini` | `[section]` then `key = value` (`"assign"` changes the ` = `) | — |
| `retroarch_cfg` | `key = "value"`, no sections | — |
| `flycast_mapping` | Flycast's `[digital]` `bindN = keycode:action` lines | Adds each `"binds"` entry as the next number, unless its keycode is already bound |
| `xml_prefs` | Android SharedPreferences | `"type"`: `int`, `long`, `boolean`, `string`, or `set` (a one-string set, only ever added). An `int` of 0 counts as unbound |
| `melonds_mapper` | melonDS for Android `controller_config.json` | `"binds"`: input name → keycode. Set only where the input has no assignment and the keycode is free |

The user's own bindings are never replaced in `add` mode: a hotkey whose setting the user bound to something else stays theirs.

### Settings in Nano

**Settings → Gamepad Settings → Hotkeys:**

| Row | Property | Default |
|---|---|---|
| Universal Hotkeys | `persist.gammaos.hotkeys.enable` | On |
| Save State Button | `persist.gammaos.hotkeys.save` | R1 (`BUTTON_R1`) |
| Load State Button | `persist.gammaos.hotkeys.load` | L1 |
| Fast Forward Button | `persist.gammaos.hotkeys.fastforward` | R2 |
| Rewind Button | `persist.gammaos.hotkeys.rewind` | L2 |
| Quit Game Button | `persist.gammaos.hotkeys.quit` | Start |
| Hold to Quit | `persist.gammaos.hotkeys.quithold` | On |
| Emulator Support | read-only: each profile, what works and its note | — |

- A button can be A, B, X, Y, L1, R1, L2, R2, L3, R3, Start, Select, a D-pad direction, or Off. A D-pad direction works only in drastic-nano: the D-pad reaches Android as a hat axis, which `PhoneWindowManager` never sees.
- Two hotkeys can't share a button: choosing a button another hotkey has swaps the two.

### drastic-nano (Nano's DS player)

drastic-nano reads the pad itself, so PhoneWindowManager leaves the Anbernic button to it. Its input loop has the same chord rule and reads the same properties:

| Action | What drastic-nano does |
|---|---|
| Save | Quick save to slot 0 |
| Load | Quick load from slot 0 |
| Fast forward | Its fast-forward toggle |
| Quit | Its exit, after the hold; it saves slot 9 for Quick Resume |
| Rewind | Nothing; it has no rewind |

A Menu binding on the Anbernic button doesn't fire after a chord.

Nano's libretro player only runs during the Quick Resume preview, so it has no hotkeys.

## Emulator support

Bindings come from each emulator's source code at the shipped version.

| Emulator | Save | Load | Fast forward | Rewind | Quit | How |
|---|---|---|---|---|---|---|
| RetroArch | F2 | F4 | Space (toggle) | R held | `exit` (Esc, auto-save) | `retroarch.cfg` keyboard binds. Keyboard hotkeys work without the Select hotkey-enable button. Rewind needs a core that can rewind, with Rewind on in RetroArch |
| PPSSPP | F2 | F4 | F6 held | F7, one snapshot per press | `exit` | `PSP/SYSTEM/controls.ini` `[ControlMapping]`, keyboard device `1-` and default device `0-` appended. Rewind needs `RewindSnapshotInterval` set in PPSSPP |
| Flycast | F2 | F4 | F6 (toggle) | — | `exit` | `mappings/Android_Virtual.cfg`: the injected keys come from the "Virtual" device, which Flycast maps like a pad |
| ARMSX2 | F2 | F4 | F6 (toggle) | — | F8 (save and exit), then `exit` | `shared_prefs/ARMSX2.xml` `pad.*.keycode`. Not if "Emulate USB Keyboard" is on |
| Azahar | F2 (quick slot) | F4 | F6 (turbo toggle) | — | F8 (close game), then `exit` | `shared_prefs` `InputMapping_HostAxis_<keycode>` sets. Azahar's own hotkey Enable button must be unset |
| WatermelonDS | F2 | F4 | F6 (toggle) | F7 opens its rewind screen | F8 (exit game), then `exit` | `files/controller_config.json`. Rewind needs Rewind on in its settings |
| ScummVM | — | — | Ctrl+F (SCUMM games) | — | Ctrl+Q, then `exit` | Built-in ScummVM keys |
| drastic-nano | slot 0 | slot 0 | toggle | — | saves, then exits | Built in |
| DraStic (app) | — | — | — | — | `exit` (Esc) | Its keymap is shared with drastic-nano, so it is left alone |
| Dolphin 2609a | — | — | — | — | `exit` | This build has no Android hotkeys; they arrived in later dev builds |
| M64Plus FZ | — | — | — | — | `exit` | Hotkeys live in private controller profiles and need long holds |
| hakuX | — | — | — | — | `exit` | Save states only from its in-game menu; no fast forward |
| Vita3K | — | — | — | — | `exit` | No save states or fast forward |
| Any other app | — | — | — | — | `exit` | `fallback.json` |

## Rewind

Rewind works on every system whose emulator or core can rewind; performance is not a concern. The hotkey never turns rewind on in an emulator's settings. If the user enabled it there, the button works; if not, the emulator ignores the key. RetroArch's `rewind_enable` stays as the user set it (off in the shipped config).

## Power button

Holding the power button for 7 s shuts the console down, in a game or in Nano. `PhoneWindowManager` handles it. Shutdown closes the game first where it can: drastic-nano saves slot 9, and Quick Resume is prepared. drastic-nano's own 5 s power-off was removed so the two don't race.

## Adding an emulator

1. Find its hotkey settings in the source: which actions exist, where they are stored, the format, and whether key events from the virtual keyboard (device id -1) are accepted.
2. Write a profile:
   - `actions` sends unused keys;
   - `provision` binds those keys in the emulator's file;
   - `note` says anything a user should know.
3. Copy it to `/data/system/gammaos/hotkeys/`. The framework picks it up on the next hotkey; Nano binds the keys the next time it comes to the front. Then test it.
4. Ship it in `mods/hotkeys/`.

## Testing

On the device, for each emulator:

1. **Check the bindings:** start a game, return to Nano, then check that the emulator's file gained the bindings (`adb shell cat <file>`).
2. **Check each action:** try each chord. To tell a binding problem from a chord problem, inject the key directly:
   - a tap: `adb shell input keyevent KEYCODE_F2`;
   - a hold: `adb shell input keyevent --longpress KEYCODE_R`.
3. **Watch the log:** `adb logcat -s GammaHotkeys` shows each action, the profile and the driver, and Nano logs `hotkeys: bound keys in <file>`.
4. **Check the edge cases:**
   - Anbernic alone opens the Quick Menu on release;
   - Anbernic + an unmapped button does nothing;
   - the 10 s emergency hold still recovers;
   - rewind released when the Quick Menu opens.
