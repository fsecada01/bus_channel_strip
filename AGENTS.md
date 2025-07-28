# AI Agent Collaboration Notes

This document provides context for AI agents collaborating on this project, outlining the current issues and a plan for resolution.

## Current Task: Fix `iced` GUI Compilation Errors

The project fails to build when the `gui` feature is enabled due to a number of breaking changes in the `nih-plug-iced` dependency. The errors are located in `src/editor.rs` and `src/lib.rs`.

### Key Issues & Analysis

1.  **`IcedEditor` Trait Mismatch**: The implementation of the `IcedEditor` trait in `src/editor.rs` is outdated. The signatures for the `new`, `update`, and `style` methods do not match the versions in the current `nih-plug-iced` API.

2.  **Deprecated Styling System**: The custom styling logic in the `style` module at the bottom of `src/editor.rs` uses a deprecated `StyleSheet` trait. The new API appears to use a `Theme` enum returned from a `style` method on the `IcedEditor` trait itself.

3.  **Broken Parameter Update Logic**: The `update` function attempts to handle a `ParamMessage` by accessing fields (`param_id`, `normalized_value`) that no longer exist on the struct. The logic to find the parameter pointer and send the update to the `GuiContext` is also incorrect.

4.  **Rust Borrow-Checker Violations**: The `view` method in `src/editor.rs` creates multiple mutable borrows of `self` by calling helper methods (e.g., `self.api5500_module()`) inside the UI construction, which also mutably borrows `self.scrollable_state`. This is a fundamental ownership issue that needs to be refactored.

5.  **Incorrect `GuiContext` Creation**: In `src/lib.rs`, the `editor` function is passing the wrong type to `editor::create`. It needs to call `async_executor.create_gui_context()` to get the required `Arc<dyn GuiContext>`.

### Recommended Plan of Action

1.  **Fix `lib.rs`**: Correct the `editor` function to properly create the `GuiContext` by calling `async_executor.create_gui_context()`.

2.  **Refactor `editor.rs`**:
    *   **Correct `IcedEditor` Implementation**:
        *   Update the `new` function signature to match the trait.
        *   Update the `update` function signature and rewrite the parameter update logic to use the correct fields from `ParamMessage` and the correct `GuiContext` methods.
        *   Implement the `style` method on `IcedEditor` to return a `Theme`, and remove the old `style` module.
    *   **Resolve Borrow-Checker Errors**:
        *   Refactor the `view` method. The helper methods for building UI modules (`api5500_module`, etc.) should not take `&mut self`.
        *   Instead, the `view` method should call these helpers once to get the UI elements, and then construct the final view with them. This will prevent the multiple mutable borrow errors.

This comprehensive approach should resolve all the compilation errors and get the GUI working again.