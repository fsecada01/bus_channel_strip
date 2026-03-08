---
title: Contributing
description: How to build, develop, and contribute to Bus Channel Strip
---

Thank you for your interest in contributing to Bus Channel Strip. This guide covers everything you need to get a working development environment, run quality gates, and submit changes.

## Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust (nightly) | latest nightly | Required for vizia-plug GUI. Install via `rustup toolchain install nightly` |
| LLVM/Clang | 19+ | Required for Skia bindings on Windows. Download from [releases.llvm.org](https://releases.llvm.org/) |
| Node.js | 20+ | Required for the documentation site only |
| Git | any recent | Standard version control |
| Reaper | 7.x | Primary DAW for plugin testing. A free evaluation license is available |

## Clone and Build (Plugin)

```bash
git clone https://github.com/fsecada01/bus_channel_strip.git
cd bus_channel_strip
```

### Fast development check (no GUI, seconds)

```bash
cargo check --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq"
```

### Full build with GUI

```bash
cargo +nightly build --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
```

### Production bundle (VST3 + CLAP)

On Windows, set LLVM environment variables before bundling:

```bash
# Windows (cmd)
set LLVM_HOME=C:\Program Files\LLVM
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
cargo +nightly run --package xtask -- bundle bus_channel_strip --release ^
  --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
```

```bash
# Windows (bash/PowerShell)
LLVM_HOME="C:/Program Files/LLVM" \
LIBCLANG_PATH="C:/Program Files/LLVM/bin" \
cargo +nightly run --package xtask -- bundle bus_channel_strip --release \
  --features "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
```

Bundles are written to `target/bundled/`. Install them in your VST3/CLAP directories or use `just deploy`.

## Clone and Build (Documentation Site)

```bash
cd site
npm install
npm run dev     # hot-reload dev server on http://localhost:4321
npm run build   # production build to site/dist/
npm run preview # preview the production build locally
```

## Quality Gate

Run these before every pull request:

```bash
# Format (nightly rustfmt required for some format options)
cargo +nightly fmt

# Lint — must be warning-free
cargo clippy -- -D warnings

# Tests
cargo test
```

Or all at once via the justfile:

```bash
just qa
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `api5500` | yes | 5-band semi-parametric API5500-style EQ |
| `buttercomp2` | yes | Airwindows ButterComp2 compressor (C++ FFI) |
| `pultec` | yes | Pultec EQP-1A style EQ with tube saturation |
| `transformer` | yes | Transformer coloration module (4 vintage models) |
| `punch` | yes | Clipper + Transient Shaper with 8x oversampling |
| `dynamic_eq` | yes | 4-band dynamic EQ with sidechain masking analysis |
| `gui` | **no** | vizia-plug GUI. Requires nightly Rust + LLVM. Off by default to keep CI fast |

Feature flags are additive. You can build a subset of modules for faster iteration:

```bash
cargo build --features "api5500,pultec"
```

:::note
`gui` is intentionally excluded from default features. Including it in defaults would force Skia to compile on every CI target, including macOS Intel cross-compile where it fails due to SIMD header restrictions in Xcode's clang.
:::

## PR Workflow

### Branch naming

```
feat/short-description
fix/issue-number-short-description
docs/what-you-are-documenting
refactor/what-changed
```

### Commit style

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(punch): add cubic clip mode with adjustable knee
fix(dynamic_eq): prevent denormal accumulation in detector path
docs(site): add contributing guide
```

### PR checklist

Before opening a pull request, verify:

- [ ] `cargo +nightly fmt` — no formatting changes needed
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] No new allocations in `process()` or any function it calls
- [ ] No parameter `#[id = "..."]` values were changed
- [ ] All new `unsafe` blocks have a safety comment explaining the invariant
- [ ] If adding a new module: feature-gated with `#[cfg(feature = "...")]`

## Audio Thread Rules

:::danger[Audio Thread — Hard Constraints]
The following are **unconditional prohibitions** in `process()` and any function called from it. Violating these will cause audio dropouts, undefined behavior, or data races in the DAW.

**No heap allocations:**
```rust
// FORBIDDEN in process()
let v = Vec::new();          // allocates
let s = format!("{}", x);    // allocates
let b = Box::new(thing);     // allocates
```

**No locks:**
```rust
// FORBIDDEN in process()
let guard = mutex.lock();    // can block
```

**No panics:**
```rust
// FORBIDDEN in process()
value.unwrap();              // panics on None
slice[index];                // panics on out-of-bounds (use .get())
```

**No I/O:**
```rust
// FORBIDDEN in process()
println!("debug");           // syscall
std::fs::read("file");       // syscall
```

**No thread spawning** — all work completes within the process block.

**Correct cross-thread communication** uses atomics:
```rust
// OK: lock-free parameter passing
let value = self.params.gain.smoothed.next();
// OK: lock-free flag signaling
self.analysis_requested.store(true, Ordering::Relaxed);
```
:::

## Parameter ID Stability

:::caution[Never Change Parameter IDs]
Parameter IDs (`#[id = "..."]`) are **permanent identifiers** written into every DAW session and preset file that uses this plugin. Changing an ID — even just renaming it — will silently break every existing session.

Rules:
- Never change an existing `#[id = "..."]` value
- Never reuse a retired ID for a different parameter
- To rename a parameter's *display name*, change only the string passed to `FloatParam::new()`, not the `#[id]`
- To add a new parameter, give it a new unique ID that has never been used

If you must remove a parameter, keep its `#[id]` reserved in a comment so it is never accidentally reused.
:::
