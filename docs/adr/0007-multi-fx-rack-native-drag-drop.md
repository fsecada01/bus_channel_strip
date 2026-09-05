# ADR-0007: Multi-FX Rack — Slot-and-Library Model with Native vizia Drag-Drop

**Status**: Implemented

**Date**: 2026-04 (rack redesign); drag-drop rewritten 2026-04-26 after research pass
**Deciders**: Project (Claude Code orchestration session)

---

## Context

The original fixed 7-module horizontal strip (1800×650, per-module widths 140–320px) had no UI for the `module_order_*` params that already existed in the backend — reordering only worked via DAW automation — and no concept of an empty slot (modules were always present, only bypassable). Slate Digital's Virtual Mix Rack, Audio Assault's Mix Locker, and IK Multimedia's MixBox all solve this with a common pattern: separate the *rack* (ordered slots) from the *library* (available modules).

A first drag-drop implementation used a hand-rolled `on_press_down` + chassis-level `MouseUp` capture state machine. It failed silently and unpredictably. A five-agent research pass (`claudedocs/research_synthesis_20260426-203348.md`) traced this to baseview's Win32 `SetCapture` lifecycle having no `WM_CAPTURECHANGED` handler — a mouse-up delivered outside the host window could leave capture permanently stuck, and the hand-rolled state machine was structurally the wrong shape for vizia's event model (see also [[feedback_vizia_mouse_capture]]).

## Decision

**Adopt the rack/library separation** as the GUI's structural model: slots are first-class (`empty | filled | focused | bypassed`), backed by the existing `module_order_1..7` params (an `Empty` slot uses a sentinel index the audio thread treats as pass-through). Seven slots — matching the seven-module chain, giving one spare "Empty" lane — with the module library living in a **left sidebar as the sole add path** (an earlier per-slot inline picker was removed in a consolidation pass; one mental model for adding modules beats two competing affordances).

**Replace the hand-rolled drag state machine with vizia's built-in `on_drag`/`on_drop` API.** The slot body itself is the drag source (no separate `≡` handle — matches the VMR convention); `on_drag` fires the moment the cursor leaves the source view with LMB held, and `on_drop` fires on the target's `MouseUp` through normal hover resolution rather than manual capture. A `WindowEvent::MouseLeave` handler at the chassis root emits `DragCancel` defensively if a drag is in flight when the cursor leaves the editor window, mitigating the underlying [vizia#407](https://github.com/vizia/vizia/issues/407) capture-lifecycle gap rather than working around it with more hand-rolled state.

**Swap-or-push hit-test on drop**, matching the stated UX requirement: cursor X position within the target slot's bounds at release determines the outcome — left third = insert before (push later slots right), middle third = swap source↔target, right third = insert after. `Data::reorder` only emits `RawParamEvent`s for slots that actually changed position, minimizing preset diff size and avoiding spurious automation events.

**Focus mode demoted to keyboard-only** (`1`–`7` to focus a real-module slot, `Esc` to exit) rather than click-to-focus on the module body, since the body is now the drag source and click semantics would compete with drag-start. Empty slots are non-focusable. The mini-map and per-slot `≡ DRAG` handle from the original design were removed as redundant once the always-visible rack and body-as-drag-source landed.

## Consequences

**Easier:**
- Drag-drop now works reliably because it uses vizia's actual event model instead of fighting it — no more silent failures traced back to `SetCapture`.
- Adding a module is a single, unambiguous action path (sidebar), eliminating a class of "which picker did I mean" confusion the original two-affordance design had.

**Harder:**
- Any custom interaction that wants both click-select and drag from the same element needs to explicitly resolve that conflict per vizia's model — this codebase resolved it here by giving drag-start exclusive claim to the slot body and pushing focus to explicit keyboard shortcuts.
- The `MouseLeave`-triggered `DragCancel` is a mitigation for a real upstream vizia/baseview bug, not a fix; if vizia#407 changes behavior upstream this codepath should be re-verified.

**Unchanged:**
- The underlying `module_order_*` parameter scheme, and its role as the source of truth for signal-chain order, is unchanged by the UI rewrite — only how the GUI writes to it changed.
- Chain presets, mini-map, and macro controls remain out of scope for this pass (see the design doc's original Non-Goals) — no decision was made to add them, they were simply never part of this ADR's scope.
