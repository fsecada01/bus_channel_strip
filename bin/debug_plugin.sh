#!/bin/bash
# Debug plugin build and install script
# Usage: ./bin/debug_plugin.sh

set -e  # Exit on any error

echo "=== Building VST3 plugin for debugging ==="
echo "Features: api5500, buttercomp2, pultec, transformer, gui (iced-rs GUI)"
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