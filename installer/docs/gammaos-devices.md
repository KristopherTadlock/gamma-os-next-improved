# GammaOS devices: candidate reference

Every handheld with a GammaOS Next 1.0–1.3 or GammaOS Nano 1.4.x release, as listed in the Thimsy Installer catalog (`installer/catalog/devices/`). Each device has its own catalog entry with `status: candidate`. None has a `source`, `artifacts`, `build` or `flash` section yet: this page is the starting point for adding them.

Data was collected on 2026-11-12 from the GitHub releases API (`gh api --paginate repos/TheGammaSqueeze/GammaOSNext/releases`, 42 releases), the release notes, and the GammaOS Next wiki (commit `f33a23a9`, 2026-10-24). Sizes are in bytes as reported by GitHub. Every sha256 below is GitHub's own asset `digest` field; every asset on the chosen releases has one. Passwords are copied from the release notes, where GammaOS publishes them once a release goes public.

"Unconfirmed" means the sources disagree or don't say. Check these items before making a device buildable.

## Summary

| Catalog id | Name | SoC | GammaOS release | Upstream install | Password |
|---|---|---|---|---|---|
| `anbernic-rg-cube` | Anbernic RG Cube | Unisoc T820 | `v.1.2.0-ANBERNICRGCUBE` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg556` | Anbernic RG556 | Unisoc T820 | `v.1.2.0-ANBERNICRG556` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg406h` | Anbernic RG406H | Unisoc T820 | `v.1.2.0-ANBERNIC406H` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg406v` | Anbernic RG406V | Unisoc T820 | `v.1.2.0-ANBERNIC406V` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg476h` | Anbernic RG476H | Unisoc T820 | `v.1.2.0-ANBERNICRG476H` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg-slide` | Anbernic RG Slide | Unisoc T820 | `v.1.2.0-ANBERNICRGSLIDE` | Unisoc PAC (ResearchDownload) | Published |
| `anbernic-rg405m` | Anbernic RG405M | Unisoc T618 | `v.1.1.0-ANBERNICT618` | fastboot scripts, web unlock | Published |
| `anbernic-rg405v` | Anbernic RG405V | Unisoc T618 | `v.1.1.0-ANBERNICT618` | fastboot scripts, web unlock | Published |
| `anbernic-rg505` | Anbernic RG505 | Unisoc T618 | `v.1.1.0-ANBERNICT618` | fastboot scripts, web unlock | Published |
| `anbernic-rg-vita` | Anbernic RG Vita | Unisoc T618 (unconfirmed, see entry) | `v1.3.1-ANBERNICRGVITA` | Unisoc PAC (UpgradeDownload) | Published |
| `anbernic-rg-rotate` | Anbernic RG Rotate | Unisoc T618 | `v1.4.1-ANBERNICRGROTATE` | Unisoc PAC (UpgradeDownload) | Published |
| `anbernic-rg557` | Anbernic RG557 | MediaTek Dimensity 8300 | `v.1.2.1-ANBERNICRG557` | SP Flash Tool V6 | Published |
| `anbernic-rg477m` | Anbernic RG477M | MediaTek Dimensity 8300 | `v.1.2.1-ANBERNICRG477M` | SP Flash Tool V6 | Published |
| `anbernic-rg477v` | Anbernic RG477V | MediaTek Dimensity 8300 | `v.1.2.1-ANBERNICRG477V` | SP Flash Tool V6 | Published |
| `anbernic-rg-vita-pro` | Anbernic RG Vita Pro | Rockchip RK3576 | `v1.3.0-ANBERNICRGVITAPRO` | microSD self-installer (raw image) | Not encrypted (one archive checked) |
| `anbernic-rg-ds` | Anbernic RG DS | Rockchip RK3568 | `v1.4.1-ANBERNICRGDS` | microSD self-installer (raw image) | Published |
| `anbernic-rg-ds-plus` | Anbernic RG DS Plus | Rockchip RK3568 | `v1.4.4-ANBERNICRGDSPLUS` | Runs from microSD (Rockchip SD Tool) | Not published |
| `ktpocket-kt-r1` | KTPocket KT-R1 | MediaTek Helio G99 | `v.1.0.0-KTR1` | fastboot scripts | Published |
| `zpg-a1-unicorn` | ZPG A1 Unicorn | MediaTek Helio G99 | `v.1.0.0-ZPGA1UNICORN` | fastboot scripts | Published |
| `ayaneo-pocket-micro` | AYANEO Pocket Micro | MediaTek Helio G99 | `v.1.0.0-AYANEOPOCKETMICRO` | fastboot scripts | Published |
| `retroid-pocket-4-pro` | Retroid Pocket 4 Pro | MediaTek Dimensity 1100 | `v.1.0.0-RETROIDPOCKET4PRO` | fastboot scripts | Published |
| `retroid-pocket-classic` | Retroid Pocket Classic | Qualcomm Snapdragon G1 Gen 2 | `v.1.0.0-RETROIDPOCKETCLASSIC` | fastboot scripts | Published |
| `ayaneo-pocket-air-mini` | AYANEO Pocket Air Mini | MediaTek Helio G90T | `v1.3.1-AYANEOPOCKETAIRMINI` | SP Flash Tool v5 (scatter) | Published |
| `gamemt-e6-max` | GameMT E6 MAX | Allwinner A527 | `v.1.1.0-GAMEMTE6MAX` | fastboot unlock, then PhoenixSuit | Published |
| `mangmi-air-x` | MANGMI Air X | Qualcomm Snapdragon 662 | `v.1.2.0-MANGMIAIRX` | Qualcomm EDL (QFIL) | Published |
| `trimui-brick` | TrimUI Brick | Allwinner A133P | `v1.4.1-TRIMUIBRICK` | Runs from microSD (PhoenixCard) | Published |
| `magicx-mini-zero-28` | MagicX Mini Zero 28 | Allwinner A133P | `v1.4.1-MAGICXMINIZERO28` | Runs from microSD (PhoenixCard) | Published |
| `magicx-xu20-v32` | MagicX XU20 V32 | Allwinner A133P | `v1.4.1-MAGICXXURETROX20V32` | Runs from microSD (PhoenixCard) | Published |
| `magicx-zero-40` | MagicX Zero 40 | Allwinner A133P | `v1.4.1-MAGICXZERO40` | Runs from microSD (PhoenixCard) | Published |
| `gkd-350h-ultra` | GKD 350H Ultra | Rockchip RK3576S | `v1.4.1-GKD350HULTRA` | Runs from microSD (Rockchip SD Tool) | Published |

## How the catalog values were chosen

- **`android_api`:** 33 for the v1.0 builds (Android 13 / LineageOS 20), 34 for everything from v1.1 on (Android 14 / LineageOS 21), as each release's notes state. The Pocket Air Mini's release notes don't state a version (34 inferred).
- **`install`:** from the tool upstream uses, following `universal_installer.md` section 8. Unisoc PAC and fastboot: `untested` on all three desktops. SP Flash Tool: Windows and Linux `untested`, macOS `none`. Qualcomm EDL (planned with `qdl`): `untested` on all three. microSD devices whose upstream card writer runs everywhere (Rockchip SD Tool, or any raw imager): `guided` on all three. Allwinner PhoenixCard devices: Windows `guided`, Linux and macOS `none`, with the generic "Run Thimsy Installer on a Windows PC" `host_message`. `android: none` everywhere.
- **`unlock`:** `guided` where upstream has a manual unlock tool (T820 `UnlockBootloader.bat`, T618 web unlock), `automated` where it is `fastboot flashing unlock` (Helio G99 devices), `none` where the wiki has no unlock step (RG Vita, RG Rotate, SP Flash Tool, EDL and microSD devices, Retroid Pocket Classic).
- **`identify`:** only the Unisoc download-mode ID `1782:4d00` (already used by the RG Cube and RG556) on devices flashed as a PAC in download mode. No other USB IDs were sourced, so other entries have no `identify`.
- **`notes`:** one shared, translated line per install family. The per-device release lives on this page only.
- **Hardware specs** (display, RAM, GPU) come from the maker or retailer spec pages listed under Sources, not from GammaOS. `ram_gb` is the smallest variant. `armv82` follows the CPU cores (Cortex-A55/A75/A76/A78 class: true; Cortex-A53/A72/A73 class: false). `vulkan` is true for every GPU here, but the PowerVR GE8300 (A133P) builds have not been checked for a working Vulkan driver.

## Devices

### Anbernic RG Cube (`anbernic-rg-cube`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-cube` |
| Name | Anbernic RG Cube |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNICRGCUBE`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRGCUBE) (GammaOS Next - Anbernic RG Cube - v1.2, published 2025-12-11) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested) |
| Password protected | Yes. Password published in the release notes: `f297133b935e68072bd77309645cca86ab24bd26847e42c8080db143157a090a` (same value as `source.password` in the catalog) |
| Catalog status | testing (the only buildable device); listed for reference |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_Cube_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `d7b3a4eee7c88c5129cd3070c0df605c87e19585c97a08ad6880c6e7567673a6` |
| `RG_Cube_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `bd51bba348030df62a22110a929468e17d83495d73a72df85ce9868b9b7c4ebf` |
| `RG_Cube_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `78cd87ba3308093581b53e17f512907495c3a0b96acb70f7c4d8dfc3de6e6c78` |
| `RG_Cube_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `eec844dab87e340f30a1e345af81cb4e35ac7036d02e534397460937462352a7` |
| `RG_Cube_GammaOS_Next_Full_v1.2.7z.005` | 461088802 | 439.7 MiB | `71666923d21b2c6e03dab6105c73fe3f71e20abae3054566b324ee0bcb9e66bf` |
| `RG_Cube_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `589e7bac4ebfeb2b6b23fd4f8e6a83456243cb673e0fb13b4e2a0fcdc8b6a1f2` |
| `RG_Cube_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `5132c8aad4f2f72bbded32db8f45d06f41f484f815ef54360d311278e4088dce` |
| `RG_Cube_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `d87e4cdcdcb428b813aa632ff9603e821fdf6d56d4dd11e59f0e9f82ae9ee941` |
| `RG_Cube_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `32896bb8245f4d5946985dad72fcdde13277c06ad0c488611d266eaf43b7e266` |
| `RG_Cube_GammaOS_Next_Lite_v1.2.7z.005` | 264659810 | 252.4 MiB | `2001510cd8e1b51896a0cef933165d27b898e9dd2cc505c8a53a46c3b9f0383f` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG556 (`anbernic-rg556`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg556` |
| Name | Anbernic RG556 |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNICRG556`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRG556) (GammaOS Next - Anbernic RG556 - v1.2, published 2025-12-11) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested) |
| Password protected | Yes. Password published in the release notes: `1361978909684ced7e8ead25fbd62761cc85b3ba4f9d191b5355fa0c93a3f3c7` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_556_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `15fc5f45dcd33cfeaa99013b2df109a24f876a73c61b059bd405c12f2ac65b5a` |
| `RG_556_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `126569b4f543f118df34c93bd5dd6d3ed329427da7396c5a6ca8c1c01b332080` |
| `RG_556_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `6cdc0c33cff82a1389d26bb70a289778c11b43c9666046b2c872d95520cd5450` |
| `RG_556_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `81a62ff93caefd20065f3313f69acf49f19121554fa9d168ad00a8909c84fa50` |
| `RG_556_GammaOS_Next_Full_v1.2.7z.005` | 524288000 | 500.0 MiB | `4d5c96a0f1e38b945f44de40c542133b33a6becd5dd735e5339e36a64e58f1df` |
| `RG_556_GammaOS_Next_Full_v1.2.7z.006` | 60857066 | 58.0 MiB | `837520d86e839a93dfa84fc304390ed3a036d7b5b8bae43690706da3c275683e` |
| `RG_556_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `0814efdbdcd7b3cfabfa66526773bd4af02cec01ff69ddd5d5cbeb9fe9da91d1` |
| `RG_556_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `2452b2776be45f7732ac4b5dd1692b3b413c4166394151238d65df0171a6b0d5` |
| `RG_556_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `a74442627afdbd01c8108ae5323fabd87b79a2f8d907a06ed7cb52627df1a97b` |
| `RG_556_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `b4acf9c64cef4d0dcd68a2fbbf368e006ee3923af71415ecb8d74a3dbcea18f4` |
| `RG_556_GammaOS_Next_Lite_v1.2.7z.005` | 388710042 | 370.7 MiB | `9df967cf499d8baeb618b273572c75465b03a77d4b813d0faffd96176b4430b6` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG406H (`anbernic-rg406h`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg406h` |
| Name | Anbernic RG406H |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNIC406H`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNIC406H) (GammaOS Next - Anbernic RG406H - v1.2, published 2025-12-09) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested) |
| Password protected | Yes. Password published in the release notes: `39811b6439a6b9352025b92ba1629f933a997261a775fd4b249b5880ed291a74` (same password as the RG406V release) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_406H_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `790186910fe2ec74ba2bcbee8dfc82f03f49db8c446ab0285fe887332d518f4c` |
| `RG_406H_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `bf7cc233c9727ba5e8a9f13bbcafb91dedfada9f6aa8f3e2537c9a28cc932ad6` |
| `RG_406H_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `1fb7eae7780d9b9dddd5f6ea595331e9d4f814dcd9aaedebc463345dcbaeeb6d` |
| `RG_406H_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `63b92dcdaaec724f1dcdf2a9177e7748aea0dd2f8cb4476fc63c8956c5782724` |
| `RG_406H_GammaOS_Next_Full_v1.2.7z.005` | 524288000 | 500.0 MiB | `14538ee7d7d034b97eb9c23eeb3df04e9ea5174cc94f71483c171c9ad03f26e9` |
| `RG_406H_GammaOS_Next_Full_v1.2.7z.006` | 63581666 | 60.6 MiB | `4e9d68418bcf110ede807600848e47fc3d59b7faf0e273cd4d094a0e9191c30c` |
| `RG_406H_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `f57e5096a080d1f16d5ebc0f4e279aae724f179f10ffa4873b9f3e15ceb0cf33` |
| `RG_406H_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `50499243eed4284df088ebf31520da7cb2643d062dc44711601607a634ce398f` |
| `RG_406H_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `9466938717e17ea1d521b996cc7a26ef828027ce7b0dd3e46e7f7ff32776e83a` |
| `RG_406H_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `51079634197a73f6e7097bfcce863259092f83efa2b1c354bf9dbf0a4a89ff6b` |
| `RG_406H_GammaOS_Next_Lite_v1.2.7z.005` | 391553954 | 373.4 MiB | `1c39706a236ae8720178da1a2572842d5c60eca7c6f52bc9b53913c2d142ab66` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG406V (`anbernic-rg406v`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg406v` |
| Name | Anbernic RG406V |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNIC406V`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNIC406V) (GammaOS Next - Anbernic RG406V - v1.2, published 2025-12-09) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested) |
| Password protected | Yes. Password published in the release notes: `39811b6439a6b9352025b92ba1629f933a997261a775fd4b249b5880ed291a74` (same password as the RG406H release) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_406V_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `34710e7b06a5394de3521700c91706d2bf482c7565ab63cda4c337b5e7c4ab86` |
| `RG_406V_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `d812122b73e76d5c87c6df7c88315e3f95069ecbbdc33c6594fae94797fef5a2` |
| `RG_406V_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `372457c677cb41bb7e34c2c05a23efc96b6ec557f05a0af6bd86b65fb3e8bb3a` |
| `RG_406V_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `81e9be956de42b2167bc26a2e2bd8ee7f9b6f11b8e5e6910270dd9de0b870cf0` |
| `RG_406V_GammaOS_Next_Full_v1.2.7z.005` | 524288000 | 500.0 MiB | `1506c3e68115a8cc8be000e74e6e7d401317693d21b346d7524f02dbf54b7339` |
| `RG_406V_GammaOS_Next_Full_v1.2.7z.006` | 54582498 | 52.1 MiB | `eb408a566e726ad787cb917dacf599cb35d10330f212913c2b10bc7b3564bb11` |
| `RG_406V_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `18e0cdc9b2313da9ff3ac764b46cf6cfbef838f940c60fa35b1c616467e90ad6` |
| `RG_406V_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `bc4fb95e94a7c69cd6fe4ff0c38445d1f4400427cdde41f2aecb0a6c977ea250` |
| `RG_406V_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `e353deddee9bd486b2dcb21291b193af58bf0ee227861c629c8de413c5beb60c` |
| `RG_406V_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `54d5d2ef1f4070ef7bb6dbd133e32b659c67a11b53dc6e7f7a90edee109edb13` |
| `RG_406V_GammaOS_Next_Lite_v1.2.7z.005` | 382563746 | 364.8 MiB | `c4beb48d9a702572f38203877206f7032c4dbd4728a66da40fe9ae7d62070559` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG476H (`anbernic-rg476h`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg476h` |
| Name | Anbernic RG476H |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNICRG476H`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRG476H) (GammaOS Next - Anbernic RG476H - v1.2, published 2025-12-09) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested) |
| Password protected | Yes. Password published in the release notes: `64215dc9e32b3a38595ff44ea85d9a40a486ad801d6718b401ec0e97104472f7` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_476H_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `28f3745820f55ed472e9a0d24396647c244e19ce31223ab312cffee7376951ac` |
| `RG_476H_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `a9815f3c4250e9b1dceef19491638b86e8f6e7fe2bcfa4a41b52b7263594f8f7` |
| `RG_476H_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `28f03e48ab46a6d58f4f74c7c7d37f4199a00d76b87944ffa354b70461ebb4df` |
| `RG_476H_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `561ff6e52f8d0794255614a04ef81d8dee4f698129f1e1932ff8e26e4e67202e` |
| `RG_476H_GammaOS_Next_Full_v1.2.7z.005` | 524288000 | 500.0 MiB | `7e80b0a39383c944b62c4a0e42d87427bf48e56d91eb7a0266cf5ab7235092f6` |
| `RG_476H_GammaOS_Next_Full_v1.2.7z.006` | 53386994 | 50.9 MiB | `c8ab466d356859a1535010df4ab8de6cede91a38742521ccf5f10e186d104d98` |
| `RG_476H_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `e9e01245d0d87318466e061e8d25271af705e5882cce7bb991906d2cb217b0e1` |
| `RG_476H_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `5a2284e38e89eaf4acc048dc763cdf21fc8a769e0c1c7b1b79026eb21d1515e0` |
| `RG_476H_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `58f1885d5e41d75ad17d382d892d83f2cfa29fad0359f170897c5820a31b0a4e` |
| `RG_476H_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `0314645b45d7e29c01f7fff0924cfa826ae35d68549cf897c9ea54846f4c94b5` |
| `RG_476H_GammaOS_Next_Lite_v1.2.7z.005` | 381362466 | 363.7 MiB | `a3ce377170a22c5da00a194cb94ee3ab0d451d45227ad6978d50bbaadce0d639` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG Slide (`anbernic-rg-slide`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-slide` |
| Name | Anbernic RG Slide |
| SoC | Unisoc T820 |
| GammaOS release | [`v.1.2.0-ANBERNICRGSLIDE`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-ANBERNICRGSLIDE) (GammaOS Next - Anbernic RG Slide - v1.2, published 2025-12-09) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with ResearchDownload (Windows) from `UnisocTools.7z`; bootloader unlocked first with `UnlockBootloader.bat` (download mode: hold Back, plug in USB). Upgrade: FDL1, FDL2 and Super only. Community Linux route: beebono/GammaOS-Linux-Tools (upstream: untested). On the RG Slide the Back button is the one on the right of the screen |
| Password protected | Yes. Password published in the release notes: `e3ea63e364d0db3c040a5475c5de5585e4e22dc2cb945b7dd988bca4fd969d76` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.001` | 524288000 | 500.0 MiB | `a3f3140de89a7197bdba8ff43c01bf5037de47936ffc0b4ab8fe1d2db9fea8f6` |
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.002` | 524288000 | 500.0 MiB | `851f33e4d6fa603b7d3fe8d6a9a1786f15e7ae8d5bbd866d68102e1eee18cc56` |
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.003` | 524288000 | 500.0 MiB | `860e7b4fa5b71b3c448594116869e9c3a223d52cc28c675e2a44cf23196055fc` |
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.004` | 524288000 | 500.0 MiB | `8a943ee293ad1884e309bc4cd5398d9ff71276e5d936da51ab77592bea2f169b` |
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.005` | 524288000 | 500.0 MiB | `f4fed9eba22532a828b18b4a82edf65355ed274eed06b1b1884382ae964ad6be` |
| `RG_Slide_GammaOS_Next_Full_v1.2.7z.006` | 55346178 | 52.8 MiB | `d1ef614ec2924913f5c88ca5661633773c39f9f334571c20fdca4f607a806a07` |
| `RG_Slide_GammaOS_Next_Lite_v1.2.7z.001` | 524288000 | 500.0 MiB | `825a73fac3556ed7ff0f58ba930299abc74731153935f5a577c70f004fa8aa75` |
| `RG_Slide_GammaOS_Next_Lite_v1.2.7z.002` | 524288000 | 500.0 MiB | `ddc7480f0af7d34c161085d19db2670174319148487089f39ec7caea92a14030` |
| `RG_Slide_GammaOS_Next_Lite_v1.2.7z.003` | 524288000 | 500.0 MiB | `0cf60f0c0f2032b32818803fbd8ccc042afb84f3c0193f609571955cfff0fe8c` |
| `RG_Slide_GammaOS_Next_Lite_v1.2.7z.004` | 524288000 | 500.0 MiB | `1ec57b96f36a4d0a3f1ce18222e7c5f916a3bc9d5bc016059706db481c3e94c7` |
| `RG_Slide_GammaOS_Next_Lite_v1.2.7z.005` | 383327474 | 365.6 MiB | `5a0d276cd07a586830b4ef0cf331d3b9de3f9a2836ec03b439eb50bb4c23da44` |
| `UnisocTools.7z` | 15144699 | 14.4 MiB | `5caa80610ce86aebfc572815df76b2270d3d3a59ada0e2796eb2e79e7c985e75` |

### Anbernic RG405M (`anbernic-rg405m`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg405m` |
| Name | Anbernic RG405M |
| SoC | Unisoc T618 |
| GammaOS release | [`v.1.1.0-ANBERNICT618`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.1.0-ANBERNICT618) (GammaOS Next - Anbernic RG405M / RG405V / RG505 - v1.1, published 2025-11-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | fastboot: `FlashPartitions.bat`/`.sh` then `EraseUserData` from fastbootd, after a bootloader unlock with the web tool (thegammasqueeze.github.io/subut-rehost) or `unisoc-unlock` (Python). Fresh install only. Wiki gives Windows, macOS and Linux commands |
| Password protected | Yes. Password published in the release notes: `e70d48dd31d62a88b3c4f774072c93a04aecb4a0255bf55fe00458406172212b` (one password for the whole T618 release) |
| Release covers | RG405M, RG405V and RG505 in one release; only this device's assets are listed. The release notes say it "must be installed fresh from the original GammaOS v1.5.1"; the wiki says the fresh install replaces stock, v1.5.1 or any ROM. Which is required is unconfirmed |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG405M_GammaOSNext_Full_v1.1.7z.001` | 524288000 | 500.0 MiB | `1b191ee85d551c9eb7d4648a3ce2f155dfad6460a697799c821994a26bb3d15f` |
| `RG405M_GammaOSNext_Full_v1.1.7z.002` | 524288000 | 500.0 MiB | `44bcbf9107967c893a4bfbe3defd81a425705150de06e59c2d6f00c3970429cb` |
| `RG405M_GammaOSNext_Full_v1.1.7z.003` | 524288000 | 500.0 MiB | `b0c9f412d7ff80082e9b473aa804380925e24ccca509db2abf823d66548ee377` |
| `RG405M_GammaOSNext_Full_v1.1.7z.004` | 524288000 | 500.0 MiB | `fe6901d2ecad54fd7acf6d4056ceed456f53b1d0ebf0292959584db44e0b2204` |
| `RG405M_GammaOSNext_Full_v1.1.7z.005` | 524288000 | 500.0 MiB | `92eb92062ac7fb33fc0c43244b6eeb7a793cf8acb439a19722253670fdcfd840` |
| `RG405M_GammaOSNext_Full_v1.1.7z.006` | 4162364 | 4.0 MiB | `ee7839124cd78828d1971b763ad5f7763b4f3c8d88e20073263b34a5d494e5f9` |
| `RG405M_GammaOSNext_Lite_v1.1.7z.001` | 524288000 | 500.0 MiB | `bf782a089f21119e709fef52873a90cbb92562c686bad57069aa49fde095cbe0` |
| `RG405M_GammaOSNext_Lite_v1.1.7z.002` | 524288000 | 500.0 MiB | `8e9479e6e79dee9682148006e0fdc9396e137e605a53efc322f35e9b4f622c3c` |
| `RG405M_GammaOSNext_Lite_v1.1.7z.003` | 524288000 | 500.0 MiB | `8769149a393e45a7c82cd77d62ae3af9f2724f88d40e4a0ae51c2172a314a840` |
| `RG405M_GammaOSNext_Lite_v1.1.7z.004` | 524288000 | 500.0 MiB | `f3b05197d8bec7590046a4be0ed59f44dca0a0bfc5d2351d889f31d977ab14bc` |
| `RG405M_GammaOSNext_Lite_v1.1.7z.005` | 328916032 | 313.7 MiB | `3cceee0c865ea4b7bfca9578cae6d7a5512c02b8df76d0791b3613f79a25fe69` |

### Anbernic RG405V (`anbernic-rg405v`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg405v` |
| Name | Anbernic RG405V |
| SoC | Unisoc T618 |
| GammaOS release | [`v.1.1.0-ANBERNICT618`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.1.0-ANBERNICT618) (GammaOS Next - Anbernic RG405M / RG405V / RG505 - v1.1, published 2025-11-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | fastboot: `FlashPartitions.bat`/`.sh` then `EraseUserData` from fastbootd, after a bootloader unlock with the web tool (thegammasqueeze.github.io/subut-rehost) or `unisoc-unlock` (Python). Fresh install only. Wiki gives Windows, macOS and Linux commands |
| Password protected | Yes. Password published in the release notes: `e70d48dd31d62a88b3c4f774072c93a04aecb4a0255bf55fe00458406172212b` (one password for the whole T618 release) |
| Release covers | Shared T618 release; see the RG405M entry for the v1.5.1 prerequisite question |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG405V_GammaOSNext_Full_v1.1.7z.001` | 524288000 | 500.0 MiB | `3a74228a55fde2d421213079120d8a6189bd907189f1b8e714beb8f2866c7dbf` |
| `RG405V_GammaOSNext_Full_v1.1.7z.002` | 524288000 | 500.0 MiB | `e3f3078a8d1e2badbcdffb85eac90366f565bf521cc15578933f2c8e903af6fb` |
| `RG405V_GammaOSNext_Full_v1.1.7z.003` | 524288000 | 500.0 MiB | `5133c425a17fb055533da91bfe74bde54456a08953d9ddad95ac7e1d87141b9e` |
| `RG405V_GammaOSNext_Full_v1.1.7z.004` | 524288000 | 500.0 MiB | `a95e23a16040cfab69a944293fa6188e0e711594ff13aacfc2048cabc83d0295` |
| `RG405V_GammaOSNext_Full_v1.1.7z.005` | 524288000 | 500.0 MiB | `8c8ca36182180de5d5f6f100f21af286d23ebfe076da87d0ac0938f12782f460` |
| `RG405V_GammaOSNext_Full_v1.1.7z.006` | 4213249 | 4.0 MiB | `42159812b5cb03aa49b1b8cd9fe69f65d1342a1d643e70399b7c81635c07552c` |
| `RG405V_GammaOSNext_Lite_v1.1.7z.001` | 524288000 | 500.0 MiB | `a89cba94df684b4930aca75dcf070b3623e0bde08affc3561ea685fa926c5d0b` |
| `RG405V_GammaOSNext_Lite_v1.1.7z.002` | 524288000 | 500.0 MiB | `5829813a77b0621456d9170015ee6391ea1f863d6e31bef53c4daecb42606edf` |
| `RG405V_GammaOSNext_Lite_v1.1.7z.003` | 524288000 | 500.0 MiB | `8c851dc7c120acde4a44ed30808a9df92c999f0ed47a83f218f589ee12e6bda3` |
| `RG405V_GammaOSNext_Lite_v1.1.7z.004` | 524288000 | 500.0 MiB | `980b37d982de973ae7083b39cfe90208c322233161bd11601b961bf3231236c4` |
| `RG405V_GammaOSNext_Lite_v1.1.7z.005` | 328740864 | 313.5 MiB | `2d85c46fac9e74e7dd11b285ce2b053f71014205c70cf581782df7164a6aa5a3` |

### Anbernic RG505 (`anbernic-rg505`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg505` |
| Name | Anbernic RG505 |
| SoC | Unisoc T618 |
| GammaOS release | [`v.1.1.0-ANBERNICT618`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.1.0-ANBERNICT618) (GammaOS Next - Anbernic RG405M / RG405V / RG505 - v1.1, published 2025-11-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | fastboot: `FlashPartitions.bat`/`.sh` then `EraseUserData` from fastbootd, after a bootloader unlock with the web tool (thegammasqueeze.github.io/subut-rehost) or `unisoc-unlock` (Python). Fresh install only. Wiki gives Windows, macOS and Linux commands |
| Password protected | Yes. Password published in the release notes: `e70d48dd31d62a88b3c4f774072c93a04aecb4a0255bf55fe00458406172212b` (one password for the whole T618 release) |
| Release covers | Shared T618 release; see the RG405M entry for the v1.5.1 prerequisite question |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG505_GammaOSNext_Full_v1.1.7z.001` | 524288000 | 500.0 MiB | `cc2b65194bfe36901aab63071d5072636f0dcc7e7b323ffedc91ce44a8f3d9d6` |
| `RG505_GammaOSNext_Full_v1.1.7z.002` | 524288000 | 500.0 MiB | `58627d9ac538451ca3aa4208ca91524ec511df1d409de4ca58efa342e649c460` |
| `RG505_GammaOSNext_Full_v1.1.7z.003` | 524288000 | 500.0 MiB | `0a115c284719932a3cf598259ab2407a00940b1883127aeacd106ac05cda12db` |
| `RG505_GammaOSNext_Full_v1.1.7z.004` | 524288000 | 500.0 MiB | `b113bbc3323589c5fe8111ac7e84bb0d49860741d5b07baeaf93a6b5646dd8e7` |
| `RG505_GammaOSNext_Full_v1.1.7z.005` | 524288000 | 500.0 MiB | `6fdbc5f4ae1e3f21794edd6198715f39e9115bb313e8661451f9c6e10ff972c5` |
| `RG505_GammaOSNext_Full_v1.1.7z.006` | 28848426 | 27.5 MiB | `978be1c40323e54d71007ecd15d7010a368a5d801ba7816d2644599318111f5a` |
| `RG505_GammaOSNext_Lite_v1.1.7z.001` | 524288000 | 500.0 MiB | `59ab385d728b1a2ee7c70494c2cd5a7a4297443a7d65064276c21b231a0de333` |
| `RG505_GammaOSNext_Lite_v1.1.7z.002` | 524288000 | 500.0 MiB | `b0a161939cd95205f4f992b8dcfc39c1b3f87f363a9e4e1f5ae7cca0d5f81d6a` |
| `RG505_GammaOSNext_Lite_v1.1.7z.003` | 524288000 | 500.0 MiB | `8a1ff2090867441dee1f23ed82bab2804f6c63b7008d8cc6f0aa33c41fdebaea` |
| `RG505_GammaOSNext_Lite_v1.1.7z.004` | 524288000 | 500.0 MiB | `70c2f572f3660a340f9d4d3c07894726f2f280bf7c4a1770d3454bef712207bc` |
| `RG505_GammaOSNext_Lite_v1.1.7z.005` | 353665185 | 337.3 MiB | `6d1f76be3e57f15d7e67fe0d5385394fe4ff9c17f88bff7c381faab97eecafaf` |

### Anbernic RG Vita (`anbernic-rg-vita`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-vita` |
| Name | Anbernic RG Vita |
| SoC | Unisoc T618 (see note) |
| GammaOS release | [`v1.3.1-ANBERNICRGVITA`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.3.1-ANBERNICRGVITA) (GammaOS Next - Anbernic RG Vita - v1.3.1, published 2026-04-27) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Unisoc `.pac` flashed with UpgradeDownload (Windows) from `UnisocTools_RG_VITA.zip`; download mode: hold Back and the Anbernic button, plug in USB. No unlock step in the wiki. One archive holds both `RG_VITA_GammaOS_Lite_v1.3.1.pac` and `RG_VITA_GammaOS_Full_v1.3.1.pac`. Fresh install only |
| Password protected | Yes. Password posted in the release notes (UPDATE line): `DFBDVAWERRFG45WGR32FVRT$gvSDFVBSDFVSDFVWE5T54` |
| SoC note | The release notes say "Unisoc T820 platform", but Anbernic's spec sheet (via retrohandhelds.gg and liliputing) lists the Unisoc T618, and `UnisocTools_RG_VITA.zip` is byte-identical (same sha256) to the RG Rotate's T618 (`ums512`) tool bundle. Catalog uses T618; treat as unconfirmed until the PAC is inspected |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_VITA_GammaOS_v1.3.1.7z.001` | 1048576000 | 1000.0 MiB | `704d2acf4800f68bc941c1aec2efca6f3e8368e1db009a302503a465ba8af4f7` |
| `RG_VITA_GammaOS_v1.3.1.7z.002` | 1048576000 | 1000.0 MiB | `0e5453bf5448ff940d06f4299e98d8820dbd9ecba9f37ec944726e114ab6a95a` |
| `RG_VITA_GammaOS_v1.3.1.7z.003` | 1048576000 | 1000.0 MiB | `3ec54b5c9a33357ac912ccac4014320e722588b3c5b6a1b22bedf75716ba767e` |
| `RG_VITA_GammaOS_v1.3.1.7z.004` | 1048576000 | 1000.0 MiB | `e5ec44a63f8904fa41d1420da86703e04f8d05beefd56248fb9eea441be2391f` |
| `RG_VITA_GammaOS_v1.3.1.7z.005` | 1048576000 | 1000.0 MiB | `d55bb9322569d5c9a96c935caa8a1570fc42807d62bf7409bfdb6bb8c11518ff` |
| `RG_VITA_GammaOS_v1.3.1.7z.006` | 842646911 | 803.6 MiB | `885099ed570c4f2fa121db3445c375d85a5c188e95b38422a1d2ab250ff7d67e` |
| `UnisocTools_RG_VITA.zip` | 57686410 | 55.0 MiB | `3b94905051d77abb014a091cedfcda4668ddd5f7e4e1430769000010a5913fdf` |

### Anbernic RG Vita Pro (`anbernic-rg-vita-pro`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-vita-pro` |
| Name | Anbernic RG Vita Pro |
| SoC | Rockchip RK3576 |
| GammaOS release | [`v1.3.0-ANBERNICRGVITAPRO`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.3.0-ANBERNICRGVITAPRO) (GammaOS Next - Anbernic RG Vita Pro - v1.3.0, published 2026-04-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | microSD self-installer: write `*_SDCARDINSTALL.img` raw to a card (Rufus, balenaEtcher, Raspberry Pi Imager or `dd`, on Windows, macOS and Linux), boot the device from it and it flashes internal storage. Fresh install only |
| Password protected | No (verified for `RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z`: the 7z end header is unencrypted, single LZMA2 coder, no AES; it holds `RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.img`). The release notes and wiki mention no password. The other three archives were not checked |
| Variants | Four archives: Lite/Full, each standard (factory overclock) or `NO_OC_` (stock clocks) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `NO_OC_RG_VITA_PRO_GammaOS_Full_v1.3.0_SDCARDINSTALL.7z.001` | 2086666240 | 1.94 GiB | `623f2fe59a12b7f2bcb70deedea9bd5dc66e6590d63c48cbfb886666f40e1a46` |
| `NO_OC_RG_VITA_PRO_GammaOS_Full_v1.3.0_SDCARDINSTALL.7z.002` | 1089985574 | 1.02 GiB | `880da0ef207d4a0d3e3aedcdbd8992d6006934d3fab82371d77af10fb16fad91` |
| `NO_OC_RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z.001` | 2086666240 | 1.94 GiB | `e4b3d51e875b5dec16fd5dec70dc3c04688b3f1af5691a7bafd1e25b8a8fd4b3` |
| `NO_OC_RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z.002` | 1242039856 | 1.16 GiB | `93e9af90ea87aa0a6de38584f93e211136d01dee556be4f84f570f130fdfef91` |
| `RG_VITA_PRO_GammaOS_Full_v1.3.0_SDCARDINSTALL.7z.001` | 2086666240 | 1.94 GiB | `cf5fdd968b448e1b356eba4bbd6ae92f10d0cbe12b6cedffa6cabbd578b5b9f3` |
| `RG_VITA_PRO_GammaOS_Full_v1.3.0_SDCARDINSTALL.7z.002` | 1090021535 | 1.02 GiB | `876f6dd1458ab8d9ac3fcda05c8896918207a1d71fc37fb293eef1c4be9bfbe1` |
| `RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z.001` | 2086666240 | 1.94 GiB | `b62e4527168535ccbcf31f77cb8ab9e24eeb6ba87a5d7514fa9d8901d7d873c7` |
| `RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z.002` | 1242017582 | 1.16 GiB | `0c70e751df98e1f9825a6dd62a99d88a46d313974993de2f9da9ee8c41817f86` |

### Anbernic RG Rotate (`anbernic-rg-rotate`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-rotate` |
| Name | Anbernic RG Rotate |
| SoC | Unisoc T618 (`ums512`) |
| GammaOS release | [`v1.4.1-ANBERNICRGROTATE`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-ANBERNICRGROTATE) (GammaOS Nano v1.4.1 (Anbernic RG Rotate) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on top) |
| Install method (upstream) | Unisoc `.pac` flashed with UpgradeDownload (Windows) from `UnisocTools_RG_ROTATE.zip`; download mode: slide the device open, hold the Back button in the middle of the joypad, plug in USB. Remove SD cards first (patched SPL). No unlock step in the wiki. Fresh install |
| Password protected | Yes. Password published in the release notes (public release): `o4eURadAja0G4CXZF4JD6CPz9J7M5bQn5oMyi51d5l1CXtZ9LK` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `ums512_1h10_rg_rotate_gammaos_1.4.1_full.7z.001` | 2096103424 | 1.95 GiB | `79ad726867243cc63d5cbbef76f21c3d4a447b7c2f015b88b39c26476f2d9c54` |
| `ums512_1h10_rg_rotate_gammaos_1.4.1_full.7z.002` | 913824652 | 871.5 MiB | `a68ffb7134c37b2355f444b1aae35528fcf6fc04d621c5453537d6db3881a85f` |
| `ums512_1h10_rg_rotate_gammaos_1.4.1_lite.7z.001` | 2096103424 | 1.95 GiB | `73e81ca11213ac7fad2b511a2f2453842583ca7276aae1a9be62fd8112e9dbc7` |
| `ums512_1h10_rg_rotate_gammaos_1.4.1_lite.7z.002` | 803191244 | 766.0 MiB | `6984f21214c355c7eb2a08e5718dedb49daa0088223a9a93796ddafc7da34975` |
| `UnisocTools_RG_ROTATE.zip` | 57686410 | 55.0 MiB | `3b94905051d77abb014a091cedfcda4668ddd5f7e4e1430769000010a5913fdf` |

### Anbernic RG DS (`anbernic-rg-ds`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-ds` |
| Name | Anbernic RG DS |
| SoC | Rockchip RK3568 |
| GammaOS release | [`v1.4.1-ANBERNICRGDS`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-ANBERNICRGDS) (GammaOS Nano v1.4.1 (Anbernic RG DS) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on top) |
| Install method (upstream) | microSD self-installer. For v1.4.1 and older: write the `.img` raw with balenaEtcher, Raspberry Pi Imager, Rufus or `dd`; the device flashes its internal storage from the card. From v1.4.4 the card must be written with TheGammaSqueeze/rockchip_sd_tool v1.3.0+ (Windows, Linux, macOS) as a "Firmware update card" |
| Password protected | Yes. Password published in the release notes (public release): `y4nsAmZcpwhMe1j9007Jv9wiBT9ccBEUP3pdexH9L3obwGVMD9` |
| Why v1.4.1 | Newest RG DS release with a public password. v1.4.4 (`v1.4.4-ANBERNICRGDS`) is newer but its password is still supporter-only. Older: v1.4.0 (public password), v1.2.2 (Android 14 non-Nano, public password) |
| Variants | Core and Full, each also as `_NO_OC_UV` (stock clocks and voltages) |
| Display | Two 4.0-inch 640×480 screens (catalog `display` shows one panel) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG_DS_GammaOS_Core_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `d4c8c71662a77ca7a17b2d1496ecad02c35e5294f2e0fb9d532c9f9e7702ac2e` |
| `RG_DS_GammaOS_Core_v1.4.1.7z.002` | 2096103424 | 1.95 GiB | `6c5f05c3700bf26f316f36758c92abf312c7927659fa5f053aaafec6a34a6ebc` |
| `RG_DS_GammaOS_Core_v1.4.1.7z.003` | 1439993676 | 1.34 GiB | `6dea7bacc19a75c0824c491f96ba1e45f9155b3f6847c4bc22814204afaa65fd` |
| `RG_DS_GammaOS_Core_v1.4.1_NO_OC_UV.7z.001` | 2096103424 | 1.95 GiB | `d4c05222a99aa88feae1ab15077c00502c54143a5bbfeb381268cdc9d36aa6a2` |
| `RG_DS_GammaOS_Core_v1.4.1_NO_OC_UV.7z.002` | 2096103424 | 1.95 GiB | `f6ff8f6b853ae644a8e8314b127d1192a7f11735ca0f6a523472bcc662eba5e5` |
| `RG_DS_GammaOS_Core_v1.4.1_NO_OC_UV.7z.003` | 1418381676 | 1.32 GiB | `f4d16aa0c24b3997bde6899512eee37e2b0bb21341bb6d6f4c96af4651a3de11` |
| `RG_DS_GammaOS_Full_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `a7036c196f5cec42bd71b105917ea9020b33d63a980252af31709cafc924a131` |
| `RG_DS_GammaOS_Full_v1.4.1.7z.002` | 2096103424 | 1.95 GiB | `5f095fcc2736550d048e9a82867d508fdfad7d737740fc640de49d6c3316f198` |
| `RG_DS_GammaOS_Full_v1.4.1.7z.003` | 1518268204 | 1.41 GiB | `58e3b3bc49d8a5ee2676a9c87007d9cd32a77d7c4468fe419f44ff7ff0ccad81` |
| `RG_DS_GammaOS_Full_v1.4.1_NO_OC_UV.7z.001` | 2096103424 | 1.95 GiB | `02688e6a70cd6c711c636f8972ea5a6e2e7a2f5263cd589ce47da0fdb77155c7` |
| `RG_DS_GammaOS_Full_v1.4.1_NO_OC_UV.7z.002` | 2096103424 | 1.95 GiB | `1ef539508a17d1b2a50da0eda2d655dcd0b80c9a1aafb68955308a66ebaf658f` |
| `RG_DS_GammaOS_Full_v1.4.1_NO_OC_UV.7z.003` | 1518259148 | 1.41 GiB | `103d484dcb3a9b4676726f1ae6ceb84a72094732a7ed8c76476d3e6e5a85c713` |

### Anbernic RG DS Plus (`anbernic-rg-ds-plus`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg-ds-plus` |
| Name | Anbernic RG DS Plus |
| SoC | Rockchip RK3568 |
| GammaOS release | [`v1.4.4-ANBERNICRGDSPLUS`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.4-ANBERNICRGDSPLUS) (GammaOS Nano v1.4.4 (Anbernic RG DS Plus) Release Notes, published 2026-10-03) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core; stated in the v1.4.2 notes) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with TheGammaSqueeze/rockchip_sd_tool (Windows, Linux, macOS; recommended) or the patched Windows-only `GammaOS_SDDiskTool_v1.69_SDBootOnly.zip`. dd, Win32DiskImager and balenaEtcher do not work |
| Password protected | Yes, password not published: supporter-only (posted in the Patreon/Ko-fi Discord channel). Same for v1.4.3; v1.4.2 has no downloads. No public RG DS Plus build exists yet |
| Stock OS | Ships with Linux; Android comes only from the GammaOS card |
| Display | Two 4.3-inch 1024×768 screens |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `GammaOS_SDDiskTool_v1.69_SDBootOnly.zip` | 487395 | 0.5 MiB | `b43042730f1b65cb41aa561e0f217ccb0d13b654f306d7294f2ca13f22a176d9` |
| `RG_DS_Plus_GammaOS_Core_v1.4.4.7z.001` | 2096103424 | 1.95 GiB | `4067492a46a616d3b6a08b6ae57e41848fe89a33d690cefcb28b85af975a0b7b` |
| `RG_DS_Plus_GammaOS_Core_v1.4.4.7z.002` | 476190786 | 454.1 MiB | `cb2bfce2cd57ed8b87b40c30e872ab2f9980ba465fa83350a8b231213fcc32ae` |
| `RG_DS_Plus_GammaOS_v1.4.4_boot_fix-OTA.zip` | 12779107 | 12.2 MiB | `ec31ddff92e27fbd6d225e36ff3aded53da5f71e892f9320e3584964c3418425` |

### Anbernic RG557 (`anbernic-rg557`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg557` |
| Name | Anbernic RG557 |
| SoC | MediaTek Dimensity 8300 |
| GammaOS release | [`v.1.2.1-ANBERNICRG557`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.1-ANBERNICRG557) (GammaOS Next - Anbernic RG557- v1.2.1 [With Bootloop Fix], published 2025-12-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | MediaTek SP Flash Tool V6 (Windows only, bundled as `SP_Flash_Tool_V6.2404_Win.zip`): Format UFS-LUA0 and UFS-LUA1, Download with `flash_128GB.xml` or `flash_256GB.xml` matching the unit's storage, then re-flash preloaders (Write Memory). Wrong XML can brick |
| Password protected | Yes. Password published in the release notes: `c11f181d9938a58fc273d9ec7e92e3407dec3bc31c76923e3a829ea7df624017` (same password as the RG477M and RG477V releases) |
| Block | Do not install on units that shipped with stock v1.26 (hardware change) |
| Note | The firmware parts were replaced on 27 December 2025 with the same file names (bootloop fix); the sizes and digests here are the current files. Upstream also recommends the `thermal_safety_protection` Magisk module |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG557_GammaOS_Next_v1.2.1.7z.001` | 1048576000 | 1000.0 MiB | `06e03d73b58b90a6f8fa9a6a03a03e5ee39974fe25eccb0e26b78878fcedab81` |
| `RG557_GammaOS_Next_v1.2.1.7z.002` | 1048576000 | 1000.0 MiB | `3a6ec30119bfa4cb4a21995d2e71c0692f370e5fa799d43d7ca22d37e0f95c5d` |
| `RG557_GammaOS_Next_v1.2.1.7z.003` | 1048576000 | 1000.0 MiB | `fc07c517a9fc768688600b996dacb7de1c6e01c1994b17b1f3232b683a252d93` |
| `RG557_GammaOS_Next_v1.2.1.7z.004` | 1048576000 | 1000.0 MiB | `f92433501fbeeb8c57c79cbf632f0bb5bc24814c361675fa338e260fdfc47500` |
| `RG557_GammaOS_Next_v1.2.1.7z.005` | 655738422 | 625.4 MiB | `2a6ec53e94fc12523c31bf3f8b5525685f9fa83aa9b968674de0acbbeaaaf31d` |
| `SP_Flash_Tool_V6.2404_Win.zip` | 21722734 | 20.7 MiB | `ba6746932c8e3553b363f7bd0ae43882cd44dfd8ee814ae81052c98e572076e2` |
| `thermal_safety_protection.zip` | 12977 | 0.0 MiB | `5cb2a90dfd677e10aa254910c8f6451729eb3da49685c084496782954318352b` |
| `thermal_safety_protection_2.zip` | 15623 | 0.0 MiB | `bf184bc6c179ff3e6f75a2c7d6f0983323471279e5b6249bd384886d65298eaf` |

### Anbernic RG477M (`anbernic-rg477m`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg477m` |
| Name | Anbernic RG477M |
| SoC | MediaTek Dimensity 8300 |
| GammaOS release | [`v.1.2.1-ANBERNICRG477M`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.1-ANBERNICRG477M) (GammaOS Next - Anbernic RG477M- v1.2.1, published 2025-12-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | MediaTek SP Flash Tool V6 (Windows only, bundled as `SP_Flash_Tool_V6.2404_Win.zip`): Format UFS-LUA0 and UFS-LUA1, Download with `flash_128GB.xml` or `flash_256GB.xml` matching the unit's storage, then re-flash preloaders (Write Memory). Wrong XML can brick |
| Password protected | Yes. Password published in the release notes: `c11f181d9938a58fc273d9ec7e92e3407dec3bc31c76923e3a829ea7df624017` (same password as the RG557 and RG477V releases) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG477M_GammaOS_Next_v1.2.1.7z.001` | 1048576000 | 1000.0 MiB | `853ac3cf9378f5a83cee581ca9c87fca923f5c334b0b01920087ba2539f8b695` |
| `RG477M_GammaOS_Next_v1.2.1.7z.002` | 1048576000 | 1000.0 MiB | `9f17431af657fecd13cf75a746d650701651f9377874a5107f16d593d6d1141d` |
| `RG477M_GammaOS_Next_v1.2.1.7z.003` | 1048576000 | 1000.0 MiB | `d5151657a0bf5526c04ff2df8f6479756b31108cb73974c1b7839a936f8bbe71` |
| `RG477M_GammaOS_Next_v1.2.1.7z.004` | 1048576000 | 1000.0 MiB | `956a177bf04aa7d5e04780c187280853b8aa5a4c1fc4d403edf126e2f32677df` |
| `RG477M_GammaOS_Next_v1.2.1.7z.005` | 1048576000 | 1000.0 MiB | `b0eb261efb16aba57777d0f854ffaa6f6281b97f8a1e39d633861f4cf66f0180` |
| `RG477M_GammaOS_Next_v1.2.1.7z.006` | 1048576000 | 1000.0 MiB | `90c81d6fcd26287404c6053377405474c618ed12a3a917389f9c2da6fac91379` |
| `RG477M_GammaOS_Next_v1.2.1.7z.007` | 1048576000 | 1000.0 MiB | `1e7fc7167675fa33dd5d75341665a3106374e85c21b3ef78d5caf7fde36d85d5` |
| `RG477M_GammaOS_Next_v1.2.1.7z.008` | 29810228 | 28.4 MiB | `cf2bf6dea167b8faee85df6c17ece8059499fda8e05c462552136b726fc5d44e` |
| `SP_Flash_Tool_V6.2404_Win.zip` | 21722734 | 20.7 MiB | `ba6746932c8e3553b363f7bd0ae43882cd44dfd8ee814ae81052c98e572076e2` |
| `thermal_safety_protection.zip` | 12977 | 0.0 MiB | `5cb2a90dfd677e10aa254910c8f6451729eb3da49685c084496782954318352b` |
| `thermal_safety_protection_2.zip` | 15623 | 0.0 MiB | `bf184bc6c179ff3e6f75a2c7d6f0983323471279e5b6249bd384886d65298eaf` |

### Anbernic RG477V (`anbernic-rg477v`)

| Field | Value |
|---|---|
| Catalog id | `anbernic-rg477v` |
| Name | Anbernic RG477V |
| SoC | MediaTek Dimensity 8300 |
| GammaOS release | [`v.1.2.1-ANBERNICRG477V`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.1-ANBERNICRG477V) (GammaOS Next - Anbernic RG477V- v1.2.1, published 2025-12-23) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | MediaTek SP Flash Tool V6 (Windows only, bundled as `SP_Flash_Tool_V6.2404_Win.zip`): Format UFS-LUA0 and UFS-LUA1, Download with `flash_128GB.xml` or `flash_256GB.xml` matching the unit's storage, then re-flash preloaders (Write Memory). Wrong XML can brick |
| Password protected | Yes. Password published in the release notes: `c11f181d9938a58fc273d9ec7e92e3407dec3bc31c76923e3a829ea7df624017` (same password as the RG557 and RG477M releases) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `RG477V_GammaOS_Next_v1.2.1.7z.001` | 1048576000 | 1000.0 MiB | `54f817bb5a7734d07de327e2e730d7a527a23b5fc2f8a9cc5d27414f8f9a703a` |
| `RG477V_GammaOS_Next_v1.2.1.7z.002` | 1048576000 | 1000.0 MiB | `6767dda1cbeea4664bf0349cdc31f18eb8c5cbc7fbdecb4d4e515c718bdf6c6f` |
| `RG477V_GammaOS_Next_v1.2.1.7z.003` | 1048576000 | 1000.0 MiB | `3b1bc6ed27e39d39a8792428350910503f041375279ee27204e0a830fe959af6` |
| `RG477V_GammaOS_Next_v1.2.1.7z.004` | 1048576000 | 1000.0 MiB | `2a14135b02514d603d1cc1acf6978f2668964b52e2e712e3be1b7716b132ebee` |
| `RG477V_GammaOS_Next_v1.2.1.7z.005` | 655840435 | 625.5 MiB | `77756e95c411509fe7b1c9e102d6b1d40e26fd5cc221af52013400121588f8cd` |
| `SP_Flash_Tool_V6.2404_Win.zip` | 21722734 | 20.7 MiB | `ba6746932c8e3553b363f7bd0ae43882cd44dfd8ee814ae81052c98e572076e2` |
| `thermal_safety_protection.zip` | 12977 | 0.0 MiB | `5cb2a90dfd677e10aa254910c8f6451729eb3da49685c084496782954318352b` |
| `thermal_safety_protection_2.zip` | 15623 | 0.0 MiB | `bf184bc6c179ff3e6f75a2c7d6f0983323471279e5b6249bd384886d65298eaf` |

### KTPocket KT-R1 (`ktpocket-kt-r1`)

| Field | Value |
|---|---|
| Catalog id | `ktpocket-kt-r1` |
| Name | KTPocket KT-R1 |
| SoC | MediaTek Helio G99 |
| GammaOS release | [`v.1.0.0-KTR1`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.0.0-KTR1) (GammaOS Next - KTPocket KT-R1 - v1, published 2025-06-29) |
| Android base | Android 13 / LineageOS 20 (API 33), per release notes |
| Install method (upstream) | fastboot: `adb reboot bootloader`, `fastboot flashing unlock` (Volume Up to confirm), then `FlashPartitions.bat`/`.sh` and `EraseUserData.bat`/`.sh`; wiki gives Windows, macOS and Linux commands |
| Password protected | Yes. Password published in the release notes: `1C1DXWfUV3UUWREweEzyc6KjmNBvn0XM9j3H5CPh230ycTf41bhghllyCNKbc3Ev` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `KT-R1_GammaOSNext_Full_v1.7z.001` | 1992294400 | 1.86 GiB | `1d207e6ee85f43ca7b43208508382e6774b51c05e2f239bda598a30024735234` |
| `KT-R1_GammaOSNext_Full_v1.7z.002` | 222029909 | 211.7 MiB | `9b8202f311987588b5bfcb8dafb4a83990c16e8b96e195e8dc0ab144a684d878` |
| `KT-R1_GammaOSNext_Lite_v1.7z.001` | 1992294400 | 1.86 GiB | `515c564c7ea707258f9c8ca42ea7e7790bf1f7acadb8e3f23a4d5164f7c930e0` |
| `KT-R1_GammaOSNext_Lite_v1.7z.002` | 66558094 | 63.5 MiB | `d52dbc4e72c76bacea307ce2e527ac9355cb9efffbfc34654aee6d2018c27d43` |

### ZPG A1 Unicorn (`zpg-a1-unicorn`)

| Field | Value |
|---|---|
| Catalog id | `zpg-a1-unicorn` |
| Name | ZPG A1 Unicorn |
| SoC | MediaTek Helio G99 |
| GammaOS release | [`v.1.0.0-ZPGA1UNICORN`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.0.0-ZPGA1UNICORN) (GammaOS Next - ZPG A1 Unicorn - v1, published 2025-06-29) |
| Android base | Android 13 / LineageOS 20 (API 33), per release notes |
| Install method (upstream) | fastboot: `adb reboot bootloader`, `fastboot flashing unlock` (Volume Up to confirm), then `FlashPartitions.bat`/`.sh` and `EraseUserData.bat`/`.sh`; wiki gives Windows, macOS and Linux commands |
| Password protected | Yes. Password published in the release notes: `I1Lt4eD1TQ61YjiMhNgnDTCnSO055MhiuEB2X9MnhQW4s5nq2S` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `ZPG_A1_UNICORN_GammaOSNext_Full_v1.7z.001` | 1992294400 | 1.86 GiB | `bd7e74ff984e63d987b4fc962a8506340f2e489d3f2dc3a1b009bcf688134977` |
| `ZPG_A1_UNICORN_GammaOSNext_Full_v1.7z.002` | 187754550 | 179.1 MiB | `1ac2650e1d2138a052242c1995bc0e3307e31f46a60e13bed0fa0a3144da6b3d` |
| `ZPG_A1_UNICORN_GammaOSNext_Lite_v1.7z.001` | 1992294400 | 1.86 GiB | `9fee9aadd662192c55ca0f5a6e072c162020735b76b0fafd182473548c10fef4` |
| `ZPG_A1_UNICORN_GammaOSNext_Lite_v1.7z.002` | 515877 | 0.5 MiB | `6b726456d86c323928424a19ee6f4b6f55b22be4bbe864cf4c223fd0cab2b753` |

### AYANEO Pocket Micro (`ayaneo-pocket-micro`)

| Field | Value |
|---|---|
| Catalog id | `ayaneo-pocket-micro` |
| Name | AYANEO Pocket Micro |
| SoC | MediaTek Helio G99 |
| GammaOS release | [`v.1.0.0-AYANEOPOCKETMICRO`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.0.0-AYANEOPOCKETMICRO) (GammaOS Next - AYANEO Pocket Micro - v1, published 2025-07-15) |
| Android base | Android 13 / LineageOS 20 (API 33), per release notes |
| Install method (upstream) | fastboot: `adb reboot bootloader`, `fastboot flashing unlock` (Volume Up to confirm), then `FlashPartitions.bat`/`.sh` and `EraseUserData.bat`/`.sh`; wiki gives only the Windows `.bat` route |
| Password protected | Yes. Password published in the release notes: `hsVyphvQdyNGG5BNoNcrO9s4DTqiquBFV7U47Ikw63uG4kUbt3` |
| Optional asset | `experimental-ktr1-malidriver-r38p1.zip` is an optional Magisk GPU-driver module, not part of the install |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `AYANEO_Pocket_Micro_GammaOSNext_Full_v1.7z.001` | 2080491339 | 1.94 GiB | `4a5bcb7ef169d548a5307ef96b3b92fbc8013d2c28c6dbc4005fcb2e7e42234b` |
| `AYANEO_Pocket_Micro_GammaOSNext_Lite_v1.7z.001` | 1891589163 | 1.76 GiB | `98e0a0feea9f94c1260b10bc74b91a0f84b6f079b35b69ca1e871363b5afceb0` |
| `experimental-ktr1-malidriver-r38p1.zip` | 66529187 | 63.4 MiB | `b34e32253ff72b0ad72dcdac83f301ddae42d489ce5f32ce382f9cd214a88458` |

### Retroid Pocket 4 Pro (`retroid-pocket-4-pro`)

| Field | Value |
|---|---|
| Catalog id | `retroid-pocket-4-pro` |
| Name | Retroid Pocket 4 Pro |
| SoC | MediaTek Dimensity 1100 |
| GammaOS release | [`v.1.0.0-RETROIDPOCKET4PRO`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.0.0-RETROIDPOCKET4PRO) (GammaOS Next - Retroid Pocket 4 Pro - v1, published 2025-07-14) |
| Android base | Android 13 / LineageOS 20 (API 33), per release notes |
| Install method (upstream) | fastboot: `adb reboot bootloader`, `fastboot flashing unlock` (Volume Up to confirm), then `FlashPartitions.bat`/`.sh` and `EraseUserData.bat`/`.sh`; wiki gives only the Windows `.bat` route |
| Password protected | Yes. Password published in the release notes: `lXoX99MyLtybfphr82PsmV701R4ONHV7qqwIo8l2Ec8VSQyyFQ` |
| Note | The Lite build is a single `.7z.001` |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `Retroid_Pocket_4_PRO_GammaOSNext_Full_v1.7z.001` | 2096103424 | 1.95 GiB | `fb7c8eac2a807344a94aacb0d5164149d84c4dbf2d80c56a0d1422755ec50de9` |
| `Retroid_Pocket_4_PRO_GammaOSNext_Full_v1.7z.002` | 19940348 | 19.0 MiB | `27a88ff682132429ef1cd5945040c9dfebf3d0c2eacb1361590e525e58b68e07` |
| `Retroid_Pocket_4_PRO_GammaOSNext_Lite_v1.7z.001` | 1931491577 | 1.80 GiB | `a75e21d1791e6f1b8e9007ea9a6b58310905dbebf776d21c2ed9a02fb7dc9d21` |

### Retroid Pocket Classic (`retroid-pocket-classic`)

| Field | Value |
|---|---|
| Catalog id | `retroid-pocket-classic` |
| Name | Retroid Pocket Classic |
| SoC | Qualcomm Snapdragon G1 Gen 2 |
| GammaOS release | [`v.1.0.0-RETROIDPOCKETCLASSIC`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.0.0-RETROIDPOCKETCLASSIC) (GammaOS Next - Retroid Pocket Classic - v1, published 2025-07-14) |
| Android base | Android 13 / LineageOS 20 (API 33), per release notes |
| Install method (upstream) | fastboot: `FlashPartitions.bat` then `EraseUserData.bat` (Windows only in the wiki). The wiki has no bootloader-unlock step for this device (USB debugging and OEM unlocking enabled only); whether the script unlocks is unconfirmed |
| Password protected | Yes. Password published in the release notes: `WOZzBUCDGvgHB1ODUDVBHZb6nz6OWHsX7TGta7U9c7Wt3F8Nb5` |
| Block | Units made in March 2026 or later boot-loop on GammaOS (silent hardware revision); don't flash them |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `Retroid_Pocket_Classic_GammaOSNext_Full_v1.7z.001` | 2072520160 | 1.93 GiB | `813bf07ff83de2cd489c49869b36db5853fd4fbee2915a953530456a0b026ada` |
| `Retroid_Pocket_Classic_GammaOSNext_Lite_v1.7z.001` | 1898099665 | 1.77 GiB | `f8db1d4901f6edb0e3ca1e501a73f7ee96fd5c0e85e19ef7494b6ce71592f299` |

### AYANEO Pocket Air Mini (`ayaneo-pocket-air-mini`)

| Field | Value |
|---|---|
| Catalog id | `ayaneo-pocket-air-mini` |
| Name | AYANEO Pocket Air Mini |
| SoC | MediaTek Helio G90T (MT6785) |
| GammaOS release | [`v1.3.1-AYANEOPOCKETAIRMINI`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.3.1-AYANEOPOCKETAIRMINI) (GammaOS Next - AYANEO Pocket Air Mini - v1.3.2, published 2026-05-01) |
| Android base | Android 14 / LineageOS 21 (API 34): inferred (v1.3.x line, and the v1.3.2 OTA is shared with the Android 14 devices); the release notes don't state the version |
| Install method (upstream) | MediaTek SP Flash Tool v5.1924 (Windows, in `tools.zip`) with `MT6785_Android_scatter.txt`, "Format All + Download" for a fresh install, "Download Only" to upgrade and keep data. One archive holds both Full and Lite folders |
| Password protected | Yes. Password posted in the release notes (UPDATE line): `d789e6c2474292f3adb66fc2698a1758422f51bf993f60a577365592cbb5a06b1f056f2d13d7c3cc2c9707053e9ba779e5b14bd4a3cead23e368b839e2c3453a` |
| Note | The tag says v1.3.1 but the release name and files are v1.3.2. `lk-verified-reduced-ghosting-attempt.img` is an optional experimental `lk_a` image, not part of the install |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `AYANEO_Pocket_Air_Mini_GammaOS_Next_v1.3.2.7z.001` | 2086666240 | 1.94 GiB | `bb8bbe23ca42b6ae3e004202f7f671002dc9be53eadc8f658fca32ee943c36f8` |
| `AYANEO_Pocket_Air_Mini_GammaOS_Next_v1.3.2.7z.002` | 1851282633 | 1.72 GiB | `89677676b05f5289e095af77ea1a0edeeb1f900dfdad170dbfe05bd4d8de9195` |
| `lk-verified-reduced-ghosting-attempt.img` | 2097152 | 2.0 MiB | `a35883eaa10d4dfb34807f153814a02f7e8df595943424c381b6512ec2f536c8` |
| `tools.zip` | 91124570 | 86.9 MiB | `9b7b3654c283bf26f1bd13dc69de05bca9f0c9ef314f25756fa5c9f813be990c` |

### GameMT E6 MAX (`gamemt-e6-max`)

| Field | Value |
|---|---|
| Catalog id | `gamemt-e6-max` |
| Name | GameMT E6 MAX |
| SoC | Allwinner A527 |
| GammaOS release | [`v.1.1.0-GAMEMTE6MAX`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.1.0-GAMEMTE6MAX) (GammaOS Next - GameMT E6 Max (all models) - v1.1, published 2025-12-01) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | `fastboot oem unlock` first (skipping it bricks the device), then PhoenixSuit (Windows) with the GammaOS `.img`, Wipe data checked. Lite recommended (the Full build is shipped as `NOT_RECOMMENDED_...`). Covers Bluetooth and non-Bluetooth models |
| Password protected | Yes. Password published in the release notes: `dcea9ce317efd2daa366c28a1000a2ea96b36b96d11e44526105b00b483a9b03` |
| Unbrick image | `a527_android13_pro2_card0_secure_E6_V2.1P.img` on the v1.0.0 release (`v.1.0.0-GAMEMTE6MAX`) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.001` | 524288000 | 500.0 MiB | `04c536eca0ed9d665d4072feb38778ea580af0e552aa2ff419354130f659e4ad` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.002` | 524288000 | 500.0 MiB | `be207cb4c4f725cde47995ef70445e8bfa7c23ae37da71b42cff562c7593c7fd` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.003` | 524288000 | 500.0 MiB | `a6cc4fe3f80a02266328a63d6cb4d19c880128af44ae6f493c73ab01981c5633` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.004` | 524288000 | 500.0 MiB | `1ce5cb48f52422158f340c0fc1fddcf266b0c560bdab500a85cce39323ada805` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.005` | 524288000 | 500.0 MiB | `f67c46a2cffb5b5a37ac7c2c2491e5f1cc616a53c295bdaaf1752923143e06fe` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.006` | 524288000 | 500.0 MiB | `1820201556d21639af57e93a2dc98fab0baa35666798ce65339e968ee742a91d` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.007` | 524288000 | 500.0 MiB | `c40667ed1b07164c221763ddc6f716b92147491e3592bad44879983e1f96e0d6` |
| `GameMT_E6Max_GammaOSNext_Lite_v1.1.7z.008` | 375445919 | 358.1 MiB | `09cd8540119b708040eed180316cfff1f3eaef6f08a699ea171afc4917c44fdc` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.001` | 524288000 | 500.0 MiB | `54c374ca0fbe3fc4573d12ac2c57fc6eac29612bf413cffa5ab28ea3891e71dd` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.002` | 524288000 | 500.0 MiB | `ed42e81c0e562aa9abe3436aea4256bcff8f352861aa2da8af5f154465849463` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.003` | 524288000 | 500.0 MiB | `360b6274278ba703fd529fb752d8a563fdecdd4ecb188442696d74210b280d1e` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.004` | 524288000 | 500.0 MiB | `89af46fe5623774d11d6b573c4e39b5aea459f6496cc8e48c673f359401cf6b2` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.005` | 524288000 | 500.0 MiB | `eab1ac8effafd9e4524017329ead716160691b3c0c75e412a65a8d505a4e69b4` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.006` | 524288000 | 500.0 MiB | `6ae788a89be5a63778ace7bec4acf4e544b14e1fc180aef9319a53e1105c9ca0` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.007` | 524288000 | 500.0 MiB | `6f17e6fd2f165d561fc2e3d2cb1a9a851997bede59dbed910ee8d9657ccb4120` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.008` | 524288000 | 500.0 MiB | `cdb7737542bda6be26fb30953bbad865f0091c68c303e98ff3ea74deb4aba95e` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.009` | 524288000 | 500.0 MiB | `7fffce0a91e5a3aa35dfbe2dae3c97a992d4420f5e0184e9c4e23301859db77e` |
| `NOT_RECOMMENDED_GameMT_E6Max_GammaOSNext_Full_v1.1.7z.010` | 115829200 | 110.5 MiB | `d27ae7d9492a5377a92a1b0f0ac5a601fedb565356159acc7c4e0bac24a5e8e0` |

### MANGMI Air X (`mangmi-air-x`)

| Field | Value |
|---|---|
| Catalog id | `mangmi-air-x` |
| Name | MANGMI Air X |
| SoC | Qualcomm Snapdragon 662 |
| GammaOS release | [`v.1.2.0-MANGMIAIRX`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v.1.2.0-MANGMIAIRX) (GammaOS Next - MANGMI Air X - v1.2, published 2025-12-14) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes |
| Install method (upstream) | Qualcomm EDL with QFIL (Windows): hold Volume Up + Volume Down and plug in USB (QDLoader 9008), programmer `prog_firehose_ddr.elf`, `rawprogram_unsparse0.xml` + `patch0.xml` for a clean install (`rawprogram_unsparse0_upgrade.xml` keeps data), device type eMMC. QPST and the Qualcomm USB driver are on the v1.1.1 release (`v.1.1.1-MANGMIAIRX`) |
| Password protected | Yes. Password published in the release notes: `0d101b9206e1d15e8614b2f915b981b849d534d056ab0dc28398e1fa7f24b32b` |
| Hardware revisions | Pick the package by the first four characters of the serial number: MQ65 or MQ66. Each package holds both Full and Lite |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.001` | 524288000 | 500.0 MiB | `f8e017f4fbf133ddfae61db98003c0a4623596d37347813bd20d4e40d378030e` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.002` | 524288000 | 500.0 MiB | `ac245b84b93fbb42c1cc9fb9c206b1936f6df7e428335658e612ee6f17576846` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.003` | 524288000 | 500.0 MiB | `2ed2f2eee8769d262104487664d2b309613d73ed0fc9b3bd8c183843163e29e2` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.004` | 524288000 | 500.0 MiB | `628aa69d029d57bfe3116492d005398ddd9341d22c52de6909482b75848d6004` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.005` | 524288000 | 500.0 MiB | `98f66685284d9d9865901685f6c17582a6d4b8828e81114bc53daa112d3dabe6` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.006` | 524288000 | 500.0 MiB | `36a2c37a9e9172fd326d5c0479e5c8ccc8d2f0ce2812e8463df52409b01445ef` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.007` | 524288000 | 500.0 MiB | `13d7a94cdd16b6477a77c17c3b8b835ddd2f54dd3a72e5b16c601bc5bf7bbc97` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.008` | 524288000 | 500.0 MiB | `f82f15515b9a1365f04e27fcdc61f1f3b37d09e62b17993eb9feab3b455a4935` |
| `MQ65_MANGMI_AirX_GammaOSNext_v1.2.7z.009` | 93075173 | 88.8 MiB | `e9844c0e71f8c33acd550025aa50aa51e32ad36b2574480208dea65f8a2295fc` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.001` | 524288000 | 500.0 MiB | `0b9532cc825c978ebfc22e5fd88d7152135819a04692cc026e2e14ff93b5ac7f` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.002` | 524288000 | 500.0 MiB | `65f4c9e78ba853480212b72e6a5d29c1e43b081ced62002e051132b514752e42` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.003` | 524288000 | 500.0 MiB | `9ed35bf48daaf277270df78e238f0f54d6010fcfe6bff905b8376a4528e91deb` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.004` | 524288000 | 500.0 MiB | `e5692d76b61c2b45dd84a6bdd4cd6de73135e5d87044dc2fa9777b898aa15396` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.005` | 524288000 | 500.0 MiB | `5423b46da84713390897df725760fc8ff5f2bab1dbf851dd5e999a38a2ee8e17` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.006` | 524288000 | 500.0 MiB | `670bf514b847646bd8917fbc2c87461fa970019f6f1b1e18a7f7590c7a8baf0b` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.007` | 524288000 | 500.0 MiB | `b9f3f98c06385477958a027c01afa892c0dfee69f754bd18bc207bb70a9e65d1` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.008` | 524288000 | 500.0 MiB | `113b3bcb2d7eba5bb1254ce0d6cf0ad64720cd7e985c405be5572f8a15a22cce` |
| `MQ66_MANGMI_AirX_GammaOSNext_v1.2.7z.009` | 94962497 | 90.6 MiB | `9b48ed3b645c602bb15497ce8149ba545b3382f459d16bd373e74bd9f617c121` |

### TrimUI Brick (`trimui-brick`)

| Field | Value |
|---|---|
| Catalog id | `trimui-brick` |
| Name | TrimUI Brick |
| SoC | Allwinner A133P |
| GammaOS release | [`v1.4.1-TRIMUIBRICK`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-TRIMUIBRICK) (GammaOS Nano v1.4.1 (TrimUI Brick) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with PhoenixCard (Windows, `PhoenixCard.zip` on the release) using the Allwinner SD-card wiki page; no Linux/macOS writer is named upstream |
| Password protected | Yes. Password published in the release notes (public release): `TMAXLm1tAqCt6XA6K03zLieqNHRa1cA0Epcj5MAJeOJZhn0niO` (shared by all four A133P v1.4.1 releases) |
| Edition | Core only (GammaOS Nano on GammaOS Core, no Google) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `PhoenixCard.zip` | 16947316 | 16.2 MiB | `5e065e09755ee04c647e4dbf7d71fb44d5bad23937c4025dd3d8bf40b7383061` |
| `TrimUI_Brick_GammaOS_Core_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `7af5d8920ac4cdcbc8fac6c6629a20277302a7febb19c8107b2f46e39137c304` |
| `TrimUI_Brick_GammaOS_Core_v1.4.1.7z.002` | 1895180732 | 1.77 GiB | `d5adfbef5f48404dbd48e950f3798da383d1508078197859f2304c5f7368be2a` |

### MagicX Mini Zero 28 (`magicx-mini-zero-28`)

| Field | Value |
|---|---|
| Catalog id | `magicx-mini-zero-28` |
| Name | MagicX Mini Zero 28 |
| SoC | Allwinner A133P |
| GammaOS release | [`v1.4.1-MAGICXMINIZERO28`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-MAGICXMINIZERO28) (GammaOS Nano v1.4.1 (MagicX Mini Zero 28) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with PhoenixCard (Windows, `PhoenixCard.zip` on the release) using the Allwinner SD-card wiki page; no Linux/macOS writer is named upstream |
| Password protected | Yes. Password published in the release notes (public release): `TMAXLm1tAqCt6XA6K03zLieqNHRa1cA0Epcj5MAJeOJZhn0niO` (shared by all four A133P v1.4.1 releases) |
| Edition | Core only (GammaOS Nano on GammaOS Core, no Google) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `MagicX_Mini_Zero28_GammaOS_Core_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `ceb4a3c5c184e5771a3d6a32bfaca58d747707e88de479c9afd1c796e2db42e0` |
| `MagicX_Mini_Zero28_GammaOS_Core_v1.4.1.7z.002` | 1863572108 | 1.74 GiB | `7fbc79d1c9844a884319ea9e75535d7102da4edede290323c67929d27600d373` |
| `PhoenixCard.zip` | 16947316 | 16.2 MiB | `5e065e09755ee04c647e4dbf7d71fb44d5bad23937c4025dd3d8bf40b7383061` |

### MagicX XU20 V32 (`magicx-xu20-v32`)

| Field | Value |
|---|---|
| Catalog id | `magicx-xu20-v32` |
| Name | MagicX XU20 V32 |
| SoC | Allwinner A133P |
| GammaOS release | [`v1.4.1-MAGICXXURETROX20V32`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-MAGICXXURETROX20V32) (GammaOS Nano v1.4.1 (MagicX XURetro X20 V32) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with PhoenixCard (Windows, `PhoenixCard.zip` on the release) using the Allwinner SD-card wiki page; no Linux/macOS writer is named upstream |
| Password protected | Yes. Password published in the release notes (public release): `TMAXLm1tAqCt6XA6K03zLieqNHRa1cA0Epcj5MAJeOJZhn0niO` (shared by all four A133P v1.4.1 releases) |
| Edition | Core only (GammaOS Nano on GammaOS Core, no Google) |
| Name | GammaOS calls it "MagicX XURetro X20 V32"; the asset and retail name is XU20 V32 (sold as XU Retro XU20 V32) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `MagicX_XURetro_XU20_V32_GammaOS_Core_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `10ffc7be15f8c55dbebe78b671f51955bfd46bb07d781d50fbe5556faadd6621` |
| `MagicX_XURetro_XU20_V32_GammaOS_Core_v1.4.1.7z.002` | 1882788300 | 1.75 GiB | `382a67bbeb59692205d1b1da172d3a653cdb2a435a29b0425a29bd05a5c9dbde` |
| `PhoenixCard.zip` | 16947316 | 16.2 MiB | `5e065e09755ee04c647e4dbf7d71fb44d5bad23937c4025dd3d8bf40b7383061` |

### MagicX Zero 40 (`magicx-zero-40`)

| Field | Value |
|---|---|
| Catalog id | `magicx-zero-40` |
| Name | MagicX Zero 40 |
| SoC | Allwinner A133P |
| GammaOS release | [`v1.4.1-MAGICXZERO40`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-MAGICXZERO40) (GammaOS Nano v1.4.1 (MagicX Zero 40) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with PhoenixCard (Windows, `PhoenixCard.zip` on the release) using the Allwinner SD-card wiki page; no Linux/macOS writer is named upstream |
| Password protected | Yes. Password published in the release notes (public release): `TMAXLm1tAqCt6XA6K03zLieqNHRa1cA0Epcj5MAJeOJZhn0niO` (shared by all four A133P v1.4.1 releases) |
| Edition | Core only (GammaOS Nano on GammaOS Core, no Google) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `MagicX_Zero40_GammaOS_Core_v1.4.1.7z.001` | 2096103424 | 1.95 GiB | `a3b66be37d51a8d0f563355a32a7ffa47e5a5163b04674928cd079fffd79563f` |
| `MagicX_Zero40_GammaOS_Core_v1.4.1.7z.002` | 1894318972 | 1.76 GiB | `1c608d300e109b7c426567eb02fb060c7ab9430357e8adc5b25fffe796522378` |
| `PhoenixCard.zip` | 16947316 | 16.2 MiB | `5e065e09755ee04c647e4dbf7d71fb44d5bad23937c4025dd3d8bf40b7383061` |

### GKD 350H Ultra (`gkd-350h-ultra`)

| Field | Value |
|---|---|
| Catalog id | `gkd-350h-ultra` |
| Name | GKD 350H Ultra |
| SoC | Rockchip RK3576S |
| GammaOS release | [`v1.4.1-GKD350HULTRA`](https://github.com/TheGammaSqueeze/GammaOSNext/releases/tag/v1.4.1-GKD350HULTRA) (GammaOS Nano v1.4.1 (GKD 350H Ultra) Release Notes, published 2026-08-13) |
| Android base | Android 14 / LineageOS 21 (API 34), per release notes (GammaOS Nano on GammaOS Core) |
| Install method (upstream) | Runs from microSD only (internal storage untouched). Card written with TheGammaSqueeze/rockchip_sd_tool (Windows, Linux, macOS; recommended in the wiki) or `SDDiskTool_v1.69_SDBootOnly.zip` (Windows, SD Boot mode). dd, Win32DiskImager and balenaEtcher do not work |
| Password protected | Yes. Password published in the release notes (public release): `YATpU5ulYNx401dbOeYW1m1CoRsffR2MIoIWVe8r4Bn1vfXCZz` |
| Note | Single archive part (`.7z.001` only) |

| Asset | Size (bytes) | Size | sha256 |
|---|---:|---:|---|
| `GKD_350H_Ultra_GammaOS_Nano_v1.4.1.7z.001` | 1977804332 | 1.84 GiB | `f73b38e3c8faf0ec88ed3899e9609a210ec5eaa57fe69fb6677aaf23922a7107` |
| `SDDiskTool_v1.69_SDBootOnly.zip` | 480092 | 0.5 MiB | `a50cda8e26d86bd5dc901bd8ca55695c4a0678c0b3bc55893cf6bc2711726a3f` |

## Skipped and why

- **Retroid Pocket Duo Lite.** In the wiki only, as a GammaOS Next 1.5.0 *private beta* handed out to testers. Not a 1.x public release and no GitHub release exists.
- **`v.1.1.1-ANBERNICRGDS_PREVIEW`.** A content-creator preview build that the notes say "will never be made available to the public". Not a separate device: the RG DS entry uses the Nano release.
- **`v1.3.2-OTA`.** Universal OTA packages for devices already on 1.3.x, marked "NOT live yet". Not a device and not a fresh-install image.
- **README "PLANNED" devices** (AYANEO Pocket-S/DMG, Pocket AIR, Retroid Pocket Mini v1/v2, Flip 2, Pocket 5, Powkiddy X28). No release.
- **TrimUI Smart Pro.** Named in the wiki's Allwinner SD-card page title but has no GammaOS release.
- **Devices that aren't really Android install targets.** None: every listed release runs Android 14 (or 13). The Nano SD-card devices (RG DS Plus, TrimUI Brick, MagicX, GKD 350H Ultra) boot GammaOS Core Android from the card and leave internal storage untouched. The RG DS Plus ships with Linux, and Android exists only on the GammaOS card. These are still listed, as microSD-only installs.

Other releases of listed devices, not used as the base: `v.1.0.0-T820` (Android 13, RG556 / RG Cube / RG406V / RG406H, password `ngfbs78b45tygh2jhsFGBFdbgdfgw3w434t42`), `v.1.0.0-GAMEMTE6MAX` (Android 13, non-Bluetooth model), `v.1.1.1-MANGMIAIRX` (single hardware revision; hosts QPST and the Qualcomm USB driver), `v1.2.2-ANBERNICRGDS` (non-Nano Android 14, SD card or fastboot), `v1.4.0-*` Nano releases (superseded by v1.4.1; no in-place upgrade from v1.4.0 on the A133P devices), and `v1.4.3-ANBERNICRGDSPLUS` / `v1.4.4-ANBERNICRGDS` (password not public).

## Unconfirmed

- **RG Vita SoC:** T618 (Anbernic spec sheet, and the same Unisoc tool bundle as the T618 RG Rotate) against "T820 platform" in the GammaOS release notes.
- **T618 prerequisite:** the release notes say the install "must be installed fresh from the original GammaOS v1.5.1". The wiki says the fresh install replaces stock, v1.5.1 or any ROM. (`universal_installer.md` 2.2 lists "must already be on GammaOS v1.5.1" as a hard block; the wiki doesn't back that.)
- **Retroid Pocket Classic unlock:** the wiki has no unlock step. Whether the flash script unlocks, or the device needs no unlock, is not stated.
- **Retroid Pocket 4 Pro `fastboot_product: "RP4Pro"`** (already in the catalog): no source found for this value. Not changed.
- **Pocket Air Mini:** Android version not stated in its release. RAM (2 GB minimum) and display come from a third-party comparison (retrohandhelds.gg).
- **RG Vita Pro password:** only the Lite overclocked archive was checked (unencrypted). The other three archives weren't.
- **Third-party hardware specs:** KT-R1 display 1620×1080 (notebookcheck), MagicX Zero 40 display 480×800 portrait panel (retroshelf), RAM for the Mini Zero 28 / XU20 V32 / Zero 40 (2 GB, retrohandhelds/retroshelf), Snapdragon G1 Gen 2 GPU "Adreno A12" and Cortex-A78 cores (notebookcheck, pocket-gaming). GKD 350H Ultra: the retrohandhelds review gives RK3576S, 2 GB and 1600×1440.
- **Vulkan on the A133P builds:** not checked.
- **RG DS Plus:** no public password for any build, so there's nothing that can be built from public files yet.
- **`universal_installer.md` 2.2 / 8.1:** these list the RG Vita and RG Rotate as "Unisoc (other)". They're T618 (`ums512`), flashed with UpgradeDownload and with no unlock step in the wiki. The RG DS is listed there as a card-tool device. That's true from v1.4.4, but v1.4.1 and older are raw self-flashing images.

## Sources

- GitHub releases API: `gh api --paginate repos/TheGammaSqueeze/GammaOSNext/releases` (release bodies, asset names, sizes and `digest`s)
- https://github.com/TheGammaSqueeze/GammaOSNext/releases (one page per tag above)
- https://github.com/TheGammaSqueeze/GammaOSNext (README supported-devices table)
- https://github.com/TheGammaSqueeze/GammaOSNext/wiki/GammaOS-Next-Installation
- https://github.com/TheGammaSqueeze/GammaOSNext/wiki/GammaOS-Installation-Guide-for-Anbernic-RG557,-RG477M,-and-RG477V-(SP-Flash-Tool,-Full-and-Lite-Builds)
- https://github.com/TheGammaSqueeze/GammaOSNext/wiki/GammaOS-Next-Installation-for-Allwinner-SD-Card-Devices
- https://github.com/TheGammaSqueeze/GammaOSNext/wiki/GammaOS-Next-Changelog (RG DS is rk3568; Pocket Air Mini is MT6785; RK3576 core layout)
- RG Vita Pro archive header: HTTP range reads of `RG_VITA_PRO_GammaOS_Lite_v1.3.0_SDCARDINSTALL.7z.001` (start header) and `.7z.002` (end header)
- https://anbernic.com/products/rg-rotate (T618, 3 GB, 3.5" 720×720)
- https://anbernic.com/blogs/news/rg-ds-plus-vs-rg-ds (RK3568; RG DS 3 GB, 2× 640×480; RG DS Plus 1 GB, 2× 1024×768)
- https://retrohandhelds.gg/anbernic-rg-vita-and-rg-vita-pro-price-and-release-date-revealed/ and https://retrohandhelds.gg/anbernic-rg-vita-review/ and https://liliputing.com/anbernic-rg-vita-is-an-android-handheld-game-console-with-a-familiar-design/ (RG Vita T618 / 3 GB / 1280×720; Vita Pro RK3576 / 4 GB / 1920×1080)
- https://electroarcade.com.au/anbernic-t820-rg556-rg406h-rg406v-specs-games-list and https://droix.net/product/anbernic-rg-slide/ (T820 models: 8 GB, displays)
- https://handheldwiki.com/rg-477m/ and https://retrohandhelds.gg/anbernic-rg477v-first-impressions/ (Dimensity 8300, 8/12 GB, 1280×960)
- https://alexnld.com/product/anbernic-rg557-handheld-game-console-5-48-inch-amoled-screen-android-14-video-game-player-12128g-whi (RG557 12 GB minimum)
- https://retrooverload.com/anbernic-rg405m-review/, https://droix.co.uk/product/anbernic-rg405v/, https://pocket-gaming.org/device/anbernic-anbernic-rg-505/ (T618 models)
- https://www.notebookcheck.net/KT-Pocket-finally-starts-selling-KT-R1-Android-gaming-handheld-from-US-169.692354.0.html (KT-R1)
- https://retrohandhelds.gg/zpg-a1-unicorn-review/ (ZPG A1 Unicorn)
- https://droix.net/blogs/ayaneo-pocket-micro-review/ (Pocket Micro)
- https://retrohandhelds.gg/mangmi-air-x-vs-ayaneo-pocket-air-mini/ (MANGMI Air X, Pocket Air Mini)
- https://pocket-gaming.org/device/retroid-retroid-pocket-classic/ and https://www.notebookcheck.net/Snapdragon-G2-Gen-2-and-Snapdragon-G1-Gen-2-unveiled-for-mid-range-handheld-consoles.980733.0.html (Retroid Pocket Classic)
- https://lostinconsoles.com/consoles/gamemt-e6-max-gamemt (E6 MAX)
- https://www.geekyretro.com/handhelds/trimui-brick, https://www.retroshelf.org/devices/magicx-zero-40, https://retrohandhelds.gg/xu-retro-xu20-v32-review/, https://retrohandhelds.gg/magicx-mini-zero-28-review/ (A133P devices)
- https://retrohandhelds.gg/gkd350h-ultra-review/ (GKD 350H Ultra)
