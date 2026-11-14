# Thimsy Installer

One app, start to finish, that installs Thimsy OS on a supported handheld:
pick the device and a Light / Medium / Heavy profile, download the device's
official GammaOS firmware and the Thimsy pieces, build the patched firmware on
your computer, unlock the bootloader (first install only), and flash it over
USB. Design: [`../universal_installer.md`](../universal_installer.md).

| Folder | What |
|---|---|
| `engine/` | Rust workspace: `thimsy-engine` (library) and `thimsy-installer` (the CLI the app runs in the background) |
| `app/` | Flutter app (Windows, macOS, Linux; Android shell) |
| `catalog/` | Devices, profiles, shared system images, translations (`i18n/`). Embedded in the engine at build time |
| `docs/engine-protocol.md` | The JSON Lines protocol between app and engine |
| `tests/` | End-to-end test on real RG Cube firmware (`e2e.sh`), the published-firmware check (`verify-drafts.sh`) and a fake `spd_dump` |
| `tools/make-thimsy-build.sh` | Turns a firmware from `scripts/build.sh` into the Thimsy build the installer ships (system image + patches), checking the rebuild is byte-identical |
| `tools/apply_pack.py` | Writes a device pack (`thimsy-installer dev pack`) into the catalog |
| `tools/spd_dump-windows-driver.patch` | Applied to `spd_dump` for the Windows build: it says when Windows gave the device another driver than WinUSB (Unisoc's "SPRD U2S Diag", usbser, none), so the engine can switch it to WinUSB, and logs what Windows has for Unisoc USB devices |

## Status (0.1.0 test 1)

- **RG Cube, Heavy profile**: builds on every desktop and matches the Thimsy
  1.0a reference PAC byte for byte; flashes with the bundled `spd_dump` on
  Windows and Linux; macOS untested. On Windows the app installs WinUSB; when
  Unisoc's own driver is installed too (unlocking needs it, but it can't
  flash), the app asks for permission before the device is connected and
  a helper switches the device to WinUSB the moment it shows up
  (`tools/spd_dump-windows-driver.patch` reports what Windows gave it).
- Light and Medium are defined but their system images aren't published yet.
- Other devices are listed as candidates. Devices that need Windows-only
  vendor tools say so on other computers.
- Android: shows a "later version" notice; no flashing from Android yet.

## Engine

```sh
cd installer/engine
cargo test --workspace                 # unit + CLI tests (synthetic firmware)
cargo build --release
./target/release/thimsy-installer catalog
./target/release/thimsy-installer resolve --device anbernic-rg-cube --profile heavy
./target/release/thimsy-installer space --device anbernic-rg-cube --profile heavy --workspace ~/thimsy-ws
./target/release/thimsy-installer build --device anbernic-rg-cube --profile heavy --workspace ~/thimsy-ws --import ~/Downloads
./target/release/thimsy-installer flash --job ~/thimsy-ws/jobs/anbernic-rg-cube-heavy --dry-run
```

`installer/tests/e2e.sh WORKSPACE PACK_DIR` runs the real-firmware test: build,
byte-for-byte comparison, dry runs, a full flash through the fake `spd_dump`,
host checks and cleanup.

## Making the Thimsy build

`scripts/build.sh` still builds the full firmware as before. From its output:

```sh
installer/tools/make-thimsy-build.sh [--apply] out/RG_Cube_Thimsy_OS_v1.0a.pac ~/thimsy-build-out
```

This downloads the official GammaOS firmware (or takes `--source PAC`), makes
the device pack, rebuilds the PAC with the installer and stops unless the
result is byte-identical. The device pack is **one archive**,
`thimsy-build/Thimsy-<release>-<device>-<profile>.tar`: a plain tar of the
system image (zstd level 19), the device's `.bsdiff` patches and `pack.json`,
so 7-Zip or `tar` can open it. It's split into 1 GB `.tar.001`, `.002`, ...
parts only when it's over GitHub's 2 GiB asset limit (the RG Cube's is
2.08 GB, so it's one file). The engine reads the system image straight out of
the archive (no unpacking to disk) and checks the archive's sha256 and each
file's.

Upload the archive to the installer release, or ship it next to the app (the
engine looks in `<app>/thimsy-build` and `THIMSY_BUILD_DIR`). Users can also
pick it with "Use a file I already downloaded" (`--import <file>`; any file
name works if the size and sha256 match). `--apply` writes the result into
`installer/catalog` so the next engine build embeds it.

`thimsy-installer dev bundle --out X.tar FILES...` and `dev bundle-list X.tar`
make and list device-pack archives by hand.

The build also accepts the unpacked official firmware (for example
`RG_Cube_GammaOS_Next_Full_v1.2.pac`) in an import folder instead of
downloading the 7z.

`installer/tests/verify-drafts.sh WORKDIR SOURCE_PAC TAG...` repeats the
byte-identity check for published firmware releases.

## Making a device pack by hand

```sh
thimsy-installer dev pack --source official.pac --reference thimsy.pac \
  --out pack/ --device rgcube --system-name thimsy-system-1.0a-heavy
python3 installer/tools/apply_pack.py pack/pack.json \
  installer/catalog/devices/anbernic-rg-cube.yaml installer/catalog/catalog.yaml heavy installer-v0.1.0-test1
```

The pack is the shared system image (zstd) plus bsdiff deltas for each other
changed partition, checked by input and output sha256, and the recipe steps
that rebuild the reference exactly.

## Translations

`catalog/i18n/en.yaml` lists every engine/catalog string
(`thimsy-installer dev i18n-strings --src engine/crates/thimsy-engine/src --out catalog/i18n/en.yaml`);
each `i18n/<code>.yaml` translates it, keyed by the English text like Thimsy's
own `i18n/<code>.json`. A unit test fails when a language is missing a string
or changes a placeholder. The app's own strings are Flutter ARB files in
`app/lib/l10n/`.
