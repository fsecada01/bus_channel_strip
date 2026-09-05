# ADR-0009: GUI Framework — vizia-plug, with Per-Module Hardware-Inspired Color Coding

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session)

---

## Context

The plugin needed a GUI framework capable of ~86 automation parameters across seven-plus modules, DAW-automation-safe reactive updates, and a professional hardware-inspired look. The two mainstream Rust-ecosystem alternatives considered were egui (immediate-mode) and iced (Elm-architecture retained-mode); vizia-plug (Entity-Component-System, Skia rendering, `Lens`-based reactive state) was chosen instead.

Separately, with six-then-seven visually distinct modules on screen simultaneously, users needed a fast way to tell modules apart at a glance without reading labels — a plain uniform-chrome layout would make the chain unreadable at the density this plugin operates at.

## Decision

**Use vizia-plug as the GUI framework.** Its ECS architecture with `Lens`-based reactive parameter binding maps cleanly onto NIH-plug's parameter system, and Skia rendering supports the custom canvas drawing this plugin needs (the spectrum analyzer overlay, sidechain masking visualization). egui and iced are explicitly not to be substituted in — this is a standing constraint, not just an initial choice (see CLAUDE.md "What to Avoid").

**Assign every module a distinct background/accent color pair**, applied consistently across the module's slot chrome, controls, and metering:

| Module | Background | Accent |
|--------|-----------|--------|
| API5500 EQ | `#3C5064` blue-gray | `#00C8FF` cyan |
| ButterComp2 | `#282828` slate/black | `#FF8C00` orange |
| Pultec EQ | `#786450` brass | `#FFD700` gold |
| Dynamic EQ | `#465A78` steel blue | `#00FF64` green |
| Transformer | `#3C2D2D` charcoal | `#C8503C` oxide red |
| Haas | `#2D3C4A` | `#64C8FF` light blue |
| Punch | `#3A3050` deep purple | `#00A0FF` electric blue |

This scheme is deliberately hardware-inspired (evokes distinct outboard-gear faceplates rather than a uniform software-plugin look) and is treated as a stable visual identity — colors are preserved verbatim across GUI redesigns (the fixed-strip layout → slot-and-library rack transition, ADR-0007, kept every color unchanged; only the chrome around them changed).

## Consequences

**Easier:**
- Users can identify a module's function at a glance in the always-visible signal-flow mini-map and slot headers without reading text, even at small slot widths.
- Any future module gets a clear, low-ambiguity task: pick a background/accent pair visually distinct from all seven existing ones and register it in `ModuleTheme` (`src/components.rs`) — the pattern is already established, not something each new module has to design from scratch.

**Harder:**
- vizia-plug is a less mature ecosystem than egui/iced, which has produced real friction (documented across [[reference_vizia_patterns]] and [[reference_gui_state]] — missing `.inverted()` on `ParamButton`, CSS `box-shadow`/`transform` not reliably supported, Skia's non-femtovg canvas API, the `on_press_down` capture bug that drove ADR-0007). This is an accepted tradeoff, not an oversight.
- Color-blind accessibility for the module-identity scheme relies on position and shape cues as a fallback (per GUI design notes), not color alone — this should stay true for any new module color choice.

**Unchanged:**
- The color-coding table is documented in three places (CLAUDE.md, this ADR, and previously the now-removed `GUI_DESIGN.md`) — CLAUDE.md's summary is the quick-reference; this ADR is the decision record explaining *why* the scheme exists and *why* it doesn't change across redesigns.
