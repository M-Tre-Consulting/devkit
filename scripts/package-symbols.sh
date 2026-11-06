#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Package Native Debug Symbols for Google Play Store (.zip)
# ==============================================================================

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

OUTPUT_ZIP="${1:-target/release/bundle/native-debug-symbols.zip}"
PROFILE="${2:-release}"
LIB_NAME="libdevkit.so"

echo "=== Packaging Native Debug Symbols for Google Play ==="
echo "Profile:     $PROFILE"
echo "Output ZIP:  $OUTPUT_ZIP"

# Architecture mappings: Rust target triple -> Android ABI folder
declare -A ARCH_MAP=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["x86_64-linux-android"]="x86_64"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["i686-linux-android"]="x86"
)

STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGING_DIR"' EXIT

FOUND_COUNT=0

echo ""
echo "Collecting unstripped libraries:"
for TRIPLE in "${!ARCH_MAP[@]}"; do
    ABI="${ARCH_MAP[$TRIPLE]}"
    CANDIDATE="$PROJECT_ROOT/target/$TRIPLE/$PROFILE/$LIB_NAME"

    if [ ! -f "$CANDIDATE" ]; then
        CANDIDATE="$PROJECT_ROOT/target/$TRIPLE/$PROFILE/deps/$LIB_NAME"
    fi

    # Fallback to debug if release not found
    if [ ! -f "$CANDIDATE" ] && [ "$PROFILE" = "release" ]; then
        if [ -f "$PROJECT_ROOT/target/$TRIPLE/debug/$LIB_NAME" ]; then
            CANDIDATE="$PROJECT_ROOT/target/$TRIPLE/debug/$LIB_NAME"
        fi
    fi

    if [ -f "$CANDIDATE" ]; then
        echo "  -> [$ABI] from $CANDIDATE ($(du -h "$CANDIDATE" | cut -f1))"
        mkdir -p "$STAGING_DIR/$ABI"
        cp "$CANDIDATE" "$STAGING_DIR/$ABI/$LIB_NAME"
        FOUND_COUNT=$((FOUND_COUNT + 1))
    fi
done

if [ "$FOUND_COUNT" -eq 0 ]; then
    echo "Error: No unstripped '$LIB_NAME' files found under target/<triple>/$PROFILE." >&2
    echo "Please build with 'cargo apk build --release' or 'cargo build --target <arch> --release' first." >&2
    exit 1
fi

mkdir -p "$(dirname "$OUTPUT_ZIP")"
rm -f "$OUTPUT_ZIP"

echo ""
echo "Creating ZIP archive: $OUTPUT_ZIP"
if command -v zip &>/dev/null; then
    (cd "$STAGING_DIR" && zip -1 -q -r "$PROJECT_ROOT/$OUTPUT_ZIP" .)
elif command -v jar &>/dev/null; then
    (cd "$STAGING_DIR" && jar -cMf "$PROJECT_ROOT/$OUTPUT_ZIP" .)
else
    echo "Error: neither 'zip' nor 'jar' command was found." >&2
    exit 1
fi

echo ""
echo "=== Successfully created native debug symbols ZIP ==="
ls -lh "$OUTPUT_ZIP"
echo ""
echo "To upload to Google Play Console:"
echo "1. Go to: Release -> App bundle explorer -> Select version"
echo "2. Click 'Downloads' or 'Files & assets' tab"
echo "3. Under 'Native debug symbols', click the upload arrow and select this ZIP."
