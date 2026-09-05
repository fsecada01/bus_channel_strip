# ADR-0008: ButterComp2 — FFI Wrapper over Airwindows C++, Not a Rust Port

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session)

---

## Context

Airwindows' ButterComp2 implements a "bi-polar, interleaved compression" design: four distinct compressors per channel running in parallel (two sensitive to positive swing, two to negative, alternating every sample at Nyquist to create a "butterfly" processing pattern), with dynamic release control that adapts to output level and custom dithering for noise shaping. This is a nontrivial, idiosyncratic algorithm with specific numerical behavior (double-precision internal state, class-AB push-pull processing) that gives ButterComp2 its characteristic "glue" sound. The question was whether to reimplement this algorithm from scratch in Rust or wrap the original C++ source via FFI.

## Decision

**Wrap the original Airwindows C++ source via `extern "C"` FFI**, compiled through `build.rs` using the `cc` crate, rather than porting the algorithm to Rust. The state struct (`controlAposL/R`, `controlBposL/R`, `targetposL/R`, `lastOutputL/R`, dithering state, per the original `ButterComp2.h` layout) is heap-allocated once at initialization and accessed via pointer from the Rust side — never allocated on the audio thread.

Rationale: the algorithm's exact numerical character (the specific interleaved-compressor topology, the dynamic divisor tied to output level, the power-function compression curve) is precisely what gives ButterComp2 its "glue" and "holographic stereo" reputation. A from-scratch Rust reimplementation risks subtly diverging from that character in ways that are hard to detect by code review and only surface as "this doesn't sound like ButterComp2 anymore." Wrapping the reference implementation directly guarantees numerical parity with the plugin the algorithm is named after.

This is the only FFI boundary in the plugin — every other module (including Punch, which also considered porting Airwindows algorithms; see ADR-0005) is pure Rust. ButterComp2 is the exception because it's wrapping a *specific, already-correct* algorithm rather than implementing a general DSP technique from research.

## Consequences

**Easier:**
- Guaranteed numerical/sonic parity with the original Airwindows ButterComp2 — no risk of "reimplementation drift."
- The FFI surface is small and stable (3 parameters: Compress, Output, Dry/Wet) — little ongoing maintenance burden.

**Harder:**
- `cpp/buttercomp2.cpp` is a second language and build toolchain the project must keep working across all target platforms (Windows/macOS/Linux, and specifically the macOS ARM64→x86_64 cross-compile path — see [[reference_cicd_lessons]]).
- Every call into the C++ state must be wrapped in `unsafe` with an explicit safety comment justifying pointer validity and audio-thread-safety — this is the one module where that discipline is load-bearing rather than incidental.
- Adding a genuinely new compressor character (as opposed to wrapping an existing reference implementation) should default to pure Rust per ADR-0005's precedent, not extend this FFI pattern, unless a future module is likewise wrapping a specific existing algorithm whose exact character matters.

**Unchanged:**
- Parameter automation, smoothing, and NIH-plug integration for ButterComp2 work identically to every pure-Rust module from the plugin-parameter-system's point of view — the FFI boundary is invisible above the `buttercomp2.rs` wrapper.
