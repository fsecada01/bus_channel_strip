# DSP Enhancement Plan: Dual-Approach Multi-Model Dynamics

**Repo**: `fsecada01/bus_channel_strip`
**Scope**: Three feature tracks — enhanced Dynamic EQ with per-band dynamics models, multi-model standalone compressor, and shared optimizations
**Branch**: `claude/fix-docs-generation-J3uVs`

---

## Design Philosophy: Dual-Approach Dynamics

This plan places multi-model dynamics in **two locations** in the signal chain, serving distinct purposes:

1. **Dynamic EQ bands** (Track C) — Per-band `DynamicsModel` enum selects the *gain computer character* for each frequency band. The envelope follower and EQ filter stay the same; only the transfer curve changes. This is **nearly free** computationally (~5–10 extra ops/sample/band).

2. **Standalone compressor slot** (Track B) — Full compressor models (VCA/FET/Opto) replace ButterComp2 as selectable broadband compressors. Each model has its own detector, gain computer, and coloration stage.

**Why both?** They solve different problems:
- Dynamic EQ models shape *how aggressively each frequency band reacts* — e.g., an Opto-style slow response on the low band to avoid pumping, FET-style fast grab on the high band to catch sibilance.
- Standalone compressor models set the *overall compression character* of the bus — glue (ButterComp2), punch (VCA), aggression (FET), or leveling (Opto).

Combined, they add under 100 extra ops/sample total — trivial on any DAW-capable CPU.

---

## Computational Analysis

### Current Baseline (per sample, per channel)

| Module | Ops/sample | Transcendentals | Filters | State | Notes |
|--------|-----------|-----------------|---------|-------|-------|
| **ButterComp2** | ~20–25 | 0 | 0 | 160B | Pure arithmetic, bipolar butterfly |
| **Dynamic EQ** (4 bands) | ~80–100 (spikes to ~200) | `log10` ×4 + `cos/sin/powf` ×4 (hysteresis-gated) | 12 biquads | 480B | Hysteresis gates trig to ~every 2 samples |
| **Full chain** (all 6 modules) | ~300–400 | varies | ~30+ | ~2KB | Comfortable at 48kHz stereo |

### Proposed Additions — Cost Breakdown

| Addition | Extra ops/sample | Extra transcendentals | Extra filters | Extra state | When active |
|----------|-----------------|----------------------|---------------|-------------|-------------|
| **Track C: DynEQ per-band model** | ~5–10 per band (20–40 total) | 0–1 `exp()` per band (FET/Opto curves) | 0 | ~4B per band (enum) | Only on bands with non-Linear model |
| **Track B: VCA comp** | ~15–20 | 0 | 0–1 | ~80B | When VCA selected |
| **Track B: FET comp** | ~25–35 | 1 `exp()` | 1 biquad | ~120B | When FET selected |
| **Track B: Opto comp** | ~30–40 | 1 `exp()` | 1 biquad | ~200B | When Opto selected |

**Worst-case total added cost**: Opto comp (40) + 4 bands with FET curves (40) = **~80 extra ops/sample**. For context:
- A single biquad filter = ~10 ops/sample
- At 48kHz stereo, 80 ops = 7.68M ops/sec — about 0.2% of a single modern CPU core
- The existing plugin already runs ~300–400 ops/sample; this is a ~20% increase in the worst case

### Why Dynamic EQ Models Are Nearly Free

The key insight: **the envelope follower and sidechain filter are the expensive parts of the Dynamic EQ** (~15 ops + `log10` per band). The gain computer is just ~3–5 arithmetic ops. Swapping from a linear hard-knee ratio to an FET soft-knee or Opto program-dependent curve replaces those 3–5 ops with 5–10 ops. No new filters, no new envelope followers, no new state — just a different `match` arm in the gain computation.

```
Current gain computer (Linear):
  if over_db > 0.0:
      gain_change = -over_db * (1 - 1/ratio)              // 2 ops

FET-style gain computer (same envelope, different curve):
  gain = input / (1 + (input/threshold)^(ratio-1))        // ~8 ops + 1 exp()
  BUT: exp() is only needed for fractional exponents;
  for integer ratios, it's just multiplies

Opto-style gain computer (same envelope, different curve):
  gain = 1 / (1 + (envelope/threshold)^0.6)               // ~6 ops + 1 powf()
  BUT: 0.6 exponent can be approximated with a polynomial
  (error < 0.1 dB across operating range)
```

### Optimization Strategies

1. **Polynomial approximation for transcendentals**: `powf(x, 0.6)` in the Opto curve can be replaced with a 3rd-order Chebyshev polynomial fitted to the operating range (0.001–10.0), eliminating the transcendental entirely. Error < 0.1 dB.

2. **Shared envelope, branched gain**: The per-band `DynamicsModel` only affects the gain computer stage. All pre-processing (sidechain filter, envelope follower, dB conversion) is shared regardless of model.

3. **Hysteresis gating extends to all models**: The existing 0.05 dB hysteresis threshold on coefficient updates works identically for all gain curves — updates only fire during active dynamics changes.

4. **Only one standalone compressor active**: VCA/FET/Opto are mutually exclusive. No wasted computation on inactive models.

5. **SIMD-friendly `f32`**: All new code uses `f32` throughout. No `f64` conversions needed.

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

## Track A — Dynamic EQ Core Enhancements

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
- Apply same logic to all modes (Compress, Expand, Gate) and all dynamics models (Track C)

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

## Track B — Multiple Standalone Compressor Models

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

**Computational cost**: ~15–20 ops/sample, no transcendentals, 0–1 filters. Lightest model.

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

**Computational cost**: ~25–35 ops/sample, 1 `exp()`, 1 biquad filter (output transformer). Medium weight.

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

**Computational cost**: ~30–40 ops/sample, 1 `exp()`, 1 biquad filter (sidechain HPF). Heaviest model but still lightweight.

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

**Optimization**: The `powf(x, 0.6)` in the T4B curve can be replaced with a 3rd-order Chebyshev polynomial: `0.6x + 0.24x² + 0.096x³` (normalized to operating range), eliminating the transcendental with < 0.1 dB error.

**Files**: `src/opto_comp.rs` (new), referenced from `src/compressor.rs`

**Complexity**: Medium — the two-stage release and nonlinear photocell modeling require tuning, but the algorithm is simpler than FET.

---

## Track C — Dynamic EQ Per-Band Dynamics Models (NEW)

### Rationale

The Dynamic EQ's gain computer currently uses a simple linear ratio (hard-knee threshold, fixed ratio). By adding a `DynamicsModel` enum per band, users can select compression *character* per frequency band without adding new filters or envelope followers. The gain computer is the cheapest stage in the processing chain — this is where we get maximum sonic flexibility for minimum CPU cost.

**Use cases**:
- **Opto-style low band**: Slow, program-dependent compression on the lows (50–200Hz) prevents pumping while controlling rumble — the photocell's inherent sluggishness is a feature, not a bug.
- **FET-style high band**: Fast, aggressive grab on 3–8kHz catches sibilance and harshness with the FET's natural soft-knee onset.
- **Linear mid bands**: Clean, predictable dynamics on mids where you want surgical control.
- **Mix and match**: Different models per band opens up creative possibilities that no standalone compressor can offer.

### C1. DynamicsModel Enum for Dynamic EQ Bands

**What**: Add a per-band `DynamicsModel` selector that changes the gain computation curve without affecting the envelope follower or EQ filter.

**Design**:
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum DynamicsModel {
    #[name = "Linear"]
    Linear,        // Current behavior (hard knee ratio, default)
    #[name = "FET"]
    Fet,           // Soft-clip transfer curve, program-dependent ratio
    #[name = "Opto"]
    Opto,          // Photocell-style sub-linear response, slow tracking
}
```

**Gain computer replacement** (inside `process_sample`, replaces the current `match self.mode` block):

```rust
// After envelope_db and over_db are computed (unchanged):

let gain_change_db = match (self.mode, self.dynamics_model) {
    // ── Linear model (current behavior, default) ──
    (DynamicMode::CompressDownward, DynamicsModel::Linear) => {
        if over_db > 0.0 { -over_db * (1.0 - 1.0 / self.ratio) }
        else { 0.0 }
    }

    // ── FET model: soft-clip transfer curve ──
    (DynamicMode::CompressDownward, DynamicsModel::Fet) => {
        // Soft knee inherent: gain = -over / (1 + over/knee)
        // Program-dependent: higher over_db → ratio increases
        let knee = 6.0_f32;  // inherent FET soft knee
        if over_db > 0.0 {
            let effective_ratio = self.ratio * (1.0 + over_db * 0.02);
            -over_db * (1.0 - 1.0 / effective_ratio) * over_db / (over_db + knee)
        } else { 0.0 }
    }

    // ── Opto model: sub-linear photocell response ──
    (DynamicMode::CompressDownward, DynamicsModel::Opto) => {
        // T4B-style: compression amount grows sub-linearly with level
        // Polynomial approx of powf(x, 0.6): avoids transcendental
        if over_db > 0.0 {
            let x = (over_db / 40.0).min(1.0);  // normalize to 0–1
            let sub_linear = 0.6 * x + 0.24 * x * x + 0.096 * x * x * x;
            -sub_linear * 40.0 * (1.0 - 1.0 / self.ratio)
        } else { 0.0 }
    }

    // All models apply to all modes (Expand, Gate, ExpandDownward)
    // with analogous curve shaping...
    _ => { /* existing mode logic with model-appropriate curve */ 0.0 }
};
```

**What stays the same** (no changes needed):
- `BiquadPeak` sidechain filter — same envelope detection
- IIR envelope follower — same attack/release tracking
- `log10()` dB conversion — same level computation
- Hysteresis-gated coefficient updates — same optimization
- EQ filter application — same peaking filter

**What changes**:
- 3–8 extra arithmetic ops in the gain computer per band
- Opto model uses polynomial approximation (no `powf()`)
- FET model uses one extra multiply for program-dependent ratio

**New params**: 1 per band × 4 = 4 `EnumParam<DynamicsModel>` (`dyneq_bandN_dynamics_model`)

**Files**: `src/dynamic_eq.rs` (enum + gain computer), `src/lib.rs` (params)

**Complexity**: Low-Medium — localized to the gain computation block, which is ~10 lines of code per model.

### C2. Opto-Style Adaptive Release for Dynamic EQ

**What**: When `DynamicsModel::Opto` is selected on a band, the release time automatically adapts to signal content — fast for transients, slow for sustained signals — modeling the T4B photocell's dual-stage behavior.

**Why**: The Opto model's sonic character comes primarily from its *time behavior*, not just its transfer curve. Without adaptive release, an Opto-labeled gain curve is just a different ratio — the "feel" is missing.

**Design**:
- Add `opto_release_fast` and `opto_release_slow` state per band (only used when model = Opto)
- Two-stage release blend: `release_effective = lerp(60ms_coeff, release_coeff, blend_factor)`
- `blend_factor` ramps from 0 → 1 over ~200ms after gain reduction begins
- Resets to 0 when gain reduction drops below 0.5 dB

**Extra state per band**: 2 `f32` (8 bytes) — `opto_blend_factor` and `opto_fast_envelope`

**Extra ops per sample**: ~4 (blend computation) — only when Opto model active

**New params**: None — behavior is automatic when Opto model is selected.

**Files**: `src/dynamic_eq.rs`

**Complexity**: Low — 10 lines of IIR math, gated behind `DynamicsModel::Opto`.

### C3. FET-Style Program-Dependent Release for Dynamic EQ

**What**: When `DynamicsModel::Fet` is selected, release time scales with gain reduction depth — heavier compression triggers slower release, modeling the FET circuit's nonlinear recovery.

**Design**:
- `release_effective = release_base * (1.0 + gain_reduction_db.abs() * 0.1)`
- Computed per-sample from existing `gain_reduction_db` state (no new state needed)

**Extra ops per sample**: ~3 (1 abs, 1 multiply, 1 add) — only when FET model active

**New params**: None — behavior is automatic.

**Files**: `src/dynamic_eq.rs`

**Complexity**: Very low — 3 lines of code.

---

## Implementation Order (Recommended)

| Priority | Task | Effort | Impact | Extra ops/sample | Param IDs Added |
|----------|------|--------|--------|-----------------|-----------------|
| **P0** | A6. Expand Downward mode | 30 min | Medium | 0 | None (extends enum) |
| **P0** | A2. Soft Knee | 1 hr | High | ~2 per band | 4 (`dyneq_bandN_knee`) |
| **P0** | C1. DynEQ per-band DynamicsModel | 1.5 hr | High | ~5–10 per band | 4 (`dyneq_bandN_dynamics_model`) |
| **P1** | C3. FET program-dependent release | 30 min | Medium | ~3 per band (FET only) | None |
| **P1** | C2. Opto adaptive release | 1 hr | Medium | ~4 per band (Opto only) | None |
| **P1** | B1. CompressorModel enum + wrapper | 2 hr | High (architecture) | 0 (refactor) | 5 (`comp_model`, `comp_threshold`, `comp_ratio`, `comp_attack`, `comp_release`) |
| **P1** | B2. VCA Compressor | 3 hr | High | ~15–20 | None (uses B1 params) |
| **P1** | A4. Band Linking | 1 hr | Medium | ~4 (global) | 1 (`dyneq_link`) |
| **P2** | B3. FET Compressor | 4 hr | High | ~25–35 | None (uses B1 params) |
| **P2** | B4. Optical Compressor | 4 hr | High | ~30–40 | None (uses B1 params) |
| **P2** | A1. Mid/Side Processing | 3 hr | High | ~doubles DynEQ cost in M/S mode | 1 (`dyneq_stereo_mode`) |
| **P3** | A3. Auto-Threshold | 2 hr | Medium | ~5 per band | 4 (`dyneq_bandN_auto_thresh`) |
| **P3** | A5. Look-Ahead | 3 hr | Medium | ~2 (delay line read/write) | 1 (`dyneq_lookahead_ms`) |

**Total new parameters**: ~20 (keeping total plugin param count under ~110)

**Note on priority**: Track C items (C1–C3) are promoted to P0/P1 because they deliver the highest value-to-cost ratio. The per-band dynamics model is essentially free (a different `match` arm in existing code) and gives users a fundamentally new capability. The standalone compressor models (Track B) are more expensive to implement but are independent work that can proceed in parallel.

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
| `dyneq_band1_dynamics_model` | `EnumParam<DynamicsModel>` | Dynamic EQ | C1 |
| `dyneq_band2_dynamics_model` | `EnumParam<DynamicsModel>` | Dynamic EQ | C1 |
| `dyneq_band3_dynamics_model` | `EnumParam<DynamicsModel>` | Dynamic EQ | C1 |
| `dyneq_band4_dynamics_model` | `EnumParam<DynamicsModel>` | Dynamic EQ | C1 |
| `dyneq_link` | `FloatParam` | Dynamic EQ | A4 |
| `dyneq_stereo_mode` | `EnumParam<DynamicStereoMode>` | Dynamic EQ | A1 |
| `dyneq_band1_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band2_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band3_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_band4_auto_thresh` | `BoolParam` | Dynamic EQ | A3 |
| `dyneq_lookahead_ms` | `FloatParam` | Dynamic EQ | A5 |

**Backwards compatibility**: All existing parameter IDs are preserved. Existing DAW sessions will load correctly — new params initialize to defaults (`comp_model` defaults to `ButterComp2`, `dynamics_model` defaults to `Linear`), so all existing behavior is unchanged.

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
| C1 | — | `src/dynamic_eq.rs`, `src/lib.rs` |
| C2 | — | `src/dynamic_eq.rs` |
| C3 | — | `src/dynamic_eq.rs` |

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
| C1 | Small model selector dropdown per DynEQ band (below the mode selector) |
| C2–C3 | No GUI change — behavior is automatic when model is selected |

**Color coding for compressor models** (following existing module color conventions):
- ButterComp2: existing slate/orange
- VCA: steel gray / white accents (clean, clinical)
- FET: dark amber / warm orange (aggressive, vintage)
- Opto: deep blue / soft cyan (smooth, cool)

**Color coding for DynEQ dynamics models**:
- Linear: existing steel-blue/green accents (default, unchanged)
- FET: warm orange tint on band label (echoes standalone FET)
- Opto: soft cyan tint on band label (echoes standalone Opto)

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
- [ ] Polynomial approximations used where transcendentals would be per-sample (Opto powf → Chebyshev)
- [ ] DynamicsModel match arms are branchless-friendly (no heap, no divergent control flow)

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
| Chebyshev polynomial approximation | Numerical Recipes, Chapter 5 — function approximation for real-time DSP |
| Dynamic EQ design | Giannoulis et al., "A digital dynamic range compressor" (JAES 2012) — gain computer taxonomy |

---

## How to Use This Plan

Feed this file to your Claude instance with the following instruction:

> Implement the DSP enhancements described in `docs/DSP_ENHANCEMENT_PLAN.md` for the bus_channel_strip repository. Start with P0 items (A6 Expand Downward, A2 Soft Knee, C1 DynEQ per-band DynamicsModel), then P1 (C3 FET release, C2 Opto release, B1 compressor architecture, B2 VCA, A4 band linking). Each task should be a separate commit. Follow the audio thread safety checklist. Preserve all existing parameter IDs. Run `cargo test` and `cargo clippy` after each task.
