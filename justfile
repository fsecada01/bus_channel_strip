# Bus Channel Strip - Development Workflow
# Install: cargo install just  (or via winget/choco)
# Usage:   just <recipe>
# Docs:    https://github.com/casey/just

set shell := ["cmd", "/c"]
set dotenv-load := true

# Feature sets
FEATURES      := "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq,gui"
CORE_FEATURES := "api5500,buttercomp2,pultec,transformer,punch,dynamic_eq"

# Plugin install paths (Windows) — backslashes required for CMD if/md/copy
VST3_DIR := "C:\\Program Files\\Common Files\\VST3"
CLAP_DIR := "C:\\Program Files\\Common Files\\CLAP"

# Exported so PowerShell install scripts inherit them as $env: variables
export BCS_VST3_SRC  := justfile_directory() + "\\target\\bundled\\Bus-Channel-Strip.vst3\\Contents\\x86_64-win\\Bus-Channel-Strip.vst3"
export BCS_VST3_DEST := VST3_DIR + "\\Bus-Channel-Strip.vst3\\Contents\\x86_64-win\\Bus-Channel-Strip.vst3"
export BCS_CLAP_SRC  := justfile_directory() + "\\target\\bundled\\Bus-Channel-Strip.clap"
export BCS_CLAP_DEST := CLAP_DIR + "\\Bus-Channel-Strip.clap"

# System prompt file (auto-included in CLAUDE.md via @ syntax)
SYSTEM_PROMPT := "docs/SYSTEM_PROMPT.md"

# Default: list available recipes
default:
    @just --list --unsorted

# ── Build ─────────────────────────────────────────────────────────────────────

# Fast type-check (no codegen) - use for rapid iteration
check:
    cargo check --features {{CORE_FEATURES}}

# Type-check with GUI features
check-gui:
    cargo +nightly check --features {{FEATURES}}

# Debug build (no GUI, fast)
build:
    cargo build --features {{CORE_FEATURES}}

# Debug build with full GUI
build-gui:
    cargo +nightly build --features {{FEATURES}}

# Release build
release:
    cargo +nightly build --release --features {{FEATURES}}

# ── Bundle (VST3 + CLAP) ──────────────────────────────────────────────────────

# Production bundle: VST3 + CLAP with full GUI (recommended)
bundle:
    set "LLVM_HOME=C:/Program Files/LLVM" && \
    set "LIBCLANG_PATH=C:/Program Files/LLVM/bin" && \
    cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features {{FEATURES}}

# Bundle without GUI (faster, no Skia dependency)
bundle-core:
    cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features {{CORE_FEATURES}}

# Bundle with debug symbols for profiling
bundle-profile:
    set "LLVM_HOME=C:/Program Files/LLVM" && \
    set "LIBCLANG_PATH=C:/Program Files/LLVM/bin" && \
    cargo +nightly run --package xtask -- bundle bus_channel_strip --profile profiling --features {{FEATURES}}

# ── Install ───────────────────────────────────────────────────────────────────

# Install VST3 to system plugin directory.
# Copy-Item uses -ErrorAction Stop so UAC/AV/locked-file failures surface as
# non-zero exit instead of silent success. Post-copy size check catches the
# Program-Files virtualization case where the copy "succeeds" but writes to
# VirtualStore instead of the real destination.
install-vst3:
    @powershell -NoProfile -File {{justfile_directory()}}\scripts\install-vst3.ps1

# Install CLAP to system plugin directory. Same safeguards as install-vst3.
install-clap:
    @powershell -NoProfile -File {{justfile_directory()}}\scripts\install-clap.ps1

# Install both formats
install: install-vst3 install-clap
    @echo Plugin installed. Rescan in your DAW.

# Bundle and install in one step
deploy: bundle install

# ── Quality Assurance ─────────────────────────────────────────────────────────

# Run unit tests
test:
    cargo test --features {{CORE_FEATURES}}

# Lint with Clippy - treats warnings as errors
lint:
    cargo clippy --all-targets --features {{CORE_FEATURES}} -- -D warnings

# Lint with leniency (warnings only)
lint-warn:
    cargo clippy --all-targets --features {{CORE_FEATURES}}

# Format code (nightly required for best formatting)
fmt:
    cargo +nightly fmt

# Check formatting without modifying
fmt-check:
    cargo +nightly fmt --check

# Full quality gate: format check + lint + test
qa: fmt-check lint test
    @echo All quality checks passed.

# ── Debug & Inspection ────────────────────────────────────────────────────────

# Show bundled artifact sizes
sizes:
    @if exist target\bundled ( dir target\bundled ) else ( echo No bundles found. Run 'just bundle' first. )

# List DSP module source files
modules:
    @dir src\*.rs

# Count parameters by type (requires findstr)
params:
    @echo === Parameter counts ===
    @echo FloatParam: && findstr /c:"FloatParam" src\lib.rs | find /c /v ""
    @echo BoolParam:  && findstr /c:"BoolParam"  src\lib.rs | find /c /v ""
    @echo IntParam:   && findstr /c:"IntParam"   src\lib.rs | find /c /v ""
    @echo EnumParam:  && findstr /c:"EnumParam"  src\lib.rs | find /c /v ""

# Show current build environment
env:
    @echo Rust toolchain:
    @rustup show active-toolchain
    @echo LLVM/Clang:
    @clang --version 2>nul || echo   not found
    @echo Ninja:
    @ninja --version 2>nul || echo   not found
    @echo Just:
    @just --version

# Show dependency tree for core features
deps:
    cargo tree --features {{CORE_FEATURES}}

# Watch src/ for changes and run check (requires cargo-watch)
watch:
    cargo watch -x "check --features {{CORE_FEATURES}}"

# ── CI / GitHub Actions ───────────────────────────────────────────────────────

# Trigger a manual CI build on the current branch
ci:
    gh workflow run main.yml --ref main
    @echo Triggered. Watch with: just ci-watch

# Watch live CI run status (polls until complete)
ci-watch:
    gh run watch $(gh run list --workflow=main.yml --limit=1 --json databaseId --jq '.[0].databaseId')

# Show recent CI run history
ci-log:
    rtk gh run list --limit 10

# ── Git Workflow ──────────────────────────────────────────────────────────────

# Show compact status
status:
    rtk git status

# Show recent commits
log:
    rtk git log --oneline -20

# Diff staged and unstaged changes
diff:
    rtk git diff

# ── Claude AI Sessions ────────────────────────────────────────────────────────
# Note: CLAUDE.md auto-loads and @-includes docs/SYSTEM_PROMPT.md, so
# system prompt context is always active in standard 'claude' sessions.

# Start interactive Claude Code session (standard)
claude *args="":
    claude --append-system-prompt-file docs/SYSTEM_PROMPT.md {{ args }}

# Start Claude Code with explicit system prompt append
claude-prompt:
    claude --append-system-prompt-file {{SYSTEM_PROMPT}}

# Start Claude Code in auto-approval mode (skips permission prompts)
# WARNING: Use only in trusted environments - allows automatic file edits
claude-auto *args="":
    claude --dangerously-skip-permissions --append-system-prompt-file docs/SYSTEM_PROMPT.md {{ args }}

# One-shot Claude query (non-interactive) with project context
ask PROMPT:
    claude -p "{{PROMPT}}" --append-system-prompt-file {{SYSTEM_PROMPT}}

# Review recent code changes with Claude
review:
    claude -p "Review the current git diff for correctness, audio thread safety, and Rust best practices. Focus on lock-free guarantees and DSP accuracy." \
    --append-system-prompt-file {{SYSTEM_PROMPT}}

# Ask Claude to analyze a specific source file
analyze FILE:
    claude -p "Analyze src/{{FILE}} for DSP correctness, Rust idioms, potential audio thread issues, and improvement opportunities." \
    --append-system-prompt-file {{SYSTEM_PROMPT}}

# ── Mix Advisor Service ───────────────────────────────────────────────────────

# Run the advisor in development mode (auto-reloads with cargo-watch if available)
# Requires: ANTHROPIC_API_KEY environment variable
advisor-dev:
    cargo run --package bcs-advisor

# Build advisor release binary
advisor-build:
    cargo build --release --package bcs-advisor

# Run the advisor release binary
advisor:
    target\release\bcs-advisor.exe

# Check advisor compiles (fast, no Claude calls)
advisor-check:
    cargo check --package bcs-advisor

# Run advisor with verbose logging
advisor-verbose:
    set "RUST_LOG=debug" && cargo run --package bcs-advisor

# ── Documentation ────────────────────────────────────────────────────────────

# Start local docs dev server (hot-reload)
docs-dev:
    cd site && npm run dev

# Build docs site for production
docs-build:
    cd site && npm install && npm run build

# Preview the production docs build locally
docs-preview:
    cd site && npm run preview

# ── Maintenance ───────────────────────────────────────────────────────────────

# Clean build artifacts (preserves registry cache)
clean:
    cargo clean

# Force recompile of this package only (deps stay cached) then deploy
rebuild:
    cargo clean -p bus_channel_strip
    just deploy

# Update all dependencies
update:
    cargo update

# Show outdated dependencies
outdated:
    cargo outdated 2>nul || echo Install cargo-outdated: cargo install cargo-outdated
