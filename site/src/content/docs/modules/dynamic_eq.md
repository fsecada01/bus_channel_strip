---
title: Dynamic EQ
description: 4-band frequency-dependent dynamics with Compress, Expand, and Gate modes. Real-time spectral analyzer.
---

<span class="module-badge badge-dyneq">Dynamics — Position 4</span>

Dynamic EQ combines parametric EQ precision with dynamics reactivity. Each band operates as an **envelope-follower-driven peaking filter** — it only applies equalization when the sidechain level crosses the threshold, proportional to the overage and ratio. The result is EQ that responds to the music rather than being static.

## Modes

| Mode | Behavior |
|------|----------|
| **Compress Down** | Reduces gain at the EQ frequency when signal exceeds threshold — multiband compression |
| **Expand Up** | Increases gain when signal exceeds threshold — dynamic presence enhancement |
| **Gate** | Attenuates when signal falls *below* threshold — eliminates mud between hits |

### Spectral Analyzer

The GUI includes a real-time spectral overlay showing:

- **Band tint fills** — color-coded frequency regions per band
- **Gain reduction meters** — per-band GR in dB at the top of the spectrum
- **Crossover lines** — visual markers at 500 Hz, 2 kHz, and 6 kHz

---

## Per-Band Controls (×4)

Each of the four bands has identical controls:

| Control | Range | Description |
|---------|-------|-------------|
| **Enable** | On/Off | Activates this band. Disabled bands pass signal unmodified. |
| **Mode** | Compress / Expand / Gate | Processing mode (see above). |
| **Detector Freq** | 20 Hz – 20 kHz | Sidechain filter center. The envelope follower measures level at this frequency — can be set independently of the EQ band's center. |
| **EQ Freq** | 20 Hz – 20 kHz | Center frequency of the dynamic peaking filter applied to the signal. |
| **Q** | 0.1 – 10.0 | Filter bandwidth. Low Q = wider; high Q = surgical. |
| **Threshold** | −60 – 0 dBFS | Level at which processing begins. Below threshold: no EQ. Above: gain change proportional to ratio. |
| **Ratio** | 1:1 – 20:1 | Compression or expansion ratio. 2:1 is gentle; 4:1 assertive; 10:1+ approaches limiting. |
| **Attack** | 0.1 – 200 ms | Envelope follower attack time. Fast (1–5 ms) catches transients; slow (20–100 ms) allows transients through. |
| **Release** | 10 – 2000 ms | Recovery time. Too fast = pumping; too slow = over-damping. 80–300 ms is typical. |
| **Makeup Gain** | −12 – +12 dB | Static gain after dynamic processing. |
| **Solo** | On/Off | Routes only this band through a bandpass filter for monitoring. |

---

## Techniques

### Mastering De-Essing

Surgical sibilance control without affecting cymbal and instrument brightness between events:

| Control | Band 3 or 4 |
|---------|-------------|
| Mode | `Compress Down` |
| Detector Freq | `7000 – 8000 Hz` |
| EQ Freq | `7000 Hz` |
| Q | `1.5 – 2.0` |
| Threshold | `−14 to −10 dBFS` |
| Ratio | `2:1 – 3:1` |
| Attack | `3 – 8 ms` |
| Release | `50 – 100 ms` |

:::tip[De-Essing vs. Static EQ]
A high-shelf cut at 7 kHz permanently reduces all HF content. Dynamic EQ only attenuates during actual sibilant events — preserving the natural HF energy of cymbals, acoustic guitars, and consonants between sibilant moments.
:::

### Sub Control

Tame low-end peaks without flattening sub energy:

| Control | Band 1 |
|---------|--------|
| Mode | `Compress Down` |
| Detector Freq | `60 – 80 Hz` |
| EQ Freq | `60 Hz` |
| Q | `1.0` |
| Threshold | `−18 dBFS` |
| Ratio | `3:1` |
| Attack | `5 ms` |
| Release | `120 ms` |

### Mud Gate

Eliminate low-mid buildup between drum hits or guitar chords:

| Control | Band 2 |
|---------|--------|
| Mode | `Gate` |
| Detector Freq | `200 – 250 Hz` |
| EQ Freq | `220 Hz` |
| Threshold | `−22 dBFS` |
| Ratio | `4:1` |

### Dynamic Presence Enhancement

Increase upper-mid presence on transient peaks — snare and vocals cut through on loud passages:

| Control | Band 3 |
|---------|--------|
| Mode | `Expand Up` |
| Detector Freq | `2500 – 3500 Hz` |
| EQ Freq | `3000 Hz` |
| Threshold | `−24 dBFS` |
| Ratio | `2:1` |

---

## Implementation Notes

- Sidechain detection uses a +6 dB peaking filter at the detector frequency
- 0.05 dB gain hysteresis prevents redundant coefficient recomputation during stable envelope periods
- Gain reduction displayed via lock-free `AtomicU32` per band, updated from audio thread with Relaxed ordering
