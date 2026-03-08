<div align="center">

# Bus Channel Strip

**A professional 6-module bus channel strip VST3/CLAP plugin built with Rust**

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org)
[![VST3](https://img.shields.io/badge/VST3-%E2%9C%93-blue.svg)](https://steinbergmedia.github.io/vst3_doc/)
[![CLAP](https://img.shields.io/badge/CLAP-%E2%9C%93-green.svg)](https://cleveraudio.org/)
[![License](https://img.shields.io/badge/license-GPL--3.0-red.svg)](LICENSE)

*Built with [NIH-Plug](https://github.com/robbert-vdh/nih-plug), [Airwindows DSP](https://github.com/airwindows/airwindows), and [vizia](https://vizia.dev/)*

**[Documentation & Presets Site](https://fsecada01.github.io/bus_channel_strip/)** | **[Download Latest](../../releases/latest)**

</div>

---

## Signal Chain

```
[API5500 EQ] -> [ButterComp2] -> [Pultec EQ] -> [Dynamic EQ] -> [Transformer] -> [Punch]
```

All modules are reorderable via drag-to-swap handles in the GUI. Each module has an individual bypass switch and is fully automatable.

## Modules

| Module | Type | Description |
|--------|------|-------------|
| **API5500 EQ** | Semi-Parametric EQ | 5-band console EQ with API 5500 character |
| **ButterComp2** | Compressor (FFI) | Airwindows bipolar interleaved glue compressor |
| **Pultec EQ** | Passive Tube EQ | EQP-1A style with simultaneous boost/cut and tube saturation |
| **Dynamic EQ** | Dynamic Processor | 4-band frequency-dependent dynamics with spectral analyzer |
| **Transformer** | Saturation | Input/output transformer coloration, 4 vintage hardware models |
| **Punch** | Clipper + Transient | Transparent peak clipper with transient shaper, up to 8x oversampling |

## Status

| Component | Status |
|-----------|--------|
| Core DSP (6 modules) | Complete |
| ~90 automation parameters | Complete |
| Module reordering system | Complete |
| vizia-plug GUI | Complete |
| VST3 / CLAP bundle | Complete |
| CI/CD pipeline | Working |
| GitHub Pages docs site | Complete |

## Quick Start

### Download Binaries

Go to [Releases](../../releases/latest) and download for your platform:
- **Windows**: `Bus-Channel-Strip-windows.zip`
- **macOS Intel**: `Bus-Channel-Strip-macos-intel.tar.gz`
- **macOS ARM64**: `Bus-Channel-Strip-macos-arm64.tar.gz`
- **Linux**: `Bus-Channel-Strip-linux.tar.gz`

**VST3 paths:**
- Windows: `C:\Program Files\Common Files\VST3\`
- macOS: `~/Library/Audio/Plug-Ins/VST3/`
- Linux: `~/.vst3/`

---

## Build From Source

### Requirements

| Dependency | Version | Notes |
|------------|---------|-------|
| Rust nightly | latest | Required by vizia-plug |
| LLVM/Clang | 19+ | Windows: required for Skia bindgen |
| Visual Studio Build Tools | 2022 | Windows: C++ FFI compilation |

### Build Commands

```bash
# Fast type-check (no codegen)
just check

# Production bundle: VST3 + CLAP with GUI
just bundle

# Bundle + install to system plugin directory
just deploy

# All quality checks (fmt + lint + test)
just qa
```

**Windows — manual bundle command:**
```cmd
set LLVM_HOME=C:\Program Files\LLVM
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
```

Bundles output to `target/bundled/`.

### Important Build Notes

- Use `cargo +nightly` for all GUI-enabled builds (vizia-plug requirement)
- Do **not** set `BINDGEN_EXTRA_CLANG_ARGS` or `CC`/`CXX` when building with GUI
- `FORCE_SKIA_BINARIES_DOWNLOAD=1` can be used if pre-built Skia binaries are available for your platform
- Windows x86_64 builds Skia from source; LLVM 19+ is required for MSVC STL compatibility

---

## Architecture

### Audio Thread Rules (Non-Negotiable)

All `process()` paths are:
- **Allocation-free** — no `Vec::new()`, `Box::new()`, `String`, or heap allocation
- **Lock-free** — no `Mutex`, `RwLock`, or blocking sync primitives
- **Panic-free** — no `.unwrap()`, `.expect()`, or unguarded indexing
- **I/O-free** — no file access, logging, or system calls

### Key Patterns

- Biquad filter coefficients updated via `update_coefficients()` — no state reset on parameter changes
- ButterComp2 FFI: called once per buffer (O(1) FFI overhead), not once per sample
- Dynamic EQ: 0.05 dB hysteresis gate on coefficient updates to skip trig calls when envelope is stable
- Transformer: parameter value caching gates `update_frequency_response()` to actual changes only
- Punch oversampling: linear interpolation upsample, IIR downsample (pole=0.05), transient shaping pre-clip

### Technology Stack

- **NIH-Plug** — plugin framework (~90 parameters, VST3 + CLAP)
- **vizia-plug** — GUI framework (Skia-backed, ECS reactive architecture)
- **biquad 0.5.0** — filter implementations (update_coefficients() API)
- **Airwindows ButterComp2** — C++ FFI via `extern "C"` wrapper in `cpp/`
- **realfft** — FFT for spectral analyzer
- **cc** crate — C++ compilation in `build.rs`

---

## Documentation

Full control reference, settings examples, genre signal chains, and instrument bus presets:

**[fsecada01.github.io/bus_channel_strip](https://fsecada01.github.io/bus_channel_strip/)**

Internal docs in `docs/`:
- `GUI_DESIGN.md` — UI specifications and layout
- `PUNCH_MODULE_SPEC.md` — Punch DSP design and psychoacoustic research
- `CLIPPING_INSIGHTS.md` — Professional loudness techniques
- `SYSTEM_PROMPT.md` — AI session context

---

## Development

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
  spectral.rs      # FFT analysis + GainReductionData
  shaping.rs       # DSP math utilities
  styles.rs        # vizia CSS-like styles

cpp/               # C++ Airwindows FFI wrappers
xtask/             # Build tooling (bundle, install)
docs/              # Documentation + GitHub Pages site
```

### Justfile Recipes

```bash
just check        # Fast type-check
just build        # Debug build (no GUI)
just build-gui    # Debug build with GUI
just bundle       # Production VST3+CLAP bundle
just bundle-core  # Bundle without GUI
just install      # Install to system plugin dirs
just deploy       # bundle + install
just test         # Unit tests
just lint         # Clippy -D warnings
just fmt          # nightly rustfmt
just qa           # fmt-check + lint + test
just env          # Show build environment
```

---

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

Airwindows source code is copyright Chris Johnson, released under the MIT license.
