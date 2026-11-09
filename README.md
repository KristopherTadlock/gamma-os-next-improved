# gamma-os-next-improved

A modified build of [GammaOS Next](https://github.com/TheGammaSqueeze/GammaOSNext)
v1.2 (Full edition) for the **Anbernic RG Cube**.

This repository doesn't contain firmware images. It holds the scripts and
file overlays that turn the official GammaOS Next release into the modified
build, so you can rebuild it yourself.

## Base

- Upstream release: [`v.1.2.0-ANBERNICRGCUBE`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRGCUBE)
- Edition: Full (Android 14 / LineageOS 21, with Google Play)
- Prebuilt firmware: [v1.0 release](https://github.com/KristopherTadlock/gamma-os-next-improved/releases/tag/v.1.0.0-ANBERNICRGCUBE)

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

Daijisho's default player for each platform follows the cores marked `D - `
in that file:

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
| Daijisho | 1.4.80 | 1.5.0 |
| Aurora Store | 4.6.4 | 4.8.4 |
| Firefox | 139.0 | replaced by [Brave](https://github.com/brave/brave-browser) 1.99.55 |

Brave is the default browser, set by a fresh setup (`mods/15-update-apps.sh`),
so a browser chosen later stays chosen.

Aurora Store comes from IzzyOnDroid, which only keeps recent versions, so its
link in `mods/15-update-apps.sh` will need updating for later builds.
MiXplorer stays at 6.64.3 because newer builds are only posted on its XDA
thread. Magisk stays at 30.6, the version the boot image is patched with, and
can update itself after setup. See `mods/15-update-apps.sh`.

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
| Dolphin | GameCube, Wii | 2609a | GPL-2.0+ | https://github.com/dolphin-emu/dolphin/tree/409881c34357d0f105fd473167d15ab0bd9c1628 |
| ARMSX2 | PlayStation 2 | 2.9.2 | GPL-3.0 | https://github.com/ARMSX2/ARMSX2/tree/2.9.2 |
| Azahar | Nintendo 3DS | 2126.2 | GPL-3.0 | https://github.com/azahar-emu/azahar/tree/2126.2 |
| WatermelonDS | Nintendo DS | 0.8.0.rc3 | GPL-3.0 | https://github.com/SapphireRhodonite/WatermelonDS/tree/0.8.0.rc3 |
| hakuX | Xbox | 1.3.0 | GPL-3.0+ | https://github.com/rfandango/hakuX/tree/v1.3.0 |
| Vita3K | PS Vita | 0.2.1 (build 4277, mirrored) | GPL-2.0 | https://github.com/Vita3K/Vita3K/tree/2f68c1bf0d44753eddc8263b6a4dd23fcd928f22 |
| ScummVM | Adventure games | 2026.3.0 | GPL-3.0+ | https://github.com/scummvm/scummvm/tree/v2026.3.0 |

The [MaliSX2](https://github.com/bmdhacks/malisx2/tree/v0.4.0) 0.4.0 Vulkan
driver (MIT) comes pre-installed and selected in ARMSX2, with the Vulkan
renderer, and a copy is in `/sdcard/GPU Drivers/`. Dolphin and hakuX only load
custom drivers on Adreno GPUs, so they can't use it on the RG Cube's Mali-G57.

[Shattered Pixel Dungeon](https://github.com/00-Evan/shattered-pixel-dungeon/tree/v4.0.2)
4.0.2 (GPL-3.0), the full game, unmodified, is installed too, so Android Games
starts with a game. It plays with a controller and fits the square screen.

Edit `mods/added-apps.txt` to change the apps.

### ES-DE

Building with `ESDE_APK=/path/to/ES-DE.apk` replaces Daijisho with
[ES-DE](https://es-de.org) (`mods/37-esde.sh`). ES-DE's Android app is paid
and partly closed source, so its APK isn't in this repository; the build
takes it from that path and checks it's the version the build is set up for
(3.5.0). First-boot setup installs it with storage access, makes it the home
app, and puts its settings and game lists in `/sdcard/ES-DE`:

- the ROMs folder, `/sdcard/ROMs`, and the touch overlay turned off;
- for each system, the emulator Daijisho used: the `D - ` core from
  `mods/retroarch-cores.txt` or the standalone app (`mods/esde-emulators.txt`);
- the pack-in games, with KłełeAtoms launching from its folder and Beneath
  a Steel Sky as the `.scummvm` file ES-DE launches ScummVM games with.

ES-DE's first start still runs its setup screens; choose the default folders
(`ES-DE` and `ROMs` on the internal storage) to use these settings. GammaOS's
Magisk module for an ES-DE theme download error on some devices
(`MAGISK_ES-DE_fix.zip`) is copied to `/sdcard/Download`; install it in
Magisk only if theme downloads fail with "Failed to lock file".

Without `ESDE_APK`, the build keeps Daijisho, set up as below.

### 1.4.4 base and the Nano home

Building with `SYSTEM_IMG=/path/to/system.img` uses a GammaOS Next 1.4.4
system image built from GammaOS's source
([GammaOSNextDistribution-14](https://github.com/TheGammaSqueeze/GammaOSNextDistribution-14),
target `arm64_bgN`) in place of the release's 1.2 system. The RG Cube's
vendor, kernel and bootloader stay. The changes above apply to it too, adapted
to 1.4.4's zstd payloads and its app list (`/system/etc/gammaos/apps.list`),
except the Daijisho changes and ES-DE's app: GammaOS Nano replaces Daijisho,
which is removed, along with Beneath a Steel Sky's Daijisho launch file
(ScummVM lists the game itself).

Apply `patches/gammaos-next-1.4.4/0001-nano-fixes-for-the-rg-cube.patch` to the
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
- the Wi-Fi and Bluetooth wizards' result screens (Save completed, the
  connection test result, Register completed), in setup and in Settings,
  showed B for OK, but B went back a screen; they now show A, which continues;
- small text in the theme's own fonts (the game list's Gilroy, ES-DE's
  Akrobat) was fully hinted, which left its edges stair-stepped; glyphs are now
  light-hinted, snapping only vertically.

It also changes Nano:

- **Quick Menu:** the Quick Settings rows (Wi-Fi, Bluetooth, Fan Speed,
  GammaRGB, GammaEQ, Deep Sleep Mode and the controller toggles) are in the
  Quick Menu itself instead of a Quick Settings submenu, ordered with the most
  used first. Wi-Fi offers On, Off and Scan, and the row under it shows the
  network you are connected to, with its signal bars. HDMI options are one
  External Display submenu, the same as in Display Settings. System Settings is
  renamed Android Settings and sits at the bottom. Close Current App and Screen
  Orientation only show over a running app.
- **Anbernic button:** opens the Quick Menu when it is released, on the home
  screen and over a running game (pressing it again, or B, goes back to the
  game). The RG Cube reports it as F10. **Toolbox → Anbernic Button Opens Quick
  Menu** gives the button back to games.
- **Universal hotkeys:** in a game, hold the Anbernic button and press R1 to
  save, L1 to load, R2 for fast forward, L2 for rewind, or hold Start to quit,
  in every emulator that can (**Gamepad Settings → Hotkeys** changes the buttons
  and lists what each emulator supports). [HOTKEYS.md](HOTKEYS.md) describes
  it; `mods/55-hotkeys.sh` installs the emulator profiles from `mods/hotkeys`.
- **Power button:** held for 7 seconds, it shuts the console down.
- **Setup's time zone step:** with an internet connection, the clock is set
  from the internet and the time zone is looked up from the connection's IP
  address (ipapi.co, or ip-api.com if that fails) and selected on the globe,
  to confirm or change. A US time zone defaults to MM/DD/YYYY dates and the
  12-hour clock; elsewhere the defaults stay DD/MM/YYYY and 24-hour.
- **USB Settings:** the mode you choose (Charging Only, File Transfer (MTP),
  Photo Transfer (PTP) or USB Ethernet (RNDIS)) stays on and is applied each
  time a computer is connected, from boot onwards. File Transfer is the default.
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
- **Game lists** put favourites first, each part in name order.
- **Scan Folders:** adding a folder to a system asks **This Folder Only** or
  **Include Subfolders**, and that choice wins over Scan ROM Subfolders either
  way (folders added before keep following it). A folder with its subfolders
  is marked Subfolders in the list.
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
- **Nano Keyboard** is the system keyboard in Nano mode. When an app such as
  Brave wants text, it hands the field to Nano, which shows its own on-screen
  keyboard over the app (the one its search uses, driven by the controller),
  starting from the field's text. Start puts the text into the field and
  presses the field's Go, Search or Next; B leaves the field as it was. It lives in the ShaderControl system app;
  Nano selects it at every Nano boot (`persist.gammaos.nano.ime=0` keeps
  Android's keyboard), and Boot Android switches back to another keyboard.
- **Boxart Scraper** is faster. Covers and fan art for every game come first
  and show as each one lands; screenshots, manuals and videos follow, lighter
  media first. Requests reuse their connections. A ScreenScraper account that
  allows several requests at once (its `maxthreads`) gets that many, up to 4;
  without a user account, or when the limit cannot be read, and for
  TheGamesDB, the scraper makes one request at a time.
- **4:3 Cropped** is a new global shader (Nano's Global Shaders, the video
  player's shader choice and the ShaderControl app). It works on the picture a
  standalone emulator draws, not on the whole screen, and only when that
  picture is within 5% of 4:3. A picture of any other shape, such as an
  Android game drawn for the square screen, is left as it is. On
  the panel the 4:3 picture fills an 800 × 600 projection: 40 pixels are cut
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
  (comma-separated). RetroArch is always left alone: it has the same shader
  as a Vulkan slang preset, `rg_cube_4_3_cropped.slangp`, in its shader list
  (Shaders > Load, after the folders). Set RetroArch's Aspect Ratio to Full with it,
  so the shader has the whole screen to draw on. It projects every frame the
  core sends, whatever its size, since cores send frames meant for a 4:3
  screen, often with pixels that aren't square.
- **8:7 Cropped** is 4:3 Cropped stretched vertically, for games drawn with
  8:7 pixels such as the SNES's: a 4:3 picture (or a raw 8:7 frame, such
  as a core's 256 × 224) fills 800 × 720 on the panel, with the same 40 pixels
  cut from each side as 4:3 Cropped and no black bars, or 1200 × 1080 at
  1080p.
  `persist.gammaos.shader.4x3.window.<package>` and
  `persist.gammaos.shader.4x3.exclude` are shared with 4:3 Cropped.
  RetroArch's copy is `rg_cube_8_7_cropped.slangp`, also used with Aspect
  Ratio set to Full; it stretches every frame to 800 × 720.
- **Start + Select:** in Nano's menus Start and Select act when released, and
  both pressed together open or close the Quick Menu. In a game, Start +
  Select works like the Anbernic button for hotkeys (Start + Select + R1
  saves), but never opens the Quick Menu, and the game still sees both
  buttons. **Start + Select Hotkeys** (Gamepad Settings → Hotkeys) turns it
  off. See [HOTKEYS.md](HOTKEYS.md).
- **Hold to quit:** holding the Anbernic button, or Start + Select, on its own
  for 3 seconds closes the game; any other button pressed during the hold
  cancels it. This replaces Anbernic + Start (the Quit Game Button is now off
  by default).
- **Setup:** Wi-Fi comes before the time zone. With an internet connection,
  setup sets the clock from the internet, finds the time zone from the
  connection and picks the date and time formats that go with it, then shows
  them on a **Date and Time** screen to confirm; the zone can be changed on the
  globe (returning to that screen) and either format changed there. Offline,
  setup goes to the globe as before.
- **Quick Menu Bluetooth:** choosing On keeps the chooser open on Pair Device,
  and the connected devices appear as the radio comes up; Off closes it.
- **Kiosk Mode:** Start opens and closes Applications instead of the ES-DE
  menu.
- **ES-DE boot:** the cold-boot intro ends in black after the warning instead
  of revealing the XMB menu, the ES-DE theme loads while the warning is up,
  and the ES-DE home fades in.
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
its own launcher), and **Boot Nano** back. `mods/50-nano.sh` sets it up:

- the home theme is Nano's ES-DE theme engine, with
  [Colorful (Simplified)](https://github.com/anthonycaccese/colorful-simplified-es-de)
  with its Colorful (Dark) color scheme and the Theme Aspect Ratio on
  Automatic, which picks the theme's layout for the screen's shape (1:1 on
  the Cube's square screen);
- the settings screens (Nano Settings and the Quick Menu) have the Minima
  theme's look on a Midnight background (**Theme Settings → Settings Theme**
  sets it back to the XMB's, **Background Colour** changes the colour);
- the menu sounds are off (**Theme Settings → Navigation Sounds** turns them
  on);
- the stick LEDs are on and follow the screen's colour (GammaRGB);
- the performance mode is Max Performance;
- Slide Behaviour is hidden, as the RG Cube cannot slide;
- Quick Settings in Android's shade gets the Force Mirror and Resize to
  External Display tiles;
- its game systems are the ones in `mods/nano-systems.txt`, with this
  build's default emulators: the `D - ` cores from `mods/retroarch-cores.txt`
  (which Nano runs itself) and the standalone apps. ScummVM and PS Vita games
  start from their apps, because their players need each game's ID.

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

### Daijisho platforms

Daijisho gets 11 more platforms, each from Daijisho's official definition:
GameCube, Wii, 3DS, Xbox and ScummVM for the added emulators, and Virtual Boy,
32X, TurboGrafx-16 (PC Engine HuCards), Atari 2600, Atari 5200 and Atari 7800,
which have pack-in games and RetroArch cores but no platform upstream. Systems
that need a BIOS (TurboGrafx-CD, Saturn, Lynx, 3DO, ColecoVision, MSX) keep
their RetroArch cores but get no platform or pack-in game. Edit
`mods/daijisho-platforms.txt` to change them.

GammaOS's platform wallpapers come from the "BB-1 Dark" pack by
BrunoBelmonte in Daijisho's wallpaper pack collection, and the new platforms
get theirs from the same pack (`mods/daijisho-wallpapers.txt`). The pack has
no Xbox wallpaper, so `scripts/make_wallpapers.py` draws one in its layout
with Second Reality's title picture. The pack's wallpapers for the systems
that need a BIOS are copied to `/sdcard/Daijisho/Wallpapers/`, to use if you
add one of those platforms to Daijisho later.

Daijisho's player lists drop the players that can't work here: the
32-bit and Play Store RetroArch players (only the 64-bit RetroArch is
installed) and RetroArch players for cores that aren't shipped. Players for
standalone apps stay, since you can install those apps.

Upstream ships Daijisho with its game library already synced, listing the
removed games. The build rebuilds the library from the games it ships, and
makes Daijisho skip the license and `systeminfo.txt` files in the ROMs
folders when you sync again.

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
`resize2fs`, `setfattr`, `cc`, `python3`, `sudo` (to loop-mount the system
image), erofs-utils' `fsck.erofs` and `mkfs.erofs` (1.7 or later), AOSP's
`unpack_bootimg`, `lz4` and `avbtool` on `PATH`, `MKBOOTIMG`
naming AOSP's `mkbootimg` (or `mkbootimg.py`), `AVB_KEY` naming AOSP's
`external/avb/test/data/testkey_rsa4096.pem`, and about 40 GB free:

```sh
scripts/build.sh
```

This downloads the official release, applies each script in `mods/` to its
system partition, and writes `out/RG_Cube_GammaOS_Next_Improved_v1.0.pac`
(`..._v2.0.pac` with `SYSTEM_IMG`, and `..._v1.0_ES-DE.pac` with `ESDE_APK`).

The tools it uses:

- `scripts/pactool.py` lists, extracts, and replaces images in the Unisoc
  `.pac` firmware package, recomputes its CRCs, and sets the product version
  and alias that flash tools such as ResearchDownload show.
- `scripts/lptool.py` lists, extracts, and replaces logical partitions
  (`system_a`, `vendor_a`, ...) in the raw `super_full.img`.
- `tools/crc16arc.c` is the CRC-16/ARC helper `pactool.py fixcrc` uses.
- `scripts/sprd_drm.py` is the patch to the vendor kernel's DisplayPort driver
  (`sprd-drm.ko`). `scripts/patch_vendor_dlkm.py` applies it to the copy the
  kernel loads at boot, in the compressed EROFS `vendor_dlkm` partition, which
  it rebuilds with every file's owner, mode and SELinux label, the original
  UUID and build time, and no filesystem feature the original lacks.
  `scripts/patch_vendor_boot.py` applies it to the copy recovery loads, in
  `vendor_boot`, then repacks that image and signs it again with the AOSP test
  key it was signed with.

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
so its menus show over the game.

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

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
