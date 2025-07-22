# NIH-plug Bus Mixstrip Plugin

This repository serves as a starting point for building a VST3 bus mixstrip plugin using [NIH-plug](https://github.com/RustAudio/nih-plug).

## Prerequisites

- Rust (edition 2021 or later): install via https://rustup.rs
- VST3 SDK (for plugin hosting or if you want to build against the SDK directly)

## Getting Started

1. Install [Cookiecutter](https://github.com/cookiecutter/cookiecutter) if you don't already have it:
   ```bash
   pip install cookiecutter
   ```
2. Scaffold a new plugin project using the NIH-Plug template:
   ```bash
   cookiecutter https://github.com/robbert-vdh/nih-plug-template.git
   ```
3. Change into your new plugin directory (e.g., `my-plugin-name`) and start the minimal example:
   ```bash
   cd <your-plugin-directory>
   cargo run --example minimal
   ```
4. Implement your mixstrip DSP and GUI in `src/lib.rs` following the template structure.

## Project Layout

- `Cargo.toml`: Plugin metadata and dependencies (nih-plug, vst3-sys, etc.).
- `src/lib.rs`: Core DSP and GUI code of your plugin.
- `.gitignore`: Ignore build artifacts and temporary files.

## Building and Testing

```bash
# Build release VST3 plugin
cargo build --release

# The VST3 binary will be available in target/release/
``` 

## Next Steps

1. Update plugin metadata in `Cargo.toml` (name, vendor, unique ID).
2. Define mixstrip parameters, state serialization, and GUI layout.
3. Iterate: build, load into a host, and verify functionality.

---

## Plugin Building

After installing Rust (https://rustup.rs), you can compile the plugin as follows:

```shell
cargo xtask bundle bus_channel_strip --release
```

## Pre-commit Hooks

We use [`pre-commit`](https://pre-commit.com) to enforce formatting and lint checks.
After installing it via:

```bash
uv tool install pre-commit
```
Enable the Git hook with:

```bash
pre-commit install
```

To re-run formatting against all files:

```bash
pre-commit run fmt --all-files
```

# Note
# The `clippy` hook is currently disabled due to upstream dependency resolution issues
# in the `backtrace` crate; please run `cargo clippy --all-targets --all-features` manually or
# configure it in your CI pipeline.

---

Below is a rough blueprint for turning your bus‑channel‑strip into a chain of classic
outboard‑gear emulations. I’ve broken it into two parts:

## 1. High‑level signal‑chain layout

We’ll represent each “module” as a stage in the DSP pipeline. In order:

| Stage                               | Rough Description                                                                 |
|:------------------------------------|:----------------------------------------------------------------------------------|
| **1. Input EQ (API 5500 style)**    | A 4‑ or 5‑band semi‑parametric EQ with bell‑shaped filters and selectable shelving bands—modeled on the API 5500. |
| **2. Compressor (LA‑2A or VCA bus)** | Opto‑tube‑style leveling amp (LA‑2A) and/or a faster VCA “bus compressor” mode. Expose a parameter to select flavor. |
| **3. Pultec‑style Passive EQ**      | The Pultec “trick” passive low‑ and high‑boost plus cut logic.                       |
| **4. Multiband Compressor/Limiter** | Split the signal into (e.g.) three bands and apply band‑specific gain reduction.   |
| **5. Tape Emulation**               | Non‑linear saturation + high‑frequency roll‑off to emulate tape.                     |
| **6. Output Saturation/Color**      | A final saturator (soft clip, tube, or transformer style) plus an output gain stage. |

Visually in pseudocode your `process()` would look like:

```rust
fn process(&mut self, buffer: &mut Buffer, …) -> ProcessStatus {
    // 1) Input–stage EQ
    self.eq_api5500.process(buffer);

    // 2) Compression (optical or VCA)
    self.compressor.process(buffer);

    // 3) Pultec EQ
    self.pultec.process(buffer);

    // 4) Multiband compressor
    self.multiband.process(buffer);

    // 5) Tape emulation
    self.tape.process(buffer);

    // 6) Final saturation + output gain
    self.saturator.process(buffer);

    ProcessStatus::Normal
}
```

---

## 2. Step‑by‑step implementation plan

Below is a suggested roadmap. You can tackle each block in sequence, wiring parameters into your
`BusChannelStripParams` and then filling in their DSP implementations.

| Step                                   | What to Do                                                                                      |
|:---------------------------------------|:-----------------------------------------------------------------------------------------------|
| **A. Define parameters**               |                                                                                                 |
|                                         | 1. **API‑5500 EQ**: Frequency, Gain, Q knobs for each band (LF, LMF, MF, HMF, HF).             |
|                                         | 2. **Compressor**: Threshold, Ratio, Attack, Release, Makeup‑Gain, Type toggle (optical/VCA).  |
|                                         | 3. **Pultec EQ**: LF‑Boost, LF‑Cut, HF‑Boost, HF‑Cut, Cut‑Frequency knobs.                         |
|                                         | 4. **Multiband**: Band crossover freqs, thresholds, ratios, makeup‑gain.                       |
|                                         | 5. **Tape**: Drive, Tape speed (kHz roll‑off), Bias.                                            |
|                                         | 6. **Saturator**: Drive, Output‑Level.                                                          |

Add each of these to your `#[derive(Params)]` struct in `src/lib.rs`.

| **B. Implement each DSP block**        |                                                                                                 |
|                                         | - Start with a minimal placeholder (e.g. simple shelf/bell filter) then swap in your design.    |
|                                         | - Leverage crates like `dasp`, `biquad`, or write your own IIR filters.                         |
|                                         | - For compressors: a one‑pole envelope follower + gain computer for VCA; T4 opto model for LA‑2A.|

| **C. Chain them together in `process()`** |                                                                                               |
|                                         | - Call stages in series on the same buffer.                                                     |
|                                         | - Be mindful of CPU: consider oversampling tape/sat blocks only if needed.                       |

| **D. GUI layout**                      |                                                                                                 |
|                                         | - In your NIH‑Plug GUI, stack each module vertically, “rack‑style.”                              |
|                                         | - Use group boxes (or tabs) so users can collapse/expand sections.                              |

| **E. Testing & Tuning**                |                                                                                                 |
|                                         | - Build (`cargo xtask bundle bus_channel_strip --release`) and load into a host.                |
|                                         | - Tweak coefficients, time constants, saturation curves to taste.                              |

---

### Next steps

1. **Sketch the `Params` struct** in `src/lib.rs` with all the knobs you need.
2. **Wire up a simple one‑band EQ** (e.g. API 5500 low shelf) and confirm it shows in your GUI.
3. **Add the next blocks one by one**, testing in a host after each.

That should give you a clear development path. Let me know if you need code samples for any specific
module (e.g. Pultec EQ math or tape‑saturation curve), or if you’d like GUI‑layout examples in NIH‑Plug!
