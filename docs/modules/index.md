# Module Reference

Bus Channel Strip contains six DSP modules arranged in a serial signal chain. Each module:

- Has an individual **bypass** switch
- Is **fully automatable** (VST3 and CLAP)
- Can be **reordered** relative to other modules via drag-to-swap UI handles
- Operates on a **lock-free, allocation-free** audio thread

---

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
  <span class="node node-punch">Punch</span>
</div>

This default order reflects a classic mastering/bus processing workflow:

1. **Corrective EQ** (API5500) — address tonal imbalances before compression
2. **Glue compression** (ButterComp2) — unify elements dynamically
3. **Tonal shaping** (Pultec) — add character after dynamics are controlled
4. **Frequency-dependent dynamics** (Dynamic EQ) — surgical per-band control
5. **Harmonic coloration** (Transformer) — analog warmth and character
6. **Peak limiting** (Punch) — transparent ceiling with transient restoration

---

## Module Pages

| | Module | Source |
|-|--------|--------|
| :material-equalizer: | [API5500 EQ](api5500.md) | Custom Rust implementation |
| :material-waveform: | [ButterComp2](buttercomp2.md) | Airwindows C++ (FFI) |
| :material-radio: | [Pultec EQ](pultec.md) | Custom Rust implementation |
| :material-chart-bell-curve: | [Dynamic EQ](dynamic_eq.md) | Custom Rust implementation |
| :material-power-plug: | [Transformer](transformer.md) | Custom Rust implementation |
| :material-lightning-bolt: | [Punch](punch.md) | Custom Rust implementation |
