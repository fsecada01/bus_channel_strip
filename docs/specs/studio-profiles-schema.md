# Studio Profiles Schema — Mix Advisor Integration

## Overview

Studio profiles encode the sonic character of legendary recording studios and
celebrated producer/engineer approaches as structured data. The Bus Channel Strip
ReaScript mix advisor uses these profiles to translate a high-level creative brief
("make this sound like it was recorded at Power Station in 1982") into concrete
parameter guidance across all six modules.

Each profile models one of three entity types:
- **Studio** — a physical room and its equipment complement
- **Engineer/Producer** — a working methodology and preferred processing chain
- **Combination** — a blended entity (studio + artist in an unfamiliar genre)

---

## JSON Schema (studio-profile-v1)

```jsonc
{
  "$schema": "studio-profile-v1",
  "id": "string",              // kebab-case unique identifier
  "display_name": "string",
  "entity_type": "studio | engineer | combination",
  "era": "string",             // e.g. "1978–1993"
  "genre_gates": ["string"],   // genres this profile is authoritative for
                               // empty = no restriction
  "description": "string",     // 1-2 sentences for UI tooltip / LLM context

  // ── Physical room (studios only, null for engineers) ──────────────────────
  "room": {
    "location": "string",
    "acoustics": "dead | controlled | live | very-live",
    "low_end_character": "tight | neutral | warm | boomy",
    "high_end_character": "airy | neutral | present | dark",
    "reverb_tail_ms": 200       // approximate RT60 in ms
  },

  // ── Console character ─────────────────────────────────────────────────────
  "console": {
    "manufacturer": "string",   // e.g. "SSL", "Neve", "API", "MCI", "Trident"
    "model": "string",          // e.g. "4000E", "8078", "2488"
    "character": "transparent | punchy | warm | colored | aggressive",
    "transformer_color": "subtle | moderate | heavy",
    "bus_compression": "none | light | moderate | heavy"
  },

  // ── Tape machine (null if mixing to digital) ──────────────────────────────
  "tape": {
    "machine": "string",        // e.g. "Studer A80", "MCI JH-24", null
    "formula": "string",        // e.g. "Ampex 456", "Quantegy GP9"
    "speed": "15ips | 30ips",
    "saturation": "clean | light | moderate | heavy"
  },

  // ── Outboard signature ────────────────────────────────────────────────────
  "outboard": {
    "eq_preference": {
      "primary": "string",      // e.g. "Pultec EQP-1A", "API 550A", "Neve 1073"
      "character": "string",    // plain English: "passive, musical shelves"
      "low_shelf_tendency": "cut | flat | boost",
      "high_shelf_tendency": "cut | flat | boost"
    },
    "compression_preference": {
      "primary": "string",      // e.g. "SSL G-Bus", "UREI 1176", "Fairchild 670"
      "type": "VCA | optical | FET | tube | vari-mu",
      "ratio_tendency": "gentle | moderate | heavy",
      "attack_tendency": "fast | medium | slow",
      "release_tendency": "fast | medium | slow | program-dependent"
    },
    "notable_outboard": ["string"]  // other signature gear
  },

  // ── Mix tendencies (human-readable, LLM context) ─────────────────────────
  "mix_tendencies": {
    "low_end": "string",
    "midrange": "string",
    "high_end": "string",
    "dynamics": "string",
    "stereo_field": "string",
    "signature_moves": ["string"]
  },

  // ── Direct Bus Channel Strip hints ───────────────────────────────────────
  // Values are suggestions (0.0–1.0 normalized) + rationale strings.
  // The advisor presents these as "targets" with explanation, not hard-sets.
  "module_parameter_hints": {
    "api5500": {
      "band1_freq": 0.15,       // ~60 Hz: gentle low foundation
      "band1_gain": 0.55,       // slight boost (+1 to +2 dB)
      "band5_freq": 0.82,       // ~10 kHz "air"
      "band5_gain": 0.58,
      "rationale": "string"
    },
    "buttercomp2": {
      "threshold": 0.42,
      "ratio": 0.35,            // ~2:1 gentle glue
      "attack": 0.25,           // medium-slow
      "release": 0.60,          // program-dependent
      "dry_wet": 0.80,          // parallel comp tendency
      "rationale": "string"
    },
    "pultec": {
      "lf_boost_freq": 0.3,     // 60 Hz
      "lf_boost": 0.45,
      "lf_atten": 0.2,
      "hf_boost_freq": 0.7,     // 10 kHz
      "hf_boost": 0.5,
      "rationale": "string"
    },
    "dynamic_eq": {
      "band1_threshold": 0.4,
      "band1_ratio": 0.3,
      "band2_threshold": 0.5,
      "band2_ratio": 0.25,
      "rationale": "string"
    },
    "transformer": {
      "model": 0.0,             // 0=Trident, 0.33=API, 0.66=Neve, 1.0=SSL
      "drive": 0.35,
      "rationale": "string"
    },
    "punch": {
      "clip_ceiling": 0.85,     // -0.3 dBFS default
      "transient_attack": 0.4,
      "transient_sustain": 0.5,
      "rationale": "string"
    }
  },

  // ── Landmark records (for LLM context and UI display) ────────────────────
  "landmark_records": [
    {
      "artist": "string",
      "album": "string",
      "year": 1982,
      "engineer": "string",
      "producer": "string"
    }
  ],

  // ── Combination metadata (entity_type == "combination" only) ─────────────
  "combination": {
    "base_studio": "string",    // profile id
    "base_engineer": "string",  // profile id
    "genre_shift": "string",    // target genre
    "blend_notes": "string",    // narrative of how the blend was constructed
    "domain_authority": {
      // which profile drives which module dimension
      "api5500":       "studio | engineer",
      "buttercomp2":   "studio | engineer",
      "pultec":        "studio | engineer",
      "dynamic_eq":    "studio | engineer",
      "transformer":   "studio | engineer",
      "punch":         "studio | engineer"
    }
  }
}
```

---

## Blending Algorithm (Combination Profiles)

When building a combination profile, each module dimension is owned by one source
using a **domain authority** rule:

| Dimension | Default Authority | Override Condition |
|-----------|------------------|--------------------|
| Room acoustics → DynEQ crossover character | Studio | — |
| Console EQ character → API5500 / Pultec | Studio | Engineer has strong signature EQ preference |
| Compression type → ButterComp2 model | Engineer | Studio's house compressor is dominant |
| Transformer saturation | Studio | Engineer known for heavy or clean saturation |
| Transient shaping / Punch | Engineer | Engineer's genre gate is active |
| High-frequency air | Studio | — |

**Conflict detection**: if studio and engineer disagree on compression ratio tendency
by more than 2 steps (e.g., studio=gentle, engineer=heavy), flag it to the user
with a rationale and default to the engineer's preference.

**Genre gate**: if the combination involves a genre outside both profiles' `genre_gates`,
apply a conservative bridge — reduce drive, increase dry_wet to favor parallel
processing, and add a note that neither profile has direct authority.

---

## Normalization Map (ReaScript Integration)

The hints use 0.0–1.0 normalized values that map to NIH-plug parameter ranges.
The ReaScript reads actual parameter ranges via `TrackFX_GetParamIdent` +
`TrackFX_GetParam` and interpolates:

```lua
-- Convert normalized hint to actual plugin value
local function hint_to_value(track, fx, param_idx, normalized)
  local min, max = reaper.TrackFX_GetParamRange(track, fx, param_idx)
  return min + normalized * (max - min)
end
```

The A/B snapshot system saves `TrackFX_GetParam` raw values (not normalized)
so DAW automation and session recall work correctly.
