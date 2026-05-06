#!/bin/sh
# Installs the RapidChecksum KDE Dolphin service menu for the current user.
# Install via: flatpak run --command=install-dolphin-servicemenu io.github.l_raider.rapidchecksum

set -e

DEST="${HOME}/.local/share/kio/servicemenus"
SRC="/app/share/kio/servicemenus/io.github.l_raider.rapidchecksum-servicemenu.desktop"
NAME="io.github.l_raider.rapidchecksum-servicemenu.desktop"

mkdir -p "$DEST"
cp "$SRC" "$DEST/$NAME"
chmod +x "$DEST/$NAME"

echo "RapidChecksum service menu installed to:"
echo "  $DEST/$NAME"
echo ""
echo "Please restart Dolphin for the context menu entry to appear."
