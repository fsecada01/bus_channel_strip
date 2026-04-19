# Haas

<span class="module-haas">**Psychoacoustic stereo widener — M/S encoding + Haas effect comb filtering**</span>

---

## Overview

The **Haas module** widens the stereo image using two complementary techniques: M/S (mid/side) processing with independent gain control, and Haas effect comb filtering inspired by units like the WOW-Thing. It is named after Helmut Haas, whose 1951 doctoral research demonstrated that a short delay (1–40 ms) between the ears causes the brain to perceive a single fused sound, and that the direction and width of that image can be sculpted by controlling the delay and level relationship.

!!! note "Not Transaural XTC"
    The Haas module is a creative stereo widener, not a transaural crosstalk cancellation (XTC) processor. It works conventionally on stereo buses and is mono-compatible when using Side Comb mode.

**Signal flow:**

```
In L/R → M/S encode → Mid gain / Side gain
       → [Side Comb or Wide Comb delay]
       → M/S decode → Output trim → Dry/Wet blend → Out L/R
```

**Default position in chain:** Before Punch — so the clipper catches any widener-induced peaks before they hit the ceiling.

---

## Controls

| Control | Range | Description |
|---------|-------|-------------|
| **Bypass** | On/Off | Bypasses all Haas processing. |
| **Mid Gain** | −12 to +6 dB | Gain applied to the mid (M = L+R) channel after M/S encoding. Reduce to open the stereo field; boost to increase mono weight. |
| **Side Gain** | −6 to +6 dB | Gain applied to the side (S = L−R) channel. Boost to widen; cut to narrow. |
| **Comb Depth** | 0.0–1.0 | Depth of the Haas delay effect. `0` = no delay contribution. `1` = maximum comb coloration. |
| **Comb Time** | 1–20 ms (skewed) | Delay time for the comb effect. Shorter delays (1–5 ms) widen without obvious echo. Longer delays (10–20 ms) create more dramatic diffusion or pre-echo effects. Smooth automation via Hermite interpolation. |
| **Comb Mode** | Side Comb / Wide Comb | Selects the comb filter routing (see below). |
| **Mix** | 0.0–1.0 | Dry/wet blend for the entire Haas stage. `1.0` = fully processed. `0.0` = dry passthrough (without bypassing). Useful for parallel width blending. |

---

## Comb Modes

=== "Side Comb"
    The delay is applied only to the **side channel** (L−R component), then decoded back into L/R. This is the WOW-Thing style approach.

    **Character:** The width increases without affecting the mono (mid) image. Mono summing collapses the side channel, making this mode **mono-compatible** — the delay artifacts cancel when the mix is summed to mono.

    **Recommended for:** Master bus, broadcast deliverables, any situation where mono compatibility is a concern.

=== "Wide Comb"
    The delay is injected into **both L and R independently with opposite polarities** — a short delay is added to L and subtracted from R (or vice versa). This creates a diffuse, room-like stereo spread.

    **Character:** More dramatic widening than Side Comb, with a diffuse, enveloping quality. Less mono-compatible — the stereo image collapses differently when summed, though at short delay times (1–3 ms) the cancellation is minimal.

    **Recommended for:** Creative use on stems, ambience buses, or any context where a wider, more immersive image is wanted and mono compatibility is not a hard requirement.

---

## Techniques

### Subtle Bus Widening

| Control | Value |
|---------|-------|
| Comb Mode | `Side Comb` |
| Comb Time | `2–4 ms` |
| Comb Depth | `0.30–0.50` |
| Mid Gain | `0 dB` |
| Side Gain | `+2 dB` |
| Mix | `0.80` |

Gentle widening that remains mono-compatible. The short delay and moderate depth preserve transient integrity. The Mix at 0.80 keeps some dry signal to prevent phase smearing on hard solo.

### Ambient / Diffuse Width

| Control | Value |
|---------|-------|
| Comb Mode | `Wide Comb` |
| Comb Time | `8–14 ms` |
| Comb Depth | `0.60–0.80` |
| Side Gain | `+3 dB` |
| Mix | `1.0` |

Dramatic widening for cinematic or ambient buses. Check mono compatibility before delivery.

### Mono Compatibility Check

!!! tip "Verify Mono Before Export"
    After dialing in the Haas effect, sum your master to mono in your DAW and listen. If elements disappear or thin out noticeably:

    - Switch Comb Mode to `Side Comb`
    - Reduce Comb Time below 5 ms
    - Reduce Comb Depth
    - Lower Side Gain

    Side Comb at short delay times survives mono summing with minimal cancellation.

### Stereo Narrowing (Controlled Width)

| Control | Value |
|---------|-------|
| Comb Mode | `Side Comb` |
| Comb Depth | `0.0` |
| Mid Gain | `+3 dB` |
| Side Gain | `−3 dB` |
| Mix | `1.0` |

With Comb Depth at zero, the Haas module acts as a pure M/S width controller. Boosting mid and cutting side narrows the image — useful for stems that feel too wide on a dense master bus.

---

## Implementation Notes

- **Hermite interpolation** is used for the delay line read pointer, ensuring smooth, click-free automation of Comb Time even during rapid changes.
- **Output trim** is RMS-based and computed automatically to compensate for level changes introduced by the M/S gain adjustments and comb filtering. This prevents the Haas module from appearing louder or quieter than bypass when engaged.
- **Anti-denormal dither** is applied to the delay buffer to prevent CPU spikes from denormal floating-point values on long sustained tails.
- **Maximum delay size** (`MAX_DELAY_SAMPLES`) is allocated at initialization based on the maximum supported sample rate and the 20 ms upper limit of Comb Time. No heap allocation occurs during `process()`.
- The module is **lock-free and allocation-free** on the audio thread. All parameters use atomic reads; delay buffer is pre-allocated.
