# ADR-0002: Compressor Model UI — Binding-Based Control Switching

**Status**: Accepted
**Date**: 2026-03-08
**Deciders**: Project (Claude Code orchestration session)

---

## Context

The DSP Enhancement Plan (Track B, `docs/DSP_ENHANCEMENT_PLAN.md`) introduces a multi-model compressor: ButterComp2 (existing FFI), VCA, FET, and Opto. Each model exposes a different control surface:

- **ButterComp2**: `compress` (single FFI-mapped knob), `output`, `dry/wet`
- **VCA / FET / Opto**: `threshold`, `ratio`, `attack`, `release`, `makeup`, `dry/wet`

ButterComp2's `compress` knob maps directly to the C++ FFI's internal gain parameter and has no meaningful equivalent in the threshold/ratio paradigm. Showing both sets simultaneously would be confusing and misleading.

NIH-plug parameters cannot be added or removed at runtime — all parameters exist in `BusChannelStripParams` at compile time. The GUI can only show or hide controls for params that are statically declared.

---

## Decision

Use vizia's `Binding::new()` to rebuild the compressor control subtree when `comp_model` changes.

```rust
Binding::new(
    cx,
    Data::params.map(|p| p.comp_model.value() as usize), // usize satisfies Data bound
    |cx, model_lens| {
        match model_lens.get(cx) {
            0 => build_buttercomp2_controls(cx),  // compress / output / dry-wet
            1 => build_vca_controls(cx),           // threshold / ratio / atk / rel / makeup / dry-wet
            2 => build_fet_controls(cx),           // + all-buttons toggle
            3 => build_opto_controls(cx),          // threshold / compress-limit / makeup
            _ => {}
        }
    },
);
```

The model selector (a segmented `ParamButton` row) lives above the `Binding` block and is always visible.

**Why `Binding::new()` over `.display()` toggling:**
- Control sets differ structurally per model (ButterComp2's `compress` has no analog in others)
- A single shared layout would require showing/hiding individual rows — verbose and fragile
- `Binding::new()` tears down and rebuilds only the controls subtree; the selector is untouched
- Consistent with the existing module-slot swap pattern (`create_dynamic_module_slot`, editor.rs:339)

**Parameter ID stability:** Existing `comp_compress`, `comp_output`, `comp_dry_wet`, `comp_bypass` IDs are preserved. New model-specific params (`comp_threshold`, `comp_ratio`, `comp_attack`, `comp_release`, `comp_model`) use new IDs. ButterComp2 remains the default — existing DAW sessions are unaffected.

---

## Consequences

**Easier:**
- Each model's control function is self-contained and independently testable
- Adding a new model = adding one match arm + one build function
- No risk of showing irrelevant controls (e.g., `compress` knob while Opto is active)

**Harder:**
- vizia rebuilds the ECS subtree on model switch (one-time cost per user action — imperceptible)
- `EnumParam<CompressorModel>` must be mapped to `usize` for the `Binding` lens (same pattern as module slots)

**Unchanged:**
- All parameters exist in the audio thread at all times regardless of which model is selected
- Hidden model params still receive automation and respond to DAW recall
