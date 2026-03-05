# Bus Channel Strip - Project Memory

## Project Identity
- 6-module VST3/CLAP bus channel strip plugin
- Stack: Rust + NIH-Plug + vizia-plug + Airwindows C++ (FFI)
- Signal chain: API5500 EQ → ButterComp2 → Pultec EQ → Dynamic EQ → Transformer → Punch
- Current version: 0.2.0-prerelease; nightly Rust required for GUI build

## Build Commands (working as of March 2026)
- Fast check: `cargo check --features "api5500,buttercomp2,pultec,transformer,punch"`
- GUI bundle: `FORCE_SKIA_BINARIES_DOWNLOAD=1 LLVM_HOME="C:/Program Files/LLVM" LIBCLANG_PATH="C:/Program Files/LLVM/bin" cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "api5500,buttercomp2,pultec,transformer,punch,gui"`
- Do NOT set BINDGEN_EXTRA_CLANG_ARGS when building GUI
- Use `just` recipes: `just bundle`, `just check`, `just deploy`

## Key Resolved Issues
- biquad v0.5.0 API: `Type::PeakingEQ(gain_db)` (not `.set_gain()`)
- Skia builds from source on Windows x86_64 (no pre-built binaries for this arch)
- vizia-plug requires nightly Rust

## Audio Thread Rules (NEVER violate)
- No heap allocation in process()
- No mutexes or locking
- No I/O, system calls, or logging (outside nih_log! with feature gate)
- Use AtomicF32/AtomicBool for cross-thread parameter mirroring

## Parameter System
- ~75 parameters via #[derive(Params)]
- IDs are stable and baked into DAW sessions - never rename after shipping
- Always use Smoother for parameters feeding DSP math

## Available Skills
- `/dsp-audio-engineer` - DSP theory, psychoacoustics, algorithm design
- `/rust-dsp-dev` - NIH-plug patterns, FFI, lock-free, vizia-plug, shipping

## Orchestration
- Multi-agent mode activates at 2+ complexity criteria (see docs/SYSTEM_PROMPT.md)
- Coordinator=opus-4-6, Specialists=sonnet-4-6, QA=haiku-4-5
- Hard blocks: audio thread alloc, parameter ID rename, unsafe without comment

## Workflow
- `just check` - fast type-check (no GUI)
- `just qa` - fmt + lint + test gate before commits
- `just deploy` - bundle + install to system VST3 dir
- Use `rtk` prefix for all git/cargo commands for token efficiency
