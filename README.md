# Bus Channel Strip

**A professional, multi-module bus channel strip VST3/CLAP plugin for audio production.**

This plugin provides a versatile and high-quality audio processing solution, combining several classic hardware emulations into a single, easy-to-use interface. Built with Rust and the NIH-Plug framework, it offers a robust and efficient audio processing experience.

## Features

- **Five Unique Modules**: API5500 EQ, ButterComp2, Pultec EQ, Dynamic EQ, and Transformer.
- **Customizable Signal Flow**: Reorder modules to create your ideal processing chain.
- **Hardware-Inspired GUI**: A visually intuitive interface with color-coded modules for a professional workflow.
- **Cross-Platform**: Builds for Windows, macOS, and Linux.

## Modules

- **API5500 EQ**: A versatile 5-band equalizer for precise tonal shaping.
- **ButterComp2**: A smooth, transparent compressor for dynamic control.
- **Pultec EQ**: A classic passive EQ for broad, musical equalization.
- **Dynamic EQ**: A 4-band dynamic equalizer for frequency-conscious dynamics processing.
- **Transformer**: A saturation module for adding warmth, color, and character.

## Building the Plugin

To build the plugin, you will need to have the Rust toolchain installed. You can install it from [rustup.rs](https://rustup.rs).

Once you have the Rust toolchain installed, you can build the plugin with the following command:

```bash
cargo xtask bundle bus_channel_strip --release
```

This will create a VST3 and CLAP plugin in the `target/release` directory.

## Pre-commit Hooks

This project uses `pre-commit` to enforce code formatting and linting. To use it, you will need to have `pre-commit` installed. You can install it with `pip`:

```bash
pip install pre-commit
```

Once you have `pre-commit` installed, you can install the git hooks with the following command:

```bash
pre-commit install
```

To re-run formatting against all files:

```bash
pre-commit run fmt --all-files
```

## Note

The `clippy` hook is currently disabled due to upstream dependency resolution issues in the `backtrace` crate; please run `cargo clippy --all-targets --all-features` manually or configure it in your CI pipeline.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

## License

This project is licensed under the GPL-3.0-or-later license. See the [LICENSE](LICENSE) file for more details.
