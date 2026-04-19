---
title: Module Reference
description: Overview of all seven Bus Channel Strip DSP modules and their signal chain order.
---

Bus Channel Strip contains seven DSP modules in a serial signal chain. Each module:

- Has an individual **bypass** switch
- Is **fully automatable** (VST3 and CLAP)
- Can be **reordered** via the drag-to-swap handles in the GUI
- Runs on a **lock-free, allocation-free** audio thread

## Signal Chain (default order)

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

This default order reflects a classic mastering/bus processing workflow:

1. **Corrective EQ** (API5500) — address tonal imbalances before compression
2. **Glue compression** (ButterComp2) — unify elements dynamically
3. **Tonal shaping** (Pultec) — add character after dynamics are controlled
4. **Frequency-dependent dynamics** (Dynamic EQ) — surgical per-band control
5. **Harmonic coloration** (Transformer) — analog warmth and character
6. **Stereo widening** (Haas) — spatial placement before the final ceiling
7. **Peak limiting** (Punch) — transparent ceiling catches any widener-induced peaks

## Modules

| Module | Source | Purpose |
|--------|--------|---------|
| [API5500 EQ](/bus_channel_strip/modules/api5500/) | Custom Rust | 5-band semi-parametric console EQ |
| [ButterComp2](/bus_channel_strip/modules/buttercomp2/) | Airwindows C++ (FFI) | Bipolar interleaved glue compression |
| [Pultec EQ](/bus_channel_strip/modules/pultec/) | Custom Rust | Passive EQP-1A with LCR resonance, bandwidth control, 18 dB range |
| [Dynamic EQ](/bus_channel_strip/modules/dynamic_eq/) | Custom Rust | 4-band frequency-dependent dynamics |
| [Transformer](/bus_channel_strip/modules/transformer/) | Custom Rust | Analog transformer coloration (4 models) |
| [Haas](/bus_channel_strip/modules/haas/) | Custom Rust | Psychoacoustic stereo widener (M/S + comb) |
| [Punch](/bus_channel_strip/modules/punch/) | Custom Rust | Clipper + transient shaper, 8× oversampling |
