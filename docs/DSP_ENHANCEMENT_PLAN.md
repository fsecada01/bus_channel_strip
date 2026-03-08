# DSP Enhancement Plan: Dynamic EQ + Multiple Compressor Models

**Repo**: `fsecada01/bus_channel_strip`
**Scope**: Two feature tracks — enhanced Dynamic EQ and multi-model compressor
**Branch**: `claude/dsp-enhancement-plan-J3uVs`

---

## Current State Summary

### Dynamic EQ (`src/dynamic_eq.rs`)
- 4-band dynamic equalizer with per-band: frequency, Q, threshold, ratio, attack, release, makeup gain
- 3 modes: Compress Downward, Expand Upward, Gate
- Custom `BiquadPeak` struct (RBJ Cookbook) with persistent state across buffer boundaries
- Sidechain detection via +6 dB peak filter at detector frequency
- Band-isolation solo mode with RBJ bandpass
- Hysteresis-gated coefficient updates (0.05 dB threshold) to avoid per-sample trig recompute
- FFT spectral overlay and per-band GR metering shared lock-free with GUI
- ~10 params per band × 4 bands = ~40 params + bypass

### Compressor (`src/buttercomp2.rs` + `cpp/buttercomp2.cpp`)
- Single model: Airwindows ButterComp2 via C++ FFI
- 3 exposed params: compress, output, dry/wet
- Bipolar interleaved compression with butterfly processing
- Buffer-level FFI call (O(1) overhead per buffer, not per sample)
- Hard limiter at ±1.0 on output

---

## Track A — Dynamic EQ Enhancements

### A1. Mid/Side Processing Mode

**What**: Add a stereo mode selector (Stereo / Mid / Side / Mid+Side) per band or globally.

**Why**: Bus processing frequently needs different dynamics on mid vs side — e.g., tighten the low-mid center while leaving stereo width untouched, or de-ess only the side channel.

**Design**:
- Add `DynamicStereoMode` enum: `Stereo`, `MidOnly`, `SideOnly`, `MidSide` (independent mid and side processing)
- M/S encode before the dynamic band cascade, decode after
- In `MidSide` mode, each band gets two `DynamicBand` instances (one mid, one side)
- Memory: doubles the per-band state (~256 bytes × 4 bands extra)

**New params**: 1 global `EnumParam<DynamicStereoMode>` (`dyneq_stereo_mode`)

**Files**: `src/dynamic_eq.rs`, `src/lib.rs` (param + wiring)

**Complexity**: Medium — M/S encode/decode is trivial (`mid = (L+R)*0.5`, `side = (L-R)*0.5`), but doubling band state and routing needs care.

### A2. Soft Knee

**What**: Add a configurable knee width to the gain computer so compression onset is gradual rather than hard at threshold.

**Why**: Hard knee on a dynamic EQ creates audible pumping artifacts on gradual swells. Soft knee makes the transition musical.

**Design**:
- Add `knee_db: f32` per band (0 dB = hard knee, 6–12 dB = soft)
- Replace the hard `if over_db > 0.0` check with quadratic interpolation in the knee region:
  ```
  if over_db < -knee/2:
      gain_change = 0
  elif over_db > knee/2:
      gain_change = over_db * (1 - 1/ratio)  // full compression
  else:
      // Quadratic blend in knee region
      x = over_db + knee/2
      gain_change = (1 - 1/ratio) * x² / (2 * knee)
  ```
- Apply same logic to all three modes (Compress, Expand, Gate)

**New params**: 1 per band × 4 = 4 `FloatParam` (`dyneq_bandN_knee`, range 0–12 dB, default 3 dB)

**Files**: `src/dynamic_eq.rs` (gain computer in `process_sample`), `src/lib.rs` (params)

**Complexity**: Low — localized change to the gain computation block.

### A3. Auto-Gain / Auto-Threshold

**What**: An auto-threshold mode that tracks the average signal level and sets threshold relative to it, so the dynamic EQ adapts to varying input levels.

**Why**: When the Dynamic EQ is post-compressor in the chain, input levels are more predictable. But when reordered earlier, or when the mix changes, a static threshold becomes wrong. Auto-threshold keeps the dynamics musical without constant tweaking.

**Design**:
- Per-band toggle: `auto_threshold: bool`
- When enabled, maintain a slow RMS envelope (500ms–2s time constant) of the sidechain signal
- Threshold = RMS_envelope_dB + user_offset_dB (the existing threshold param becomes a relative offset)
- RMS envelope uses the same IIR follower pattern already in `DynamicBand`, just with longer time constants

**New params**: 1 toggle per band × 4 = 4 `BoolParam` (`dyneq_bandN_auto_thresh`)

**Files**: `src/dynamic_eq.rs`, `src/lib.rs`

**Complexity**: Low — adds one slow IIR per band, reinterprets threshold param.

### A4. Band Linking

**What**: A global "link" parameter (0–100%) that blends each band's gain reduction toward the average GR across all enabled bands.

**Why**: Prevents bands from fighting each other. When one band compresses hard, linked bands compress proportionally, preserving tonal balance.

**Design**:
- After computing per-band `gain_change_db` but before applying to EQ filter:
  ```
  avg_gr = mean(band[i].gain_change_db for enabled bands)
  final_gr[i] = lerp(band[i].gain_change_db, avg_gr, link_amount)
  ```
- Single global `link` param (0.0 = independent, 1.0 = fully linked)

**New params**: 1 `FloatParam` (`dyneq_link`, range 0–100%, default 0%)

**Files**: `src/dynamic_eq.rs` (`process` method), `src/lib.rs`

**Complexity**: Low — 4 lines of math between gain computation and EQ update.

### A5. Look-Ahead

**What**: Delay the audio signal relative to the sidechain by a configurable amount (0–5 ms) so the compressor reacts before the transient arrives.

**Why**: Eliminates the first few milliseconds of transient overshoot, especially important for Gate mode and fast Compress settings.

**Design**:
- Pre-allocated circular delay buffer per channel (at 48kHz, 5ms = 240 samples, so `[f32; 512]` per channel is sufficient)
- Sidechain runs on the non-delayed signal; EQ filter runs on the delayed signal
- When look-ahead = 0, bypass the delay buffer entirely (no latency penalty)
- Report latency to host via NIH-Plug's `latency()` method when look-ahead > 0

**New params**: 1 global `FloatParam` (`dyneq_lookahead_ms`, range 0–5 ms, default 0 ms)

**Files**: `src/dynamic_eq.rs`, `src/lib.rs`

**Complexity**: Medium — delay line is straightforward, but reporting plugin latency changes requires `context.set_latency_samples()` and affects the entire plugin, not just the DynEQ module.

### A6. Expand Downward Mode

**What**: Add a fourth dynamics mode: downward expansion. Below threshold, signal is attenuated by the ratio.

**Why**: Useful for noise reduction without the hard cutoff of a gate. Gradually reduces low-level signals (room noise, bleed) while leaving above-threshold content untouched.

**Design**:
- Add `ExpandDownward` variant to `DynamicMode` enum
- Gain computer: when `over_db < 0.0`, `gain_change_db = over_db * (ratio - 1.0)`
- Clamped to -96 dB like Gate mode

**New params**: None — extends existing `EnumParam<DynamicMode>`

**Files**: `src/dynamic_eq.rs` (enum + gain computer), `src/lib.rs` (if mode enum is referenced)

**Complexity**: Very low — 5 lines of code.

---

## Track B — Multiple Compressor Models

### Architecture Decision

**Pattern**: Follow the Transformer module's approach — a single `CompressorModule` struct with a `CompressorModel` enum that selects the algorithm. All models share the same parameter interface but produce different character.

**Why not separate modules?**: The compressor occupies one slot in the signal chain. Multiple models behind a single selector keeps the parameter count manageable and avoids a combinatorial explosion in the reordering system.

### B1. CompressorModel Enum + Trait Interface

**What**: Refactor the compressor slot from a single ButterComp2 FFI wrapper into a model-selectable architecture.

**Design**:
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum CompressorModel {
    #[name = "ButterComp2 (Glue)"]
    ButterComp2,
    #[name = "VCA (Punch)"]
    Vca,
    #[name = "FET (Aggro)"]
    Fet,
    #[name = "Opto (Smooth)"]
    Opto,
}
```

**Shared parameter interface** (all models map these to their internal behavior):
| Param | Current ID | Range | Notes |
|-------|-----------|-------|-------|
| Model | `comp_model` (NEW) | Enum | Selects algorithm |
| Threshold | `comp_threshold` (NEW) | -40 to 0 dB | ButterComp2 maps `compress` param internally |
| Ratio | `comp_ratio` (NEW) | 1:1 to 20:1 | ButterComp2 ignores (fixed internal ratio) |
| Attack | `comp_attack` (NEW) | 0.1 to 100 ms | ButterComp2 ignores (program-dependent) |
| Release | `comp_release` (NEW) | 10 to 1000 ms | ButterComp2 ignores (program-dependent) |
| Makeup | `comp_output` | 0 to +24 dB | Existing param, keep ID |
| Dry/Wet | `comp_dry_wet` | 0–100% | Existing param, keep ID |
| Bypass | `comp_bypass` | bool | Existing param, keep ID |
| Compress | `comp_compress` | 0.0–1.0 | Existing ButterComp2-specific param — **keep for backwards compat** |

**Parameter ID stability**: Existing `comp_compress`, `comp_output`, `comp_dry_wet`, `comp_bypass` IDs must be preserved. New params get new IDs. When model = ButterComp2, `comp_compress` drives the FFI; the new threshold/ratio/attack/release params are ignored. When model != ButterComp2, `comp_compress` is ignored and the new params drive the algorithm.

**New params**: 5 (`comp_model`, `comp_threshold`, `comp_ratio`, `comp_attack`, `comp_release`)

**Files**: New `src/compressor.rs` (multi-model wrapper), `src/buttercomp2.rs` (unchanged, used by wrapper), `src/lib.rs` (params + wiring)

**Complexity**: Medium — architectural refactor, but ButterComp2 behavior is preserved exactly.

### B2. VCA Compressor (SSL-Style)

**What**: Clean, punchy VCA compression modeled on the SSL bus compressor character.

**Character**: Fast, transparent, punch-preserving. The "mix glue" compressor for rock/pop/electronic.

**Algorithm**:
```
Detector:
  - Peak or RMS detection (RMS default for bus use)
  - Ballistics: exponential attack/release IIR

Gain computer:
  - Hard knee (or configurable soft knee)
  - gain_db = threshold + (input_db - threshold) / ratio  (when input > threshold)
  - Auto-makeup option: compensate by (threshold * (1 - 1/ratio)) dB

Transfer function:
  - Linear VCA gain application (no harmonic distortion from gain element)
  - Optional subtle even-harmonic saturation on the sidechain detector path
```

**Implementation**: Pure Rust, no FFI. All state in a struct, allocation-free.

**Key constants** (SSL-inspired defaults):
| Parameter | SSL Bus Default |
|-----------|----------------|
| Attack | 0.1 / 0.3 / 1 / 3 / 10 / 30 ms |
| Release | 100 / 300 / 600 ms + Auto |
| Ratio | 2:1 / 4:1 / 10:1 |

**Files**: `src/vca_comp.rs` (new), referenced from `src/compressor.rs`

**Complexity**: Medium — well-documented algorithm, straightforward to implement correctly.

### B3. FET Compressor (1176-Style)

**What**: Aggressive, colorful FET compression with program-dependent attack/release and harmonic distortion.

**Character**: Fast attack, punchy, adds excitement. The "character" compressor for drums, vocals, aggressive buses.

**Algorithm**:
```
Detector:
  - Peak detection (FET circuits respond to peaks)
  - Program-dependent release: release slows as GR increases
    release_effective = release_base * (1.0 + gr_db.abs() * 0.1)

Gain computer:
  - Soft knee inherent to the FET transfer curve
  - At high ratios (20:1), approaches limiting with "all-buttons" character
  - gain = input / (1 + (input/threshold)^(ratio-1))  // soft-clip style

Coloration:
  - 2nd and 3rd harmonic generation proportional to input level
  - Subtle high-frequency rolloff from the output transformer (use existing BiquadPeak,
    low-pass shelf at 15kHz, -1 to -3 dB depending on drive)
```

**"All-buttons" mode**: When ratio ≥ 12:1, engage parallel compression of all ratio paths for the characteristic aggressive, distorted sound. This is the 1176 "trick" that engineers use on room mics and parallel drum buses.

**Files**: `src/fet_comp.rs` (new), referenced from `src/compressor.rs`

**Complexity**: Medium-High — the FET transfer curve and program-dependent release need careful tuning to sound musical rather than mathematical.

### B4. Optical Compressor (LA-2A Style)

**What**: Slow, smooth optical compression with electro-luminescent gain reduction modeling.

**Character**: Transparent, leveling. The "set and forget" compressor for vocals, bass, gentle bus leveling.

**Algorithm**:
```
Detector:
  - RMS with slow integration (~50ms attack minimum)
  - Two-stage release: fast initial release + slow secondary tail
    Stage 1: ~60ms (handles transient recovery)
    Stage 2: ~1-3 seconds (handles sustained level changes)
    Blend: exponential crossfade from stage 1 to stage 2 over time

Gain computer:
  - Inherently soft knee (optical element has nonlinear response)
  - Frequency-dependent: low frequencies compress more due to
    the photocell's thermal lag (model with a gentle HPF on the
    sidechain, -3dB @ 200Hz)
  - Compression curve: gain = 1 / (1 + (envelope/threshold)^0.6)
    (the 0.6 exponent models the T4B cell's sub-linear response)

Modes:
  - Compress (default): gentle 2:1–4:1 equivalent (program-dependent)
  - Limit: faster attack, higher effective ratio (~10:1)
```

**Key insight**: The optical compressor's character comes from its *slow, asymmetric timing* and *program-dependent ratio*, not from a fixed set of controls. Threshold and the compress/limit switch are the primary controls — attack and release are largely fixed by the optical model.

**Files**: `src/opto_comp.rs` (new), referenced from `src/compressor.rs`

**Complexity**: Medium — the two-stage release and nonlinear photocell modeling require tuning, but the algorithm is simpler than FET.

---

## Implementation Order (Recommended)

| Priority | Task | Effort | Impact | Param IDs Added |
|----------|------|--------|--------|-----------------|
| **P0** | A6. Expand Downward mode | 30 min | Medium | None (extends enum) |
| **P0** | A2. Soft Knee | 1 hr | High | 4 (`dyneq_bandN_knee`) |
| **P1** | B1. CompressorModel enum + wrapper | 2 hr | High (architecture) | 5 (`comp_model`, `comp_threshold`, `comp_ratio`, `comp_attack`, `comp_release`) |
| **P1** | B2. VCA Compressor | 3 hr | High | None (uses B1 params) |
| **P1** | A4. Band Linking | 1 hr | Medium | 1 (`dyneq_link`) |
| **P2** | B3. FET Compressor | 4 hr | High | None (uses B1 params) |
| **P2** | B4. Optical Compressor | 4 hr | High | None (uses B1 params) |
| **P2** | A1. Mid/Side Processing | 3 hr | High | 1 (`dyneq_stereo_mode`) |
| **P3** | A3. Auto-Threshold | 2 hr | Medium | 4 (`dyneq_bandN_auto_thresh`) |
| **P3** | A5. Look-Ahead | 3 hr | Medium | 1 (`dyneq_lookahead_ms`) |

**Total new parameters**: ~16 (keeping total plugin param count under 100)

---

## Parameter ID Registry

New IDs introduced by this plan (must never collide with existing IDs):

| ID | Type | Module | Task |
|----|------|--------|------|
| `comp_model` | `EnumParam<CompressorModel>` | Compressor | B1 |
| `comp_threshold` | `FloatParam` | Compressor | B1 |
| `comp_ratio` | `FloatParam` | Compressor | B1 |
| `comp_attack` | `FloatParam` | Compressor | B1 |
| `comp_release` | `FloatParam` | Compressor | B1 |
| `dyneq_band1_knee` | `FloatParam` | Dynamic EQ | A2 |
| `dyneq_band2_knee` | `FloatParam` | Dynamic EQ | A2 |
| `dyneq_band3_knee` | `FloatParam` | Dynamic EQ | A2 |
| `dyneq_band4_knee` | `FloatParam` | Dynamic EQ | A2 |
| `dyneq_link` | `FloatParam` | Dynamic EQ | A4 |
| `dyneq_stereo_mode` | `EnumParam<DynamicStereoMode>` | Dynamic EQ | A1 |
| `dyneq_band1_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band2_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band3_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band4_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_lookahead_ms` | `FloatParam` | Dynamic EQ | A5 |

**Backwards compatibility**: All existing parameter IDs are preserved. Existing DAW sessions will load correctly — new params initialize to defaults, and `comp_model` defaults to `ButterComp2` so existing ButterComp2 behavior is unchanged.

---

## Files Modified Per Task

| Task | New Files | Modified Files |
|------|-----------|----------------|
| A1 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| A2 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| A3 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| A4 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| A5 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| A6 | — | `src/dynamic_eq.rs` |
| B1 | `src/compressor.rs` | `src/lib.rs`, `src/buttercomp2.rs` (minor) |
| B2 | `src/vca_comp.rs` | `src/compressor.rs` |
| B3 | `src/fet_comp.rs` | `src/compressor.rs` |
| B4 | `src/opto_comp.rs` | `src/compressor.rs` |

---

## GUI Implications

Each task has GUI work that should follow the DSP implementation:

| Task | GUI Change |
|------|-----------|
| A1 | Stereo mode dropdown in DynEQ header |
| A2 | Knee knob per band (small, secondary control) |
| A3 | Auto-threshold toggle LED per band |
| A4 | Link knob in DynEQ header |
| A5 | Look-ahead knob in DynEQ header (show latency in ms) |
| A6 | New mode option in existing band mode dropdown |
| B1 | Model selector (large, prominent) in compressor section — follow Transformer's 4-model layout |
| B2–B4 | Conditional param visibility: show threshold/ratio/attack/release for VCA/FET/Opto; show compress knob only for ButterComp2 |

**Color coding for compressor models** (following existing module color conventions):
- ButterComp2: existing slate/orange
- VCA: steel gray / white accents (clean, clinical)
- FET: dark amber / warm orange (aggressive, vintage)
- Opto: deep blue / soft cyan (smooth, cool)

---

## Audio Thread Safety Checklist

All implementations must satisfy:

- [ ] No heap allocation in `process()` — pre-allocate all buffers in `initialize()`
- [ ] No mutex/lock — use `AtomicF32`/`AtomicBool` for GUI communication
- [ ] No `.unwrap()` / `.expect()` — use `.get()` with bounds or `clamp()`
- [ ] No `format!()` / `String` / `println!()` in audio path
- [ ] Filter coefficients computed per-buffer (on param change), not per-sample (except DynEQ which is inherently per-sample but hysteresis-gated)
- [ ] Denormal guards: `f32::MIN_POSITIVE` before any `log10()` / division
- [ ] Phase-coherent stereo: identical processing on L/R unless intentionally decorrelated (M/S mode)

---

## References

| Topic | Source |
|-------|--------|
| RBJ Audio EQ Cookbook | Robert Bristow-Johnson — biquad coefficient formulas |
| Soft knee formula | DAFX (Zölzer), Chapter 4 — Digital Audio Effects |
| VCA compressor topology | SSL 4000 series service manual analysis |
| FET gain reduction curve | Universal Audio 1176 rev D/E schematic analysis |
| Optical T4B photocell model | Giannoulis et al., "Digital modeling of an optical compressor" (DAFx 2012) |
| Airwindows ButterComp2 | Chris Johnson — https://github.com/airwindows/airwindows |
| Program-dependent release | Massberg & Reiss, "Autonomous multi-band compressor" (AES 2011) |

---

## How to Use This Plan

Feed this file to your Claude instance with the following instruction:

> Implement the DSP enhancements described in `docs/DSP_ENHANCEMENT_PLAN.md` for the bus_channel_strip repository. Start with P0 items (A6 Expand Downward, A2 Soft Knee), then P1 (B1 compressor architecture, B2 VCA, A4 band linking). Each task should be a separate commit. Follow the audio thread safety checklist. Preserve all existing parameter IDs. Run `cargo test` and `cargo clippy` after each task.
