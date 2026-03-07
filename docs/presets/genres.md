# Genre Signal Chains

Complete mix bus processing chains for specific genres. Settings assume the default module order. Reorder modules as needed for your workflow.

---

## Modern Pop / R&B

**Goal:** Clean, polished, radio-ready · Punchy low end · Smooth, non-fatiguing highs

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 30 Hz · Low-mid cut: −2 dB @ 280 Hz · HM boost: +1.5 dB @ 3 kHz · High shelf: +2 dB @ 12 kHz |
| **ButterComp2** | Compress: `0.45` · Output: `0.85` · Dry/Wet: `0.60` |
| **Pultec EQ** | LF boost: +2 dB @ 60 Hz · LF cut: `0.30` · HF boost: +2 dB @ 10 kHz |
| **Dynamic EQ** | B1: Compress 100 Hz · Thr −24 dB · R 2:1 (sub tighten) · B3: Compress 3 kHz · Thr −16 dB · R 1.5:1 (smooth presence) |
| **Transformer** | Model: Modern · Input Drive: `0.25` · Input Sat: `0.20` · High Resp: `+0.3` |
| **Punch** | Mode: Soft · Ceiling: −0.5 dBFS · OS: 4× · Transient Atk: `0.30` |

**Notes:** The API5500 low-mid cut at 280 Hz prevents the vocal-guitar-bass competition zone from getting congested. ButterComp2 at Dry/Wet 0.60 provides cohesion without obvious compression. The Modern Transformer model adds API-style extended high end.

---

## Hip-Hop / Trap

**Goal:** Massive low end · Punchy kick and snare · Crystal clear highs · Maximum loudness

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 25 Hz · Low shelf: +3 dB @ 80 Hz · High shelf: +4 dB @ 16 kHz |
| **ButterComp2** | Compress: `0.60` · Output: `0.92` · Dry/Wet: `0.70` |
| **Pultec EQ** | LF boost: +5 dB @ 60 Hz · LF cut: `0.50` · HF boost: +3 dB @ 8 kHz |
| **Dynamic EQ** | B1: Expand Up 60 Hz · Thr −30 dB · R 2:1 (enhance sub) · B2: Gate 200 Hz · Thr −20 dB (clear mud between hits) |
| **Transformer** | Model: Vintage · Input Drive: `0.50` · Input Sat: `0.40` · High Resp: `+0.5` |
| **Punch** | Mode: Cubic · Ceiling: −0.3 dBFS · OS: 4× · Transient Atk: `0.55` |

**Notes:** The Pultec LF trick at 60 Hz (simultaneous boost + cut) is essential — pure boost without cut creates uncontrolled low-mid bloom. The Dynamic EQ Band 1 Expand Up enhances sub on hard-hitting passages while the Band 2 Gate cleans up the mud zone on rests. Vintage Transformer adds Neve-style weight.

---

## Rock / Alternative

**Goal:** Power and presence · Tight low end · Guitar cut-through · Dynamic punch

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 40 Hz · Low-mid cut: −3 dB @ 300 Hz · HM boost: +2 dB @ 2.5 kHz · High shelf: +1.5 dB @ 10 kHz |
| **ButterComp2** | Compress: `0.50` · Output: `0.85` · Dry/Wet: `0.65` |
| **Pultec EQ** | LF boost: +2.5 dB @ 100 Hz · HF boost: +2 dB @ 10 kHz |
| **Dynamic EQ** | B2: Compress 300 Hz · Thr −18 dB · R 2:1 · B3: Expand Up 2 kHz · Thr −24 dB · R 2:1 (presence on peaks) |
| **Transformer** | Model: British · Input Drive: `0.40` · Input Sat: `0.35` |
| **Punch** | Mode: Hard · Ceiling: −0.5 dBFS · OS: 4× · Transient Atk: `0.45` |

**Notes:** The low-mid cut at 300 Hz is critical for rock — distorted guitars produce enormous energy here and it causes "mush." British Transformer approximates the SSL G-Bus compression character common in rock production. Hard clip mode adds harmonic grit consistent with the genre.

---

## Electronic / EDM

**Goal:** Maximum loudness · Perfect clarity · Sub impact · Total cohesion

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 20 Hz · Low shelf: +2 dB @ 60 Hz · High shelf: +3 dB @ 16 kHz |
| **ButterComp2** | Compress: `0.65` · Output: `0.95` · Dry/Wet: `0.80` |
| **Pultec EQ** | LF boost: +4 dB @ 60 Hz · LF cut: `0.40` · HF boost: +4 dB @ 15 kHz |
| **Dynamic EQ** | B1: Compress sub · B2: Gate mud · B3: Compress 4 kHz harsh resonances · B4: Expand Up HF air |
| **Transformer** | Model: Modern · Input Drive: `0.30` · High Resp: `+0.6` |
| **Punch** | Mode: Soft · Ceiling: −0.1 dBFS · OS: 8× · Transient Atk: `0.40` |

**Notes:** ButterComp2 at Dry/Wet 0.80 creates maximum cohesion — all synths, drums, and samples become one unified sound. 8× oversampling on Punch is appropriate here since loudness maximization is the primary goal and CPU headroom is usually not a constraint in an EDM production with limited track count.

---

## Jazz / Acoustic

**Goal:** Natural warmth · Preserved dynamics · Depth and dimension · Minimal intervention

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: 30 Hz · Low shelf: +1.5 dB @ 80 Hz · HM: +1 dB @ 5 kHz |
| **ButterComp2** | Compress: `0.25` · Output: `0.72` · Dry/Wet: `0.30` |
| **Pultec EQ** | LF boost: +1 dB @ 60 Hz (optional) · Tube Drive: `0.10` |
| **Dynamic EQ** | B1: Compress 100 Hz · Thr −20 dB · R 1.5:1 only · other bands bypassed |
| **Transformer** | Model: Vintage · Input Drive: `0.15` · Input Sat: `0.10` · Low Resp: `+0.2` |
| **Punch** | Mode: Soft · Ceiling: −2.0 dBFS · OS: 2× · Mix: `0.60` |

**Notes:** Dynamics are a feature in jazz and acoustic music, not a problem to solve. ButterComp2 at Dry/Wet 0.30 provides barely-perceptible harmonic glue. The Punch ceiling at −2.0 dBFS preserves the natural dynamic range while preventing the occasional loud peak from causing digital overs.

---

## Death Metal / Extreme Metal

**Goal:** Maximum aggression · Tight controlled low end · Scything guitar presence · Wall of sound

| Module | Settings |
|--------|----------|
| **API5500 EQ** | HP: **50 Hz** (cut sub mud from down-tuned guitars) · Low-mid cut: **−4 dB @ 280 Hz** (chunk, not mud) · HM boost: **+4 dB @ 3.5 kHz** (guitar cut, vocal articulation) · High shelf: **+2 dB @ 12 kHz** (cymbal definition) |
| **ButterComp2** | Compress: `0.52` · Output: `0.92` · **Dry/Wet: `0.90`** (maximum glue — everything is one instrument) |
| **Pultec EQ** | LF boost: **+4 dB @ 60 Hz** (bomb) · LF cut: `0.55` (tight, controlled) · HF boost: **+3 dB @ 12 kHz** · HF BW: `0.70` |
| **Dynamic EQ** | B1: Compress 60 Hz · Thr −20 dB · R 3:1 (control blast beat sub) · B2: Gate 200 Hz · Thr −18 dB (cut mud between riff hits) · B3: Expand Up 2.5 kHz · Thr −24 dB · R 2:1 (forward on peaks) |
| **Transformer** | Model: **British** · Input Drive: `0.60` · Input Sat: `0.50` · Output Drive: `0.40` · High Resp: `+0.4` |
| **Punch** | Mode: **Cubic** · Ceiling: −0.1 dBFS · **OS: 8×** · Transient Atk: `0.65` · Mix: `1.0` |

!!! note "Death Metal Context"
    Down-tuned guitars (B standard or lower) create enormous sub-frequency content below 60 Hz. Without the aggressive 50 Hz HP filter, this sub energy destroys clarity and wastes headroom on frequencies the listener can't hear musically. The −4 dB cut at 280 Hz prevents guitars from creating a wall of mud while preserving the "chunk" of palm-muted riffs.

    **Dry/Wet 0.90 on ButterComp2** welds the double-kick, guitars, and vocals into a single coherent wall — the hallmark of Morbid Angel, Death, and Cannibal Corpse productions from Morrisound Recording. The engineers there consistently used heavy bus compression to achieve the "one amplifier" sound.

    The **British Transformer at high drive** approximates the SSL G-Bus character that powered classic Morrisound productions. Tom Morris and Scott Burns consistently tracked and mixed through SSL 4000/6000 consoles, and the British model's tight, controlled saturation captures that aggressive-but-controlled low end.

    **8× oversampling** on Punch is appropriate because death metal mastering targets maximum loudness (−5 to −8 LUFS integrated is common) and the extra CPU cost of 8× is justified by the aliasing rejection at that loudness level.
