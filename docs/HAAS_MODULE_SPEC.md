# Haas Module — Implementation Spec (revised)

Psychoacoustic stereo widener for Bus Channel Strip. Named after Helmut Haas
and the precedence effect. M/S widening + side-only comb + wide-comb L/R
crosstalk mode. No EQ, no saturation, no bass enhancement — those live in
API5500 / Pultec / Transformer. Haas is a clean spatial tool.

This spec has been reviewed against the actual codebase and a DSP design
review. Ranges and formulas reflect those corrections; the original draft
had several mono-compatibility and headroom risks that are fixed here.

---

## Placement in the signal chain

Default DSP order after this change:

```
API5500 → ButterComp2 → Pultec → Transformer → Haas → Punch → DynEQ (reserve)
```

Haas sits **before** Punch so Punch can catch any residual peaks from the
widener. The reorder system still lets users move it anywhere, but the GUI
tooltip should note "post-Punch placement may overload the clipper."

---

## Files to create or modify

| File | Change |
|------|--------|
| `src/haas.rs` | NEW — module implementation + unit tests |
| `src/lib.rs` | Add `ModuleType::Haas` variant, params, `module_order_7`, `module_type_index` arm, `dispatch_module` arm, `Default`/`initialize`/`reset` wiring, `latency_samples` impl |
| `src/editor.rs` | Add `build_haas_controls`, `build_bypass_button_for_type` arm, `build_led_indicator_for_type` arm, `build_controls_for_type` arm, `ModuleTheme::Haas` in the slot theme resolver |
| `src/components.rs` | Add `ModuleTheme::Haas` enum variant + class + accent color |
| `Cargo.toml` | Add `haas` to `[features]` default list |

**Do not** modify behaviour of any existing module or any existing parameter ID.

---

## DSP spec

### Constants

```rust
/// Ring-buffer length: next power of two above 20 ms @ 192 kHz plus
/// 4-sample Hermite-interpolation headroom (3840 + 4 = 3844, round up
/// to next pow2 = 4096). Power-of-two allows mask-based wrap.
const DELAY_BUF_LEN: usize = 4096;
const DELAY_MASK: usize = DELAY_BUF_LEN - 1;

/// Anti-denormal dither magnitude written into the delay line on every
/// sample. Alternates sign per sample so it averages to zero. Airwindows
/// convention.
const DENORMAL_DITHER: f32 = 1.0e-20;
```

### CombMode enum

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum CombMode {
    /// Side-only polarity-flip comb. ~7 ms delay. WOW-Thing style.
    /// Mono-compatible — comb contribution sums to zero on mono collapse.
    #[name = "Side Comb"]
    SideComb,
    /// Wide comb: delayed (L−R) injected with opposing signs. Produces
    /// a more diffuse image than SideComb. Depth is internally clamped
    /// to 0.5 to prevent catastrophic mono-sum notching.
    #[name = "Wide Comb"]
    WideComb,
}

impl Default for CombMode {
    fn default() -> Self { Self::SideComb }
}
```

> **Naming note:** The draft called mode 2 "Haas / crosstalk cancellation."
> That's misleading — true transaural XTC (Bauck/Cooper 1996, Gardner 1997)
> requires speaker-to-ear HRTF inversion. This implementation is a delayed
> wide-comb on the L/R axis and is named honestly.

### Struct

```rust
pub struct HaasModule {
    sample_rate: f32,

    // Heap-allocated delay buffers. 4 × 4096 × 4 B = 64 KB — safe on
    // the heap, risky on the stack. Allocated once in `new()`.
    delay_side_l: Box<[f32; DELAY_BUF_LEN]>,  // SideComb write target
    delay_side_r: Box<[f32; DELAY_BUF_LEN]>,  // SideComb inverted copy
    delay_in_l:   Box<[f32; DELAY_BUF_LEN]>,  // WideComb raw L history
    delay_in_r:   Box<[f32; DELAY_BUF_LEN]>,  // WideComb raw R history
    write_pos: usize,

    // Smoothed delay length in fractional samples. Read with 4-point
    // Hermite interpolation. Smoothing eliminates both zipper noise
    // from parameter automation AND clicks from discrete index jumps
    // — no hysteresis needed.
    smoothed_delay_samples: f32,      // SideComb path
    smoothed_xtalk_samples: f32,      // WideComb path
    target_delay_samples: f32,
    target_xtalk_samples: f32,
    delay_smooth_coeff: f32,          // ~20 ms time constant

    // Output compensation state (computed per-block from gains).
    output_trim: f32,

    // Anti-denormal dither sign flip per sample.
    denormal_sign: f32,
}
```

### API

```rust
impl HaasModule {
    pub fn new(sample_rate: f32) -> Self;

    /// Update user-facing parameters. Called once per buffer before `process()`.
    /// Gains are already in **linear amplitude**, not dB — convert at the lib.rs
    /// boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn update_parameters(
        &mut self,
        mid_gain: f32,          // linear
        side_gain: f32,         // linear
        comb_depth: f32,        // 0..1
        comb_time_ms: f32,      // 1..20
        comb_mode: CombMode,
        mix: f32,               // 0..1
    );

    /// Process a stereo buffer in place. Lock-free, allocation-free.
    pub fn process(&mut self, buffer: &mut nih_plug::buffer::Buffer);

    /// Zero all delay buffers. Called on DAW transport reset.
    pub fn reset(&mut self);

    /// Current latency introduced by the module, in samples, rounded down
    /// from the smoothed delay length. 0 when bypassed.
    pub fn latency_samples(&self) -> u32;
}
```

### Per-sample algorithm

Inside `process()`, for each sample:

1. **Optional FTZ/DAZ per process() call** — set once at the top of the
   process function, not per-sample:
   ```rust
   #[cfg(target_arch = "x86_64")]
   unsafe {
       use core::arch::x86_64::*;
       _MM_SET_FLUSH_ZERO_MODE(_MM_FLUSH_ZERO_ON);
       _MM_SET_DENORMALS_ZERO_MODE(_MM_DENORMALS_ZERO_ON);
   }
   ```
   Document the safety contract: this sets per-thread CPU state. Restore
   is not required since the audio thread runs this module every buffer.

2. **Smooth delay target.** One-pole LPF per sample:
   ```
   smoothed_delay_samples += (target_delay_samples - smoothed_delay_samples) * delay_smooth_coeff
   smoothed_xtalk_samples += (target_xtalk_samples - smoothed_xtalk_samples) * delay_smooth_coeff
   ```
   `delay_smooth_coeff = 1 - exp(-1 / (0.020 * sample_rate))` (≈ 20 ms τ).

3. **M/S encode:**
   ```
   mid  = (in_l + in_r) * 0.5 * mid_gain
   side = (in_l - in_r) * 0.5 * side_gain
   ```

4. **Write delay buffers:**
   ```rust
   // Signed anti-denormal dither prevents accumulation during silence.
   self.denormal_sign = -self.denormal_sign;
   let dither = DENORMAL_DITHER * self.denormal_sign;

   self.delay_side_l[write_pos] = side + dither;
   self.delay_side_r[write_pos] = -side + dither;
   self.delay_in_l[write_pos]   = in_l + dither;
   self.delay_in_r[write_pos]   = in_r + dither;
   ```

5. **Branch on `comb_mode`**, using fractional-sample Hermite read:

   **`SideComb`:**
   ```
   comb_l = hermite4_read(delay_side_l, write_pos, smoothed_delay_samples) * comb_depth
   comb_r = hermite4_read(delay_side_r, write_pos, smoothed_delay_samples) * comb_depth
   ```

   **`WideComb`** — hard-clamp depth to prevent mono-sum catastrophe:
   ```
   let effective_depth = comb_depth.min(0.5);
   let x_l = hermite4_read(delay_in_l, write_pos, smoothed_xtalk_samples);
   let x_r = hermite4_read(delay_in_r, write_pos, smoothed_xtalk_samples);
   let cancel = (x_l - x_r) * effective_depth * 0.5;
   comb_l =  cancel;
   comb_r = -cancel;
   ```

6. **Decode + inject:**
   ```
   wide_l = mid + side + comb_l
   wide_r = mid - side + comb_r
   ```

7. **Output compensation.** Scale by pre-computed `output_trim` so worst-case
   RMS stays ≤ unity. Computed once in `update_parameters()`:
   ```
   output_trim = 1.0 / (1.0 + side_gain * comb_depth).sqrt().max(1.0)
   ```
   This is conservative — real peak depends on material correlation — but
   it prevents the +24 dB worst case the unreviewed draft allowed.

8. **Advance write pointer:**
   ```rust
   write_pos = (write_pos + 1) & DELAY_MASK;
   ```

9. **Linear dry/wet blend** (correct for correlated signals; equal-power
   would over-boost at mix = 0.5):
   ```
   out_l = in_l + (wide_l * output_trim - in_l) * mix
   out_r = in_r + (wide_r * output_trim - in_r) * mix
   ```

### Hermite interpolation helper

```rust
/// 4-point Hermite interpolation for fractional delay reads.
/// Reference: Moorer, "The Manifold Joys of Conformal Mapping," JAES 1983.
#[inline]
fn hermite4_read(buf: &[f32; DELAY_BUF_LEN], write_pos: usize, delay: f32) -> f32 {
    let di = delay.floor();
    let frac = delay - di;
    let i = di as usize;
    let base = (write_pos + DELAY_BUF_LEN - i) & DELAY_MASK;
    let xm1 = buf[(base + 1) & DELAY_MASK];
    let x0  = buf[base];
    let x1  = buf[(base + DELAY_BUF_LEN - 1) & DELAY_MASK];
    let x2  = buf[(base + DELAY_BUF_LEN - 2) & DELAY_MASK];
    let c0 = x0;
    let c1 = 0.5 * (x1 - xm1);
    let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
    ((c3 * frac + c2) * frac + c1) * frac + c0
}
```

---

## Parameter spec (`src/lib.rs`)

All IDs are new and unique. None collide with existing IDs. Ranges reflect
the headroom-safe caps from the design review.

```rust
// ── Haas Module Parameters ──────────────────────────────────────────────
#[cfg(feature = "haas")]
#[id = "haas_bypass"]
pub haas_bypass: BoolParam,

#[cfg(feature = "haas")]
#[id = "haas_mid_gain"]
pub haas_mid_gain: FloatParam,   // -12.0..+6.0 dB, default 0.0

#[cfg(feature = "haas")]
#[id = "haas_side_gain"]
pub haas_side_gain: FloatParam,  // -6.0..+6.0 dB, default 0.0
                                 // (capped at +6 instead of +18; auto-
                                 // compensation makes higher values
                                 // pointless and unsafe)

#[cfg(feature = "haas")]
#[id = "haas_comb_depth"]
pub haas_comb_depth: FloatParam, // 0.0..1.0, default 0.0

#[cfg(feature = "haas")]
#[id = "haas_comb_time"]
pub haas_comb_time: FloatParam,  // 1.0..20.0 ms, default 7.0 (skewed)

#[cfg(feature = "haas")]
#[id = "haas_comb_mode"]
pub haas_comb_mode: EnumParam<CombMode>,  // default SideComb

#[cfg(feature = "haas")]
#[id = "haas_mix"]
pub haas_mix: FloatParam,        // 0.0..1.0, default 1.0
```

Conventions to follow (match existing modules):

- `mid_gain`, `side_gain`, `mix`, `comb_depth` use `.with_smoother(SmoothingStyle::Linear(5.0))`.
- `comb_time` uses `FloatRange::Skewed { factor: FloatRange::skew_factor(-1.0), .. }`.
- `comb_time` displays via `formatters::v2s_f32_rounded(1)` (one decimal ms).
- `mid_gain` / `side_gain` display via `formatters::v2s_f32_gain_to_db_with_decimals(1)`.
- Call `util::db_to_gain(param.smoothed.next())` at the dispatch site to convert to linear before handing to `update_parameters`.

---

## Plugin wiring (`src/lib.rs`)

### `ModuleType` enum (around line 74)

Add variant **after** `Transformer`, **before** `Punch`:

```rust
Transformer,
Haas,
Punch,
```

### `module_order_N` (around line 547-557)

**Add** `module_order_7` — do **not** reuse an existing slot. This keeps
every preset from prior versions loadable.

```rust
#[id = "module_order_7"]
pub module_order_7: EnumParam<ModuleType>,
```

Default order in `Default::default()`:

```rust
module_order_1: EnumParam::new("Module Order 1", ModuleType::Api5500EQ),
module_order_2: EnumParam::new("Module Order 2", ModuleType::ButterComp2),
module_order_3: EnumParam::new("Module Order 3", ModuleType::PultecEQ),
module_order_4: EnumParam::new("Module Order 4", ModuleType::Transformer),
module_order_5: EnumParam::new("Module Order 5", ModuleType::Haas),
module_order_6: EnumParam::new("Module Order 6", ModuleType::Punch),
module_order_7: EnumParam::new("Module Order 7", ModuleType::DynamicEQ),
```

The reorder loop in the process path (lib.rs:~2140) already iterates the
order array — just extend the iteration length to 7 and extend
`module_type_index()` (lib.rs:1511) to return 6 for Haas.

### Dispatch arm (`dispatch_module`, lib.rs:1869-1920)

```rust
#[cfg(feature = "haas")]
ModuleType::Haas => {
    let mid_gain  = util::db_to_gain(self.params.haas_mid_gain.smoothed.next());
    let side_gain = util::db_to_gain(self.params.haas_side_gain.smoothed.next());
    self.haas.update_parameters(
        mid_gain,
        side_gain,
        self.params.haas_comb_depth.smoothed.next(),
        self.params.haas_comb_time.value(),
        self.params.haas_comb_mode.value(),
        self.params.haas_mix.smoothed.next(),
    );
    if !self.params.haas_bypass.value() {
        self.haas.process(buffer);
    }
}
```

### Default / Initialize / Reset

Follow the existing `PunchModule` two-site pattern:

```rust
// In Default::default(): placeholder sample rate
haas: HaasModule::new(44100.0),

// In Plugin::initialize() at lib.rs:~2028:
self.haas = HaasModule::new(sr);

// In Plugin::reset():
self.haas.reset();
```

### Latency reporting — NEW

The plugin does not currently implement `fn latency_samples`. Add:

```rust
fn latency_samples(&self) -> u32 {
    let mut total = 0u32;
    #[cfg(feature = "haas")]
    if !self.params.haas_bypass.value() {
        total = total.saturating_add(self.haas.latency_samples());
    }
    // Future modules with latency add here.
    total
}
```

Inside NIH-plug's `Plugin` impl block. This reports the latency the host
needs to compensate for PDC.

---

## GUI (`src/editor.rs`)

EnumParams render as plain `ParamSlider` in this codebase — there is no
existing two-button-toggle pattern. Follow the same pattern as `punch_clip_mode`.

### `ModuleTheme::Haas` — add to `src/components.rs`

```rust
Haas,
// ...
Self::Haas => "haas-theme",
Self::Haas => Color::rgb(140, 160, 210),  // muted blue-lavender
```

Pick the colour so it's visually distinct from Api5500 (cyan) and DynEQ
(steel-blue). Add matching CSS in `src/styles.rs`.

### `build_haas_controls` — add to `src/editor.rs`

Layout matches existing module convention: `module_section` titled
groups, `module_row` horizontal rows.

```rust
fn build_haas_controls(cx: &mut Context) {
    #[cfg(feature = "haas")]
    VStack::new(cx, |cx| {
        components::module_section(cx, "M/S", |cx| {
            components::module_row(cx, |cx| {
                components::create_gain_slider(cx, "MID",  Data::params, |p| &p.haas_mid_gain);
                components::create_gain_slider(cx, "SIDE", Data::params, |p| &p.haas_side_gain);
            });
        });
        components::module_section(cx, "COMB", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "DEPTH", Data::params, |p| &p.haas_comb_depth);
                components::create_param_slider(cx, "TIME",  Data::params, |p| &p.haas_comb_time);
            });
            components::create_param_slider(cx, "MODE", Data::params, |p| &p.haas_comb_mode);
        });
        components::module_section(cx, "OUTPUT", |cx| {
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.haas_mix);
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}
```

### Dispatch arms

Add matching arms in `build_controls_for_type`, `build_bypass_button_for_type`,
and `build_led_indicator_for_type` — all three dispatchers already follow
a `match ModuleType` pattern for the other six modules.

---

## Tests (`src/haas.rs`)

Required, in addition to the DSP-side review tests:

1. **Identity.** `mid_gain = side_gain = 1.0`, `comb_depth = 0`, `mix = 1.0`,
   mono input (L = R). Output L == R == input after settling past the delay
   line warm-up period.
2. **Widening.** Stereo input with `L = sin, R = 0`. With `side_gain = 2.0`,
   the output `|L - R|` must exceed the input `|L - R|`.
3. **Mix = 0 is exact passthrough.** Any parameter combo, `mix = 0` →
   output equals input bit-exact.
4. **Mono-sum null (SideComb).** Mono input (L = R = impulse). Sum L+R of
   output must equal `2 * mid_gain * impulse` within −60 dB across the
   full comb-depth / comb-time parameter space.
5. **Mono-sum bounded (WideComb).** Mono input, max depth. Sum L+R notch
   must stay above −30 dB (guaranteed by the internal 0.5 depth clamp).
6. **Correlation monotonicity.** Pearson(L,R) must decrease monotonically
   as `side_gain` increases on a stereo-correlated input.
7. **DC preservation.** DC input → DC output, exact for any parameter combo
   (the mid path is unity-gain at DC when `mid_gain = 1.0`).
8. **Latency report.** `latency_samples()` must equal
   `smoothed_delay_samples.floor() as u32` after buffer-length warm-up.
9. **No click on automation sweep.** Automate `comb_time` 1 → 20 ms linearly
   over 100 ms. FFT the output; peak broadband impulse must be < −40 dBFS.
10. **Denormal survival.** Feed 10 s of silence after a loud burst. The
    delay buffers must settle to exact zero (FTZ) within 50 ms and stay
    there. No CPU spike.
11. **Reset contract.** After `reset()`, the next `process()` on a mono
    impulse must produce output equal to `mix * (mid_gain * impulse)` at
    t = 0 — i.e. no leftover state from the previous buffer.

Manual code-review items (not runnable tests):

- Confirm zero `Vec::new()`, `Box::new()`, `format!`, `String`, `.to_string()`
  inside `process` or any function it calls.
- Confirm all indexing into delay buffers uses `& DELAY_MASK`, never `%`,
  never an unchecked `[i]` that could go out of range.
- Confirm every `unsafe` block has a `// SAFETY:` comment.

---

## Feature flag

Add to `Cargo.toml`:

```toml
[features]
default = ["api5500", "buttercomp2", "pultec", "transformer", "punch", "haas", "dynamic_eq"]
haas = []
```

Every Haas-related param, struct field, dispatch arm, and GUI builder must
be gated on `#[cfg(feature = "haas")]` to match the feature-flag discipline
of the existing modules.

---

## What NOT to do

- Do not add EQ, bass enhancement, or saturation. That's API5500, Pultec,
  and Transformer's job.
- Do not use a single "WOW" macro knob. All controls are explicit.
- Do not allocate on the audio thread. All buffers live on the heap and
  are sized in `new()`.
- Do not call `.unwrap()` or `.expect()` in any code reachable from `process()`.
- Do not change any existing parameter ID, any existing module's behaviour,
  or displace a `module_order_N` slot — extend to slot 7.
- Do not rename the module. It is called Haas.
- Do not use `%` for ring-buffer wrap — always `& DELAY_MASK` (hotpath
  conventions; `%` on non-const divisor is slower and DELAY_BUF_LEN is a
  power of two specifically so mask-wrap works).
- Do not implement "true crosstalk cancellation" — the Wide Comb formula is
  not XTC and must not be labelled as such.

---

## Review summary (what changed vs the original draft)

| Topic | Original draft | Revised |
|-------|----------------|---------|
| Buffer iteration | `iter_samples_stereo_mut()` | Method doesn't exist; use `buffer.iter_samples()` nested loop (match Punch pattern) |
| Delay storage | `[f32; 4096]` on struct | `Box<[f32; 4096]]` on heap — 64 KB total, stack-unsafe |
| Delay read | Integer index | Hermite4 fractional read |
| Smoothing | 0.1 ms hysteresis on `comb_time` | 20 ms one-pole LPF + fractional read; no hysteresis needed |
| `side_gain` range | +18 dB | +6 dB cap (headroom review) |
| Type 2 mono safety | depth = 1.0 allowed | Internal `depth.min(0.5)` clamp |
| Output normalization | None | Pre-computed `output_trim` per buffer |
| Denormal handling | Not mentioned | FTZ/DAZ per process + per-sample anti-denormal dither |
| Mode naming | "Crosstalk Cancellation" | "Wide Comb" (honest) |
| Tests | 3 listed | 11 listed (mono-sum null, correlation, DC, latency, click sweep, denormal, reset) |
| Module slots | Implicit 6 | Explicit extend to `module_order_7` |
| Latency reporting | Overwrite `latency_samples` | Additive — plugin didn't have one; new impl sums across all latency-bearing modules |
| EnumParam UI | Two-button toggle | ParamSlider (matches existing codebase convention) |
| `db_to_gain` | `util::db_to_gain` | Confirmed — NIH-plug's helper, already used in lib.rs |

---

## References

- Moorer, J.A. "About This Reverberation Business." *Computer Music Journal*
  3 (2), 1979. — comb-filter math, delay-line readout.
- Gardner, W.G. *3-D Audio Using Loudspeakers.* Kluwer, 1997. §4. — transaural
  stability margins; why Type-2 depth needs clamping.
- Zölzer, U. *DAFX: Digital Audio Effects,* 2nd ed., Wiley, 2011. §5.2. —
  RMS-preserving stereo widening.
- Bauck, J. & Cooper, D. "Generalized Transaural Stereo and Applications."
  *JAES* 44 (9), 1996. — why the draft's "XTC" label was wrong.
- Moorer, J.A. "The Manifold Joys of Conformal Mapping." *JAES* 31 (11), 1983.
  — Hermite4 interpolation coefficients.
