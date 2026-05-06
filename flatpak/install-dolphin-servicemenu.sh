#!/bin/sh
# Installs the RapidChecksum KDE Dolphin service menu for the current user.
# Install via: flatpak run --command=install-dolphin-servicemenu io.github.l_raider.rapidchecksum

DEST="${HOME}/.local/share/kio/servicemenus"
SRC="/app/share/kio/servicemenus/io.github.l_raider.rapidchecksum-servicemenu.desktop"
NAME="io.github.l_raider.rapidchecksum-servicemenu.desktop"

if [ ! -f "$SRC" ]; then
    echo "Error: service menu source file not found: $SRC" >&2
    echo "The Dolphin service menu was not included in this build." >&2
    exit 1
fi

if ! mkdir -p "$DEST"; then
    echo "Error: could not create directory: $DEST" >&2
    echo "Check that you have write permission to $HOME/.local/share and that the disk is not full." >&2
    exit 1
fi

if ! cp "$SRC" "$DEST/$NAME"; then
    echo "Error: could not copy service menu file to: $DEST/$NAME" >&2
    echo "Check disk space and write permissions on $DEST." >&2
    exit 1
fi

if ! chmod +x "$DEST/$NAME"; then
    echo "Error: could not mark service menu file as executable: $DEST/$NAME" >&2
    echo "KDE requires the file to be executable to treat it as authorized." >&2
    exit 1
fi

echo "RapidChecksum service menu installed to:"
echo "  $DEST/$NAME"
echo ""
echo "Please restart Dolphin for the context menu entry to appear."
