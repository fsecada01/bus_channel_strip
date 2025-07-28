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
- 🔧 GUI temporarily disabled (egui API compatibility issues)
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
- Built with `egui`
- Module color coding:
  - **EQ**: blue-gray background, cyan accents
  - **Compressor**: slate or black, orange knobs
  - **Pultec**: brass tones, gold highlights
  - **Dynamic EQ**: steel blue, green accents
  - **Console/Tape**: charcoal or oxide red tones
- Keep GUI interactions performant and audio-thread safe

## Build Commands

- **Development build**: `cargo build`
- **Release build**: `cargo build --release`
- **Run tests**: `cargo test`
- **Bundle plugin**: `cargo xtask bundle --release` (creates VST3 and CLAP in `target/bundled/`)
- **Format code**: `cargo fmt` or `pre-commit run fmt --all-files`  
- **Lint**: `cargo clippy --all-targets --all-features` (manual run recommended due to upstream issues)

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
- 🔧 GUI temporarily disabled due to egui API compatibility issues  
- ✅ All biquad API compatibility issues resolved

## Architecture Notes

**Plugin Architecture:**
- Built on NIH-Plug framework with ~75 automation parameters
- 5 DSP modules with configurable processing order
- Lock-free, allocation-free audio processing thread
- FFI wrapper for C++ Airwindows modules via `build.rs`

**Key Dependencies:**
- `nih_plug` - Plugin framework
- `biquad` v0.5.0 - Filter implementations (updated API)
- `fundsp` - DSP utilities
- `realfft` - FFT processing
- Custom C++ FFI wrappers in `cpp/`

## Known Issues & Fixes

**CI/CD Pipeline:**
- Bundle command in workflow needs update: use `cargo xtask bundle --release` 
- Asset paths may point to directories instead of files
- Test locally: `cargo xtask bundle --release && ls -la target/bundled/`

**Biquad API Changes (RESOLVED):**
- Filter constructors now require gain parameter: `Type::PeakingEQ(gain_db)`
- No longer use `.set_gain()` method

**GUI Status:**
- Temporarily disabled due to egui API compatibility
- All GUI code remains in `src/editor.rs` for future re-enabling