---
title: Parameter Reference
description: Complete reference for all ~75 automation parameters
---

:::caution[Parameter ID Stability]
Parameter IDs are permanent. Changing an `#[id = "..."]` value breaks existing DAW sessions and presets. Never rename IDs — add new ones with new names instead.
:::

Parameter IDs are written into every DAW session file and preset. The display name (shown in the DAW) can change, but the `#[id]` string never can.

---

## Global

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Gain | `gain` | -30 to +30 | 0 | dB | Master output gain. Stored as linear, displayed as dB. Logarithmic smoothing (50ms) |

---

## API5500 EQ

5-band semi-parametric equalizer. LF and HF are shelving filters. LMF, MF, and HMF are fully parametric with Q control.

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| EQ Bypass | `eq_bypass` | on/off | off | — | Bypasses entire API5500 EQ module |
| LF Freq | `lf_freq` | 20–400 | 100 | Hz | Low shelf frequency. Skewed range |
| LF Gain | `lf_gain` | -15 to +15 | 0 | dB | Low shelf boost/cut |
| LMF Freq | `lmf_freq` | 50–2000 | 200 | Hz | Low-mid parametric center frequency |
| LMF Gain | `lmf_gain` | -15 to +15 | 0 | dB | Low-mid boost/cut |
| LMF Q | `lmf_q` | 0.1–10 | 0.7 | — | Low-mid bandwidth. Skewed range |
| MF Freq | `mf_freq` | 200–8000 | 1000 | Hz | Mid parametric center frequency |
| MF Gain | `mf_gain` | -15 to +15 | 0 | dB | Mid boost/cut |
| MF Q | `mf_q` | 0.1–10 | 0.7 | — | Mid bandwidth. Skewed range |
| HMF Freq | `hmf_freq` | 1000–15000 | 3000 | Hz | High-mid parametric center frequency |
| HMF Gain | `hmf_gain` | -15 to +15 | 0 | dB | High-mid boost/cut |
| HMF Q | `hmf_q` | 0.1–10 | 0.7 | — | High-mid bandwidth. Skewed range |
| HF Freq | `hf_freq` | 3000–20000 | 10000 | Hz | High shelf frequency |
| HF Gain | `hf_gain` | -15 to +15 | 0 | dB | High shelf boost/cut |

---

## ButterComp2

Airwindows ButterComp2 compressor. Ported from C++ via `extern "C"` FFI. Controls follow the Airwindows parameter convention (0.0–1.0 normalized ranges with internal mapping).

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Comp Bypass | `comp_bypass` | on/off | off | — | Bypasses compressor module |
| Compress | `comp_compress` | 0–1 | 0 | — | Compression amount. 0 = no compression, 1 = maximum |
| Comp Output | `comp_output` | 0–1 | 0.5 | — | Output level. 0.5 = unity gain |
| Comp Mix | `comp_dry_wet` | 0–1 | 1 | — | Dry/wet blend. 1.0 = fully compressed |

---

## Pultec EQ

Pultec EQP-1A style passive equalizer with simultaneous boost/cut and tube saturation. Frequencies follow the original hardware's stepped switch positions.

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Pultec Bypass | `pultec_bypass` | on/off | off | — | Bypasses Pultec EQ module |
| LF Boost Freq | `pultec_lf_boost_freq` | 20–100 | 60 | Hz | Low-frequency boost center frequency |
| LF Boost | `pultec_lf_boost_gain` | 0–1 | 0 | — | Low-frequency shelf boost amount |
| LF Atten | `pultec_lf_cut_gain` | 0–1 | 0 | — | Low-frequency attenuation amount |
| HF Boost Freq | `pultec_hf_boost_freq` | 5000–20000 | 10000 | Hz | High-frequency boost center. Skewed range |
| HF Boost | `pultec_hf_boost_gain` | 0–1 | 0 | — | High-frequency boost amount |
| HF Bandwidth | `pultec_hf_boost_bandwidth` | 0–1 | 0.5 | — | High-frequency boost bandwidth |
| HF Atten Freq | `pultec_hf_cut_freq` | 5000–20000 | 10000 | Hz | High-frequency attenuation frequency. Skewed range |
| HF Atten | `pultec_hf_cut_gain` | 0–1 | 0 | — | High-frequency attenuation amount |
| Tube Drive | `pultec_tube_drive` | 0–1 | 0.2 | — | Tube saturation character. 0.2 = subtle default |

---

## Dynamic EQ

4-band frequency-dependent compressor. Each band has independent frequency, threshold, ratio, attack/release, gain, Q, range, detector link, detector frequency, processing mode, enabled, and solo controls. Requires the `dynamic_eq` feature flag.

Each band's detector listens at the band's own frequency while **Det Link** is on (the default); switch it off to listen at **Detector Freq** instead. **Range** caps how far the band's gain can move in either direction. A thin bar under THRESH in the DynEQ view shows the detector's live level against the threshold.

Bands: Band 1 (Low, default 200 Hz), Band 2 (Low-Mid, default 800 Hz), Band 3 (High-Mid, default 3 kHz), Band 4 (High, default 8 kHz).

The table below shows Band 1 parameters. Bands 2–4 follow identical structure with `band2_`, `band3_`, `band4_` prefixes and different default values.

### Band 1 (Low) — default 200 Hz

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| DynEQ Bypass | `dyneq_bypass` | on/off | off | — | Bypasses entire Dynamic EQ module |
| DynEQ 1 Freq | `dyneq_band1_freq` | 20–2000 | 200 | Hz | Band 1 center frequency. Skewed range |
| DynEQ 1 Thresh | `dyneq_band1_threshold` | -60 to 0 | -18 | dB | Detection threshold |
| DynEQ 1 Ratio | `dyneq_band1_ratio` | 1–20 | 4 | — | Compression ratio. Skewed range |
| DynEQ 1 Attack | `dyneq_band1_attack` | 0.1–200 | 10 | ms | Attack time. Skewed range |
| DynEQ 1 Release | `dyneq_band1_release` | 1–2000 | 100 | ms | Release time. Skewed range |
| DynEQ 1 Gain | `dyneq_band1_gain` | -18 to +18 | 0 | dB | Band gain |
| DynEQ 1 Q | `dyneq_band1_q` | 0.3–8 | 1 | — | Band Q. Skewed range |
| DynEQ 1 On | `dyneq_band1_enabled` | on/off | on | — | Enable/disable this band |
| DynEQ 1 Detector Freq | `dyneq_band1_detector_freq` | 20–2000 | 200 | Hz | Detector frequency, used only while Det Link is off |
| DynEQ 1 Mode | `dyneq_band1_mode` | enum | CompressDownward | — | `CompressDownward`, `ExpandUpward`, or `Gate` |
| DynEQ 1 Solo | `dyneq_band1_solo` | on/off | off | — | Solo this band for monitoring |
| DynEQ 1 Det Link | `dyneq_band1_detector_link` | on/off | on | — | Detector listens at Freq instead of Detector Freq |
| DynEQ 1 Range | `dyneq_band1_range` | 0–30 | 18 | dB | Largest gain change in either direction |

### Band 2 (Low-Mid) — default 800 Hz

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| DynEQ 2 Freq | `dyneq_band2_freq` | 200–5000 | 800 | Hz | Skewed range |
| DynEQ 2 Thresh | `dyneq_band2_threshold` | -60 to 0 | -18 | dB | |
| DynEQ 2 Ratio | `dyneq_band2_ratio` | 1–20 | 4 | — | |
| DynEQ 2 Attack | `dyneq_band2_attack` | 0.1–200 | 10 | ms | |
| DynEQ 2 Release | `dyneq_band2_release` | 1–2000 | 100 | ms | |
| DynEQ 2 Gain | `dyneq_band2_gain` | -18 to +18 | 0 | dB | |
| DynEQ 2 Q | `dyneq_band2_q` | 0.3–8 | 1 | — | |
| DynEQ 2 On | `dyneq_band2_enabled` | on/off | on | — | |
| DynEQ 2 Detector Freq | `dyneq_band2_detector_freq` | 200–5000 | 800 | Hz | |
| DynEQ 2 Mode | `dyneq_band2_mode` | enum | CompressDownward | — | |
| DynEQ 2 Solo | `dyneq_band2_solo` | on/off | off | — | |
| DynEQ 2 Det Link | `dyneq_band2_detector_link` | on/off | on | — | |
| DynEQ 2 Range | `dyneq_band2_range` | 0–30 | 18 | dB | |

### Band 3 (High-Mid) — default 3 kHz

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| DynEQ 3 Freq | `dyneq_band3_freq` | 1000–15000 | 3000 | Hz | |
| DynEQ 3 Thresh | `dyneq_band3_threshold` | -60 to 0 | -18 | dB | |
| DynEQ 3 Ratio | `dyneq_band3_ratio` | 1–20 | 4 | — | |
| DynEQ 3 Attack | `dyneq_band3_attack` | 0.1–200 | 5 | ms | Faster default than Band 1/2 |
| DynEQ 3 Release | `dyneq_band3_release` | 1–2000 | 60 | ms | |
| DynEQ 3 Gain | `dyneq_band3_gain` | -18 to +18 | 0 | dB | |
| DynEQ 3 Q | `dyneq_band3_q` | 0.3–8 | 1 | — | |
| DynEQ 3 On | `dyneq_band3_enabled` | on/off | on | — | |
| DynEQ 3 Det Freq | `dyneq_band3_detector_freq` | 1000–15000 | 3000 | Hz | |
| DynEQ 3 Mode | `dyneq_band3_mode` | enum | CompressDownward | — | |
| DynEQ 3 Solo | `dyneq_band3_solo` | on/off | off | — | |
| DynEQ 3 Det Link | `dyneq_band3_detector_link` | on/off | on | — | |
| DynEQ 3 Range | `dyneq_band3_range` | 0–30 | 18 | dB | |

### Band 4 (High) — default 8 kHz

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| DynEQ 4 Freq | `dyneq_band4_freq` | 3000–20000 | 8000 | Hz | |
| DynEQ 4 Thresh | `dyneq_band4_threshold` | -60 to 0 | -18 | dB | |
| DynEQ 4 Ratio | `dyneq_band4_ratio` | 1–20 | 4 | — | |
| DynEQ 4 Attack | `dyneq_band4_attack` | 0.1–200 | 2 | ms | Fastest default — high-frequency transients |
| DynEQ 4 Release | `dyneq_band4_release` | 1–2000 | 30 | ms | |
| DynEQ 4 Gain | `dyneq_band4_gain` | -18 to +18 | 0 | dB | |
| DynEQ 4 Q | `dyneq_band4_q` | 0.3–8 | 1 | — | |
| DynEQ 4 On | `dyneq_band4_enabled` | on/off | on | — | |
| DynEQ 4 Det Freq | `dyneq_band4_detector_freq` | 3000–20000 | 8000 | Hz | |
| DynEQ 4 Mode | `dyneq_band4_mode` | enum | CompressDownward | — | |
| DynEQ 4 Solo | `dyneq_band4_solo` | on/off | off | — | |
| DynEQ 4 Det Link | `dyneq_band4_detector_link` | on/off | on | — | |
| DynEQ 4 Range | `dyneq_band4_range` | 0–30 | 18 | dB | |

---

## Transformer

Vintage transformer coloration with 4 models. Adds harmonic saturation, frequency-dependent response shaping, and transformer loading compression.

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Transformer Bypass | `transformer_bypass` | on/off | off | — | Bypasses Transformer module |
| Transformer Model | `transformer_model` | enum | Vintage | — | `Vintage`, `Iron`, `Modern`, `Warm` |
| Input Drive | `transformer_input_drive` | 0–1 | 0.2 | — | Input stage saturation drive |
| Input Saturation | `transformer_input_saturation` | 0–1 | 0.3 | — | Input saturation character |
| Output Drive | `transformer_output_drive` | 0–1 | 0.1 | — | Output stage drive |
| Output Saturation | `transformer_output_saturation` | 0–1 | 0.4 | — | Output saturation character |
| Low Response | `transformer_low_response` | -1 to +1 | 0 | — | Low-frequency transformer response shaping. 0 = flat |
| High Response | `transformer_high_response` | -1 to +1 | 0 | — | High-frequency transformer response shaping. 0 = flat |
| Transformer Compression | `transformer_compression` | 0–1 | 0.3 | — | Transformer core loading/compression |

---

## Punch

Clipper + Transient Shaper with 8x oversampling. Bypassed by default — the user must enable it intentionally. Requires the `punch` feature flag.

Transient detection occurs **before** the clipper to avoid post-clip gain modulation artifacts (pumping).

### Clipper Section

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Punch Bypass | `punch_bypass` | on/off | **on** | — | Bypassed by default. Enable intentionally |
| Clip Threshold | `punch_threshold` | -12 to 0 | -0.1 | dB | Clip ceiling. Default -0.1 dB ≈ near-0 dB |
| Clip Mode | `punch_clip_mode` | enum | Soft | — | `Hard`, `Soft` (tanh), `Cubic` (polynomial knee) |
| Softness | `punch_softness` | 0–1 | 0.3 | — | Soft/cubic clip knee width |
| Oversampling | `punch_oversampling` | enum | X8 | — | `X1`, `X2`, `X4`, `X8`, `X16`. X8 default |

### Transient Shaper Section

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Attack | `punch_attack` | -1 to +1 | 0 | — | Transient attack boost/cut. 0 = neutral |
| Sustain | `punch_sustain` | -1 to +1 | 0 | — | Transient sustain boost/cut. 0 = neutral |
| Attack Time | `punch_attack_time` | 0.1–30 | 5 | ms | Transient detector attack time. Skewed range |
| Release Time | `punch_release_time` | 10–500 | 100 | ms | Transient detector release time. Skewed range |
| Sensitivity | `punch_sensitivity` | 0–1 | 0.5 | — | Transient detector sensitivity |

### Global Punch Controls

| Parameter Name | ID | Range | Default | Unit | Notes |
|---------------|-----|-------|---------|------|-------|
| Punch Input | `punch_input_gain` | -12 to +12 | 0 | dB | Pre-processing input gain |
| Punch Output | `punch_output_gain` | -12 to +12 | 0 | dB | Post-processing output gain |
| Punch Mix | `punch_mix` | 0–1 | 1 | — | Dry/wet blend. 1.0 = fully processed |

---

## Module Order

Six parameters define the runtime processing order of all modules. Each accepts any `ModuleType` value, allowing any permutation.

| Parameter Name | ID | Default | Notes |
|---------------|-----|---------|-------|
| Module Order 1 | `module_order_1` | Api5500EQ | First in chain |
| Module Order 2 | `module_order_2` | ButterComp2 | Second in chain |
| Module Order 3 | `module_order_3` | PultecEQ | Third in chain |
| Module Order 4 | `module_order_4` | Transformer | Fourth in chain |
| Module Order 5 | `module_order_5` | Punch | Fifth in chain |
| Module Order 6 | `module_order_6` | DynamicEQ | Sixth in chain |

Valid `ModuleType` values: `Api5500EQ`, `ButterComp2`, `PultecEQ`, `DynamicEQ`, `Transformer`, `Punch`.

:::note
The default chain places DynamicEQ last (position 6). This is intentional — Dynamic EQ with sidechain analysis is a "surgical" tool that works best after the main tone-shaping modules have run.
:::
