---
title: Pultec EQ
description: EQP-1A style passive tube equalizer. Authentic LCR resonance, simultaneous boost/cut, bandwidth control, tube saturation.
---

<span class="module-badge badge-pultec">EQ — Position 3</span>

The **Pultec EQP-1A** is one of the most revered equalizers in recording history. Its passive tube circuit enables a unique behavior: the low-frequency boost and cut controls can be applied *simultaneously*, producing a characteristic curve no single filter can reproduce. The original hardware's inductor network also creates a resonant peak at the shelf corner frequency — this module models that resonance authentically.

:::tip[Quick Start]
Classic Pultec trick: set LF Boost Freq to **60 Hz**, LF Boost to **8–12 dB**, LF Cut to **4–6 dB**, and LF Boost Bandwidth to **0.67**. This carves a tight, punchy low end with real sub presence and no mud.
:::

---

## Controls

### Low Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **LF Boost Freq** | 20–300 Hz (skewed) | Corner frequency for the LF boost shelf. 60 Hz is the classic kick/bass fundamental; 100–150 Hz for warmth on guitars or synth bass. |
| **LF Boost** | 0–18 dB | Low shelf boost amount. At the corner frequency, the LCR resonant peak adds an additional bump (≈45% of boost dB, Q=1.8), characteristic of the original hardware inductor. |
| **LF Boost Bandwidth** | 0.0–1.0 | Shelf width. **0** = narrow (Q=1.0, boost concentrated just below corner). **0.67** = musical default (Q=0.5, boost spreads through 2–3× the corner frequency). **1.0** = wide (Q=0.25, broad low-end lift). |
| **LF Cut Freq** | 20–400 Hz (skewed) | Corner frequency for the simultaneous LF cut. Typically set to the same frequency as the boost or slightly lower. |
| **LF Cut** | 0–18 dB | Low shelf attenuation amount. Cuts the low-mid bloom while the boost adds sub energy — the source of the classic Pultec shape. |
| **LF Cut Bandwidth** | 0.0–1.0 | Cut shelf width. Same Q mapping as LF Boost Bandwidth. |

### High Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **HF Boost Freq** | 3–20 kHz | High-frequency boost center. 8–12 kHz for presence and air; 3–5 kHz for upper-mid bite. |
| **HF Boost** | 0–18 dB | HF boost. The Pultec HF boost is famous for being non-fatiguing at high amounts due to the gentle passive shelf shape. |
| **HF Boost BW** | 0.0–1.0 | HF boost bandwidth. Low = wide, shelf-like. High = narrow, bell-shaped. |
| **HF Cut Freq** | 5–25 kHz | HF shelving cut center. |
| **HF Cut** | 0–18 dB | HF shelf cut amount. |

### Saturation

| Control | Range | Description |
|---------|-------|-------------|
| **Tube Drive** | 0.0–1.0 | Tube saturation via tanh soft clipping. Low values (0.1–0.25) add analog warmth without audible distortion. Higher values bring in harmonic density. |
| **Bypass** | On/Off | Bypasses all processing. |

---

## LCR Resonance

The original EQP-1A's inductor network creates a resonant peak at the boost shelf corner frequency. This is not a bug — it is the reason the Pultec adds perceived punch even at modest boost levels. The resonant peak is modeled as a PeakingEQ at the corner frequency with:

- **Amplitude**: 45% of the shelf boost dB
- **Q**: 1.8

At +12 dB LF Boost / 60 Hz, you get approximately +5.4 dB of resonant peak right at 60 Hz, sitting on top of the shelf boost. This combination gives kick drums and bass instruments a tight, focused fundamental.

The **LF Boost Bandwidth** control interacts with this: a wider shelf (higher BW value) spreads the boost through more of the musical range (100–300 Hz), making it useful for guitar buses and synths in addition to bass-heavy material.

---

## The Pultec Trick

The simultaneous boost/cut is the most celebrated Pultec technique:

:::note[Classic Low-End Trick]
| Control | Value |
|---------|-------|
| LF Boost Freq | `60 Hz` |
| LF Boost | `10–12 dB` |
| LF Boost Bandwidth | `0.67` |
| LF Cut Freq | `60 Hz` |
| LF Cut | `5–7 dB` |

**What happens:** The boost adds sub and low-end energy with an LCR resonant peak at 60 Hz. The cut attenuates low-mid bloom at a lower effective frequency (the cut shelf falls off more gradually at the same frequency). The result is a tight, punchy low end with real weight and no mud — a shape no single filter can produce.

Increase LF Cut for tighter, more controlled low end. Decrease it for a more open, roomy bottom.
:::

---

## Techniques

### Master Bus Weight and Air

| Control | Value |
|---------|-------|
| LF Boost Freq | `30 Hz` |
| LF Boost | `6 dB` |
| LF Boost Bandwidth | `0.67` |
| HF Boost Freq | `12 kHz` |
| HF Boost | `4 dB` |
| Tube Drive | `0.15` |

Sub weight below 30 Hz with the LCR resonance providing a gentle 30 Hz focus, plus air above 12 kHz. Classic mastering move.

### Guitar Bus Presence

| Control | Value |
|---------|-------|
| LF Boost Freq | `100 Hz` |
| LF Boost | `5 dB` |
| LF Boost Bandwidth | `0.8` |
| HF Boost Freq | `8 kHz` |
| HF Boost | `6 dB` |
| HF Boost BW | `0.60` |

Wide LF boost at 100 Hz adds body and weight to electric guitars. The resonant peak at 100 Hz gives that characteristic upper-bass warmth. HF presence boost at 8 kHz adds cut-through.

### Bass Bus Fundamentals

| Control | Value |
|---------|-------|
| LF Boost Freq | `60 Hz` |
| LF Boost | `10 dB` |
| LF Boost Bandwidth | `0.5` |
| LF Cut Freq | `60 Hz` |
| LF Cut | `6 dB` |

Tight LF bandwidth (Q≈0.7) concentrates the boost and resonance right at the bass fundamental. The simultaneous cut removes low-mid bloat.

:::tip[After Compression]
Place Pultec *after* ButterComp2 in the chain. Tonal shaping after compression means you're shaping the already-glued, balanced signal. The Pultec LF boost after compression sounds more controlled and focused than before.
:::

## See Also

- [Techniques & Presets](/bus_channel_strip/presets/techniques/) — Pultec trick and tonal shaping recipes
- [Genre Signal Chains](/bus_channel_strip/presets/genres/) — per-genre Pultec settings
- [Instrument Buses](/bus_channel_strip/presets/buses/) — per-bus Pultec settings
