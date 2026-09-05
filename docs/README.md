# docs/

Internal project documentation — architecture decisions, process, and research. This is **not** the published documentation site; that's `../site/` (Astro + Starlight, deployed to GitHub Pages), which is the canonical source for user-facing module/preset/install docs.

| File / dir | Contents |
|---|---|
| `adr/` | Architecture Decision Records — the *why* behind DSP, UI, and integration decisions. Start with `adr/README.md`. |
| `SYSTEM_PROMPT.md` | Extended AI session context, `@`-imported by the repo's `CLAUDE.md` — don't move or rename without updating that reference. |
| `TESTING.md` | Testing strategy. |
| `V2_ROADMAP.md` | Forward-looking roadmap (not a decision record — decisions made while executing it should get their own ADR). |
| `CLIPPING_INSIGHTS.md` | Research notes underpinning the Punch module's clipping design (see [ADR-0005](adr/0005-punch-pre-clip-transient-shaping.md)). |

## History

As of 2026-09-05 this directory was cleaned up: module/preset/install docs that duplicated the live site (`site/src/content/docs/`) were removed, one-shot planning docs that were already executed or superseded were removed, and design specs containing real architectural decisions (Haas, Punch, Sheen, the multi-FX rack redesign, ButterComp2's FFI approach, the GUI color-coding system, and the Mix Advisor integration) were converted into ADRs 0004–0010. ADR-0010 was originally drafted from the pre-implementation spec and described a design that didn't match what had actually been built (`advisor/`, `scripts/bcs_advisor/`, `jsfx/`); it was corrected shortly after to describe the real Rust-broker architecture and mark itself Implemented. See git history for the removed originals if deeper implementation detail is ever needed.
