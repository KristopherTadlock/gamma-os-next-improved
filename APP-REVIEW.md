# Apps shipped in Thimsy OS (1.4.4 base): review

**Decision (2026-11-12):** keep an app if it makes sense on at least one GammaOS-supported device, per GammaOS Next's README: RG Cube, RG Vita/Vita Pro, RG477M/V, RG557/556, RG406H/V, RG476H, RG Slide, RG405M/V, RG505, RG Rotate, AYANEO Pocket Air Mini and Pocket Micro, Retroid Pocket 4 Pro and Classic, KT-R1, ZPG Unicorn A1, GameMT E6 MAX, Mangmi Air X. For the dual-screen RG DS, see the GammaOS Nano list. None has a camera, a modem or NFC.
- **Removed in test 22:** Updater (Thimsy will get its own; Nano's online GammaOS update check is off too), Aperture, ApertureLensLauncher, CameraExtensionsProxy, Stk, messaging, EmergencyInfo, NfcNci, Tag, SecureElement, QuickAccessWallet, Gallery2, BasicDreams, PhotoTable, EasterEgg, BuiltInPrintService, Jelly, GammaBrowser (Nano now opens Android's default browser, Brave), GammaToast, GammaDisplayLoading.
- **Kept:**
  - DualStackControl and SecondaryHome (dual-screen devices);
  - Glimpse (the one gallery for full-Android mode; Nano has its own viewer);
  - Brave (default browser, installed by setup); Fulguris instead on devices with 1.3 GB of RAM or less;
  - Eleven, MusicFX, Etar, Contacts, LocalContactsBackup, Recorder, Profiles.

Built image `out/target/product/tdgsi_arm64_ab` (system, product, system_ext) plus the apps setup installs (`gammaos/apps.list`, `mods/added-apps.txt`, Magisk via `customization.sh`), as of 2026-11-12. The RG Cube has no camera, no cellular modem or SIM, no NFC and no GPS.

## Needs action

| App | Problem |
|---|---|
| **Updater** (LineageOS updater) | Checks `https://ota.gammaos.sh/api/v1/{device}/{variant}`, GammaOS's update server. Nano's own update check (`NanoOtaCheck.cpp`) uses the same URL. On Thimsy, both would offer stock GammaOS updates, and installing one would replace Thimsy. Remove Updater, and turn off or repoint Nano's online check (keep its install-from-storage path for Thimsy packages). |
| **DraStic** + **DrasticSf** | Closed source; to be replaced by WatermelonDS as the default DS emulator. DrasticSf is the small host app for the built-in DS player and goes with it. |

## No clear use on the RG Cube

| App | Why |
|---|---|
| Aperture, ApertureLensLauncher, CameraExtensionsProxy | Camera app and its helpers; no camera. |
| Stk (SIM Toolkit), messaging (SMS), EmergencyInfo | Phone and SIM features; no modem. |
| NfcNci, Tag, SecureElement, QuickAccessWallet | NFC and tap-to-pay; no NFC. |
| DualStackControl | GammaOS's per-app split of one app across two internal screens (`DualStackController.java`), made for dual-screen devices; the Cube has one screen. |
| SecondaryHome | Launcher for a second internal screen (the RG DS on the TV/desktop build). An external display uses Nano. |
| Jelly, GammaBrowser | Two more browsers next to Brave, which setup installs. Keep one. |
| Gallery2, Glimpse | Two galleries; Nano has a photo viewer. Keep one at most, for full-Android mode. |
| Eleven | Music player; Nano has one. |
| MusicFX | Android's equalizer; GammaEQ does the same. |
| BasicDreams, PhotoTable, EasterEgg | Screensavers and the Android version easter egg. |
| BuiltInPrintService | Printing. |
| Etar (calendar), Contacts, LocalContactsBackup, Profiles | Phone-organiser features of little use on a handheld. Optional. |
| Recorder | Sound recorder; only useful if the Cube's microphone is used. |

Removing a system app is a `PRODUCT_PACKAGES` change in `device/phh/treble` / `vendor/lineage` (or a `mods/` delete for the 1.2 base). Some have dependants, so test each:
- removing NfcNci needs the NFC feature XML gone too;
- removing Contacts can leave ContactsProvider without a UI, which is fine.

## Clear use (keep)

**Installed by setup:**
- **emulators:** RetroArch, PPSSPP, Flycast, M64Plus FZ, Dolphin, ARMSX2, Azahar, WatermelonDS, hakuX, Vita3K, ScummVM;
- **pack-in game:** Shattered Pixel Dungeon;
- **apps:** Aurora Store (apps without Google), Brave (browser), Magisk (root, modules).

**GammaOS/Thimsy system apps:**
- ShaderControl (Thimsy Shaders, and the Nano Keyboard input method);
- GammaShares (network shares in Android's file picker);
- SecondaryDisplayControl (apps on an external display);
- LaunchGuardControl (blocks wrong emulator launches);
- JoystickLedPicker (stick lights);
- GammaEQ (equalizer);
- Screen Mapper (touch-to-button mapping).

**Android and LineageOS:**
- Settings, SystemUI, Trebuchet (full-Android home);
- DocumentsUI (Files), PackageInstaller, LatinIME (keyboard), WebView;
- LineageParts, LineageSetupWizard, Seedvault (backup), AccessibilityMenu;
- ThemePicker, Backgrounds and LiveWallpapersPicker (full-Android wallpapers);
- ExactCalculator;
- the framework providers and services (Shell, MtpService, DownloadProvider, MediaProvider, BluetoothMidiService, CertInstaller, KeyChain, CaptivePortalLogin, NetworkStack, and so on).
