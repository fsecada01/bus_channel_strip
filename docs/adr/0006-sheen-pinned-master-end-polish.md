# ADR-0006: Sheen — Pinned Master-End Polish Coat, Default-On, Excluded from Auto-Gain

**Status**: Implemented (v1.0.0)

**Date**: 2026-04-28 (spec sign-off)
**Deciders**: Project (Claude Code orchestration session)

---

## Context

The plugin wanted a "sounds like hardware out of the box" default character — a curated, always-on finishing stage — without adding a front-panel control surface that competes with the seven reorderable modules for attention. Three separate architectural questions needed resolving: where this stage sits in the signal path and whether it should be reorderable like the other modules; whether it should be on or off by default; and whether the plugin's global auto-gain compensation should normalize its output.

## Decision

**Sheen is pinned post-Punch, pre-master-gain, and is explicitly not a slot module** — not in `enum ModuleType`, not in `module_order_*` params, not reorderable. This is intentional: the polish coat is a chassis property, not a user choice. Treating it as a reorderable slot would let a user accidentally place it before the clipper, defeating the design (the five stages are meant to shape what the clipper ultimately sees). Discovery happens through a brushed-brass brand-plate affordance (click → back-view panel with five sliders) rather than a slot UI, mutually exclusive with the existing DynEQ back view.

**Five-stage pipeline, fixed processing order:** BODY (low shelf) → PRESENCE (peak) → AIR (high shelf) → WARMTH (Sonnox-Inflator-style waveshaper) → WIDTH (frequency-dependent M/S side scaler). EQ stages precede the waveshaper so warmth's harmonic generation reacts to the already-tonally-shaped signal; warmth precedes width so the generated harmonic content also passes through M/S processing rather than being layered on top of it afterward (the Clariphonic/Vitamin convention).

Each stage's fixed frequency/Q and default gain came from cross-referencing multiple published polish-plugin teardowns (Slate Thickness, Pultec 100 Hz, Vitamin LO/HI bands, Pensado factory presets, Maag AIR band, iZotope Ozone, Soothe2) rather than any single hardware unit — notably, AIR's high-shelf default (+1.8 dB @ 14 kHz) is a **deliberate perceptual-compensation choice**, not hardware emulation: console-bus heritage research found no measured air bump in SSL G / Neve 33609 / API 2500 / Studer / Trident (all flat to 20 kHz at unity). WARMTH uses the public-domain reverse-engineered Sonnox Inflator polynomial at Curve=0 (the most-loved balanced setting) rather than either of two alternative tape/transformer-harmonics polynomials that were also researched — those are deferred as possible v1.1 alternate warmth modes rather than shipped now, since Inflator has the broadest user familiarity and is mathematically well-established.

**Ships default-on** (`sheen_bypass = false`) at the researched factory values. This is the core product framing decision: the plugin should sound finished immediately, with the brass plate serving only as an optional discovery layer. Consequence accepted explicitly: **DAW sessions saved before Sheen existed will play back subtly differently** once the plugin updates, since Sheen is now always in the signal path at those defaults. Users wanting bit-identical pre-1.0 playback must manually flip the back-panel master bypass. The plugin's existing `global_bypass` param still bypasses the entire chain including Sheen, so a clean dry/wet A/B remains available.

**Excluded from `global_auto_gain` compensation.** Auto-gain on a polish stage would defeat the polish — the whole point of the WARMTH stage's +1 LU of perceived loudness and BODY/AIR's RMS gain is that it's audible, not an artifact to normalize away.

## Consequences

**Easier:**
- No new module-order UI surface needed; Sheen's discovery mechanism reuses the existing DynEQ back-view pattern (mutual exclusion, Esc-to-close) rather than inventing a new interaction model.
- Product story is simple to communicate: "it already sounds right," with an escape hatch for power users.

**Harder:**
- Any future module that also wants master-end placement must negotiate ordering against Sheen's fixed position explicitly — the pinned-module pattern isn't generalized to more than one instance yet.
- The DAW-session playback change on upgrade is a real compatibility break that must be called out in every release's notes going forward, not just v1.0.0's.

**Unchanged:**
- The "RESTORE FACTORY" back-panel action is UI-only (fires `SetParameterNormalized` events), not itself an automatable parameter — avoids ambiguous DAW-session state.
- Auto-gain continues to compensate every other module normally; only Sheen is exempted.
