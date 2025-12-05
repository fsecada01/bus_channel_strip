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
└── HStack (Lunchbox Slots - 480px height, 1400px total width)
    ├── Slot 1: API5500 EQ (320px width)
    ├── Slot 2: ButterComp2 (320px width)
    ├── Slot 3: Pultec EQ (320px width)
    ├── Slot 4: Transformer (320px width)
    └── Slot 5: Punch (320px width)
```

**Total Size**: 1400x600 pixels (FIXED - needs to be responsive)

### Module Themes
| Module | Background Gradient | Accent Color |
|--------|-------------------|--------------|
| API5500 | `#2a3a4a → #364050` | `#40a0d0` (cyan) |
| ButterComp2 | `#2a2a2a → #322a28` | `#ff9640` (orange) |
| Pultec | `#3a3428 → #423828` | `#ffd700` (gold) |
| Transformer | `#2a2a2a → #362a28` | `#cc6633` (rust) |
| Punch | `#2a2a3a → #3a3050` | `#00a0ff` (electric blue) |

---

## Known Issues

### Issue 1: GUI Cut Off / Not Visible on Expansion
**Symptoms**: GUI is cut off, not visible when VST window is expanded

**Possible Causes**:
1. Fixed pixel dimensions (1400x600) don't adapt to window size
2. Missing `width(Stretch(1.0))` on containers
3. ViziaState size doesn't update on window resize
4. Parent containers use fixed Pixels instead of Stretch

**Investigation Steps**:
1. Check `default_state()` in `src/editor.rs` - uses fixed `(1400, 600)`
2. Verify container sizing - should use `Stretch(1.0)` for responsive
3. Check if vizia-plug supports dynamic resizing
4. Review vizia examples for resizable window patterns

### Issue 2: Module Reordering
**Current State**: Module order parameters exist (`module_order_1` through `module_order_6`)

**Question**: How is reordering done?
- Via dropdown selectors per slot?
- Via drag-and-drop?
- Via separate settings panel?

**Investigation Needed**:
1. Search codebase for module ordering UI
2. Check if drag-and-drop is implemented
3. Determine UX pattern for reordering

---

## Task Breakdown

### Task 1: Make GUI Resizable
**Priority**: High
**Complexity**: Medium
**Model**: Sonnet

**Subtasks**:
1. Change fixed dimensions to responsive units
2. Implement window resize handling
3. Test in multiple DAW environments
4. Ensure minimum size constraints

### Task 2: Verify Module Reordering UI
**Priority**: Medium
**Complexity**: Low
**Model**: Haiku

**Subtasks**:
1. Search for existing reordering UI
2. Document current implementation
3. Recommend improvements if needed

### Task 3: Update Documentation
**Priority**: Medium
**Complexity**: Low
**Model**: Haiku

**Subtasks**:
1. Update `GUI_DESIGN.md` with resizing info
2. Update `CLAUDE.md` with vizia agent info
3. Update `README.md` if needed
4. Update `PUNCH_MODULE_SPEC.md` status

---

## Claude Model Selection

| Task Type | Model | Reasoning |
|-----------|-------|-----------|
| **Architecture & Design** | Opus | Complex layout decisions, vizia architecture understanding |
| **Implementation** | Sonnet | Standard vizia component development |
| **Bug Investigation** | Sonnet | Debugging layout/rendering issues |
| **Documentation** | Haiku | Straightforward markdown updates |
| **Simple Fixes** | Haiku | CSS tweaks, small parameter changes |

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

- [ ] vizia agent specification complete
- [ ] GUI resizing implemented
- [ ] Module reordering verified
- [ ] Documentation updated
- [ ] Testing complete

---

## Master Agent Invocation

To spawn the vizia GUI specialist agent:

```
Task tool:
  - subagent_type: "general-purpose"
  - model: "sonnet" (or "opus" for complex architecture)
  - prompt: [Include context from this spec + specific task]
```

The agent should have access to:
- Full vizia documentation (via WebFetch)
- Current project files (editor.rs, components.rs, styles.rs)
- Ability to implement changes and rebuild
