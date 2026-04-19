# Pultec EQ

<span class="module-pultec">**EQP-1A style passive tube equalizer with authentic LCR resonance and simultaneous boost/cut**</span>

---

## Overview

The **Pultec EQP-1A** is one of the most revered equalizers in recording history. Its passive tube circuit creates a unique behavior: the low-frequency boost and cut controls can be applied *simultaneously*, producing a characteristic curve with a tight, controlled peak followed by an attenuated shelf — a shape no single filter can reproduce.

Original hardware units from the 1950s sell for tens of thousands of dollars. This module models the passive EQ behavior including:

- **LCR resonant bump** — At the shelf corner frequency, an inductor-capacitor-resistor resonance in the original hardware creates a peak on top of the shelf. This module replicates it with a PeakingEQ filter at the corner frequency set to approximately 45% of the shelf gain at Q=1.8.
- **Simultaneous boost and cut** — The classic Pultec trick: apply LF Boost and LF Cut together for a tight, controlled low end with real sub presence and no mud.
- **Bandwidth controls** — Independent control over how wide the boost and cut shelves spread, modeled after the inductor saturation behavior that varies with drive level.
- **Tube saturation** — tanh soft clipping for harmonic richness.

All filter math routes through `shaping::biquad_coeffs` to correct the biquad 0.5.0 frequency normalization bug (which places filters 4× below the requested corner without this fix).

---

## Controls

### Low Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **LF Boost Freq** | 20–300 Hz (skewed) | Corner frequency for the low shelf boost. The LCR resonant peak also sits at this frequency. |
| **LF Boost** | 0–18 dB | Low shelf boost amount. At the corner frequency, the LCR resonant peak adds an additional bump (≈45% of boost dB at Q=1.8), so a 10 dB shelf produces roughly +14.5 dB at the resonant peak. |
| **LF Boost Bandwidth** | 0.0–1.0 | Shelf width. `0` = narrow (Q=1.0; boost concentrated below corner). `1` = wide (Q=0.25; boost spreads through approximately 3× the corner frequency). Default **0.67** (Q≈0.5) is the musical sweet spot — spreads the boost through the guitar and synth range. |
| **LF Cut Freq** | 20–400 Hz (skewed) | Corner frequency for the low shelf cut. Can be set independently of boost frequency. |
| **LF Cut** | 0–18 dB | Low shelf attenuation amount. Simultaneous use with LF Boost is the hallmark Pultec technique. |
| **LF Cut Bandwidth** | 0.0–1.0 | Cut shelf width. Same Q mapping as LF Boost Bandwidth. Narrower cut = more focused attenuation at the corner. Wider cut = rolls off a broader low-mid region. |

### High Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **HF Boost Freq** | 3–20 kHz | High-frequency boost center. 10–12 kHz for presence/air; 3–8 kHz for upper-mid brightness. |
| **HF Boost** | 0–18 dB | HF peaking/shelf boost. The original Pultec HF boost is celebrated for being non-fatiguing even at high amounts. |
| **HF Boost BW** | 0.0–1.0 | HF boost bandwidth. Lower values = broader, more shelf-like. Higher values = narrower bell. |
| **HF Cut Freq** | 5–25 kHz | HF shelving cut center. |
| **HF Cut** | 0–18 dB | HF shelf cut amount. |

### Saturation

| Control | Range | Description |
|---------|-------|-------------|
| **Tube Drive** | 0.0–1.0 | Tube saturation character via tanh soft clipping. Low values (0.10–0.25) add analog warmth without audible distortion. Higher values add harmonic density and compression character. |
| **Bypass** | On/Off | Bypasses all processing. |

---

## The LCR Resonance

The original EQP-1A uses an inductor-capacitor-resistor (LCR) network to create its shelving curves. A property of this passive circuit is that the inductor resonates at its natural frequency — which coincides with the selected boost frequency. This resonance adds a peak on top of the shelf, giving the Pultec its characteristic "smile" shape where the corner frequency is boosted more than the shelf region below it.

This module replicates the resonance with a PeakingEQ filter at the corner frequency with gain equal to approximately 45% of the shelf boost amount and Q=1.8. The result: a 12 dB shelf boost at 60 Hz produces roughly +17.4 dB at 60 Hz itself before the shelf settles to +12 dB below.

!!! note "Resonance Scales with Boost Amount"
    The LCR peak gain is proportional to the shelf gain — both increase together as you raise LF Boost. At very low boost amounts (under 3 dB), the resonance is barely audible. At full boost (18 dB), the resonant peak is significant and characteristic of the hardware sound.

---

## The Pultec Trick

The most celebrated Pultec technique exploits the simultaneous boost/cut interaction:

!!! example "Classic Low-End Trick"
    | Control | Value |
    |---------|-------|
    | LF Boost Freq | `60 Hz` |
    | LF Boost | `9 dB` |
    | LF Boost Bandwidth | `0.67` |
    | LF Cut Freq | `60 Hz` |
    | LF Cut | `6 dB` |

    **What happens:** The boost adds sub energy at 60 Hz, including the LCR resonant peak. The cut simultaneously attenuates the lower shelf region, tightening the low-mid bloom without removing the sub punch. The result is a controlled low end with real sub presence and no mud — a curve no single filter can produce.

    *Adjust LF Cut up or down to taste: more cut = tighter, less cut = more open.*

---

## Techniques

### Sub Weight on Master Bus

| Control | Value |
|---------|-------|
| LF Boost Freq | `30 Hz` |
| LF Boost | `6 dB` |
| LF Boost Bandwidth | `0.67` |
| LF Cut Freq | `60 Hz` |
| LF Cut | `3 dB` |
| HF Boost Freq | `12 kHz` |
| HF Boost | `4 dB` |
| Tube Drive | `0.15` |

Classic mastering move: sub weight with an air boost. The 30 Hz boost adds foundation without muddiness. Tube Drive adds subtle harmonic glue at low values.

### Guitar Bus Presence

| Control | Value |
|---------|-------|
| HF Boost Freq | `8 kHz` |
| HF Boost | `7 dB` |
| HF Boost BW | `0.60` |
| HF Cut Freq | `20 kHz` |
| HF Cut | `3 dB` |

Presence boost with a gentle ultra-HF rolloff. Tames fizz while adding cut-through.

### Drum Bus Low-End Control

| Control | Value |
|---------|-------|
| LF Boost Freq | `60 Hz` |
| LF Boost | `12 dB` |
| LF Boost Bandwidth | `0.40` |
| LF Cut Freq | `100 Hz` |
| LF Cut | `8 dB` |

Narrow bandwidth concentrates the boost below 60 Hz for kick punch. The cut at 100 Hz removes the low-mid boominess that a wide boost would introduce.

!!! tip "After Compression"
    Place Pultec *after* ButterComp2 in the chain. Tonal shaping after compression means you're shaping the already-glued, balanced signal rather than pre-compression material. The Pultec LF boost after compression sounds more controlled and intentional than before.

!!! tip "Bandwidth for Genre"
    - **LF Boost Bandwidth 0.67 (default):** Musical sweet spot — spreads boost through 100–300 Hz, suits most mixes.
    - **LF Boost Bandwidth 0.20–0.40:** Narrow, sub-focused boost. Good for hip-hop and EDM where sub kick needs to punch without affecting upper bass.
    - **LF Boost Bandwidth 0.80–1.0:** Wide, euphonic shelf. Good for jazz, acoustic, or orchestral where you want a full, room-filling low end without a specific frequency peak.
