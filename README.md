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

On Linux with `curl`, 7-Zip (`7zz` or `7z`), `xz`, `unzip`, `e2fsck` and
`resize2fs`, `setfattr`, `cc`, `python3`, `sudo` (to loop-mount the system
image), and about 40 GB free:

```sh
scripts/build.sh
```

This downloads the official release, applies each script in `mods/` to its
system partition, and writes `out/RG_Cube_GammaOS_Next_Improved_v1.0.pac`.

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
