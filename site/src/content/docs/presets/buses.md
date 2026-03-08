---
title: Instrument Bus Settings
description: Optimized starting configurations for Drum, Bass, Guitar, Vocal, Synth, and Master bus processing.
---

Optimized starting configurations for common stem/bus processing scenarios. Each bus has different goals — use these as starting points and adjust to suit your mix.

:::note[Module Reference]
Each technique below links to the full module reference. See [API5500 EQ](/bus_channel_strip/modules/api5500/), [ButterComp2](/bus_channel_strip/modules/buttercomp2/), [Pultec EQ](/bus_channel_strip/modules/pultec/), [Dynamic EQ](/bus_channel_strip/modules/dynamic_eq/), [Transformer](/bus_channel_strip/modules/transformer/), and [Punch](/bus_channel_strip/modules/punch/) for parameter details and Quick Start settings.
:::

---

## Drum Bus

**Goal:** Impact, cohesion, transient punch, controlled low end

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 30 Hz · Low shelf: +2 dB @ 60 Hz · HM boost: +3 dB @ 4 kHz |
| **ButterComp2** | Compress: `0.50` · Output: `0.85` · **Dry/Wet: `0.48`** (NY parallel) |
| **Pultec EQ** | LF boost: `0.45` @ 60 Hz + LF cut: `0.35` (Pultec trick) |
| **Dynamic EQ** | B1: Compress 80 Hz · Thr −22 dB · R 3:1 · B3: Compress 3 kHz · Thr −18 dB · R 2:1 |
| **Transformer** | Model: Vintage · Input Drive: `0.35` |
| **Punch** | Mode: Cubic · Ceiling: −0.5 dBFS · Transient Atk: `0.55` |

- Use Dry/Wet 0.45–0.55 on ButterComp2 for drum buses — this is the classic NY parallel approach that brings up room mics and subtle hits without squashing transients
- The Dynamic EQ 3 kHz band compresses hash frequencies from snare wire and cymbal bleed without dulling overall brightness
- Punch Transient Attack at 0.55 restores kick and snare punch after the compression stages

---

## Bass Bus

**Goal:** Defined low end, even dynamics, clarity in the mix

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 30 Hz · Low-mid cut: −1 dB @ 300 Hz · HM boost: +1 dB @ 1 kHz |
| **ButterComp2** | Compress: `0.45` · Output: `0.80` · Dry/Wet: `0.70` |
| **Pultec EQ** | LF boost: +2 dB @ 30–60 Hz · HF boost: +1.5 dB @ 5 kHz |
| **Dynamic EQ** | B1: Compress 80 Hz · Thr −18 dB · R 3:1 · Attack 5 ms · Release 150 ms |
| **Transformer** | Model: Vintage · Input Drive: `0.20` · Input Sat: `0.15` |
| **Punch** | Mode: Soft · Ceiling: −1.0 dBFS |

- Set the Dynamic EQ detector frequency to 80 Hz (the fundamental) rather than full-range — this prevents high-frequency transients from triggering the sub compressor, which would cause pumping
- The Pultec 5 kHz HF boost adds the "click" of the bass pick or finger attack, improving definition in dense mixes
- Vintage Transformer at low drive adds weight and warmth without obvious saturation

---

## Guitar Bus

**Goal:** Cohesion across multiple guitar tracks, presence, controlled low mids

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 80 Hz · Low-mid cut: −3 dB @ 200 Hz · HM boost: +2 dB @ 3 kHz |
| **ButterComp2** | Compress: `0.40` · Output: `0.82` · Dry/Wet: `0.50` |
| **Pultec EQ** | HF boost: +2.5 dB @ 8–10 kHz · HF BW: `0.55` |
| **Dynamic EQ** | B2: Compress 200 Hz · Thr −18 dB · R 2:1 (mud control) |
| **Transformer** | Model: British · Input Drive: `0.40` · Input Sat: `0.35` |
| **Punch** | Mode: Soft · Ceiling: −0.8 dBFS · Transient Atk: `0.25` |

- The 200 Hz low-mid cut on API5500 is critical for distorted guitars — this is where guitar mud accumulates and causes the "boxy" sound
- British Transformer captures the SSL console sound common in hard rock and pop-rock
- Keep ButterComp2 Dry/Wet below 0.55 for guitars — heavy compression on a multi-mic'd guitar bus can kill the dynamics that make the part feel human

---

## Vocal Bus

**Goal:** Polished, even, present; de-essed; forward in the mix

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 100 Hz · HM boost: +3 dB @ 3 kHz |
| **ButterComp2** | Compress: `0.35` · Output: `0.78` · Dry/Wet: `0.40` |
| **Pultec EQ** | HF boost: +2 dB @ 10 kHz (air) · Tube Drive: `0.12` |
| **Dynamic EQ** | B3: Compress 7 kHz · Thr −12 dB · R 2.5:1 · Atk 4 ms · Rel 60 ms (de-essing) |
| **Transformer** | Model: Vintage · Input Drive: `0.15` · Input Sat: `0.10` |
| **Punch** | Mode: Soft · Ceiling: −2.0 dBFS · Mix: `0.70` |

- Use the Dynamic EQ Band 3 de-essing approach rather than a separate de-esser — it integrates into the gain staging chain and lets you set a precise frequency-dependent threshold
- Vintage Transformer at very low drive adds barely perceptible harmonics that give vocals that "recorded through analog hardware" quality
- Punch at Mix 0.70 allows transient variation in the vocal delivery to come through — fully wet limiter on vocals can make them sound mechanical

---

## Synth / Keys Bus

**Goal:** Spectral density, warmth, clarity in the frequency spectrum

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 40 Hz · Low-mid: subtle adjustments for the specific synths |
| **ButterComp2** | Compress: `0.55` · Output: `0.88` · Dry/Wet: `0.60` |
| **Pultec EQ** | HF boost: +3 dB @ 10–15 kHz (sparkle) |
| **Dynamic EQ** | B3: Compress harsh resonances at 3–5 kHz if needed |
| **Transformer** | Model: Modern · Input Drive: `0.25` · High Resp: `+0.4` |
| **Punch** | Mode: Soft · Ceiling: −0.5 dBFS |

- Synth buses often contain a wide range of frequency content depending on patches — inspect the spectral analyzer in Dynamic EQ before applying any cuts
- Modern Transformer with extended high response adds API "presence" character without warming the sound

---

## Master Bus

**Goal:** Final glue, polish, and loudness maximization

:::caution[Use Sparingly]
Master bus processing should be subtle. If it's obvious, the mix needs adjustment at the stem level. The goal is to make the mix sound like it was recorded through great equipment — not to fix problems.
:::

| Module | Settings |
|--------|----------|
| **API5500 EQ** | Very gentle final sweetening only |
| **ButterComp2** | Compress: `0.30` · Output: `0.75` · Dry/Wet: `0.38` |
| **Pultec EQ** | Subtle: LF boost `0.20` @ 30 Hz + HF boost `0.30` @ 12 kHz |
| **Dynamic EQ** | Transparency mode — minimal engagement, only address specific issues |
| **Transformer** | Model: Vintage · Input Drive: `0.12` · Output Drive: `0.08` |
| **Punch** | Mode: Soft · Ceiling: −0.3 to −0.1 dBFS · OS: 4× or 8× |

### Streaming Targets

| Platform | Integrated LUFS | True Peak |
|----------|----------------|-----------|
| Spotify | −14 LUFS | −1.0 dBTP |
| Apple Music | −16 LUFS | −1.0 dBTP |
| YouTube | −14 LUFS | −1.0 dBTP |
| CD / Physical | As loud as needed | −0.3 dBTP |
| Mp3 encoding | Any | −1.0 dBTP (intersample headroom) |
