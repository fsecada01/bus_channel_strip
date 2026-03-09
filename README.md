<div align="center">

# Bus Channel Strip

**Six modules. One chain. The glue your bus has been missing.**

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org)
[![VST3](https://img.shields.io/badge/VST3-supported-blue.svg)](https://steinbergmedia.github.io/vst3_doc/)
[![CLAP](https://img.shields.io/badge/CLAP-supported-green.svg)](https://cleveraudio.org/)
[![License](https://img.shields.io/badge/license-GPL--3.0-red.svg)](LICENSE)

*Built with [NIH-Plug](https://github.com/robbert-vdh/nih-plug), [Airwindows DSP](https://github.com/airwindows/airwindows), and [vizia](https://vizia.dev/)*

**[Documentation & Presets](https://fsecada01.github.io/bus_channel_strip/)** | **[Download Latest Release](../../releases/latest)**

</div>

---

Bus Channel Strip is a single plugin that replaces six inserts on your master or stem bus. Load it once and run your mix through a console EQ, an Airwindows glue compressor, a passive tube EQ, a dynamic EQ with sidechain support, a vintage transformer stage, and a transparent loudness maximizer — in that order, or in any order you like.

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

## What's New in v0.4.0

- **ButterComp2 gets four compressor models** — Classic (original Airwindows bipolar algorithm), VCA, Optical, and FET. Each model changes the compression character while keeping the signature NY parallel blend intact.
- **API5500 EQ layout redesigned** — LF and HF shelves now sit side by side in a two-column layout. The three parametric bands (LMF, MF, HMF) are ordered low to high, matching the physical console.
- **Pultec HF controls restored** — HF boost bandwidth and HF cut knobs are now fully visible in the GUI.
- **Transformer restructured** — Output saturation is now exposed as a dedicated control. The panel is organized into discrete INPUT, OUTPUT, and TONE sections.
- **Integer parameter display** — All non-percentage parameters now display as clean integers in automation lanes. No more reading `−12.000000 dB` in your DAW.
- **Global bypass** — Zero-latency passthrough toggle for the entire chain.
- **Auto Gain compensation** — RMS-based gain correction with an ~5 second time constant. Keeps the bypassed and processed levels matched.

---

## The Signal Chain

```
[API5500 EQ] -> [ButterComp2] -> [Pultec EQ] -> [Dynamic EQ] -> [Transformer] -> [Punch]
```

Every module can be reordered by clicking its drag handle and swapping it with any other slot. Every module has an individual bypass switch. The chain is fully automatable — all ~90 parameters are exposed to your DAW.

---

## Modules

| Module | Category | What it does to your mix |
|--------|----------|--------------------------|
| **API5500 EQ** — *5-band semi-parametric* | Console EQ | Broad, musical shelving on the lows and highs, three overlapping parametric bands (LMF / MF / HMF) for surgical or broad-brush tonal shaping, and a high-pass filter. Gives the mix the forward, punchy character of a large-format API console. |
| **ButterComp2** — *Airwindows bipolar interleaved* | Glue Compressor | The richest glue compressor in the chain. Chris Johnson's bipolar interleaved algorithm knits elements together without dulling transients. Four models — **Classic** (original Airwindows), **VCA**, **Optical**, and **FET** — give you density with attitude. Built-in NY parallel blend lets you dial in exactly how much cement you pour. |
| **Pultec EQ** — *EQP-1A passive tube* | Tone Shaper | Simultaneous boost and cut on the same low frequency band: the classic Pultec trick for adding weight without muddiness. A tube saturation stage adds the harmonic richness that makes passive EQs sound like they cost more than your monitors. HF boost bandwidth and HF cut controls let you tame or open the top end the way a vintage unit would. |
| **Dynamic EQ** — *4-band frequency-dependent dynamics* | Surgical Dynamics | Compresses, expands, or gates each of four frequency bands independently — only when the level in that band crosses its threshold. A real-time spectral analyzer shows you what's happening while GR meters show how hard each band is working. Optional sidechain input for frequency-targeted ducking or de-essing driven by another signal. |
| **Transformer** — *4 vintage hardware models* | Saturation / Color | Runs your signal through an emulated transformer core in four flavors: **Vintage** (Neve-style iron warmth), **Modern** (API-style punch), **British** (SSL-style clarity and grit), and **American** (custom character). Independent input and output transformer stages let you push the front end hard and tame the output separately. Frequency response shaping from the transformer model is included. |
| **Punch** — *Clipper + transient shaper* | Loudness / Limiting | Final brick in the chain. Hard, Soft, and Cubic clipping modes push into the ceiling while up to 8x oversampling keeps aliasing out of the audible range. A pre-clip transient shaper (attack, sustain, release) lets you sculpt the attack shape before the limiter acts on it — the correct order for transient control without pumping. A parallel Mix knob blends the clipped signal with the dry for NY-style limiting. |

---

## Global Controls

- **Global Bypass** — Engages zero-latency passthrough for the entire chain. Use it for A/B comparisons at a glance.
- **Auto Gain** — RMS-based output compensation (~5 second time constant) that matches the processed and bypassed levels. Turn it on when evaluating processing decisions so the louder signal doesn't win.
- **Module Reordering** — Click the drag handle ( = ) on any module slot and then click another slot's handle to swap positions. The chain processes left to right in the order shown.

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
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
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

- **NIH-Plug** — plugin framework (~90 parameters, VST3 + CLAP output)
- **vizia-plug** — GUI framework (Skia GPU backend, ECS reactive architecture)
- **biquad 0.5.0** — filter implementations
- **Airwindows ButterComp2** — C++ FFI via `extern "C"` wrapper in `cpp/`
- **realfft** — FFT for the spectral analyzer

### Source Layout

```
src/
  lib.rs           # Plugin entry, parameter definitions, process() dispatch
  api5500.rs       # 5-band semi-parametric EQ
  buttercomp2.rs   # Airwindows ButterComp2 FFI wrapper
  pultec.rs        # Pultec EQP-1A tube EQ
  dynamic_eq.rs    # 4-band dynamic EQ
  transformer.rs   # Transformer saturation module
  punch.rs         # Clipper + transient shaper with oversampling
  editor.rs        # vizia GUI
  components.rs    # Reusable GUI components
  spectral.rs      # FFT analysis + gain reduction metering
  shaping.rs       # DSP math utilities
  styles.rs        # vizia CSS-like styles

cpp/               # C++ Airwindows FFI wrappers
xtask/             # Build tooling (bundle, install)
docs/              # Documentation + GitHub Pages site
```

---

## Documentation

Full control reference, genre signal chain examples, and preset descriptions:

**[fsecada01.github.io/bus_channel_strip](https://fsecada01.github.io/bus_channel_strip/)**

Internal design documents in `docs/`:
- `GUI_DESIGN.md` — UI specifications and layout
- `PUNCH_MODULE_SPEC.md` — Punch DSP design and psychoacoustic research
- `CLIPPING_INSIGHTS.md` — Professional loudness techniques

---

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

Airwindows source code is copyright Chris Johnson, released under the MIT license.
