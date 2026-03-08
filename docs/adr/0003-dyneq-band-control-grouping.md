# ADR-0003: DynEQ Band Control Grouping — Essential vs Advanced

**Status**: Accepted
**Date**: 2026-03-08
**Deciders**: Project (Claude Code orchestration session)

---

## Context

ADR-0001 established the expand/collapse pattern for DynEQ band controls. This ADR records the specific grouping decision: which 4 controls are Tier 1 (always visible) and which 4 are Tier 2 (expand to reveal).

Three grouping candidates were considered:

| Option | Tier 1 | Tier 2 |
|--------|--------|--------|
| A — Original order | FREQ, THRESH, RATIO | Q, MODE, ATK, REL, GAIN |
| B — Workflow order | MODE, FREQ, THRESH, GAIN | RATIO, Q, ATK, REL |
| C — Gain last | FREQ, THRESH, MODE | RATIO, Q, ATK, REL, GAIN |

---

## Decision

**Option B**: MODE, FREQ, THRESH, GAIN are always visible.

**Rationale:**

1. **MODE first**: The compression mode (Compress Down / Expand Up / Gate) determines the *semantic* of the band. A user evaluating a band needs to know what it does before interpreting any other value. It is the highest-priority control.

2. **FREQ and THRESH next**: Once MODE is known, FREQ (where) and THRESH (when) fully describe the band's operating point. Together with MODE, these three controls are sufficient to hear and evaluate the band's effect in context.

3. **GAIN in Tier 1**: Makeup gain closes the gain-staging loop for a band that is actively compressing. Moving it to Tier 2 would require an expand action any time the user compensates for GR — a common operation, not an advanced one.

4. **RATIO, Q, ATK, REL in Tier 2**: These are character controls. RATIO sets compression depth (often left at 4:1 for de-essing, 2:1 for gentle control). Q sets band width (narrower for surgical, wider for tonal). Attack and release shape the dynamic response. All four are typically configured during session setup and revisited infrequently.

---

## Consequences

**Easier:**
- Compact view is meaningful on its own — MODE + FREQ + THRESH + GAIN is a complete operational summary
- Future parameters (dynamics model from ADR plan Track C, soft knee, M/S mode) slot into Tier 2 without redesign

**Harder:**
- RATIO is hidden by default; users accustomed to seeing it may need to expand once per session
- Minor: label width for "THRESH" is longer than "Q" — CSS may need slight adjustment for compact alignment

**Unchanged:**
- Parameter IDs, DSP behavior, audio thread — none affected
