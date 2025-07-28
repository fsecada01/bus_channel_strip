# CLAUDE.md

This document provides context and guidelines for AI assistance with the bus channel strip plugin development.

## Project Overview

A multi-module bus channel strip VST plugin built with NIH-Plug and Airwindows-based DSP modules in Rust.

**Signal Flow**: `[API5500 EQ] → [ButterComp2] → [Pultec EQ] → [Dynamic EQ] → [Transformer]`

**Current Status**: ALL 5 CORE MODULES COMPLETE! 🎉

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

- Build: `cargo build`
- Run tests: `cargo test`
- Bundle plugin: `cargo xtask bundle <plugin_name>`

## File Structure

- `src/lib.rs` - Main plugin entry point
- `src/api5500.rs` - EQ module implementation
- `cpp/` - FFI wrappers for Airwindows modules
- `xtask/` - Build tooling and bundling scripts