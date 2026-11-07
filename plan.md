# Plan: replace Daijisho with ES-DE

Status: proposal. Nothing here is implemented yet.

## Summary

ES-DE's Android release can run every emulator and pack-in game this build
ships, uses the same `ROMs/<system>` folder names, and can be the home app.
The obstacle is licensing, not engineering: **the Android APK is paid and
partly closed source**, so it can't be bundled in a public release without
the ES-DE developers' permission.

The plan is therefore:

1. Ask the ES-DE project for permission to preinstall the APK.
2. Meanwhile, make the build able to **preconfigure** ES-DE: settings,
   default emulators, game lists and art go into `/sdcard/ES-DE`, so that a
   user who installs their own purchased APK gets a working setup.
3. Add a **private build option** (`ESDE_APK=/path/to/ES-DE.apk`) that
   preinstalls the user's APK and makes ES-DE the home app instead of
   Daijisho. Public builds keep Daijisho until permission is granted.

## Facts this plan relies on

From the ES-DE Android documentation (`ANDROID.md`, `FAQ-ANDROID.md`,
`USERGUIDE.md`) and its `es_systems.xml` / `es_find_rules.xml`:

- **Distribution.** The APK is sold on Patreon, the Samsung Galaxy Store
  (`org.es_de.frontend.galaxy`) and Huawei AppGallery. The Patreon build
  (`org.es_de.frontend`) is a plain APK that can be sideloaded. "There is some
  Android-specific code that is copyrighted and closed source", so it can't
  be built from source either.
- **Requirements.** 64-bit Android 10 or later. The compatibility list
  includes the RG Cube on stock Android 13, and GammaOS Next on another
  device.
- **Onboarding.** On first start a configurator asks for "All files access"
  (`MANAGE_EXTERNAL_STORAGE`), the data folder (default `/sdcard/ES-DE`) and
  the ROMs folder (default `/sdcard/ROMs`). It can optionally create the
  system folders and their `systeminfo.txt` files. It runs again if the
  storage permission is revoked or either folder disappears.
- **Data folder.** Everything except downloaded themes lives in the data
  folder and survives reinstalling the app:
  - `settings/es_settings.xml`
  - `gamelists/<system>/gamelist.xml`
  - `downloaded_media/<system>/<type>/<game>.png`
  - `custom_systems/es_systems.xml` and `es_find_rules.xml`
- **Default emulator per system.** A gamelist can carry
  `<alternativeEmulator><label>…</label></alternativeEmulator>`, which picks
  one of the system's `<command label="…">` entries (`GamelistFileParser.cpp`).
- **Home app.** ES-DE can be selected as the home app; quitting is then
  disabled.
- **Themes.** Only the Linear theme is bundled; others come from the
  in-app theme downloader.
- **Touch overlay.** It's on by default; `InputTouchOverlay` in
  `es_settings.xml` turns it off. The RG Cube has physical controls.

## Compatibility with this build

### Systems and emulators

ES-DE's bundled `es_systems.xml` already has entries for every emulator
this build installs. Its find rules match our package names: RetroArch
`com.retroarch.aarch64`, Dolphin, ARMSX2, Azahar, WatermelonDS, hakuX,
Vita3K, ScummVM, PPSSPP, Flycast, M64Plus FZ and DraStic. Our ROM folders
use ES-DE's system names. Checked: gb, gbc, gba, nds, gc, wii, n3ds, xbox,
scummvm, psp, pico8, pcengine, tg16, nes, snes, virtualboy, mastersystem,
gamegear, genesis, sega32x, dreamcast, ngpc, atari2600/5200/7800, n64, psx,
ps2, psvita and saturn.

RetroArch commands load cores from
`%INTERNALDATA%/com.retroarch.aarch64/cores/<core>_libretro_android.so` with
the config at `/sdcard/Android/data/com.retroarch.aarch64/files/retroarch.cfg`.
Both are where GammaOS installs them.

### Default emulators

ES-DE uses the first command of each system unless the gamelist names
another. To match our Daijisho defaults, each system's `gamelist.xml` gets an
`alternativeEmulator` label:

| System | Daijisho default | ES-DE label |
|---|---|---|
| gb, gbc | SameBoy | `SameBoy` |
| gba | mGBA | `mGBA` |
| nes | Mesen | `Mesen` |
| snes | Snes9x | `Snes9x - Current` |
| nds | WatermelonDS | `WatermelonDS (Standalone)` |
| n3ds | Azahar | `Azahar (Standalone)` |
| gc, wii | Dolphin | `Dolphin (Standalone)` |
| xbox | hakuX | `hakuX (Standalone)` |
| scummvm | ScummVM | `ScummVM (Standalone)` |
| psx | PCSX ReARMed | `PCSX ReARMed` |
| virtualboy | Beetle VB | `Beetle VB` |
| mastersystem, gamegear, genesis | Genesis Plus GX | `Genesis Plus GX` |
| sega32x | PicoDrive | `PicoDrive` |
| pcengine, tg16 | Beetle PCE FAST | `Beetle PCE FAST` |
| ngpc | Beetle NeoPop | `Beetle NeoPop` |
| atari2600 / 5200 / 7800 | Stella / a5200 / ProSystem | `Stella` / `a5200` / `ProSystem` |

The remaining systems (psp, dreamcast, n64, pico8, saturn, arcade and so on)
should be generated rather than typed. A script maps each Daijisho default
player (from `daijisho_defaults.py`'s output) to the ES-DE label, and fails
on any player it can't map. Labels for the shipped cores must stay in sync
with `mods/retroarch-cores.txt`.

### Pack-in games

| Game | What ES-DE needs |
|---|---|
| Most games | Nothing: their extensions are in ES-DE's lists. |
| ScummVM (Beneath a Steel Sky) | ES-DE launches `.scummvm` files containing the game ID. Ship `ROMs/scummvm/Beneath a Steel Sky.scummvm` with the text `sky`, next to or instead of the Daijisho `.dpt` file. ScummVM still reads the game from its own storage, so test that the launch reaches it. |
| PSP (KłełeAtoms, a folder with `EBOOT.PBP`) | Use a folder link in the gamelist, so the folder launches its `EBOOT.PBP` directly. Check the stored path format by creating one in ES-DE's metadata editor first. |
| PICO-8 (`.p8.png`) | `.png` is in the pico8 extension list; set the game's name in the gamelist so it doesn't show as "Cherry Bomb.p8". |
| License and `systeminfo.txt` files | Ignored, since `.txt` isn't a game extension for any of our systems. |

### Art

- **System art** comes from the theme. Linear covers all our systems, so
  the Daijisho platform wallpapers aren't needed.
- **Game art** goes in `downloaded_media/<system>/{covers,screenshots,…}`.
  Ship only art the game's license covers, such as screenshots taken from
  the free games themselves. Otherwise leave it to the user's own
  ScreenScraper scrape.

## Implementation

### 1. Ask for permission

Contact the ES-DE developers through their Patreon or Discord. Ask whether
a free firmware build may preinstall the Patreon APK, and under what terms
(for example, only for users who bought it, or with a link to buy). Record
the answer in this file. Without written permission, no public build
includes the APK.

### 2. Preconfiguration (public builds)

New `mods/36-esde.sh`, run after `30-bundled-roms.sh`. It builds
`/system/etc/esde.tar.xz` from the ROM pack and a new list,
`mods/esde-defaults.txt` (system and label). The archive contains:

- `ES-DE/settings/es_settings.xml`:
  - `ROMDirectory` `/storage/emulated/0/ROMs`
  - `InputTouchOverlay` false
  - theme `linear-es-de`
  - the rest left at ES-DE's defaults
- `ES-DE/gamelists/<system>/gamelist.xml`: an entry for each pack-in game
  (name, description, the license in the description), the
  `alternativeEmulator` label, and the KłełeAtoms folder link.
- `ROMs/scummvm/Beneath a Steel Sky.scummvm`.

First-boot setup extracts it to `/sdcard` and gives it to the user, like the
spare Daijisho wallpapers. Daijisho stays installed and stays the home app.

Two things need testing on the device, because the onboarding code is
closed:

- Does the configurator keep an existing `es_settings.xml` and gamelists
  when the user picks the default folders?
- Does granting `MANAGE_EXTERNAL_STORAGE` in setup
  (`appops set --uid <pkg> MANAGE_EXTERNAL_STORAGE allow`) skip the
  permission step?

If the configurator overwrites the settings, ship them as
`ES-DE/settings/es_settings.xml.gammaos` and document a one-time copy.

### 3. Private build option

`ESDE_APK=/path/to/ES-DE.apk bash scripts/build.sh`, used only by people who
bought the APK, for their own devices. When it's set:

- `40-added-apps.sh` copies the APK to `/system/etc` and installs it in
  first-boot setup, with the storage appop granted.
- `35-daijisho.sh` is skipped and the Daijisho install is dropped from
  `setup.sh`, which frees about 30 MB.
- The home-app lines in `setup.sh` point at ES-DE:
  `cmd package set-home-activity <pkg>/<activity>` and the matching
  `pm set-home-activity`. Read the activity name from the APK with
  `aapt dump badging`, and check it's the one ES-DE registers for HOME.
- The build output gets a different name (for example `…_v1.0_esde.pac`),
  and `build.sh` refuses to name it like the public release, so a private
  build isn't published by mistake.

### 4. Docs

- README: an ES-DE section explaining both modes and the purchase links.
- PACK-IN-GAMES.txt: the `.scummvm` file.
- Release notes: how to install a purchased APK onto a public build. The
  setup files are already in `/sdcard/ES-DE`; install the APK, open it, keep
  the default folders, then optionally set it as the home app in Settings >
  Apps > Default apps.

## Testing on the RG Cube

1. Configurator: does it keep the preconfigured settings and gamelists?
   Does it skip the storage step when the appop is granted?
2. Every pack-in game launches with the intended emulator from ES-DE.
   Especially check ScummVM (`.scummvm` reaching the game in ScummVM's own
   storage), KłełeAtoms (folder link), Cherry Bomb, and the Dolphin, Azahar
   and WatermelonDS games, which use FileProvider or SAF.
3. Emulators that use scoped storage (M64Plus FZ, others listed in
   `ANDROID.md`) still need their `ROMs/<system>` folder granted once.
   Decide whether to document that or pre-grant it.
4. As the home app: the boot time, return from games, the SD card mount
   delay noted in the FAQ, and the HDMI docking behaviour GammaOS adds.
5. Theme downloader: the FAQ says it needs SELinux enforcing on some
   handhelds. GammaOS runs permissive (`setenforce 0` at boot), so check it
   works.

## Open questions

- The ES-DE developers' answer about redistribution.
- Whether GammaOS's own Daijisho-specific features (for example the HDMI
  docking or button remapping overlays) assume Daijisho is the launcher.
- Whether to keep Daijisho installed alongside ES-DE in the private build,
  as a fallback.
