# Sheen Module Specification

Status: **Draft, awaiting sign-off** (2026-04-28). Once approved, implementation
proceeds in the order laid out at the bottom of this doc.

## 1. Purpose & product framing

Sheen is a hidden, always-on master-end "polish coat" — a curated multi-stage
DSP module that makes the chassis sound finished out of the box without
exposing any front-panel control. Discovery happens by clicking the existing
**API** brand mark (which becomes a brushed-brass plate); the back panel
reveals five sliders for users who want to tune the polish themselves.

The product story v1.0 ships on:

- **Front panel:** unchanged surface for casual users — same seven-module rack,
  same controls. The only visible difference is that the brand mark looks like
  a stamped brass plate, hinting at depth without demanding interaction.
- **Default sound:** "this plugin sounds like hardware out of the box" — a
  measured, research-grounded set of factory values that lifts every mix
  subtly without ever sounding obviously processed.
- **Back panel:** click the plate, and the chassis flips to a five-slider
  back view (TILT body / PRESENCE / AIR / WARMTH / WIDTH) plus per-stage
  bypasses and a RESTORE FACTORY button. Mutual exclusion with the existing
  DynEQ back view — only one back panel can be open at a time.

## 2. Signal-chain placement

Sheen sits **post-Punch, pre-master-gain**:

```
[7 reorderable slots] → [Punch (last slot)] → [Sheen] → [global gain] → out
```

Sheen is **not** a slot module:

- Not in `enum ModuleType`
- Not in `module_order_*` params
- Not reorderable
- Always present, always last (before master gain)

This pinning is intentional. The polish coat is a chassis property, not a user
choice; treating it as a slot would let the user accidentally place it before
the clipper, defeating the algorithm.

Sheen is gated by the existing `global_bypass` param. When the user bypasses
the plugin globally, the entire processing chain (including Sheen) is skipped
— a clean A/B reveals exactly what Sheen contributes.

## 3. Five-stage DSP pipeline

In processing order:

```
in → BODY (low shelf) → PRESENCE (peak) → AIR (high shelf) → WARMTH (waveshaper) → WIDTH (M/S) → out
```

**Stage order rationale:** EQ stages first so their tonal contour is what the
warmth shaper sees; warmth before width so harmonic content goes through the
M/S processing rather than being added on top of it (Clariphonic / Vitamin
convention — adds depth instead of just spreading).

### 3.1 BODY — low shelf

- **Filter:** second-order low shelf (RBJ cookbook, computed via
  `shaping::biquad_coeffs` to sidestep the `biquad` crate's `from_params` bug
  documented in `feedback_biquad_crate_bug.md`)
- **Frequency:** 100 Hz fixed
- **Range:** -2.0 to +3.0 dB
- **Factory default:** **+1.0 dB**
- **Citation:** Polish-plugin consensus — Slate Thickness, Pultec 100 Hz,
  Vitamin LO band, Pensado factory low-shelf preset, Wells ToneCentric
  low-mid weighting all converge here. (Polish-plugin teardown synthesis,
  task #26.)

### 3.2 PRESENCE — peak EQ

- **Filter:** second-order peaking (RBJ cookbook)
- **Frequency:** 3 kHz fixed
- **Q:** 1.0 fixed
- **Range:** -3.0 to +3.0 dB
- **Factory default:** **0.0 dB** (transparent at default)
- **Citation:** AR-1 smile curve cuts -0.5 dB here; Pensado factory presets
  cut -1 dB; Maag's design philosophy avoids 2-4 kHz boost. We default to
  flat (no opinionated cut) so the slider is purely discoverable, not a
  factory tonal signature. (Polish-plugin teardown synthesis, task #26.)

### 3.3 AIR — high shelf

- **Filter:** second-order high shelf (RBJ cookbook)
- **Frequency:** 14 kHz fixed
- **Q:** 0.5 fixed (low-Q for smooth, phase-friendly shape)
- **Range:** 0.0 to +4.0 dB
- **Factory default:** **+1.8 dB**
- **Citation:** Cross-product agreement is strongest here. Maag AIR band
  (10-16 kHz), Pultec EQP-1A (10/16 kHz), Slate Shimmer (8-15 kHz),
  Vitamin HI band, iZotope Ozone air band, Soothe2 "Fresh Air" presets all
  converge on a low-Q shelf in the 12-15 kHz region at +1.5 to +2 dB.
  (Polish-plugin teardown synthesis, task #26.)
- **Note:** The console-bus heritage research (task #25) found NO measured
  air bump in any of SSL G / Neve 33609 / API 2500 / Studer / Trident — these
  units are flat to 20 kHz at unity. The air shelf is therefore a
  *perceptual-compensation* design choice, not a hardware-emulation artifact.
  We include it because every successful polish plugin does, and because real
  modern monitors and ears benefit from it. The user can defeat it via the
  back-panel bypass.

### 3.4 WARMTH — Sonnox Inflator-style waveshaper

- **Algorithm:** Public-domain reverse-engineered Sonnox Inflator transfer
  function (RCJacH JSFX, nulls original at all Curve settings):

  ```
  f(x) = A·x + B·x² + C·x³ - D·(x² - 2x³ + x⁴)
  A = 1.5 + 0.01·curve
  B = -0.02·curve
  C = 0.01·curve - 0.5
  D = 0.0625 - 0.0025·curve + 0.000025·curve²
  ```

- **Curve:** **0** fixed (balanced even+odd, soft-tube character — the most
  loved Inflator setting per polish-plugin synthesis)
- **Effect (mix):** 0% to 100% wet
- **Factory default:** **20%**
- **Citation:** Inflator is the most studied, most-loved harmonic generator
  in mastering polish. Patent-free (Frindle's algorithm is trade-secret only;
  the polynomial is public via reverse engineering). At Curve=0 / Effect=20%
  it adds ~+1 LU perceived loudness with negligible measurable IMD.
  (Polish-plugin teardown synthesis, task #26.)
- **Cross-check vs hardware research:** Tape/transformer research (task #27)
  arrived at a different polynomial (`y = x + a2·x² + a3·x³ + a5·x⁵` with
  `a2 ≈ 0.0014, a3 ≈ 0.0010, a5 ≈ 0.00012`) targeting 0.1% THD at -12 dBFS.
  Console research (task #25) recommends `a2 ≈ 0.0032, a3 ≈ 0.0008` for
  2nd-dominant -70 dBc. Both are valid and could be exposed as alternative
  warmth modes in v1.1. **For v1.0 we ship Inflator as the single algorithm**
  — it has the broadest user familiarity, is mathematically nailed down,
  and the 20% default sits comfortably within the harmonic ranges measured
  for tape and transformers at low drive.
- **Aliasing:** 2× oversampling for the polynomial. Without it, the cubic
  term aliases into the audible band on hot input. Reuse the linear-interp
  upsampler / IIR-smoothed downsampler pattern from `src/punch.rs`
  (`MAX_OS_FACTOR=2` here, no need for Punch's 8×).

### 3.5 WIDTH — frequency-dependent M/S sides scaler

- **Algorithm:** M/S encode (mid = (L+R)/2, side = (L-R)/2) → frequency-
  selective side gain → M/S decode. Side gain is a function of frequency:
  - Below 150 Hz: side gain forced to 0 (mono lows — protects bass)
  - 150-500 Hz: linear ramp from 0 to 1× side gain (smooth crossover)
  - Above 500 Hz: 1× × `(1 + 0.25·width_param)` — i.e. width=0 leaves
    sides untouched, width=1 boosts sides by +25% (~+1.9 dB on side
    energy, perceptually noticeable but never gimmicky)
- **Range:** 0.0 to 1.0
- **Factory default:** **0.5** (= +12.5% side gain above 500 Hz)
- **Citation:** Vitamin per-band width with LO locked mono is the
  gold-standard pattern. +10-15% sides above 500 Hz is the consensus
  "polish width" amount. (Polish-plugin teardown synthesis, task #26.)
- **Implementation note:** The 150/500 Hz crossover is implemented with two
  Linkwitz-Riley 4th-order LP/HP pairs operating only on the side channel
  (the mid channel passes through unchanged). This adds ~10 LOC vs the
  simpler "single shelf on side" approach but preserves phase coherence
  for solo/mute checks. Reuse the M/S encode/decode pattern from
  `src/haas.rs`.

## 4. Parameter list

11 new automation parameters. None reuse existing IDs.

| ID                         | Type      | Range          | Default | Notes                              |
|----------------------------|-----------|----------------|---------|------------------------------------|
| `sheen_bypass`             | BoolParam | true/false     | false   | Master Sheen on/off (default ON)   |
| `sheen_body_db`            | FloatParam| -2.0..=3.0 dB  | +1.0    | Body low-shelf gain                |
| `sheen_body_bypass`        | BoolParam | true/false     | false   | Per-stage bypass                   |
| `sheen_presence_db`        | FloatParam| -3.0..=3.0 dB  | 0.0     | Presence peak gain                 |
| `sheen_presence_bypass`    | BoolParam | true/false     | false   | Per-stage bypass                   |
| `sheen_air_db`             | FloatParam| 0.0..=4.0 dB   | +1.8    | Air high-shelf gain                |
| `sheen_air_bypass`         | BoolParam | true/false     | false   | Per-stage bypass                   |
| `sheen_warmth`             | FloatParam| 0.0..=1.0      | 0.20    | Inflator Effect (mix amount)       |
| `sheen_warmth_bypass`      | BoolParam | true/false     | false   | Per-stage bypass                   |
| `sheen_width`              | FloatParam| 0.0..=1.0      | 0.50    | Side-band scaler above 500 Hz      |
| `sheen_width_bypass`       | BoolParam | true/false     | false   | Per-stage bypass                   |

All FloatParams use `SmoothingStyle::Linear(5.0)` for click-free transitions
(matches existing convention).

The "RESTORE FACTORY" back-panel button is **not** a param — it's a UI-only
event that emits `RawParamEvent::SetParameterNormalized` for each Sheen
param to restore the defaults above. Treating it as automation would create
ambiguous state in DAW sessions.

## 5. UI specification

### 5.1 Brass plate (front panel)

The existing chassis-header brand block (`Label::new(cx, "API").class("chassis-brand")`
plus the title) becomes a single clickable brushed-brass plate.

**Visual:**
- Background: brass gradient
  `linear-gradient(135deg, #c8a04a 0%, #e8c878 50%, #a0823a 100%)`
- Inset shadow: `inset 0 1px 2px rgba(0,0,0,0.4)`
- Outer highlight: 1px top edge `rgba(255,255,255,0.15)`
- Text: dark engraved style — `color: #2a1f0a; text-shadow: 0 1px 0 rgba(255,255,255,0.18)`
- Font: same chassis font, 700 weight, +1px letter-spacing
- Padding: 6px 14px, border-radius 3px
- Hover: brightens 5-10%, cursor `Hand`
- Active (Sheen back view open): persistent inner glow + slight darken so the
  user knows how to flip back

**Interaction:**
- `on_press` emits `AppEvent::OpenSheen`
- Single-click only (no drag, no double-click semantics)

**Layout:** plate is the same width/height the existing brand block occupies
— no chassis layout reflow.

### 5.2 Sheen back view

Pattern-match on the existing DynEQ back view (`build_dyneq_back_view`,
`Data::dyneq_open`, `AppEvent::OpenDynEq`/`CloseDynEq`).

**Layout (top to bottom):**

1. **Header row:** large "SHEEN" wordmark in brass typography (matches the
   plate) + "← BACK" pill button on the right (`AppEvent::CloseSheen`)
2. **Master bypass strip:** single large toggle "SHEEN BYPASS" + small
   "RESTORE FACTORY" button on the right
3. **Five vertical slider columns:** equal-width, gap 12px
   - **BODY** | **PRESENCE** | **AIR** | **WARMTH** | **WIDTH**
   - Each column: stage label (top) → vertical slider → value readout → per-
     stage bypass toggle (bottom)
   - Slider styling: brass-themed (warm gold track, brushed-metal thumb)
   - Use `ParamSlider` for value/bypass; build_*_slider helpers in
     `components.rs` may need a brass-themed variant or the existing one
     can be reused with a `.class("brass-control")` modifier
4. **Footer:** omitted in v1.0. The signal-flow diagram lands in v1.1 if user
   feedback shows people wanting it. Skipping it keeps the back-view layout
   clean and the surface area to test smaller.

### 5.3 Mutual exclusion with DynEQ back view

`Data::dyneq_open` and `Data::sheen_open` are mutually exclusive: opening one
closes the other. Implementation in the model:

```rust
AppEvent::OpenDynEq => { self.dyneq_open = true; self.sheen_open = false; }
AppEvent::OpenSheen => { self.sheen_open = true; self.dyneq_open = false; }
AppEvent::CloseDynEq => { self.dyneq_open = false; }
AppEvent::CloseSheen => { self.sheen_open = false; }
```

The strip view's `display(Data::dyneq_open.map(...))` becomes
`display(Data::any_back_view_open.map(...))` — a derived lens that returns
`Display::None` when either back view is open.

### 5.4 Esc key

Esc currently exits focus mode and cancels in-flight drags. Extend it to
also close any open back view:

```rust
Code::Escape => {
    self.focused_slot = None;
    self.drag_source = None;
    self.drop_target = None;
    self.dyneq_open = false;
    self.sheen_open = false;
}
```

## 6. Default-on behavior

Sheen ships with `sheen_bypass = false` (i.e., on) and the factory values
above. This is the intentional product framing: the plugin sounds finished
out of the box. The brass plate is a discovery layer for users who want to
see *why* it sounds that way.

Two consequences worth flagging:

- **DAW-session loading:** existing sessions saved before Sheen existed will
  load with Sheen ON at factory defaults (since `sheen_bypass` defaults to
  false). This will subtly change the playback of every existing session.
  Users who want bit-identical playback of pre-1.0 sessions can flip the
  back-panel master bypass.
- **A/B comparison via global bypass:** the global `global_bypass` param
  bypasses the entire chain including Sheen, so an honest dry/wet A/B
  reveals exactly what the chassis (modules + Sheen together) contributes.

## 7. Auto-gain interaction (decided)

Sheen is **excluded from `global_auto_gain` compensation**. Auto-gain on a
polish stage defeats the polish — the +1 LU of perceived loudness from the
WARMTH stage and the RMS gain from BODY/AIR are *the point*, not artifacts
to compensate. The chassis output gain remains controlled by the master gain
slider; users who want to trim Sheen's contribution can do so manually.

## 8. Implementation order

Once this spec is signed off:

1. **`src/sheen.rs`** — DSP module (~300-400 LOC). Five stages, computed
   coefficients in `initialize()`, lock-free `process()`. Reuse:
   - `shaping::biquad_coeffs` for filter coefficients (avoid biquad-crate bug)
   - Per-channel biquad state arrays (per `feedback_stereo_biquod_state.md`)
   - 2× linear-interp oversampler / IIR-smoothed downsampler from `punch.rs`
   - M/S encode/decode pattern from `haas.rs`
2. **`src/lib.rs`** — Add 11 params via `#[derive(Params)]`. Insert
   `Sheen::process()` call after Punch processing and before master gain.
   Update `BusChannelStripParams::default()` with factory values above.
3. **`src/editor.rs`** — Add `Data::sheen_open: bool` field, `AppEvent::
   OpenSheen` / `CloseSheen` variants, mutual-exclusion logic. Replace the
   existing brand block with the brass plate. Add `build_sheen_back_view`
   following the DynEQ pattern. Extend Esc key handler.
4. **`src/styles.rs`** — Add `.brand-plate-brass`, `.sheen-back-view`,
   `.sheen-slider-column`, `.brass-control` classes.
5. **Tuning pass in Reaper** — load on master bus, AB factory defaults vs
   bypass on drum/bass/vocal/full-mix material across genres. Adjust until
   default-ON reliably sounds better than bypass without ever sounding
   processed. **The most important step.** Document final values back into
   this spec.
6. **Bump version → 1.0.0**, write release notes, tag, ship.

## 9. Citation index

- **Console-bus heritage** (task #25): `claudedocs/research_console_bus_heritage_*.md`
  (synthesised inline above; full transcript in agent output)
- **Polish-plugin teardowns** (task #26): `claudedocs/research_polish_plugins_*.md`
  (synthesised inline above; full transcript in agent output)
- **Tape/transformer harmonics** (task #27): `claudedocs/research_tape_transformer_*.md`
  (synthesised inline above; full transcript in agent output)
