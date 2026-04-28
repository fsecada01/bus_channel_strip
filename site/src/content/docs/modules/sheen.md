---
title: Sheen — Master-End Polish Coat
description: Hidden five-stage polish module pinned at the end of the chain. Default-on at research-grounded factory tuning. Click the brushed-brass brand plate to open the back view.
---

Sheen is the **eighth module** in the chassis but it's not a slot. It sits **pinned at the master end of the chain** (post-Punch, pre-master-gain) and is always on at factory defaults — so the plugin sounds finished out of the box. The only front-panel surface is the **brushed-brass "API Bus Channel Strip" brand plate** in the chassis header. Click it to flip the chassis to the Sheen back view and tune the polish.

## Why hidden?

Real consoles don't have an "amount of console-ness" knob. The polish coat is a *property of the chassis*, not a control surface. Most users never need to flip the panel — they just hear that the plugin sounds expensive. Advanced users discover the brass plate and gain the full five-slider control. Both audiences are served without compromise.

## Signal flow

```
[in] → BODY (low shelf) → PRESENCE (peak) → AIR (high shelf)
     → WARMTH (Sonnox Inflator polynomial @ 2× oversample)
     → WIDTH (M/S side-only HPF + shelf)
     → [out]
```

Stage order matters. EQ stages run first so their tonal contour is what the warmth shaper sees. Warmth runs before width so harmonic content goes through the M/S processing rather than being added on top of it (Clariphonic / Vitamin convention — adds depth instead of just spreading).

## The five stages

| Stage | Algorithm | Range | Factory default |
|---|---|---|---|
| **BODY** | Low shelf @ 100 Hz, Q=0.707 | -2.0 to +3.0 dB | **+1.0 dB** |
| **PRESENCE** | Peak EQ @ 3 kHz, Q=1.0 | -3.0 to +3.0 dB | **0.0 dB** (transparent) |
| **AIR** | High shelf @ 14 kHz, Q=0.5 | 0.0 to +4.0 dB | **+1.8 dB** |
| **WARMTH** | Sonnox Inflator polynomial @ Curve=0, 2× oversampled | 0–100% mix | **20%** |
| **WIDTH** | M/S side-only: HPF @ 150 Hz + shelf @ 500 Hz | 0–100% | **50%** (= +12.5% sides above 500 Hz) |

### BODY

A gentle low shelf at 100 Hz with conservative Q. At the +1.0 dB factory default it adds subtle solidity to bass and lower-midrange content without crossing into mud territory. Shared territory with Slate Thickness, Pultec 100 Hz, Vitamin LO band, Pensado factory low-shelf preset, and Wells ToneCentric low-mid weighting — all polish plugins converge here.

### PRESENCE

A peak EQ at 3 kHz with Q=1.0. **Defaults to transparent (0 dB)** — the slider is purely discoverable. Move it up for vocal forwardness and snare crack; move it down for the AR-1 / Pensado "smile" cut.

### AIR

A low-Q high shelf at 14 kHz. Low Q matters here — high-Q air shelves create a resonant peak just below the corner that sounds glassy. At the +1.8 dB factory default, this is the strongest cross-product agreement in the polish-plugin space (Maag AIR band, Pultec EQP-1A, Slate Shimmer, Vitamin HI, Ozone air band, Soothe2 "Fresh Air" all converge in the 12–15 kHz / +1.5–2 dB region). Note: the **console-bus heritage research** found no measured air bump in any classic master bus (SSL G, Neve 33609, API 2500, Studer, Trident are all flat to 20 kHz). The air shelf is therefore a *perceptual-compensation design choice*, not hardware emulation.

### WARMTH

The Sonnox Inflator transfer function at Curve=0, applied wet/dry at 20% factory mix:

```
f(x) = 1.5·x − 0.0625·x² − 0.375·x³ − 0.0625·x⁴
```

This is the public-domain reverse-engineered polynomial from RCJacH's open-source JSFX (which nulls Sonnox at every Curve setting). Curve=0 produces a balanced even+odd harmonic mix that resembles a soft tube saturator. Patent-free, mathematically nailed down, and the most-loved harmonic generator in the polish-plugin category.

The stage runs at **2× oversampling** (linear-interp upsample → polynomial → average + IIR downsample) to push the worst-case alias of the quartic term above 22 kHz at 44.1 kHz host rate.

At the 20% default mix on a typical -10 dBFS signal, WARMTH adds ~+0.83 dB perceived loudness with negligible measurable IMD. Push to 100% for ~+3.5 dB low-level gain plus pronounced soft-saturation character.

### WIDTH

A frequency-dependent M/S side-channel processor:

- **Below 150 Hz**: side gain forced to 0 (mono lows protect bass)
- **150–500 Hz**: side passes through neutrally
- **Above 500 Hz**: side gets +0…+1.94 dB shelf depending on slider position

Implementation: M/S encode → HPF the side at 150 Hz (kills mono-bass) → high-shelf the side at 500 Hz with gain proportional to slider → M/S decode. The mid channel passes through unchanged. Held intentionally subtle — width slamming sounds gimmicky.

## Factory defaults: where do they come from?

Every factory value traces back to a citation. v1.0 development included three parallel deep-research reports:

1. **Classic console-bus measurements** — SSL G-Series, Neve 33609, API 2500, Studer 169, Trident A-Range. Measured tonal signatures at unity (no compression engaged), harmonic profiles, and crosstalk characteristics.
2. **Polish-plugin teardowns** — Slate Revival, Kush AR-1, Sonnox Inflator, Maag EQ4, Soundtoys Decapitator Tone, Acustica Pensado, Waves Vitamin, Greg Wells ToneCentric, iZotope Ozone Vintage Tape, PSP Vintage Warmer 2, Brainworx bx_console N/E/G. Stage configurations, default knob positions, and consensus polish recipes.
3. **Tape and transformer harmonic profiles** — Studer A800 / Ampex ATR-102 / MCI JH series with 456 / GP9 tape; Jensen JT-11-DM, Lundahl LL1538/LL1517, Carnhill VTB1148, UTC A-20, Sowter 9120. Measured harmonic spectra at -18 / -12 / -6 dBFS reference levels.

The full synthesis lives in `docs/SHEEN_MODULE_SPEC.md` in the repository, with per-stage citations.

## UX

**Brass plate** — replaces the static "API Bus Channel Strip" label with a brushed-brass surface. Hover brightens it; click flips the chassis to the Sheen back view. While the back view is open, the plate stays glowing so you know how to flip back.

**Back view layout** — header with `← STRIP VIEW` button + `SHEEN` brass wordmark; master-bypass strip with `MASTER BYPASS` toggle and `↺ RESTORE FACTORY` button on the right; five vertical slider columns (BODY / PRESENCE / AIR / WARMTH / WIDTH) with per-stage bypass below each slider.

**Mutual exclusion** — the Sheen back view and the Dynamic EQ back view are mutually exclusive. Opening one auto-closes the other. `Esc` closes any open back view.

**RESTORE FACTORY** — re-writes every Sheen parameter to its factory default in one event-frame batch. Implemented as `RawParamEvent::SetParameterNormalized` writes so the host sees the change as automation, not as model-only state mutation.

## Auto-gain interaction

Sheen is **excluded from `global_auto_gain` compensation**. Auto-comp on a polish stage defeats its purpose — the +1 LU of perceived loudness from WARMTH and the RMS gain from BODY/AIR are *the point*, not artifacts to compensate. The chassis output gain stays under the master gain slider; users who want to trim Sheen's contribution can do so manually.

## Compatibility

Sessions saved before v1.0 load with Sheen ON at factory defaults — playback will subtly differ from pre-1.0 behavior. For bit-identical playback of a pre-1.0 session, flip the back-panel master Sheen bypass.

11 new automation params (5 stage values + 5 stage bypasses + 1 master Sheen bypass) — none reuse existing param IDs.

## Parameters

| ID | Type | Range | Default |
|---|---|---|---|
| `sheen_bypass` | Bool | true/false | **false** (Sheen ON) |
| `sheen_body_db` | Float | -2.0..=3.0 dB | +1.0 |
| `sheen_body_bypass` | Bool | true/false | false |
| `sheen_presence_db` | Float | -3.0..=3.0 dB | 0.0 |
| `sheen_presence_bypass` | Bool | true/false | false |
| `sheen_air_db` | Float | 0.0..=4.0 dB | +1.8 |
| `sheen_air_bypass` | Bool | true/false | false |
| `sheen_warmth` | Float | 0.0..=1.0 | 0.20 |
| `sheen_warmth_bypass` | Bool | true/false | false |
| `sheen_width` | Float | 0.0..=1.0 | 0.50 |
| `sheen_width_bypass` | Bool | true/false | false |

All FloatParams use `SmoothingStyle::Linear(5.0)` for click-free transitions. Cached in the audio thread once per buffer; filter coefficients regenerate only when the cached value actually changes.
