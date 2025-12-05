# AI Agent Collaboration Notes

This document provides context for AI agents collaborating on this project, outlining the current issues and a plan for resolution.

## Current Task: Implement Modern `vizia` GUI Architecture

The project needs GUI implementation using vizia's Entity-Component-System (ECS) pattern with vizia-plug integration. The vizia-plug framework provides modern GUI capabilities with Skia rendering and CSS-like styling.

### Key Requirements & Analysis

1.  **Modern vizia Architecture**: Need to implement the Entity-Component-System (ECS) pattern with reactive state management and CSS-like styling.

2.  **NIH-Plug Integration**: Use `vizia-plug` with proper editor creation and parameter binding for seamless DAW integration.

3.  **Parameter System**: Implement proper parameter binding using vizia's reactive data system with `Lens` traits for parameter access and updates.

4.  **Module Layout**: Create 5 distinct module UIs (API5500 EQ, ButterComp2, Pultec EQ, Dynamic EQ, Transformer) with color-coded professional design.

5.  **State Management**: Use `IcedState` for editor state persistence and proper `EditorFlags` for initialization.

6.  **Performance**: Leverage vizia's ECS architecture and Skia rendering for efficient GUI performance with built-in caching.

### Recommended Implementation Plan

1.  **Update `src/lib.rs`**: 
    *   Update the `editor` function to use vizia-plug integration with proper parameter access
    *   Ensure vizia editor state is properly initialized and passed to the GUI

2.  **Rewrite `src/editor.rs`** using modern vizia patterns:
    *   **App Structure**: Create `BusChannelStripEditor` struct implementing vizia's app creation pattern
    *   **Event System**: Use vizia's event system for parameter updates and UI interactions  
    *   **Data Binding**: Implement proper data flow using vizia's `Lens` system for parameter changes
    *   **View Construction**: Create view hierarchy building the complete 5-module layout with vizia widgets
    *   **State Management**: Use vizia's reactive state system for GUI state persistence

3.  **Module Implementation**:
    *   Create individual module rendering functions for each DSP module
    *   Apply color-coded professional styling per GUI_DESIGN.md specifications
    *   Implement proper knob, button, and meter widgets
    *   Add bypass states and visual feedback

4.  **Parameter Integration**:
    *   Wire all ~75 plugin parameters through vizia's reactive data system
    *   Ensure real-time parameter updates and DAW automation compatibility
    *   Implement proper parameter formatting and display using vizia's data binding

5.  **Testing & Polish**:
    *   Test in DAW environment for parameter automation
    *   Verify module bypass states and visual feedback
    *   Optimize rendering performance

### Key Resources
- See `CLAUDE.md` and `GUI_DESIGN.md` for complete vizia architecture guidance
- Reference vizia documentation: https://vizia.dev/
- vizia-plug GitHub: https://github.com/vizia/vizia-plug
- vizia examples: https://github.com/vizia/vizia/tree/main/examples

This modern approach uses vizia's Entity-Component-System architecture with Skia rendering for professional audio plugin GUIs.