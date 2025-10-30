#!/bin/bash

# Simple installer for DeepSearch on Linux
# USAGE: After building with 'cargo build --release', run this script from within the 'packaging/linux' directory with sudo.
# sudo ./install.sh

# Stop on error
set -e

# 1. Check for root privileges
if [ "$(id -u)" -ne 0 ]; then
    echo "Please run this script with sudo: sudo ./install.sh"
    exit 1
fi

# 2. Define paths
# The script assumes it's run from 'packaging/linux', so the target is two levels up.
BINARY_SRC="../../target/release/DeepSearch"
BINARY_DEST="/usr/local/bin/deepsearch"
ICON_SRC="../../assets/icon.png"
ICON_DEST_DIR="/usr/share/icons/hicolor/256x256/apps"
ICON_DEST_NAME="deepsearch.png"
DESKTOP_FILE_SRC="deepsearch.desktop"
DESKTOP_FILE_DEST="/usr/share/applications/deepsearch.desktop"

# 3. Check if source binary exists
if [ ! -f "$BINARY_SRC" ]; then
    echo "Error: Release binary not found at $BINARY_SRC"
    echo "Please run 'cargo build --release' first."
    exit 1
fi

# 4. Copy the binary
echo "Installing DeepSearch binary to $BINARY_DEST..."
install -m 755 "$BINARY_SRC" "$BINARY_DEST"

# 5. Copy the icon
echo "Installing icon to $ICON_DEST_DIR..."
install -d "$ICON_DEST_DIR" # Create directory if it doesn't exist
install -m 644 "$ICON_SRC" "$ICON_DEST_DIR/$ICON_DEST_NAME"

# 6. Copy the desktop file
echo "Installing desktop file to /usr/share/applications..."
install -m 644 "$DESKTOP_FILE_SRC" "$DESKTOP_FILE_DEST"

# 7. Update caches
echo "Updating icon cache and desktop database..."
# Check if commands exist before running them to avoid errors on some systems
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor
fi
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database -q /usr/share/applications
fi

echo ""
echo "Installation complete! You should now find 'DeepSearch' in your application menu."
