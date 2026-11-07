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

Aurora Store comes from IzzyOnDroid, which only keeps recent versions, so its
link in `mods/15-update-apps.sh` will need updating for later builds.
MiXplorer stays at 6.64.3 because newer builds are only posted on its XDA
thread. Magisk stays at 30.6, the version the boot image is patched with, and
can update itself after setup. See `mods/15-update-apps.sh`.

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

1.4.4 adds GammaOS Nano, a console-style home. The build makes the device
boot into it from the first boot, where Nano's setup wizard runs first-boot
setup; **Boot Android** in the power menu switches to normal Android (with
its own launcher), and **Boot Nano** back. `mods/50-nano.sh` sets it up:

- the home theme is Nano's ES-DE theme engine, with
  [Colorful (Simplified)](https://github.com/anthonycaccese/colorful-simplified-es-de)
  in its 1:1 layout for the square screen and its Colorful (Dark) color scheme;
- its game systems are the ones in `mods/nano-systems.txt`, with this
  build's default emulators: the `D - ` cores from `mods/retroarch-cores.txt`
  (which Nano runs itself) and the standalone apps. ScummVM and PS Vita games
  start from their apps, because their players need each game's ID.

The boot mode and theme are `build.prop` defaults, so changes made in Nano
stick.

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
the build uses.

## Building

On Linux with `curl`, 7-Zip (`7zz` or `7z`), `xz`, `zstd`, `unzip`, `e2fsck` and
`resize2fs`, `setfattr`, `cc`, `python3`, `sudo` (to loop-mount the system
image), and about 40 GB free:

```sh
scripts/build.sh
```

This downloads the official release, applies each script in `mods/` to its
system partition, and writes `out/RG_Cube_GammaOS_Next_Improved_v1.0.pac`
(`..._v2.0.pac` with `SYSTEM_IMG`).

The tools it uses:

- `scripts/pactool.py` lists, extracts, and replaces images in the Unisoc
  `.pac` firmware package, recomputes its CRCs, and sets the product version
  and alias that flash tools such as ResearchDownload show.
- `scripts/lptool.py` lists, extracts, and replaces logical partitions
  (`system_a`, `vendor_a`, ...) in the raw `super_full.img`.
- `tools/crc16arc.c` is the CRC-16/ARC helper `pactool.py fixcrc` uses.

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
