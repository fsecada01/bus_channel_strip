# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

This document provides context and guidelines for AI assistance with the bus channel strip plugin development.

## Project Overview

A multi-module bus channel strip VST plugin built with NIH-Plug and Airwindows-based DSP modules in Rust.

**Signal Flow**: `[API5500 EQ] → [ButterComp2] → [Pultec EQ] → [Dynamic EQ] → [Transformer]`

**Current Status**: 
- ✅ ALL 5 CORE MODULES IMPLEMENTED AND FUNCTIONAL
- ✅ MODULE REORDERING SYSTEM COMPLETE
- ✅ PROFESSIONAL PARAMETER SET (~75 parameters)
- ✅ ALL COMPILATION ERRORS FIXED
- ✅ LOCAL BUILD AND BUNDLE WORKING
- ✅ vizia-plug GUI INTEGRATION WORKING (September 2025)
- ✅ SUCCESSFUL VST3 AND CLAP BUNDLE CREATION
- 🔧 CI/CD pipeline needs bundle command fixes

## Development Guidelines

### Audio Processing Requirements
- All real-time audio processing must be **lock-free** and **allocation-free**
- Parameters must be automation-safe and uniquely identified
- Use `#[derive(Params)]` for parameter bindings

### DSP Implementation
- Implement math shaping functions in `src/shaping.rs` for reuse across modules
- Common shaping functions:
  - `sigmoid(x)` / `tanh(x)` for soft knees and saturation
  - `poly(x) + log(x)` for filter or tone control curves
  - `log2(x)`, `exp(x)` for perceptual/gain scaling

### FFI Integration
- Airwindows modules must be wrapped in FFI-safe C++ using `extern "C"` interface
- FFI wrappers go in `cpp/*.cpp`
- Use `build.rs` for FFI compilation

### GUI Development
- Built with `vizia` via `vizia-plug` for modern, performant GUI
- Follow vizia architecture patterns: Entity-Component-System (ECS) with reactive state management
- Use CSS-like styling with performant rendering via Skia graphics library
- Module color coding:
  - **EQ**: blue-gray background, cyan accents
  - **Compressor**: slate or black, orange knobs
  - **Pultec**: brass tones, gold highlights
  - **Dynamic EQ**: steel blue, green accents
  - **Console/Tape**: charcoal or oxide red tones
- Keep GUI interactions performant and audio-thread safe
- See `GUI_DESIGN.md` for complete design specifications

**Key vizia Resources:**
- vizia-plug GitHub: https://github.com/vizia/vizia-plug
- vizia book: https://vizia.dev/
- vizia examples: https://github.com/vizia/vizia/tree/main/examples

## Build Commands

### Core Development
- **Development build**: `cargo build` (core modules)
- **Development build with GUI**: `cargo +nightly build --features "api5500,buttercomp2,pultec,transformer,gui"`
- **Release build**: `cargo build --release`
- **Run tests**: `cargo test`

### Plugin Bundle Creation (Production)
- **RECOMMENDED**: Manual command with minimal environment:
  ```cmd
  set FORCE_SKIA_BINARIES_DOWNLOAD=1
  cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,gui"
  ```
- **Core modules only**: `cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer"`
- **Simplified build script**: `bin\preflight_build_simple.bat`

### Code Quality
- **Format code**: `cargo +nightly fmt` or `pre-commit run rustfmt-nightly --all-files`  
- **Lint**: `cargo clippy --all-targets --all-features`
- **Install pre-commit hooks**: `pre-commit install`

### Important Build Notes
- ✅ Build xtask first: `cargo +nightly build --package xtask`
- ✅ Use minimal environment variables to avoid Skia build conflicts
- ❌ Complex preflight script (`bin\preflight_build.bat`) causes Skia compilation issues
- ❌ Do not set BINDGEN_EXTRA_CLANG_ARGS or CC/CXX environment variables when building GUI

## File Structure

**Core Plugin:**
- `src/lib.rs` - Main plugin entry point with ~75 parameters and module reordering
- `src/api5500.rs` - 5-band semi-parametric EQ (custom implementation)
- `src/buttercomp2.rs` - Airwindows ButterComp2 FFI wrapper  
- `src/pultec.rs` - Custom Pultec EQP-1A style EQ with tube saturation
- `src/dynamic_eq.rs` - 4-band dynamic EQ with frequency-dependent compression
- `src/transformer.rs` - Transformer coloration module (4 vintage models)
- `src/editor.rs` - Professional GUI implementation (temporarily disabled)
- `src/shaping.rs` - Common DSP shaping functions
- `src/spectral.rs` - FFT analysis utilities

**Build System:**
- `cpp/` - FFI wrappers for Airwindows modules
- `xtask/` - Build tooling and bundling scripts
- `build.rs` - C++ compilation for FFI

**Documentation:**
- `AGENTS.md` - Original project specification and agent roles
- `GUI_DESIGN.md` - Complete GUI specifications and design  
- `GEMINI.md` - Code organization notes from other AI assistant

## Recent Development Notes

**Biquad API Compatibility Issues:**
- The biquad crate API has changed - Type enum constructors now require parameters
- `Type::PeakingEQ` → `Type::PeakingEQ(gain_db)` 
- `Type::LowShelf` → `Type::LowShelf(gain_db)`
- `Type::HighShelf` → `Type::HighShelf(gain_db)`
- The `.set_gain()` method has been removed

**Current Build Status:**
- ✅ Core plugin functionality is complete
- 🔧 vizia GUI partially working - uses pre-built Skia binaries approach
- ✅ All biquad API compatibility issues resolved
- 🔧 Missing ninja build dependency preventing final vizia compilation

## Architecture Notes

**Plugin Architecture:**
- Built on NIH-Plug framework with ~75 automation parameters
- 5 DSP modules with configurable processing order
- Lock-free, allocation-free audio processing thread
- FFI wrapper for C++ Airwindows modules via `build.rs`

**Key Dependencies:**
- `nih_plug` - Plugin framework
- `vizia_plug` - vizia GUI integration for NIH-Plug (modern GUI framework)
- `biquad` v0.5.0 - Filter implementations (updated API)
- `fundsp` - DSP utilities
- `realfft` - FFT processing
- `augmented-dsp-filters` - Additional filter implementations
- `idsp` - Integer DSP operations
- `skia-bindings` - Skia graphics library bindings (uses pre-built binaries)
- Custom C++ FFI wrappers in `cpp/`

**Feature Flags:**
- Default features: `api5500`, `buttercomp2`, `pultec`, `transformer`, `gui`
- Optional: `dynamic_eq` (4-band dynamic EQ with hierarchical sub-features)
- Build with specific modules: `cargo build --features "api5500,pultec"`

## Known Issues & Fixes

**CI/CD Pipeline:**
- Bundle command in workflow needs update: use `cargo xtask bundle bus_channel_strip --release` 
- Asset paths may point to directories instead of files
- Test locally: `cargo xtask bundle bus_channel_strip --release && ls -la target/bundled/`

**Biquad API Changes (RESOLVED):**
- Filter constructors now require gain parameter: `Type::PeakingEQ(gain_db)`
- No longer use `.set_gain()` method

**vizia-plug GUI Status (RESOLVED - September 2025):**
- ✅ Successfully integrated vizia-plug for modern GUI framework
- ✅ Fixed dependency configuration in `Cargo.toml` (removed conflicting skia-safe dependency)
- ✅ Updated to nightly Rust toolchain (required by vizia-plug)
- ✅ vizia-plug handles Skia compilation automatically with pre-built binaries
- ✅ Successful VST3 and CLAP bundle creation with GUI enabled
- ✅ Build time significantly reduced (no manual Skia compilation needed)

**vizia Build Configuration:**
- Uses `FORCE_SKIA_BINARIES_DOWNLOAD=1` to download pre-compiled Skia
- Avoids depot_tools, gn compilation, and Git repository state issues
- Pre-built binary URL: `https://github.com/rust-skia/skia-binaries/releases/download/0.84.0/`
- Simplified environment variables for MSVC target detection

**Next Steps:**
1. Install ninja build tool: `sudo apt install ninja-build` (Linux) or use Windows native build
2. Complete vizia GUI build with: `cargo xtask bundle bus_channel_strip --release --features api5500,buttercomp2,transformer,gui`
3. Test VST3 plugin in DAW environment