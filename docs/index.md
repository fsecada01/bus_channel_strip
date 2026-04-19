# Bus Channel Strip

**Professional 6-module bus processor — VST3 & CLAP**

> Built with [NIH-Plug](https://github.com/robbert-vdh/nih-plug), [Airwindows DSP](https://github.com/airwindows/airwindows), and [vizia](https://vizia.dev/) · GPL-3.0-or-later · v0.5.0

---

## Signal Chain

<div class="signal-chain">
  <span class="node node-eq">API5500 EQ</span>
  <span class="arrow">→</span>
  <span class="node node-comp">ButterComp2</span>
  <span class="arrow">→</span>
  <span class="node node-pultec">Pultec EQ</span>
  <span class="arrow">→</span>
  <span class="node node-dyneq">Dynamic EQ</span>
  <span class="arrow">→</span>
  <span class="node node-xfm">Transformer</span>
  <span class="arrow">→</span>
  <span class="node node-haas">Haas</span>
  <span class="arrow">→</span>
  <span class="node node-punch">Punch</span>
</div>

All seven modules are **reorderable** via drag-to-swap handles in the GUI. Each module has an individual bypass switch and is fully automatable.

---

## Modules at a Glance

| Module | Type | Character |
|--------|------|-----------|
| [**API5500 EQ**](modules/api5500.md) | 5-band semi-parametric EQ | Fast, punchy, API console sound |
| [**ButterComp2**](modules/buttercomp2.md) | Airwindows bipolar interleaved compressor | Lush, musical glue compression |
| [**Pultec EQ**](modules/pultec.md) | EQP-1A style passive tube EQ | Simultaneous boost/cut, LCR resonance, tube warmth |
| [**Dynamic EQ**](modules/dynamic_eq.md) | 4-band frequency-dependent dynamics | Compress, expand, or gate per band |
| [**Transformer**](modules/transformer.md) | Input/output transformer coloration | 4 vintage hardware models |
| [**Haas**](modules/haas.md) | Psychoacoustic stereo widener | M/S + Haas comb, mono-compatible modes |
| [**Punch**](modules/punch.md) | Transparent clipper + transient shaper | Up to 8× oversampling |

---

## Quick Links

- [Module Reference](modules/index.md) — every parameter explained
- [Settings Examples & Techniques](presets/techniques.md) — NY compression, Pultec trick, Neve warmth
- [Genre Signal Chains](presets/genres.md) — Pop, Hip-Hop, Rock, EDM, Jazz, Death Metal
- [Instrument Bus Presets](presets/buses.md) — Drum, Bass, Guitar, Vocal, Synth, Master
- [Installation](install.md) — download, paths, build from source

---

## Architecture

The audio thread is **lock-free and allocation-free**:

- No `Mutex`, no heap allocation in `process()`
- Biquad filters updated via `update_coefficients()` — no state reset on parameter changes, no clicks
- ButterComp2 FFI: one call per buffer (O(1) overhead, not O(block_size))
- Dynamic EQ: 0.05 dB hysteresis gate prevents redundant `cos/sin/powf` calls when envelope is stable
- Punch: pre-clip transient shaping prevents post-clip pumping artifacts

[Download the latest release :material-download:](https://github.com/fsecada01/bus_channel_strip/releases/latest){ .md-button .md-button--primary }
[View on GitHub :material-github:](https://github.com/fsecada01/bus_channel_strip){ .md-button }
