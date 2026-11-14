<p align="center"><img src="docs/thimsy.png" alt="The Thimsy OS mascot: an elf in a thimble helmet, raising a sword" width="160"></p>

# Thimsy OS

Thimsy OS, formerly gamma-os-next-improved, is a modified build of
[GammaOS Next](https://github.com/TheGammaSqueeze/GammaOSNext)
v1.2 (Full edition) for the **Anbernic RG Cube**.

This repository doesn't contain firmware images. It holds the scripts and
file overlays that turn the official GammaOS Next release into the modified
build, so you can rebuild it yourself.

## Base

- Upstream release: [`v.1.2.0-ANBERNICRGCUBE`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRGCUBE)
- Edition: Full (Android 14 / LineageOS 21, with Google Play)
- Prebuilt firmware: none published at the moment; build it yourself (see Building)

## Changes from upstream

Emulators updated to their latest releases (installed on first boot):

| Emulator | Upstream v1.2 | This build |
|---|---|---|
| RetroArch | 1.22.1 | 1.22.2 |
| RetroArch cores | 152 cores, mixed 2023-2025 builds | one core per system (see `mods/retroarch-cores.txt`), latest buildbot nightlies at build time |
| Flycast | 2.5 | 2.7 |
| PPSSPP | 1.19.2 | 1.20.4 |

The RetroArch cores are cut down to a curated set (see
`mods/retroarch-cores.txt`), which shrinks `retroarch.tar.xz` from 336 MB to
271 MB. Game Boy and Game Boy Advance link-cable play over RetroArch netplay
(local network or internet) is available through the DoubleCherryGB and gpSP
cores. Edit `mods/retroarch-cores.txt` to change the selection.

The cores marked `D - ` in that file are each system's default emulator in
Nano and ES-DE. Where they differ from upstream v1.2's defaults (Daijisho's):

| Platform | Upstream v1.2 | This build |
|---|---|---|
| Game Boy, Game Boy Color | Gambatte | SameBoy |
| Game Boy Advance | gpSP | mGBA |
| NES | Nestopia | Mesen |

PlayStation games run on Beetle PSX HW, which supports RetroArch netplay. It
needs a PlayStation BIOS (for example `scph5501.bin`) in
`/sdcard/RetroArch/system`; PCSX ReARMed, which runs without one, is still
shipped and can be chosen per game or for the system.

RetroArch's core info files are refreshed to match the new cores, and the
PPSSPP core's assets are removed with the core (PSP uses the standalone
PPSSPP app). Mupen64Plus FZ stays at 3.0.335 because newer builds
are only on Google Play, and DraStic r2.6.0.4a is its latest release.

Other apps:

| App | Upstream v1.2 | This build |
|---|---|---|
| Daijisho | 1.4.80 | removed |
| MiXplorer | 6.64.3 | removed |
| Aurora Store | 4.6.4 | 4.8.4 |
| Firefox | 139.0 | replaced by [Brave](https://github.com/brave/brave-browser) 1.99.55 |
| — | — | [Fulguris](https://github.com/Slion/Fulguris) 2.1.2 (F-Droid build), 1.4.4 base, only on devices with 1.3 GB of RAM or less |

Daijisho and MiXplorer are closed apps with no grant to redistribute them, so
they are removed (`mods/15-update-apps.sh`). Android's home is Trebuchet,
LineageOS's launcher, which both bases ship. Files are managed in Android's
own Files app, or in Nano's File Explorer on the 1.4.4 base.

Brave is the default browser, set by a fresh setup (`mods/15-update-apps.sh`),
so a browser chosen later stays chosen. Setup skips Brave on devices with
1.3 GB of RAM or less; on the 1.4.4 base they get Fulguris, a lighter browser,
as their default instead (the `maxmem:` condition in `apps.list`).

Aurora Store comes from IzzyOnDroid, which only keeps recent versions, so its
link in `mods/15-update-apps.sh` will need updating for later builds.
Magisk stays at 30.6, the version the boot image is patched with, and
can update itself after setup. See `mods/15-update-apps.sh`.

**No Google apps.** Upstream's Full image carries Google Play Store, Play
Services, Services Framework, Google's setup and its sync adapters; this build
leaves them all out (`device/phh/treble/lineage_arm64_bgN.mk`). Aurora Store
installs Play apps without them, though apps that need Play Services (Google
sign-in, Firebase notifications, some licence checks) won't work. To add Google
Play yourself: install a Google apps package for Android 14 arm64 that installs
as a Magisk module (the system partition is read-only, so recovery-flashed
packages can't write to it), restart, then register the device at
<https://www.google.com/android/uncertified> with its Google Services Framework
ID, since this build isn't Play Protect certified.

File Transfer (MTP) is the default USB mode (Developer options → Default USB
configuration), so a computer sees the device's storage as soon as it's
plugged in (`mods/25-usb-mtp.sh`).

The built-in controller reports its face buttons by position, like an Xbox
controller (`mods/27-controller-layout.sh`): the bottom button (printed B)
is A, the right one (printed A) is B, the left one (printed Y) is X and the
top one (printed X) is Y. The vendor's layout reported them by their print,
so games put PlayStation Cross on the right button instead of the bottom one.
RetroArch's profile for the controller and PPSSPP's controls, which were set
for the print, are swapped to match, so they keep Cross on the bottom button.
Nano reads the buttons directly, so in its menus A still confirms; ABXY Swap
stays off. Controller Enable (gammapad) reads only the vendor's layout, so with
it on, apps see the buttons by their print again.

### Added emulators

These standalone emulators are added and installed on first boot. Each is
unmodified and distributed under its license; the source for each version is
linked below.

| App | Systems | Version | License | Source |
|---|---|---|---|---|
| Dolphin | GameCube, Wii | 2609-197 (development build) | GPL-2.0+ | https://github.com/dolphin-emu/dolphin/tree/41799046d68b2e5ea7cfde91f122320a472b7a61 |
| ARMSX2 | PlayStation 2 | 2.9.2 | GPL-3.0 | https://github.com/ARMSX2/ARMSX2/tree/2.9.2 |
| Azahar | Nintendo 3DS | 2126.2 | GPL-3.0 | https://github.com/azahar-emu/azahar/tree/2126.2 |
| WatermelonDS | Nintendo DS | 0.8.0.rc3 | GPL-3.0 | https://github.com/SapphireRhodonite/WatermelonDS/tree/0.8.0.rc3 |
| hakuX | Xbox | 1.3.0 at `a7cdb5a` (built from source) | GPL-3.0+ | https://github.com/rfandango/hakuX/tree/a7cdb5a975e31730d488c47f6f10440b16c1e6f4 |
| Vita3K | PS Vita | 0.2.1 (build 4277, mirrored) | GPL-2.0 | https://github.com/Vita3K/Vita3K/tree/2f68c1bf0d44753eddc8263b6a4dd23fcd928f22 |
| ScummVM | Adventure games | 2026.3.0 | GPL-3.0+ | https://github.com/scummvm/scummvm/tree/v2026.3.0 |

The [MaliSX2](https://github.com/bmdhacks/malisx2/tree/v0.4.0) 0.4.0 Vulkan
driver (MIT) comes pre-installed and selected in ARMSX2, with the Vulkan
renderer, and a copy is in `/sdcard/GPU Drivers/`. hakuX can use it too: import
the ZIP in hakuX's Settings > Graphics > GPU driver. Dolphin only loads custom
drivers on Adreno GPUs, so it can't use it on the RG Cube's Mali-G57.

hakuX is built from its source at commit `a7cdb5a` (newer than its 1.3.0
release), which adds custom Vulkan drivers on Mali GPUs. It is the release
variant signed with this project's key and mirrored in this repo's
`app-mirror-1` release, with the revisions of its native dependencies.

[Shattered Pixel Dungeon](https://github.com/00-Evan/shattered-pixel-dungeon/tree/v4.0.2)
4.0.2 (GPL-3.0), the full game, unmodified, is installed too, so Android Games
starts with a game. It plays with a controller and fits the square screen.

Edit `mods/added-apps.txt` to change the apps.

[SeedlessDS](https://github.com/SapphireRhodonite/SeedlessDS) 1.0.1
(GPL-3.0-or-later; its core is in
[SeedlessDS-Core](https://github.com/SapphireRhodonite/SeedlessDS-Core)) is a
supported Nintendo DS emulator for low-end devices, but isn't installed by
default. It comes from WatermelonDS's developer and runs SeedlessCoreDS, which
is reconstructed from DraStic r2.6.0.4a. Install its APK (package
`com.seedlessds.app`) and choose it for Nintendo DS or DSi in Nano's Game
Systems editor; WatermelonDS stays the default. Nano starts each game in it by
its file path, the universal hotkeys bind its quick save, quick load and fast
forward, and Install Your Own BIOS Files gives it the DS BIOS and a 256 KB
firmware from your own dumps.

### ES-DE

Building with `ESDE_APK=/path/to/ES-DE.apk` adds
[ES-DE](https://es-de.org) as the home app (`mods/37-esde.sh`). ES-DE's Android app is paid
and partly closed source, so its APK isn't in this repository; the build
takes it from that path and checks it's the version the build is set up for
(3.5.0). First-boot setup installs it with storage access, makes it the home
app, and puts its settings and game lists in `/sdcard/ES-DE`:

- the ROMs folder, `/sdcard/ROMs`, and the touch overlay turned off;
- for each system, the build's default emulator: the `D - ` core from
  `mods/retroarch-cores.txt` or the standalone app (`mods/esde-emulators.txt`);
- the pack-in games, with KłełeAtoms launching from its folder and Beneath
  a Steel Sky as the `.scummvm` file ES-DE launches ScummVM games with.

ES-DE's first start still runs its setup screens; choose the default folders
(`ES-DE` and `ROMs` on the internal storage) to use these settings. GammaOS's
Magisk module for an ES-DE theme download error on some devices
(`MAGISK_ES-DE_fix.zip`) is copied to `/sdcard/Download`; install it in
Magisk only if theme downloads fail with "Failed to lock file".

Without `ESDE_APK`, the 1.2 build has no game frontend: the home is
Trebuchet, and games start from RetroArch and the emulator apps.

### 1.4.4 base and the Nano home

Building with `SYSTEM_IMG=/path/to/system.img` uses a GammaOS Next 1.4.4
system image built from GammaOS's source
([GammaOSNextDistribution-14](https://github.com/TheGammaSqueeze/GammaOSNextDistribution-14),
target `arm64_bgN`) in place of the release's 1.2 system. The RG Cube's
vendor, kernel and bootloader stay. The changes above apply to it too, adapted
to 1.4.4's zstd payloads and its app list (`/system/etc/gammaos/apps.list`),
except ES-DE's app: GammaOS Nano is the home. Beneath a Steel Sky's files ship
in `/sdcard/ROMs/scummvm/Beneath a Steel Sky/`, beside its `.scummvm` file
(which holds its ScummVM ID, `sky`), in place of ScummVM's own storage:
first-boot setup adds it to ScummVM's game list by that path, and
RetroArch's ScummVM core, which isn't shipped, reads the same copy once its Online Updater downloads it.

Apply `patches/thimsy/0001-thimsy.patch` to the
source tree (`git apply`, from its root) before building the system image. It
fixes GammaOS Nano bugs:

- settings that are switches or choices, Performance Mode among them, opened
  as a 0-255 slider that did nothing;
- Nano's settings had no Fan Speed and, of the HDMI options, only External as
  Primary; they get Fan Speed, Force Mirror, Resize to External Display and
  Secondary Display Apps;
- in the ES-DE theme the X, Y, shoulder, trigger and stick-click buttons its
  help bar shows did nothing, or acted on the hidden XMB menu, and Y could not
  take away a favourite imported from ES-DE's game lists;
- Nano's File Transfer (MTP) only worked while a full-screen "MTP Active"
  screen stayed open;
- on the first boot, games could not be started from ES-DE's game lists until
  another app had been opened or the console restarted: ES-DE's launch did not
  clear a leftover search, as the XMB's launches do, so it looked for the game
  in empty search results;
- in Nano mode, with no system bars, apps saw the square screen as portrait
  and took their portrait layouts: Dolphin drew its game in the top half,
  leaving the bottom half for touch controls. A square screen now counts as
  landscape when the device's orientation is landscape, as GammaOS already did
  for the RG Rotate;
- the Wi-Fi and Bluetooth wizards' result screens (Save completed, the
  connection test result, Register completed), in setup and in Settings,
  showed B for OK, but B went back a screen; they now show A, which continues;
- small text in the theme's own fonts (the game list's font, ES-DE's
  default face) was fully hinted, which left its edges stair-stepped; glyphs are now
  light-hinted, snapping only vertically.

It also changes Nano:

- **Quick Menu:** Power, Global Shaders, Notifications, Wi-Fi, Bluetooth,
  Display (Screen Brightness and the External Display submenu, the same as in
  Display Settings), Device (Performance Mode, Fan Speed, RGB, EQ, Quick Resume
  and Deep Sleep Mode), Controller (Controller Bindings, ABXY Swap, DPAD/Analog
  Swap, Global Sensitivity and stick inversion) and Kiosk Mode. Kiosk Mode is
  the last row while it is off, and the first, as **Kiosk Mode (ON)**, while it
  is on. Over a running app, Close Current App comes just after Power (after
  Resume Audio Player while music plays), and Display also has Screen
  Orientation. Wi-Fi offers On, Off and Scan, and the row under it
  shows the network you are connected to, with its signal bars. Android's own
  Settings app is in **Settings → System Settings → Android Settings**, and
  GammaRGB and GammaEQ are named RGB and EQ.
- **Settings lists:** a setting's name too long for its row wraps onto a second
  line, and a choice too wide for the side panel is drawn smaller to fit.
  **System Information** shows the Thimsy release (Thimsy 1.0a).
- **Anbernic button:** opens the Quick Menu when it is released, on the home
  screen and over a running game (pressing it again, or B, goes back to the
  game). The RG Cube reports it as F10. **Toolbox → Anbernic Button Opens Quick
  Menu** gives the button back to games.
- **Universal hotkeys:** in a game, hold the Anbernic button and press R1 to
  save, L1 to load, R2 for fast forward, L2 for rewind, or hold it on its own
  for 3 seconds to quit, in every emulator that can (**Gamepad Settings → Hotkeys** changes the buttons
  and lists what each emulator supports). [HOTKEYS.md](HOTKEYS.md) describes
  it; `mods/55-hotkeys.sh` installs the emulator profiles from `mods/hotkeys`.
- **Power button:** held for 7 seconds, it shuts the console down, on every
  home. A small service (`nano-powerwatch`) backs that up apart from Android
  and Nano: held 12 seconds with no shutdown started, it asks init for one, and
  a shutdown still running a minute later is ended by powering off from the
  kernel. Only the power chip's own long-press cut-off covers a frozen kernel.
- **Setup's time zone step:** with an internet connection, the clock is set
  from the internet and the time zone is looked up from the connection's IP
  address (ipapi.co, or ip-api.com if that fails) and selected on the globe,
  to confirm or change. A US time zone defaults to MM/DD/YYYY dates and the
  12-hour clock; elsewhere the defaults stay DD/MM/YYYY and 24-hour.
- **USB:** File Transfer (MTP) is applied each time a computer is connected,
  from boot onwards, and kept up while it stays connected.
- **ES-DE theme:**
  - **Settings Theme** (Theme Settings) gives the settings screens the Minima
    theme's look while the game menus keep ES-DE's.
  - **ES-DE Media Folder** (Theme Settings) is ES-DE's game media directory, so
    covers, screenshots and manuals can live on the SD card. Nano's own scraped
    art already can (Boxart Scraper → Boxart Folder).
  - The Start menu gets the focused platform's settings (name, emulator, icon,
    folders, scraper), Game Systems and Boxart Scraper.
  - Select in a game list opens the game's options: Emulator (a different
    emulator for that game), Rename / Edit Title, Set Boxart, Scrape This Game,
    Favorites, Collections, Information and Search Games. The XMB's game option
    menu gets Emulator for This Game too.
  - **Delete Game** (under Rename / Edit Title, in both menus) deletes the
    game's file, and the files a `.cue`, `.gdi`, `.ccd` or `.m3u` uses, after
    asking **Delete Game** or **Delete Game and Media** (cover, fan art,
    screenshots, videos and the rest). It also leaves Favorites, collections,
    Recently Played and, with the ES-DE folder on the card, its gamelist.
  - **Set Boxart** offers the game's own pictures (screenshot, title screen,
    miximage, fan art and the rest) and **Browse Files...** for a PNG or JPEG
    anywhere on the device, the card or a share. The picked cover shows at
    once, in every theme, in place of the theme's scraped cover or miximage.
  - **Search Games** opens the keyboard over the game's options, which stay
    open behind it until a result is chosen.
  - **Information** shows where the game's file is, as ES-DE's metadata editor
    does: its path in the system folder and the system, `subfolder/Game.zip
    [SNES]`.
  - **Touch:** swipe left or right to change system, up to open a system's
    games and down to go back. Tap a system to open it; tap a game to pick it
    and tap it again to start it. Drag a game list, the systems or the menu to
    scroll, and let go while moving to fling. A tap outside the menu closes it.
  - The clock shows in the system and game views, and Nano's own menus (the
    Quick Menu, a game's options) show the theme's clock and battery, Wi-Fi and
    Bluetooth icons, the same as the views.
- **Game lists** put favourites first, each part in name order.
- **Scan Folders:** adding a folder to a system asks **This Folder Only** or
  **Include Subfolders** (selected first), and that choice wins over Scan ROM
  Subfolders either way, which covers only the default `ROMs/<system>`
  folders. Folders added before the choice existed, and the folders Auto-add
  Systems from Folder adds, scan their subfolders, as ES-DE does. A folder
  with its subfolders is marked Subfolders in the list. A folder added while
  a scan is running is scanned as soon as that scan finishes.
- **Game information** pages turn a full page at a time with the shoulder
  buttons, the last line of a page repeated at the top of the next.
- **Background Colour** (Theme Settings) gets a Custom option with a colour
  picker.
- **Slide Behaviour** is hidden on devices that cannot slide (the build sets
  `ro.gammaos.nano.no_slide`).
- **Android Games:** Nano starts an entry as the game's own app rather than
  through the helper app, so it follows the game like any other. Any app's
  options in Applications can **Add to Android Games** or **Remove from
  Android Games**. The Boxart Scraper knows the system (ScreenScraper
  searches by the game's name). A game shows its icon until it has a cover,
  and a scraped or chosen cover replaces the icon in ES-DE themes too.
- **Touch** follows what is on screen: every input registers where it is drawn
  and what touching it does (`NanoTouch.h`): rows, entries, buttons, sliders,
  keyboard keys, dialog buttons and the button hints, which press their button
  ("B Back" goes back). Lists scroll with a drag and fling, in every theme and
  player. Before, each screen worked out its own layout again for touch, and
  taps could land on the wrong item. In the ES-DE system view a swipe moves
  one system, with no fling. A two-finger tap opens or closes the Quick Menu,
  as the Anbernic button does, wherever Nano is on screen. Over a game or app
  it does nothing, so a game's two-thumb touch controls never raise the menu
  (`persist.gammaos.nano.two_finger_quickmenu=true` turns it on over apps and
  games other than the DS emulator). A three-finger tap opens or closes the
  Applications view, as Anbernic + Start does, in Nano's own menus only. Neither
  tap also taps what is under the fingers.
- **Touch on the RG Cube:** its touchscreen is mounted a quarter turn from the
  picture (`ro.input_flinger.primary_touch_orientation=ORIENTATION_90`).
  Android turns touches around for apps, but Nano and drastic-nano read the
  touchscreen directly, so every touch arrived rotated: only the middle of the
  screen worked and vertical drags read as sideways ones. They now turn the
  raw axes back by that mounting, as Android's input reader does, unless the
  device sets Nano's own `osk_touch_*` calibration
  (`persist.gammaos.nano.touch_orientation` overrides it).
- **Dialogs** wrap long text and shrink it to fit, at 640x480 too, in every
  theme; their buttons fit or stack.
- **Nano Keyboard** is the system keyboard in Nano mode. When an app such as
  Brave wants text, it hands the field to Nano, which shows its own on-screen
  keyboard over the app (the one its search uses, driven by the controller),
  starting from the field's text. The app stays undimmed behind it. Start puts the text into the field and
  presses the field's Go, Search or Next; B leaves the field as it was. It lives in the ShaderControl system app;
  Nano selects it at every Nano boot and when setup finishes, trying again for
  half a minute while Android doesn't list it yet (`persist.gammaos.nano.ime=0`
  keeps Android's keyboard), and Boot Android switches back to another
  keyboard.
- **Touch keyboard:** Nano's keyboard opens in one of two forms, by what
  opened it. After a button press it is the controller keyboard (the
  focus-driven grid with its button hints); after a touch, a tap on a field in
  Nano or in an app, it is the touch keyboard: the full width of the screen,
  keys as large as the screen allows with no gaps between them, and the key
  under the finger lit. A button above the keyboard switches between the two,
  and a d-pad or A press on the touch keyboard brings the controller one.
- **ES-DE game lists** (each system's `gamelist.xml`: titles, descriptions,
  dates, favourites) are read on a worker thread. Every system is queued as
  soon as the home has its system list, at idle priority so it only takes CPU
  the boot leaves free; the system on screen moves to the front at normal
  priority. They were read on the drawing thread the first time a system was
  shown, so a large one stalled the screen.
- **Android mode's style overlays:** the 42 icon packs, icon shapes, fonts and
  display cutout simulations are removed (`mods/12-trim-overlays.sh`); Android
  reads every overlay at each boot. The navigation bar modes, the black theme
  and the device configuration overlays stay.
- **Setup:** the first boot's Android setup no longer holds the setup wizard
  on a "Please Wait" screen before Wi-Fi: the wizard goes on, and its Wi-Fi
  scan waits for Android, scanning as soon as it can.
- **Languages:** Nano's prompts, messages and menus, the ES-DE theme and the
  DS emulator's menu included, are translated into the 21 languages the Nano
  Keyboard types: Spanish, French, German, Italian, Portuguese, Dutch,
  Russian, Polish, Turkish, Greek, Hebrew, Arabic, Thai, Japanese, Korean,
  Simplified and Traditional Chinese, Danish, Finnish, Swedish and Norwegian
  (bokmål), the last seven new. The translations are keyed by the English
  text (`ps3xmb/i18n/<code>.json`); a file in `/data/system/nano_i18n/`
  overrides the shipped one.
- **Boxart Scraper** is faster. Covers and fan art for every game come first
  and show as each one lands; screenshots, manuals and videos follow, lighter
  media first. Requests reuse their connections. A ScreenScraper account that
  allows several requests at once (its `maxthreads`) gets that many, up to 4;
  without a user account, or when the limit cannot be read, and for
  TheGamesDB, the scraper makes one request at a time.
- **4:3 Cropped** is a new global shader (Nano's Global Shaders, the video
  player's shader choice and the ShaderControl app). It works on the picture a
  standalone emulator draws, not on the whole screen, whatever it shows: turn
  the shader off for a game it doesn't suit. The game area is the emulator's
  picture when that is within 5% of 4:3, or else the centred 4:3 area of it
  (where an emulator such as Dolphin letterboxes a 4:3 game). On
  the panel the game area fills an 800 × 600 projection: 40 pixels are cut
  from each side and the remaining 720 × 600 is shown with 60 pixels of black
  above and below. On a 1080p display it fills 1600 × 1200: 60 pixels are cut
  from the top and bottom, with black at the sides. Scaling is sharp
  bilinear, and every 4:3 picture is scaled, whatever its size. Only the game
  is filtered, with no extra frame copy; menus,
  overlays and touch controls drawn as their own layers, and the emulator's
  own launcher, settings and pause screens, stay as they are. The filter
  follows the emulator's game view (an Android SurfaceView); an emulator that
  draws its game straight into its window is left alone unless
  `persist.gammaos.shader.4x3.window.<package>` is set to `1`. To leave an
  app alone, add it to `persist.gammaos.shader.4x3.exclude`
  (comma-separated). RetroArch is filtered too: it draws its game into its
  window, so it counts as set for that property, and its own menu is scaled
  with the game. Use RetroArch's 4:3 or Core Provided aspect ratio with the
  global shader, not one of the presets below, or the game is cropped twice.
  RetroArch also has the same shader
  as a Vulkan slang preset, `rg_cube_4_3_cropped.slangp`, in its shader list
  (Shaders > Load, after the folders). Set RetroArch's Aspect Ratio to Full with it,
  so the shader has the whole screen to draw on. It projects every frame the
  core sends, whatever its size, since cores send frames meant for a 4:3
  screen, often with pixels that aren't square. Its projection is sized from
  RetroArch's output rather than in fixed pixels: 10/9 of the output's largest
  4:3 area, which is 800 × 600 on the panel and 1600 × 1200 at 1080p, so it
  crops the same whatever resolution RetroArch renders at.
- **8:7 Cropped** is 4:3 Cropped stretched vertically, for games drawn with
  8:7 pixels such as the SNES's: the 4:3 game area (or a raw 8:7 frame, such
  as a core's 256 × 224) fills 800 × 720 on the panel, with the same 40 pixels
  cut from each side as 4:3 Cropped and no black bars, or 1200 × 1080 at
  1080p.
  `persist.gammaos.shader.4x3.window.<package>` and
  `persist.gammaos.shader.4x3.exclude` are shared with 4:3 Cropped.
  RetroArch's copy is `rg_cube_8_7_cropped.slangp`, also used with Aspect
  Ratio set to Full; it stretches every frame to 800 × 720 (as tall as the
  output's shorter side and 10/9 as wide).
- **Start + Select:** in Nano's menus Start and Select act when released, and
  both pressed together open or close the Quick Menu. In a game, Start +
  Select works like the Anbernic button for hotkeys (Start + Select + R1
  saves), but never opens the Quick Menu, and the game still sees both
  buttons. **Start + Select Hotkeys** (Gamepad Settings → Hotkeys) turns it
  off. See [HOTKEYS.md](HOTKEYS.md).
- **Hold to quit:** holding the Anbernic button, or Start + Select, on its own
  for 3 seconds closes the game; any other button pressed during the hold
  cancels it. The optional Quit Game Button is off by default, and Start and
  Select cannot be it (Anbernic + Start opens Applications).
- **Brightness:** hold the Anbernic button and press Volume Up / Down, in
  Nano's menus and in games alike. Select + Volume is now plain volume.
- **Shader on / off:** hold the Anbernic button and press Select to turn the
  display shader on or off, in Nano's menus, in games and in the DS emulator.
  The selected shader is kept. This replaces Power + Select, which only turned
  it off; Power is plain Power again.
- **Applications:** hold the Anbernic button and press Start to open the
  Applications view, and again (or Start alone) to close it. Over a game the
  overlay rises on it, and closing it, or B, returns to the game. It does
  nothing in the DS emulator. Start and Select can no longer be hotkey buttons.
- **Minima Applications grid:** in the Minima theme, and in the ES-DE theme
  when its Settings Theme is Minima (the ES-DE menu's Applications, Kiosk
  Mode's Start and Anbernic + Start), Applications is a launcher grid: five
  columns of app icons with their names under a search bar that filters as
  you type (Up from the first row, a tap or Select opens the keyboard; Y
  clears it). The d-pad or stick moves around the grid, L1 / R1 turn a page,
  A or a tap starts the app, X or holding a tile opens its options, Y pins
  it, and a drag scrolls, flinging when let go. The XMB and DSi themes, and
  ES-DE with the XMB Settings Theme, keep their lists.
- **Back to the ES-DE home:** when a game or app exits (holding Back or the
  Anbernic button, Close Current App, or the app closing), the
  ES-DE theme returns to its home screen, with any Quick Menu, settings
  screen, Applications or ES-DE menu left open before the launch closed.
- **Browser controls:** with Brave, or [Fulguris](https://github.com/Slion/Fulguris)
  if it is installed, in front, gammapad turns the controller into a mouse and
  the browser's keyboard shortcuts, and the browser's own controller support
  never sees the pad:

  | Control | Action |
  |---|---|
  | A | Enter |
  | B | Back (closes menus) |
  | X | Refresh |
  | Y | Bookmark the page |
  | L1 / R1 | Previous / next tab |
  | R2 / L2 | Left / right click (hold to drag) |
  | Left stick | Scroll |
  | Right stick | Mouse cursor |
  | D-pad | Arrow keys |
  | L3 / R3 | Close tab / new tab |
  | Start | Address bar |
  | Select | Bookmarks |

  Every action also works without thumbsticks: Select + L1 / R1 close and
  open tabs, Select + D-pad scrolls, and Select + A turns cursor mode on or
  off. In cursor mode the D-pad moves the cursor (faster the longer it is
  held), A clicks (hold to drag) and X right-clicks. The Anbernic button (with
  its hotkeys), the Back key and Start + Select (held together) still work as
  everywhere else, and the
  controls step aside while Nano's overlay or keyboard is up, and mouse mode
  turns off while they run.
  `persist.gammaos.gamepad.browser_pkgs` lists the browsers they apply to
  (comma-separated, a trailing `*` matches by prefix; default
  `com.brave.browser,net.slions.fulguris.*`). Fulguris's bookmark shortcut is
  Ctrl+B rather than Chromium's Ctrl+D, so Y sends that there.
- **Move ES-DE to microSD** (Game Settings): moves the ES-DE folder and
  everything Nano keeps about your games to `ES-DE` on the card, in ES-DE's own
  files: favourites, hidden games, titles, metadata and play counts in
  `gamelists/<system>/gamelist.xml`, collections in
  `collections/custom-<name>.cfg`, covers and fan art in
  `downloaded_media/<system>/covers|fanart`. From then on the card is the only
  copy, and its `ES-DE` folder is a library mainline ES-DE can use: put the card
  in a device running ES-DE (or bring one from there) and both see the same
  games, favourites, collections and media. A Nano scrape keeps covers, fan art
  and metadata already in the folder (ES-DE's, for example) unless Overwrite is
  on, and Clear Boxart deletes only pictures Nano downloaded. Systems the
  card's ES-DE ROM folder has games for and Nano does not list yet (Xbox, for
  one) are added; running the move again on a card that already holds ES-DE
  picks them up too.
- **Move Emulator Files to microSD** (Game Settings): moves the settings of
  every emulator GammaOS ships, and the saves and states kept with them, to
  `emulator_config/<emulator>` on the card and points each emulator there:
  RetroArch (its `RetroArch` folder, apart from the box-art thumbnails, which
  stay on internal storage; the shaders, overlays, filters and controller
  profiles it keeps in its app folder stay there), PPSSPP, Flycast, DraStic
  (its whole system folder), M64Plus FZ (its game data, which it copies in and
  out around each game), the built-in DS emulator, ARMSX2 (its data folder, without the BIOS and caches), ScummVM (its
  saves), Vita3K (its whole memory card and system, installed games included)
  and Azahar (its folder, without caches). They reach the card by plain folder
  paths, which GammaOS's MediaProvider allows every app. Azahar only takes a
  folder picked in Android's folder picker, so Nano gives it the card's folder
  that way, through system_server. Dolphin (`Config`, `GameSettings`,
  `StateSaves`, `GC`, `Wii`) and hakuX (its `native` folder: hard disk, save
  states, caches) have no setting for these folders, so each moved folder is
  replaced by a link to the card; apps reach `Android/data` and their own
  folder directly here, not through FUSE, so they follow it. GammaOS keeps
  RetroArch's and WatermelonDS's saves next to the ROMs, so those stay where
  they are. SeedlessDS, if installed, does the same by default; set to keep its
  saves and states in its app folder or a folder of their own, they move to
  `emulator_config/seedlessds`, given to it like Azahar's folder. Its settings
  stay in its app folder, which it has no setting for. An emulator that keeps
  its files on the card needs the card in to start.
- Both moves ask first, show their progress and never overwrite what is already
  on the card. Internal copies of files the card already had go to
  `Internal storage/Nano Backup`, and you choose: keep them there as a backup,
  delete them, or overwrite the card's copies with them.
- **Setup with a microSD card:** after the emulators are installed, setup
  offers **Keep ES-DE on the microSD card** and **Keep emulator files on the
  microSD card** (both Off unless turned on). An option that is on runs its
  move straight away, without questions, with its progress on screen: files the
  card already has are kept, and the internal copies are deleted. Only what has
  files moves; an emulator that has none yet stays as it is. With nothing in
  the internal ES-DE folder and an `ES-DE` folder already on the card, Thimsy
  uses that one. Without a card, setup skips these steps and the BIOS step.
- **BIOS dialog** (the last setup step with a card in, and **Install Your Own
  BIOS Files** in Game Settings at any time): Thimsy includes no BIOS files,
  and the dialog says so first; it copies files you have dumped from your own
  consoles. In setup the step is marked optional (most systems work without
  BIOS files) and points to Game Settings for later. It lists the files the
  emulators need (required, then needed for some games), then the ones that
  only improve compatibility or add features, each with its system, emulators,
  file name and SHA-1. The version shown is the one for the system language's
  region; **Alternates** on a row lists the other versions with their SHA-1.
  **this list** opens All Supported Files: everything the installer accepts, by
  system, with how each file is recognised and how to include it. **Select
  Folder** opens a folder browser (on the card's `bios` folder when there is
  one); every file in the chosen folder is recognised by its contents, whatever
  its name, and copied where each emulator looks for it, with progress and a
  short report (installed, already installed, kept because a different file was
  there, not recognised; a folder with nothing recognised says so and that you
  need to provide your own). **Skip** looks at nothing. Controller (d-pad or
  stick to move, A, B, L1/R1 to page) and touch (tap, drag, fling) both work.
  BIOS files installed before follow a later Move Emulator Files to microSD.
- [BIOS.md](BIOS.md) lists every supported BIOS and firmware file, where each
  one is installed, and how it is matched.
- **Storage access:** every emulator GammaOS ships is granted its storage
  permissions at boot and when it is installed, all-files access included for
  the ones that ask for it (Azahar, ARMSX2, Vita3K), so none of them prompts.
- **Background scraping:** a box-art scrape runs in its own small service
  (`gammaos-nano-scrape`), so it carries on while you browse, start a game and
  come back, from either Nano home. Circle closes the progress window and the
  scrape keeps going; Square cancels it. "Scraping X/Y" shows left of the
  Wi-Fi and battery icons while it runs, and a banner says when it is done.
  Starting another scrape while one runs asks whether to add it to the queue;
  queued scrapes run in turn and add their games to the count ("Scraping
  10/100" becomes "Scraping 10/200"). Square cancels the running scrape and
  everything queued. The service runs at low CPU and disk priority, gives way to
  a game when memory runs short, and uses one connection on devices with about
  1 GB of RAM.
- **Controller Bindings** (Quick Menu) set to Off sends the controller to apps
  as a plain gamepad, for browser games: no browser controls, no Start + Select
  hotkeys or hold to quit (DS games included), and mouse mode turns off with
  its combo. The Anbernic button, its hotkeys and Back still work. It is On
  again after a restart (`sys.gammaos.bindings.enabled`).
- **Setup:** Wi-Fi comes before the time zone. With an internet connection,
  setup sets the clock from the internet, finds the time zone from the
  connection and picks the date and time formats that go with it, then shows
  them on a **Date and Time** screen to confirm; the zone can be changed on the
  globe (returning to that screen) and either format changed there. A Wi-Fi
  connection still coming up gets up to 5 seconds before setup gives up on it.
  With Wi-Fi off, setup goes to the globe as before.
- **Quick Menu Bluetooth:** choosing On keeps the chooser open on Pair Device,
  and the connected devices appear as the radio comes up; Off closes it.
- **Kiosk Mode:** Start opens and closes Applications instead of the ES-DE
  menu.
- **ES-DE boot:** the ES-DE home shows as soon as it is ready, after a few
  black frames that hide its theme art while it decodes.
- **ES-DE menu → Applications** no longer locks the console up when it starts
  an app from the boot home: it hands off like every other launch instead of
  starting the app behind the home, which kept the screen and the controls.
- **The first game after boot:** the Anbernic button opens the Quick Menu and
  holding Back returns to the game list it was launched from. The overlay home,
  which starts at that first launch, reset the hand-off signal
  (`service.bootanim.exit`) the framework reads as "the app has taken over", and
  it now restores the system and game the launch came from.
- **ES-DE menu:** B at the top of the menu closes it.

1.4.4 adds GammaOS Nano, a console-style home. The build makes the device
boot into it from the first boot, where Nano's setup wizard runs first-boot
setup; **Boot Android** in the power menu switches to normal Android (with
its own launcher, Trebuchet), and **Boot Nano** back. `mods/50-nano.sh` sets it up:

- the home theme is Nano's ES-DE theme engine, with
  [Colorful (Simplified)](https://github.com/anthonycaccese/colorful-simplified-es-de)
  with its Colorful (Dark) color scheme and the Theme Aspect Ratio on
  Automatic, which picks the theme's layout for the screen's shape (1:1 on
  the Cube's square screen);
- the theme's fonts are open ones: its commercial Gilroy is replaced by
  [Urbanist](https://github.com/coreyhu/Urbanist), whose letter sizes and
  line height match it, and Akrobat, in the theme and as ES-DE's default face
  in Nano, by [Barlow](https://github.com/jpt/barlow) Semi Condensed, both
  under the SIL Open Font License and downloaded at build time with their
  licenses. Its Quake picture, a Quake Champions render, is left out, so a
  Quake system would show the theme's default picture. Both changes are noted
  in the theme's README;
- the settings screens (Nano Settings and the Quick Menu) have the Minima
  theme's look on a Midnight background (**Theme Settings → Settings Theme**
  sets it back to the XMB's, **Background Colour** changes the colour);
- the menu sounds are off (**Theme Settings → Navigation Sounds** turns them
  on);
- the stick LEDs are on and follow the screen's colour (RGB);
- the performance mode is Max Performance;
- Slide Behaviour is hidden, as the RG Cube cannot slide;
- Quick Settings in Android's shade gets the Force Mirror and Resize to
  External Display tiles;
- its game systems are the ones in `mods/nano-systems.txt`, with this
  build's default emulators: the `D - ` cores from `mods/retroarch-cores.txt`
  (which Nano runs itself) and the standalone apps. ScummVM games are
  `.scummvm` files holding the game's ID in ScummVM's game list, as ES-DE
  uses them; Nano starts the ScummVM app on that game. PS Vita games start
  from Vita3K, because its player needs each game's title ID.

The boot mode, theme, sounds, LEDs and performance mode are `build.prop`
defaults, so changes made in Nano stick.

Nano also gets an **Android Games** system listing the installed Android
games: the apps that declare themselves games, apart from the emulators and
tools setup installs. A small helper app built into the image
(`tools/android-games`) keeps one `.app` file per game in
`/sdcard/ROMs/androidgames`, with the game's icon as its art in
`/sdcard/ES-DE/downloaded_media/androidgames`, updated at boot and when an
app is installed or removed, and starts the game when Nano opens its file.
Nano shows new games after its next scan (at boot, or **Rescan Games**).
Building it needs a JDK 17 (`javac`, `keytool`), the Android build tools
`aapt2`, `d8`, `zipalign` and `apksigner`, and `ANDROID_JAR` set to an API 34
`android.jar`; a GammaOS source tree has all of them after a build.

### Pack-in games

The bundled ROM pack is replaced with one game per platform, each with an
explicit grant to redistribute it and no third-party characters or names.
Each game is installed with its license notice. See
[PACK-IN-GAMES.txt](PACK-IN-GAMES.txt) for the games, their licenses, and the
upstream games that were removed and why; `mods/bundled-roms.txt` is the list
the build uses. Each game also comes with its description and details, box
art, a screenshot, its title screen and a marquee where there is one
(`mods/pack-in-media.txt`), for the ES-DE theme and Nano's other themes. The
box art is drawn for this project in each console's style and some
screenshots are captured from the games; `tools/pack-in-art` has the scripts.

## Building

On Linux with `curl`, 7-Zip (`7zz` or `7z`), `xz`, `zstd`, `unzip`, `e2fsck` and
`resize2fs`, `setfattr`, `flock`, `cc`, `python3`, `sudo` (to loop-mount the system
image), erofs-utils' `fsck.erofs` and `mkfs.erofs` (1.7 or later), AOSP's
`unpack_bootimg`, `lz4` and `avbtool` on `PATH`, `MKBOOTIMG`
naming AOSP's `mkbootimg` (or `mkbootimg.py`), `AVB_KEY` naming AOSP's
`external/avb/test/data/testkey_rsa4096.pem`, and about 40 GB free:

```sh
scripts/build.sh
```

This downloads the official release, applies each script in `mods/` to its
system partition, and writes `out/RG_Cube_Thimsy_OS_v1.0.pac`
(`..._v1.0a.pac` with `SYSTEM_IMG`, and `..._v1.0_ES-DE.pac` with `ESDE_APK`).

- `sudo` asks for its password once, at the start, and the build keeps it
  valid until it ends (it refreshes it every minute in the background). A
  build left unattended past a password prompt it can't answer, for example
  one started without a terminal, needs `NOPASSWD` for the build user in
  sudoers instead.
- One build at a time uses `work/`; a second one stops at once. A build also
  stops when an earlier one left its image mounted or on a loop device.
- The release's PAC is checked against its own CRCs the first time it is used
  (`pactool.py verify`), and its sha256 is recorded next to it; later builds
  check that.
- The RetroArch cores, their info files and the PPSSPP core's
  assets are the libretro buildbot nightlies pinned, by sha256, in
  `mods/retroarch-cores.lock`. `scripts/refresh_cores.sh` (or the build with
  `REFRESH_CORES=1`) downloads the current ones into `work/cache` and rewrites
  the lock. The buildbot keeps only its latest nightly, so a build without
  those cached files fails once a core is rebuilt there, until the lock is
  refreshed. `ALLOW_BUNDLED_CORES=1` lets a core that can't be downloaded as
  pinned keep the base image's copy instead of failing.
- The Android Games helper is signed with `work/cache/nano/android-games.keystore`.
  When it is missing the build stops; `NEW_KEYSTORE=1` creates a new key,
  which devices with the old one won't take as an update.

The tools it uses:

- `scripts/pactool.py` lists, extracts, and replaces images in the Unisoc
  `.pac` firmware package, checks and recomputes its CRCs, and sets the
  product version and alias that flash tools such as ResearchDownload show.
- `scripts/lptool.py` lists, extracts, and replaces logical partitions
  (`system_a`, `vendor_a`, ...) in the raw `super_full.img`.
- `tools/crc16arc.c` is the CRC-16/ARC helper `pactool.py fixcrc` and
  `verify` use.
- `scripts/sprd_drm.py` is the patch to the vendor kernel's DisplayPort driver
  (`sprd-drm.ko`). `scripts/patch_vendor_dlkm.py` applies it to the copy the
  kernel loads at boot, in the compressed EROFS `vendor_dlkm` partition, which
  it rebuilds (`scripts/erofs_image.py`) with every file's owner, mode and
  SELinux label, the original UUID and build time, and no filesystem feature
  the original lacks. `scripts/patch_vendor_boot.py` applies it to the copy
  recovery loads, in `vendor_boot`, then repacks that image and signs it again
  with the AOSP test key it was signed with. The same two scripts change the
  order the kernel modules load in (Boot time, below).
- `scripts/odm_sepolicy.py` compiles the SELinux policy for the new system,
  exactly as init would, and stores it on the odm partition with the system's
  policy hashes. Anbernic's odm carries the policy for the 1.2 system, so
  without this init compiled the policy at every boot, about a second before
  anything else could start.

### Tests

```sh
tests/run.sh          # quick: under a minute; build.sh runs these first (SKIP_TESTS=1 skips them)
tests/run.sh slow     # the slow ones (a PAC past 4 GiB, the mods that need the build cache)
tests/run.sh all
THIMSY_TEST_CACHE=work/cache tests/run.sh slow   # adds the mods that need downloads, from a copy of the cache
```

The tests use only Python's standard library (`python3 -I -m unittest discover
-s tests -t .` runs the quick ones). `tests/unit` covers the scripts in
`scripts/` and lints the data in `mods/`; `tests/e2e` runs the mods on fake
system trees as root in a user namespace (`unshare -Ur`, with `sudo` and
`setfattr` shimmed and no network), runs each twice to check a second run
changes nothing, packs a tree through ext4, a super image and a PAC and reads
it back, and runs first-boot `setup.sh` (from `~/gammaos-src/gammaos`, or
`AOSP=<tree>`) in a chroot with fake Android commands. `tests/fixtures` builds
the images they use. The ext4 tests need `mke2fs`, `debugfs`, `dumpe2fs` and
`e2fsck` (on `PATH` or in the AOSP host build); the mod and `setup.sh` tests
need user namespaces. Tests skip when what they need is missing.

### Boot time

- **Apps compiled after setup:** setup installs the apps without dexopt (it would
  add seconds per APK), and Android's background compile only runs when the
  console is idle and charging. `thimsy-dexopt.sh` runs once, a minute after the
  first boot after setup, at background priority: emulators and games are
  compiled in full (`speed`), every other installed app gets `verify`. It sets
  `persist.thimsy.dexopt_done` when every app compiled, and runs again at the
  next boot otherwise.
- No launch fade: a launch hands off to the game or app at once in every theme
  (the 0.26 s fade and the DSi tile-lift effect are removed).
- The startup sound is decoded while Android is still starting, then waits for
  AudioService, so the home and its sound come about 0.4 s sooner.
- AppStandbyController asks for the network score service, which the minimal
  boot doesn't start, only when it is published: asking for a missing service
  logged a WTF with a stack trace for every installed app as boot completed.
- The image's ART code is compiled for the userfaultfd garbage collector
  (`PRODUCT_ENABLE_UFFD_GC := true`), the one Android 14 uses on the RG Cube's
  kernel. Compiled for the other one, it was thrown away and compiled again on
  the device before the first boot could start, about 23 s of black screen.
- `phh-prop-handler.sh` restarts audioserver for the Bluetooth audio setting
  only when the audio setup actually changes. It re-applied the saved value at
  every boot and restarted audio just after it started; init holds a restart
  back 5 s, and system_server waited about 2 s for it.
- No boot intro: Nano's cold-boot animation, its photosensitivity notice and
  its jingle are removed in every theme, so the home, or the setup wizard on a
  new device, shows as soon as it is ready. The intro ran 13–17 s over a
  system that was ready sooner. Its sounds and images are no longer installed.
- **Boot screen:** from power-on the screen shows "Thimsy", the logo and
  "Loading..." on midnight blue (#1a1a2e, Nano's "Midnight" background
  colour). The bootloader shows it until Android can draw (the BootLogo
  partition's boot picture, `scripts/bootlogo.py`); Nano then draws the same
  picture with a line saying what the device is doing ("Starting Android",
  "Loading your theme", ...) and closes it as soon as the home can be drawn.
  Both are drawn by the same code (`bootscreen/NanoBootArt.cpp` in the Nano
  patch, and its host tool `nano-bootlogo` for the bootloader's picture), from
  the vector logo and Android's Roboto, at the screen's own size, so they look
  the same and stay sharp on any screen, 640x480 included. The charging
  pictures and the fastboot logo are Anbernic's.
- **Android's boot animation**, shown when the console boots into full
  Android, is the Thimsy logo fading in on the same midnight blue, in place
  of GammaOS's cube (`mods/58-bootanimation.sh`). It is drawn from the vector
  logo by `tools/bootanimation/make_bootanimation.py`:

  ```sh
  python3 tools/bootanimation/make_bootanimation.py \
      "$AOSP/frameworks/base/cmds/gammaos-nano/bootscreen/thimsy.svg" \
      mods/bootanimation/bootanimation.zip "$AOSP/vendor/lineage/bootanimation"
  ```

  The last argument also writes the animation into an AOSP tree
  (`bootanimation.tar` and `desc.txt`). Lineage's makefile does not rebuild
  the zip when they change, so delete
  `out/target/product/tdgsi_arm64_ab/obj/BOOTANIMATION/bootanimation.zip`
  first. The mod puts the animation in the image either way, since the patch
  can only carry text.
- **Bootloader wait:** the RG Cube's bootloader is unlocked (Thimsy needs
  that), so at every boot it waits about 10.5 s on the boot picture, checking
  the power button every 100 ms, before it starts Android. The bootloader is
  signed, so that can't be changed, and nothing of Thimsy's runs yet. Tapping
  power once pauses the wait and tapping it again ends it at once; one tap
  alone leaves it paused until the next. The boot picture itself shows only
  the Thimsy title and logo, and fastboot mode shows the same picture instead
  of Anbernic's logo.
- The boot screen redraws at full rate until the home has settled under it
  (then at 10 fps while it waits only for the startup sound). The ES-DE home's
  idle pacing (one redraw a second when nothing moves) applied to it too, so
  the 8 frames it waits for under the home took about 7 s.
- The ES-DE theme's pictures are decoded on four worker threads, at the size
  they are drawn, and only uploaded on the drawing thread. They were decoded
  there one batch per frame, at full size (1080x1080 for each system picture
  of Colorful (Simplified)).
- **Boot sound:** with the ES-DE theme and Startup Sound on (its own setting,
  on by default, apart from the navigation sounds, which are off), the first 2 s
  of Pixabay's "Computer Startup Sound Effect" (Pixabay Content License) play
  as the boot screen gives way to the home. `mods/57-boot-sound.sh` downloads
  it at build time, checks its SHA-256 and installs it; the repository does
  not carry it. Sound can only play once Android's audio is up, so the boot
  screen stays up until it is (a minute at most, if audio never starts) and
  the sound plays as it goes: the home shows about 3–5 s later than without
  it. A game cannot start any sooner either way, since boot only
  completes after audio is up.
- The kernel's random generator: the RG Cube's hardware generator does not feed
  it, so anything wanting random numbers early waited about 2 s for it, and the
  vendor's BoringSSL self tests held up init that long at every boot.
  `nano-seedrng` credits it at early-init with a seed the previous boot saved
  on `/metadata` (deleted as it is read, a fresh one saved once the device is
  up); on a device whose generator is ready it does nothing.
- The LED driver (`cust-leds.ko`) takes a second to start. It was loaded in the
  first stage, where nothing else can run; it now loads in a process of its
  own beside the vendor's module loader (`init.rc` in the Nano patch,
  `scripts/patch_vendor_boot.py`, `scripts/patch_vendor_dlkm.py`). A build
  without `SYSTEM_IMG` keeps the stock system, which lacks that, so the driver
  loads at the end of the module list instead.
- The RG Cube's board lists two touchscreen drivers, FocalTech and Goodix, on
  the same I2C bus and the same reset and interrupt pins (it ships with either
  panel). FocalTech loaded first and spent 1.3 s retrying a chip that is not
  there before it gave the pins up, and the Goodix touch, the controls and the
  charger waited behind it. Goodix now loads first: the fitted panel's driver
  takes the pins and the other one fails at once. They can't load at the same
  time, since both reset the same pin.
- The background module list started with the Wi-Fi/Bluetooth base, which
  pulls in the PCIe driver, and that waits a second for a link the RG Cube
  does not have (nothing is on its PCIe). The display and GPU drivers, the
  controls, touch and audio waited behind it; the radios and PCIe now load
  last.
- SurfaceFlinger, and Nano with it, waited about 1.7 s for the display
  composer, which started with the other vendor services: the early start
  list did not know the RG Cube's name for it, `vendor.hwcomposer-2-4`. On the
  RG Cube, the early start waits for the display and GPU drivers' device nodes
  first, and starts the power HAL with it: the composer waits for that before
  it answers, and the vendor starts it only once `/data` is up, about 2 s later.
- A restart or power-off waited 3 s for services that don't exit when asked:
  `wificond`, and the vendor's GNSS daemon (`gpsd`), which ignores the request.
  Both are now stopped outright. init's own messages during a restart also go
  to pmsg, which pstore keeps across it, so the shutdown can be timed from
  `/sys/fs/pstore` on the next boot (the kernel log does not survive it);
  `nano-seedrng` logs the time from the start of a restart to the next kernel
  starting.
- Nano sets up the USB mode (MTP and the rest) in the background once
  system_server's USB service is up, with a cable in. Before that its helper,
  a Java process, could only fail and retry while the system was starting.

### External monitors

Plugging a monitor into the 1.2 vendor kernel crashed the console, and it kept
rebooting while the monitor stayed attached. On plug-in, the vendor composer
sets the largest mode the monitor lists. For a 3840x2160 mode the driver
reads the screen buffer's format, but that first commit has no buffer, so the
kernel panicked. 4K at 60 Hz also needs more than the two DisplayPort lanes the
Cube trains over USB-C. The driver also discarded the monitor's preferred mode
(it only ever preferred TV 1080p60), so a 1440p monitor that also accepts 4K got 4K.

The patched driver:

- offers only modes up to the bandwidth of 4K at 30 Hz (a 297 MHz pixel
  clock): 4K at 30 Hz, 1440p at 60 Hz, 1080p at 120 Hz and everything below;
- keeps TV 1080p60 the only preferred mode, as the vendor driver does, so the
  composer sets 1080p at 60 Hz as soon as a monitor connects. The vendor
  composer lists only the modes the driver marks preferred or user-defined,
  so the monitor's own preferred mode becomes user-defined instead of being
  dropped (the list is then the monitor's own mode, 1080p60 and 640x480);
- treats a missing buffer as RGB instead of crashing.

Android mirrors the console's windows onto a connected monitor, but Nano's
own screen is a SurfaceFlinger layer, not a window, so the mirror left it out.
Nano puts a mirror of its layer on the monitor, scaled to fit and centred,
so its menus show over the game. The mirror does not follow its source's
visibility, so Nano shows and hides it with its own layer; before, the menu
last drawn stayed over the game on the monitor.

A connected monitor is the main screen by default: External as Primary is
on and Force Mirror off (`persist.gammaos.ext.primary=1`,
`persist.gammaos.ext.force_mirror=false` in `build.prop`, so a change in
Nano's External Display settings sticks). The External Docking, External
Mirror and External Resize Quick Settings tiles read these as on/off values;
before, the mirror tile could not read `false` and turned Force Mirror on
whenever System UI started. With External as Primary, when the
monitor connects the console's screen takes the monitor's size and the
Cube's backlight goes off, so games render at the monitor's resolution. The
settings apply when the monitor next connects.

When a display connects, Nano sets 1920 x 1080 at the rate nearest 60 Hz
without asking (a display without 1080p keeps the mode it came up in). With
`persist.gammaos.nano.ext_ask` set to 1, Nano asks instead which of these
modes to use, with the display's preferred mode (the first timing in its
EDID) selected and the mode in use marked. The choice is saved for that
display in Android's display settings and used whenever it reconnects;
Cancel keeps the current mode. Nano asks once per connection. Connected
during a game, it opens the Quick Menu over the game (pausing it) with the
chooser on top, and goes back to the game once a mode is chosen or the
chooser is cancelled.

The build also grows the system partition by about 431 MB, into the
unallocated space at the end of the `super` image (`scripts/lptool.py grow`);
the other partitions don't move. The upstream system filesystem is built
with deduplicated blocks, which can't be mounted read-write, so the build
grows the filesystem to fill the partition and unshares the blocks. With
all of the changes above, about 250 MB of it is still free.

## Flashing

Flash the result the same way as stock GammaOS Next. Follow the upstream
[installation guide](https://github.com/TheGammaSqueeze/GammaOSNext/wiki).
Flashing erases all user data.

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE). The Thimsy
logo and the pictures drawn from it are under
[CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/): free to use
and redistribute, with credit to the Thimsy OS project, but not for commercial
use.
[ATTRIBUTION.md](ATTRIBUTION.md) credits the art, fonts and sounds in the
image.
