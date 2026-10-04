#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Build Android App Bundle (.aab) for DevKit
# ==============================================================================

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

APK_PATH="${1:-target/release/apk/devkit.apk}"
OUTPUT_AAB="${2:-target/release/bundle/devkit.aab}"
MANIFEST_PATH="${3:-target/release/apk/AndroidManifest.xml}"
RES_DIR="${PROJECT_ROOT}/res"

echo "=== Building Android App Bundle (.aab) ==="
echo "Input APK:      $APK_PATH"
echo "Output AAB:     $OUTPUT_AAB"
echo "Manifest:       $MANIFEST_PATH"
echo "Resources:      $RES_DIR"

# 1. Validate input APK and manifest
if [ ! -f "$APK_PATH" ]; then
    echo "Error: Release APK not found at '$APK_PATH'. Please build the APK first." >&2
    exit 1
fi

if [ ! -f "$MANIFEST_PATH" ]; then
    echo "Error: Manifest not found at '$MANIFEST_PATH'." >&2
    exit 1
fi

# 2. Locate Android SDK & aapt2
ANDROID_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -z "$ANDROID_ROOT" ] || [ ! -d "$ANDROID_ROOT" ]; then
    if [ -d "$HOME/Android/Sdk" ]; then
        ANDROID_ROOT="$HOME/Android/Sdk"
    else
        echo "Error: ANDROID_HOME or ANDROID_SDK_ROOT must be set." >&2
        exit 1
    fi
fi

# Find aapt2
AAPT2_BIN=""
if command -v aapt2 &>/dev/null; then
    AAPT2_BIN="$(command -v aapt2)"
else
    AAPT2_BIN="$(find "$ANDROID_ROOT/build-tools" -name aapt2 -type f | sort -V | tail -n 1)"
fi

if [ -z "$AAPT2_BIN" ] || [ ! -x "$AAPT2_BIN" ]; then
    echo "Error: aapt2 executable not found in PATH or Android build-tools." >&2
    exit 1
fi
echo "Using aapt2:    $AAPT2_BIN"

# Find android.jar
ANDROID_JAR="$(find "$ANDROID_ROOT/platforms" -name android.jar -type f | sort -V | tail -n 1)"
if [ -z "$ANDROID_JAR" ] || [ ! -f "$ANDROID_JAR" ]; then
    echo "Error: android.jar not found under '$ANDROID_ROOT/platforms'." >&2
    exit 1
fi
echo "Using SDK JAR:  $ANDROID_JAR"

# 3. Locate or download bundletool
BUNDLETOOL_JAR="${BUNDLETOOL_JAR:-$HOME/.bundletool/bundletool.jar}"
if [ ! -f "$BUNDLETOOL_JAR" ]; then
    echo "bundletool not found at '$BUNDLETOOL_JAR'. Downloading latest bundletool..."
    mkdir -p "$(dirname "$BUNDLETOOL_JAR")"
    BUNDLETOOL_VERSION="1.18.3"
    curl -sSL "https://github.com/google/bundletool/releases/download/${BUNDLETOOL_VERSION}/bundletool-all-${BUNDLETOOL_VERSION}.jar" -o "$BUNDLETOOL_JAR"
fi
echo "Using bundletool: $BUNDLETOOL_JAR"

# 4. Check packaging tool (zip or jar)
if ! command -v zip &>/dev/null && ! command -v jar &>/dev/null; then
    echo "Error: neither 'zip' nor 'jar' command was found." >&2
    exit 1
fi

# 5. Build AAB via temporary staging directory
BUILD_TMP="$(mktemp -d)"
trap 'rm -rf "$BUILD_TMP"' EXIT

echo "--> Step 1: Compiling resources..."
"$AAPT2_BIN" compile --dir "$RES_DIR" -o "$BUILD_TMP/compiled_res.zip"

echo "--> Step 2: Linking protobuf resources..."
"$AAPT2_BIN" link \
    --proto-format \
    -o "$BUILD_TMP/proto_res.apk" \
    -I "$ANDROID_JAR" \
    --manifest "$MANIFEST_PATH" \
    --auto-add-overlay \
    "$BUILD_TMP/compiled_res.zip"

echo "--> Step 3: Assembling base bundle module..."
mkdir -p "$BUILD_TMP/base/manifest"
unzip -q "$BUILD_TMP/proto_res.apk" -d "$BUILD_TMP/proto_extracted"
mv "$BUILD_TMP/proto_extracted/AndroidManifest.xml" "$BUILD_TMP/base/manifest/"
mv "$BUILD_TMP/proto_extracted/resources.pb" "$BUILD_TMP/base/"
mv "$BUILD_TMP/proto_extracted/res" "$BUILD_TMP/base/"
rm -rf "$BUILD_TMP/proto_extracted"

# Extract native libraries from the compiled APK
unzip -q "$APK_PATH" "lib/*" -d "$BUILD_TMP/base"

echo "--> Step 4: Packaging module zip..."
if command -v zip &>/dev/null; then
    (cd "$BUILD_TMP/base" && zip -q -r "../base.zip" .)
else
    (cd "$BUILD_TMP/base" && jar -cMf "$BUILD_TMP/base.zip" .)
fi

echo "--> Step 5: Building App Bundle..."
mkdir -p "$(dirname "$OUTPUT_AAB")"
java -jar "$BUNDLETOOL_JAR" build-bundle \
    --modules="$BUILD_TMP/base.zip" \
    --output="$OUTPUT_AAB" \
    --overwrite

echo "--> Step 6: Validating App Bundle..."
java -jar "$BUNDLETOOL_JAR" validate --bundle="$OUTPUT_AAB"

# 6. Optional signing with jarsigner if credentials are provided
KEYSTORE="${KEYSTORE_PATH:-${CARGO_APK_RELEASE_KEYSTORE:-}}"
if [ -n "$KEYSTORE" ] && [ -f "$KEYSTORE" ]; then
    STOREPASS="${KEYSTORE_PASSWORD:-${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}}"
    KEY_ALIAS="${KEY_ALIAS:-${ANDROID_KEY_ALIAS:-}}"
    KEY_PASS="${KEY_PASSWORD:-${ANDROID_KEY_PASSWORD:-$STOREPASS}}"
    if [ -n "$KEY_ALIAS" ]; then
        echo "--> Step 7: Signing AAB with jarsigner..."
        jarsigner \
            -keystore "$KEYSTORE" \
            -storepass "$STOREPASS" \
            -keypass "$KEY_PASS" \
            "$OUTPUT_AAB" \
            "$KEY_ALIAS"
        jarsigner -verify "$OUTPUT_AAB"
        echo "AAB signing and verification complete."
    fi
fi

echo "=== Successfully built AAB at: $OUTPUT_AAB ==="
ls -lh "$OUTPUT_AAB"
