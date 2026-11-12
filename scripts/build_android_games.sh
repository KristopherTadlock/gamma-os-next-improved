#!/usr/bin/env bash
# Builds the Android Games helper app (tools/android-games) into OUT_APK,
# signed with KEYSTORE, which is created on first use.
#
# Needs javac and keytool (JDK 17), the Android build tools aapt2, d8,
# zipalign and apksigner on PATH, and ANDROID_JAR set to an android.jar for
# API 34 (for example platforms/android-34/android.jar in the Android SDK).
#
# Usage: build_android_games.sh OUT_APK KEYSTORE
set -euo pipefail

SRC="$(cd "$(dirname "$0")/../tools/android-games" && pwd)"
out="$(realpath -m "$1")"
keystore="$(realpath -m "$2")"
: "${ANDROID_JAR:?set ANDROID_JAR to an API 34 android.jar}"
for tool in javac keytool aapt2 d8 zipalign apksigner; do
  command -v "$tool" >/dev/null || { echo "$tool is required" >&2; exit 1; }
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/classes" "$work/dex"
javac -source 11 -target 11 -nowarn -classpath "$ANDROID_JAR" -d "$work/classes" \
  $(find "$SRC/src" -name '*.java')
d8 --release --min-api 25 --lib "$ANDROID_JAR" --output "$work/dex" $(find "$work/classes" -name '*.class')
aapt2 link -o "$work/unsigned.apk" -I "$ANDROID_JAR" --manifest "$SRC/AndroidManifest.xml"
(cd "$work/dex" && zip -q -j "$work/unsigned.apk" classes.dex)
zipalign -f -p 4 "$work/unsigned.apk" "$work/aligned.apk"

if [[ ! -f "$keystore" ]]; then
  mkdir -p "$(dirname "$keystore")"
  keytool -genkeypair -keystore "$keystore" -storepass android -alias androidgames \
    -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Thimsy Android Games" >/dev/null 2>&1
fi
apksigner sign --ks "$keystore" --ks-pass pass:android --out "$out" "$work/aligned.apk"
echo "built $out"
