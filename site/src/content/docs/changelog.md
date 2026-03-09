---
title: Changelog
description: Release history and notable changes
---

For full release notes, binaries, and platform-specific archives, see the [GitHub Releases page](https://github.com/fsecada01/bus_channel_strip/releases).

---

## v0.4.0 — 2026-03

### ButterComp2 — new compression models

Three new compressor models join the original Airwindows Classic algorithm. Select the model from the dropdown at the top of the ButterComp2 module panel; the controls below update to match.

- **VCA** — Hard-knee voltage-controlled amplifier compression. Fast, precise transient response. Threshold (dB), Ratio (1–20), Attack (ms), Release (ms), Character % (0–100, controls 1176-style color), Mix. Use on drum buses and any source where you want predictable, controllable gain reduction.
- **Optical (Opt)** — Soft-knee program-dependent compression. Threshold (dB), Character % (0–100, controls program-dependent release behavior and tube warmth), Attack (ms), Release (ms), Mix. Use on vocals, acoustic instruments, and bass.
- **FET** — Field-effect transistor compression driven by an input gain stage rather than a threshold control. Input (dB), Output (dB), Ratio (1–20), Attack (ms), Release (ms), Auto Release (toggle — enables program-dependent release), Mix. Use on snares, room mics, and any bus where you want forward, saturated character.
- **Classic** — Airwindows ButterComp2 (original, unchanged). Bipolar interleaved compression with Compress, Output, and Dry/Wet. VCA, Optical, and FET are native Rust implementations; Classic compiles from Airwindows C++ via Rust FFI.

### Parameter display — integer formatting

All non-percentage float parameters now display as integers throughout the UI, automation lanes, and parameter tooltips. This eliminates long decimal strings on thresholds, ratios, EQ gains, and timing values. Existing automation data is unaffected — the underlying parameter ranges are unchanged.

### UI — API5500 EQ layout

- LF and HF shelves moved to a two-column layout, freeing vertical space in the module slot
- Parametric bands now ordered low-to-high: LMF, MF, HMF

### UI — Pultec EQ

- HF boost bandwidth control now visible in the panel
- HF cut frequency and gain controls now visible

### UI — Transformer

- Output saturation control exposed
- Panel sections structured as INPUT / OUTPUT / TONE for clarity

### UI — ButterComp2 model panel

- Model selector dropdown at top of module
- Per-model control panel updates via `Binding::new` pattern — controls swap without resizing the module slot
- Fixed-height panel prevents adjacent modules from shifting when switching models

### DSP — global bypass

- Global bypass now passes audio through with zero processing overhead via early return from `process()`
- Previously, bypass still evaluated the module order dispatch loop

### DSP — global auto gain

- New global Auto Gain compensation: RMS level measured pre- and post-processing chain
- ~5-second smoothing time constant; ±18 dB correction range
- Compensates for level changes introduced by heavy compression or EQ boost without manual output trimming

### Fix — NaN sentinel values

- Replaced NaN-based dirty-check sentinels in the compressor model switching logic
- Previously, switching models could leave NaN values in internal state, causing audio silence until the plugin was reset

---

## v0.3.0 — 2026-03

### Documentation site
- Migrated documentation from MkDocs to **Astro 5 + Starlight 0.32** for improved navigation, search, and theming
- Added Architecture, Contributing, Changelog, and Parameter Reference pages
- Live site deployed to GitHub Pages at [fsecada01.github.io/bus_channel_strip](https://fsecada01.github.io/bus_channel_strip/)

### CI/CD pipeline fixes
- Replaced deprecated `actions/create-release@v1` with `gh release create` and `GITHUB_TOKEN` permissions
- Fixed Skia cross-compile failure on macOS ARM64 runners: Xcode's clang on `aarch64` blocks x86 SIMD intrinsic headers (`mmintrin.h`, `emmintrin.h`) even when targeting `x86_64-apple-darwin`. Fix: ARM64 builds include GUI, Intel builds compile core modules only without Skia
- Removed `gui` from `[features] default` in `Cargo.toml` to prevent Skia from compiling on all CI targets regardless of `--features` flags passed to `xtask bundle`
- Used `actions/download-artifact@v4` with `merge-multiple: true` for flat artifact glob upload to releases

### Release artifacts (4 platform targets)
- `windows-x86_64` — VST3 + CLAP with GUI (Skia built from source with LLVM 19)
- `linux-x86_64` — VST3 + CLAP with GUI
- `macos-aarch64` (Apple Silicon) — VST3 + CLAP with GUI
- `macos-x86_64` (Intel) — VST3 + CLAP, **core modules only, no GUI** (Skia cross-compile limitation, tracked as Issue #1)

---

## v0.2.x

### Punch module
- New **Punch** module: clipper + transient shaper with 8x oversampling
- Three clip modes: Hard, Soft (tanh), Cubic (polynomial soft knee)
- Transient detection with configurable attack/release times and sensitivity
- **Pumping fix**: moved transient detection + shaping to pre-clip stage. Previously, post-clip transient shaping created time-varying gain modulation that audibly pumped on every note attack
- Downsample IIR pole reduced from 0.3 → 0.05, further eliminating pumping artifacts
- Transient detector now uses native sample rate (not the oversampled rate)
- Punch module is bypassed by default — user must enable it intentionally

### Module reordering UI
- Drag-to-swap module ordering implemented in the vizia GUI
- Click the drag handle (≡) to select a slot as swap source
- Click another slot's handle to swap positions
- Visual feedback: white border + yellow module name on selected slot
- Handle label changes reactively: "MOVE" (self) / "SWAP HERE" (others) / "CANCEL"

### vizia-plug GUI integration
- Replaced placeholder GUI with full **vizia-plug** integration (September 2025)
- Fixed dependency configuration: removed conflicting `skia-safe` direct dependency
- Switched to nightly Rust toolchain (required by vizia-plug)
- Skia builds from source on Windows x86_64 using LLVM 19 + MSVC STL
- Window: 1820×820 px, six module slots with per-module color coding
- Spectrum analyzer canvas with real-time FFT display, band tint overlays, and GR bars

### Dynamic EQ — sidechain masking analysis
- Optional stereo sidechain input (second audio I/O layout in CLAP/VST3)
- Lock-free one-shot analysis pipeline: GUI arms analysis, audio thread runs FFT on sidechain snapshot, results delivered via `AtomicBool` + `AnalysisResult`
- Spectral overlap detection: `overlap[k] = main_fft_mag[k] * sc_fft_mag[k]`
- Suggests threshold and target frequency for masking band
- Two-step UX: "ANALYZE SC" arms; "APPLY RESULT" sets DynEQ parameters

---

## v0.1.x

### Initial implementation
- Core plugin skeleton with NIH-Plug framework
- **API5500 EQ**: 5-band semi-parametric (LF shelf, LMF/MF/HMF parametric, HF shelf)
- **ButterComp2**: Airwindows C++ module wrapped via `extern "C"` FFI, compiled with `cc` crate
- **Pultec EQ**: EQP-1A style with simultaneous boost/cut and tube saturation (`tanh` shaping)
- **Transformer**: 4 vintage transformer models (Vintage, Iron, Modern, Warm) with input/output saturation
- **Dynamic EQ**: 4-band frequency-dependent compression with configurable mode (compress downward / expand upward)
- Module reordering system: six `EnumParam<ModuleType>` parameters for runtime signal chain ordering
- Lock-free audio thread: no allocations, no locks, no panics in `process()`
- Migrated from `biquad` v0.4 to v0.5.0 API: filter constructors now require gain parameter (`Type::PeakingEQ(gain_db)`, `Type::LowShelf(gain_db)`, `Type::HighShelf(gain_db)`); `.set_gain()` removed
- Sample-accurate automation enabled (`SAMPLE_ACCURATE_AUTOMATION: bool = true`)
