#!/bin/bash
# Debug plugin build and install script
# Usage: ./bin/debug_plugin.sh

set -e  # Exit on any error

# Set required environment variables
export BUILD_DATE=20250810
export FORCE_SKIA_BINARIES_DOWNLOAD=1
export CARGO_BUILD_JOBS=6

echo "=== Building VST3 plugin for debugging ==="
echo "Features: api5500, buttercomp2, pultec, transformer, gui (vizia GUI with Skia graphics)"
echo "Build dependencies: ninja-build, clang, build-essential installed ✓"

# Build with full GUI features - ninja and clang are now available
cargo xtask bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,gui"

echo "=== Removing old plugin ==="
if [ -d "/mnt/c/Program Files/Common Files/VST3/Bus-Channel-Strip.vst3" ]; then
    rm -rf "/mnt/c/Program Files/Common Files/VST3/Bus-Channel-Strip.vst3"
    echo "Removed old plugin"
else
    echo "No existing plugin found"
fi

echo "=== Installing new plugin ==="
cp -r "target/bundled/Bus-Channel-Strip.vst3" "/mnt/c/Program Files/Common Files/VST3/"
echo "Plugin installed successfully!"

echo "=== Plugin ready for testing in Reaper ==="
echo "Location: /mnt/c/Program Files/Common Files/VST3/Bus-Channel-Strip.vst3"