# 🎨 Bus Channel Strip GUI Design

## **Professional GUI Implementation Complete!**

This document describes the comprehensive GUI design for our professional bus channel strip plugin. The implementation provides a hardware-inspired interface with color-coded modules and intuitive workflow.

## **🖼️ GUI Architecture**

### **Layout Overview**
- **Default Size**: 1800x650 pixels (responsive, adapts to DAW window)
- **Minimum Size**: 1680x620 pixels (ensures all modules visible)
- **Horizontal Strip Layout**: 5 modules side-by-side (320px each)
- **Color-Coded Modules**: Based on AGENTS.md guidelines
- **Master Section**: Header area with gain controls
- **Responsive**: Uses vizia Stretch(1.0) units to adapt to window resizing

### **Signal Flow Visualization**
```
[API5500 EQ] → [ButterComp2] → [Pultec EQ] → [Dynamic EQ] → [Transformer] → [PUNCH]
```

### **Module Reordering**
- **Backend**: Parameters `module_order_1` through `module_order_6` control signal flow
- **Current UI**: Not yet implemented - modules appear in fixed positions
- **TODO**: Add dropdown selectors above each module slot to change module order
- **Workaround**: Use DAW automation to change module order parameters

## **🎛️ Module Layout & Colors**

### **1. API5500 EQ Module (180px wide)**
- **Colors**: Blue-gray background (#3C5064), Cyan accents (#00C8FF)
- **Layout**: 
  - Title bar with bypass button
  - LF/HF shelving controls (large knobs)
  - LMF/MF/HMF parametric bands (small knobs)
- **Controls**: 15 parameters total
  - LF: Frequency, Gain
  - LMF/MF/HMF: Frequency, Gain, Q (each)
  - HF: Frequency, Gain
  - Global bypass

### **2. ButterComp2 Module (140px wide)**
- **Colors**: Slate/black background (#282828), Orange accents (#FF8C00)
- **Layout**:
  - Title bar with bypass button
  - 3 large knobs vertically stacked
  - Gain reduction meter
- **Controls**: 4 parameters
  - Compress, Output, Mix
  - Bypass switch
  - Visual gain reduction feedback

### **3. Pultec EQ Module (160px wide)**
- **Colors**: Brass background (#786450), Gold accents (#FFD700)
- **Layout**:
  - Title bar with bypass button
  - Low frequency section (boost/atten)
  - High frequency section (boost/bandwidth/atten)
  - Tube drive control
- **Controls**: 10 parameters
  - LF: Boost Freq, Boost Gain, Attenuation
  - HF: Boost Freq, Boost Gain, Bandwidth, Cut Freq, Cut Gain
  - Tube Drive, Bypass

### **4. Dynamic EQ Module (200px wide)**
- **Colors**: Steel blue background (#465A78), Green accents (#00FF64)
- **Layout**:
  - Title bar with bypass button
  - 4 bands in compact vertical columns
  - Per-band enable buttons
- **Controls**: 33 parameters (8 per band + bypass)
  - Per Band: Frequency, Threshold, Ratio, Attack, Release, Gain, Q, Enable
  - Global bypass

### **5. Transformer Module (320px wide)**
- **Colors**: Charcoal background (#3C2D2D), Oxide red accents (#C8503C)
- **Layout**:
  - Title bar with bypass button
  - Model selector dropdown
  - Input stage controls
  - Output stage controls
  - Frequency response controls
  - Compression control
- **Controls**: 9 parameters
  - Model (4 types), Input Drive/Saturation
  - Output Drive/Saturation, Low/High Response
  - Compression, Bypass

### **6. Punch Module (320px wide)** ✨ NEW
- **Colors**: Deep purple/magenta background (#3A3050), Electric blue accents (#00A0FF)
- **Layout**:
  - Title bar with bypass button
  - CLIPPER section: Threshold, Softness
  - TRANSIENTS section: Attack, Sustain, Sensitivity
  - OUTPUT section: Input Gain, Output Gain, Mix
- **Controls**: 14 parameters
  - Clip Threshold, Clip Mode, Softness, Oversampling
  - Attack, Sustain, Attack Time, Release Time, Sensitivity
  - Input Gain, Output Gain, Mix
  - Bypass
- **Default State**: BYPASSED (user must enable)

## **🎚️ Control Elements**

### **Knob Design**
- **Sizes**: 50px (large), 35px (medium), 25px (small), 20px (mini)
- **Style**: Dark base with colored indicator line
- **Range**: 270° rotation (-135° to +135°)
- **Feedback**: Value display on hover

### **Button Design**
- **Bypass Buttons**: 30x20px, "ON" when active
- **Enable Buttons**: 20x15px, colored when active
- **Model Selector**: 70x20px dropdown-style

### **Meter Design**
- **Gain Reduction**: 15x80px vertical bars
- **Colors**: Match module accent colors
- **Range**: 0dB to -20dB visual range

## **🎨 Color Scheme (AGENTS.md Compliant)**

```css
/* Module Colors */
--eq-bg: #3C5064;              /* API5500 EQ background */
--eq-accent: #00C8FF;          /* API5500 EQ accents (cyan) */

--comp-bg: #282828;            /* ButterComp2 background */
--comp-accent: #FF8C00;        /* ButterComp2 accents (orange) */

--pultec-bg: #786450;          /* Pultec EQ background */
--pultec-accent: #FFD700;      /* Pultec EQ accents (gold) */

--dyneq-bg: #465A78;           /* Dynamic EQ background */
--dyneq-accent: #00FF64;       /* Dynamic EQ accents (green) */

--transformer-bg: #3C2D2D;     /* Transformer background */
--transformer-accent: #C8503C; /* Transformer accents (rust/oxide) */

--punch-bg: #3A3050;           /* Punch background (deep purple) */
--punch-accent: #00A0FF;       /* Punch accents (electric blue) */

/* Global Colors */
--main-bg: #191920;         /* Main background */
--text-primary: #FFFFFF;    /* Primary text */
--text-secondary: #CCCCCC;  /* Secondary text */
```

## **⚡ Interactive Features**

### **Real-time Feedback**
- **Gain Reduction Meters**: Live compression visualization
- **Parameter Value Display**: Hover tooltips with units
- **Bypass State Indicators**: Visual feedback for bypassed modules

### **Signal Flow Indicator**
- **Bottom Panel**: Horizontal module chain visualization
- **Active Modules**: Highlighted in accent colors
- **Bypassed Modules**: Grayed out
- **Flow Arrows**: Visual connection between modules

## **🔧 Technical Implementation**

### **Framework**: NIH-plug + vizia
**Current Status**: GUI implemented with vizia-plug framework. Modern Entity-Component-System architecture with Skia rendering.

### **vizia Architecture Pattern**
Following vizia's Entity-Component-System pattern with reactive state management:

```rust
// Core vizia application structure
pub struct BusChannelStripEditor {
    params: Arc<BusChannelStripParams>,
    // GUI state managed by vizia's ECS
}

// vizia app creation
pub fn create_vizia_editor(
    params: Arc<BusChannelStripParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    vizia_plug::create_vizia_editor(
        editor_state,
        ViziaEditorSize::default(),
        move |cx| {
            // Build GUI hierarchy
            BusChannelStripEditor::new(params.clone()).build(cx);
        },
    )
}

// Data binding with Lens traits
#[derive(Lens)]
struct EditorData {
    params: Arc<BusChannelStripParams>,
}

// Module rendering functions using vizia widgets
fn build_api5500_module(cx: &mut Context, params: Arc<BusChannelStripParams>)
fn build_buttercomp_module(cx: &mut Context, params: Arc<BusChannelStripParams>)
fn build_pultec_module(cx: &mut Context, params: Arc<BusChannelStripParams>)
fn build_dynamic_eq_module(cx: &mut Context, params: Arc<BusChannelStripParams>)
fn build_transformer_module(cx: &mut Context, params: Arc<BusChannelStripParams>)
```

### **Key vizia Documentation Resources**
- **vizia-plug GitHub**: https://github.com/vizia/vizia-plug
- **vizia book**: https://vizia.dev/
- **vizia examples**: https://github.com/vizia/vizia/tree/main/examples
- **vizia API docs**: https://docs.rs/vizia/
- **Skia graphics library**: https://skia.org/
- **vizia architecture guide**: Entity-Component-System with reactive data binding

### **Parameter Binding with vizia**
- **Real-time Updates**: Use vizia's reactive data system for thread-safe parameter updates
- **Parameter Access**: Use `Lens` traits for reading and updating parameter values
- **Automation Ready**: Full DAW automation support via vizia-plug integration
- **Event Handling**: Parameter changes flow through vizia's event system
- **Preset Management**: State save/restore capability

```rust
// Parameter update in vizia
impl View for ParameterKnob {
    fn element(&self) -> Option<&'static str> { Some("knob") }
    
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_event, _| match param_event {
            ParamEvent::ValueChanged(value) => {
                // Update parameter via vizia-plug
                self.param_ptr.set_normalized_value(cx, *value);
            }
        });
    }
}
```

### **Performance Optimizations with vizia**
- **Efficient Rendering**: vizia's ECS system with Skia rendering minimizes redraws
- **Entity Caching**: Use vizia's entity-based caching for static elements
- **Thread Safety**: vizia-plug handles audio-thread safe parameter access
- **Update Optimization**: Only update GUI when data actually changes via reactive system
- **Layout Optimization**: Use vizia's flexbox-inspired layout system for efficient positioning

## **📱 Responsive Design**

### **Scalability**
- **Fixed Layout**: Professional hardware aesthetic
- **DPI Awareness**: High-resolution display support
- **Minimum Size**: 1000x600 (maintains readability)

### **Accessibility**
- **High Contrast**: Clear visual hierarchy
- **Color Blind Friendly**: Shape and position cues
- **Keyboard Navigation**: Full keyboard accessibility

## **🚀 Build Requirements**

### **Dependencies**
```toml
[dependencies]
nih_plug = { git = "https://github.com/robbert-vdh/nih-plug.git" }
vizia_plug = { git = "https://github.com/vizia/vizia-plug.git", optional = true }
skia-safe = { version = "0.84", features = ["gl", "textlayout"], optional = true }
atomic_float = { version = "0.1", optional = true }
# Additional DSP dependencies
biquad = "0.5.0"
fundsp = "0.20.0"
augmented-dsp-filters = "2.5.0"
idsp = "0.18.0"
realfft = "3.5.0"

[features]
gui = ["vizia_plug", "atomic_float", "skia-safe"]
```

### **System Requirements**
- **Linux**: `pkg-config`, `libasound2-dev`, `libgl1-mesa-dev`, `libx11-dev`
- **Windows**: Visual Studio Build Tools
- **macOS**: Xcode Command Line Tools

### **Compilation**
```bash
# Fast type-check (no GUI, fastest feedback)
just check

# Debug build with GUI
just build-gui

# Production bundle: VST3 + CLAP with full GUI (Windows)
just bundle
# Equivalent manual command:
# FORCE_SKIA_BINARIES_DOWNLOAD=1 \
# LLVM_HOME="C:/Program Files/LLVM" \
# LIBCLANG_PATH="C:/Program Files/LLVM/bin" \
# cargo +nightly run --package xtask -- bundle bus_channel_strip --release \
#   --features "api5500,buttercomp2,pultec,transformer,punch,gui"

# Bundle without GUI (faster, no Skia)
just bundle-core

# Bundle + install to system VST3 dir
just deploy

# Install pre-commit hooks
pre-commit install

# Format code (nightly required)
just fmt
# OR: cargo +nightly fmt
```

**Important**: Do NOT use `cargo xtask bundle` directly — use `just bundle` or the explicit `cargo +nightly run --package xtask --` form. Do NOT set `BINDGEN_EXTRA_CLANG_ARGS` when building with GUI (breaks Skia).

## **Future Enhancements**

### **Phase 2 Features**
- **Module Reordering UI**: Dropdown selectors per slot (backend `module_order_*` params exist; UI pending)
- **Spectrum Analyzer**: Real-time frequency display per EQ module
- **Preset Browser**: Visual preset management
- **Skin Support**: Multiple visual themes

### **Advanced Features**
- **MIDI Learn**: Parameter automation mapping
- **A/B Compare**: Settings comparison

## **Status**

✅ vizia-plug integration complete (ECS, Skia rendering)
✅ All 6 module UIs implemented
✅ Responsive layout (1800x650 default, 1680x620 minimum)
✅ All parameters bound to vizia Lens system
✅ Plugin passes DAW testing (Reaper) — "sounds great!"
🔧 Module reordering GUI not yet implemented (DAW automation workaround available)

---

*This GUI design creates a professional, hardware-inspired interface that matches the quality of our world-class DSP implementation.*