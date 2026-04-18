#!/usr/bin/env bash
# Build a Flatpak for RapidChecksum.
#
# First run:  chmod +x build-flatpak.sh
# Then:       ./build-flatpak.sh
#
# Re-run any time you change Cargo.lock or source code.
# The script regenerates cargo-sources.json automatically when Cargo.lock
# is newer than the cached sources file.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

MANIFEST="flatpak/com.rapidchecksum.app.yml"
BUILD_DIR="flatpak/build"
REPO_DIR="flatpak/repo"
GENERATOR="flatpak/flatpak-cargo-generator.py"
CARGO_SOURCES="flatpak/cargo-sources.json"
GENERATOR_URL="https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"

# ── Dependency checks ────────────────────────────────────────────────────────
for cmd in flatpak flatpak-builder python3 curl pip3; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: '$cmd' is not installed. Please install it and try again." >&2
        exit 1
    fi
done

# Ensure the Python dependency needed by flatpak-cargo-generator is present
if ! python3 -c "import aiohttp" &>/dev/null; then
    echo "Installing required Python package: aiohttp..."
    pip3 install --user aiohttp
fi

# ── Download the generator once ──────────────────────────────────────────────
if [[ ! -f "$GENERATOR" ]]; then
    echo "Downloading flatpak-cargo-generator.py..."
    curl -fsSL "$GENERATOR_URL" -o "$GENERATOR"
fi

# ── Regenerate cargo sources when Cargo.lock is newer ────────────────────────
if [[ ! -f "$CARGO_SOURCES" || Cargo.lock -nt "$CARGO_SOURCES" ]]; then
    echo "Generating offline cargo sources from Cargo.lock..."
    python3 "$GENERATOR" Cargo.lock -o "$CARGO_SOURCES"
fi

# ── Build ─────────────────────────────────────────────────────────────────────
echo "Running flatpak-builder..."
flatpak-builder \
    --force-clean \
    --user \
    --install-deps-from=flathub \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    "$MANIFEST"

echo
echo "Build complete!"
echo
echo "  Test without installing:"
echo "    flatpak-builder --run $BUILD_DIR $MANIFEST rapidchecksum"
echo
echo "  Install for current user:"
echo "    flatpak remote-add --user --no-gpg-verify rc-local $REPO_DIR"
echo "    flatpak install --user rc-local com.rapidchecksum.app"
echo "    flatpak run com.rapidchecksum.app"
echo
echo "  Create a single-file bundle:"
echo "    flatpak build-bundle $REPO_DIR rapidchecksum.flatpak com.rapidchecksum.app"
