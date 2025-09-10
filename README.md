# Bus Channel Strip VST Plugin

A professional multi-module bus channel strip VST plugin built with NIH-Plug and Airwindows-based DSP modules in Rust.

## Features

### Signal Chain
**[API5500 EQ] → [ButterComp2] → [Pultec EQ] → [Dynamic EQ] → [Transformer]**

### DSP Modules
- **API5500 EQ**: 5-band semi-parametric equalizer with classic API 5500 character
- **ButterComp2**: Airwindows bi-polar interleaved compression system
- **Pultec EQ**: Custom EQP-1A style EQ with tube saturation modeling  
- **Dynamic EQ**: 4-band dynamic EQ with frequency-dependent compression
- **Transformer**: Transformer coloration module with 4 vintage models

### Current Status
- ✅ **ALL 5 CORE MODULES IMPLEMENTED AND FUNCTIONAL**
- ✅ **MODULE REORDERING SYSTEM COMPLETE** 
- ✅ **PROFESSIONAL PARAMETER SET (~75 parameters)**
- ✅ **ALL COMPILATION ERRORS FIXED**
- ✅ **SUCCESSFUL BUILD AND BUNDLE WORKING**
- ✅ **SUCCESSFUL VST3 AND CLAP BUNDLE CREATION**
- 🔧 **vizia GUI IMPLEMENTATION** in progress (ECS architecture ready)
- 🔧 **CI/CD PIPELINE** needs bundle command fixes

## Build Requirements

### Dependencies
- **Rust Nightly** (required for vizia-plug)
- **Visual Studio Build Tools 2022** with C++ workload
- **Windows 10/11 SDK**
- **LLVM/Clang** (for bindgen)
- **Ninja** (for Skia build system)

### Quick Setup
```bash
# Install Rust nightly
rustup toolchain install nightly

# Build without GUI
cargo build --no-default-features --features "api5500,buttercomp2,pultec,transformer"

# Build with GUI  
cargo +nightly build --features "api5500,buttercomp2,pultec,transformer,gui"

# Create plugin bundles (recommended command)
set FORCE_SKIA_BINARIES_DOWNLOAD=1
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,gui"
```

### Windows Build Scripts
For Windows users, use the automated build scripts:
```batch
# Simplified build (recommended)
bin\preflight_build_simple.bat

# Full build and install to DAW
bin\debug_plugin_simple.bat
```

## Beta Testing Todos

### Pre-Release Testing
- [ ] **DAW Compatibility Testing**
  - [ ] Test VST3 in Reaper, Pro Tools, Logic Pro X, Cubase, FL Studio
  - [ ] Test CLAP in Bitwig Studio, Reaper (CLAP support)
  - [ ] Verify parameter automation in each DAW
  - [ ] Test preset save/load functionality

- [ ] **Audio Quality Verification**
  - [ ] A/B test each module against reference implementations
  - [ ] Measure THD+N, frequency response, phase response
  - [ ] Test with various sample rates (44.1kHz, 48kHz, 88.2kHz, 96kHz)
  - [ ] Verify no audio artifacts, clicks, or pops
  - [ ] Test bypass functionality for each module

- [ ] **Performance Testing**
  - [ ] CPU usage benchmarks in different DAWs
  - [ ] Memory leak testing during extended sessions
  - [ ] Stress test with large buffer sizes and high channel counts
  - [ ] Test real-time performance under various system loads

- [ ] **GUI Testing**  
  - [ ] Test GUI responsiveness and parameter updates
  - [ ] Verify GUI scaling on different screen resolutions
  - [ ] Test knob/slider interaction and value display
  - [ ] Verify module reordering interface works correctly
  - [ ] Test GUI with accessibility tools

### Platform Testing
- [ ] **Windows Testing**
  - [ ] Windows 10 (x64)
  - [ ] Windows 11 (x64)  
  - [ ] Different VST host applications
  - [ ] Plugin scanner compatibility

- [ ] **macOS Testing** (Future)
  - [ ] macOS 12+ (Intel and Apple Silicon)
  - [ ] Audio Unit format support
  - [ ] Logic Pro X integration
  - [ ] GateKeeper and code signing

- [ ] **Linux Testing** (Future)
  - [ ] Ubuntu 20.04+ LTS
  - [ ] JACK and ALSA support
  - [ ] Various Linux DAWs (Ardour, Bitwig)

### Documentation & Distribution
- [ ] **User Documentation**
  - [ ] Parameter reference guide
  - [ ] Module descriptions and use cases
  - [ ] Installation instructions
  - [ ] Preset library creation

- [ ] **Beta Release Preparation**  
  - [ ] Create installer packages
  - [ ] Set up crash reporting system
  - [ ] Prepare beta feedback collection system
  - [ ] Create changelog and version tracking

### Known Issues to Address
- [ ] CI/CD pipeline bundle command needs update
- [ ] Complete vizia GUI implementation (architecture ready)
- [ ] Ninja build dependency for final vizia compilation
- [ ] Dead code warnings cleanup (non-critical)

## Architecture

### Plugin Framework
- **NIH-Plug**: Modern Rust plugin framework with ~75 automation parameters
- **vizia**: Modern GUI framework with CSS-like styling and Skia rendering
- **Lock-free Processing**: Allocation-free audio processing thread
- **Module Reordering**: Dynamic signal chain configuration

### Key Dependencies
- `nih_plug` - Plugin framework and host communication
- `vizia_plug` - vizia GUI integration for NIH-Plug (optional)
- `atomic_float` - Thread-safe float operations for GUI (optional)
- `biquad` v0.5.0 - Filter implementations
- `fundsp` - DSP utilities and filters  
- `realfft` - FFT processing for spectral analysis
- `augmented-dsp-filters` - Additional filter types
- Custom C++ FFI wrappers for Airwindows modules

### Build System
- **Rust Nightly**: Required for vizia-plug GUI features
- **vizia-plug**: Handles Skia compilation automatically with pre-built binaries
- **FFI Integration**: C++ Airwindows modules via `extern "C"` interfaces in `cpp/` directory
- **xtask**: Custom build tooling for plugin bundling
- **Cross-platform**: Windows (primary), macOS and Linux (planned)

## File Structure

```
bus_channel_strip/
├── src/
│   ├── lib.rs              # Main plugin entry point
│   ├── api5500.rs          # 5-band semi-parametric EQ
│   ├── buttercomp2.rs      # Airwindows ButterComp2 wrapper
│   ├── pultec.rs           # Pultec EQP-1A style EQ
│   ├── dynamic_eq.rs       # 4-band dynamic EQ (optional)
│   ├── transformer.rs      # Transformer coloration
│   ├── editor.rs           # vizia GUI implementation
│   ├── components.rs       # Reusable GUI components
│   ├── shaping.rs          # Common DSP shaping functions
│   └── spectral.rs         # FFT analysis utilities
├── cpp/                    # FFI wrappers for C++ modules
├── assets/                 # GUI assets and resources
├── bin/                    # Build and utility scripts
└── target/bundled/         # Built plugin bundles (VST3/CLAP)
```

## Contributing

See `CLAUDE.md` for detailed development guidelines and `AGENTS.md` for original project specifications.

## License

GPL-3.0-or-later

---

**Status**: Ready for beta testing phase
**Build Date**: September 2025
**Framework**: NIH-Plug + vizia + Airwindows DSP