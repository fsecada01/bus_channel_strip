# AGENTS

This document tracks the agents and their roles in the development of the bus channel strip plugin using NIH-Plug and Airwindows-based DSP modules.

## Core Agents

- **User**: Provides plugin vision, DSP design ideas, and selects target behaviors (e.g., analog modeling, soft knee compression).
- **Developer**: Implements DSP logic in Rust using NIH-Plug. Integrates Airwindows modules via C++ FFI. Manages GUI, parameter mapping, and audio-safe architecture.
- **CI**: Automates builds, tests, and bundling of the plugin (VST3, AU, etc.) using `cargo`, `build.rs`, and cross-platform validation.

## AI-Enhanced Agents

- **Claude / Gemini / LLM Agents**:  
  - Generate, refactor, or optimize DSP logic in Rust or C++
  - Follow the signal flow: `[EQ] → [Compressor] → [Pultec] → [Dynamic EQ] → [Console/Tape]`
  - Use or suggest shaping functions:
    - `sigmoid(x)` / `tanh(x)` for soft knees and saturation
    - `poly(x) + log(x)` for filter or tone control curves
    - `log2(x)`, `exp(x)` for perceptual/gain scaling
  - Can assist in creating or modifying:
    - `build.rs` for FFI compilation
    - FFI wrappers (`cpp/*.cpp`) for Airwindows modules
    - GUI layout using `egui`
    - Parameter bindings with `#[derive(Params)]`

## Development Standards

- All real-time audio processing must be **lock-free** and **allocation-free**.
- Parameters must be automation-safe and uniquely identified.
- Airwindows modules must be wrapped in FFI-safe C++ using `extern "C"` interface.
- Math shaping functions can be implemented in `src/shaping.rs` and reused across modules.
- Git submodules or dependency scripts may be used to pull external DSP libraries.

---

## 🖼️ GUI Design Guidance (for AI/Agents)

NIH-Plug supports custom GUIs via `egui`. For photorealistic hardware emulation:

- Use `egui::Image` or custom widgets to render bitmap knobs, panels, and meters.
- Group modules visually with consistent color-coding:
  - **EQ**: blue-gray background, cyan accents  
  - **Compressor**: slate or black, orange knobs  
  - **Pultec**: brass tones, gold highlights  
  - **Dynamic EQ**: steel blue, green accents  
  - **Console/Tape**: charcoal or oxide red tones

Agents may implement:
- Sprite-based knobs (rotary or stepped)
- Layered GUI with static panel backgrounds and interactive zones
- External GUI systems (e.g., `iced`, `wgpu`, `skia`) for full custom UIs

Ensure GUI interactions remain performant and audio-thread safe.

---

## CI/CD Pipeline Troubleshooting

**RESOLVED ISSUES:**

1. **Bundle Command (FIXED):**
   - ✅ Correct command: `cargo xtask bundle bus_channel_strip --release`
   - The workflow was attempting `cargo xtask bundle --release` which requires package specification

2. **Asset Upload Structure (FIXED):**
   - ✅ **CLAP**: `Bus-Channel-Strip.clap` is a file - can upload directly
   - ✅ **VST3**: `Bus-Channel-Strip.vst3` is a directory (standard VST3 bundle structure)
   - ✅ **Solution**: Create zip archive of VST3 directory for distribution

**Updated Workflow:**
- Bundle command now uses correct syntax
- VST3 gets zipped into `Bus-Channel-Strip-vst3.zip` before upload
- CLAP uploads directly as file
- Improved debugging output shows bundled structure

**Local Testing Commands:**
```bash
cargo xtask bundle bus_channel_strip --release
ls -la target/bundled/
file target/bundled/Bus-Channel-Strip.clap  # Shows: ELF shared object
file target/bundled/Bus-Channel-Strip.vst3  # Shows: directory
find target/bundled/Bus-Channel-Strip.vst3 -type f  # Shows actual .so file
```

_TODO: Add agent-specific script hooks or CI triggers (e.g. for updating Airwindows modules or verifying FFI integrity)._ 
