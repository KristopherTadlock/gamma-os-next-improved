# GammaOS changes after 1.4.4: what to bring to the RG Cube

Source: `TheGammaSqueeze/GammaOSNextDistribution-14`, branch `develop`. The
1.4.4 build (`lineage-21.0-20261002`) ends at commit `0bc531e2d`
(2 October 2026). Since then there are 408 commits up to 26 October 2026, and
on 23 October the tree was bumped to **Version 1.5.0**. 1.5.0 is in a private
beta for the Retroid Pocket Duo Lite and isn't released yet.

What the RG Cube is, for judging each change: one 720x720 screen, Unisoc
T820, Mali-G57 GPU, 8 GB RAM. GammaOS's home screen draws through
SurfaceFlinger on Unisoc rather than taking the panel directly. GammaEQ runs
the Cube's speaker profile, and the Cube can dock to HDMI.

| Area | Commits | Verdict |
|---|---:|---|
| azahar-nano, a built-in 3DS emulator | 213 | Optional, experimental |
| 3DS HOME Menu home theme | ~75 | Optional (see the note on its look) |
| Home screen (Nano) fixes and speed-ups | ~25 | **Yes** |
| System, power, audio and input fixes | ~15 | **Yes** |
| DraStic-nano (DS player) | ~25 | Mostly no (dual-screen work); a few yes |
| Dual-screen devices (RP Duo Lite, RG DS) | ~45 | No |
| Bundled Pokémon ORAS demo | 1 | **No** |

## Yes: general fixes and improvements

**Home screen (GammaOS Nano)**
- `nano: 60 fps on the SurfaceFlinger path (pacing and display power off the render thread)`. The Cube draws through SurfaceFlinger, so this is the most relevant change of all.
- `nano: do not block the frame on SurfaceFlinger in the second display check`. Avoids stalls when HDMI is connected.
- `nano: large game libraries stay light on art memory and work`.
- `nano: game lists can show their subfolders as folders`.
- `nano: overlay raise decodes the menu icons on worker threads`. The power-hold menu opens faster.
- `nano: glass bevel blur as a running sum`. Cheaper XMB icon effects.
- `nano: DS cartridge titles come before scraped names`.
- `nano: optional CPU affinity mask for the home process` (off unless set).
- `gammaos-nano: MTP screen no longer resets the USB gadget in a loop`.
- `gammaos-nano: home no longer blank after leaving a DS game`.
- `gammaos-nano: Applications leads the Games list, Recently Played follows, in every sort mode`.
- `nano: User Guide covers every home theme`.

**Sleep and power**
- `nano, azahar-nano: let the device suspend with the screen off`.
- `Ultra low power sleep parks audio and sensor sessions without ending them`.
- `drastic-nano: stop the exclusive audio stream for the device sleep`.

**System**
- `system_server: network time sync in nano mode` (two commits). The clock stays correct without booting full Android.
- `SensorManager: apps in nano mode see the real sensor list`.
- `audiopolicy: retry outputs that failed to open at module load`.
- `libaudiohal: only audioserver exits when the audio HAL dies`. An audio crash no longer takes more down with it.
- `GammaEQ: per-thread filter state and the deep-buffer sample formats`. GammaEQ drives the Cube's speaker profile.
- `Stop the WebView child zygote with the lazy 32-bit zygote`. Frees memory.
- `PhoneWindowManager: foreground app for the key overrides is the focused window's`.
- `inputflinger: a built-in game controller's keys are not tied to display 0`. Relevant when docked to HDMI.
- `gammapad: let the old virtual pad node go before creating the next one`.
- `Emulator data folders move with the setting`.
- `Display colour: LiveDisplay and per-screen calibration on every surface`. The LiveDisplay part applies to the Cube; the per-screen calibration is RG DS hardware.

**DraStic-nano, the parts that apply to one screen**
- `drastic-nano: microphone noise gate`.
- `drastic-nano: optional CPU affinity mask for the whole process` (off unless set).
- `drastic-nano: keep every core of each core_ctl cluster online for the session`. Check it's harmless on Unisoc's CPU scheduler first.

## Optional

**azahar-nano: a 3DS emulator built into GammaOS Nano** (213 commits, from
8 October). It's a native build of Azahar inside the system image, with the
same in-game menu as DraStic-nano. Several commits target the Cube's Mali GPU:
- `PICA and renderer trims for the Mali devices`
- `extended dynamic state on the Arm driver`
- `smaller JIT dispatch table on Mali`

It's three weeks old and still changing daily: many commits are profiling
switches, and the core is a prebuilt that's refreshed often. The Azahar app is
already installed. Suggestion: include it as a second choice and keep the
Azahar app as the default, until it's tested on the Cube.

**3DS HOME Menu theme** (a fifth home theme, from 14 October). It recreates
the 3DS home menu: folders, banners, colour themes, custom themes from the SD
card, its own setup wizard and dialogs. It's built for two screens, but
"stacks on single panels", so it can work on the Cube's one square screen.
That layout needs checking on the device.

A note on its look: like GammaOS's PS3 XMB theme, it reproduces a console
maker's interface, including the Rodin typeface, a commercial font. GammaOS
already ships the XMB theme in 1.4, so this adds no new kind of risk. If the
project wants to stay strictly clear of console makers' trade dress, though,
these themes are the place to draw the line.

## No

- **Dual-screen and other device-specific work:**
  - the RP Duo Lite panels and pacing;
  - the second panel's backlight and brightness;
  - two touch digitizers;
  - the lid switch;
  - the Control Center on a bottom panel;
  - Qualcomm gauges and audio;
  - Turnip for the Adreno 610;
  - the trinket display HAL.

  The Cube has none of this hardware. These changes are harmless if they come
  along with the rest, but they need no work.
- **`Bundled ROMs: Pokemon Omega Ruby and Alpha Sapphire Special Demo under n3ds`.**
  A commercial demo, which this project's pack-in rules exclude. If we build
  from `develop`, drop it from the ROM pack.
- **`GammaOS Toolbox asks for a disclaimer on every entry`.** Neutral; it comes
  along with the Toolbox.

## How to bring them in

The changes are tangled: most of the yes list lives in the same
`gammaos-nano` sources as the 3DS theme and azahar-nano work. Picking them one
at a time onto 1.4.4 would mean resolving conflicts commit by commit.
Suggested order:

1. Build 1.4.4 (`0bc531e2d`) first, and get it booting on the Cube. That
   proves the port.
2. Build `develop` at a pinned commit, leaving out the ORAS demo, and compare
   the two on the device. The yes list and both optional features come along.
   Anything that misbehaves on the Cube can be reverted on top.
3. Keep the result private: 1.4.4 until GammaOS makes it public, and anything
   from 1.5.0 until GammaOS releases 1.5.0.
