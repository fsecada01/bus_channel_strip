## Summary

<!-- What does this PR change and why? -->

## Modules Affected

<!-- Check all that apply -->
- [ ] API5500 EQ (`src/api5500.rs`)
- [ ] ButterComp2 (`src/buttercomp2.rs` / `cpp/`)
- [ ] Pultec EQ (`src/pultec.rs`)
- [ ] Dynamic EQ (`src/dynamic_eq.rs`)
- [ ] Transformer (`src/transformer.rs`)
- [ ] Punch (`src/punch.rs`)
- [ ] GUI / Editor (`src/editor.rs`, `src/components.rs`, `src/styles.rs`)
- [ ] Build system (`build.rs`, `xtask/`, `Cargo.toml`)
- [ ] CI/CD (`.github/workflows/`)

## Audio Thread Safety Checklist

All items must be confirmed before merge:

- [ ] No heap allocation in `process()` path (no `Vec`, `Box`, `String`, `format!`)
- [ ] No mutex or locking in `process()` path
- [ ] No `unwrap()` / `expect()` / unguarded indexing in `process()` path
- [ ] No I/O or system calls in `process()` path
- [ ] Cross-thread state uses `AtomicF32` / `AtomicBool` only

## Parameter Stability

- [ ] No `#[id = "..."]` values were renamed or removed (would break existing DAW sessions)
- [ ] If parameter IDs changed: migration notes included below

## DSP Correctness

- [ ] Filter coefficients computed outside process loop (in `initialize()` or on param change)
- [ ] Gain conversions happen at parameter boundaries (dB ↔ linear)
- [ ] Non-linear operations run at appropriate oversample ratio (min 4x)
- [ ] Denormal guard in place for stateful filters

## Quality Gate

```bash
just qa   # runs: fmt-check + lint + test
```

- [ ] `just fmt-check` passes
- [ ] `just lint` passes (no clippy warnings)
- [ ] `just test` passes
- [ ] `just bundle-core` builds successfully

## Testing Notes

<!-- How was this tested? DAW used, test signal, what you listened for -->

## AI Assistance

<!-- If Claude or another AI assisted with this PR, note which parts and which agents/skills were used -->
<!-- e.g. "Orchestrated via SYSTEM_PROMPT.md workflow: DSP Specialist designed filter coefficients, Rust Engineer implemented" -->
