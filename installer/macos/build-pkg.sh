#!/usr/bin/env bash
# macOS installer .pkg builder (issue #26 DoD: "macOS pkg installer produced
# and verified"). Packages the xtask bundle output into an unsigned pkg that
# installs both plugin bundles to the standard per-machine Audio Plug-Ins
# locations. Signing/notarization is a separate, not-yet-addressed bullet on
# the same issue — this script deliberately does not pass `--sign`.
#
# Usage: installer/macos/build-pkg.sh <bundle_dir> <version> <output_pkg>
#   bundle_dir  target/bundled — must contain Bus-Channel-Strip.vst3 and
#               Bus-Channel-Strip.clap (each a macOS bundle folder)
#   version     e.g. 1.0.0
#   output_pkg  path to write the .pkg to

set -euo pipefail

BUNDLE_DIR="${1:?usage: build-pkg.sh <bundle_dir> <version> <output_pkg>}"
VERSION="${2:?usage: build-pkg.sh <bundle_dir> <version> <output_pkg>}"
OUTPUT_PKG="${3:?usage: build-pkg.sh <bundle_dir> <version> <output_pkg>}"

for name in Bus-Channel-Strip.vst3 Bus-Channel-Strip.clap; do
  if [[ ! -d "$BUNDLE_DIR/$name" ]]; then
    echo "error: $BUNDLE_DIR/$name not found — run xtask bundle first" >&2
    exit 1
  fi
done

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

ROOT_DIR="$WORK_DIR/root"
mkdir -p "$ROOT_DIR/Library/Audio/Plug-Ins/VST3" "$ROOT_DIR/Library/Audio/Plug-Ins/CLAP"

cp -R "$BUNDLE_DIR/Bus-Channel-Strip.vst3" "$ROOT_DIR/Library/Audio/Plug-Ins/VST3/"
cp -R "$BUNDLE_DIR/Bus-Channel-Strip.clap" "$ROOT_DIR/Library/Audio/Plug-Ins/CLAP/"

pkgbuild \
  --root "$ROOT_DIR" \
  --identifier "com.francissecada.bus-channel-strip" \
  --version "$VERSION" \
  --install-location "/" \
  "$OUTPUT_PKG"

echo "Built $OUTPUT_PKG"
