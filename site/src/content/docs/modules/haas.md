---
title: Haas
description: Psychoacoustic stereo widener using M/S encoding and Haas effect comb filtering. Side Comb and Wide Comb modes with Hermite-interpolated delay.
---

<span class="module-badge badge-haas">Widener — Position 6</span>

The **Haas module** widens the stereo image using two complementary techniques: M/S (mid/side) processing with independent gain control, and Haas effect comb filtering. It is named after Helmut Haas, whose 1951 doctoral research demonstrated that a short delay (1–40 ms) between the ears causes the brain to perceive a single fused sound, and that the direction and width of that image can be sculpted by controlling the delay and level relationship.

:::note[Not Transaural XTC]
The Haas module is a creative stereo widener, not a transaural crosstalk cancellation (XTC) processor. It works conventionally on stereo buses and is mono-compatible when using Side Comb mode.
:::

**Signal flow:**

```
In L/R → M/S encode → Mid gain / Side gain
       → [Side Comb or Wide Comb delay]
       → M/S decode → Output trim (RMS-safe) → Dry/Wet blend → Out L/R
```

**Default position in chain:** Before Punch — so the clipper catches any widener-induced peaks before they exceed the ceiling.

---

## Controls

| Control | Range | Description |
|---------|-------|-------------|
| **Bypass** | On/Off | Bypasses all Haas processing. |
| **Mid Gain** | −12 to +6 dB | Gain applied to the M/S mid (sum) channel before the comb filter. Reduce to push the mix wider; boost to increase mono focus. |
| **Side Gain** | ±6 dB | Gain applied to the M/S side (difference) channel. Increasing side gain makes the stereo image wider; reducing narrows it. |
| **Comb Depth** | 0.0–1.0 | Amount of comb filter contribution. 0 = pure M/S width control. 1 = maximum Haas comb effect. |
| **Comb Time** | 1–20 ms (skewed) | Delay time for the comb filter. Shorter (1–5 ms) = tight, subtle widening. Longer (10–20 ms) = pronounced spatial depth. |
| **Mode** | Side Comb / Wide Comb | Comb filter routing (see below). |
| **Mix** | 0.0–1.0 | Dry/wet blend. 1.0 = fully processed; 0.5 = equal blend for parallel widening. |

---

## Comb Modes

=== "Side Comb"
    The comb filter is applied only to the **side channel** (L−R). A polarity-flip comb delay is injected into the side signal — because side energy sums to zero on mono collapse, the comb contribution is fully mono-compatible.

    **Characteristic:** WOW-Thing–style widening. Tight, coherent center with enhanced stereo width in the diffuse field. The mono image is not affected.

    **Recommended for:** Master bus, vocal bus, any context where mono compatibility is critical.

=== "Wide Comb"
    A delayed (L−R) signal is injected with opposing signs into L and R. This creates a more diffuse, spacious effect than Side Comb.

    **Characteristic:** Broader, more ambient widening. Comb depth is internally clamped to 0.5 to cap the L-channel peak at ≈+3.5 dB worst case and stay within the RMS budget.

    **Recommended for:** Synth buses, ambient tracks, reverb returns. Check mono compatibility before committing on a master bus.

---

## Techniques

### Subtle Bus Widening

| Control | Value |
|---------|-------|
| Mid Gain | `0 dB` |
| Side Gain | `+2 dB` |
| Comb Depth | `0.3` |
| Comb Time | `7 ms` |
| Mode | `Side Comb` |
| Mix | `1.0` |

A small side boost + light comb gives noticeable width without disturbing the center image. Works well on guitar and synth buses.

### Mono Focus (Narrowing)

| Control | Value |
|---------|-------|
| Mid Gain | `+2 dB` |
| Side Gain | `−4 dB` |
| Comb Depth | `0.0` |
| Mix | `1.0` |

Reduce side gain to pull an overly wide mix toward center. Useful for mono-checking or on bass buses where too much side energy causes low-end smear.

### Ambient Depth (Wide Comb)

| Control | Value |
|---------|-------|
| Side Gain | `+3 dB` |
| Comb Depth | `0.5` |
| Comb Time | `14 ms` |
| Mode | `Wide Comb` |
| Mix | `0.7` |

The wider comb time at 0.7 mix creates a natural room-like widening without the pitched quality of a short comb. Effective on synth pads, strings, and ambient buses.

:::tip[Mono Compatibility Check]
After applying any widening, briefly set **Mix to 0.0** (full dry) and compare the stereo image. Then listen to your DAW output in mono. Side Comb mode is safe for mono — Wide Comb mode may introduce comb coloration at the mono sum, particularly noticeable with high Comb Depth values.
:::

---

## Implementation Notes

- **Delay interpolation**: Hermite 4-point cubic interpolation for smooth, artifact-free delay time automation. Delay time is additionally smoothed by a one-pole LPF (τ = 20 ms) to eliminate zipper noise during sweeps.
- **Ring buffer**: Power-of-two length (4096 samples), wrap via bitwise AND mask — avoids modulo on the audio thread.
- **Anti-denormal**: 1e-20 alternating dither written into the delay line every sample (Airwindows convention).
- **Output trim**: RMS-safe automatic compensation based on mid+side peak budget: `1 / sqrt(1 + |side_gain| * comb_depth)`. Prevents the processed signal from exceeding the input RMS.
- **MAX_DELAY_SAMPLES**: Hard limit at `DELAY_BUF_LEN − 4` (4 samples reserved for Hermite headroom). At 192 kHz, this covers ~20 ms.
