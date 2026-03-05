# AI Agent Collaboration Notes

This document provides context for AI agents collaborating on this project. It reflects the **current state** of the codebase and serves as a quick-reference for multi-agent orchestration.

---

## Current Project Status

All 6 core modules are implemented, tested, and functional. The plugin builds and ships as both VST3 and CLAP via CI/CD.

| Module | File | Status |
|--------|------|--------|
| API5500 EQ | `src/api5500.rs` | Complete |
| ButterComp2 | `src/buttercomp2.rs` + `cpp/buttercomp2.cpp` | Complete (FFI) |
| Pultec EQ | `src/pultec.rs` | Complete |
| Dynamic EQ | `src/dynamic_eq.rs` | Complete (optional feature) |
| Transformer | `src/transformer.rs` | Complete |
| Punch | `src/punch.rs` | Complete (clipper + transient shaper, 8x OS) |

**GUI**: vizia-plug (ECS, Skia rendering) — implemented, 1800x650 default size, responsive.
**Parameters**: ~75 automation parameters via `#[derive(Params)]`.
**Build**: Nightly Rust required for GUI; `just bundle` for production builds.

---

## Multi-Agent Orchestration

The full orchestration protocol — including complexity criteria, agent roster, 7-step workflow, and escalation rules — is defined in `docs/SYSTEM_PROMPT.md`. All agents should read that document as their primary context.

### Agent Roster (Quick Reference)

| Role | Model | Skill | When to Use |
|------|-------|-------|-------------|
| Coordinator | `claude-opus-4-6` | — | Complex task decomposition, cross-domain conflicts, architectural decisions |
| DSP Specialist | `claude-sonnet-4-6` | `/dsp-audio-engineer` | Algorithm design, filter math, signal chain, psychoacoustics |
| Rust Engineer | `claude-sonnet-4-6` | `/rust-dsp-dev` | Implementation, FFI safety, NIH-plug idioms, lock-free patterns |
| QA Verifier | `claude-haiku-4-5-20251001` | — | Format/clippy gate, parameter ID audit, audio thread safety sweep |

### Orchestration Trigger

Activate multi-agent mode when 2+ of these criteria apply:
1. Modifies 3+ source files
2. Spans multiple domains (DSP + Rust + GUI)
3. Architectural decision (parameter IDs, module order, FFI interface)
4. New DSP module from scratch
5. Debugging unknown audio artifact
6. >150 lines of new/rewritten code
7. Plugin API stability impact

---

## Known Outstanding Work

| Item | Priority | Notes |
|------|----------|-------|
| Module reorder GUI | Medium | Backend `module_order_*` params exist; GUI dropdowns not yet implemented |
| Phase 5: Optimization | Low | CPU profiling, SIMD for oversampling, A/B vs reference plugins |
| Preset system | Low | No factory presets yet |
| Dynamic EQ feature flag | Low | Implemented but disabled by default |

---

## Key Architectural Invariants

These must be respected by all agents at all times:

1. **No allocations in `process()`** — no `Vec`, `Box`, `String`, `format!()`
2. **No locking in `process()`** — no `Mutex`, `RwLock`
3. **Parameter IDs are stable** — `#[id = "..."]` values baked into DAW sessions; never rename
4. **All `unsafe` blocks require a safety comment** — no exceptions
5. **Nightly Rust for GUI** — `cargo +nightly` when building with `gui` feature
6. **Do not set `BINDGEN_EXTRA_CLANG_ARGS`** when building GUI (breaks Skia)

---

## Key Resources

- Orchestration workflow: `docs/SYSTEM_PROMPT.md`
- GUI design spec: `docs/GUI_DESIGN.md`
- Punch module spec: `docs/PUNCH_MODULE_SPEC.md`
- ButterComp2 algorithm: `docs/buttercomp2_analysis.md`
- Clipping research: `docs/CLIPPING_INSIGHTS.md`
- vizia-plug: https://github.com/vizia/vizia-plug
- vizia docs: https://vizia.dev/
