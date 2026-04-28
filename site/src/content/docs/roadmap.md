---
title: v2.0 Roadmap
description: "Stop sounding digital." A perceived-quality release targeting the four artifacts that read as digital — top-octave aliasing, midrange phase smear, lifeless saturation, and overly-precise stereo. ~3-month scope.
---

v2.0 is a **perceived-quality release**, not a feature dump. The chassis identity, signal chain, and module set stay the same. We make every existing module sound and feel substantially better, and we ship the missing UI surface (presets, HiDPI, resizing, themes) that v1.0 left for later.

The full requirements specification lives in [`docs/V2_ROADMAP.md`](https://github.com/fsecada01/bus_channel_strip/blob/main/docs/V2_ROADMAP.md) on GitHub. This page is the public-facing summary.

## Theme: "Stop sounding digital"

The meme targets four specific perceptual artifacts. v2.0 fixes each:

| Symptom | DSP cause | v2.0 fix |
|---|---|---|
| Aliased top octave | 1× host-rate processing in nonlinear modules | Internal 4× oversampling everywhere with nonlinearity |
| Smeared / phasey midrange | Cascaded biquads accumulate phase | TPT state-variable topology for EQ; optional linear-phase mode for mastering |
| Lifeless saturation | Memoryless waveshapers | Hysteresis modeling in Transformer + Sheen WARMTH (Preisach-lite) |
| Overly-precise stereo | Identical L/R coefficients, no decorrelation | Per-channel coefficient micro-detuning (TMT-style); subtle inter-channel crosstalk |

## DSP track (weeks 1-6)

- [ ] **Universal 4× internal oversampling** across every module containing a nonlinearity (ButterComp2, Pultec tube saturation, Transformer, Sheen WARMTH). Punch oversampling normalized as a global quality preset.
- [ ] **TPT (topology-preserving transform) state-variable filter cores** replace biquad direct-form-1 in API5500, Pultec, DynamicEQ, and Sheen EQ stages — eliminates phase smear at extreme Q and cleanly accepts coefficient automation.
- [ ] **Optional linear-phase mode** on Pultec (mastering EQ) — windowed-sinc FIR with FFT convolution, ~512-sample latency at 4×.
- [ ] **Single-cell Preisach hysteresis model** in Transformer core + as an opt-in "tape" mode on Sheen WARMTH — gives saturation actual *memory* (output depends on input history, not just present input).
- [ ] **Per-channel coefficient micro-detuning** (TMT-style ±0.3%) on API5500, Pultec, Transformer, Sheen — produces natural stereo decorrelation rather than the unnaturally precise center image of identical L/R coefficients.
- [ ] **ButterComp2 adaptive envelope** — program-dependent attack curve that softens on transient-dense material.
- [ ] **Punch true-peak detection** (ITU-R BS.1770-4) with intersample peak metering and a -1 dBTP default ceiling.

## UI track (weeks 4-10)

- [ ] **Resizable window** (1100×750 to 2400×1400, locked aspect) with HiDPI awareness — currently locked at 1300×860.
- [ ] **Preset system** — 20 factory presets across genres, browser UI with categories, user save/load to `~/Documents/Bus Channel Strip/Presets/`, current-preset name in chassis header with diff indicator.
- [ ] **Per-module inline metering** — tiny reactive spectrum strip atop each EQ module, consistent GR meters on compressors, saturation meters on Transformer / Punch / Sheen WARMTH.
- [ ] **Visual polish pass** — typography rhythm, spacing rhythm, knob spring-back micro-interactions, follow-cursor value tooltips during drag, custom SVG icon set replacing Unicode glyphs.
- [ ] **Theming** — ships with two curated themes (Studio dark, Daylight bright). User-customizable colors deferred to v2.1.
- [ ] **Tooltips** — hover any control >800ms for control name + one-sentence description.

## Foundation work (opportunistic, no dedicated track)

- [ ] **`lib.rs` refactor** — extract per-module parameter definitions into a `params/` module (forced by the preset system work).
- [ ] **CI/CD fix** — repair the macOS / Linux GitHub Actions builds that currently fail (forced by the signed-installer work).
- [ ] **macOS code signing + notarization** — required for "broader audience" distribution.
- [ ] **Installer packages** (Windows MSI, macOS pkg) replacing zip extraction.
- [ ] **Test coverage expansion** — every new DSP module tested at the same density as Sheen.

## Compatibility notes (planned)

- Parameter IDs from v1.0 stay stable.
- TPT EQ migration is a *breaking sound change* — sessions saved in v1.0 will load and play in v2.0 but null within ~0.1 dB of the biquad versions at moderate settings, larger at extreme Q.
- Hysteresis on Transformer is a *bigger sound change* — a new `transformer_hysteresis_bypass` BoolParam (default OFF, i.e. hysteresis on) lets users flip back to v1.0 behavior for bit-identical playback.

## Phasing

```
Week 1-2:   TPT filter prototype (one module: API5500), validation
Week 2-4:   4× oversampling rollout across nonlinear modules
Week 3-5:   Hysteresis model implementation + tuning
Week 4-6:   Stereo decorrelation; ButterComp2 adaptive envelope
Week 4-5:   UI: lib.rs refactor + params/ module extraction
Week 5-7:   UI: preset system (data model + browser UI + 20 factory presets)
Week 6-8:   UI: resizable window + HiDPI
Week 7-9:   UI: per-module inline metering + visual polish pass
Week 8-9:   UI: theming + tooltips
Week 10-11: CI fix; macOS signing + notarization; installers
Week 11-12: Final tuning, release notes, v2.0 tag, GitHub release
```

If any track slips hard, the cuts (in order) are: linear-phase Pultec mode → Daylight theme → custom SVG icons → ButterComp2 adaptive envelope.

## Deferred to v2.1+

- A/B compare snapshots
- MIDI parameter learn
- Multi-instance link (matched processing across stems)
- Sidechain routing for non-DynEQ modules
- New DSP modules
- User-customizable theme colors
- Workflow features generally

## Success criteria

v2.0 is "done" when:

1. AB-comparing v1.0 and v2.0 on a full mix, the difference is immediately audible and consistently described as "more open" or "less digital" by at least 3 outside listeners.
2. Window resizes smoothly across at least 3 sizes without layout regressions.
3. The 20 factory presets cover the documented genres and load instantly without parameter glitches.
4. Signed installers work on a fresh Windows 11 + macOS 14 install without security warnings.
5. CPU on a single instance with full chain at 4× oversampling stays under 15% on a 2020-era M1 Mac mini at 48 kHz / 256-sample buffer.

---

**Want to track progress?** The same checklist with live status lives in the project [README](https://github.com/fsecada01/bus_channel_strip#coming-in-v20). The full requirements spec with open architectural questions is in [`docs/V2_ROADMAP.md`](https://github.com/fsecada01/bus_channel_strip/blob/main/docs/V2_ROADMAP.md).
