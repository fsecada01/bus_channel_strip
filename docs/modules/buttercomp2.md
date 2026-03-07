# ButterComp2

<span class="module-comp">**Airwindows bipolar interleaved compression system**</span>

---

## Overview

Chris Johnson's ButterComp2 is described as *"the single richest, lushest 'glue' compressor."* Rather than modeling a traditional VCA or optical circuit, ButterComp2 implements **four independent compressors per channel in a bipolar, interleaved configuration**. This creates a complex, harmonically rich gain reduction character that feels more like analog hardware than conventional plugin compressors.

The result is compression that *breathes* with the music rather than squashing it — each of the four internal compressors responds to transients differently, and their interleaved operation produces a distinctive harmonic texture that engineers describe as "glue."

!!! note "Airwindows FFI"
    ButterComp2 is compiled from [Chris Johnson's C++ source](https://github.com/airwindows/airwindows) via Rust FFI. The audio processing is called once per buffer (O(1) FFI overhead) for real-time safety.

---

## Controls

| Control | Range | Description |
|---------|-------|-------------|
| **Compress** | 0.0 – 1.0 | Overall compression depth. Values above 0.6 become aggressive. Sweet spot: 0.3–0.55 for glue, 0.65–0.82 for NY-style parallel. |
| **Output** | 0.0 – 1.0 | Post-compression makeup gain. Start at 0.75–0.85 and trim to match bypass level. |
| **Dry/Wet** | 0.0 – 1.0 | Built-in parallel blend. 0 = fully dry, 1 = fully wet. This is the NY parallel blend control. |
| **Bypass** | On/Off | Bypasses all processing. |

---

## NY-Style Parallel Compression

The **New York compression** technique blends a heavily-compressed copy underneath the dry signal. The compressed signal brings up room ambience, subtle details, and low-level sustain without affecting transients (which come from the dry path).

=== "Subtle Glue (Mix Bus)"

    | Control | Value |
    |---------|-------|
    | Compress | `0.42` |
    | Output | `0.82` |
    | Dry/Wet | `0.55` |

    *Gentle cohesion that makes elements feel related without obvious compression artifacts.*

=== "NY Drums (Classic)"

    | Control | Value |
    |---------|-------|
    | Compress | `0.78` |
    | Output | `0.90` |
    | Dry/Wet | `0.33` |

    *Aggressive compression blended sparingly — brings up room, snare rattle, hi-hat bleed. The dry signal handles transients.*

=== "Heavy Glue (Electronic)"

    | Control | Value |
    |---------|-------|
    | Compress | `0.62` |
    | Output | `0.95` |
    | Dry/Wet | `0.78` |

    *All elements welded together. Suitable for electronic music where density > dynamics.*

!!! tip "The NY Technique"
    Start with Dry/Wet at **0.25** and slowly increase while listening to the bottom of your mix. Stop when you hear the sustain and room character "fill in" under the transients. Usually this is somewhere between 0.28 and 0.45 for drum buses.

---

## Tone Character

| Compress value | Character |
|----------------|-----------|
| 0.20 – 0.35 | Transparent, barely perceptible gain reduction, mainly adds harmonic glue |
| 0.35 – 0.55 | Musical glue, sustain enhancement, gel between elements |
| 0.55 – 0.70 | Assertive compression, audible gain reduction, dense character |
| 0.70 – 0.85 | Heavy compression — use lower Dry/Wet for NY parallel blend |
| 0.85 – 1.00 | Maximum compression — distortion character at high Dry/Wet values |
