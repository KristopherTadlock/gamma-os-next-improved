# BIOS and system files in Thimsy OS

Some emulators in Thimsy OS need files from the original consoles (BIOS, boot ROMs, firmware) to run, or run better with them. Thimsy OS does not include or download any of them; you provide your own. This page lists every file Thimsy OS can install, how it recognises them and where each one goes.

## Installing

Put the files in a folder (any folder, any names, subfolders are fine), then either:

- in the first-boot setup, at the BIOS step, choose **Install** and pick the folder (a folder named `bios` on the microSD card is offered first), or
- any time later: **Game Settings > Install BIOS Files**.

Thimsy OS looks through the whole folder, installs everything it recognises and shows a short report: installed, already installed, different file already there (kept), not recognised.

## How files are recognised

- **By content, not by name.** Every file whose size matches a known file is checked by its SHA-1 (or MD5 where no SHA-1 is known). The name in your folder doesn't matter: each file is copied under the name its emulators need, to every emulator that uses it (one PlayStation BIOS, for example, serves all three PlayStation cores).
- **Arcade BIOS zips** (`neogeo.zip`, `pgm.zip`, `qsound.zip`, `stvbios.zip`) are recognised by the files inside them (name, size and CRC32, then each one's SHA-1) and installed as the whole zip under its proper name. Other zips are ignored: Thimsy OS never takes files out of a zip. 7z and rar files are ignored.
- **Any PS2 BIOS** of 4 MB with a valid ROMDIR and ROMVER is accepted (ARMSX2 checks it the same way), even if it isn't in the list below; it keeps its own name.
- **PS Vita system software** is recognised by its update package header, any version.
- **Files marked "name and size"** are console-unique or come in many versions, so they are matched by their exact file name (and size where shown). They must keep that name. They are only copied where no such file exists yet.
- A DS `firmware.bin` must also carry the firmware's "MAC" mark, and an Xbox `bios.bin` must be exactly 256 KB or 1 MB.
- Files larger than 1 GB are never looked at, and links leading outside the chosen folder are not followed.

## Installer behaviour

- Nothing is ever overwritten. If the same file is already there it counts as already installed; if a different file is there it is kept and reported.
- Each file is written to a temporary file next to its destination, synced, then renamed into place, and given the owner, permissions and SELinux label the emulator's folder needs.
- Thimsy OS only touches an emulator when at least one file for it was newly installed in this run: only then does it stop the app, change its settings (ARMSX2's BIOS choice, M64Plus FZ's 64DD IPL, ScummVM's extra path, Vita3K's first-run flag), set ownership or labels, or create its folders. A folder with nothing recognised changes nothing at all.
- An emulator app that isn't installed is skipped and reported as not installed.
- Before copying, the free space on each drive is checked (each file rounded up to the drive's block size, plus 8 MB spare). Files for a drive without room are skipped with a message; files for other drives still install.
- Where an emulator uses one file of several (ARMSX2's BIOS, YabaSanshiro's `saturn_bios.bin`), the version for your region is chosen from the system language: Japanese selects Japan; English (US) and other languages of the Americas select USA; European languages select Europe; others select USA. All matched versions are installed anyway. ARMSX2's choice is only set when it has none or its file is gone.
- **PS Vita:** Vita3K installs its firmware itself. Thimsy OS copies the package into Vita3K's folder, marks Vita3K's first-run setup as done, starts Vita3K and asks it to install the package (about a minute; Vita3K's window shows meanwhile), then closes it. This is skipped when the firmware is already installed, or when there isn't room for about three times the package size.
- What was installed where is remembered in `/data/system/nano_bios/installed.json`. After **Move Emulator Files to microSD**, files that are now missing where an emulator looks are copied again from a copy that is still there (most move with their emulator's folder anyway).

## Bundled

- **C-BIOS 0.29a** for MSX (free, 2-clause BSD licence; the licence is in each machine folder and in NOTICE) is shipped in RetroArch's `system/Machines` (`MSX - C-BIOS`, `MSX2 - C-BIOS`, `MSX2+ - C-BIOS`). It runs MSX cartridge games. The real blueMSX `Machines` and `Databases` folders add disk support; installing them keeps the C-BIOS machines as they are.

## Never installed

- **Games, and files that belong to a game**: for example Sonic & Knuckles' `sk.bin` / `sk2chip.bin`, the KOF95 and Ultraman Saturn cartridges, CPS-3 game sets, per-game BIOS files such as `3do_arcade_saot.bin`. These are games, not system files.
- **Cheat cartridges** (Game Genie, Action Replay ROMs): not system files.
- **Flycast's arcade BIOS sets** (`naomi.zip`, `naomi2.zip`, `awbios.zip`, `hod2bios.zip`, `f355bios.zip`, `f355dlx.zip`, `airlbios.zip`): left out on purpose.
- **WatermelonDS** keeps its built-in free BIOS: its custom BIOS mode also needs the console-unique DS firmware, so Thimsy OS doesn't switch it. DSi files are not installed for the same reason.

These systems need no BIOS: PSP, Xbox, 3DS (decrypted games), N64 cartridges, Virtual Boy, Atari 2600, WonderSwan, Neo Geo Pocket, PICO-8, 32X, and CPS-1/CPS-2 in FBNeo need no BIOS.

## Listed in setup

The setup step and Game Settings > Install BIOS Files list these (the files for your region are shown; the others are under Alternates). Everything in the next section is installed, listed or not.

| | System | Emulators | Files |
|---|---|---|---|
| Required | PlayStation 2 | ARMSX2 | `ps2-0230a-20080220.bin`, `ps2-0230e-20080220.bin`, `ps2-0230j-20080220.bin`, `ps2-0220a-20060210.bin`, `ps2-0200a-20040614.bin`, `ps2-0200e-20040614.bin`, `ps2-0160a-20020207.bin`, `ps2-0100j-20000117.bin` |
| Required | PS Vita | Vita3K | `PSVUPDAT.PUP`, `PSP2UPDAT.PUP` |
| Required | Saturn | YabaSanshiro, Beetle Saturn | `mpr-17933.bin`, `sega_101.bin`, `saturn_bios.bin` |
| Required | 3DO | Opera | `panafz10.bin`, `panafz1.bin`, `panafz1j.bin`, `goldstar.bin`, `sanyotry.bin`, `panafz10e-anvil.bin` |
| Required for Sega CD games | Sega CD | Genesis Plus GX, PicoDrive | `bios_CD_U.bin`, `bios_CD_E.bin`, `bios_CD_J.bin` |
| Required for CD games | TurboGrafx-16 / PC Engine | Beetle PCE Fast | `syscard3.pce`, `syscard2.pce`, `syscard1.pce`, `gexpress.pce` |
| Required for Neo Geo games | Neo Geo | FBNeo, MAME, MAME 2003-Plus | `neogeo.zip` |
| Required for PGM games | PGM | FBNeo, MAME, MAME 2003-Plus | `pgm.zip` |
| Required for CPS-2 games in MAME | CPS-2 | MAME | `qsound.zip` |
| Optional | PlayStation | Beetle PSX HW, SwanStation, PCSX ReARMed | `scph5501.bin`, `scph5502.bin`, `scph5500.bin`, `scph101.bin`, `scph7001.bin`, `scph1001.bin`, `psxonpsp660.bin` |
| Optional | Dreamcast | Flycast | `dc_boot.bin` |

## All files by system

"Listed" means shown in the setup dialog. "How" is how the file must be in your folder.

### 3DO

Emulators: Opera. Required. Required for Japanese games that use Kanji fonts (not listed in setup). Optional (not listed in setup). BIOS variants without the RSA check.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `panafz10.bin` Panasonic FZ-10 | 1 MB | `3c912300775d1ad730dc35757e279c274c0acaad` | `51f2f43ae2f3508a14d9f56597e2d3ce` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz10.bin` |
| `panafz1.bin` Panasonic FZ-1 | 1 MB | `34bf189111295f74d7b7dfc1f304d98b8d36325a` | `f47264dd47fe30f73ab3c010015c155b` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz1.bin` |
| `panafz1j.bin` Panasonic FZ-1J (Japan) | 1 MB | `ec7ec62d60ec0459a14ed56ebc66761ef3c80efc` | `a496cfdded3da562759be3561317b605` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz1j.bin` |
| `goldstar.bin` GoldStar GDO-101M | 1 MB | `c4a2e5336f77fb5f743de1eea2cda43675ee2de7` | `8639fd5e549bd6238cfee79e3e749114` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/goldstar.bin` |
| `sanyotry.bin` Sanyo IMP-21J TRY | 1 MB | `b01c53da256dde43ffec4ad3fc3adfa8d635e943` | `35fa1a1ebaaeea286dc5cd15487c13ea` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/sanyotry.bin` |
| `panafz10e-anvil.bin` Panasonic FZ-10-E (Anvil) | 1 MB | `a900371f0cdcdc03f79557f11d406fd71251a5fd` | `a48e6746bd7edec0f40cff078f0bb19f` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz10e-anvil.bin` |
| `panafz1-kanji.bin` Panasonic FZ-1 Kanji ROM | 933636 bytes | `acd39a8fee1b9d2950d5ab447846c11fb31af63e` | `b8dc97f778a6245c58e064b0312e8281` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz1-kanji.bin` |
| `panafz1j-kanji.bin` Panasonic FZ-1J Kanji ROM | 1 MB | `884515605ee243577ab20767ef8c1a7368e4e407` | `c23fb5d5e6bb1c240d02cf968972be37` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz1j-kanji.bin` |
| `panafz10ja-anvil-kanji.bin` Panasonic FZ-10JA Kanji ROM | 1 MB | `2e857b957803d0331fd229328df01f3ffab69eee` | `428577250f43edc902ea239c50d2240d` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz10ja-anvil-kanji.bin` |
| `panafz10-norsa.bin` Panasonic FZ-10 | 1 MB | `f05e642322c03694f06a809c0b90fc27ac73c002` | `1477bda80dc33731a65468c1f5bcbee9` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz10-norsa.bin` |
| `panafz10e-anvil-norsa.bin` Panasonic FZ-10-E (Anvil) | 1 MB | `2765c7b4557cc838b32567d2428d088980295159` | `cf11bbb5a16d7af9875cca9de9a15e09` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz10e-anvil-norsa.bin` |
| `panafz1j-norsa.bin` Panasonic FZ-1J | 1 MB | `a417587ae3b0b8ef00c830920c21af8bee88e419` | `f6c71de7470d16abe4f71b1444883dc8` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/panafz1j-norsa.bin` |

### Atari 5200

Emulators: a5200. Optional (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `5200.rom` | 2 KB | `6ad7a1e8c9fad486fbec9498cb48bf5bc3adc530` | `281f20ea4320404ec820fb7ec0693b38` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/5200.rom` |

### Atari 7800

Emulators: ProSystem. Optional (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `7800 BIOS (U).rom` USA | 4 KB | `d9d134bb6b36907c615a594cc7688f7bfcef5b43` | `0763f1ffb006ddbe32e52d497ee848ae` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/7800 BIOS (U).rom` |
| `7800 BIOS (E).rom` Europe | 16 KB | `5a140136a16d1d83e4ff32a19409ca376a8df874` | `397bb566584be7b9764e7a68974c4263` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/7800 BIOS (E).rom` |

### Atari Lynx

Emulators: Beetle Lynx. Required (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `lynxboot.img` | 512 bytes | `e4ed47fae31693e016b081c6bda48da5b70d7ccb` | `fcd403db69f54290b51035d82f835e7b` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/lynxboot.img` |

### ColecoVision

Emulators: GearColeco. Required (not listed in setup). Required for ADAM software (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `colecovision.rom` | 8 KB | `45bedc4cbdeac66c7df59e9e599195c778d86a92` | `2c66f5911e5b42b8ebe113403548eee7` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/colecovision.rom` |
| `eos.rom` ADAM EOS | 8 KB | MD5 only | `01df3140909f09aa9aac4f88890f676c` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/eos.rom` |
| `writer.rom` ADAM SmartWriter | 32 KB | MD5 only | `4fe4f6800076ea3d897d4285653447bd` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/writer.rom` |

### CPS-2

Emulators: MAME. Required for CPS-2 games in MAME. Recognised by its contents.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `qsound.zip` QSound sound chip | zip | recognised by its contents |  | yes | Put it anywhere in the folder you pick (any name; zips are opened) | RetroArch `system/mame/bios/qsound.zip` |

`qsound.zip` is recognised when it holds `dl-1425.bin` (checked by name, size and CRC32, then SHA-1). Known members:

| Member | Size | CRC32 | SHA-1 |
|---|---|---|---|
| `dl-1425.bin` | 8192 | `d6cf5ef5` | `555f50fe5cdf127619da7d854c03f4a244a0c501` |

Other files in the zip (other BIOS versions such as UniBIOS) come along with it.

### Dreamcast

Emulators: Flycast. Optional. A few games need it. Optional (not listed in setup). Flash memory (settings) of a Dreamcast.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `dc_boot.bin` | 2 MB | `8951d1bb219ab2ff8583033d2119c899cc81f18c` | `e10c53c2f8b90bab96ead2d368858623` | yes | Put it anywhere in the folder you pick (any name) | Flycast `data/dc_boot.bin`; RetroArch `system/dc/dc_boot.bin` |
| `dc_nvmem.bin` Flash memory | 128 KB | matched by name and size |  | no | Must keep this exact name | Flycast `data/dc_nvmem.bin`; RetroArch `system/dc/dc_nvmem.bin` (only if none there) |
| `dc_flash.bin` Flash memory | 128 KB | matched by name and size |  | no | Must keep this exact name | Flycast `data/dc_nvmem.bin`; RetroArch `system/dc/dc_nvmem.bin` (only if none there) |

### Game Boy / Game Boy Color

Emulators: SameBoy, mGBA. Optional (not listed in setup). Shows the console's start-up screen. Optional (not listed in setup). Game Boy games in Game Boy Advance mode.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `dmg_boot.bin` Game Boy | 256 bytes | `4ed31ec6b0b175bb109c0eb5fd3d193da823339f` | `32fbbd84168d3482956eb3c5051637f5` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dmg_boot.bin`; RetroArch `system/gb_bios.bin` |
| `cgb_boot.bin` Game Boy Color | 2304 bytes | `1293d68bf9643bc4f36954c1e80e38f39864528d` | `dbfce9db9deaa2567f6a84fde55f9680` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/cgb_boot.bin`; RetroArch `system/gbc_bios.bin` |
| `sgb_boot.bin` Super Game Boy | 256 bytes | `aa2f50a77dfb4823da96ba99309085a3c6278515` | `d574d4f9c12f305074798f54c091a8b4` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/sgb_boot.bin`; RetroArch `system/sgb_bios.bin` |
| `sgb2_boot.bin` Super Game Boy 2 | 256 bytes | `93407ea10d2f30ab96a314d8eca44fe160aea734` | `e0430bca9925fb9882148fd2dc2418c1` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/sgb2_boot.bin` |
| `agb_boot.bin` Game Boy Advance | 256 bytes | matched by name and size |  | no | Must keep this exact name | RetroArch `system/agb_boot.bin` (only if none there) |

### Game Boy Advance

Emulators: mGBA, gpSP. Optional (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `gba_bios.bin` | 16 KB | `300c20df6731a33952ded8c436f7f186d25d3492` | `a860e8c0b6d573d191e4ec7db1b1e4f6` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/gba_bios.bin` |

### GameCube

Emulators: Dolphin. Optional (not listed in setup). Plays the console's start-up animation. Optional (not listed in setup). DSP ROMs from a GameCube (Dolphin has its own).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `IPL.bin` Europe v1.2 | 2 MB | `80b8744ff5e43585392f55546bd03a673d11ef5f` | `db92574caab77a7ec99d4605fd6f2450` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/EUR/IPL.bin` |
| `IPL.bin` Europe v1.1 | 2 MB | `f3cd0c7c61cbcefa85e7de3aff4cfa50bc508714` | `339848a0b7c2124cf155276c1e79cbd0` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/EUR/IPL.bin` |
| `IPL.bin` Europe v1.0 | 2 MB | `f27c63e5394e2fd1606f70df004c4fc2d6027700` | `0cdda509e2da83c85bfe423dd87346cc` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/EUR/IPL.bin` |
| `IPL.bin` NTSC v1.2 (USA, Japan) | 2 MB | `ef9194ab4804aa0aa8540d846caf291b28331165` | `b17148254a5799684c7d783206504926` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/USA/IPL.bin`; Dolphin `GC/JAP/IPL.bin` |
| `IPL.bin` NTSC v1.1 (USA, Japan) | 2 MB | `239eacd86527ff9a75aeb7282da65797baeef010` | `019e39822a9ca3029124f74dd4d55ac4` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/USA/IPL.bin`; Dolphin `GC/JAP/IPL.bin` |
| `IPL.bin` NTSC v1.0 (USA, Japan) | 2 MB | `a1837968288253ed541f2b11440b68f5a9b33875` | `fc924a7c879b661abc37cec4f018fdf3` | no | Put it anywhere in the folder you pick (any name) | Dolphin `GC/USA/IPL.bin`; Dolphin `GC/JAP/IPL.bin` |
| `dsp_rom.bin` DSP ROM | 8 KB | matched by name and size |  | no | Must keep this exact name | Dolphin `GC/dsp_rom.bin` (only if none there) |
| `dsp_coef.bin` DSP coefficients | 4 KB | matched by name and size |  | no | Must keep this exact name | Dolphin `GC/dsp_coef.bin` (only if none there) |

### Genesis / Master System / Game Gear

Emulators: Genesis Plus GX. Optional (not listed in setup). Shows the console's start-up screen.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `bios_MD.bin` Genesis | 2 KB | `3f50b76b0529db7f79c396b5e808cc0786ffc311` | `d3293ebaaa7f4eb2a6766b68a0fb4609` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_MD.bin` |
| `bios_U.sms` Master System (USA, Europe) | 8 KB | `c315672807d8ddb8d91443729405c766dd95cae7` | `840481177270d5642a14ca71ee72844c` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_U.sms`; RetroArch `system/bios_E.sms` |
| `bios_J.sms` Master System (Japan) | 8 KB | `a8c1b39a2e41137835eda6a5de6d46dd9fadbaf2` | `24a519c53f67b00640d0048ef7089105` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_J.sms` |
| `bios.gg` Game Gear | 1 KB | `914aa165e3d879f060be77870d345b60cfeb4ede` | `672e104c3be3a238301aceffc3b23fd6` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios.gg` |

### MSX

Emulators: blueMSX. Required for MSX disk games (not listed in setup). Cartridge games already run on the built-in free C-BIOS.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `Machines, Databases` blueMSX 2.8.2 | folder | folder; recognised by `Databases/msxromdb.xml` (`573aa830b7bef4452808b40df883469907db2fed`) and `Machines/Shared Roms/MSX.rom` (`e998f0c441f4f1800ef44e42cd1659150206cf79`) |  | no | Copy the Machines and Databases folders as they are | RetroArch `system/Databases/` (whole folder); RetroArch `system/Machines/` (whole folder) |

### Neo Geo

Emulators: FBNeo, MAME, MAME 2003-Plus. Required for Neo Geo games. Recognised by its contents. MAME 2003-Plus reads it from the game's folder.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `neogeo.zip` Neo Geo BIOS set | zip | recognised by its contents |  | yes | Put it anywhere in the folder you pick (any name; zips are opened) | RetroArch `system/fbneo/neogeo.zip`; RetroArch `system/mame/bios/neogeo.zip` |

`neogeo.zip` is recognised when it holds `sm1.sm1`, `sfix.sfix`, `000-lo.lo`, `sp-s3.sp1` or `sp-s2.sp1` or any `sp-...` file (checked by name, size and CRC32, then SHA-1). Known members:

| Member | Size | CRC32 | SHA-1 |
|---|---|---|---|
| `sp-s3.sp1` | 131072 | `91b64be3` | `720a3e20d26818632aedf2c2fd16c54f213543e1` |
| `sp-s2.sp1` | 131072 | `9036d879` | `4f5ed7105b7128794654ce82b51723e16e389543` |
| `sm1.sm1` | 131072 | `94416d67` | `42f9d7ddd6c0931fd64226a60dc73602b2819dcf` |
| `sfix.sfix` | 131072 | `c2ea0cfd` | `fd4a618cdcdbf849374f0a50dd8efe9dbab706c3` |
| `000-lo.lo` | 131072 | `5a86cff2` | `5992277debadeb64d1c1c64b0a92d9293eaf7e4a` |
| `sfix.sfx` | 131072 | `354029fc` | `4ae4bf23b4c2acff875775d4cbff5583893ce2a1` |
| `mame.sm1` | 131072 | `97cf998b` | `977387a7c76ef9b21d0b01fa69830e949a9a9626` |
| `mamelo.lo` | 65536 | `e09e253c` | `2b1c719531dac9bb503f22644e6e4236b91e7cfc` |

Other files in the zip (other BIOS versions such as UniBIOS) come along with it.

### NES / Famicom

Emulators: Mesen, FCEUmm. Required for Famicom Disk System games (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `disksys.rom` | 8 KB | `57fe1bdee955bb48d357e463ccbf129496930b62` | `ca30b50f880eb660a320674ed365ef7a` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/disksys.rom` |

### Nintendo 3DS

Emulators: Azahar. Optional (not listed in setup). From your own 3DS. Not needed for decrypted games: Azahar has its own fonts and system data. The real shared font fixes Chinese, Korean and Taiwanese text.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `boot9.bin` ARM9 boot ROM | 64 KB | matched by name and size |  | no | Must keep this exact name | Azahar `sysdata/boot9.bin` (only if none there) |
| `sector0x96.bin` | 512 bytes | matched by name and size |  | no | Must keep this exact name | Azahar `sysdata/sector0x96.bin` (only if none there) |
| `otp.bin` | 256 bytes | matched by name and size |  | no | Must keep this exact name | Azahar `sysdata/otp.bin` (only if none there) |
| `seeddb.bin` | any size | matched by name |  | no | Must keep this exact name | Azahar `sysdata/seeddb.bin` (only if none there) |
| `shared_font.bin` | any size | matched by name |  | no | Must keep this exact name | Azahar `sysdata/shared_font.bin` (only if none there) |
| `keys.txt` | any size | matched by name |  | no | Must keep this exact name | Azahar `sysdata/keys.txt` (only if none there) |
| `movable.sed` | 288 bytes to 320 bytes | matched by name and size |  | no | Must keep this exact name | Azahar `nand/private/movable.sed` (only if none there) |
| `SecureInfo_A` | 273 bytes | matched by name and size |  | no | Must keep this exact name | Azahar `nand/rw/sys/SecureInfo_A` (only if none there) |
| `LocalFriendCodeSeed_B` | 272 bytes | matched by name and size |  | no | Must keep this exact name | Azahar `nand/rw/sys/LocalFriendCodeSeed_B` (only if none there) |

### Nintendo 64

Emulators: M64Plus FZ, Mupen64Plus. Required for 64DD disks (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `64DD_IPL.bin` 64DD IPL (Japan) | 4 MB | `bf861922dcb78c316360e3e742f4f70ff63c9bc3` | `8d3d9f294b6e174bc7b1d2fd1c727530` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/Mupen64plus/IPL.n64`; M64Plus FZ `files/64DD_IPL.bin`; M64Plus FZ's 64DD IPL setting points at it if unset |

### Nintendo DS

Emulators: DraStic, built-in DS player. Optional (not listed in setup). Optional (not listed in setup). Firmware of your own DS.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `bios7.bin` ARM7 BIOS | 16 KB | `24f67bdea115a2c847c8813a262502ee1607b7df` | `df692a80a5b1bc90728bc3dfc76cd948` | no | Put it anywhere in the folder you pick (any name) | DraStic `system/nds_bios_arm7.bin`; built-in DS player `system/nds_bios_arm7.bin` |
| `bios9.bin` ARM9 BIOS | 4 KB | `bfaac75f101c135e32e2aaf541de6b1be4c8c62d` | `a392174eb3e572fed6447e956bde4b25` | no | Put it anywhere in the folder you pick (any name) | DraStic `system/nds_bios_arm9.bin`; built-in DS player `system/nds_bios_arm9.bin` |
| `firmware.bin` DS firmware | 128 KB | matched by name and size |  | no | Must keep this exact name | DraStic `system/nds_firmware.bin` (only if none there) |
| `firmware.bin` DS Lite firmware | 256 KB | matched by name and size |  | no | Must keep this exact name | DraStic `system/nds_firmware.bin` (only if none there) |
| `firmware.bin` DS firmware (iQue) | 512 KB | matched by name and size |  | no | Must keep this exact name | DraStic `system/nds_firmware.bin` (only if none there) |

### PGM

Emulators: FBNeo, MAME, MAME 2003-Plus. Required for PGM games. Recognised by its contents.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `pgm.zip` PGM BIOS set | zip | recognised by its contents |  | yes | Put it anywhere in the folder you pick (any name; zips are opened) | RetroArch `system/fbneo/pgm.zip`; RetroArch `system/mame/bios/pgm.zip` |

`pgm.zip` is recognised when it holds `pgm_t01s.rom`, `pgm_m01s.rom`, `pgm_p01s.u20` or `pgm_p02s.u20` (checked by name, size and CRC32, then SHA-1). Known members:

| Member | Size | CRC32 | SHA-1 |
|---|---|---|---|
| `pgm_t01s.rom` | 2097152 | `1a7123a0` | `cc567f577bfbf45427b54d6695b11b74f2578af3` |
| `pgm_m01s.rom` | 2097152 | `45ae7159` | `d3ed3ff3464557fd0df6b069b2e431528b0ebfa8` |
| `pgm_p01s.u20` | 131072 | `e42b166e` | `2a9df9ec746b14b74fae48b1a438da14973702ea` |
| `pgm_p02s.u20` | 131072 | `78c15fa2` | `885a6558e022602cc6f482ac9667ba9f61e75092` |

Other files in the zip (other BIOS versions such as UniBIOS) come along with it.

### PlayStation

Emulators: Beetle PSX HW, SwanStation, PCSX ReARMed. Optional. Much better compatibility; recommended. Optional (not listed in setup). The PS1 BIOS of a PS3.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `scph5501.bin` USA | 512 KB | `0555c6fae8906f3f09baf5988f00e55f88e9f30b` | `490f666e1afb15b7362b406ed1cea246` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph5501.bin` |
| `scph5502.bin` Europe | 512 KB | `f6bc2d1f5eb6593de7d089c425ac681d6fffd3f0` | `32736f17079d0b2b7024407c39bd3050` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph5502.bin` |
| `scph5500.bin` Japan | 512 KB | `b05def971d8ec59f346f2d9ac21fb742e3eb6917` | `8dd7d5296a650fac7319bce665a6a53c` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph5500.bin` |
| `scph101.bin` USA v4.5 | 512 KB | `dcffe16bd90a723499ad46c641424981338d8378` | `6e3735ff4c7dc899ee98981385f6f3d0` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph101.bin` |
| `scph7001.bin` USA v4.1 | 512 KB | `14df4f6c1e367ce097c11deae21566b4fe5647a9` | `1e68c231d0896b7eadcad1d7d8e76129` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph7001.bin` |
| `scph1001.bin` USA v2.0 | 512 KB | `10155d8d6e6e832d6ea66db9bc098321fb5e8ebf` | `924e392ed05558ffdb115408c263dccf` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/scph1001.bin` |
| `psxonpsp660.bin` Region-free, from a PSP | 512 KB | `96880d1ca92a016ff054be5159bb06fe03cb4e14` | `c53ca5908936d412331790f4426c6c33` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/psxonpsp660.bin` |
| `ps1_rom.bin` From a PS3 | any size | `c40146361eb8cf670b19fdc9759190257803cab7` | `81bbe60ba7a3d1cea1d48c14cbcc647b` | no | Must keep this exact name | RetroArch `system/ps1_rom.bin` |

### PlayStation 2

Emulators: ARMSX2. Required. Any other PS2 BIOS that ARMSX2 accepts is installed too. Optional (not listed in setup). Files dumped together with a PS2 BIOS.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `ps2-0230a-20080220.bin` USA v2.30 | 4 MB | `f9229fe159d0353b9f0632f3fdc66819c9030458` | `21038400dc633070a78ad53090c53017` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0230a-20080220.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0230e-20080220.bin` Europe v2.30 | 4 MB | `9915b5ba56798f4027ac1bd8d10abe0c1c9c326a` | `dc69f0643a3030aaa4797501b483d6c4` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0230e-20080220.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0230j-20080220.bin` Japan v2.30 | 4 MB | `fbd54bfc020af34008b317dcb80b812dd29b3759` | `80ac46fa7e77b8ab4366e86948e54f83` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0230j-20080220.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0220a-20060210.bin` USA v2.20 | 4 MB | `92e488d5b2705e4cca83d4d1efbc421012faf83e` | `cb801b7920a7d536ba07b6534d2433ca` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0220a-20060210.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0200a-20040614.bin` USA v2.00 | 4 MB | `7a62e5f48603582707e9898eb055ea3eaee50d4c` | `d333558cc14561c1fdc334c75d5f37b7` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0200a-20040614.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0200e-20040614.bin` Europe v2.00 | 4 MB | `434bc0b4eb4827da0773ec0795aadc5162569a07` | `dc752f160044f2ed5fc1f4964db2a095` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0200e-20040614.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0160a-20020207.bin` USA v1.60 | 4 MB | `f9a5d629a036b99128f7cb530c6e3ca016e9c8b7` | `d5ce2c7d119f563ce04bc04dbc3a323e` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0160a-20020207.bin`; ARMSX2's BIOS setting is set to it if it has none |
| `ps2-0100j-20000117.bin` Japan v1.00 | 4 MB | `aea061e6e263fdcc1c4fdbd68553ef78dae74263` | `acf4730ceb38ac9d8c7d8e21f2614600` | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/ps2-0100j-20000117.bin`; ARMSX2's BIOS setting is set to it if it has none |
| Any other PS2 BIOS | 4 MB | checked by ARMSX2 |  | yes | Put it anywhere in the folder you pick (any name) | ARMSX2 `bios/<its name>`; ARMSX2's BIOS setting is set to it if it has none |
| `<BIOS name>.nvm .mec .rom1 .rom2 .erom` Beside the BIOS, same name | any size | matched by name |  | no | Next to its PS2 BIOS, same base name | ARMSX2 `bios/<BIOS name>.<ext>` (only if none there) |

### PS Vita

Emulators: Vita3K. Required. Free download from Sony, any version 3.60 or later.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `PSVUPDAT.PUP` System software | any size | recognised by its contents |  | yes | The PSVUPDAT.PUP file; Vita3K installs it, which takes about a minute | installed by Vita3K |
| `PSP2UPDAT.PUP` Font package | any size | recognised by its contents |  | yes | The PSP2UPDAT.PUP file; Vita3K installs it, which takes about a minute | installed by Vita3K |

### PSP

Emulators: PPSSPP. Optional (not listed in setup). Fonts from a PSP's flash0 (PPSSPP has its own).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `jpn0.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/jpn0.pgf` (only if none there) |
| `kr0.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/kr0.pgf` (only if none there) |
| `ltn0.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn0.pgf` (only if none there) |
| `ltn1.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn1.pgf` (only if none there) |
| `ltn2.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn2.pgf` (only if none there) |
| `ltn3.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn3.pgf` (only if none there) |
| `ltn4.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn4.pgf` (only if none there) |
| `ltn5.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn5.pgf` (only if none there) |
| `ltn6.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn6.pgf` (only if none there) |
| `ltn7.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn7.pgf` (only if none there) |
| `ltn8.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn8.pgf` (only if none there) |
| `ltn9.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn9.pgf` (only if none there) |
| `ltn10.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn10.pgf` (only if none there) |
| `ltn11.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn11.pgf` (only if none there) |
| `ltn12.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn12.pgf` (only if none there) |
| `ltn13.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn13.pgf` (only if none there) |
| `ltn14.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn14.pgf` (only if none there) |
| `ltn15.pgf` | any size | matched by name |  | no | Must keep this exact name | PPSSPP `PSP/flash0/font/ltn15.pgf` (only if none there) |

### Saturn

Emulators: YabaSanshiro, Beetle Saturn. Required. Beetle Saturn needs the BIOS of the game's region.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `mpr-17933.bin` USA / Europe | 512 KB | `faa8ea183a6d7bbe5d4e03bb1332519800d3fbc3` | `3240872c70984b6cbfda1586cab68dbe` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/mpr-17933.bin` |
| `sega_101.bin` Japan | 512 KB | `df94c5b4d47eb3cc404d88b33a8fda237eaf4720` | `85ec9ca47d8f6807718151cbcca8b964` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/sega_101.bin` |
| `saturn_bios.bin` Japan v1.00 | 512 KB | `2b8cb4f87580683eb4d760e4ed210813d667f0a2` | `af5828fdff51384f99b3c4926be27762` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/saturn_bios.bin` (only for Japan, or when no other Saturn BIOS is there) |

### ScummVM

Emulators: ScummVM. Optional (not listed in setup). Roland MT-32 / CM-32L sound.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `MT32_CONTROL.ROM` MT-32 | 64 KB | matched by name and size |  | no | Must keep this exact name | ScummVM extra path `MT32_CONTROL.ROM` (only if none there) |
| `MT32_PCM.ROM` MT-32 | 512 KB | matched by name and size |  | no | Must keep this exact name | ScummVM extra path `MT32_PCM.ROM` (only if none there) |
| `CM32L_CONTROL.ROM` CM-32L | 64 KB | matched by name and size |  | no | Must keep this exact name | ScummVM extra path `CM32L_CONTROL.ROM` (only if none there) |
| `CM32L_PCM.ROM` CM-32L | 1 MB | matched by name and size |  | no | Must keep this exact name | ScummVM extra path `CM32L_PCM.ROM` (only if none there) |

### Sega CD

Emulators: Genesis Plus GX, PicoDrive. Required for Sega CD games. Games need the BIOS of their region.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `bios_CD_U.bin` USA | 128 KB | `f4f315adcef9b8feb0364c21ab7f0eaf5457f3ed` | `2efd74e3232ff260e371b99f84024f7f` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_CD_U.bin` |
| `bios_CD_E.bin` Europe | 128 KB | `f891e0ea651e2232af0c5c4cb46a0cae2ee8f356` | `e66fa1dc5820d254611fdcdba0662372` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_CD_E.bin` |
| `bios_CD_J.bin` Japan | 128 KB | `4846f448160059a7da0215a5df12ca160f26dd69` | `278a9397d192149e84e820ac621a8edd` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/bios_CD_J.bin` |

### Sega ST-V

Emulators: Beetle Saturn, MAME, MAME 2003-Plus. Required for ST-V games (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `epr-20091.ic8` Japan | 512 KB | `eff0f54c70bce05ff3a289bf30b1027e1c8cd117` |  | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/epr-20091.ic8` |
| `epr-17952a.ic8` USA | 512 KB | `eaf1c3e5d602e1139d2090a78d7e19f04f916794` |  | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/epr-17952a.ic8` |
| `epr-17954a.ic8` Europe | 512 KB | `af79cff317e5b57d49e463af16a9f616ed1eee08` |  | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/epr-17954a.ic8` |
| `stvbios.zip` ST-V BIOS set | zip | recognised by its contents |  | no | Put it anywhere in the folder you pick (any name; zips are opened) | RetroArch `system/mame/bios/stvbios.zip` |

`stvbios.zip` is recognised when it holds `epr-20091.ic8` or `epr-17952a.ic8` or `epr-17954a.ic8` or `epr-19730.ic8` (checked by name, size and CRC32, then SHA-1). Known members:

| Member | Size | CRC32 | SHA-1 |
|---|---|---|---|
| `epr-20091.ic8` | 524288 | `59ed40f4` | `eff0f54c70bce05ff3a289bf30b1027e1c8cd117` |
| `epr-17952a.ic8` | 524288 | `d1be2adf` | `eaf1c3e5d602e1139d2090a78d7e19f04f916794` |
| `epr-17954a.ic8` | 524288 | `f7722da3` | `af79cff317e5b57d49e463af16a9f616ed1eee08` |
| `epr-19730.ic8` | 524288 | `d0e0889d` | `fae53107c894e0c41c49e191dbe706c9cd6e50bd` |

Other files in the zip (other BIOS versions such as UniBIOS) come along with it.

### Super Nintendo

Emulators: bsnes, Snes9x. Required for Satellaview, Sufami Turbo, Super Game Boy and special-chip games (not listed in setup).

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `BS-X.bin` Satellaview | 1 MB | `604556b2e62860af18db5a77f2956ebc75450020` | `fed4d8242cfbed61343d53d48432aced` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/BS-X.bin` |
| `STBIOS.bin` Sufami Turbo | 256 KB | `ef86ea192eed03d5c413fdbbfd46043be1d7a127` | `d3a44ba7d42a74d3ac58cb9c14c6a5ca` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/STBIOS.bin` |
| `SGB1.sfc` Super Game Boy | 256 KB | `973e10840db683cf3faf61bd443090786b3a9f04` | `b15ddb15721c657d82c5bab6db982ee9` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/SGB1.sfc` |
| `SGB2.sfc` Super Game Boy 2 | 512 KB | `e5b2922ca137051059e4269b236d07a22c07bc84` | `8ecd73eb4edf7ed7e81aef1be80031d5` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/SGB2.sfc` |
| `cx4.data.rom` Cx4 | 3 KB | `a002f4efba42775a31185d443f3ed1790b0e949a` | `037ac4296b6b6a5c47c440188d3c72e3` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/cx4.data.rom` |
| `dsp1.data.rom` DSP-1 | 2 KB | `589c158685fe3cf88c7ca77cb6083f0e4de9c7c3` | `3d81b45fa0c2aa8b852dfb1ece7c0971` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp1.data.rom` |
| `dsp1.program.rom` DSP-1 | 6 KB | `c5d283604beebb3d832f15a0b3fc60d10dd4bddc` | `ae209fbe789fbf11a48aea5ab1197321` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp1.program.rom` |
| `dsp1b.data.rom` DSP-1B | 2 KB | `19d5e822fe15ee6942e9921c3d8275a761a969df` | `1e3f568634a7d8284020dddc0ae905bc` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp1b.data.rom` |
| `dsp1b.program.rom` DSP-1B | 6 KB | `03c6c0bdb40846c42aeff6b716e1ea0e001c3ae3` | `d10f446888e097cbf500f3f663cf4f6d` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp1b.program.rom` |
| `dsp2.data.rom` DSP-2 | 2 KB | `aed9167cc2897cd8d0432f0b8f713fabbc48f5c4` | `e9417e29223b139c3c4b635a2a3b8744` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp2.data.rom` |
| `dsp2.program.rom` DSP-2 | 6 KB | `51a274955ef493cfcf2efad7e2ee54738c81eb15` | `aa6e5922a3ed5ded54f24247c11143c5` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp2.program.rom` |
| `dsp3.data.rom` DSP-3 | 2 KB | `e7ec3da203a4edba2b17c06d9384787304a025a0` | `0a81210c0a940b997dd9843281008ee6` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp3.data.rom` |
| `dsp3.program.rom` DSP-3 | 6 KB | `11b7ba3501a8542af4cdfdbd1c9702ae0a0e3fae` | `d99ca4562818d49cee1f242705bba6f8` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp3.program.rom` |
| `dsp4.data.rom` DSP-4 | 2 KB | `8f2ffb9c1702aa2f76191e98f3fef43f73e5aab7` | `ee4990879eb68e3cbca239c5bc20303d` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp4.data.rom` |
| `dsp4.program.rom` DSP-4 | 6 KB | `1abee528a59238fd5668fdcffd598d079cb056c2` | `a151023b948b90ffc23a5b594bb6fef2` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/dsp4.program.rom` |
| `st010.data.rom` ST010 | 4 KB | `bd116501ec41c438fff210fd03233c6c9d15a514` | `254d70762b6f59f99c27c395aba7d07d` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st010.data.rom` |
| `st010.program.rom` ST010 | 48 KB | `2d0f34074ed3c6dfd61f767ee8b18e699b9769ac` | `1d70019179a59a566a0bb5d3f2845544` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st010.program.rom` |
| `st011.data.rom` ST011 | 4 KB | `798ab84110ca93a6e73a540f4146d21f8d9e1442` | `10bd3f4aa949737ab9836512c35bcc29` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st011.data.rom` |
| `st011.program.rom` ST011 | 48 KB | `375eca92592621e83881f3a9d7fd5ea3e5b306dc` | `95222ebf1c0c2990bcf25db43743f032` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st011.program.rom` |
| `st018.data.rom` ST018 | 32 KB | `b19c0f8f207d62fdabf4bf71442826063bccc626` | `49c898b60d0f15e90d0ba780dd12f366` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st018.data.rom` |
| `st018.program.rom` ST018 | 128 KB | `388e3721b94cd074d6ba0eca8616523d2118a6c3` | `dda40ccd57390c96e49d30a041f9a9e7` | no | Put it anywhere in the folder you pick (any name) | RetroArch `system/st018.program.rom` |

### TurboGrafx-16 / PC Engine

Emulators: Beetle PCE Fast. Required for CD games.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `syscard3.pce` Super CD-ROM2 System 3.0 | 256 KB | `79f5ff55dd10187c7fd7b8daab0b3ffbd1f56a2c` | `38179df8f4ac870017db21ebcbf53114` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/syscard3.pce` |
| `syscard2.pce` CD-ROM2 System 2.x | 256 KB | `88da02e2503f7c32810f5d93a34849d470742b6d` | `3cdd6614a918616bfc41c862e889dd79` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/syscard2.pce` |
| `syscard1.pce` CD-ROM2 System 1.x | 256 KB | `a39a66da7de6ba94ab84d04eef7afeec7d4ee66a` | `2b7ccb3d86baa18f6402c176f3065082` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/syscard1.pce` |
| `gexpress.pce` Game Express CD Card | 32 KB | `014881a959e045e00f4db8f52955200865d40280` | `6d2cb14fc3e1f65ceb135633d1694122` | yes | Put it anywhere in the folder you pick (any name) | RetroArch `system/gexpress.pce` |

### Xbox

Emulators: hakuX. Optional (not listed in setup). Files of your own Xbox.

| File | Size | SHA-1 / check | MD5 | Listed | How | Installed to |
|---|---|---|---|---|---|---|
| `mcpx.bin` MCPX boot ROM | 512 bytes | matched by name and size |  | no | Must keep this exact name | hakuX `console/mcpx.bin` (only if none there) |
| `bios.bin` BIOS (256 KB flash) | 256 KB | matched by name and size |  | no | Must keep this exact name | hakuX `console/bios.bin` (only if none there) |
| `bios.bin` BIOS (1 MB flash) | 1 MB | matched by name and size |  | no | Must keep this exact name | hakuX `console/bios.bin` (only if none there) |
| `eeprom.bin` EEPROM | 256 bytes | matched by name and size |  | no | Must keep this exact name | hakuX `console/eeprom.bin` (only if none there) |

## Where each emulator's files go

| Emulator | Before Move Emulator Files to microSD | After |
|---|---|---|
| RetroArch | RetroArch's system folder (system_directory in retroarch.cfg; by default Internal storage/RetroArch/system) | microSD card/emulator_config/retroarch/system |
| ARMSX2 | Internal storage/Android/data/com.armsx2/files/bios | unchanged (the BIOS stays on internal storage) |
| Dolphin | Internal storage/Android/data/org.dolphinemu.dolphinemu/files/GC | microSD card/emulator_config/dolphin/GC (the GC folder becomes a link to it) |
| Flycast | Flycast's home folder/data (by default Internal storage/Android/data/com.flycast.emulator/files/data) | microSD card/emulator_config/flycast/data |
| DraStic | DraStic's system folder (by default its app folder, files/DraStic/system) | microSD card/emulator_config/drastic/system |
| built-in DS player | /data/system/drastic-nano/DraStic/system (Thimsy's built-in DS player) | unchanged |
| M64Plus FZ | Internal storage/Android/data/org.mupen64plusae.v3.fzurita/files (and its 64DD IPL setting) | unchanged |
| PPSSPP | PPSSPP's memory stick/PSP/flash0/font | microSD card/emulator_config/ppsspp/PSP/flash0/font |
| ScummVM | ScummVM's extra path (if unset: its app folder, files/extra, set as the extra path) | unchanged |
| Azahar | Azahar's folder (only when one has been chosen in Azahar) | microSD card/emulator_config/azahar |
| hakuX | hakuX's app folder, files/console | unchanged |
| Vita3K | installed by Vita3K itself into its memory card folder (pref-path) | microSD card/emulator_config/vita3k (moves with Vita3K's memory card) |

Arcade BIOS zips go to RetroArch's `system/fbneo/` (FBNeo) and `system/mame/bios/` (MAME). MAME 2003-Plus only reads BIOS zips from the folder of the game itself, so copy `neogeo.zip` / `pgm.zip` / `stvbios.zip` next to those games for it.
