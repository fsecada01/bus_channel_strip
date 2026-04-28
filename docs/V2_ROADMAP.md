# Bus Channel Strip 2.0 — Roadmap Requirements

Status: **Draft**, awaiting sign-off (2026-04-28). This is a requirements
spec, not an implementation plan. Once approved, individual themes get
their own `docs/*_SPEC.md` files before any code lands.

## 1. Goals & non-goals

### Theme

**"Stop sounding digital."** v2.0 is a perceived-quality release. The
chassis identity, signal chain, and module set stay the same; we make
every existing module sound and feel substantially better, and we ship
the missing UI surface (presets, HiDPI, resizing, themes) that v1.0
left for later.

### Audience

Primary: technical-mixing-nerds who already ship with the plugin (the
v1.0 core audience).
Secondary: a broader cohort reachable through better presets, signed
installers, and a usable factory experience out of the box.

### Time budget

**~3 months.** Rough split:
- Weeks 1-6: DSP track
- Weeks 4-10: UI track (overlaps DSP)
- Weeks 9-12: Integration, tuning, release

### Non-goals (deferred to 2.1+)

- A/B compare snapshots
- MIDI parameter learn
- Multi-instance link
- New DSP modules
- Sidechain routing for non-DynEQ modules
- Workflow features generally

If a workflow feature appears here, it's because it falls naturally out
of the DSP or UI work and costs nothing extra. Otherwise it waits.

## 2. The "sounds digital" diagnosis

The meme targets four specific perceptual artifacts. v2.0 fixes each:

| Symptom | DSP cause | v2.0 fix |
|---|---|---|
| Aliased top octave | 1× host-rate processing in nonlinear modules | Internal 4× oversampling everywhere with nonlinearity |
| Smeared / phasey midrange | Cascaded biquads accumulate phase | State-variable / TPT topology for EQ; optional linear-phase mode for mastering |
| Lifeless saturation | Memoryless waveshapers | Hysteresis modeling in Transformer + Sheen WARMTH (Preisach-lite) |
| Overly-precise stereo | Identical L/R coefficients, no decorrelation | Per-channel coefficient micro-detuning (TMT-style); subtle inter-channel crosstalk |

These four fixes constitute the DSP track.

## 3. DSP track (weeks 1-6)

### 3.1 Universal 4× internal oversampling

**Status quo:** Punch runs up to 8× (user-selectable). Sheen WARMTH
runs at 2×. API5500 / Pultec / DynamicEQ / Transformer / Haas all run
at 1× host rate. ButterComp2 runs at 1×.

**Target:** Every module that contains a nonlinearity (saturation,
clipping, harmonic generator) runs at **4× internal sample rate**
unconditionally. Linear stages (pure EQ, pure compression with
linear-domain gain) stay at host rate.

**Modules affected:** ButterComp2, Pultec (tube saturation), Transformer
(saturation core), Punch (already higher; expose 2× / 4× / 8× as a
quality preset rather than per-module), Sheen WARMTH (bump 2× → 4×).

**Trade:** ~2-3× CPU on a full-chain instance. Acceptable on a master
bus where one instance does the work of seven; not acceptable on
per-channel inserts (which is not our use case).

**Open question:** Do we ship a "CPU saver" toggle that drops to 2× for
older hardware, or is 4× the floor? My recommendation: 4× is the floor.
A polish plugin that requires CPU restraint sells the wrong story.

### 3.2 Filter topology upgrade

**Status quo:** Every EQ uses biquad direct-form-1 (RBJ cookbook
coefficients). Adequate for ±6 dB at moderate Q; beyond that, phase
accumulates audibly across the chain. The `biquad` 0.5.0 frequency-
normalization workaround is in place (`shaping::biquad_coeffs`).

**Target:** Replace the EQ filter cores with **TPT (topology-preserving
transform) state-variable filters** for API5500, Pultec, and Sheen's
EQ stages. TPT preserves analog-prototype phase response, has stable
behavior at extreme Q, and accepts coefficient automation without state
glitches.

**Optional v2.0 stretch:** A **linear-phase mode** toggle on Pultec
specifically (not API5500 — Pultec is the mastering EQ; API is the
console EQ where minimum-phase is part of its character). Linear-phase
adds latency (~512 samples at 4×) but eliminates phase smear entirely.
Use a windowed-sinc FIR with FFT convolution.

**Modules affected:** API5500, Pultec, DynamicEQ (4 bands), Sheen
(BODY + PRESENCE + AIR + WIDTH-shelf).

**Open question:** Do we keep the existing biquad path as a fallback
for legacy session loads, or migrate everything? My recommendation:
hard-migrate. State-variable filters will null against the old biquads
within ~0.1 dB at moderate settings; users won't notice except that
their mixes sound better.

### 3.3 Hysteresis-modeled saturation

**Status quo:** Transformer's saturation is a memoryless `tanh`-style
waveshaper with frequency-dependent input EQ. Sheen WARMTH is a
memoryless polynomial. Both produce harmonics that match real hardware
*statically* but lack the **memory** that makes real magnetic gear feel
"alive" — output depends only on present input, not on input history.

**Target:** Add a **Preisach-lite hysteresis model** (single-cell
play-operator, not full Preisach) to Transformer's core and as an
optional mode on Sheen WARMTH. The play-operator adds ~50 samples of
internal state and produces the asymmetric / level-history-dependent
distortion signature characteristic of Studer tape and Carnhill iron.

**Modules affected:** Transformer (primary), Sheen WARMTH (secondary,
as a "tape" sub-mode toggle).

**Reference:** Holters & Parker, "A Combined Model for a Bucket
Brigade Device and Its Input and Output Filters" (DAFx-18); Wright &
Välimäki, "Real-Time Black-Box Modelling with Recurrent Neural
Networks" (DAFx-19, for context); the Airwindows "ToTape6" source
comments by Chris Johnson for tuning intuition.

**Open question:** How aggressive? A faithful Preisach is expensive
(~30 cells × per-sample × per-cell hysteresis math). Single-cell play
is cheap and captures 80% of the perceptual character. My
recommendation: single-cell for v2.0; consider multi-cell for v2.1
if the single-cell version doesn't deliver.

### 3.4 Stereo decorrelation (TMT-style)

**Status quo:** Every stereo module processes L and R with identical
coefficients but separate state (per `feedback_stereo_biquad_state.md`).
This is correct for "transparent" but creates an unnaturally precise
center image — the meme's "digital" feel.

**Target:** Add **per-channel coefficient micro-detuning** to API5500,
Pultec, Transformer, and Sheen. Each channel's filter frequencies
deviate by ±0.3% from the nominal value, randomly seeded once per
plugin instance. The deviation is fixed for the instance lifetime
(re-seeded on `reset()`), so two instances on the same bus produce
slightly different colorations — like running two real channel strips.

**Modules affected:** API5500, Pultec, Transformer, Sheen.

**Trade:** None measurable. Add ~16 bytes per module for the seed and
twin coefficient sets. CPU identical.

**Open question:** Do we expose the deviation amount as a parameter
(`tmt_amount: 0..=1`) or hard-code it? My recommendation: hard-code at
the magic value (0.3% — Brainworx's published number for bx_console).
A knob for "how much randomness do you want" creates choice paralysis
without obvious benefit.

### 3.5 ButterComp2 attack/release refinements

**Status quo:** ButterComp2's bipolar interleaved algorithm is
excellent but its envelope follower is fixed-shape. Real bus
compressors have a slight program-dependent attack curve that softens
on transient-dense material.

**Target:** Add an **adaptive envelope shape** that softens attack
when the input crest factor exceeds a threshold (drum bus auto-
softening, vocal bus auto-tightening). Implementation: per-buffer
crest factor measurement, smoothed envelope time scaled accordingly.

**Modules affected:** ButterComp2.

**Open question:** Expose the adaptation amount as a parameter or
auto? My recommendation: auto with a single bypass toggle. Same
choice-paralysis argument.

### 3.6 Punch true-peak detection

**Status quo:** Punch detects peaks at the oversampled rate, which is
better than host rate but still not true-peak (which requires
intersample analysis at ~32× equivalent).

**Target:** Add **ITU-R BS.1770-4 true-peak detection** on Punch's
output. Display the inter-sample peak in the GUI; have the clipper
ceiling default to -1 dBTP instead of -0.3 dBFS to leave headroom for
DAC reconstruction artifacts.

**Modules affected:** Punch.

## 4. UI track (weeks 4-10)

### 4.1 Resizable window + HiDPI

**Status quo:** Window is fixed at 1300×860. No display-scale
awareness. On a 4K display the plugin looks tiny.

**Target:**
- Resizable from 1100×750 (minimum) to 2400×1400 (maximum), maintaining
  aspect ratio with letterboxing. Slot widths and font sizes scale
  proportionally.
- HiDPI awareness — query the host display scale and pre-scale Skia
  rendering accordingly.
- Persist user-chosen size across sessions via a non-automatable param
  (similar to existing zoom level).

**Open question:** Do we fully decouple aspect ratio (free resize) or
maintain it (anchored corners)? My recommendation: maintain. The
internal layout depends on consistent slot widths.

### 4.2 Preset system

**Status quo:** Zero presets ship. Users see the factory parameter
defaults and have to figure out everything from scratch.

**Target:**
- **Factory preset bank** — ~20 presets across genres and material
  types (rock drum bus, pop vocal bus, hip-hop master, jazz acoustic
  master, EDM master, podcast voice, classical orchestra, etc.)
- **Preset browser UI** — list view with categories, single-click
  load, current-preset name displayed in chassis header
- **User save/load** — persist to a `~/Documents/Bus Channel Strip/
  Presets/` directory; .bcs JSON files versioned with parameter ID
  schema for future migration
- **Preset-vs-current diff indicator** — small dot next to the preset
  name when current state diverges from loaded preset

**Open question:** Use NIH-Plug's built-in preset infrastructure, or
roll our own? Built-in works through DAW preset menus but doesn't give
us the browser UI. Recommendation: both — NIH-Plug's preset infra for
DAW integration, plus our own browser UI on top of the same JSON files.

**Lib.rs refactor opportunity:** Building this cleanly will force the
parameter definitions out of `lib.rs` into a `params/` module. That
unblocks faster iteration on everything else.

### 4.3 Per-module inline metering

**Status quo:** Only DynEQ has a real spectrum view (in its back
panel). Every other module is "blind" — you can't see what it's doing
without opening the back panel or trusting your ears.

**Target:**
- **Tiny inline spectrum strip** at the top of each EQ module (API5500,
  Pultec, Sheen back-view) showing the realized frequency response
  curve, updated reactively as sliders move. ~24px tall, full slot
  width.
- **GR meter** at the top of each compressor module (ButterComp2 + the
  per-band DynEQ already-existing meter promoted to consistent style).
- **Saturation meter** for Transformer / Punch / Sheen WARMTH showing
  current harmonic-content level.

**Trade:** Adds reactive computation per parameter change. Negligible.

### 4.4 Visual polish pass

**Status quo:** Functional, themed dark UI with brass plate accent.
v1.0 is consistent but utilitarian.

**Target:**
- **Typography rhythm** — consistent type scale (12 / 14 / 16 / 20 /
  24px), consistent letter-spacing per role
- **Spacing rhythm** — 4 / 8 / 12 / 16 / 24px grid; audit current
  spacing against it
- **Knob micro-interactions** — subtle spring-back animation on
  release, value tooltip that follows the cursor during drag
- **Slider easing** — value text fades in/out smoothly during interaction
- **Consistent iconography** — replace the current Unicode-glyph icons
  (✕, ↺, ◀, etc.) with a small custom SVG icon set rendered through
  Skia paths
- **Meter aesthetics** — gradient fills, smooth needle ballistics

**Open question:** Custom SVG icons require shipping path data in the
binary. Acceptable cost (~5 KB), but it's a maintenance commitment.
Recommendation: yes, do it. Unicode glyphs are part of "looks digital."

### 4.5 Theming

**Status quo:** Single dark theme. Brass accent on the brand plate;
otherwise slate / cyan / orange / brass accents per module.

**Target:**
- **Two ship-with themes**: "Studio" (current dark) and "Daylight"
  (bright neutral, intended for outdoor laptops and accessibility)
- **No user-customizable colors in v2.0** — that's a workflow feature,
  deferred. Two curated themes is enough.

**Open question:** Should the brass plate stay brass in Daylight mode
or convert to a different metal (brushed aluminum)? Recommendation:
stay brass — it's brand identity, not theme.

### 4.6 Tooltips / inline help

**Status quo:** Zero tooltips. Users have to read the README to know
what each control does.

**Target:** Hover any control for >800ms shows a tooltip with the
control name + a one-sentence description. Persisted tooltip text
lives in `src/tooltips.rs` so it's discoverable in source.

## 5. Foundation work (opportunistic, no dedicated track)

These land inside the DSP and UI tracks rather than getting their own
phase:

- **CI/CD fix** — when shipping signed installers (UI track integration
  phase), fix the Mac/Linux GitHub Actions builds that currently fail.
- **lib.rs refactor** — when building the preset system (UI track),
  extract `params/` module with one file per module's parameters.
- **Code signing + notarization** for macOS — required for "broader
  audience" distribution. Lands in week 11-12.
- **Installer packages** (Windows MSI, macOS pkg) — week 11-12.
- **Test coverage** — every new DSP module gets unit tests at the same
  density as Sheen (8 tests for 5 stages). New UI gets snapshot tests
  where vizia supports them.

## 6. Compatibility & migration

**Parameter IDs are stable.** No v1.0 param ID changes. New v2.0 params
get new IDs.

**Topology change for EQs is a breaking sound change.** Sessions saved
in v1.0 will load and play but sound subtly different (TPT EQ vs
biquad EQ at the same parameter values null within ~0.1 dB at
moderate settings, larger at extreme Q). Document this in the v2.0
release notes; same compatibility note we used for Sheen in v1.0.

**Hysteresis on Transformer is a bigger sound change.** Sessions
loaded in v2.0 with Transformer enabled will sound noticeably warmer.
Add a `transformer_hysteresis_bypass` BoolParam defaulting to `false`
(hysteresis ON), so users who want bit-identical playback can flip it
back.

## 7. Phasing & milestones

```
Week 1-2:  TPT filter prototype (one module: API5500), validation
Week 2-4:  4× oversampling rollout across nonlinear modules
Week 3-5:  Hysteresis model implementation + tuning
Week 4-6:  Stereo decorrelation; ButterComp2 adaptive envelope
Week 5-7:  DSP track integration testing in Reaper across material types
Week 4-5:  UI: lib.rs refactor + params/ module extraction
Week 5-7:  UI: preset system (data model + browser UI + 20 factory presets)
Week 6-8:  UI: resizable window + HiDPI
Week 7-9:  UI: per-module inline metering + visual polish pass
Week 8-9:  UI: theming (Studio + Daylight) + tooltips
Week 9-10: UI track integration testing; preset tuning pass
Week 10-11: CI fix; macOS signing + notarization; installers
Week 11-12: Final tuning, release notes, v2.0 tag, GitHub release
```

Buffer: ~1.5 weeks distributed across the schedule. If any track slips
hard, the cuts (in order) are: linear-phase Pultec mode → Daylight
theme → custom SVG icons → adaptive envelope on ButterComp2.

## 8. Success criteria

v2.0 is "done" when:

1. AB-comparing v1.0 and v2.0 on a full mix, the difference is
   immediately audible and consistently described as "more open" or
   "less digital" by at least 3 outside listeners (small-N qualitative
   sample, not a formal study).
2. Window resizes smoothly across at least 3 sizes without layout
   regressions.
3. The 20 factory presets cover the genres listed in §4.2 and load
   instantly without parameter glitches.
4. Signed installers work on a fresh Windows 11 + macOS 14 install
   without security warnings.
5. CPU on a single instance with full chain at 4× oversampling stays
   under 15% on a 2020-era M1 Mac mini at 48 kHz / 256-sample buffer.

## 9. Open questions for sign-off

These need a decision before any code lands:

1. **4× oversampling floor or "CPU saver" toggle?** (Recommendation: 4× floor.)
2. **TPT migration or fallback?** (Recommendation: hard migrate.)
3. **Preisach hysteresis depth — single-cell or multi-cell?** (Recommendation: single-cell for v2.0.)
4. **TMT detuning — exposed knob or hard-coded?** (Recommendation: hard-coded at 0.3%.)
5. **ButterComp2 adaptation — exposed knob or auto?** (Recommendation: auto with bypass.)
6. **Resize aspect ratio — locked or free?** (Recommendation: locked.)
7. **Linear-phase Pultec — ship in v2.0 or defer?** (Recommendation: ship if week 6 milestone hits on schedule, defer otherwise.)

Answer the seven and I'll write the per-theme implementation specs
(`docs/V2_TPT_FILTER_SPEC.md`, `docs/V2_HYSTERESIS_SPEC.md`,
`docs/V2_PRESET_SYSTEM_SPEC.md`, etc.) — one per theme — before any
code touches `src/`.
