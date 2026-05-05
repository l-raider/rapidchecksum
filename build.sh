#!/usr/bin/env bash
set -euo pipefail

# Set to "release" for an optimized build, or "debug" for an unoptimized build
BUILD_PROFILE="release"

if [ "$BUILD_PROFILE" = "release" ]; then
    echo "Starting cargo build --release..."
    cargo build --release
    echo "Binary: target/release/rapidchecksum"
else
    echo "Starting cargo build..."
    cargo build
    echo "Binary: target/debug/rapidchecksum"
fi
