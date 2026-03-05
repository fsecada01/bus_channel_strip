# Bus Channel Strip - Development Workflow
# Install: cargo install just  (or via winget/choco)
# Usage:   just <recipe>
# Docs:    https://github.com/casey/just

set shell := ["cmd", "/c"]
set dotenv-load := true

# Feature sets
FEATURES      := "api5500,buttercomp2,pultec,transformer,punch,gui"
CORE_FEATURES := "api5500,buttercomp2,pultec,transformer,punch"

# Plugin install paths (WSL-style when running under bash on Windows)
VST3_DIR := "/c/Program Files/Common Files/VST3"
CLAP_DIR := "/c/Program Files/Common Files/CLAP"

# System prompt file (auto-included in CLAUDE.md via @ syntax)
SYSTEM_PROMPT := "docs/SYSTEM_PROMPT.md"

# Default: list available recipes
default:
    @just --list --unsorted

# ── Build ─────────────────────────────────────────────────────────────────────

# Fast type-check (no codegen) - use for rapid iteration
check:
    cargo check --features "{{CORE_FEATURES}}"

# Type-check with GUI features
check-gui:
    cargo +nightly check --features "{{FEATURES}}"

# Debug build (no GUI, fast)
build:
    cargo build --features "{{CORE_FEATURES}}"

# Debug build with full GUI
build-gui:
    cargo +nightly build --features "{{FEATURES}}"

# Release build
release:
    cargo +nightly build --release --features "{{FEATURES}}"

# ── Bundle (VST3 + CLAP) ──────────────────────────────────────────────────────

# Production bundle: VST3 + CLAP with full GUI (recommended)
bundle:
    FORCE_SKIA_BINARIES_DOWNLOAD=1 \
    LLVM_HOME="C:/Program Files/LLVM" \
    LIBCLANG_PATH="C:/Program Files/LLVM/bin" \
    cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "{{FEATURES}}"

# Bundle without GUI (faster, no Skia dependency)
bundle-core:
    cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features "{{CORE_FEATURES}}"

# Bundle with debug symbols for profiling
bundle-profile:
    FORCE_SKIA_BINARIES_DOWNLOAD=1 \
    LLVM_HOME="C:/Program Files/LLVM" \
    LIBCLANG_PATH="C:/Program Files/LLVM/bin" \
    cargo +nightly run --package xtask -- bundle bus_channel_strip --profile profiling --features "{{FEATURES}}"

# ── Install ───────────────────────────────────────────────────────────────────

# Install VST3 to system plugin directory
install-vst3:
    cp -r "target/bundled/Bus-Channel-Strip.vst3" "{{VST3_DIR}}/"
    @echo "Installed VST3 -> {{VST3_DIR}}/Bus-Channel-Strip.vst3"

# Install CLAP to system plugin directory
install-clap:
    cp -r "target/bundled/Bus-Channel-Strip.clap" "{{CLAP_DIR}}/" 2>/dev/null && \
    echo "Installed CLAP -> {{CLAP_DIR}}/Bus-Channel-Strip.clap" || \
    echo "CLAP bundle not found (may not have been built)"

# Install both formats
install: install-vst3 install-clap
    @echo "Plugin installed. Rescan in your DAW."

# Bundle and install in one step
deploy: bundle install

# ── Quality Assurance ─────────────────────────────────────────────────────────

# Run unit tests
test:
    cargo test --features "{{CORE_FEATURES}}"

# Lint with Clippy - treats warnings as errors
lint:
    cargo clippy --all-targets --features "{{CORE_FEATURES}}" -- -D warnings

# Lint with leniency (warnings only)
lint-warn:
    cargo clippy --all-targets --features "{{CORE_FEATURES}}"

# Format code (nightly required for best formatting)
fmt:
    cargo +nightly fmt

# Check formatting without modifying
fmt-check:
    cargo +nightly fmt --check

# Full quality gate: format check + lint + test
qa: fmt-check lint test
    @echo "All quality checks passed."

# ── Debug & Inspection ────────────────────────────────────────────────────────

# Show bundled artifact sizes
sizes:
    @ls -lh target/bundled/ 2>/dev/null || echo "No bundles found. Run 'just bundle' first."

# List DSP module source files
modules:
    @ls -la src/*.rs

# Count parameters by type
params:
    @echo "=== Parameter counts ===" && \
    echo "FloatParam:  $(grep -c 'FloatParam' src/lib.rs)" && \
    echo "BoolParam:   $(grep -c 'BoolParam'  src/lib.rs)" && \
    echo "IntParam:    $(grep -c 'IntParam'   src/lib.rs)" && \
    echo "EnumParam:   $(grep -c 'EnumParam'  src/lib.rs)"

# Show current build environment
env:
    @echo "Rust:   $(rustup show active-toolchain 2>/dev/null)" && \
    echo "LLVM:   $(clang --version 2>/dev/null | head -1 || echo 'not found')" && \
    echo "Ninja:  $(ninja --version 2>/dev/null | head -1 || echo 'not found')" && \
    echo "Just:   $(just --version 2>/dev/null)"

# Show dependency tree for core features
deps:
    cargo tree --features "{{CORE_FEATURES}}"

# Watch src/ for changes and run check (requires cargo-watch)
watch:
    cargo watch -x "check --features {{CORE_FEATURES}}"

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

# Start Claude Code with explicit system prompt append (if flag is supported)
claude-prompt:
    claude --append-system-prompt "$(cat {{SYSTEM_PROMPT}})"

# Start Claude Code in auto-approval mode (skips permission prompts)
# WARNING: Use only in trusted environments - allows automatic file edits
claude-auto *args="":
    claude --dangerously-skip-permissions --append-system-prompt-file docs/SYSTEM_PROMPT.md {{ args }}

# One-shot Claude query (non-interactive) with project context
ask PROMPT:
    claude -p "{{PROMPT}}" --append-system-prompt "$(cat {{SYSTEM_PROMPT}})"

# Review recent code changes with Claude
review:
    claude -p "Review the current git diff for correctness, audio thread safety, and Rust best practices. Focus on lock-free guarantees and DSP accuracy." \
    --append-system-prompt "$(cat {{SYSTEM_PROMPT}})"

# Ask Claude to analyze a specific source file
analyze FILE:
    claude -p "Analyze src/{{FILE}} for DSP correctness, Rust idioms, potential audio thread issues, and improvement opportunities." \
    --append-system-prompt "$(cat {{SYSTEM_PROMPT}})"

# ── Maintenance ───────────────────────────────────────────────────────────────

# Clean build artifacts (preserves registry cache)
clean:
    cargo clean

# Update all dependencies
update:
    cargo update

# Show outdated dependencies
outdated:
    cargo outdated 2>/dev/null || echo "Install cargo-outdated: cargo install cargo-outdated"
