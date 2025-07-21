# NIH-plug Bus Mixstrip Plugin

This repository serves as a starting point for building a VST3 bus mixstrip plugin using [NIH-plug](https://github.com/RustAudio/nih-plug).

## Prerequisites

- Rust (edition 2021 or later): install via https://rustup.rs
- VST3 SDK (for plugin hosting or if you want to build against the SDK directly)

## Getting Started

1. Install [Cookiecutter](https://github.com/cookiecutter/cookiecutter) if you don't already have it:
   ```bash
   pip install cookiecutter
   ```
2. Scaffold a new plugin project using the NIH-Plug template:
   ```bash
   cookiecutter https://github.com/robbert-vdh/nih-plug-template.git
   ```
3. Change into your new plugin directory (e.g., `my-plugin-name`) and start the minimal example:
   ```bash
   cd <your-plugin-directory>
   cargo run --example minimal
   ```
4. Implement your mixstrip DSP and GUI in `src/lib.rs` following the template structure.

## Project Layout

- `Cargo.toml`: Plugin metadata and dependencies (nih-plug, vst3-sys, etc.).
- `src/lib.rs`: Core DSP and GUI code of your plugin.
- `.gitignore`: Ignore build artifacts and temporary files.

## Building and Testing

```bash
# Build release VST3 plugin
cargo build --release

# The VST3 binary will be available in target/release/
``` 

## Next Steps

1. Update plugin metadata in `Cargo.toml` (name, vendor, unique ID).
2. Define mixstrip parameters, state serialization, and GUI layout.
3. Iterate: build, load into a host, and verify functionality.

---

## Plugin Building

After installing Rust (https://rustup.rs), you can compile the plugin as follows:

```shell
cargo xtask bundle bus_channel_strip --release
```
