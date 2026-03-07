# Punch

<span class="module-punch">**Transparent clipper + transient shaper with up to 8× oversampling**</span>

---

## Overview

Punch is a two-stage loudness processor:

1. **Clipper** — eliminates intersample peaks and creates headroom for loudness maximization
2. **Transient Shaper** — restores or enhances attack characteristics that preceding compression may have dulled

The key design decision: **transient shaping is applied pre-clip**. This means the shaper operates on the natural signal before the clipper sets the ceiling. If shaping were applied post-clip, the time-varying gain would create pumping artifacts on every note attack. Pre-clip shaping lets the clipper naturally limit the enhanced peaks.

!!! warning "Default Oversampling"
    The default is **4× oversampling**. This balances CPU usage with aliasing rejection for most use cases. Use 8× for critical mastering work where maximum transparency is required.

---

## Controls

| Control | Range | Description |
|---------|-------|-------------|
| **Clip Mode** | Hard / Soft / Cubic | Waveshaper algorithm (see below). |
| **Ceiling** | −12 – 0 dBFS | Peak ceiling. −0.3 to −0.1 dBTP typical for streaming. −1.0 dB for mp3 intersample headroom. |
| **Oversampling** | 1×, 2×, 4×, 8× | Upsample before clipping, downsample after. Higher = fewer aliasing artifacts, more CPU. |
| **Transient Attack** | 0.0 – 1.0 | Transient detection and upward expansion amount on attacks. Restores punch after compression. |
| **Transient Sustain** | 0.0 – 1.0 | Sustain portion enhancement. Lengthens decay of transient events. |
| **Transient Release** | 0.0 – 1.0 | Speed at which transient detection decays between events. |
| **Mix** | 0.0 – 1.0 | Dry/wet blend for the entire Punch stage. Allows parallel clipping (more transparent). |
| **Bypass** | On/Off | Bypasses all processing. |

---

## Clip Modes

=== "Soft (tanh)"
    ```
    y = tanh(x / ceiling) × ceiling
    ```
    Smoothest onset, most transparent. Begins softly limiting before the ceiling,
    adding gentle saturation as levels approach it. Recommended for vocal and acoustic buses,
    master bus processing, and anywhere transparency is paramount.

=== "Cubic"
    Polynomial waveshaper. Falls between Hard and Soft:

    - More headroom before onset than Hard
    - Harder onset than Soft once ceiling is reached
    - Adds odd-order harmonics (3rd, 5th) characteristic of solid-state saturation

    **Recommended default for most buses.**

=== "Hard"
    ```
    y = clip(x, -ceiling, ceiling)
    ```
    Abrupt ceiling — maximum loudness, most aggressive character.
    Adds strong odd harmonics. Use for rock/metal buses where harmonic grit is appropriate,
    or when a few dB of additional headroom vs. Soft mode is needed.

---

## Oversampling

| Setting | Aliasing | CPU | Use Case |
|---------|----------|-----|----------|
| 1× | High | Minimal | Fast iteration, monitoring only |
| 2× | Moderate | Low | Rough mixes, stem passes |
| 4× | Low | Moderate | **Default — recommended for most work** |
| 8× | Very low | Higher | Critical mastering, maximum transparency |

The oversampler uses linear interpolation upsampling and a very light IIR filter (pole = 0.05) for downsampling. The low pole value was chosen specifically to avoid the downsampler contributing to pumping artifacts.

---

## Techniques

### Transparent Loudness Maximization

| Control | Value |
|---------|-------|
| Clip Mode | `Soft` or `Cubic` |
| Ceiling | `−0.3 dBFS` |
| Oversampling | `4×` |
| Transient Attack | `0.30 – 0.50` |
| Mix | `1.0` |

Set the ceiling first so the meter just catches the loudest peaks. Then dial in Transient Attack to recover kick and snare punch lost to preceding compression stages. The pre-clip shaping ensures attack restoration happens before limiting, not after.

### Parallel Clipping

Mix at `0.70 – 0.85` to blend the clipped signal under the dry signal:

- Dry path preserves original transients
- Clipped path adds density, compression, and loudness
- More transparent than full wet at equivalent loudness levels

### Streaming Targets

| Platform | Target LUFS | Ceiling |
|----------|-------------|---------|
| Spotify | −14 LUFS integrated | −1.0 dBTP |
| Apple Music | −16 LUFS integrated | −1.0 dBTP |
| YouTube | −14 LUFS integrated | −1.0 dBTP |
| Tidal | −14 LUFS integrated | −1.0 dBTP |
| CD / Physical | Max loudness | −0.3 dBTP |

!!! note "Intersample Peaks"
    Set ceiling to **−1.0 dBFS** if the final delivery format is mp3 or AAC. Lossy encoding can create intersample peaks above the true peak of the source, and −1.0 dBFS provides sufficient headroom to avoid clipping in the encoder.

---

## Implementation Notes

- Upsampling: linear interpolation with `filter_state[1]` tracking previous input across buffer boundaries
- Downsampling: simple average + very light IIR (pole = 0.05) — the low pole value prevents the downsampler from contributing to pumping
- Transient detector uses the native sample rate (not the oversampled rate) to avoid detection artifacts at oversampled rates
- Maximum oversampling factor: 16× | Maximum block size: 8192 samples
