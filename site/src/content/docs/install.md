---
title: Installation
description: Download and install Bus Channel Strip VST3/CLAP plugin on Windows, macOS, and Linux.
---

## Download Pre-Built Binaries

Go to the [latest release](https://github.com/fsecada01/bus_channel_strip/releases/latest) and download the archive for your platform.

| Platform | File | GUI |
|----------|------|-----|
| Windows | `Bus-Channel-Strip-windows.zip` | ✓ |
| macOS Apple Silicon | `Bus-Channel-Strip-macos-arm64.tar.gz` | ✓ |
| macOS Intel | `Bus-Channel-Strip-macos-intel.tar.gz` | — (headless) |
| Linux | `Bus-Channel-Strip-linux.tar.gz` | ✓ |

:::note[macOS Intel]
The Intel build does not include the vizia GUI due to a Skia cross-compilation limitation on ARM64 CI runners. Intel Mac users can use the ARM64 build via Rosetta 2, or [build from source](#build-from-source) locally to get the full GUI.
:::

---

## Plugin Directories

### Windows

**VST3:**
```
C:\Program Files\Common Files\VST3\Bus-Channel-Strip.vst3\
```

**CLAP:**
```
C:\Program Files\Common Files\CLAP\Bus-Channel-Strip.clap
```

Using the `just install` recipe (requires admin terminal for CLAP):
```powershell
just install
```

### macOS

**VST3:**
```
~/Library/Audio/Plug-Ins/VST3/
```

**CLAP:**
```
~/Library/Audio/Plug-Ins/CLAP/
```

### Linux

**VST3 (user):**
```
~/.vst3/
```

**VST3 (system-wide):**
```
/usr/lib/vst3/
```

**CLAP:**
```
~/.clap/
```

After installing, rescan plugins in your DAW. In Reaper: `Options → Preferences → Plug-ins → VST → Re-scan`.

---

## Build From Source

### Requirements

| Dependency | Notes |
|------------|-------|
| **Rust nightly** | `rustup toolchain install nightly` — required by vizia-plug |
| **LLVM/Clang 19+** | Windows only, required for Skia/bindgen |
| **Visual Studio Build Tools 2022** | Windows only, C++ FFI compilation |
| **clang / gcc** | Linux, for C++ FFI |

### Build Commands

```bash
# Fast type-check (no codegen)
just check

# Production bundle: VST3 + CLAP with GUI
just bundle

# Bundle + install to system plugin directories
just deploy

# Quality gate (fmt + lint + test)
just qa
```

**Windows — manual command:**

```cmd
set LLVM_HOME=C:\Program Files\LLVM
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
cargo +nightly run --package xtask -- bundle bus_channel_strip --release ^
  --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
```

Output bundles land in `target/bundled/`.

### Feature Flags

| Flag | Description |
|------|-------------|
| `api5500` | API5500 EQ module |
| `buttercomp2` | ButterComp2 compressor (includes C++ FFI compilation) |
| `pultec` | Pultec EQ module |
| `transformer` | Transformer module |
| `punch` | Punch clipper + transient shaper |
| `dynamic_eq` | 4-band Dynamic EQ with spectral analyzer |
| `gui` | vizia-plug GUI (requires nightly Rust + LLVM on Windows) |

All features are enabled by default except `gui`. Build without GUI for headless/CI environments:

```bash
cargo build --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq"
```

---

## DAW Compatibility

| DAW | VST3 | CLAP | Notes |
|-----|------|------|-------|
| **Reaper** | ✓ | ✓ | Primary tested host |
| **Bitwig Studio** | ✓ | ✓ | CLAP native support |
| **Cubase** | ✓ | — | VST3 native |
| **FL Studio** | ✓ | — | VST3 support |
| **Pro Tools** | ✓ | — | VST3 (AAX not supported) |
| **Logic Pro** | ✓ | — | VST3; AU not supported |
| **Ableton Live** | ✓ | — | VST3 |
