# Thimsy Installer app (Flutter)

The single window users see. It never talks to devices or parses firmware
itself: it runs the bundled engine, `thimsy-installer`, and reads the JSON
Lines it prints (see `../docs/engine-protocol.md`).

Flutter **3.50.0** (stable). Material comes from `package:material_ui`.

## Run

```sh
flutter pub get                                  # also generates lib/l10n/app_localizations*.dart
flutter run -d linux --dart-define=FAKE_ENGINE=true   # canned engine, no device needed
flutter run -d linux                                  # real engine (see below)
```

`FAKE_ENGINE=true` swaps in `lib/engine/fake_engine.dart`, which answers like
the real engine (RG Cube, a needs-Windows MediaTek device, journeys, space
checks, cleaning) so every screen can be tried.

## How the engine is found

1. `THIMSY_ENGINE` environment variable, if set (handy during development:
   `THIMSY_ENGINE=../engine/target/debug/thimsy-installer flutter run -d linux`).
2. Otherwise `<directory of the app executable>/engine/thimsy-installer`
   (`thimsy-installer.exe` on Windows).

Every command gets `--lang <code> --json`. `<code>` is the Thimsy language
code of the UI language (`en`, `ar`, … `zh-cn`, `zh-tw`), so the engine's
stage titles, errors and device guides come back translated.

## Languages

English plus Thimsy's 21 languages: `lib/l10n/app_<locale>.arb` (`zh` is
Simplified, `zh_Hant` Traditional). `test/l10n_test.dart` checks every ARB
has exactly the keys and placeholders of `app_en.arb`. The globe button in
the step rail picks a language (saved with shared_preferences); the default
follows the system, with English as the fallback. Arabic and Hebrew lay the
window out right to left.

## Packaging (CI)

Build, then copy the engine (and its `tools/` folder) next to the app
executable:

| OS | Build | Copy the engine to |
|---|---|---|
| Linux | `flutter build linux --release` | `build/linux/x64/release/bundle/engine/` |
| Windows | `flutter build windows --release` | `build/windows/x64/runner/Release/engine/` |
| macOS | `flutter build macos --release` | `build/macos/Build/Products/Release/thimsy_installer.app/Contents/MacOS/engine/` |

Linux build packages: `clang cmake ninja-build pkg-config libgtk-3-dev
liblzma-dev libstdc++-12-dev`.

The macOS app runs without the App Sandbox (`macos/Runner/*.entitlements`),
because the engine needs USB, the network and arbitrary folders. Android
builds show "comes in a later version": the engine isn't bundled there yet.

## Tests

```sh
flutter analyze
flutter test
```

`test/flow_test.dart` drives the whole walkthrough with the fake engine:
both journeys (first time with unlock / returning with update), the
needs-Windows dialog and export, not-enough-space (the build is never
started), flash retries, freeing space, dry runs, languages and RTL.
`test/process_engine_test.dart` runs a shell-script stand-in engine to
check arguments and JSON Lines handling.
