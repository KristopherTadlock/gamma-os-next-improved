# Universal hotkeys

Applies to Thimsy OS built from GammaOS Next 1.4.4 source for the RG Cube (branch `nano-144`).

Status: implemented. The per-emulator bindings come from each emulator's source code and are not yet tested on a device (see [Testing](#testing)).

## What it does

Hold the **Anbernic button**, or **Start + Select**, in a game and press another button:

| Action | Default chord | Kind |
|---|---|---|
| Save state | Anbernic + R1, or Start + Select + R1 | one shot |
| Load state | Anbernic + L1, or Start + Select + L1 | one shot |
| Fast forward | Anbernic + R2, or Start + Select + R2 | toggle |
| Rewind | Anbernic + L2, or Start + Select + L2 | toggle |
| Quit game | Anbernic, or Start + Select, held on its own for 3 s | one shot |

- R is "forward" (save, fast forward) and L is "back" (load, rewind).
- Quit needs a 3 s hold with nothing else pressed, so it can't happen by accident. Any other key during the hold cancels it until every button is let go and the hold starts again.
- The buttons can be changed under **Settings → Gamepad Settings → Hotkeys**. *Quit Game Button* adds a button chord for quit; it is off by default.
- The Anbernic button on its own opens the Quick Menu when it is **released**, over a game and in Nano's own menus alike. Start + Select opens the Quick Menu only in Nano's own menus, never in a game.
- Anbernic + Volume Up / Down changes the screen brightness everywhere. It is not a hotkey, and the Quick Menu does not open on that release.
- Anbernic + Select turns the display shader on or off everywhere: Nano's menus, apps and games, and drastic-nano. The selected shader is kept (CRT Simple if none was ever chosen), so a shader that makes the screen unreadable can be turned off without seeing it. It replaces Power + Select, which is plain Power again.
- Anbernic + Start opens the Applications view, and closes it again. Over a game the overlay rises on it, and closing it (Anbernic + Start, Start or B) returns to the game. In drastic-nano it does nothing: Nano's overlay can't show during a DS session.
- Neither is a hotkey: both work with Universal Hotkeys off, the Quick Menu does not open on that release, and the game never sees that Start or Select. Start and Select can't be hotkey buttons.
- With Start + Select, the game still sees Start and Select: they can't be held back without delaying them. Start + Select is not a chord when another pad button is already held (A + B + Start + Select resets many games).
- On the touchscreen, a two-finger tap opens or closes the Quick Menu, and a three-finger tap opens or closes the Applications view, in Nano's own menus (not over a game or app).

Each emulator has its own hotkeys, or none. A per-emulator **profile** turns the five actions into whatever that emulator understands. Adding an emulator means adding a profile file, with no code change.

## How it works

```
 pad --> PhoneWindowManager.interceptKeyBeforeQueueing
            |  chord detector (Anbernic held + button)
            v
         ThimsyHotkeyController (system_server)
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

   A Volume key while armed is not swallowed: it changes the brightness (the volume key handler reads the Anbernic button's physical state), and the Quick Menu does not open on release.

   Select or Start pressed while the Anbernic button is physically held comes before the hotkeys: Select flips `persist.gammaos.shader.enable`, and Start sets `sys.gammaos.nano.qs_summon=2` with `show_overlay=1`, so Nano rises on the Applications view. The button's repeats and key-up are swallowed. Select works even with the Anbernic button given back to apps (Toolbox); Start needs it to be Nano's.
3. **Anbernic up:**
   - after a chord: nothing more;
   - alone, within 3 s: the overlay rises on the Quick Menu.
4. **Hold to quit:** the Anbernic button held on its own for 3 s runs quit, and its release opens nothing. Any other key pressed during the hold cancels it. With Universal Hotkeys off there is no hold to quit, and a release within 10 s opens the Quick Menu as before. The 10 s emergency recovery hold (BACK, or a pad's mode button) is unchanged, and checks the key is still down when it fires. The D-pad reaches Android as a hat axis, so over an Android app it doesn't cancel a hold.
5. **Start + Select:** the second of the two pressed while the other is held, with no other pad button down, arms the chord. Both keys still reach the app. Their down state is tracked from the key events, and a chord whose key-ups went missing (the overlay holds them back) is dropped at the next key.
   - A hotkey button pressed while both are held runs its hotkey and is swallowed like an Anbernic chord. Any other key reaches the app. A game's own combination that starts with Start + Select and then adds a hotkey button (L + R + Start + Select resets DS games) runs the hotkeys instead; *Start + Select Hotkeys* off avoids that.
   - Held for 3 s with nothing else pressed, it runs quit. Any other key, or Start or Select pressed again, cancels the hold until both are let go.
   - It never opens the Quick Menu over a game.
   - *Start + Select Hotkeys* (`persist.gammaos.hotkeys.startselect`) turns it off.
6. **Quit button:** if *Quit Game Button* is set, that button must stay down for 1 s (*Hold to Quit*). Releasing it earlier cancels.
7. **Order of presses:** only Anbernic first, then the button, is a chord. A button already held when Anbernic goes down passes through untouched.
8. **A lost key-up:** if the Anbernic button's key-up goes missing, a later chord checks that it is physically down before acting. Start + Select reads both buttons' physical state.

Nano's own menus follow the same rule. The Quick Menu toggles on release, and a button pressed while the Anbernic button is held cancels it and is ignored. Start and Select act when they are released, so pressing both together toggles the Quick Menu once both are up. A key pressed while either is held cancels its action. Anbernic + Volume is brightness, and cancels the Quick Menu like any other chord. Anbernic + Select (shader) and Anbernic + Start (Applications) act on the press and are not a Start + Select chord; Select's search and Start's own action don't run. Applications closes back to where it was opened from, and Start on its own closes it too.

### ThimsyHotkeyController (system_server)

`frameworks/base/services/core/java/com/android/server/policy/ThimsyHotkeyController.java`. It:

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
| `key` | Taps a keyboard key (virtual keyboard, `SOURCE_KEYBOARD`). `"mods": ["CTRL", "SHIFT", "ALT"]` puts the modifiers on the event, and also presses and releases the modifier keys around it. The virtual keyboard reports F1–F12 in `InputDevice.hasKeys`, for emulators that only map keys a device says it has |
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
| Start + Select Hotkeys | `persist.gammaos.hotkeys.startselect` | On |
| Quit Game Button | `persist.gammaos.hotkeys.quit` | Off |
| Hold to Quit | `persist.gammaos.hotkeys.quithold` | On |
| Emulator Support | read-only: each profile, what works and its note | — |

- A button can be A, B, X, Y, L1, R1, L2, R2, L3, R3, a D-pad direction, or Off. Start and Select are reserved for Anbernic + Start and Anbernic + Select; a hotkey set to one of them earlier is turned Off. A D-pad direction works only in drastic-nano: the D-pad reaches Android as a hat axis, which `PhoneWindowManager` never sees.
- Two hotkeys can't share a button: choosing a button another hotkey has swaps the two.
- **Controller Bindings** in the Quick Menu (`sys.gammaos.bindings.enabled`, On at every boot) set to Off turns off Start + Select's hotkeys and its hold to quit, in apps and in drastic-nano, along with the other controller bindings (the browser controls, mouse mode and its combo). The Anbernic button's hotkeys stay.

### drastic-nano (Nano's DS player)

drastic-nano reads the pad itself, so PhoneWindowManager leaves the Anbernic button to it. Its input loop has the same chord rule, including Start + Select and the 3 s hold to quit, and reads the same properties:

| Action | What drastic-nano does |
|---|---|
| Save | Quick save to slot 0 |
| Load | Quick load from slot 0 |
| Fast forward | Its fast-forward toggle |
| Quit | Its exit, after the hold; it saves slot 9 for Quick Resume |
| Rewind | Nothing; it has no rewind |

A Menu binding on the Anbernic button doesn't fire after a chord. Anbernic + Select turns the display shader on or off there too; on the DRM path drastic-nano draws the panel itself (with its own shaders), so the change shows once the game ends. Anbernic + Start does nothing there, since Nano's overlay can't show while drastic-nano owns the panel. The game sees neither button.

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
| SeedlessDS 1.0.1 (not installed by default) | F2 (quick slot) | F4 | F6 (toggle) | — | `exit` | `shared_prefs/keymap.xml` `s_1` (Fast Forward), `s_2` (Quick Save), `s_3` (Quick Load): its emulator shortcuts, which have no keys by default. The file exists once its first game has started. Only its first key map profile is bound. It has no rewind and no exit shortcut (Back opens its menu) |
| ScummVM | — | — | Ctrl+F (SCUMM games) | — | Ctrl+Q, then `exit` | Built-in ScummVM keys |
| drastic-nano | slot 0 | slot 0 | toggle | — | saves, then exits | Built in |
| DraStic (app) | — | — | — | — | `exit` (Esc) | Its keymap is shared with drastic-nano, so it is left alone |
| Dolphin 2609-197 | F2 (selected slot) | F4 | F6 (toggle) | — | `exit` | `Config/Hotkeys.ini` `[Hotkeys]`, bound to `` `Android/0/Virtual:F2` `` and so on. Dolphin only maps keys a device reports in `hasKeys`, so the virtual keyboard reports F1–F12 (`EventHub.cpp`) |
| M64Plus FZ | — | — | — | — | `exit` | Hotkeys live in private controller profiles and need long holds |
| hakuX | — | — | — | — | `exit` | Save states only from its in-game menu; no fast forward |
| Vita3K | — | — | — | — | `exit` | No save states or fast forward |
| Any other app | — | — | — | — | `exit` | `fallback.json` |

## Rewind

Rewind works on every system whose emulator or core can rewind; performance is not a concern. The hotkey never turns rewind on in an emulator's settings. If the user enabled it there, the button works; if not, the emulator ignores the key. RetroArch's `rewind_enable` stays as the user set it (off in the shipped config).

## Power button

Holding the power button for 7 s shuts the console down, in a game or in Nano. `PhoneWindowManager` handles it. Shutdown closes the game first where it can: drastic-nano saves slot 9, and Quick Resume is prepared. drastic-nano's own 5 s power-off was removed so the two don't race.

## Devices

"The Anbernic button" in this document means the device's **function button**. On the RG Cube it is the Anbernic logo button. Other handhelds GammaOS supports have a Mode, Home or Menu button instead, or only a Back button. Every chord above, and the coming Anbernic + bottom face button, works through the same lookup, so they work the same on every device.

### How a device's buttons are found

One table, `/system/etc/gammaos/device_keys.json` (from `mods/device-keys.json`, installed by `mods/55-hotkeys.sh`), lists each device's function and Back buttons. Every reader picks the first entry whose `match` fits the device's system properties, or the table's `default` if none does:

| Reader | Code | Uses |
|---|---|---|
| PhoneWindowManager | `ThimsyDeviceKeys.java`, read once at `systemReady` | Android keycodes: the chord detector, the Anbernic + Volume / Select / Start checks, the 10 s recovery hold |
| Nano | `NanoDeviceKeys.cpp`, read on first use | evdev codes: its menus' Anbernic handling, Anbernic + Volume / Select / Start |
| drastic-nano | the same `NanoDeviceKeys.cpp` | evdev codes: its chord detector, and which input devices it opens |

`ThimsyHotkeyController` doesn't change. It only injects keyboard keys, and never reads the pad.

The two resolvers are held to the same test cases (`gammaos-nano/test/fixtures/devkeys/cases.json`), so they always pick the same keys.

- **No table, or a broken one:** the RG Cube's keys, the ones Thimsy always used. Those are F10 from a gamepad, `BUTTON_MODE` and Back.
- **`match`:** a list of alternatives. All the properties an alternative names must match, and any one alternative is enough. A value matches case-insensitively, or as a prefix when it ends in `*`.
- **The properties used:**
  - `ro.gammaos.device`: only newer GammaOS vendor images set it (`anbernicrgds`, `anbernicrgrotate`, `trimuibrick`, `magicxxu20v32`, `pocketairmini`, `retroidpocketduolite`, ...). The T820, T618 RG405M/V/RG505, MediaTek, Retroid 4 Pro/Classic, KT-R1, ZPG, Pocket Micro and E6 Max images don't, so those match by model or platform.
  - `ro.product.vendor.model`, `ro.product.odm.model`, `ro.product.odm_dlkm.model`: the real strings, spaces included (`RGCUBE`, `RG 406H`, `RG 477V`, `RG VITA`, `Retroid Pocket Classic`). Anbernic shares vendor images, so the RG406V reports `RG 406H` and the RG Slide reports `RG 476H`; their keys are the same.
  - `ro.product.vendor.manufacturer`;
  - `ro.board.platform` (`ums9620` for T820, `ums512` for T618, `ceres` for A133P, `mt6897`, `mt6893`).
- **A key:**
  - `linux`, the evdev code Nano and drastic-nano read;
  - `android`, the keycode name PhoneWindowManager sees after the key layout. A key without one (the AYANEO codes 753 and 754, which no layout maps) is for Nano and drastic-nano only;
  - `gamepad_only`, so a keyboard's F10 still reaches the app.
- **An entry without `function` or `back`:** takes the default's. `"function": []` means the device has no function button.
- **`"fallback": "back"`:** Back doubles as the function button (below).

The default function buttons are F10 (gamepad only), `BUTTON_MODE` and `HOME`, and the default Back is `BACK`. The Cube's own entry leaves out `HOME`, so nothing on the Cube changes.

Nano maps the function button's code to the Cube's F10 as it reads it, after the button pickers (which record the raw code). The rest of its menu code then needs no change. F10 and `BUTTON_MODE` keep working there on every device, as before.

**GammaPad remaps.** On many newer images (`persist.gammaos.gamepad.enable=1` in the vendor `build.prop`), GammaPad grabs the pad and re-emits it as "Xbox Wireless Controller", after its default remaps:
- RG DS and RG DS Plus: Mode 316 → 204.
- RG Vita and RG Vita Pro: F10 68 → 204.
- Pocket Air Mini: AYANEO 754 → 158, ≡ 753 → 204.

Such entries list both the raw and the remapped code, so the key matches whichever arrives. On the virtual pad 204 reaches Android as `NOTIFICATION` (`Generic.kl`, product `0b13`) or as `ALL_APPS` (the `02fd` layout), so both are listed.

On the RG DS, 204 is also the key Nano's Control Center toggles on. With the Mode button as the function button, it raises the Quick Menu and toggles the Control Center at once. That needs a product decision there: turn the remap off, or give the Control Center another key.

**The `045e:02fd` layout.** `Vendor_045e_Product_02fd.kl` (in AOSP, `frameworks/base/data/keyboards`) now maps `key 68 F10`. Without it, a GammaPad that presents as `045e:02fd` (the old vendor `gammapad`, or the `0x02fd` preset) would hand the Anbernic button to PhoneWindowManager as `KEYCODE_UNKNOWN`. The RG Cube's own pad (`484b:1101`) doesn't use it.

### Device map

Sources:
- **level 1:** the device tree or key layout inside a GammaOS release image or stock dump;
- **level 2:** driver or daemon source or binary;
- **level 3:** GammaOS vendor configuration (`GammaOSCoreVendor`);
- **level 4:** reviews and spec pages, for labels.

The research notes are `work/device-buttons-research.md`. "GP" is the code after GammaOS's default GammaPad remap.

| Device | SoC | Detected by | Function button → evdev / Android | Back button → evdev / Android | Confidence (source) |
|---|---|---|---|---|---|
| RG Cube | T820 | vendor/odm model `RGCUBE` | Anbernic logo → 68 / `F10`; 316 / `BUTTON_MODE` kept | Back → 158 / `BACK` | verified (1: v1.2 dtbo, vendor key layout, `build.prop`) |
| RG556, RG406H, RG406V, RG476H, RG Slide | T820 | `RG556`, `RG 406H`, `RG 476H`, or `ums9620` + Anbernic | Anbernic / Home → 68 / `F10` | Back → 158 / `BACK` (Slide: right of the screen) | verified (1: each model's dtbo) |
| RG477M, RG477V, RG557 | Dimensity 8300 | odm model `RG477M`, `RG 477V`, `RG557` | Home → 68 / `F10` | Back → 158 / `BACK` | verified (1: stock dtbo from the Unbricker dumps) |
| RG Rotate | T618 | `anbernicrgrotate` | side Function → 68 / `F10`; a second key sends 67 / `F9` | centre home/return → 158 / `BACK` | codes verified (1), labels likely (4); which side key is 67 or 68 unconfirmed |
| RG Vita | **T618** | `anbernicrgvita`, `RG VITA` | Anbernic → 68 / `F10` (GP 204 / `NOTIFICATION`) | Back → 158 / `BACK` | verified (1, 3) |
| RG405V | T618 | odm/vendor model `RG405V` (assumed string) | side Function → 68 / `F10` | front Home → 158 / `BACK` | likely (1 codes, 4 labels) |
| RG405M, RG505 | T618 | `RG405M` (verified), `RG505` (assumed), or `ums512` + Anbernic | **None** (68 is wired to no button) | Home / Back → 158 / `BACK` | codes verified (1), labels likely (4). **Back doubles as function** |
| RG Vita Pro | RK3576 | `anbernicrgvitapro`, `RG VITA Pro` | Anbernic → 68 / `F10` (GP 204) | Back → 158 / `BACK` (on `adc-keys`) | verified (1, 3) |
| RG DS, RG DS Plus | RK3568 | `anbernicrgds`, `anbernicrgdsplus` | Mode → 316 / `BUTTON_MODE` (GP 204) | adc-keys button → 158 / `BACK` | codes verified (1: v1.4.4 OTA dtb), labels unverified |
| TrimUI Brick, MagicX Zero 40, Mini Zero 28, XU20 V32 | A133P | `trimuibrick`, `magicxzero40`, `magicxminizero28`, `magicxxu20v32`, or `ceres` + `QUAD-CORE A133 aw4` | **None** | **Menu** → 158 / `BACK` | verified (1, 2). **Menu doubles as function** |
| GKD 350H Ultra | RK3576 | `gkd350hultra` | "F1" → 316 / `BUTTON_MODE` | "F" → 158 / `BACK` | codes verified (1), labels from the device tree |
| AYANEO Pocket Air Mini | MT6785 | `pocketairmini` | ≡ → 753 / none (GP 204 / `NOTIFICATION`) | AYANEO → 754 / none (GP 158 / `BACK`) | codes verified (1, 3); the role split is GammaOS's |
| Retroid Pocket 4 Pro | Dimensity 1100 | `mt6893` + `Moorechip` | Home → 316 / `BUTTON_MODE` | Back → 158 / `BACK` | verified (2: `retroidpad` binary) |
| Retroid Pocket Classic | Qualcomm `parrot` | `Retroid Pocket Classic` | **None distinct** (Home also sends 158) | Back and Home → 158 / `BACK` | verified (2). **Back doubles as function** |
| Retroid Pocket Duo Lite (beta), Duo | Qualcomm | `retroidpocketduolite`, `retroidpocketduo` | Home → 316 / `BUTTON_MODE` | Back → 158 / `BACK` | verified (2: `RetroidPad.c`) |
| GameMT E6 Max | A527 `saturn` | `A527 PRO` + `saturn` | Home → 316 / `BUTTON_MODE` (likely) | default (unverified) | partial (1: stock dtb) |
| Mangmi Air X | SD680 | `mangmiairxmq65`, `mangmiairxmq66` | Home → 316 / `BUTTON_MODE` (likely) | default (unverified) | likely (3) |
| KT Pocket KT-R1 | `mt6789` | odm model `KTPocket-R1` | default list | default | unverified: detection only |
| ZPG A1 Unicorn | `mt6789` | odm model `ZPG A1` | default list | default | unverified: detection only |
| AYANEO Pocket Micro | `mt6789` | `Pocket Micro` + AYANEO | default list | default | unverified: detection only |

Not supported, so not in the table: AYN Thor, the GammaOS Core (Android TV) devices, and the planned AYANEO Pocket S/DMG and Air, Retroid Pocket Mini, Flip 2 and 5, and Powkiddy X28. They get the default.

**Still to check with `adb shell getevent -lp`, then `getevent -l` while pressing each system button:**
- **KT-R1:** which buttons send `KEY_MENU`, `KEY_HOMEPAGE`, `BTN_START` and `BTN_SELECT`.
- **ZPG A1 and Pocket Micro:** the Home and Back codes.
- **Mangmi Air X:** Back's code, and confirm Home is 316.
- **E6 Max (both revisions):** Back's code, and the USB "Lenovo GamePad" codes.
- **RG Rotate:** which side key sends 67 and which sends 68.
- **RG Vita:** what the 67 "RESET" key is.
- **RG405M and RG505:** that nothing is wired to 68, and the RG505's model string.
- **RG405V:** that the side button sends 68, and its model string.
- **RG DS / DS Plus and Air Mini:** the printed labels.
- **TrimUI and MagicX:** when Menu also sends 256.
- **Any GammaPad device on a Thimsy build:** `dumpsys input`, to see which virtual product ID (`0b13` or `02fd`) and key layout are in use.

### Back acts when it is let go

On every device, the RG Cube included, the Back button acts on its release, never on its press. A Back press that turns into a chord or a hold therefore never also goes back.

- **Over a running app (PhoneWindowManager):**
  - The press is held back.
  - Let go on its own, the app gets a normal Back press and release (injected).
  - Held to the long press, it runs the long-press-Back exit as before (where a long-press action is set), and the release sends nothing.
  - Any other key pressed while it is held (Volume for brightness, a RetroArch Back + button combination) means nothing is sent.
  - Held into the 10 s recovery hold, nothing is sent; the recovery hold itself still starts on the press.
  - The device's Back key is used even where its raw code isn't `KEY_BACK` (the AYANEO button, 754, on the Pocket Air Mini): Nano and drastic-nano read it as Back, and over apps PhoneWindowManager sends the tap as `BACK`.
- **Nano's menus:**
  - Back is held back like Start and Select, and acts (goes back, closes the keyboard or dialog) when let go.
  - Another key pressed meanwhile cancels it.
  - Holding it no longer repeats.
- **drastic-nano:** Back already acted on release (a tap opens its menu, a long press exits). The device's Back key reads as Back there too.
- **Quick Resume preview:** the same, already on release.

The decision on release is `devkeys::backRelease` (C++) and `ThimsyDeviceKeys.backRelease` (Java), held to the same test cases. With nothing else pressed:
- Back alone, released short of the 10 s recovery hold: Back.
- Where Back doubles as the function button:
  - under 1 s: Back;
  - 1 to 3 s: the Quick Menu;
  - 3 s or more: nothing (that was the hold to quit).

### Devices without a function or Back button

- **No function button, only Back** (RG405M, RG505, and the Retroid Pocket Classic, whose Home and Back both send 158), `"fallback": "back"`. Back + a button replaces the Anbernic chord. Back works like the Anbernic button over a game, except on its own:
  - Back + a button: that button's hotkey. Back + Volume is brightness, Back + Select toggles the shader, and Back + Start opens Applications.
  - Back held on its own for 3 s quits the game.
  - Back tapped and let go within 1 s: the game gets a Back press and release, sent on the release.
  - Back held between 1 s and 3 s: the Quick Menu, as the Anbernic button's release does.
  - In Nano's menus Back goes back when let go within 1 s, and held 1 to 3 s it opens the Quick Menu. Back + Volume, Select and Start work there too.
  - In drastic-nano, Back keeps its own tap (in-game menu) and long press (exit). Back + a button is a hotkey and opens no menu.
  - Toolbox → *Anbernic Button Opens Quick Menu* off makes Back plain Back again over apps.
- **Only a Menu button** (TrimUI Brick and the MagicX handhelds):
  - Their Menu button sends `KEY_BACK` (158), so it is their Back button (`"function": []`, `"fallback": "back"`).
  - It doubles as the function button exactly as Back does above: Menu + a button runs the hotkeys, holding it opens the Quick Menu or quits, and a tap is Back on release.
  - Being `BACK`, it is the 10 s recovery key too.
  - B is not Back on these devices.
- **A device with a function button but no Back button** (none known): the function button becomes the 10 s recovery key. PhoneWindowManager adds it when a device's entry has no Back.
- **On every device:** Start + Select still does every hotkey and the 3 s quit, and Power held raises the Quick Menu over a game.

### Adding a device

1. Find the function and Back buttons' codes:
   - `adb shell getevent -lp` gives the input devices and the codes each sends;
   - `adb shell getevent -l` shows a press;
   - the key layout (`/vendor/usr/keylayout/Vendor_XXXX_Product_YYYY.kl`, or `Generic.kl`) gives the Android keycode.
2. Find a property that identifies the device: `adb shell getprop ro.gammaos.device`, `ro.product.vendor.model`.
3. Add an entry to `mods/device-keys.json`, before any broader entry that would also match it. Say in `verified` where the codes come from.
4. To try it without a rebuild, push it to `/system/etc/gammaos/device_keys.json` (root, remounted system) and reboot. `logcat -s NanoDeviceKeys` and `logcat | grep "device keys"` show what Nano and PhoneWindowManager picked.

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
   - Anbernic, or Start + Select, held 3 s quits; a key pressed during the hold cancels it;
   - Start + Select + R1 saves, and Start + Select released opens nothing in a game;
   - Anbernic + an unmapped button does nothing;
   - the 10 s emergency hold still recovers;
   - Anbernic + Select turns the shader off and on again, in Nano, in a game and in drastic-nano, and Power on its own sleeps;
   - Anbernic + Start opens Applications over a game, and Start, B or Anbernic + Start returns to the game;
   - rewind released when the Quick Menu opens.
   - Back acts on release: in an app (Settings, a browser) a tap goes back once when let go, a long press runs the exit, and Back held while Volume changes the brightness never goes back. In Nano's menus Back goes back on release and never repeats while held.
