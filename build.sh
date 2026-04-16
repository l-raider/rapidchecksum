#!/usr/bin/env bash
set -euo pipefail

# Set to "release" for an optimized build, or "debug" for an unoptimized build
BUILD_PROFILE="release"

if [ "$BUILD_PROFILE" = "release" ]; then
    cargo build --release
    echo "Binary: target/release/rapidchecksum"
else
    cargo build
    echo "Binary: target/debug/rapidchecksum"
fi
