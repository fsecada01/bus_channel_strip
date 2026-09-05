# ADR-0005: Punch — Pre-Clip Transient Shaping, Pure-Rust Clipper, 8x Default Oversampling

**Status**: Implemented

**Date**: 2025-12 (Phases 1–4); Phase 5 (SIMD/profiling/presets) still pending
**Deciders**: Project (Claude Code orchestration session)

---

## Context

Clipping alone (used to gain headroom for louder masters) tends to produce flat, lifeless mixes: ears use transients to perceive "punch," and clipping shortens exactly the attack portion that carries that perception (Fastl & Zwicker 2007; Hirsh temporal-perception research). The module needed to restore transient character after clipping without reintroducing the peaks the clipper exists to control.

Three implementation-shape decisions had to be made: (1) where in the signal path transient shaping happens relative to the clipper, (2) whether to port Airwindows C++ algorithms (ClipOnly2, Surge, Pop2, ADClip8) via FFI or implement in pure Rust, and (3) what oversampling factor to ship as the default.

A real bug surfaced this decision's importance directly: an earlier implementation applied transient shaping **after** the clipper, which created time-varying gain modulation that was audible as pumping on every note attack.

## Decision

**Transient detection and shaping happen pre-clip; the clipper is the last stage before output.** The clipper then naturally limits whatever peaks the transient shaper produces, instead of the shaper fighting a static, already-clipped signal. This was a direct fix for the pumping bug — the fix also reduced the downsample IIR pole from 0.3 to 0.05 (it was independently contributing to the pumping) and moved the transient detector to run at native sample rate rather than the oversampled rate.

**Pure Rust implementation, not FFI**, chosen over porting Airwindows ClipOnly2/Surge/Pop2 algorithms via `cpp/`:
- ClipOnly2-equivalent transparent clipping is ~100 lines and well-understood mathematically.
- Differential-envelope transient detection (fast envelope − slow envelope) is well-documented and doesn't need Airwindows' specific implementation.
- Avoids extending the FFI surface area beyond ButterComp2 (see ADR-0008) for an algorithm simple enough not to need it.
- Better cross-platform build story — no new C++ compilation unit.

**Three clip modes** (Hard / Soft / Cubic) ship instead of one, because hard clipping is most transparent under 3 dB of gain reduction but soft/cubic knees are better for heavier clipping and add "analog" character where wanted.

**Default oversampling factor: 8x** (of the available 1x/4x/8x/16x). Rationale: 4x gives good aliasing rejection at low CPU cost but Punch's design brief called for "very good" rejection as the safe default; 16x's CPU cost is reserved for mastering use where users opt in deliberately. 8x is Airwindows' own recommended default for clipper oversampling per the source research (Alex Emrich's clipping methodology video), matching community practice.

**Default state: bypassed.** Because clipping is powerful enough to sound harsh badly misconfigured, Punch ships bypassed rather than on-by-default (contrast with Sheen, ADR-0006, which is deliberately default-on).

## Consequences

**Easier:**
- The pumping-class of bug is structurally prevented for any future clip-mode addition, since shaping always happens before the stage that limits peaks.
- No FFI/build-system surface added for this module — pure Rust keeps `cpp/` scoped to ButterComp2 only.

**Harder:**
- Reimplementing means the plugin doesn't get Airwindows' more exotic clip characters (ADClip8's multi-stage saturation, ClipSoftly's "tubey" full-waveform reshaping) without separate future work — Phase 5 (SIMD optimization, A/B testing vs. reference plugins) remains open.
- Oversampling ratio and clip-mode defaults were tuned against a specific research source (a single YouTube methodology video) rather than measured against a broader corpus — worth re-validating if the perceived character drifts from user feedback.

**Unchanged:**
- Signal-chain position is user-configurable via the module reorder system; the spec's own recommendation (end of chain, after Transformer) is a default, not an enforced constraint.
