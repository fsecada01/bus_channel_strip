# ADR-0004: Haas Module — M/S + Comb Widening, Not True Crosstalk Cancellation

**Status**: Implemented

**Date**: 2026-01 (spec revised after DSP design review)
**Deciders**: Project (Claude Code orchestration session)

---

## Context

The signal chain needed a dedicated spatial-widening tool distinct from EQ/saturation modules (API5500, Pultec, Transformer already own tone). The original draft spec called its second comb mode "Haas / crosstalk cancellation," implying true transaural XTC (speaker-to-ear HRTF inversion, per Bauck & Cooper 1996 / Gardner 1997) — which this implementation does not do. The draft also had two headroom-safety gaps: `side_gain` allowed up to +18 dB, and Wide Comb allowed depth up to 1.0, both of which could produce worst-case +24 dB output and catastrophic mono-sum notching.

Placement in the chain also needed a decision: Haas must sit **before** Punch (in the default order) so Punch's clipper can catch any residual peaks the widener introduces — placing it post-Punch risks overloading the clipper.

## Decision

Implement Haas as an M/S encoder with two selectable comb modes, explicitly **not** labeled as crosstalk cancellation:

- **SideComb** (default): side-only polarity-flip comb (~7 ms), mono-compatible by construction — comb contribution sums to zero on mono collapse.
- **WideComb**: delayed `(L−R)` injected with opposing signs for a more diffuse image; depth is **internally clamped to 0.5** regardless of the user-facing 0–1 range, to prevent catastrophic mono-sum notching.

Safety and quality decisions baked into the DSP:
- `side_gain` range capped at **+6 dB** (not +18) — auto-compensation makes higher values pointless and unsafe.
- Pre-computed `output_trim = 1.0 / (1.0 + side_gain * comb_depth).sqrt().max(1.0)` per buffer keeps worst-case RMS ≤ unity.
- 4-point Hermite interpolation (Moorer 1983) for fractional-sample delay reads, with a 20 ms one-pole smoother on the delay-length target — eliminates both zipper noise and click artifacts without needing hysteresis.
- Heap-allocated `Box<[f32; 4096]>` delay buffers (4 × 16 KB = 64 KB total) — too large for the stack.
- Signed anti-denormal dither (±1e-20, alternating sign) written into delay buffers every sample; FTZ/DAZ set once per `process()` call, not per-sample.
- Default chain position: **after Transformer, before Punch** (`module_order_5`), added as a new slot (`module_order_7` extends the array) rather than displacing any existing slot — preserves every pre-Haas preset.
- Linear (not equal-power) dry/wet blend — correct for correlated signals; equal-power over-boosts at mix = 0.5.

## Consequences

**Easier:**
- Users get an honestly-named, mono-safe widening tool with no risk of the "crosstalk cancellation" label setting false expectations.
- The output-trim safety net means no combination of controls can silently clip downstream modules.

**Harder:**
- No true transaural crosstalk cancellation is available in this plugin — if that's ever wanted, it requires a genuinely different algorithm (HRTF inversion), not an extension of this module.
- WideComb's depth is invisibly clamped at 0.5 internally even though the param range reads 0–1; this must stay documented so a future contributor doesn't "fix" it into an unsafe range.

**Unchanged:**
- Haas performs no EQ, bass enhancement, or saturation — those responsibilities stay with API5500 / Pultec / Transformer.
- The module-reorder system still lets users move Haas anywhere in the chain; the default-order rationale is a recommendation, not an enforced constraint.
