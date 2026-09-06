# ADR-0014: ButterComp2 Program-Dependent Release Adaptation

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session), issue #18

---

## Context

ButterComp2 (the FFI-wrapped Classic model, `cpp/buttercomp2.cpp`) uses one fixed release-time coefficient (`release_speed = 0.001 / sample_rate`) regardless of program material. Issue #18 asked for a "slight" program-dependent adaptation — the release ballistic should respond differently to transient-dense material (drum bus) versus sustained material (vocal bus), matching how analog glue compressors (SSL bus comp, dbx-style program-dependent release) behave, instead of one shape for everything. Decided 2026-09-06 (open question resolved via user sign-off on the issue): auto-on with a single bypass toggle, no continuous "amount" parameter — matching the precedent set by #16's hysteresis bypass.

Scope is deliberately narrow: `src/buttercomp2.rs` contains four unrelated compressor implementations sharing one file (`FetCompressor`, `VcaCompressor`, `OpticalCompressor`, and the FFI-wrapped `ButterComp2` struct). The issue's language ("ButterComp2's bipolar interleaved compression algorithm," "touches the FFI-wrapped ButterComp2 state") targets only the Classic model — the other three already have their own auto-release/dynamics behavior and are out of scope here.

A pre-existing but unused `double dynamic_release = release_speed * release_mod;` computation was found dead in the C++ file, with no other reference anywhere — evidence of a prior half-finished attempt at exactly this feature. Removed as directly superseded by this implementation, not unrelated cleanup.

## Decision

Add a per-block crest-factor (peak/RMS, stereo-linked, measured from the *dry* input before compression) measurement in `buttercomp2_process_stereo`, mapped to a release-time scale multiplier and applied to the existing `release_speed` coefficient:

```
target_scale = 2^((kCrestRefDb - crest_db) / kCrestRangeDb), clamped to [0.6, 2.0]
```

`kCrestRefDb = 9.0` is treated as a "neutral" well-mixed bus signal. Crest factor *above* the reference (transient/bursty material) maps to a *smaller* scale — the release-tracking coefficient (`dynamic_release_speed = release_speed * release_scale`) shrinks, so the compressor's gain-reduction state converges more slowly ("softens," avoiding audible pumping on percussive hits). Crest factor *below* the reference (sustained, low-crest material like a held vocal or bass note) maps to a *larger* scale, tightening the response. The `[0.6, 2.0]` bound is deliberately modest — "slight" per the issue — versus FetCompressor's existing ~27x auto-release span.

The scale multiplier itself is block-rate smoothed (`kCrestSmoothTcSeconds = 0.1`, a one-pole with `exp(-block_duration/tc)`) so it tracks the material's overall character rather than jittering block-to-block, and is applied **causally**: block N is processed using the smoothed scale *left over from before this call* (captured before the crest stats loop runs), then block N's own stats update the smoothed value for block N+1. Zero added latency, no lookahead.

`ButterComp2State` gains two fields: `adaptive_envelope_bypass` (bool, default `false` — adaptive on) and `crest_scale_smoothed` (f64, default/reset value `1.0` — neutral, matching the original fixed-shape baseline exactly when bypassed). A new FFI setter, `buttercomp2_set_adaptive_envelope_bypass`, follows the existing `buttercomp2_set_compress`/`_set_output`/`_set_dry_wet` pattern; `ButterComp2::update_parameters` in `src/buttercomp2.rs` gained a fourth `bool` argument threading it through. A new `comp_adaptive_env_bypass: BoolParam` (default `false`) was added to `ButterComp2Params`, gated `#[cfg(feature = "buttercomp2")]` like the model's other controls, and wired into `process_module_buttercomp()`'s `ButterComp2Model::Classic` arm in `src/lib.rs`. `buttercomp2_reset()` resets `crest_scale_smoothed` to `1.0` so a transport restart doesn't carry over a stale learned value from unrelated prior material.

### `std::clamp` portability

`build.rs` only sets `/std:c++17` on the `windows-msvc` target branch; `windows-gnu` and `apple-darwin` rely on the toolchain default, not guaranteed ≥C++17. `std::clamp` was avoided in favor of `std::max(min, std::min(max, x))`, matching this file's own pre-existing clamping idiom used for `compress`/`output`/`dry_wet`.

## Consequences

**Easier:**
- The crest measurement is a fixed-size, allocation-free pre-pass over the block already in hand — no new buffering, no lookahead, no violation of the audio-thread rules in `docs/SYSTEM_PROMPT.md`.
- Reuses the existing `release_speed` coefficient's role as a single scalar multiplier rather than introducing a second, parallel envelope-follower path — the change is a multiply, not a rewrite of the gain-reduction math.

**Harder:**
- The `[0.6, 2.0]` bound and `kCrestRefDb`/`kCrestRangeDb` constants were chosen by reasoning about the issue's "slight" wording and FetCompressor's existing auto-release span as a reference point, not tuned by ear against real program material — a future revisit should A/B against real drum-bus/vocal-bus sources if the character is challenged.
- Because `buttercomp2_process_stereo` is called once per host buffer (not once per fixed block size), the crest measurement's effective time window varies with host buffer size. This is consistent with how the rest of the file already treats `num_samples` as the processing unit, but means the *exact* adaptation speed is host-dependent even though the `kCrestSmoothTcSeconds` time constant itself is not.

**Unchanged:**
- Parameter IDs for every existing ButterComp2 control. `comp_adaptive_env_bypass` is additive.
- Output when `comp_adaptive_env_bypass` is `true`: bit-identical to pre-#18 behavior (`release_scale` pinned to `1.0`).
- The other three compressor models sharing this file (`FetCompressor`, `VcaCompressor`, `OpticalCompressor`) — untouched.

## DoD note

"No measurable CPU regression" was not empirically profiled — the crest pre-pass is O(block_size) with no allocation, matching the existing per-block cost class, but this is reasoned rather than measured.
