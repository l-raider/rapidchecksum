#!/usr/bin/env bash
# Build a Flatpak for RapidChecksum.
#
# First run:  chmod +x build-flatpak.sh
# Then:       ./build-flatpak.sh
#
# Re-run any time you change Cargo.lock or source code.
# The vendor directory is regenerated automatically when Cargo.lock changes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

MANIFEST="flatpak/com.rapidchecksum.app.yml"
BUILD_DIR="flatpak/build"
REPO_DIR="flatpak/repo"
VENDOR_DIR="flatpak/vendor"
VENDOR_CONFIG="flatpak/cargo-vendor-config.toml"

# ── Dependency checks ────────────────────────────────────────────────────────
for cmd in flatpak flatpak-builder cargo rsvg-convert; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: '$cmd' is not installed. Please install it and try again." >&2
        echo "  rsvg-convert is provided by: sudo apt install librsvg2-bin" >&2
        exit 1
    fi
done

# ── Vendor all crates when Cargo.lock is newer than the vendor dir ───────────
if [[ ! -d "$VENDOR_DIR" || Cargo.lock -nt "$VENDOR_DIR" ]]; then
    echo "Vendoring crates (cargo vendor)..."
    cargo vendor "$VENDOR_DIR" > "$VENDOR_CONFIG"
    touch "$VENDOR_DIR"   # update mtime so we don't re-vendor unnecessarily
fi

# ── Generate icon PNGs from SVG when the SVG is newer ────────────────────────
SVG_ICON="flatpak/icons/com.rapidchecksum.app.svg"
ICON_SENTINEL="flatpak/icons/hicolor/256x256/apps/com.rapidchecksum.app.png"
if [[ ! -f "$ICON_SENTINEL" || "$SVG_ICON" -nt "$ICON_SENTINEL" ]]; then
    echo "Generating icon PNGs from SVG..."
    for size in 16 32 48 64 128 256 512; do
        dir="flatpak/icons/hicolor/${size}x${size}/apps"
        mkdir -p "$dir"
        rsvg-convert -w "$size" -h "$size" "$SVG_ICON" \
            -o "$dir/com.rapidchecksum.app.png"
    done
    mkdir -p "flatpak/icons/hicolor/scalable/apps"
    cp "$SVG_ICON" "flatpak/icons/hicolor/scalable/apps/com.rapidchecksum.app.svg"
fi

# ── Ensure flathub remote is present (needed to fetch SDKs) ──────────────────
if ! flatpak remote-list --user | grep -q '^flathub'; then
    echo "Adding flathub remote..."
    flatpak remote-add --user --if-not-exists flathub \
        https://dl.flathub.org/repo/flathub.flatpakrepo
fi

# ── Ensure the rust-stable SDK extension is installed ────────────────────────
# Read the KDE branch from the manifest, then look up the freedesktop branch
# from the specific KDE SDK metadata to find the matching rust-stable branch.
KDE_BRANCH=$(awk '/^runtime-version:/{gsub(/[^0-9.]/, "", $2); print $2; exit}' "$MANIFEST")
KDE_BRANCH="${KDE_BRANCH:-6.10}"
FD_BRANCH=$(flatpak info --show-metadata "org.kde.Sdk/x86_64/${KDE_BRANCH}" 2>/dev/null \
    | awk '/^\[Extension org\.freedesktop\.Platform\.GL\]/{f=1}
           f && /^versions=/{split($0,a,"[=;]"); print a[2]; exit}')
FD_BRANCH="${FD_BRANCH:-25.08}"
RUST_EXT="org.freedesktop.Sdk.Extension.rust-stable/x86_64/${FD_BRANCH}"
if ! flatpak list --runtime --columns=ref --all 2>/dev/null | grep -qF "$RUST_EXT"; then
    echo "Installing ${RUST_EXT}..."
    flatpak install --user -y flathub "$RUST_EXT"
fi

# ── Build ─────────────────────────────────────────────────────────────────────
echo "Running flatpak-builder..."
flatpak-builder \
    --force-clean \
    --disable-rofiles-fuse \
    --user \
    --install-deps-from=flathub \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    "$MANIFEST"

# ── Create the distributable .flatpak bundle ──────────────────────────────────
BUNDLE="rapidchecksum.flatpak"
echo "Creating ${BUNDLE}..."
flatpak build-bundle "$REPO_DIR" "$BUNDLE" com.rapidchecksum.app

echo
echo "Done! Bundle created: ${BUNDLE}"
echo
echo "  Test without installing:"
echo "    flatpak-builder --run $BUILD_DIR $MANIFEST rapidchecksum"
echo
echo "  Install from bundle:"
echo "    flatpak install --user ${BUNDLE}"
echo "    flatpak run com.rapidchecksum.app"
