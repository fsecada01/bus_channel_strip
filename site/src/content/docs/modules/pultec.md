---
title: Pultec EQ
description: EQP-1A style passive tube equalizer. Simultaneous boost/cut low-end shaping, HF presence, tube saturation.
---

<span class="module-badge badge-pultec">EQ — Position 3</span>

The **Pultec EQP-1A** is one of the most revered equalizers in recording history. Its passive tube circuit enables a unique behavior: the low-frequency boost and cut controls can be applied *simultaneously*, producing a characteristic curve with a tight, controlled peak followed by an attenuated shelf — a shape no single filter can reproduce.

Original hardware units from the 1950s sell for tens of thousands of dollars. This module models the passive EQ behavior including boost/cut interaction, stepped frequency selections, and tube saturation coloration.

---

## Controls

### Low Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **LF Boost Freq** | 20, 30, 60, 100 Hz | Center frequency for the LF boost shelf. 60 Hz is the classic kick/bass fundamental. |
| **LF Boost** | 0.0 – 1.0 (0–8 dB) | Low shelf boost amount. Quadratic mapping for fine control at low values. |
| **LF Cut** | 0.0 – 1.0 (0 to −6 dB) | Simultaneous LF cut, applied at a lower frequency than the boost. This is the source of the classic Pultec low-end shape. |

### High Frequency Section

| Control | Range | Description |
|---------|-------|-------------|
| **HF Boost Freq** | 5, 8, 10, 12, 15, 20 kHz | HF boost center. 10–12 kHz for presence/air; 5–8 kHz for brightness. |
| **HF Boost** | 0.0 – 1.0 (0–10 dB) | High peaking boost. The Pultec HF boost is famous for being non-fatiguing at high amounts. |
| **HF Boost BW** | 0.0 – 1.0 (Q 0.6–2.0) | Bandwidth. Low = wide shelf-like; high = narrow bell. |
| **HF Cut Freq** | 5, 10, 20 kHz | HF shelving cut center. |
| **HF Cut** | 0.0 – 1.0 (0 to −8 dB) | HF shelf cut amount. |

### Saturation

| Control | Range | Description |
|---------|-------|-------------|
| **Tube Drive** | 0.0 – 1.0 | Tube saturation via tanh soft clipping. Low values (0.10–0.25) add analog warmth without audible distortion. |
| **Bypass** | On/Off | Bypasses all processing. |

---

## The Pultec Trick

The most celebrated Pultec technique exploits the simultaneous boost/cut interaction:

:::note[Classic Low-End Trick]
| Control | Value |
|---------|-------|
| LF Boost Freq | `60 Hz` |
| LF Boost | `0.55` (≈ 4.4 dB) |
| LF Cut | `0.42` (≈ −2.5 dB) |

**What happens:** The boost adds sub energy at 60 Hz. The cut attenuates low-mid bloom at a lower frequency (~36 Hz). The result is a tight, controlled low end with real sub presence and no mud — a curve no single filter can produce.

Adjust LF Cut up/down to taste: more cut = tighter, less cut = more open bottom.
:::

---

## Techniques

### Air and Warmth (Master Bus)

| Control | Value |
|---------|-------|
| LF Boost Freq | `30 Hz` |
| LF Boost | `0.25` |
| HF Boost Freq | `12 kHz` |
| HF Boost | `0.38` |
| Tube Drive | `0.15` |

Classic mastering move: sub weight + air boost. Tube Drive adds subtle harmonic glue.

### Guitar Bus Presence

| Control | Value |
|---------|-------|
| HF Boost Freq | `8 kHz` |
| HF Boost | `0.45` |
| HF Boost BW | `0.60` |
| HF Cut Freq | `20 kHz` |
| HF Cut | `0.20` |

Presence boost with a gentle ultra-HF rolloff. Tames fizz while adding cut-through.

:::tip[After Compression]
Place Pultec *after* ButterComp2. Tonal shaping after compression means you're shaping the already-glued signal rather than pre-compression material. The Pultec LF boost after compression sounds more controlled than before.
:::
