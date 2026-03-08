# Compressor Model UI Switching — Implementation Spec

## Overview

The ButterComp2 editor slot gains a model selector `EnumParam` that swaps the visible control
surface between three compressor personalities (Classic, VCA, Optical) using `Binding::new()`.
New parameters for VCA and Optical models are added to `ButterComp2Params` with frozen IDs; their
DSP stubs pass audio through unchanged and are explicitly out of scope for this spec.

---

## State / Parameters

### Enum definition

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ButterComp2Model {
    Classic,
    Optical,
    Vca,
}

impl Default for ButterComp2Model {
    fn default() -> Self { ButterComp2Model::Classic }
}
```

Required `#[derive]` list: `Debug, Clone, Copy, PartialEq, Eq, Enum` (NIH-Plug `Enum` derive).

### New parameters — `ButterComp2Params` sub-struct

Add alongside existing `comp_compress`, `comp_output`, `comp_dry_wet`:

```rust
/// Model selector — always visible
#[id = "comp_model"]
pub model: EnumParam<ButterComp2Model>,

// VCA model parameters
#[id = "comp_vca_thresh"]
pub vca_thresh: FloatParam,   // –60.0 to 0.0 dB,  default –18.0, Linear smoothing 5 ms

#[id = "comp_vca_ratio"]
pub vca_ratio: FloatParam,    // 1.0 to 20.0,       default 4.0,   Linear smoothing 5 ms

#[id = "comp_vca_atk"]
pub vca_atk: FloatParam,      // 0.1 to 100.0 ms,   default 10.0,  Linear smoothing 5 ms

#[id = "comp_vca_rel"]
pub vca_rel: FloatParam,      // 10.0 to 1000.0 ms, default 100.0, Linear smoothing 5 ms

// Optical model parameters
#[id = "comp_opt_thresh"]
pub opt_thresh: FloatParam,   // –60.0 to 0.0 dB,   default –18.0, Linear smoothing 5 ms

#[id = "comp_opt_speed"]
pub opt_speed: FloatParam,    // 0.1 to 10.0 (normalized speed), default 1.0, Linear 5 ms

#[id = "comp_opt_char"]
pub opt_char: FloatParam,     // 0.0 to 1.0 (character blend),   default 0.5, Linear 5 ms
```

All new `FloatParam` instances use `Smoother::new(SmoothingStyle::Linear(5.0))`.

**Parameter ID freeze**: the seven IDs above (`comp_model`, `comp_vca_thresh`, `comp_vca_ratio`,
`comp_vca_atk`, `comp_vca_rel`, `comp_opt_thresh`, `comp_opt_speed`, `comp_opt_char`) are
permanently reserved. Do not reuse or rename them in future versions.

---

## Enum Definition (Binding Wiring)

`Binding::new()` requires a lens whose target implements `Data`. `EnumParam<ButterComp2Model>` does
not satisfy this directly — map it to `usize` via the param's raw integer value.

Add a `usize` mirror field to the GUI `Data` struct:

```rust
buttercomp2_model_idx: usize,   // mirrors comp_model EnumParam; updated on param change
```

Update this field in `Data::event()` when `RawParamEvent` fires for `comp_model`, or initialize it
from `params.buttercomp2.model.value() as usize` in `Data::new()`.

Alternatively, read the param value directly inside the `Binding::new()` closure from
`cx.data::<Data>()` — either pattern is acceptable as long as the lens target is `usize`.

---

## `Binding::new()` Wiring

Inside `build_buttercomp2_controls()`:

```rust
// Model selector — always visible, above the binding
ParamSlider::new(cx, params, |p| &p.buttercomp2.model);   // or a dedicated enum widget

// Reactive control surface
Binding::new(cx, Data::buttercomp2_model_idx, |cx, model_lens| {
    let model_idx = *model_lens.get(cx);
    match model_idx {
        0 => build_classic_controls(cx, params),    // ButterComp2Model::Classic as usize == 0
        1 => build_optical_controls(cx, params),    // ButterComp2Model::Optical as usize == 1
        2 => build_vca_controls(cx, params),        // ButterComp2Model::Vca    as usize == 2
        _ => build_classic_controls(cx, params),    // safe fallback
    }
});
```

The `Binding::new()` tears down and rebuilds only the control surface subtree, not the model
selector row. This is intentional and acceptable here because model switches are rare,
infrequent user actions — not per-frame reactive updates.

---

## Control Surface Function Signatures

```rust
fn build_classic_controls(
    cx: &mut Context,
    params: Arc<BusChannelStripParams>,
)
// Sliders: comp_compress, comp_output, comp_dry_wet (existing behavior, unchanged)

fn build_vca_controls(
    cx: &mut Context,
    params: Arc<BusChannelStripParams>,
)
// Sliders: comp_vca_thresh, comp_vca_ratio, comp_vca_atk, comp_vca_rel

fn build_optical_controls(
    cx: &mut Context,
    params: Arc<BusChannelStripParams>,
)
// Sliders: comp_opt_thresh, comp_opt_speed, comp_opt_char
```

Each function builds a `VStack` of `ParamSlider` rows with consistent height so the slot does not
resize when the model changes. All three functions must produce a VStack of identical pixel height.

---

## DSP Stub Note

**VCA and Optical DSP are explicitly OUT OF SCOPE for this spec.**

In `ButterComp2::process()`, add a model branch that reads `params.buttercomp2.model.value()` and
routes to the existing Classic algorithm for all three cases as a safe stub:

```rust
match params.buttercomp2.model.value() {
    ButterComp2Model::Classic => { /* existing buttercomp2 DSP */ }
    ButterComp2Model::Vca     => { /* TODO: VCA DSP — pass through for now */ }
    ButterComp2Model::Optical => { /* TODO: Optical DSP — pass through for now */ }
}
```

Pass-through means: copy `in_l`/`in_r` to `out_l`/`out_r` without modification. No allocation,
no locks. The stub satisfies the audio thread rules unconditionally.

---

## Implementation Steps

1. Define `ButterComp2Model` enum in `src/buttercomp2.rs` with the three variants and required derives.
2. Add `model: EnumParam<ButterComp2Model>` and the six new `FloatParam` fields to `ButterComp2Params` using the frozen `#[id]` strings.
3. Add `buttercomp2_model_idx: usize` to the GUI `Data` struct; initialize from `params.buttercomp2.model.value() as usize` in `Data::new()`; keep it updated in `Data::event()` on param change.
4. Refactor `build_buttercomp2_controls()` to render the model selector above a `Binding::new(cx, Data::buttercomp2_model_idx, ...)` block; extract `build_classic_controls()`, `build_vca_controls()`, `build_optical_controls()` as separate functions.
5. Ensure all three control surface functions produce a `VStack` of equal pixel height; set explicit `height: Pixels(N)` on the outer VStack in each to prevent slot resize on switch.
6. Add DSP stubs in `ButterComp2::process()` matching on `model.value()` — Classic uses existing logic, VCA and Optical pass through.

---

## Guardrails

- **Parameter ID freeze** — the eight IDs listed in this spec (`comp_model`, `comp_vca_*`, `comp_opt_*`) must never be renamed, reused, or removed without a breaking-change migration note; doing so breaks existing DAW sessions.
- **No audio thread allocation** — new `FloatParam` smoothers are initialized at plugin load; `process()` only calls `.smoothed.next()`, which is allocation-free.
- **No layout jank on model switch** — all three control surface VStacks must have identical pixel height; the ButterComp2 slot must not change size when the model selector changes.
- **`Binding::new()` scope** — only the control surface subtree (below the model selector) is inside the `Binding`; the model selector `ParamSlider` itself is outside and always visible.
- **Existing IDs untouched** — `comp_compress`, `comp_output`, `comp_dry_wet` keep their current `#[id]` strings and ranges unchanged; `build_classic_controls()` renders them identically to today.
- **No `.unwrap()` in `process()`** — the model `match` must include a `_` fallback arm.

---

## Acceptance Criteria

1. Switching the model selector to VCA displays exactly four sliders (Thresh, Ratio, Attack, Release) and hides the Classic sliders — no other slot changes size or layout.
2. Switching back to Classic restores the original three sliders (Compress, Output, Dry/Wet) with their current automation values intact.
3. All eight new parameter IDs appear in a DAW's automation lane list when the plugin is loaded; the three existing IDs remain present and unmodified.
4. With the VCA or Optical model selected, audio passes through the ButterComp2 slot unmodified (stub behavior) — confirmed by null-testing in Reaper (invert + sum = silence at unity gain).
5. `cargo +nightly fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass with no new failures after the full change set is applied.
