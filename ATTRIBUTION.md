# Thimsy OS: attribution for the art, fonts and sounds it ships

This file credits every image, font and sound in the Thimsy OS 1.0a image, with
its work, author, source and license. [NOTICE](NOTICE) has the notices for the
code and the downloaded parts of the build. Paths starting `nano/` are in
`frameworks/base/cmds/gammaos-nano/` of the GammaOS source tree, and paths starting
`ps3xmb/` are in `nano/ps3xmb/`. They are installed in `/system/etc/nano_xmb/`.

Files still to be replaced are not listed here; they are not covered by any license.

## Thimsy OS's own art

| work | where | author | license |
|---|---|---|---|
| The Thimsy logo, an elf in a thimble helmet raising a sword (`nano/bootscreen/thimsy.svg`, `docs/thimsy.png`) | Nano's boot screen (`/system/etc/nano_xmb/boot/logo.svg`), the bootloader's boot picture (`scripts/bootlogo.py`) | the Thimsy OS project | [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/): free to use and redistribute, with credit to the Thimsy OS project; no commercial use |
| Android boot animation: the Thimsy logo fading in on Midnight blue (`mods/bootanimation/bootanimation.zip`, drawn by `tools/bootanimation/make_bootanimation.py`) | `/system/product/media/bootanimation.zip` (`mods/58-bootanimation.sh`) | the Thimsy OS project | as the logo |
| Controller drawings for the button map (`ps3xmb/controllers/svg/*.svg`, `controllers.json`, from `work/controller-art`) | Nano's button map (`/system/etc/nano_xmb/controllers/`, with its `LICENSE`) | the Thimsy OS project | CC BY-NC 4.0, credited as "Controller drawings by the Thimsy OS project, CC BY-NC 4.0" |
| Box art and title marquees of the pack-in games (`mods/pack-in-art`, drawn by `tools/pack-in-art/make_boxes.py` around each game's own pictures), and the screenshots and title screens captured from the games (`tools/pack-in-art/capture.py`) | `/sdcard/ES-DE` after setup | Thimsy OS, from each game's pictures | each game's license (below) |

The box art's lettering is drawn with Archivo Black, Russo One, Exo 2 and other
Google Fonts faces under the SIL Open Font License 1.1 (`make_boxes.py`, `STYLES`).
Only the pictures ship, not the fonts.

## Pack-in games' pictures

Each game's box art, screenshot, title screen and marquee is under that game's
license. They are either drawn or captured for Thimsy OS (above) or downloaded from
the game's own project. [PACK-IN-GAMES.txt](PACK-IN-GAMES.txt) and
`mods/pack-in-media.txt` give the source and license of each picture, and
`mods/rom-licenses` has each game's license file.

| system | game and authors | license | source |
|---|---|---|---|
| Game Boy | Big2Small, by Matthew D. Steele | GPL-3.0; art and music from CC0 and public-domain sources | https://github.com/mdsteele/big2small |
| Game Boy Color | Tobu Tobu Girl Deluxe, by Tangram Games (music by potato-tan) | code MIT; art, text, sound and music CC BY 4.0 | https://github.com/SimonLarsen/tobutobugirl-dx |
| Game Boy Advance | The Hat Chooses the Wizard, by Corwin Kuiper and Gwilym Inzani | MPL-2.0; music "Sylvan Waltz" by Otto Halmén, CC BY 3.0 | https://github.com/agbrs/agb |
| Nintendo DS | Blimp Chicken DS, by TJ M | CC0 1.0 | https://github.com/My-name-is-TJ/blimp-chicken-ds |
| GameCube | Vacpyre, by Martin Holtkamp (Solarscape Games) | MIT | https://github.com/mholtkamp/vacpyre |
| Wii | chro.mono, by Thomas Perl (Wii port by Alberto Mardegan) | GPL-2.0 or later; music CC0, sound effects CC BY 3.0 / CC0, font SIL OFL, all credited in the game | https://github.com/mardy/chromono |
| Nintendo 3DS | Anarch, by Miloslav Číž (drummyfish); 3DS port by blitzdoughnuts | CC0 plus a waiver of all rights | https://gitlab.com/drummyfish/anarch |
| ScummVM | Beneath a Steel Sky, by Revolution Software (backgrounds by Dave Gibbons) | freeware, free redistribution with its readme and notices; content (C) Revolution Software | https://www.scummvm.org/games/#games-sky |
| NES | Lala the Magical, by The Mojon Twins | CC BY-NC-SA 3.0 | https://www.mojontwins.com/juegos_mojonos/lala-the-magical/ |
| SNES | Super Sudoku, by Raphaël Assenat | MIT | https://github.com/raphnet/super_sudoku |
| Virtual Boy | VUEngine Platformer Demo, by Jorge Eremiev and Christian Radke | MIT | https://github.com/VUEngine/VUEngine-Platformer-Demo |
| Master System | Gravity Beam: Master Gaiden, by Mathew Carr | BSD-3-Clause | https://www.smspower.org/Homebrew/GravityBeamMasterGaiden-SMS |
| Game Gear | Waternet, by Willems Davy | MIT | https://github.com/joyrider3774/waternet |
| Genesis / Mega Drive | Miniplanets REMIX, by Javier Degirolmo (Sik) | zlib | https://github.com/sikthehedgehog/miniplanets |
| Sega 32X | Dungeons of Noudar 3D, 32X port by Haroldo O. Pinheiro of Daniel Monteiro's game | BSD-2-Clause | https://github.com/haroldo-ok/noudar-32x |
| Dreamcast | Demon Bazooka, by Rich Stokes | MIT | https://github.com/richstokes/dreamcast-homebrew |
| PC Engine / TurboGrafx-16 | Worm, by Andy McCall | CC BY-NC 4.0 | https://github.com/andymccall/worm |
| Neo Geo Pocket Color | Stargunner, by Tixul | MIT (Copyright (c) 2026 NGPC Template Contributors) | https://github.com/Tixul/Stargunner |
| PSP | KłełeAtoms, by Nightwolf-47 | MIT | https://github.com/Nightwolf-47/KleleAtoms-PSP |
| PICO-8 | Cherry Bomb, by Krystian Majewski (music by Sebastian Haßler) | CC BY-NC-SA 4.0 | https://www.lexaloffle.com/bbs/?pid=cherrybomb |
| Atari 2600 | 2048 2600, by Carlos Duarte do Nascimento (cartridge label by Chester) | MIT | https://github.com/chesterbr/2048-2600 |
| Atari 5200 | Curse of the Lost Miner, by BAHA Software (Michael Kalouš) | public domain | https://github.com/baktragh/culomin_sources |
| Atari 7800 | robotfindskitten, by Thomas Mathys | GPL-2.0 or later | https://robotfindskitten.org/download/Atari7800/ |

## GammaOS Nano

| work | files | author | source | license |
|---|---|---|---|---|
| RetroArch "Monochrome" system icons | `ps3xmb/icons_retroarch/*.png` (849), and the copies built into Nano (`nano/xmb_icons.h`) | the libretro project and the retroarch-assets contributors | https://github.com/libretro/retroarch-assets | CC BY 4.0 |
| Earth maps for the Time Zone and music globes (Blue Marble, Earth's City Lights; origin inferred from their content) | `ps3xmb/globe/earth_day.png`, `earth_night.png`, `earth_clouds.png` | NASA Goddard Space Flight Center (Blue Marble: Reto Stöckli; City Lights: data by Marc Imhoff (NASA GSFC) and Christopher Elvidge (NOAA NGDC), image by Craig Mayhew and Robert Simmon) | https://visibleearth.nasa.gov | public domain (NASA asks for credit) |
| Settings and status bar icons, rasterised from Android's vector icons | `ps3xmb/icons_ui/*.png`, `ps3xmb/nds/nds_sb_*.png`, `nano/NanoMenuSbIcons.h` | The Android Open Source Project | https://source.android.com | Apache-2.0 |
| ES-DE's help glyphs, controller, badge and system status icons | `nano_esde_assets/{help,controllers,badges}/*.svg`, `ps3xmb/systemstatus/*.svg` | ES-DE (Copyright Northwestern Software AB, Leon Styhre, Alec Lofquist); the status icons are based on Phosphor Icons | https://gitlab.com/es-de/emulationstation-de, https://phosphoricons.com | MIT |
| Font Awesome 4.7.0 (icon glyphs) | `ps3xmb/fonts/fontawesome-webfont.ttf` | Dave Gandy | https://github.com/FortAwesome/Font-Awesome/tree/v4.7.0 | SIL Open Font License 1.1 |
| Rounded M+ 1c Medium (the DSi theme's digits) | `ps3xmb/nds/dsvecnum.ttf` | Copyright 2016 The Rounded M+ Project Authors | http://jikasei.me/ | SIL Open Font License 1.1 |
| Barlow Semi Condensed Medium and SemiBold, in place of Akrobat (ES-DE's default face) | `/system/etc/nano_xmb/fonts/akrobat-regular.ttf`, `akrobat-semibold.ttf`, with `Barlow-OFL.txt` (`mods/50-nano.sh`) | Copyright 2017 The Barlow Project Authors (Jeremy Tribby) | https://github.com/jpt/barlow/tree/1.422 | SIL Open Font License 1.1 |
| Minima theme sounds | `ps3xmb/audio/minima_{cursor,ok,back,drill,error}.wav` | GammaOS (TheGammaSqueeze) | https://github.com/TheGammaSqueeze/GammaOSNext | Apache-2.0 |
| "Retro Arcade Game Music" (the GammaEQ preview loop) | `ps3xmb/audio/preview.wav`, and `preview_fast.wav` in GammaEQ | original_soundtrack (Viacheslav Starostin) | https://pixabay.com/music/upbeat-retro-arcade-game-music-408074/ | Pixabay Content License |
| "Computer Startup Sound Effect" (the ES-DE home's startup sound) | `/system/etc/nano_xmb/audio/esde_boot.mp3` (`mods/57-boot-sound.sh`) | Dragon Studio | https://pixabay.com/sound-effects/film-special-effects-computer-startup-sound-effect-312870/ | Pixabay Content License |
| Daijishou platform definitions (data, not art) | `ps3xmb/daijishou/*.json` | Copyright (c) 2022 TapiocaFox (Yves Chen) | https://github.com/TapiocaFox/Daijishou | MIT |

The Pixabay Content License allows use in a product such as this one without
credit; it does not allow selling or distributing the sound on its own.

## ES-DE themes

### Colorful (Simplified)

`/system/etc/nano_esde_themes/colorful-simplified-es-de/`, downloaded by
`mods/50-nano.sh`: [Colorful (Simplified)](https://github.com/anthonycaccese/colorful-simplified-es-de)
for ES-DE by ant (anthonycaccese), based on the Colorful theme for LaunchBox by
[viking](https://forums.launchbox-app.com/profile/70421-viking/) and
[faeran](https://forums.launchbox-app.com/profile/76940-faeran/). License:
Creative Commons Attribution-NonCommercial-ShareAlike 2.0
(https://creativecommons.org/licenses/by-nc-sa/2.0/). The theme's own README, with its
credits, is installed with it.

The theme's credits:

- Most artwork originally created by [viking](https://forums.launchbox-app.com/profile/70421-viking/)
  and [faeran](https://forums.launchbox-app.com/files/file/2081-colorful-bigbox-theme). That covers
  the system pictures (`_inc/systems/images/*.webp`), the theme's icons and images (`_inc/images`)
  and its sounds (`_inc/sounds`).
- Systems that did not exist in the Colorful set: some were sourced from the work of
  [g-spawn](https://github.com/g-spawnPL) for the [ckau-book](https://github.com/CkauNui/ckau-book/tree/master)
  theme, edited to add transparency.
- OpenBOR sprites (`openbor.webp`) originally created by [DOA687](https://www.deviantart.com/doa687).
- M.U.G.E.N. sprites (`mugen.webp`) originally created by
  [mpadillathespriter](https://www.deviantart.com/mpadillathespriter).
- EasyRPG artwork (`easyrpg.webp`) originally created by [SeraphCircle](https://twitter.com/SeraphCircle).
- Doom artwork (`doom.webp`) originally created by [azakachi-rd-17](https://www.deviantart.com/azakachi-rd-17).
- LowresNX artwork (`lowresnx.webp`) by
  [Duimon](https://forums.libretro.com/t/duimon-hsm-mega-bezel-graphics-and-presets-feedback-and-updates/28146/1049).
- The base folder image of the All Games, Favorites, Last Played and Custom Collections pictures
  (`auto-allgames.webp`, `auto-favorites.webp`, `auto-lastplayed.webp`, `custom-collections.webp`)
  was sourced from [Rick Patrick](https://www.softicons.com/designers/rick-patrick).
- The icons on those folder pictures were sourced from [FontAwesome](https://fontawesome.com/search?o=r&m=free)
  (Font Awesome Free icons, CC BY 4.0).

The other artists' pictures carry no license of their own; they ship as part of the
theme's release.

The system pictures show consoles, their screens and their makers' marks, which belong
to their respective owners (see Third-party marks below).

Changes in Thimsy OS's copy (also noted at the end of its README):

- The Gilroy font (Radomir Tinkov, commercial) is replaced by
  [Urbanist](https://github.com/coreyhu/Urbanist/releases/tag/1.330) 1.330 (Light, Regular,
  SemiBold, Bold), Copyright 2024 The Urbanist Project Authors, SIL Open Font License 1.1
  (`_inc/fonts/Urbanist-OFL.txt`).
- The help bar's Akrobat Bold (Fontfabric) is replaced by
  [Barlow](https://github.com/jpt/barlow/tree/1.422) Semi Condensed Bold 1.422, Copyright 2017 The
  Barlow Project Authors, SIL Open Font License 1.1 (`_inc/fonts/Barlow-OFL.txt`).
- `quake.webp`, a Quake Champions render credited to noorkabir (imgbin.com), is removed: its
  rights are id Software's. A replacement is needed; until then the Quake system shows the
  theme's default picture (`_default.webp`). Quake is not one of Thimsy's systems.

### Slate

`/system/etc/nano_esde_themes/slate-es-de.zip` (in GammaOS Nano): Slate, ES-DE's default theme,
by the ES-DE project, based on [recalbox-multi](https://gitlab.com/recalbox/recalbox-themes) by
the Recalbox community, with graphics from the [Carbon](https://github.com/RetroPie/es-theme-carbon)
theme by Rookervik, console and controller vector graphics by Bezza191 and logotype graphics by
Dan Patrick. License: Creative Commons Attribution-NonCommercial-ShareAlike 2.0 (its `LICENSE` and
`CREDITS.md` are in the zip). Its console and brand logos are excluded from that license: "The
used logos and trademarks are copyright of their respective owners."

Slate's fonts are [Exo 2](https://github.com/googlefonts/Exo-2.0) (Regular, SemiBold and Bold
Condensed) by Natanael Gama, Copyright 2013 The Exo 2 Project Authors, SIL Open Font License 1.1.

## Android and the GammaOS base

| work | files | author | license |
|---|---|---|---|
| System fonts (Roboto, Noto, Lato, Rubik and others); Roboto also draws the boot screen's text | `/system/fonts/*`, `/system/product/fonts/*` | Google, The Android Open Source Project, LineageOS, and each font's authors | Apache-2.0 or SIL Open Font License 1.1 |
| Ringtones, alarms, notifications and UI sounds | `/system/product/media/audio/**` | The Android Open Source Project, LineageOS | Apache-2.0 |
| Charger images | `/system/etc/res/images/charger/battery_{fail,scale}.png` | The Android Open Source Project | Apache-2.0 |

## Third-party marks

These ship as parts of the themes above and belong to their owners. They are used to
name the systems they stand for:

- the console and brand logos in Slate (`*/images/logo.svg`);
- the console product photos, screens and marks in Colorful's system pictures (for example
  `ps2.webp` shows a Sony TV, a PlayStation 2 and its boot screen);
- the PlayStation, Xbox and Nintendo face-button glyphs in ES-DE's help icons.
