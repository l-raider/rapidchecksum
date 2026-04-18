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

# ── Detect the freedesktop SDK branch the installed KDE SDK is based on ──────
# The rust-stable SDK extension branch must match this (e.g. "24.08").
KDE_VER="6.9"
FD_BRANCH=$(flatpak info --show-metadata "org.kde.Sdk/x86_64/${KDE_VER}" 2>/dev/null \
    | awk -F'[=/]' '/^runtime=/{print $NF; exit}')

if [[ -z "$FD_BRANCH" ]]; then
    # KDE SDK not installed yet — flatpak-builder will install it; we cannot
    # detect the branch until after install. Fall back to querying flathub.
    FD_BRANCH=$(flatpak remote-info --user flathub \
        "org.freedesktop.Sdk.Extension.rust-stable" 2>/dev/null \
        | awk '/Branch:/{print $2}' | sort -V | tail -1)
fi

if [[ -z "$FD_BRANCH" ]]; then
    echo "Warning: could not detect freedesktop SDK branch; defaulting to 24.08."
    FD_BRANCH="24.08"
else
    echo "Detected freedesktop SDK branch: ${FD_BRANCH}"
fi

# Write a temporary manifest with the correct rust-stable branch substituted in.
# The temp file stays next to the original so relative paths (path: ..) still work.
TMP_MANIFEST="flatpak/_tmp_build.yml"
trap 'rm -f "${TMP_MANIFEST}"' EXIT
sed "s|rust-stable//[0-9][0-9.]*|rust-stable//${FD_BRANCH}|g" \
    "$MANIFEST" > "$TMP_MANIFEST"

# ── Build ─────────────────────────────────────────────────────────────────────
echo "Running flatpak-builder..."
flatpak-builder \
    --force-clean \
    --user \
    --install-deps-from=flathub \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    "$TMP_MANIFEST"

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
