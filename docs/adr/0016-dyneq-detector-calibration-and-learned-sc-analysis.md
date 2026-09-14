# ADR-0016: DynEQ Detector Calibration and Learned Sidechain Analysis

**Status**: Accepted — detector, bell, LINK, RANGE, trigger meter, static GAIN and DETECT implemented; learned analysis (Analyzer v2) accepted, implemented separately

**Deciders**: Project (Claude Code orchestration session), 2026-09-14

---

## Context

Three complaints about the Dynamic EQ on a drum bus traced back to measurable defects rather than tuning:

- **Drums only compressed with THRESH near −40 dB.** `DynamicEQ::process` rectified the stereo input (`max(|l|, |r|)`) *before* each band's band-pass. Rectifying moves a tone's energy to DC and 2f, so the band-pass read tonal content 12–26 dB low. Separately, a band whose DET FREQ had drifted from FREQ listened somewhere other than where it acted, and the 10 ms RMS stage under-read short hits by roughly 10 dB on top of attack smoothing.
- **Crunch in the upper mids.** Bell coefficients were redesigned only when gain reduction moved by more than a hysteresis step, so fast gain changes landed as small coefficient jumps. Each band's GAIN was a broadband multiplier, so four bands at +4–5 dB stacked into +14 dB of level into later stages. Sheen WARMTH's Inflator polynomial folded over for inputs outside [−1.5, 1].
- **ANALYZE SC suggested one band, apparently at random.** It scored a single FFT frame, took a single bin and mapped it to the nearest band, so the answer depended on which frame the click landed on.

## Decision

**Detection.** Each channel runs through its own band-pass before any rectification; per-channel RMS (or peak) is linked by taking the louder channel, then one attack/release smoother drives both channels. A module-level `dyneq_detect_mode` chooses the ballistics: `Rms` (default, 10 ms window) or `Peak` (squared band-pass output straight into attack/release). Peak is a listening choice for transient material, not a correction — RMS reading short hits low is how RMS behaves.

**Detector LINK** (`dyneq_bandN_detector_link`, default on). The detector listens at FREQ; DET FREQ is used only when LINK is off. Existing sessions load the default and therefore detect at FREQ — deliberately, since the saved mismatches were silent errors. The analyzer hides the DET FREQ marker for linked bands.

**Per-sample bell.** A `BellPrewarp` caches the frequency/Q prewarp at block rate; `apply_eq_stereo` redesigns the bell whenever its gain differs bit-for-bit from the previous sample. No hysteresis, and steady gain costs nothing. The TPT SVF (ADR-0011) keeps this transient-free.

**RANGE** (`dyneq_bandN_range`, 0–30 dB, default 18). Clamps the dynamic gain change in both directions, which also bounds Gate. Expand Up keeps its 24 dB cap, and the total bell gain (static plus dynamic) is clamped to ±24 dB so the filter can't overflow into the old NaN latch.

**Static GAIN.** GAIN is a static bell boost or cut at FREQ added to the dynamic change (`bell_dB = GAIN + Δ`), not a broadband multiplier. It matches what the control looks like beside FREQ and Q; broadband level belongs to the master output gain. Parameter IDs and ranges are unchanged.

**Trigger meter.** Each band tracks its loudest envelope per audio block and publishes it through `GainReductionData.trigger_db`. A 3 px bar under THRESH shows it on the slider's −60…0 dB scale with a threshold tick and a 1.5 s fall, amber once over threshold. It draws from the atomics directly, adding no vizia timer.

**WARMTH domain guard.** The Inflator input is clamped to [−1.5, 1.0], the polynomial's own f′ = 0 points, so the curve stays monotonic and is sample-identical inside that range.

**Learned sidechain analysis (Analyzer v2).** ANALYZE SC learns for 3 s of FFT frames instead of one, scoring only frames where the sidechain is present (at least 8). A smoothed (±1/6 octave) main×sidechain overlap score is searched greedily for up to four peaks within 24 dB of the largest, each assigned to the band whose FREQ range contains it (nearest current FREQ breaks ties), with Q from the −3 dB width clamped to [0.7, 4]. Each band's threshold comes from the main input's level history through that band's detector response: 90th percentile − 6 dB for Compress Down, 50th percentile + 3 dB for Expand Up and Gate, clamped to [−60, 0]. The work lives in a pre-allocated `spectral::MaskingLearner`, finalized one bounded step per block. The GUI offers a per-band suggestion with USE and an APPLY ALL, which write FREQ, Q, THRESH and turn LINK on.

A true sidechain *trigger* per band is deferred: it changes `process`'s signature and adds four controls, and the per-channel detector above is the seam it would use.

## Consequences

**Easier:**
- THRESH values mean what they say: a tone at −20 dB RMS in band reads −20 dB, and the trigger bar shows where hits actually land.
- Gain reduction can't run away — RANGE bounds every mode, and the bell's ±24 dB cap is enforced in one place.
- Analyzer suggestions are repeatable and cover every band the sidechain actually masks.

**Harder:**
- Saved sessions change sound in three deliberate ways: detection at FREQ (LINK on), gain reduction capped at 18 dB, and GAIN no longer lifting broadband level. Sessions that relied on GAIN for level need the master output gain instead.
- The analyzer holds a few hundred frames of 1/12-octave history (≈135 KB at 192 kHz), sized in `initialize()`.
- Peak versus RMS is a per-session judgement the user now has to make for transient material.

**Unchanged:**
- Every existing parameter ID and range. DET FREQ still loads and still works with LINK off.
