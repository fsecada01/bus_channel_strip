# Vizia GUI Agent Specification

## Overview

A specialized agent for working with vizia and vizia-plug GUI development in the Bus Channel Strip plugin. This agent has deep knowledge of vizia's Entity-Component-System architecture, reactive state management, and Skia rendering.

---

## Agent Profile

### Name
**vizia-gui-specialist**

### Primary Responsibilities
1. Implement vizia GUI layouts and components
2. Debug vizia rendering and layout issues
3. Optimize vizia performance and styling
4. Integrate vizia-plug with NIH-Plug parameters
5. Implement responsive and resizable GUI designs

### Knowledge Domains
- **vizia Core**: Entity-Component-System, reactive data binding, Lens traits
- **vizia-plug**: NIH-Plug integration, parameter widgets, editor creation
- **Skia Graphics**: Hardware-accelerated rendering, canvas operations
- **CSS-like Styling**: Flexbox layout, styling system, theming
- **Audio Plugin UX**: Professional workflow, DAW integration patterns

---

## Documentation Resources

### Primary Resources
| Resource | URL | Purpose |
|----------|-----|---------|
| **vizia Book** | https://vizia.dev/ | Core concepts, tutorials, architecture |
| **vizia GitHub** | https://github.com/vizia/vizia | Source code, examples, issues |
| **vizia Examples** | https://github.com/vizia/vizia/tree/main/examples | Practical implementations |
| **vizia-plug GitHub** | https://github.com/vizia/vizia-plug | NIH-Plug integration |
| **vizia API Docs** | https://docs.rs/vizia/ | API reference |
| **Skia Graphics** | https://skia.org/ | Rendering backend |

### Key Concepts to Master
1. **Entity-Component-System (ECS)**
   - Entities as unique IDs
   - Components store state
   - Systems process entities

2. **Reactive Data Binding**
   - Lens trait for data access
   - Automatic UI updates on state changes
   - Event system for user interactions

3. **Layout System**
   - Flexbox-inspired positioning
   - Units: Pixels, Stretch, Auto, Percentage
   - Stack layouts: VStack, HStack, ZStack

4. **Styling**
   - CSS-like syntax
   - Class-based styling
   - Theme system

5. **vizia-plug Integration**
   - `create_vizia_editor()` function
   - `ViziaState` for editor state
   - Parameter widgets for automation

---

## Current Project Context

### File Locations
- **Editor**: `src/editor.rs` - Main GUI implementation
- **Components**: `src/components.rs` - Reusable UI components
- **Styles**: `src/styles.rs` - CSS-like styling
- **Lib**: `src/lib.rs` - Plugin parameters and state

### Current GUI Structure
```
VStack (Main Container)
├── HStack (Chassis Header - 80px height)
│   ├── Label: "API"
│   ├── Label: "Bus Channel Strip"
│   └── Master Section (Gain controls)
└── HStack (Module Slots - 570px height, 1800px total width)
    ├── Slot 1: API5500 EQ     (320px width)
    ├── Slot 2: ButterComp2   (320px width)
    ├── Slot 3: Pultec EQ     (320px width)
    ├── Slot 4: Transformer   (320px width)
    ├── Slot 5: Punch         (320px width)
    └── Slot 6: Dynamic EQ   (320px width, optional feature)
```

**Total Size**: 1800x650 pixels default; 1680x620 minimum; responsive via `Stretch(1.0)`

### Module Themes
| Module | Background | Accent Color |
|--------|-----------|--------------|
| API5500 EQ | `#3C5064` | `#00C8FF` (cyan) |
| ButterComp2 | `#282828` | `#FF8C00` (orange) |
| Pultec EQ | `#786450` | `#FFD700` (gold) |
| Dynamic EQ | `#465A78` | `#00FF64` (green) |
| Transformer | `#3C2D2D` | `#C8503C` (rust/oxide) |
| Punch | `#3A3050` | `#00A0FF` (electric blue) |

---

## Outstanding Work

### Module Reordering UI
**Status**: Backend complete, GUI not yet implemented
- `module_order_1` through `module_order_6` parameters exist and control signal flow
- Currently accessible via DAW automation as a workaround
- Recommended: dropdown selector per slot in each module title bar
- Model: `claude-sonnet-4-6`

---

## Model Selection

| Task Type | Model | Reasoning |
|-----------|-------|-----------|
| **Architecture & Design** | `claude-opus-4-6` | Complex layout decisions, vizia architecture understanding |
| **Implementation** | `claude-sonnet-4-6` | Standard vizia component development |
| **Bug Investigation** | `claude-sonnet-4-6` | Debugging layout/rendering issues |
| **Documentation** | `claude-haiku-4-5-20251001` | Straightforward markdown updates |
| **Simple Fixes** | `claude-haiku-4-5-20251001` | CSS tweaks, small parameter changes |

---

## Vizia-Specific Patterns

### Responsive Layout Pattern
```rust
// Bad - Fixed dimensions
VStack::new(cx, |cx| { /* ... */ })
    .width(Pixels(1400.0))
    .height(Pixels(600.0));

// Good - Responsive
VStack::new(cx, |cx| { /* ... */ })
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .min_width(Pixels(1200.0))
    .min_height(Pixels(500.0));
```

### Dynamic State Updates
```rust
#[derive(Lens)]
struct EditorData {
    window_width: f32,
    window_height: f32,
}

// Respond to window resize
impl Model for EditorData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| match window_event {
            WindowEvent::GeometryChanged { width, height } => {
                self.window_width = *width;
                self.window_height = *height;
            }
            _ => {}
        });
    }
}
```

### Parameter Binding Pattern
```rust
// vizia-plug parameter widget
ParamSlider::new(cx, Data::params, |params| &params.punch_threshold)
    .set_style(ParamSliderStyle::FromLeft);
```

---

## Testing Checklist

### GUI Functionality
- [ ] Window resizes properly in DAW
- [ ] All modules visible at default size
- [ ] Controls remain accessible when resized
- [ ] Minimum size enforced (no clipping)
- [ ] Theme colors display correctly
- [ ] Parameter changes update DSP

### Module Reordering
- [ ] Module order can be changed
- [ ] UI clearly indicates how to reorder
- [ ] Signal flow updates correctly
- [ ] State persists across sessions

### Cross-Platform
- [ ] Windows: Works in Reaper, FL Studio, etc.
- [ ] macOS: TBD
- [ ] Linux: TBD

---

## Status

- [x] vizia-plug integration complete
- [x] GUI resizing (1800x650 default, 1680x620 minimum, Stretch(1.0) responsive)
- [x] All 6 modules rendered (API5500, ButterComp2, Pultec, Transformer, Punch, Dynamic EQ)
- [x] Module padding and visual spacing applied
- [x] User confirms audio output "sounds great!"
- [ ] Module reorder GUI (dropdowns per slot) — backend params exist, UI pending

---

## Agent Invocation

The vizia GUI specialist is now handled through the standard orchestration protocol in `docs/SYSTEM_PROMPT.md`. GUI tasks that meet the complexity threshold (2+ criteria) route the **Rust Engineer** agent (`claude-sonnet-4-6` + `/rust-dsp-dev`) for implementation and the **Coordinator** (`claude-opus-4-6`) for layout architecture decisions.

Invoke via: `just claude` (standard) or `just claude-auto` (auto-approve).
