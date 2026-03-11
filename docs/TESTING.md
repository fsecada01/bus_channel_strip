# Bus Channel Strip — Testing Guide

## Current State

### Unit Tests (109 tests)

All unit tests live inline in each DSP module as `#[cfg(test)] mod tests { ... }` blocks.
This is idiomatic Rust for unit testing and is the only practical approach for `cdylib` crates
(plugin DLLs cannot be linked against from external `tests/` integration test crates).

Run all unit tests:
```bash
cargo test --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq"
# or via justfile:
just test
```

| Module | Tests | Coverage |
|--------|-------|----------|
| `shaping.rs` | 20 | `sigmoid`, `tanh_saturation`, `exp_curve`, `poly_log_curve`, `soft_knee_compress`, `Filter` |
| `spectral.rs` | 14 | `SpectrumData` roundtrip/dirty-flag, `AnalysisResult` defaults, `GainReductionData`, f32 bit-packing |
| `buttercomp2.rs` | 22 | `FetRatio`, `FetCompressor` (init/reset/GR cap/envelope clamp/dirty-check/attack formula), VCA + Optical |
| `dynamic_eq.rs` | 18 | `BiquadPeak` (identity/state-preservation/freq-clamping), `DynamicBand` (compress/gate/disabled), `DynamicEQ` API |
| `pultec.rs` | 10 | Construction, quadratic gain curves, tube_drive clamping, freq clamping, Q range |
| `api5500.rs` | 5 | Construction, gain clamping (+/-12 dB), multiple sample rates |
| `transformer.rs` | 12 | All 4 saturation models (zero-amount, finite output, bounded), model cache coherence, reset |
| `punch.rs` | 10 | Hard/soft/cubic clip, envelope follower, transient detector, oversampler (pre-existing) |

### Fixed Bug: NaN Dirty-Check in Compressor Models (fixed in commit `4ad5ea0c`)

During test development, a critical bug was discovered and fixed in `FetCompressor`,
`VcaCompressor`, and `OpticalCompressor`. All three used `f32::NAN` as a dirty-check
sentinel to force coefficient recomputation on first call to `update_parameters()`.
This pattern is broken in IEEE754:

```rust
// BROKEN: (x - NaN).abs() = NaN, NaN > threshold = false → never triggered
let atk_changed = (attack_ms - self.cached_attack_ms).abs() > 0.0001;
```

**Impact before fix**:
- `FetCompressor`: no compression ever applied (effective_threshold stuck at NaN)
- `VcaCompressor`: NaN output on every sample (NaN propagates through `f32::clamp()`)
- `OpticalCompressor`: same NaN propagation

**Fix applied** — explicit `.is_nan()` guard on every dirty-check:
```rust
let atk_changed = self.cached_attack_ms.is_nan()
    || (attack_ms - self.cached_attack_ms).abs() > 0.0001;
```

This was present in the v0.4.0 release. Fixed and released in v0.4.1.

---

## Future State: Integration Testing Workflow

### Tier 1: Plugin Format Validation (Recommended for CI)

These tools validate that the plugin correctly implements the VST3/CLAP specifications
without requiring a DAW.

#### pluginval (VST3)

```bash
# Install
winget install JUCE.pluginval
# or download from: https://github.com/juce-framework/pluginval/releases

# Run (after bundling)
pluginval --validate-in-process "target/bundled/Bus Channel Strip.vst3"

# Stricter validation (level 1-10)
pluginval --strictness-level 5 --validate-in-process "target/bundled/Bus Channel Strip.vst3"
```

What it tests: load/unload stability, parameter accessibility, NaN-free processing,
preset round-trips, stress testing (random parameter changes during audio processing).

#### clap-validator (CLAP)

By robbert-vdh (NIH-plug author) — purpose-built for the CLAP format.

```bash
cargo install clap-validator
clap-validator validate "target/bundled/Bus Channel Strip.clap"
```

Tests: scan, create/destroy, process blocks, parameter enumeration, state save/restore.

Add to `justfile`:
```just
validate-vst3: bundle
    pluginval --validate-in-process "target/bundled/Bus Channel Strip.vst3"

validate-clap: bundle
    clap-validator validate "target/bundled/Bus Channel Strip.clap"

validate: validate-vst3 validate-clap
```

---

### Tier 2: Custom Rust Test Host (Signal Chain E2E)

A `[[bin]]` target that imports DSP modules directly and runs audio through the full
signal chain without going through the VST3/CLAP wrapper ABI.

Add to `Cargo.toml`:
```toml
[[bin]]
name = "test_host"
path = "src/bin/test_host.rs"
required-features = ["api5500", "buttercomp2", "pultec", "transformer", "punch", "dynamic_eq"]
```

Planned tests:

| Test | Input | Assertion |
|------|-------|-----------|
| Full chain passthrough | Silence | Output is silence (no DC offset injection) |
| EQ frequency response | Swept sine, +6 dB at 1 kHz | Measure power at 1 kHz ±0.5 dB of expected |
| Compressor gain reduction | -6 dBFS sine, threshold -12 dBFS | GR within expected range for given ratio |
| Punch clipper hard limit | +6 dBFS burst | Output peak ≤ clip threshold |
| Oversampling aliasing | Sine near Nyquist | Alias products ≤ -80 dBFS |
| Module bypass | Any signal | Bypassed module output == input |
| NaN/Inf safety | DC offset + silence alternating | All output samples are finite |
| Parameter automation | Sweep all params during processing | No clicks (energy continuity) |

Scaffold:
```rust
// src/bin/test_host.rs
use bus_channel_strip::{
    api5500::Api5500, punch::PunchModule, /* ... */
};

fn sine(freq: f32, sr: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin()).collect()
}

fn rms_db(buf: &[f32]) -> f32 {
    let rms = (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt();
    20.0 * rms.max(1e-9).log10()
}

fn assert_near(label: &str, actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: got {actual:.2}, expected {expected:.2} ±{tolerance:.2}"
    );
}

fn main() {
    // TODO: implement test cases
    println!("All integration tests passed.");
}
```

Run with:
```bash
cargo run --bin test_host --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq"
```

---

### Tier 3: Golden File Regression Testing

Compare rendered output WAV files against known-good reference files to catch DSP regressions.

Workflow:
1. Generate golden files once from a verified-good build: `just generate-golden`
2. On each CI run, render the same test signals and diff against golden files
3. Flag any deviation beyond a tolerance threshold (e.g., max sample error < -80 dBFS)

```just
generate-golden:
    cargo run --bin test_host -- --write-golden golden/

regression-test:
    cargo run --bin test_host -- --compare-golden golden/ --tolerance-db -80
```

This is the most powerful regression guard — it catches subtle DSP changes that don't
affect NaN/finiteness but do change the sound.

---

### Tier 4: REAPER Headless Rendering (Optional)

REAPER can render a prepared project from the CLI, enabling full DAW-in-the-loop testing:

```bash
# Windows — renders a .rpp project without opening the GUI
reaper.exe -splashlog nul -nosplash -renderproject "tests/reaper/full_chain_test.rpp"
```

Workflow:
1. Create a `.rpp` test project with the plugin loaded, automation written, a test tone input
2. Render headlessly to WAV
3. Parse the WAV output and assert on frequency/amplitude characteristics

Requires: a REAPER license, the plugin bundle installed, and a prepared test project.
This is the highest-fidelity test but hardest to maintain in CI.

---

## CI Integration Plan

Suggested GitHub Actions job order:

```
cargo test (unit)     →  pluginval (VST3)  →  clap-validator (CLAP)
                                 ↓
                     test_host binary (signal chain)
                                 ↓
                     golden file regression check
```

Unit tests already run in CI on every push. Format validation and the custom test host
should be added as a separate `validate` job that runs on `push` to `main` and on
release tags, after the bundle job succeeds.
