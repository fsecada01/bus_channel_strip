---
title: Architecture
description: Plugin internals, signal routing, and design decisions
---

Bus Channel Strip is built on the NIH-Plug framework with a lock-free, allocation-free audio processing pipeline, a vizia-plug GUI, and C++ FFI for one module (ButterComp2).

## Signal Chain

The default processing order follows the classic 500-series console layout. Every module can be reordered at runtime:

```
Audio In (stereo)
     │
     ▼
┌─────────────┐
│  API5500 EQ │  5-band semi-parametric EQ (custom Rust)
└──────┬──────┘
       ▼
┌─────────────┐
│ ButterComp2 │  Airwindows compressor (C++ FFI)
└──────┬──────┘
       ▼
┌─────────────┐
│  Pultec EQ  │  EQP-1A style EQ with tube saturation (Rust)
└──────┬──────┘
       ▼
┌─────────────┐
│ Dynamic EQ  │  4-band frequency-dependent compression (Rust)
└──────┬──────┘
       ▼
┌─────────────┐
│ Transformer │  Vintage transformer coloration (Rust)
└──────┬──────┘
       ▼
┌─────────────┐
│    Punch    │  Clipper + Transient Shaper, 8x oversampling (Rust)
└──────┬──────┘
       ▼
Audio Out (stereo)
```

Each module has a bypass parameter. When bypassed, the module's `process()` is skipped entirely and audio passes through untouched.

## NIH-Plug Integration

The plugin is implemented via the `Plugin` trait from [NIH-Plug](https://github.com/robbert-vdh/nih-plug):

```rust
impl Plugin for BusChannelStrip {
    const NAME: &'static str = "Bus Channel Strip";
    const VENDOR: &'static str = "Francis Secada";
    // ...

    fn params(&self) -> Arc<dyn Params> { self.params.clone() }
    fn initialize(&mut self, ...) -> bool { /* allocate buffers, init FFT */ }
    fn process(&mut self, buffer: &mut Buffer, ...) -> ProcessStatus { /* DSP */ }
}
```

Parameters are declared with `#[derive(Params)]` on `BusChannelStripParams`. Each field gets a macro attribute:

```rust
#[id = "lf_gain"]
pub lf_gain: FloatParam,
```

The `#[id]` string is the permanent, DAW-facing identifier. The field name and display name can change; the ID must not.

## Module Reordering System

Six `EnumParam<ModuleType>` parameters (`module_order_1` through `module_order_6`) define the processing order at runtime:

```rust
pub enum ModuleType {
    Api5500EQ, ButterComp2, PultecEQ, DynamicEQ, Transformer, Punch,
}
```

In `process()`, the plugin reads these six parameters, builds an ordered dispatch array, then iterates through it:

```rust
let order = [
    params.module_order_1.value(),
    params.module_order_2.value(),
    // ... through 6
];

for module_type in &order {
    match module_type {
        ModuleType::Api5500EQ => eq.process(buffer, &params),
        ModuleType::ButterComp2 => comp.process(buffer, &params),
        // ...
    }
}
```

Audio is double-buffered through `temp_buffer_1` and `temp_buffer_2` (pre-allocated in `initialize()`) to avoid any allocation on the audio thread.

## Parameter System

Parameters use NIH-Plug's smoother infrastructure. All continuous parameters use a 5ms linear smoother to prevent zipper noise on automation:

```rust
FloatParam::new("LF Gain", 0.0, FloatRange::Linear { min: -15.0, max: 15.0 })
    .with_smoother(SmoothingStyle::Linear(5.0))
    .with_unit(" dB")
```

The gain parameter uses logarithmic smoothing (more natural for gain changes):

```rust
FloatParam::new("Gain", util::db_to_gain(0.0), FloatRange::Skewed { ... })
    .with_smoother(SmoothingStyle::Logarithmic(50.0))
```

The plugin currently exposes approximately 75–90 automation parameters across all modules plus module order.

## FFI Boundary (ButterComp2)

The ButterComp2 compressor is ported from [Airwindows](https://github.com/airwindows/airwindows) C++ source. It is wrapped in a thin `extern "C"` interface in `cpp/buttercomp2.cpp`:

```cpp
extern "C" {
    ButterComp2State* buttercomp2_create(float sample_rate);
    void buttercomp2_destroy(ButterComp2State* state);
    void buttercomp2_process(ButterComp2State* state,
                             float* left, float* right,
                             int num_samples,
                             float compress, float output, float dry_wet);
}
```

`build.rs` compiles this using the `cc` crate:

```rust
cc::Build::new()
    .cpp(true)
    .file("cpp/buttercomp2.cpp")
    .compile("buttercomp2");
```

The Rust wrapper in `src/buttercomp2.rs` holds the raw pointer as a `NonNull<ButterComp2State>`, allocated in `new()` and freed in `Drop`. The state pointer is created once during `initialize()` and reused on every audio thread call — no allocation occurs on the audio thread.

Safety invariant: the pointer is valid for the lifetime of `ButterComp2`, single-threaded access is guaranteed because NIH-Plug calls `process()` from a single audio thread.

## Lock-Free Audio Thread Design

The audio thread is completely lock-free. Cross-thread communication uses atomics:

```rust
// Parameter from GUI → audio thread (via NIH-Plug's param system)
let gain = self.params.gain.smoothed.next(); // lock-free smoothed read

// GUI → audio: request a sidechain analysis
self.analysis_requested.store(true, Ordering::Relaxed);

// Audio → GUI: signal completion
self.analysis_result.ready.store(true, Ordering::Release);
```

`std::sync::Mutex` is forbidden on the audio thread. All shared state uses `AtomicF32`, `AtomicBool`, or `AtomicU32` with appropriate ordering guarantees.

## Spectrum Analyzer Pipeline

The spectrum analyzer feeds the GUI's frequency display using a lock-free ring buffer:

1. **Audio thread** — Each `process()` call writes samples into `fft_ring` (a pre-allocated circular buffer of `FFT_SIZE` samples).
2. **FFT** — When the ring is full, `realfft` computes the magnitude spectrum. Results are stored as `AtomicU32` (raw `f32` bits) in `SpectrumData`.
3. **GUI thread** — The vizia canvas reads `SpectrumData` bins with `Acquire` ordering and draws the spectrum line.

The sidechain masking analysis follows the same pattern: `sc_ring` holds a sidechain snapshot, the analysis is triggered by an `AtomicBool` flag from the GUI, and results are delivered back via `AnalysisResult` with Release/Acquire ordering.

## Feature Flag Topology

All modules and the GUI are gated behind Cargo feature flags:

```toml
[features]
default = ["api5500", "buttercomp2", "pultec", "transformer", "punch", "dynamic_eq"]
api5500 = []
buttercomp2 = []
pultec = []
dynamic_eq = []
transformer = []
punch = []
gui = ["vizia_plug", ...]
```

`gui` is **not** in default features for a critical reason: including it would cause Skia to compile on every CI target. On macOS Intel cross-compile (aarch64 → x86_64), Xcode's clang blocks x86 SIMD intrinsic headers even when targeting x86_64, making Skia impossible to build. Keeping `gui` out of defaults lets Intel CI targets build the core plugin without Skia.

In source, modules are conditionally compiled:

```rust
#[cfg(feature = "api5500")]
mod api5500;
#[cfg(feature = "api5500")]
use api5500::Api5500;
```

Parameters for feature-gated modules are also gated, but some shared infrastructure (`analysis_requested`, `analysis_result`, `gr_data`) is always compiled to keep the editor `create()` call site unconditional.

## GUI (vizia-plug)

The GUI is built with [vizia-plug](https://github.com/vizia/vizia-plug), which integrates the [vizia](https://vizia.dev/) UI framework with NIH-Plug.

Key design decisions:

- **Why vizia over egui/iced**: vizia uses a true ECS (Entity-Component-System) architecture with reactive `Lens` state propagation, avoiding manual state synchronization. It uses Skia as its rendering backend, giving hardware-accelerated 2D graphics with anti-aliasing.
- **ECS architecture**: Each UI widget is an entity. State flows through `Lens` traits — when a parameter changes, only the widgets bound to that parameter redraw.
- **Parameter binding**: `ParamSlider`, `ParamButton`, and `ParamKnob` widgets bind directly to `ParamPtr` via NIH-Plug's `ParamSetter`, ensuring all changes are properly routed through the host's automation system.
- **Skia canvas**: The spectrum analyzer is a custom `View` that implements `fn draw(&self, cx: &mut DrawContext, canvas: &Canvas)`. The canvas uses the Skia API (`skia-safe` 0.84.0), not femtovg.
- **CSS-like styling**: Colors, borders, and layout properties are set via `src/styles.rs` using vizia's CSS-like stylesheet system.
- **Module color coding**: Each module slot has a distinct color identity — EQ is blue-gray/cyan, Compressor is slate/orange, Pultec is brass/gold, Dynamic EQ is steel-blue/green, Transformer is charcoal, Punch is red/orange.

The GUI window is 1820×820 pixels with six equal-width module slots plus global controls.
