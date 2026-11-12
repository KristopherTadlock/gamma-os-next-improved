"""Fake /system trees for the mods: the 1.2 base (setup.sh lists every step) and the 1.4.4 Nano base
(apps.list and post-install hooks). Only what the offline mods touch is there."""
import io
import os
import subprocess
import tarfile
import zipfile

# The stock PPSSPP and RetroArch binds, set for the buttons' print (Android key codes).
CONTROLS_INI = """[ControlMapping]
Up = 10-19
Down = 10-20
Circle = 10-96
Cross = 10-97
Square = 10-100
Triangle = 10-99
Start = 10-108
"""

SETUP_12 = """#!/system/bin/sh
echo "Installing RetroArch."
pm install /system/etc/RetroArch_aarch64.apk

echo "Installing Aurora Store." && \\
pm install /system/etc/AuroraStore_4.6.2.apk

echo "Installing added apps."
pm install /system/etc/ARMSX2-2.9.2.apk
pm install /system/etc/scummvm-2026.3.0.apk

echo "Granting read/write permissions to RetroArch."
pm grant com.retroarch.aarch64 android.permission.WRITE_EXTERNAL_STORAGE

echo "Granting permissions to applications."
pm grant org.ppsspp.ppsspp android.permission.WRITE_EXTERNAL_STORAGE
echo "Done."
"""

SETUP_NANO = """#!/system/bin/sh
step() { echo "$*"; }
install_apps() {
    case "$cond" in
        minmem:*) ;;
        maxmem:*) ;;
    esac
}
post_retroarch() {
    extract_archive /system/etc/retroarch.tar.zst
}

step "Starting configuration of the Thimsy system..."
settings put secure navigation_mode 0
install_apps
mkdir -p /data/setupcompleted
step "All settings have been applied successfully."
"""

APPS_LIST = """# display name | package | source | condition | post-install hook
RetroArch|com.retroarch.aarch64|/system/etc/RetroArch_aarch64.apk||post_retroarch
PPSSPP|org.ppsspp.ppsspp|/system/etc/ppsspp_1.20.3.apk||post_ppsspp
ARMSX2 PS2 emulator|com.armsx2|/system/etc/ARMSX2-2.9.2.apk||
"""


def write(path, data, mode=0o644):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data.encode() if isinstance(data, str) else data)
    os.chmod(path, mode)


def tar_bytes(files):
    """A tar of FILES {name: data or None for a folder}, owned like an app's payload."""
    out = io.BytesIO()
    with tarfile.open(fileobj=out, mode="w", format=tarfile.GNU_FORMAT) as t:
        for name, data in files.items():
            info = tarfile.TarInfo(name)
            info.uid = info.gid = 10059
            info.uname = info.gname = "u0_a59"
            info.mtime = 1700000000
            if data is None:
                info.type, info.mode = tarfile.DIRTYPE, 0o2770
                t.addfile(info)
            else:
                info.size, info.mode = len(data), 0o660
                t.addfile(info, io.BytesIO(data))
    return out.getvalue()


def payload(path, files):
    """Writes FILES as a payload compressed the way PATH's name says."""
    raw = tar_bytes(files)
    tool = {".zst": ["zstd", "-q", "-c"], ".xz": ["xz", "-c"], ".gz": ["gzip", "-n", "-c"]}[os.path.splitext(path)[1]]
    write(path, subprocess.run(tool, input=raw, capture_output=True, check=True).stdout)


def read_payload(path):
    """{name: data or None} of a payload."""
    tool = {".zst": ["zstd", "-q", "-dc"], ".xz": ["xz", "-dc"], ".gz": ["gzip", "-dc"]}[os.path.splitext(path)[1]]
    raw = subprocess.run(tool + [path], capture_output=True, check=True).stdout
    with tarfile.open(fileobj=io.BytesIO(raw)) as t:
        return {m.name: (t.extractfile(m).read() if m.isfile() else None) for m in t}


def malisx2_zip(path):
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("meta.json", '{"name": "MaliSX2", "version": "0.4.0"}')
        z.writestr("libvulkan_malisx2.so", b"\x7fELF fake driver")
        z.writestr("README.md", "not extracted")


def build(system, nano):
    """Writes a fake /system at SYSTEM: the 1.4.4 (Nano) base when NANO, else the 1.2 base."""
    version = "GammaOS_Next_v1.4.4" if nano else "GammaOS_Next_Full_v1.2"
    write(f"{system}/build.prop", f"""# begin build properties
ro.build.user=someone
ro.build.host=their-machine
ro.lineage.version={version}
ro.lineage.display.version={version}
ro.product.system.model=RG Cube
""")
    write(f"{system}/product/etc/build.prop", "ro.product.product.name=cube\n")
    for overlay in ("IconPackCircularAndroid", "IconShapeSquircle", "FontNotoSerifSource",
                    "DisplayCutoutEmulationTall", "NavigationBarModeGestural", "ThemedIconsOverlay"):
        write(f"{system}/product/overlay/{overlay}/{overlay}.apk", overlay)
    media = f"{system}/product/media" if nano else f"{system}/media"
    write(f"{media}/bootanimation.zip", b"PK stock animation")
    os.symlink("bootanimation.zip", f"{media}/bootanimation-dark.zip")
    write(f"{system}/etc/RetroArch_aarch64.apk", "ra")
    write(f"{system}/etc/ARMSX2-2.9.2.apk", "armsx2")
    malisx2_zip(f"{system}/etc/malisx2-0.4.0.zip")
    ppsspp = {"sdcard/PSP": None, "sdcard/PSP/SYSTEM": None,
              "sdcard/PSP/SYSTEM/controls.ini": CONTROLS_INI.encode(),
              "sdcard/PSP/SYSTEM/ppsspp.ini": b"[General]\n",
              "data/data/org.ppsspp.ppsspp": None}
    payload(f"{system}/etc/ppsspp.tar.zst" if nano else f"{system}/etc/ppsspp.tar.xz", ppsspp)
    if nano:
        write(f"{system}/bin/setup.sh", SETUP_NANO, 0o755)
        write(f"{system}/etc/gammaos/apps.list", APPS_LIST)
        os.makedirs(f"{system}/etc/nano_xmb/audio")
    else:
        write(f"{system}/bin/setup.sh", SETUP_12, 0o755)
    return system
