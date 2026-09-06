# ADR-0011: TPT State-Variable Filter Core + Pultec Linear-Phase Mode

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session), issue #15

---

## Context

Every EQ stage (API5500, Pultec, DynamicEQ, Sheen) ran on `biquad::DirectForm1`. Two problems motivated a replacement: DynamicEQ recomputes coefficients on nearly every sample under active gain reduction, and DF1's delayed-sample state means a coefficient swap mid-stream briefly evaluates a mix of old and new coefficients — audible as a burst under fast modulation. Separately, the roadmap (§3.2) asked for a linear-phase option on Pultec so mastering-style use doesn't shift the track's phase.

## Decision

Replace `DirectForm1` with a trapezoidal-integrated (TPT/ZDF) state-variable filter core (`src/svf.rs`), following Andrew Simper's `SvfLinearTrapOptimised2` formulation. The state variables are the integrator outputs themselves rather than delayed samples, so a coefficient swap lands on the new response with no transient — this is what makes it safe to call `update_coefficients` every sample. It nulls against the old DF1 cores to f32 rounding (see `svf.rs` tests), so existing sessions keep their tonality.

For Pultec's linear-phase mode, add `LinearPhaseEngine`: an overlap-save FFT convolver whose 513-tap kernel is sampled from the same TPT coefficients the minimum-phase chain uses (`design_kernel`), so the two modes measure the same and differ only in phase. Latency is a fixed 512 samples (`LP_HOP` + half the kernel length).

Two follow-on decisions from the initial implementation:

- **Kernel redesign is throttled** (`KERNEL_REDESIGN_MIN_INTERVAL = LP_HOP`). Pultec's params carry no smoother, so automation could otherwise re-run the 4096-point IFFT + 5-stage evaluation + 1024-point FFT on nearly every sample. Throttling to once per hop bounds the cost to the same order as the convolution itself, at the cost of up to one hop of latency before a parameter change reaches the kernel.
- **Reported latency tracks the `pultec_linear_phase` toggle alone, not module-order membership.** An earlier version zeroed latency when Pultec left the active chain, but that meant dragging it out of the rack mid-playback silently dropped the FIR's buffered audio on the same block the reported latency dropped to 0. `process()` now keeps draining the FIR via `process_bypassed()` whenever Pultec isn't dispatched but linear-phase is still engaged, so the signal always carries whatever delay the plugin promised the host.

API5500 also gained a `reset()` it never had (a real gap surfaced during review, unrelated to the migration itself). Sheen's EQ filters are deliberately *not* reset on transport start — see its own `reset()` doc comment — since the leading silence lets residual energy decay naturally and resetting would introduce a step.

## Consequences

**Easier:**
- One filter core (`svf.rs`) backs every EQ stage; adding a new response type is a `SvfType` match arm, not a new struct.
- DynamicEQ's per-sample coefficient recomputation is finally artifact-free.
- Linear-phase mode reuses the exact same coefficient math as minimum-phase, so the two can't drift apart.

**Harder:**
- The FIR kernel design (`design_kernel`) is a second, more expensive code path that must stay allocation-free and throttled — any future stage added to Pultec's chain needs its coefficients folded into `stage_coefficients()` and accounted for in the kernel designer.
- Latency reporting is now stateful across `process()` calls (`pultec_reported_latency`) rather than a pure function of the current block; changes to the dispatch/bypass logic in `lib.rs` must keep `process_bypassed()`'s fallback drain in sync with whatever determines latency.

**Unchanged:**
- Parameter ranges and IDs — this is a topology swap under the hood, not a preset-breaking change.
