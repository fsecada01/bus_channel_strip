# ADR-0014: Punch True-Peak Detection (ITU-R BS.1770-4)

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session), issue #19

---

## Context

Punch's clipper previously metered peaks at the oversampled rate (4x/8x/16x, user-selectable), which is closer to true-peak than host-rate metering but still not a proper ITU-R BS.1770-4 true-peak measurement, and nothing in the GUI displayed the reading at all. Issue #19 asked for: (1) a dedicated true-peak (intersample) detector on Punch's final output, (2) that reading surfaced in the GUI, and (3) the clipper ceiling's default changed from its prior -0.1 dB to -1.0 dBTP, leaving headroom for DAC reconstruction/intersample overshoot.

ITU-R BS.1770-4 requires a minimum of 4x oversampling for intersample-peak detection but does not mandate a specific interpolation kernel — only adequate stopband attenuation. The project already has a halfband-Kaiser-windowed `Oversampler` (`src/oversampler.rs`, ~-40 dB stopband, used by the clipper's own anti-aliasing oversampling), so no new filter design was needed.

## Decision

Add a private `TruePeakDetector` in `src/punch.rs`, one per channel, each wrapping its own `Oversampler::new_at_factor(4, max_block_size)`. The detector only ever calls `upsample()` — a peak meter never needs to reconstruct back to native rate, so `downsample()` is never called. For each input sample, the detector takes `max(|s|)` across the 4 interpolated points the oversampler returns for that interval, converts to dB, and applies peak-hold-then-decay ballistics (300 ms hold, 20 dB/s decay — a standard PPM-style ballistic) so a brief excursion stays legible on the meter rather than flickering.

The detector measures Punch's **final output** (post input-gain, post transient-shaping, post-clip, post-mix, post-output-gain) — not the pre-clip or oversampled-clipper-internal signal — so the reading always reflects what will actually reach the DAC.

GUI surfacing follows the existing `GainReductionData` lock-free pattern (`src/spectral.rs`, used for DynEQ's per-band gain-reduction display): a new `TruePeakData { channels: [AtomicU32; 2] }` struct, written by the audio thread with `Ordering::Relaxed` (display-only, staleness is fine) right after `self.punch.process(buffer)` in `lib.rs::process_module_punch`, and read every frame by a new `PunchTruePeakMeter` custom vizia `View` (mirroring `SpectrumCanvas`'s draw-time atomic-read pattern) rendered as a small two-row bar meter in Punch's control panel.

`punch_threshold`'s (`src/params/punch.rs`) default changed from -0.1 dB to -1.0 dB. Issue #19's own text cited the prior default as "-0.3 dBFS," which doesn't match the actual prior code default (-0.1 dB) — the change is from the real prior value, not the issue's stated one.

## Consequences

**Easier:**
- `TruePeakDetector` is a small, self-contained primitive reusing existing oversampling infrastructure — no new filter design, no new heap allocation beyond the `Oversampler`'s own pre-allocated scratch buffers.
- The GUI wiring is a direct copy of an already-proven pattern (`GainReductionData` → `SpectrumCanvas`), so no new lock-free-metering design risk.

**Harder:**
- None identified — this is additive metering plus one parameter default change, with no structural changes to the clipper's signal path.

**Unchanged:**
- No parameter IDs added or changed. `punch_threshold`'s ID, range, and unit are untouched — only its default plain value changed, which affects only newly-created sessions/presets (existing saved sessions store their own value and are unaffected).
- The clipper's own signal path (input gain → transient shape → oversample → clip → downsample → wet HPF → mix → output gain) is unchanged; true-peak detection is a read-only tap on the final output, not a new stage in the chain.
