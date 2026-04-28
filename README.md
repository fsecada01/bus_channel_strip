<div align="center">

# Bus Channel Strip

**Seven modules. One chain. A hidden polish coat at the end. The glue your bus has been missing.**

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org)
[![VST3](https://img.shields.io/badge/VST3-supported-blue.svg)](https://steinbergmedia.github.io/vst3_doc/)
[![CLAP](https://img.shields.io/badge/CLAP-supported-green.svg)](https://cleveraudio.org/)
[![License](https://img.shields.io/badge/license-GPL--3.0-red.svg)](LICENSE)

*Built with [NIH-Plug](https://github.com/robbert-vdh/nih-plug), [Airwindows DSP](https://github.com/airwindows/airwindows), and [vizia](https://vizia.dev/)*

**[Documentation & Presets](https://fsecada01.github.io/bus_channel_strip/)** | **[Download Latest Release](../../releases/latest)**

</div>

---

Bus Channel Strip is a single plugin that replaces eight inserts on your master or stem bus. Load it once and run your mix through a console EQ, an Airwindows glue compressor, a passive tube EQ, a dynamic EQ with sidechain support, a vintage transformer stage, a psychoacoustic stereo widener, a transparent loudness maximizer, and a hidden master-end **polish coat** — in that order, or in any order you like (the first seven are reorderable; the polish coat is pinned at the end).

Every module is individually bypassable and fully automatable. Every parameter reads as a clean integer in your DAW's automation lanes. The signal chain has a global bypass for zero-latency passthrough and RMS-based Auto Gain compensation so level differences don't fool your ears when you're comparing.

---

## Download

Go to [**Releases**](../../releases/latest) and grab the archive for your platform:

| Platform | File |
|----------|------|
| Windows (x64) | `Bus-Channel-Strip-windows.zip` |
| macOS Apple Silicon | `Bus-Channel-Strip-macos-arm64.tar.gz` |
| macOS Intel | `Bus-Channel-Strip-macos-intel.tar.gz` |
| Linux (x64) | `Bus-Channel-Strip-linux.tar.gz` |

**Install paths:**

| Format | Windows | macOS | Linux |
|--------|---------|-------|-------|
| VST3 | `C:\Program Files\Common Files\VST3\` | `~/Library/Audio/Plug-Ins/VST3/` | `~/.vst3/` |
| CLAP | `C:\Program Files\Common Files\CLAP\` | `~/Library/Audio/Plug-Ins/CLAP/` | `~/.clap/` |

> **macOS Intel note:** The Intel build ships without the GUI (Skia cross-compile limitation on Apple Silicon runners). All DSP is present and functional; use the ARM64 build if you need the visual interface.

---

## What's New in v1.0.0

Two major workstreams ship together: **Sheen** (a hidden master-end polish coat) and a **full multi-fx rack UX redesign** that finally makes module reordering feel native.

### Sheen — master-end polish coat

A new pinned DSP stage at the end of the chain (post-Punch, pre-master-gain) that makes the chassis sound finished out of the box. The brushed-brass **API** brand plate in the chassis header is the only front-panel surface — click it to flip into a hidden back view with five sliders. Always-on by default at research-grounded factory tuning:

| Stage | Algorithm | Factory default |
|---|---|---|
| **BODY** | Low shelf @ 100 Hz | +1.0 dB |
| **PRESENCE** | Peak EQ @ 3 kHz, Q=1.0 | 0.0 dB (transparent) |
| **AIR** | High shelf @ 14 kHz, Q=0.5 | +1.8 dB |
| **WARMTH** | Sonnox Inflator polynomial @ Curve=0, 2× oversampled | 20% mix |
| **WIDTH** | M/S side-only: HPF @ 150 Hz + shelf @ 500 Hz | +12.5% sides above 500 Hz, mono <150 Hz |

Defaults are anchored in three parallel research reports — classic console-bus measurements (SSL G, Neve 33609, API 2500, Studer, Trident), polish-plugin teardowns (Slate Revival, Kush AR-1, Maag EQ4, Sonnox Inflator, Pensado, Vitamin, Ozone, PSP Vintage Warmer, bx_console), and tape + transformer harmonic profiles (Studer A800, Ampex ATR-102, Jensen, Lundahl, Carnhill). See `docs/SHEEN_MODULE_SPEC.md` for citations and stage-by-stage rationale. Excluded from `Auto Gain` — auto-comp on a polish stage defeats its purpose.

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

---

## The Signal Chain

```
[API5500 EQ] -> [ButterComp2] -> [Pultec EQ] -> [Dynamic EQ] -> [Transformer] -> [Haas] -> [Punch] -> [Sheen]
```

The first seven modules occupy reorderable slots — drag any module's body to swap it with another slot, insert it before, or insert it after, with a live cyan/yellow drop indicator showing where it'll land. **Sheen** is pinned at the master end (post-Punch, pre-master-gain); it's the chassis-level "polish coat" exposed via the brushed-brass brand plate. Every module is individually bypassable. The chain is fully automatable — all ~86 parameters are exposed to your DAW.

---

## Modules

| Module | Category | What it does to your mix |
|--------|----------|--------------------------|
| **API5500 EQ** — *5-band semi-parametric* | Console EQ | Broad, musical shelving on the lows and highs, three overlapping parametric bands (LMF / MF / HMF) for surgical or broad-brush tonal shaping, and a high-pass filter. Gives the mix the forward, punchy character of a large-format API console. |
| **ButterComp2** — *Airwindows bipolar interleaved* | Glue Compressor | The richest glue compressor in the chain. Chris Johnson's bipolar interleaved algorithm knits elements together without dulling transients. Four models — **Classic** (original Airwindows), **VCA**, **Optical**, and **FET** — give you density with attitude. Built-in NY parallel blend lets you dial in exactly how much cement you pour. |
| **Pultec EQ** — *EQP-1A passive tube* | Tone Shaper | Simultaneous boost and cut on the same low frequency band: the classic Pultec trick for adding weight without muddiness. An authentic LCR resonant bump at the shelf corner models the original hardware's inductor resonance. LF Boost and Cut up to 18 dB each with independent bandwidth controls. Tube saturation adds harmonic richness. |
| **Dynamic EQ** — *4-band frequency-dependent dynamics* | Surgical Dynamics | Compresses, expands, or gates each of four frequency bands independently — only when the level in that band crosses its threshold. A real-time spectral analyzer shows you what's happening while GR meters show how hard each band is working. Optional sidechain input for frequency-targeted ducking or de-essing driven by another signal. |
| **Transformer** — *4 vintage hardware models* | Saturation / Color | Runs your signal through an emulated transformer core in four flavors: **Vintage** (Neve-style iron warmth), **Modern** (API-style punch), **British** (SSL-style clarity and grit), and **American** (custom character). Independent input and output transformer stages let you push the front end hard and tame the output separately. Frequency response shaping from the transformer model is included. |
| **Haas** — *Psychoacoustic stereo widener* | Stereo Width | M/S encoding with independent mid/side gain, then Haas effect comb filtering in two modes: **Side Comb** (mono-compatible, WOW-Thing style) or **Wide Comb** (diffuse L-R delay injection). Hermite interpolation keeps automation smooth and click-free. RMS-safe automatic output trim. Positioned before Punch so the clipper catches any widener-induced peaks. |
| **Punch** — *Clipper + transient shaper* | Loudness / Limiting | Final brick in the reorderable chain. Hard, Soft, and Cubic clipping modes push into the ceiling while up to 8x oversampling keeps aliasing out of the audible range. A pre-clip transient shaper (attack, sustain, release) lets you sculpt the attack shape before the limiter acts on it — the correct order for transient control without pumping. A parallel Mix knob blends the clipped signal with the dry for NY-style limiting. |
| **Sheen** — *Pinned master-end polish coat* | Polish / Glue | Hidden behind the brushed-brass brand plate in the chassis header. Five always-on stages applied in series at research-grounded factory tuning: low-shelf body, presence peak, air shelf, Sonnox-Inflator-style harmonic warmth (2× oversampled), and frequency-dependent M/S width. Click the plate to open the back view and tune; click `↺ RESTORE FACTORY` to revert. Excluded from Auto Gain by design. |

---

## Global Controls

- **Global Bypass** — Engages zero-latency passthrough for the entire chain (including Sheen). Use it for A/B comparisons at a glance.
- **Auto Gain** — RMS-based output compensation (~5 second time constant) that matches the processed and bypassed levels. Sheen is intentionally excluded from this calculation — auto-comp on a polish stage defeats its purpose.
- **Module Reordering** — Click and drag any reorderable slot's body to a new position. Drop in the **left third** of a target to insert before, the **middle** to swap, the **right third** to insert after. A live cyan bar (insert) or yellow ring (swap) shows the resolved drop intent before you release. Drop on an empty slot to move there. A floating ghost label tracks the cursor showing what you're moving.
- **Focus Mode** — Press `1`..`7` to focus a real-module slot (collapses every other slot to a tab so the focused module gets the full chassis width). Press `Esc` to exit.
- **Brushed-Brass Plate** — The "API Bus Channel Strip" brand mark in the chassis header is clickable; it opens the hidden Sheen back view. Mutually exclusive with the Dynamic EQ back view.

---

## Build From Source

### Requirements

| Dependency | Version | Notes |
|------------|---------|-------|
| Rust nightly | latest | Required by vizia-plug |
| LLVM / Clang | 19+ | Windows only — required for Skia bindgen |
| VS Build Tools | 2022 | Windows only — C++ FFI compilation |

### Quick Commands

```bash
just check        # Fast type-check (no codegen)
just bundle       # Production VST3 + CLAP bundle with GUI
just deploy       # bundle + install to system plugin directories
just qa           # fmt-check + lint + test
```

### Windows — Full Bundle Command

```cmd
set LLVM_HOME=C:\Program Files\LLVM
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,haas,dynamic_eq,sheen,gui"
```

Bundles output to `target/bundled/`.

**Important:** Do not set `BINDGEN_EXTRA_CLANG_ARGS`, `CC`, or `CXX` when building with the `gui` feature — they conflict with Skia's build system. Windows builds Skia from source; LLVM 19+ is required for MSVC STL compatibility.

### All Justfile Recipes

```bash
just check        # Fast type-check
just build        # Debug build (no GUI)
just build-gui    # Debug build with GUI
just bundle       # Production VST3+CLAP bundle
just bundle-core  # Bundle without GUI (faster iteration)
just install      # Install to system plugin dirs
just deploy       # bundle + install
just test         # Unit tests
just lint         # Clippy -D warnings
just fmt          # nightly rustfmt
just qa           # fmt-check + lint + test
just env          # Show build environment
```

---

## Architecture Notes

### Audio Thread Guarantees

All `process()` paths are allocation-free, lock-free, panic-free, and I/O-free. No heap allocation, no mutexes, no `.unwrap()`, no file or system calls. Parameter communication between the GUI and the audio thread uses atomics only.

### Implementation Details

- Biquad filter coefficients update via `update_coefficients()` — no state reset on parameter changes
- ButterComp2 FFI is called once per buffer, not once per sample
- Dynamic EQ uses a 0.05 dB hysteresis gate on coefficient updates to skip trigonometric calls when the envelope is stable
- Transformer parameter caching gates `update_frequency_response()` to actual changes only
- Punch oversampling uses linear interpolation upsample and IIR downsample (pole = 0.05); transient shaping runs pre-clip to prevent pumping

### Technology Stack

- **NIH-Plug** — plugin framework (~86 parameters, VST3 + CLAP output)
- **vizia-plug** — GUI framework (Skia GPU backend, ECS reactive architecture); v1.0.0 uses vizia's native `on_drag` / `on_drop` API for rack drag-drop
- **biquad 0.5.0** — filter implementations (routed through `shaping::biquad_coeffs` to work around the v0.5.0 frequency-normalization bug)
- **Airwindows ButterComp2** — C++ FFI via `extern "C"` wrapper in `cpp/`
- **realfft** — FFT for the spectral analyzer

### Source Layout

```
src/
  lib.rs           # Plugin entry, parameter definitions, process() dispatch (slot loop + Sheen tail)
  api5500.rs       # 5-band semi-parametric EQ
  buttercomp2.rs   # Airwindows ButterComp2 FFI wrapper
  pultec.rs        # Pultec EQP-1A tube EQ
  dynamic_eq.rs    # 4-band dynamic EQ
  transformer.rs   # Transformer saturation module
  haas.rs          # Psychoacoustic stereo widener (M/S + Haas comb)
  punch.rs         # Clipper + transient shaper with oversampling
  sheen.rs         # Pinned master-end polish coat (5 stages, default-on)
  editor.rs        # vizia GUI: chassis header + brass plate + library sidebar +
                   #   scrollable rack with native drag-drop / live drop preview /
                   #   floating ghost / focus mode / mini-map / DynEQ + Sheen back views
  components.rs    # Reusable GUI components
  spectral.rs      # FFT analysis + gain reduction metering
  shaping.rs       # DSP math utilities + biquad_coeffs workaround
  styles.rs        # vizia CSS-like styles (includes brass plate + Sheen back-view themes)

cpp/               # C++ Airwindows FFI wrappers
xtask/             # Build tooling (bundle, install)
docs/              # Documentation + GitHub Pages site (includes SHEEN_MODULE_SPEC.md
                   #   and MULTI_FX_UI_DESIGN.md)
```

---

## Documentation

Full control reference, genre signal chain examples, and preset descriptions:

**[fsecada01.github.io/bus_channel_strip](https://fsecada01.github.io/bus_channel_strip/)**

Internal design documents in `docs/`:
- `SHEEN_MODULE_SPEC.md` — Sheen DSP design with research citations and factory-default rationale
- `MULTI_FX_UI_DESIGN.md` — Rack UX design: consolidation pass + drag-drop redesign with hit-test semantics
- `GUI_DESIGN.md` — UI specifications and layout
- `PUNCH_MODULE_SPEC.md` — Punch DSP design and psychoacoustic research
- `CLIPPING_INSIGHTS.md` — Professional loudness techniques

---

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

Airwindows source code is copyright Chris Johnson, released under the MIT license.
