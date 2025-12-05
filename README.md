<div align="center">

# 🎛️ Bus Channel Strip VST Plugin

**A professional multi-module bus channel strip VST3/CLAP plugin built with Rust**

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![VST3](https://img.shields.io/badge/VST3-✓-blue.svg)](https://steinbergmedia.github.io/vst3_doc/)
[![CLAP](https://img.shields.io/badge/CLAP-✓-green.svg)](https://cleveraudio.org/)
[![License](https://img.shields.io/badge/license-GPL--3.0-red.svg)](LICENSE)
[![Cross Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#platform-support)

*Built with [NIH-Plug](https://github.com/robbert-vdh/nih-plug), [Airwindows DSP](https://github.com/airwindows/airwindows), and [vizia](https://vizia.dev/) GUI framework*

</div>

---

## ✨ Features

### 🔊 Signal Chain
```
[🎚️ API5500 EQ] → [🗜️ ButterComp2] → [📻 Pultec EQ] → [⚡ Dynamic EQ] → [🎭 Transformer] → [💥 Punch]
```

### 🎛️ DSP Modules
| Module | Type | Description |
|--------|------|-------------|
| **🎚️ API5500 EQ** | Semi-Parametric | 5-band equalizer with classic API 5500 character |
| **🗜️ ButterComp2** | Compressor | Airwindows bi-polar interleaved compression system |
| **📻 Pultec EQ** | Tube EQ | Custom EQP-1A style EQ with tube saturation modeling |
| **⚡ Dynamic EQ** | Frequency-Dependent | 4-band dynamic EQ with intelligent compression |
| **🎭 Transformer** | Saturation | Transformer coloration with 4 vintage models |
| **💥 Punch** | Clipper + Transient | Transparent clipping with transient restoration for louder, punchier mixes |

### 🚀 Current Status
> **✅ PRODUCTION READY** - Full CI/CD pipeline with automated releases

| Component | Status | Description |
|-----------|--------|-------------|
| 🔧 **Core DSP** | ✅ **COMPLETE** | All 6 modules implemented and functional |
| 🎛️ **Parameters** | ✅ **COMPLETE** | ~90 automation parameters with module reordering |
| 🏗️ **Build System** | ✅ **COMPLETE** | Successful VST3/CLAP bundle creation |
| 🤖 **CI/CD Pipeline** | ✅ **WORKING** | Multi-platform builds (Windows/macOS/Linux) |
| 🎨 **GUI** | ✅ **INTEGRATED** | vizia-plug with Skia graphics rendering |
| 📦 **Releases** | ✅ **AUTOMATED** | GitHub releases with cross-platform binaries |

## 🚀 Quick Start

### 📦 Download Ready-to-Use Binaries
**Recommended for most users**

1. Go to [**Releases**](../../releases/latest)
2. Download the package for your platform:
   - 🪟 **Windows**: `Bus-Channel-Strip-windows.zip`
   - 🍎 **macOS Intel**: `Bus-Channel-Strip-macos-intel.tar.gz`
   - 🍎 **macOS ARM64**: `Bus-Channel-Strip-macos-arm64.tar.gz`
   - 🐧 **Linux**: `Bus-Channel-Strip-linux.tar.gz`
3. Extract to your VST3/CLAP plugin directory
4. Restart your DAW and enjoy!

---

## 🛠️ Build From Source

<details>
<summary><b>🔧 System Requirements</b></summary>

### 📋 Dependencies
| Requirement | Version | Purpose |
|-------------|---------|---------|
| **🦀 Rust Nightly** | `1.70+` | Required for vizia-plug GUI features |
| **🔨 Build Tools** | VS 2022 | C++ compilation for FFI modules |
| **🪟 Windows SDK** | 10/11 | Windows target compilation |
| **⚡ LLVM/Clang** | Latest | Bindgen and cross-compilation |

</details>

<details>
<summary><b>⚡ Quick Build Commands</b></summary>

```bash
# 🦀 Install Rust nightly
rustup toolchain install nightly

# 🏗️ Core build (no GUI)
cargo build --no-default-features --features "api5500,buttercomp2,pultec,transformer"

# 🎨 Full build with GUI
cargo +nightly build --features "api5500,buttercomp2,pultec,transformer,gui"

# 📦 Create production bundles (recommended)
set FORCE_SKIA_BINARIES_DOWNLOAD=1
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,gui"
```

</details>

<details>
<summary><b>🪟 Windows Build Scripts</b></summary>

For Windows users, automated build scripts are provided:

```batch
# 🚀 Simplified build (recommended)
bin\preflight_build_simple.bat

# 🎯 Full build and install to DAW
bin\debug_plugin_simple.bat
```

</details>

---

## 🧪 Testing & Quality Assurance

<details>
<summary><b>🎵 DAW Compatibility Testing</b></summary>

| DAW | VST3 | CLAP | Status | Notes |
|-----|------|------|--------|-------|
| 🎛️ **Reaper** | ✅ | ✅ | Planned | Industry standard compatibility |
| 🎹 **Pro Tools** | ✅ | ❌ | Planned | VST3 support only |
| 🍎 **Logic Pro X** | ✅ | ❌ | Planned | macOS VST3 + AU planned |
| 🎼 **Cubase** | ✅ | ❌ | Planned | VST3 native support |
| 🎶 **FL Studio** | ✅ | ❌ | Planned | Parameter automation testing |
| 🔄 **Bitwig Studio** | ✅ | ✅ | Planned | CLAP native support |

**Testing Checklist:**
- [ ] Parameter automation in each DAW
- [ ] Preset save/load functionality
- [ ] Plugin scanner compatibility
- [ ] Real-time performance optimization

</details>

<details>
<summary><b>🔊 Audio Quality Verification</b></summary>

| Test Category | Metrics | Status |
|---------------|---------|--------|
| **📊 THD+N** | < 0.01% @ 1kHz | Planned |
| **📈 Frequency Response** | ±0.1dB 20Hz-20kHz | Planned |
| **⏱️ Phase Response** | Linear phase option | Planned |
| **🔄 Sample Rates** | 44.1-192kHz support | Planned |
| **🚫 Artifacts** | Click/pop detection | Planned |

**Quality Standards:**
- ✅ Lock-free real-time processing
- ✅ Allocation-free audio thread
- ✅ Professional parameter ranges
- 🔄 Reference implementation A/B testing

</details>

<details>
<summary><b>⚡ Performance Benchmarks</b></summary>

| Platform | CPU Usage | Memory | Latency |
|----------|-----------|--------|---------|
| **🪟 Windows 11** | TBD | TBD | TBD |
| **🍎 macOS 14+** | TBD | TBD | TBD |
| **🐧 Linux** | TBD | TBD | TBD |

**Performance Goals:**
- [ ] < 5% CPU usage @ 44.1kHz/64 samples
- [ ] Zero memory leaks in 24h+ sessions
- [ ] Sub-millisecond parameter updates
- [ ] Stress testing with 100+ instances

</details>

---

## 🏗️ Technical Architecture

<details>
<summary><b>🔧 Plugin Framework</b></summary>

| Component | Technology | Purpose |
|-----------|------------|---------|
| **🦀 Core Framework** | [NIH-Plug](https://github.com/robbert-vdh/nih-plug) | Modern Rust plugin framework with ~75 parameters |
| **🎨 GUI System** | [vizia](https://vizia.dev/) + Skia | CSS-like styling with hardware-accelerated rendering |
| **🔄 Processing** | Lock-free/Allocation-free | Real-time audio thread safety |
| **🎛️ Modularity** | Dynamic reordering | User-configurable signal chain |

</details>

<details>
<summary><b>📦 Dependencies</b></summary>

### 🔑 Core Dependencies
```toml
nih_plug = { git = "https://github.com/robbert-vdh/nih-plug.git" }    # Plugin framework
vizia_plug = { git = "https://github.com/vizia/vizia-plug.git" }      # GUI integration
biquad = "0.5.0"                                                      # Filter implementations
fundsp = "0.20.0"                                                     # DSP utilities
realfft = "3.5.0"                                                     # FFT processing
augmented-dsp-filters = "2.5.0"                                       # Additional filters
```

### 🎨 GUI Dependencies
```toml
atomic_float = "0.1"                    # Thread-safe GUI operations
skia-safe = { version = "0.84" }        # Graphics rendering
```

### 🔗 FFI Integration
- **C++ Airwindows**: `extern "C"` interfaces in `cpp/` directory
- **Build System**: Custom `build.rs` for C++ compilation

</details>

<details>
<summary><b>🌍 Platform Support</b></summary>

| Platform | Status | Formats | Notes |
|----------|--------|---------|-------|
| **🪟 Windows** | ✅ **Production** | VST3, CLAP | Primary development platform |
| **🍎 macOS Intel** | ✅ **Production** | VST3, CLAP | CI/CD automated builds |
| **🍎 macOS ARM64** | ✅ **Production** | VST3, CLAP | Native Apple Silicon support |
| **🐧 Linux** | ✅ **Production** | VST3, CLAP | Ubuntu 22.04+ LTS |
| **🍎 Audio Units** | 🔄 **Planned** | AU | macOS native format |

</details>

<details>
<summary><b>📁 Project Structure</b></summary>

```
🎛️ bus_channel_strip/
├── 🦀 src/                 # Rust source code
│   ├── lib.rs              # Plugin entry point & parameter management
│   ├── api5500.rs          # 5-band semi-parametric EQ module
│   ├── buttercomp2.rs      # Airwindows ButterComp2 FFI wrapper
│   ├── pultec.rs           # Pultec EQP-1A tube EQ implementation
│   ├── dynamic_eq.rs       # 4-band dynamic EQ (optional feature)
│   ├── transformer.rs      # Transformer saturation module
│   ├── editor.rs           # vizia GUI implementation
│   ├── components.rs       # Reusable GUI components
│   ├── shaping.rs          # Common DSP math functions
│   └── spectral.rs         # FFT analysis utilities
├── 🔗 cpp/                 # C++ FFI wrappers for Airwindows
├── 🎨 assets/              # GUI resources and themes
├── ⚙️ xtask/               # Custom build tooling
├── 🛠️ bin/                 # Build scripts and utilities
├── 🤖 .github/workflows/   # CI/CD automation
└── 📦 target/bundled/      # Output: VST3/CLAP bundles
```

</details>

---

## 🤝 Contributing & Support

<div align="center">

### 📚 Documentation

**[`CLAUDE.md`](CLAUDE.md)** • Development guidelines and AI assistant context

**[`docs/`](docs/)** • Extended documentation:
- **[`AGENTS.md`](docs/AGENTS.md)** - Original project specifications and architecture
- **[`GUI_DESIGN.md`](docs/GUI_DESIGN.md)** - Complete GUI specifications and responsive design
- **[`PUNCH_MODULE_SPEC.md`](docs/PUNCH_MODULE_SPEC.md)** - Punch module DSP research and implementation
- **[`VIZIA_AGENT_SPEC.md`](docs/VIZIA_AGENT_SPEC.md)** - vizia GUI specialist documentation
- **[`CLIPPING_INSIGHTS.md`](docs/CLIPPING_INSIGHTS.md)** - Professional loudness techniques

### 🐛 Issues & Features
Found a bug? Have a feature request?
[**🔗 Open an Issue**](../../issues) • [**💬 Discussions**](../../discussions)

### 📄 License
**[GPL-3.0-or-later](LICENSE)** • Free and open source

</div>

---

<div align="center">

**🎵 Ready for Production** • **September 2025** • **Rust + NIH-Plug + vizia + Airwindows**

[![Download Latest Release](https://img.shields.io/badge/Download-Latest%20Release-brightgreen?style=for-the-badge)](../../releases/latest)

</div>