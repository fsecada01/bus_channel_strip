# Architecture Decision Records

This directory captures significant architectural decisions made for Bus Channel Strip.

## Format

Each ADR follows the [Michael Nygard template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions):

- **Title**: Short noun phrase
- **Status**: Proposed | Accepted | Deprecated | Superseded by ADR-XXXX
- **Context**: The situation that forced a decision
- **Decision**: What was decided and the key rationale
- **Consequences**: What becomes easier, harder, or different as a result

## Numbering

ADRs are numbered sequentially (`0001`, `0002`, …). Numbers are never reused. Superseded ADRs keep their file and gain a "Superseded by" status line.

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](0001-dyneq-band-expand-collapse-ui.md) | DynEQ Band Controls: Expand/Collapse Layout | Accepted |
| [0002](0002-compressor-model-ui-switching.md) | Compressor Model UI: Binding-Based Control Switching | Accepted |
| [0003](0003-dyneq-band-control-grouping.md) | DynEQ Band Control Grouping: Essential vs Advanced | Accepted |
| [0004](0004-haas-psychoacoustic-widening.md) | Haas: M/S + Comb Widening, Not True Crosstalk Cancellation | Implemented |
| [0005](0005-punch-pre-clip-transient-shaping.md) | Punch: Pre-Clip Transient Shaping, Pure-Rust Clipper, 8x Default Oversampling | Implemented |
| [0006](0006-sheen-pinned-master-end-polish.md) | Sheen: Pinned Master-End Polish Coat, Default-On, Excluded from Auto-Gain | Implemented |
| [0007](0007-multi-fx-rack-native-drag-drop.md) | Multi-FX Rack: Slot-and-Library Model with Native vizia Drag-Drop | Implemented |
| [0008](0008-buttercomp2-ffi-wrapper.md) | ButterComp2: FFI Wrapper over Airwindows C++, Not a Rust Port | Implemented |
| [0009](0009-gui-framework-and-color-coding.md) | GUI Framework: vizia-plug, with Per-Module Color Coding | Implemented |
| [0010](0010-mix-advisor-reascript-architecture.md) | Mix Advisor: Rust HTTP Broker + ReaScript/ReaImGui Client, Studio-Profile-v1 Schema | Implemented |
| [0011](0011-tpt-svf-and-pultec-linear-phase.md) | TPT State-Variable Filter Core + Pultec Linear-Phase Mode | Implemented |

`resources/` holds supporting data files referenced by an ADR (currently: `studio-profiles.json` for ADR-0010) that don't belong inline in the decision record itself.
