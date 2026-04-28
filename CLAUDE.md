# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

This document provides context and guidelines for AI assistance with the bus channel strip plugin development.

# Extended AI Session Context
@docs/SYSTEM_PROMPT.md

## Project Overview

A multi-module bus channel strip VST plugin built with NIH-Plug and Airwindows-based DSP modules in Rust. **Currently at v1.0.0** (see GitHub releases for v1.0.0 notes).

**Signal Flow**: `[API5500 EQ] → [ButterComp2] → [Pultec EQ] → [Dynamic EQ] → [Transformer] → [Haas] → [Punch] → [Sheen]`

The first seven modules occupy reorderable slots driven by the `module_order_*` params. **Sheen** is pinned to the master end of the chain (post-Punch, pre-master-gain) and is not a slot module — it's a chassis-level "polish coat" exposed only via the brushed-brass brand plate that flips into a hidden back view.

**Current Status (v1.0.0)**:
- ✅ ALL 7 SLOT MODULES + SHEEN POLISH STAGE IMPLEMENTED
- ✅ MULTI-FX RACK REDESIGN: native vizia drag-drop, swap-or-insert hit-test, live drop preview, floating ghost label, focus mode (1-7 / Esc), library sidebar as sole add path
- ✅ BRUSHED-BRASS BRAND PLATE → SHEEN BACK VIEW (mutually exclusive with DynEQ back view)
- ✅ ~86 AUTOMATION PARAMETERS
- ✅ LOCAL BUILD, BUNDLE, AND DEPLOY WORKING
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
- **RECOMMENDED**: `just bundle` — uses the `FEATURES` var (api5500,buttercomp2,pultec,transformer,punch,haas,dynamic_eq,sheen,gui) and handles env vars automatically.
- **Manual full-feature command**:
  ```cmd
  set LLVM_HOME=C:\Program Files\LLVM
  set LIBCLANG_PATH=C:\Program Files\LLVM\bin
  cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,haas,dynamic_eq,sheen,gui"
  ```
- **Core modules only (no GUI, fast iteration)**: `just bundle-core` — same feature list minus `gui`

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
- `src/lib.rs` - Main plugin entry point (~86 parameters, slot reordering, master-end Sheen dispatch)
- `src/api5500.rs` - 5-band semi-parametric EQ (custom implementation)
- `src/buttercomp2.rs` - Airwindows ButterComp2 FFI wrapper
- `src/pultec.rs` - Custom Pultec EQP-1A style EQ with tube saturation
- `src/dynamic_eq.rs` - 4-band dynamic EQ with frequency-dependent compression
- `src/transformer.rs` - Transformer coloration module (4 vintage models)
- `src/haas.rs` - Psychoacoustic stereo widener (M/S encoding + Haas effect comb filtering, two modes)
- `src/punch.rs` - Clipper + Transient Shaper module (hard/soft/cubic clip, 8x oversampling, transient detection)
- `src/sheen.rs` - **Pinned master-end "polish coat"** — 5 stages (BODY low shelf, PRESENCE peak, AIR high shelf, WARMTH Sonnox Inflator polynomial @ 2× oversample, WIDTH M/S side-only). Not a slot module. Default-on at factory tuning.
- `src/editor.rs` - vizia GUI: chassis header + brass plate, library sidebar, scrollable rack with native drag-drop + live drop preview + floating ghost, DynEQ back view, Sheen back view (mutually exclusive)
- `src/components.rs` - Reusable vizia UI components
- `src/styles.rs` - CSS-like styling for vizia GUI (includes brass plate + Sheen back view themes)
- `src/shaping.rs` - Common DSP shaping functions and `biquad_coeffs` helper that works around the biquad 0.5.0 frequency-normalization bug
- `src/spectral.rs` - FFT analysis utilities

**Build System:**
- `cpp/` - FFI wrappers for Airwindows modules
- `xtask/` - Build tooling and bundling scripts
- `build.rs` - C++ compilation for FFI
- `justfile` - Recipes (`check`, `build`, `bundle`, `install`, `deploy`, `qa`); `FEATURES` and `CORE_FEATURES` are the canonical feature lists used by every recipe

**Documentation:**
- `docs/SYSTEM_PROMPT.md` - Extended AI session context (orchestration protocol, audio-thread rules, code standards)
- `docs/SHEEN_MODULE_SPEC.md` - Sheen module DSP spec, factory-default rationale, citations from the three research reports
- `docs/MULTI_FX_UI_DESIGN.md` - Rack UX design (consolidation pass + drag-drop redesign with hit-test semantics)
- `docs/AGENTS.md` - Original project specification and agent roles
- `docs/GUI_DESIGN.md` - Complete GUI specifications and design
- `docs/PUNCH_MODULE_SPEC.md` - Punch module DSP specification and psychoacoustic research
- `docs/VIZIA_AGENT_SPEC.md` - vizia GUI specialist agent specification
- `docs/CLIPPING_INSIGHTS.md` - Professional loudness techniques research
- `docs/buttercomp2_analysis.md` - ButterComp2 FFI analysis

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
- Built on NIH-Plug framework with ~86 automation parameters
- 7 reorderable slot modules + 1 pinned master-end module (Sheen)
- Lock-free, allocation-free audio processing thread
- FFI wrapper for C++ Airwindows modules via `build.rs`
- GUI uses vizia's native drag-drop API (`on_drag` / `on_drop`) — the previous hand-rolled `on_press_down` capture state machine was silently failing under baseview's Win32 `SetCapture` lifecycle (vizia#407)

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
- Default features: `api5500`, `buttercomp2`, `pultec`, `transformer`, `punch`, `haas`, `dynamic_eq`, `sheen`
- `gui` is NOT in defaults (kept opt-in so CI builds without GUI don't compile Skia for nothing); the justfile `FEATURES` recipe variable adds it for `bundle` / `deploy`
- Sheen is a default feature because it's part of the chassis identity (always present in v1.0.0+)
- Build with specific modules: `cargo build --features "api5500,pultec,punch"`

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

**vizia Build Configuration (Windows):**
- ✅ Requires LLVM/Clang 19+ for MSVC STL compatibility
- ✅ Set `LLVM_HOME=C:\Program Files\LLVM` and `LIBCLANG_PATH=C:\Program Files\LLVM\bin`
- ✅ skia-bindings 0.84.0 builds from source on Windows (no x86_64 pre-built binaries available)
- ✅ Use `cargo +nightly` for vizia-plug compatibility

**Successful Build Command (Windows):**
```cmd
set LLVM_HOME=C:\Program Files\LLVM
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,haas,gui"
```

**Or use the build script:** `bin\preflight_build_simple.bat`