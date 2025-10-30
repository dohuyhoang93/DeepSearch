#!/bin/bash

# Script to create a macOS .app bundle for DeepSearch
# USAGE: After building with 'cargo build --release', run this script from the project root.
# ./packaging/macos/package_macos.sh

# Stop on error
set -e

# --- Configuration ---
APP_NAME="DeepSearch"
# This script is run from the project root
BINARY_SRC="./target/release/DeepSearch"
DEST_DIR="./target/release"
APP_BUNDLE_PATH="$DEST_DIR/$APP_NAME.app"
ICON_SRC="./assets/icon.png"
PLIST_SRC="./packaging/macos/Info.plist"

echo "Creating macOS application bundle for $APP_NAME..."

# 1. Check for source binary
if [ ! -f "$BINARY_SRC" ]; then
    echo "Error: Release binary not found at $BINARY_SRC"
    echo "Please run 'cargo build --release' first."
    exit 1
fi

# 2. Create the bundle directory structure
echo "Creating bundle structure at $APP_BUNDLE_PATH"
rm -rf "$APP_BUNDLE_PATH" # Clean up old bundle if it exists
mkdir -p "$APP_BUNDLE_PATH/Contents/MacOS"
mkdir -p "$APP_BUNDLE_PATH/Contents/Resources"

# 3. Copy the binary and rename it
echo "Copying binary..."
cp "$BINARY_SRC" "$APP_BUNDLE_PATH/Contents/MacOS/deepsearch"

# 4. Copy the Info.plist
echo "Copying Info.plist..."
cp "$PLIST_SRC" "$APP_BUNDLE_PATH/Contents/Info.plist"

# 5. Create the .icns file from the source icon
echo "Creating .icns file..."
# Create a temporary iconset directory
ICONSET_DIR=$(mktemp -d)
# Create different sizes required for the .icns file using sips
sips -z 16 16     "$ICON_SRC" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32     "$ICON_SRC" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32     "$ICON_SRC" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64     "$ICON_SRC" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128   "$ICON_SRC" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256   "$ICON_SRC" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256   "$ICON_SRC" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512   "$ICON_SRC" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512   "$ICON_SRC" --out "$ICONSET_DIR/icon_512x512.png"
sips -z 1024 1024 "$ICON_SRC" --out "$ICONSET_DIR/icon_512x512@2x.png"
# Convert the iconset to an .icns file
iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE_PATH/Contents/Resources/icon.icns"
# Clean up the temporary directory
rm -rf "$ICONSET_DIR"

echo ""
echo "Successfully created $APP_BUNDLE_PATH"
echo "You can now drag this file to your Applications folder."
