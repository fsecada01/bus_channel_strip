# Multi-FX UI Design

Design specification for the bus channel strip's next-generation interface.
Shifts the GUI from a fixed 7-module horizontal strip to a slot-and-library
rack model inspired by Slate Digital's Virtual Mix Rack (VMR) and Audio
Assault's Mix Locker.

## Motivation

The current GUI renders all seven modules side-by-side in a 1800x650
layout. This has three structural problems:

1. **Density ceiling** — with modules ranging 140-320px and ~75 parameters
   on screen at once, every control is small and reading the chain state
   at a glance is hard.
2. **No UI for reordering** — `module_order_1..6` parameters exist in
   the backend, but users can only change order via DAW automation. The
   signal chain is effectively fixed.
3. **No notion of empty slots** — modules are always present and either
   active or bypassed. There is no way to remove a module from the graph
   or substitute it.

VMR, Mix Locker, and IK Multimedia's MixBox solve these same problems
with a common architectural pattern: separate the *rack* (ordered slots)
from the *library* (available modules). That pattern is what this doc
specifies.

## Inspiration Sources

| Product | Pattern borrowed |
|---------|------------------|
| Slate Digital VMR | 8-slot rack, Dream Strips (chain presets), module library sidebar, drag-and-drop reordering, consistent module chrome |
| Audio Assault Mix Locker | Compact faceplate grid, free + paid module distinction treated as one UI |
| IK Multimedia MixBox | 500-series slot metaphor, click-to-focus detail view |

Patterns explicitly *not* adopted: skeuomorphic screw ornamentation,
simple/advanced mode toggles, and macro controls (see Non-Goals).

## Design Principles

1. **Rack, not strip.** Slots are first-class. A slot is `empty`, `filled`,
   `focused`, or `bypassed`. Modules occupy slots; they are not the slots
   themselves.
2. **Two densities, one layout.** Default view shows all slots in compact
   form. Clicking a module promotes it to a focus panel; siblings collapse
   to header-only cards. No separate "advanced mode" — same UI, different
   focus.
3. **Consistent chrome, expressive content.** Every module shares the same
   frame (title bar, bypass, drag handle, I/O meter, GR meter). Color
   coding and interior controls carry the module identity.
4. **Signal flow is always legible.** Left-to-right layout, visible arrows
   between slots, bypassed modules grayed. A mini-map at the top shows the
   full chain even when a module is focused.
5. **Chain presets over macros.** Ship stock chains ("Drum Bus", "Vocal
   Bus", "Mix Buss Glue", "Mastering Lite"). Do not ship a macro
   assignment UI in this phase.

## Architecture

### Slot component

A slot is a component with four mutually-exclusive states:

```
Slot {
    index: 0..=5,              // position in chain
    state: SlotState,
    module: Option<ModuleKind>, // None when Empty
}

enum SlotState {
    Empty,      // no module assigned; shows "+" to open library picker
    Filled,     // module assigned, active, compact view
    Focused,    // module assigned, expanded, full controls
    Bypassed,   // module assigned, audio passes through untouched
}
```

Backend mapping: the existing `module_order_1..6` params encode
`ModuleKind` indices; an `Empty` slot uses a sentinel value (e.g. 255)
that the audio thread treats as pass-through. Each module's existing
`bypass` param drives the `Bypassed` state.

### Module library

A sidebar (left edge, collapsible) or top-bar dropdown lists the
available `ModuleKind` values:

- API5500 EQ
- ButterComp2
- Pultec EQ
- Dynamic EQ
- Transformer
- Haas
- Punch

Users drag a library item onto a slot, or click "+" on an empty slot and
pick from a menu. Duplicate placement is allowed only if DSP state is
cheap to duplicate — for v1, restrict to one instance per module.

### Focus view

Clicking a filled slot toggles focus. Layout:

```
+------------------------------------------------------------+
| [chain mini-map: API > Comp > PULTEC > Dyn > Tx > Punch]   |
+------------------------------------------------------------+
| [API][C][ PULTEC — FOCUSED, full controls       ][D][T][P] |
|  hdr  hdr  ------- expanded 600px wide ---------  hdr hdr hdr
+------------------------------------------------------------+
```

Sibling slots collapse to 80px header cards showing: name, bypass
toggle, GR meter, I/O meter. Clicking a sibling swaps focus.

Escape key or clicking the focused module's title bar returns to the
default strip view.

### Chain presets

Store a chain preset as a named snapshot of:
- Slot ordering (`module_order_*`)
- Per-module bypass state
- A small curated set of "signature" params per module (not all ~75)

Surfaced as a top-bar selector with 6-8 stock chains. Users can save
their own. Chain presets are independent from full plugin presets —
changing a chain preset does not overwrite fine-tuned parameter values
in unrelated modules.

Stock chains (initial proposal):

| Name | Chain order | Notes |
|------|-------------|-------|
| Drum Bus | Transformer > API5500 > ButterComp2 > Punch | Transformer first for glue, Punch for transient control |
| Vocal Bus | Pultec > API5500 > Dynamic EQ > ButterComp2 | Pultec shine, DynEQ for de-ess, Comp last |
| Mix Buss Glue | API5500 > ButterComp2 > Pultec > Transformer | Classic broadcast order |
| Mastering Lite | Dynamic EQ > Pultec > ButterComp2 > Punch | Surgical > musical > glue > ceiling |
| Wide Bus | API5500 > ButterComp2 > Haas > Transformer | Stereo enhancement workflow |
| Empty | (all slots empty) | Start-from-scratch |

## Visual Design

### Slot dimensions

Normalize slot widths to a single rhythm. Default strip view:

- Compact slot: 220px wide × 520px tall
- Focused slot: 600px wide × 520px tall
- Sibling collapsed: 80px wide × 520px tall (header only)

Modules that need more real estate (Dynamic EQ's 4 bands, Transformer's
many stages) get a "double-wide" flag (440px compact) sparingly — VMR
allows this pattern for e.g. FG-EQ and it reads cleanly.

### Chrome

Every slot renders the same outer frame:

```
+----------------------------------+
| [drag]  MODULE NAME      [bypass]|  <- title bar, 28px
+----------------------------------+
| [I/O meter]   ... controls ...   |
|                                  |
|                    [GR meter]    |
+----------------------------------+
```

The title bar uses the module's accent color as a thin top border; the
body uses the module background color. This preserves the color
identity while enforcing visual consistency.

### Color coding

Unchanged from current `GUI_DESIGN.md`:

| Module | Background | Accent |
|--------|-----------|--------|
| API5500 EQ | `#3C5064` | `#00C8FF` cyan |
| ButterComp2 | `#282828` | `#FF8C00` orange |
| Pultec EQ | `#786450` | `#FFD700` gold |
| Dynamic EQ | `#465A78` | `#00FF64` green |
| Transformer | `#3C2D2D` | `#C8503C` oxide red |
| Haas | TBD (proposed: `#2D3C4A` with `#64C8FF` light blue) | |
| Punch | `#3A3050` | `#00A0FF` electric blue |

### Signal flow mini-map

A 32px-tall strip above the rack shows every slot as a colored pill
with an arrow between pills:

```
[ API5500 ] -> [ ButterComp2 ] -> [ Pultec ] -> [ empty ] -> [ Transformer ] -> [ Punch ]
```

Clicking a pill focuses that slot. Bypassed pills are desaturated;
empty slots render as dashed outlines.

## Interaction Model

### Reordering

Drag a slot's title bar (via the drag handle icon) and drop between two
other slots. During drag:
- Dragged slot becomes semi-transparent
- Drop targets show a vertical insertion indicator
- Releasing outside the rack cancels the operation

On drop, the corresponding `module_order_*` params are updated
atomically via the `ParamSetter` API. The audio thread picks up the new
order on the next block boundary (no glitches — existing smoothing
handles the transition).

### Adding / removing modules

- Empty slot "+" button → opens library menu → pick module → slot fills
- Focused slot "remove" action (in title-bar overflow menu) → slot
  becomes empty
- Drag from library sidebar onto empty slot → fills slot
- Drag from library sidebar onto filled slot → replaces (with confirm)

### Bypass vs remove

Distinct actions:
- **Bypass** keeps the module in the chain with its state intact; audio
  passes through. Toggleable from the title bar.
- **Remove** returns the slot to empty; module state is lost. Action
  lives in an overflow menu to prevent accidents.

### Keyboard

| Key | Action |
|-----|--------|
| `Esc` | Exit focus view |
| `1`..`6` | Focus slot N |
| `B` | Toggle bypass on focused slot |
| `Delete` | Remove module from focused slot (with confirm) |
| `Tab` / `Shift+Tab` | Cycle focus between slots |

## Migration Path

The current fixed-column layout does not need to ship alongside the new
rack — this is a full replacement. Staged implementation:

1. **Slot component scaffolding** — add `Slot` view, wire to
   `module_order_*` params, render existing modules inside slot chrome
   without layout changes yet.
2. **Library sidebar** — add the library panel, implement drag-to-slot
   for adding/replacing modules, keep current positions as default.
3. **Focus view** — add compact/focused slot states, implement sibling
   collapse, wire up keyboard shortcuts.
4. **Chain presets** — add top-bar selector, define stock chains, wire
   to snapshot save/restore.
5. **Mini-map + polish** — add top strip mini-map, drag-to-reorder
   animations, drop indicators.

Each step is independently shippable. Steps 1-2 deliver the most user
value; step 3 onward is refinement.

## Non-Goals

These are deliberately out of scope for this redesign:

- **Macro controls** (VMR feature). Mapping multiple params to one
  knob is a sizable UX surface and adds complexity to preset storage.
  Defer to a later phase.
- **Simple/advanced mode toggle**. The focus view delivers the same
  benefit (less visual noise when working on one module) without
  doubling UI work.
- **Skin/theme system**. Single polished theme > multiple mediocre ones.
- **Cross-instance drag-and-drop** (VMR has this). Requires a session
  service or DAW-specific hooks; not worth it for a single-strip
  plugin.
- **Per-module wet/dry mix**. Modules that need mix already have it
  (ButterComp2, Punch); adding it universally is out of scope.
- **MIDI learn / A-B compare**. Referenced in existing GUI_DESIGN.md
  future work; still deferred.

## Open Questions

1. **Slot count.** VMR has 8; we have 7 modules. Fix to 7 slots or
   allow up to 8 with empty slots? Recommendation: 6 slots, enforce
   one-instance-per-module — matches current `module_order_1..6`.
2. **Library position.** Left sidebar (VMR) or top-bar dropdown
   (Mix Locker)? Sidebar eats width in a bus plugin; dropdown is
   tighter. Recommendation: top-bar dropdown, revisit if library grows.
3. **Slot width normalization.** Several modules currently use wider
   panels (Dynamic EQ 200px, Transformer 320px, Punch 320px). Enforce
   220px uniform or allow declared "double-wide"? Recommendation:
   allow double-wide, cap at 2 per chain to keep total under 1800px.
4. **Chain preset storage.** Embed in plugin preset file or separate
   JSON? Recommendation: embedded — ensures DAW session restores
   everything atomically.

## References

- Slate Digital VMR 3.0 — <https://slatedigital.com/vmr-3/>
- VMR documentation — <https://docs.slatedigital.com/VMR/Virtual%20Mix%20Rack.html>
- MusicRadar VMR review — <https://www.musicradar.com/reviews/tech/slate-digital-virtual-mix-rack-616256>
- Audio Assault Mix Locker — <https://audioassault.mx/getmixlocker>
- Mix Locker announcement — <https://bedroomproducersblog.com/2025/03/03/audio-assault-mix-locker/>
- Voger Design — Audio plugin UX — <https://vogerdesign.com/blog/make-audio-plugin-with-great-ux/>
- Best practices for audio plugin UI — <https://www.numberanalytics.com/blog/best-practices-audio-plugin-ui>
