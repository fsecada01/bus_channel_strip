# ADR-0001: DynEQ Band Controls — Expand/Collapse Layout

**Status**: Accepted
**Date**: 2026-03-08
**Deciders**: Project (Claude Code orchestration session)

---

## Context

The Dynamic EQ back view shows 4 band columns, each with 8 sliders visible simultaneously:
`FREQ`, `THRESH`, `RATIO`, `Q`, `MODE`, `ATK ms`, `REL ms`, `GAIN`.

All 8 are always visible regardless of whether the user is actively adjusting them. This creates visual noise: the controls a user adjusts on every band (what the band does, where, and when) compete visually with controls that are typically set once and left alone (timing, Q, ratio fine-tuning).

The back view window is already 1820×820px. Adding future parameters (soft knee, dynamics model, M/S mode) would make the layout unmanageable without a grouping strategy.

---

## Decision

Split each band's controls into two tiers separated by a per-band expand/collapse toggle (▶/▼ chevron in the band header):

**Tier 1 — Essential (always visible):**
- `MODE` — what the band does (Compress / Expand / Gate)
- `FREQ` — where it acts
- `THRESH` — when it activates
- `GAIN` — makeup gain result

**Tier 2 — Advanced (hidden until expanded):**
- `RATIO` — compression depth
- `Q` — band width
- `ATK ms` — attack timing
- `REL ms` — release timing

Default state: collapsed. Expand state is per-band (bands are independent).

**Rationale for this grouping**: MODE + FREQ + THRESH are the minimum controls needed to hear and evaluate a band's effect. GAIN closes the gain-staging loop. RATIO, Q, and timing are character controls — set once during session setup, rarely touched thereafter.

---

## Consequences

**Easier:**
- Visual clarity in the compact state — 4 essential controls per band instead of 8
- Future parameters (dynamics model, soft knee, M/S) have a natural home in Tier 2
- Consistent with how engineers actually use dynamic EQ (dial in the band, then refine)

**Harder:**
- Expand state is session-only (not persisted to DAW state) — users must re-expand after reloading
- Requires `dyneq_band_expand: [bool; 4]` in `Data` and a `ToggleDynEqBandExpand(usize)` event

**Unchanged:**
- All parameters always exist in the DSP layer — no parameters are added or removed
- Audio thread is completely unaffected
- Existing DAW sessions load correctly
