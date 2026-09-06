# ADR-0013: Stereo Micro-Detuning (TMT-Style Per-Channel Coefficient Deviation)

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session), issue #17

---

## Context

Every stereo module in the plugin processes L and R with identical filter coefficients and only separate per-channel state (`feedback_stereo_biquad_state.md`). That's correct for "transparent," but it's part of what makes the plugin read as "digital" — real analog channel strips never have two physically identical channels; component tolerances give each side (and each unit) a slightly different corner frequency. Issue #17 asked for per-channel coefficient micro-detuning — ±0.3% deviation from nominal, seeded once per plugin instance and fixed for its lifetime — across API5500, Pultec, Transformer, and Sheen.

The issue's own open question — hard-code the ±0.3% deviation, or expose it as a `tmt_amount` parameter — was signed off 2026-09-06 per the issue's own recommendation: hard-code, no new parameter. A "how much randomness" knob adds choice paralysis without a real benefit; 0.3% is Brainworx's published number for `bx_console`.

## Decision

Add `src/detune.rs`: `DetuneRng`, a minimal splitmix64 PRNG (no `rand` crate dependency exists in this project), seeded once via `SystemTime`-derived entropy at construction time (off the audio thread — every module's `new()` runs during plugin instantiation, never inside `process()`). `micro_detune_pair(rng)` draws a `[f32; 2]` pair, each element `1.0 ± (up to TMT_MAX_DEVIATION)` where `TMT_MAX_DEVIATION = 0.003`. `reset()` — which per `lib.rs`'s own doc comment "can be called from the audio thread and may not allocate" — re-draws the pair via pure integer arithmetic (`next_u64`'s splitmix64 step), never a fresh OS-entropy call.

One seed/detune-pair **per module instance**, not per band — matching the issue's own "~16 bytes per module" trade-off claim. Every filter stage within a module shares that module's single `detune: [f32; 2]`.

Per-module integration:

- **API5500** (`Filter::new`/`update_parameters` gained a `detune: [f32; 2]` parameter, applied via `SvfCoefficients::new_detuned_pair`): `update_parameters()` runs unconditionally every buffer (`lib.rs::process_module_api5500`), so `reset()` redrawing `self.detune` alone is sufficient — the next buffer's unconditional recompute picks it up.
- **Pultec**: same unconditional-every-buffer pattern as API5500 for the five SVF stages. The linear-phase FIR kernel (`src/linear_phase.rs` cascade) stays a **single shared kernel**, not per-channel — doubling the FFT/convolution engine to detune it would violate the issue's "CPU identical" trade-off claim for a linear-phase mode that's off by default. It approximates with channel 0's detuned corner; the decorrelation is real in minimum-phase mode (the default) where each channel's `TptSvf` runs independently.
- **Sheen**: BODY/PRESENCE/AIR (`Filter`, same primitive as API5500) gate coefficient recompute behind manual `dirty_body`/`dirty_presence`/`dirty_air` flags set only on slider movement — `reset()` must *also* force those flags `true`, or a freshly-redrawn detune never takes effect until the next parameter move. WIDTH's `width_hpf`/`width_shelf` are intentionally excluded: they only ever see the mono-derived M/S side signal, so there's no L/R to decorrelate.
- **Transformer**: `low_shelf`/`high_shelf` were a single shared `DirectForm1<f32>` (not per-channel) processed sequentially for both L and R through the same filter state — the exact anti-pattern `feedback_stereo_biquad_state.md` documents, discovered as a byproduct of this work. Splitting to `[DirectForm1<f32>; 2]` was necessary plumbing to detune them per-channel, not scope creep. `update_frequency_response()` only runs when the cached model/response values change (`update_parameters()`'s own dirty check) — `reset()` calls it directly with the last-known cached values (skipped if still the NaN sentinel, i.e. before the first real `update_parameters()` call) so a redrawn detune takes effect immediately rather than waiting for the next parameter move.

## Consequences

**Easier:**
- `DetuneRng`/`micro_detune_pair` is a single reusable primitive — any future module needing the same per-instance stereo variance reuses it directly.
- No new automation surface, no preset-compatibility question: the deviation is hard-coded per the issue's sign-off.

**Harder:**
- Pultec's linear-phase mode only approximately reflects the detune (channel 0's corner drives the shared kernel design) — an intentional, documented trade-off, not an oversight.
- Transformer's shelf filters required a structural change (shared → per-channel `DirectForm1`) beyond what a minimal "add a detune field" change would need — worth flagging in review since it touches more of the file than the other three modules.

**Unchanged:**
- No parameter IDs added or changed in any of the four modules.
- API5500's pre-existing biquad-reference null test needed one line (`eq.detune = IDENTITY_DETUNE`) to keep asserting exact equality against the untuned reference — the only pre-existing test across all four modules that ±0.3% detune broke.
