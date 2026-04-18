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
for cmd in flatpak flatpak-builder cargo; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: '$cmd' is not installed. Please install it and try again." >&2
        exit 1
    fi
done

# ── Vendor all crates when Cargo.lock is newer than the vendor dir ───────────
if [[ ! -d "$VENDOR_DIR" || Cargo.lock -nt "$VENDOR_DIR" ]]; then
    echo "Vendoring crates (cargo vendor)..."
    cargo vendor "$VENDOR_DIR" > "$VENDOR_CONFIG"
    touch "$VENDOR_DIR"   # update mtime so we don't re-vendor unnecessarily
fi

# ── Ensure flathub remote is present (needed to fetch SDKs) ──────────────────
if ! flatpak remote-list --user | grep -q '^flathub'; then
    echo "Adding flathub remote..."
    flatpak remote-add --user --if-not-exists flathub \
        https://dl.flathub.org/repo/flathub.flatpakrepo
fi

# ── Ensure the rust-stable SDK extension is installed ────────────────────────
# flatpak-builder resolves the branch from the SDK automatically when no
# branch is specified in sdk-extensions — but it won't auto-install the
# extension, so we do it here using the same branch as org.freedesktop.Sdk.
FD_BRANCH=$(flatpak info --show-metadata "org.kde.Sdk" 2>/dev/null \
    | awk '/^\[Extension org\.freedesktop\.Platform\.GL\]/{f=1}
           f && /^versions=/{split($0,a,"[=;]"); print a[2]; exit}')
FD_BRANCH="${FD_BRANCH:-25.08}"
RUST_EXT="org.freedesktop.Sdk.Extension.rust-stable/x86_64/${FD_BRANCH}"
if ! flatpak list --runtime --columns=ref 2>/dev/null | grep -qF "$RUST_EXT"; then
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
