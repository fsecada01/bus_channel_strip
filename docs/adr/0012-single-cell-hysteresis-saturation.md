# ADR-0012: Single-Cell Play-Operator Hysteresis for Transformer + Sheen WARMTH

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session), issue #16

---

## Context

Transformer's saturation and Sheen's WARMTH polynomial are both memoryless: output depends only on the present input sample. Real magnetic gear (tape, transformer cores) has hysteresis — output depends on input *history* — which is part of what makes it sound "alive" (roadmap §3.3). Issue #16 asked for a Preisach-lite model, with an open question on depth: single-cell play-operator (cheap, ~80% of the perceptual character) or a full multi-cell Preisach (~30x the per-sample cost). Decided 2026-09-05: single-cell for v2.0, per the issue's own recommendation.

## Decision

Add `HysteresisCell` (`src/hysteresis.rs`): the discrete play (backlash) operator `y[n] = max(x[n]-r, min(x[n]+r, y[n-1]))`, one `f32` of state. `r` (the loop half-width) is `MAX_THRESHOLD * amount`, where `amount` reuses each module's existing intensity knob — `saturation_amount` for Transformer, `warmth_effect` (mix) for Sheen. No new "hysteresis amount" parameter was added; this keeps the change scoped to what #16 asked for.

Integration points:
- **Transformer** (`TransformerStage::process_sample`): the play operator runs on each oversampled sample, between the drive stage and the model-specific saturation curve (`saturate_by_model`). A new `transformer_hysteresis_bypass` BoolParam (default `false` — hysteresis ON) drives `r` to 0 when bypassed — an exact passthrough (see `HysteresisCell::process`'s `r < 1e-6` branch), making that path bit-identical to pre-#16 v1.0 output, the migration requirement from roadmap §6, while still calling the cell every sample so `y_prev` keeps tracking the live signal instead of freezing (which would click on re-engagement). `TransformerStage` is a single instance shared by the L and R channels — only the oversamplers are per-channel — so the cell is `[HysteresisCell; 2]` indexed by a `ch` parameter threaded through `process_sample`; a single shared cell would let each channel's play-operator memory bleed into the other's output, caught in review before merge.
- **Sheen WARMTH** (`SheenModule::process_warmth`): a new `sheen_warmth_tape_mode` BoolParam (default `false` — off) optionally routes the oversampled signal through the cell before the Inflator polynomial, using the same zero-amount-when-off passthrough pattern as Transformer. Default off means WARMTH's factory sound is unchanged unless a user opts in — no migration note needed here, unlike Transformer. Sheen's hysteresis was already `[HysteresisCell; 2]` at the module level (module-per-channel state was already correct here, unlike Transformer's shared-stage layout above).

Both integrations run the cell at the oversampled rate (inside the existing 4x loop), not at native rate before oversampling. The play operator's threshold is amplitude-based, not time-based, so this is a resolution choice, not a smoothing choice — running it at the finer oversampled grid is closer to how the underlying physical hysteresis behaves continuously, and required no new oversampler plumbing since both stages already loop at that rate for their existing nonlinearity.

Both modules flush their hysteresis cells' state on `reset()`. Sheen's WARMTH deactivation-flush path (added in #27 for `warmth_os`) now also resets `warmth_hysteresis`, for the same reason: stale state must not leak into the stage's next activation.

## Consequences

**Easier:**
- `HysteresisCell` is a single reusable primitive; the multi-cell path (if #16's DoD's "reconsider for 2.1" ever triggers) would run several of these in parallel and sum outputs, not require new machinery.
- No new automation surface — reusing existing knobs means no preset-compatibility question beyond the two new bypass/mode bools, which default to preserving prior behavior.

**Harder:**
- The cell's threshold scaling (`MAX_THRESHOLD = 0.15`) was tuned by ear/inspection against the cited references (Holters & Parker DAFx-18; Airwindows ToTape6), not derived from a closed-form fit — a future revisit should re-tune against real tape/transformer measurements if the "80% of perceptual character" claim is challenged.

**Unchanged:**
- Parameter IDs for every existing Transformer/Sheen control. The two new BoolParams are additive.

## Amendment (2026-09-14): level-relative loop width

User report: Console/Tape produced severe distortion at saturation values above 0.1 on both input and output stages. Root cause was the absolute half-width `r = 0.15 * amount`: it acts as a dead band, so any signal smaller than `r` was held flat (silence below roughly -29 dBFS at saturation 0.59) and everything else got crossover-style distortion. THD for a -24 dBFS sine at saturation 0.1 measured -27 dB.

Changes:
- `HysteresisCell` now scales `r` by a per-cell peak follower (100 ms release): `r = 0.06 * amount * peak`. The loop has the same shape at any level. The cell takes the rate it is clocked at in `new`, and clamps the tracked level so NaN input cannot latch it.
- Transformer applies hysteresis to the wet path only (`saturate_by_model(dry, wet_in, ..)`); the dry share of the blend no longer carries the loop's lag.
- Transformer's loading compressor has per-channel state, a 150 ms detector release and 20 ms gain smoothing (it previously used a shared stereo envelope with a ~0.5 ms release, which gain-modulated at waveform rate). It now runs even when saturation is below 0.01.
- The Modern curve is `d / sqrt(1 + k·d²)`, which is monotonic (the old `d / (1 + k·d²)` folded back).

Migration: existing sessions using Transformer will sound cleaner at low and moderate saturation, and `transformer_hysteresis_bypass = true` is no longer bit-identical to v1.0 because the compressor changed. Sheen's opt-in tape mode picks up the same level-relative cell. No parameter IDs changed.
