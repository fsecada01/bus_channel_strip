# DynEQ Band Expand/Collapse — Implementation Spec

## Overview

Each of the four DynEQ bands gains a chevron toggle (▶/▼) in its header that reveals or hides
Tier 2 controls (RATIO, Q, ATK ms, REL ms) via CSS `.display()` toggling. Expand state lives
exclusively in the GUI `Data` struct as an `Arc<[AtomicBool; 4]>` and never touches the audio
thread or plugin parameter system.

---

## State / Parameters

### `Data` struct addition

```rust
// In Data struct — GUI-only, never sent to audio thread
dyneq_band_expand: Arc<[AtomicBool; 4]>,
```

Initialization in `Data::new()`:

```rust
dyneq_band_expand: Arc::new([
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
]),
```

No plugin `Params` additions. No `#[id]` annotations. No audio thread state.

---

## Event Schema

### `AppEvent` addition

```rust
AppEvent::ToggleDynEQBand(usize),   // band index 0–3
```

### Handler in `Data::event()`

```rust
AppEvent::ToggleDynEQBand(idx) => {
    let current = self.dyneq_band_expand[idx].load(Ordering::Relaxed);
    self.dyneq_band_expand[idx].store(!current, Ordering::Relaxed);
    cx.emit(AppEvent::RedrawDynEQ);   // triggers .display() re-evaluation
}
```

`AppEvent::RedrawDynEQ` is a no-op event used solely to prompt vizia to re-check `.display()`
lenses on the Tier 2 VStack. Alternatively, wrap `Arc<[AtomicBool; 4]>` in an outer lens that
increments a `u32` counter stored alongside it so the lens invalidates naturally — see Lens
Workaround below.

---

## Lens Workaround

`[bool; 4]` does not implement vizia `Data`. `Arc<[AtomicBool; 4]>` alone is not lens-trackable.

**Recommended approach — counter + AtomicBool array:**

Add a companion `dyneq_expand_gen: u32` field to `Data` (implements `Data` via `Copy`). Increment
it in `ToggleDynEQBand`. The `.display()` map closure captures `band_idx` and `Arc` by clone:

```rust
let expand_arc = cx.data::<Data>().unwrap().dyneq_band_expand.clone();
tier2_vstack.display(
    Data::dyneq_expand_gen.map(move |_gen| {
        if expand_arc[band_idx].load(Ordering::Relaxed) {
            Display::Flex
        } else {
            Display::None
        }
    })
);
```

The `dyneq_expand_gen: u32` field in `Data` is the lens target; it increments on every toggle,
invalidating the map closure and causing vizia to re-evaluate `.display()`.

`Data` field additions (both required):

```rust
dyneq_band_expand: Arc<[AtomicBool; 4]>,
dyneq_expand_gen:  u32,                    // incremented on each toggle; u32 implements Data
```

---

## Macro Refactor

### Updated signature

```rust
macro_rules! dyneq_band_col {
    ($cx:expr, $label:expr, $mode:expr, $freq:expr, $thresh:expr, $gain:expr,
     $ratio:expr, $q:expr, $atk:expr, $rel:expr, $on:expr, $solo:expr,
     $band_idx:literal) => { ... }
}
```

`$band_idx` is a `usize` literal (0–3), used both for `ToggleDynEQBand` and the expand lens.

### Macro body structure

```
VStack (band column)
  ├── HStack (band header)
  │     ├── Label ($label)
  │     ├── ParamButton (ON — $on)
  │     ├── ParamButton (SOLO — $solo)
  │     └── Button "▶"/"▼" → cx.emit(AppEvent::ToggleDynEQBand($band_idx))
  │           .class("dyneq-chevron")
  ├── [Tier 1 — always visible]
  │     ParamSlider MODE, FREQ, THRESH, GAIN
  └── [Tier 2 VStack — .display(expand_lens)]
        ParamSlider RATIO, Q, ATK, REL
```

The Tier 2 VStack's `.display()` uses the lens pattern described in Lens Workaround above.
The chevron button label updates reactively using the same `dyneq_expand_gen` lens:

```rust
Label::new(cx, Data::dyneq_expand_gen.map(move |_| {
    if expand_arc2[band_idx].load(Ordering::Relaxed) { "▼" } else { "▶" }
}))
```

---

## Chevron Button

| Property | Value |
|----------|-------|
| Widget | `Button::new()` wrapping a `Label` |
| Placement | Rightmost element in band header `HStack` |
| CSS class | `.dyneq-chevron` |
| Label (collapsed) | `"▶"` |
| Label (expanded) | `"▼"` |
| On press | `cx.emit(AppEvent::ToggleDynEQBand($band_idx))` |
| Width | Fixed `Pixels(24.0)` |

CSS in `styles.rs`:

```css
.dyneq-chevron {
    background-color: transparent;
    border-width: 0px;
    color: #8899aa;
}
.dyneq-chevron:hover {
    color: #ffffff;
}
```

---

## Implementation Steps

1. Add `dyneq_band_expand: Arc<[AtomicBool; 4]>` and `dyneq_expand_gen: u32` to `Data`; initialize both in `Data::new()`.
2. Add `AppEvent::ToggleDynEQBand(usize)` variant; implement handler in `Data::event()` to flip the `AtomicBool` and increment `dyneq_expand_gen`.
3. Update `dyneq_band_col!` macro: add `$band_idx` parameter, add chevron `Button` to the header `HStack`, split controls into Tier 1 (always visible) and Tier 2 `VStack` with `.display()` gating.
4. Add `.dyneq-chevron` CSS rule to `styles.rs`.
5. Verify all four `dyneq_band_col!` call sites in `editor.rs` pass the correct literal band index (0–3).

---

## Guardrails

- **No new plugin params** — `dyneq_band_expand` and `dyneq_expand_gen` are GUI-only `Data` fields; they never appear in `Params`, never get `#[id]`, and never cross the audio thread boundary.
- **No `Binding::new()` for expand toggle** — use `.display()` only; `Binding::new()` tears down and rebuilds ECS subtrees causing layout jank.
- **No audio thread access** — `AtomicBool` is written only from `Data::event()` (UI thread); audio `process()` never reads or writes expand state.
- **No per-band ECS rebuild** — the Tier 2 VStack is always present in the ECS tree; only its CSS `display` property changes.
- **No layout size change for containing slot** — band columns must maintain fixed width regardless of expanded state; only height may change.
- **`Ordering::Relaxed` is correct here** — expand state is display-only with no cross-thread ordering requirement.

---

## Acceptance Criteria

1. Clicking the chevron on Band 1 shows its Tier 2 controls (RATIO, Q, ATK, REL); clicking again hides them — no other band is affected.
2. All four bands are independently expandable/collapsible in any combination simultaneously.
3. `cargo +nightly fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` all pass with no new failures.
4. The DynEQ slot width does not change when any band is expanded or collapsed; only the slot height adjusts.
