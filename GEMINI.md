
## Code Changes - Logic Abstraction

To improve code organization and readability, the logic for each processing module has been abstracted into separate files within a new `src/dsp/` directory. This follows the principle of "separation of concerns."

### New File Structure:

```
/
├── src/
│   ├── lib.rs
│   └── dsp/
│       ├── mod.rs
│       ├── api5500.rs
│       ├── buttercomp2.rs
│       ├── pultec.rs
│       ├── dynamic_eq.rs
│       └── transformer.rs
...
```

### Summary of Changes:

1.  **Created a `dsp` module:** A new directory `src/dsp` was created to hold the logic for each processing module.
2.  **Abstracted Modules:** The logic for `Api5500`, `ButterComp2`, `PultecEQ`, `DynamicEQ`, and `TransformerModule` was moved into their own files within the `src/dsp/` directory. This makes the main `lib.rs` file much cleaner and easier to navigate.
3.  **Updated `lib.rs`:**
    *   Added `mod dsp;` to make the new module available.
    *   Updated `use` statements to import the modules from their new location.
    *   The `process` function now calls the methods on the module instances, but the core logic remains the same. The code is now more organized and modular.

This refactoring significantly improves the project's structure, making it more maintainable and scalable. Each module's logic is now self-contained, which simplifies future development and debugging.