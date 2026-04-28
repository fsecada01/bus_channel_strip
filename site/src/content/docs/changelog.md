---
title: Changelog
description: Release history and notable changes
---

For full release notes, binaries, and platform-specific archives, see the [GitHub Releases page](https://github.com/fsecada01/bus_channel_strip/releases).

---

## v1.0.0 — 2026-04

Two major workstreams shipped together: **Sheen** (a hidden master-end polish coat) and a **full multi-fx rack UX redesign** that finally makes module reordering feel native.

### Sheen — master-end polish coat

A new pinned DSP stage at the end of the chain (post-Punch, pre-master-gain) that makes the chassis sound finished out of the box. The brushed-brass **API** brand plate in the chassis header is the only front-panel surface — click it to flip into a hidden back view with five sliders.

Always-on by default at research-grounded factory tuning:

- **BODY** — low shelf @ 100 Hz, default +1.0 dB
- **PRESENCE** — peak EQ @ 3 kHz, Q=1.0, default 0.0 dB (transparent at default)
- **AIR** — high shelf @ 14 kHz, Q=0.5, default +1.8 dB
- **WARMTH** — Sonnox Inflator polynomial @ Curve=0, 2× oversampled, default 20% mix
- **WIDTH** — M/S side-only HPF @ 150 Hz + shelf @ 500 Hz, default 50%

Defaults are anchored in three parallel research reports — classic console-bus measurements (SSL G, Neve 33609, API 2500, Studer, Trident), polish-plugin teardowns (Slate Revival, Kush AR-1, Maag EQ4, Sonnox Inflator, Pensado, Vitamin, Ozone, PSP Vintage Warmer, bx_console), and tape + transformer harmonic profiles (Studer A800, Ampex ATR-102, Jensen, Lundahl, Carnhill). See [`docs/SHEEN_MODULE_SPEC.md`](https://github.com/fsecada01/bus_channel_strip/blob/main/docs/SHEEN_MODULE_SPEC.md) for citations and stage-by-stage rationale.

Sheen is **excluded from `global_auto_gain`** — auto-comp on a polish stage defeats its purpose. See the [Sheen module page](/bus_channel_strip/modules/sheen/) for the full reference.

### Multi-fx rack redesign

The seven-slot rack got a ground-up UX overhaul:

- **Native vizia drag-drop** replaces the previous hand-rolled mouse-capture state machine that was silently failing under baseview's Win32 `SetCapture` lifecycle.
- **Swap-or-insert semantics** decided by cursor X within the target slot at release time: left third = insert before, middle = swap, right third = insert after.
- **Live drop-position preview** while dragging — bright cyan bar pinned to the target slot's left or right edge (insert) or full yellow ring around the slot (swap), so you see the resolved drop intent before releasing.
- **Floating drag-ghost label** tracks the cursor showing the dragged module's tag.
- **Empty slots collapse by default** to narrow dashed tabs; the **library sidebar** is now the sole "add module" affordance.
- **Focus mode** — press `1`..`7` to focus a real-module slot (collapses every other slot to a tab); `Esc` exits focus, cancels drag, or closes any open back view.
- **Chain mini-map** appears as a band above the chassis only when a slot is focused, hidden otherwise.
- **Brushed-brass brand plate** is the entry point to the Sheen back view; mutually exclusive with the DynEQ back view.
- **Click-lag fix** — the chassis-level `MouseUp` listener that previously routed every click through the broken drag state machine has been removed.

### Compatibility

11 new automation params (Sheen) on top of the existing ~75 — no reused IDs. Existing DAW sessions load with Sheen ON at factory defaults; playback will subtly differ from pre-1.0 sessions. For bit-identical playback, flip the back-panel master Sheen bypass.

### Other improvements

- `justfile` `FEATURES` and `CORE_FEATURES` lists corrected (`sheen` and `haas` were missing from the bundle commands)
- vizia event-routing documentation: clickable HStacks need `on_press` on each child Label because vizia's `on_press` is leaf-targeted, not bubbling
- Module-level cached params now use `.value()` instead of `.smoothed.next()` — the latter advances 1 sample per call

### Test coverage

- 8 new Sheen unit tests
- 159 lib tests passing
- Verified in Reaper across drum / bass / vocal / full-mix material; rack drag-drop verified with swap, insert-before, insert-after, drag-to-empty-slot, and drag-cancel-by-leaving-window

---

## v0.5.0 — 2026-04

### Pultec EQ — authentic LCR resonance and bandwidth control

The LF section is completely remodeled to match the sound of the original EQP-1A hardware.

- **LCR resonant bump**: A PeakingEQ at the shelf corner frequency (45% of shelf gain, Q=1.8) models the inductor resonance of the original hardware. This resonant peak is why Pultec LF boosts sound focused and punchy rather than soft and woolly.
- **18 dB range** for both LF Boost and LF Cut (extended from ~8 dB).
- **LF Boost Freq**: now spans 20–300 Hz with skewed scaling (was stepped 20/30/60/100 Hz).
- **LF Cut Freq**: now spans 20–400 Hz with skewed scaling (was 20–200 Hz).
- **New: LF Boost Bandwidth** — controls shelf width from narrow (Q=1.0) to wide (Q=0.25). At the default of 0.67, the shelf spreads through the musical 100–300 Hz range. This is the difference between a Pultec that sounds sub-only and one that sounds full and present on guitars, keys, and mix buses.
- **New: LF Cut Bandwidth** — same Q mapping as LF Boost BW.
- All filter math now routes through `shaping::biquad_coeffs` to apply the correct Nyquist normalization (`f0 * 2 / sr`), fixing the biquad 0.5.0 frequency normalization bug that was placing every filter at 1/4 its intended corner frequency.

### New module: Haas stereo widener

A psychoacoustic stereo widener using M/S encoding and Haas effect comb filtering.

- **Two modes**: Side Comb (WOW-Thing style, mono-compatible) and Wide Comb (diffuse, L-R delay injection).
- **Delay time**: 1–20 ms, Hermite-interpolated with one-pole smoothing (τ=20 ms) to prevent zipper noise on automation sweeps.
- **RMS-safe output trim**: automatic gain compensation based on mid/side gains and comb depth.
- **Anti-denormal protection**: Airwindows-style 1e-20 alternating dither in the delay line.
- **Default position**: before Punch — so the clipper catches any widener-induced peaks before they hit the ceiling.
- Feature flag: `haas` (enabled by default).

### Plugin integration tests

New `src/plugin_integration_tests.rs` file exercises the full plugin pipeline rather than isolated module tests. Catches parameter wiring failures, bypass default regressions, and coefficient initialization bugs. Currently covers Pultec module end-to-end.

### CI fix

`src/haas.rs` was missing from the repository (untracked locally). Added to git. The `haas` feature is in `[features] default`, so all CI targets were failing with `E0583: file not found for module 'haas'`.

---

## v0.4.0 — 2026-03

### ButterComp2 — new compression models

Three new compressor models join the original Airwindows Classic algorithm. Select the model from the dropdown at the top of the ButterComp2 module panel; the controls below update to match.

- **VCA** — Hard-knee voltage-controlled amplifier compression. Fast, precise transient response. Threshold (dB), Ratio (1–20), Attack (ms), Release (ms), Character % (0–100, controls 1176-style color), Mix. Use on drum buses and any source where you want predictable, controllable gain reduction.
- **Optical (Opt)** — Soft-knee program-dependent compression. Threshold (dB), Character % (0–100, controls program-dependent release behavior and tube warmth), Attack (ms), Release (ms), Mix. Use on vocal, acoustic instrument, and bass stems.
- **FET** — Field-effect transistor compression driven by an input gain stage rather than a threshold control. Input (dB), Output (dB), Ratio (1–20), Attack (ms), Release (ms), Auto Release (toggle — enables program-dependent release), Mix. Use on drum buses and any stem where you want a forward, saturated character.
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
