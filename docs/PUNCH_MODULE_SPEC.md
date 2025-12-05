# Punch Module Specification

## Overview

A combined **Clipper + Transient Shaper** module designed to achieve louder mixes while preserving perceived energy and punch. This module addresses the common problem where clipping alone results in flat, lifeless mixes.

### Design Philosophy

> "Bring your transients back after clipping. Use a parallel send with a transient shaper, or an expansion tool to bring your clipped transients back. The transients will be more even than previously as they've been sliced."

---

## Signal Flow Architecture

```
                    ┌─────────────────────────────────┐
                    │         PUNCH MODULE            │
                    ├─────────────────────────────────┤
                    │                                 │
[Input] ──→ [Gain] ─┼──┬──→ [Clipper] ──→ [Mix] ──┬──┼──→ [Output]
                    │  │         ↓          ↑     │  │
                    │  │    [Oversampling]  │     │  │
                    │  │                    │     │  │
                    │  └──→ [Transient] ────┘     │  │
                    │       Detector/Shaper       │  │
                    │       (parallel blend)      │  │
                    │                             │  │
                    └─────────────────────────────────┘
```

---

## Parameters

### Clipper Section

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `clip_threshold` | -12dB to 0dB | -1dB | Ceiling where clipping begins |
| `clip_mode` | Hard / Soft / Cubic | Hard | Clipping algorithm |
| `softness` | 0% - 100% | 0% | Soft clip knee (0 = pure hard clip) |
| `oversampling` | 1x / 4x / 8x / 16x | 8x | Anti-aliasing quality |

### Transient Shaper Section

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `attack` | -100% to +100% | +20% | Transient attack boost/cut |
| `sustain` | -100% to +100% | 0% | Body/sustain adjustment |
| `attack_time` | 0.1ms - 30ms | 5ms | Transient detection window |
| `release_time` | 10ms - 500ms | 100ms | Envelope release |
| `sensitivity` | 0% - 100% | 50% | Transient detection threshold |

### Global Controls

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `input_gain` | -12dB to +12dB | 0dB | Drive into clipper |
| `output_gain` | -12dB to +12dB | 0dB | Makeup gain |
| `mix` | 0% - 100% | 100% | Dry/wet blend |
| `bypass` | On/Off | Off | Module bypass |

---

## Psychoacoustic Principles

### Why Clipping Alone Sounds Flat

1. **Transient Masking**: Our ears use transients to perceive "punch" and "energy"
2. **Loudness vs Energy**: RMS loudness increases but perceived energy decreases
3. **Temporal Envelope**: The attack portion of sounds defines their character

### Transient Perception Science (Research-Based)

Based on research from [Ira Hirsh's temporal perception studies](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1363770/) and [temporal integration research](https://pmc.ncbi.nlm.nih.gov/articles/PMC11463558/):

#### Critical Time Windows

| Time Range | Perceptual Function |
|------------|---------------------|
| **1-20ms** | Phase perception, transient onset detection |
| **17-20ms** | "Magic threshold" - minimum interval for auditory temporal order |
| **20-100ms** | Auditory pattern emergence, transient character |
| **100-200ms** | Separate auditory events, post-masking effects |
| **200-500ms** | Temporal integration for loudness perception |

#### Frequency-Dependent Integration

| Frequency | Integration Time Constant |
|-----------|---------------------------|
| 125 Hz | ~160ms |
| 1000 Hz | ~83ms |
| 4000 Hz | ~52ms |

**Key Insight**: Lower frequencies require longer integration times. This means:
- Bass transients need longer detection windows
- High-frequency transients are perceived more quickly
- Multi-band transient detection may be beneficial

#### Loudness vs Duration

> "Sound signals of longer duration are perceived as louder than signals of shorter duration" - [Fastl and Zwicker (2007)](https://www.diva-portal.org/smash/get/diva2:1433470/FULLTEXT01.pdf)

This explains why clipping (shortening transient peaks) reduces perceived loudness even when RMS increases.

### Restoration Strategy

1. **Detect** transients using differential envelope following (fast - slow envelope)
2. **Target** the 5-30ms window for attack enhancement
3. **Preserve** the 20-100ms range for perceived "punch"
4. **Blend** carefully - transient enhancement above +6dB often sounds unnatural

---

## Engineering Considerations

### Clipping Algorithms

#### Hard Clip
- Mathematically: `y = clamp(x, -threshold, threshold)`
- Introduces odd harmonics (3rd, 5th, 7th...)
- Most transparent for small amounts (<3dB of gain reduction)
- Aliasing risk without oversampling

#### Soft Clip (Polynomial/Cubic)
- Smoother knee transition
- Reduced high-frequency harmonic content
- More "analog" character
- Better for larger amounts of clipping

#### Tanh Saturation
- Natural compression curve approaching ±1.0 asymptotically
- Emulates tube/tape saturation
- Warmer tonal character
- Most forgiving for aggressive settings

### Oversampling Requirements

| Factor | Aliasing Rejection | CPU Cost | Use Case |
|--------|-------------------|----------|----------|
| 1x | Poor | Minimal | Testing only |
| 4x | Good | Low | Real-time mixing |
| 8x | Very Good | Medium | Recommended default |
| 16x | Excellent | High | Mastering |

### Transient Detection Methods

1. **Differential Envelope**: Fast envelope - Slow envelope = Transient
2. **Derivative-based**: Rate of amplitude change (d/dt of signal)
3. **Crest Factor**: Peak / RMS ratio over short window (~10ms)

---

## Airwindows Research

### Recommended Modules for Punch Implementation

Based on comprehensive research of the [Airwindows repository](https://github.com/airwindows/airwindows):

#### Primary Clipping Modules

| Module | Function | Best For | Key Characteristics |
|--------|----------|----------|---------------------|
| **[ClipOnly2](https://www.airwindows.com/cliponly2/)** | Transparent hard clip | Safety limiting | Interpolates between samples to soften edges; works up to 700kHz; pure bypass until -0.2dB |
| **[ADClip8](https://www.airwindows.com/adclip8/)** | Multi-stage saturation | Mastering | 8 modes from Normal to Apotheosis (6 stages); handles intersample peaks mechanically |
| **[OneCornerClip](https://www.airwindows.com/onecornerclip-vst/)** | Full-bandwidth clip | Aggressive processing | Asymmetric curve entry/exit; preserves frequency content |
| **[ClipSoftly](https://www.airwindows.com/clipsoftly/)** | Extreme soft clip | Fatness/warmth | Reshapes all samples (not just peaks); "tubey" character |

#### Transient Shaping Modules

| Module | Function | Best For | Key Characteristics |
|--------|----------|----------|---------------------|
| **[Surge](https://www.airwindows.com/surge/)** | Transient emphasis | Punch enhancement | "Attacks pop way out"; single threshold control; funky/bouncy dynamics |
| **[Pop2](https://www.airwindows.com/pop2/)** | Attack "splat" | Drum punch | Compression + ClipOnly2 output; distorted attack character; separate attack/decay |
| **[Point](https://www.airwindows.com/point-vst/)** | Transient designer | Precise shaping | Super-fast reaction; Point + Reaction controls |
| **[Pressure6](https://www.airwindows.com/pressure6/)** | Springy dynamics | Attack clarity | Makes attacks "pounce forward" |

#### Supporting Modules

| Module | Function | Potential Use |
|--------|----------|---------------|
| **Slew/SlewOnly** | Slew rate limiting | Transient softening |
| **GoldenSlew** | Slew + filtering | High-frequency control |
| **Creature** | Soft slew saturation | Dynamics-dependent saturation |
| **Recurve** | Fast transient cutter | Taming excessive peaks |

### Recommended Implementation Approaches

#### Option A: Transparent Punch (Recommended for Bus/Master)
```
[Surge] → [ClipOnly2]
```
- **Surge**: Enhances transient attacks with fluid compression
- **ClipOnly2**: Transparent safety clipping without coloration
- **Character**: Clean, modern, punchy

#### Option B: Colored Punch (Character Processing)
```
[Pop2] → [ClipSoftly]
```
- **Pop2**: Attack "splat" with built-in clipping
- **ClipSoftly**: Warm, fat soft saturation
- **Character**: Vintage, warm, aggressive

#### Option C: Multi-Stage Mastering
```
[Surge] → [ADClip8 (Afterburner mode)]
```
- **Surge**: Transient emphasis
- **ADClip8**: Controlled multi-stage saturation with intersample peak handling
- **Character**: Professional, controlled loudness

#### Option D: Custom Implementation (Most Flexible)
Port the algorithms rather than FFI wrapping:
- Use **ClipOnly2** algorithm for transparent clipping
- Implement differential envelope transient detector
- Add **Surge**-style compression for transient emphasis

### FFI Integration Notes

**Existing Infrastructure**: The project already has FFI wrappers for ButterComp2 in `cpp/` directory.

**Recommended Approach**:
1. **Phase 1**: Implement clipper in pure Rust (simpler algorithms)
2. **Phase 2**: Port transient detection from Surge/Pop2
3. **Phase 3**: If needed, add Airwindows FFI for advanced modules

**Why Pure Rust First**:
- ClipOnly2 algorithm is relatively simple (~100 lines)
- Transient detection is well-documented mathematically
- Avoids additional FFI complexity
- Better cross-platform compatibility

---

## GUI Design

### Color Scheme
- **Background**: Deep purple/magenta (#4A3050)
- **Accents**: Electric blue (#00A0FF)

### Layout (200px width)

```
┌──────────────────────────────────────────────────────┐
│  PUNCH                                    [BYPASS]   │
├────────────────────────┬─────────────────────────────┤
│     CLIPPER            │      TRANSIENTS             │
│  ┌───┐  ┌───┐  ┌───┐  │   ┌───┐  ┌───┐  ┌───┐      │
│  │THR│  │SFT│  │ OS│  │   │ATK│  │SUS│  │SNS│      │
│  └───┘  └───┘  └───┘  │   └───┘  └───┘  └───┘      │
│  Thresh Soft   Over   │   Attack Sustain Sens       │
│         ness  sample  │                             │
│                       │   ┌───┐  ┌───┐              │
│  [HARD][SOFT][CUBIC]  │   │ A │  │ R │              │
│      clip mode        │   └───┘  └───┘              │
│                       │   Atk    Rel                │
│                       │   Time   Time               │
├───────────────────────┴─────────────────────────────┤
│  ┌───┐              ┌───┐              ┌───┐        │
│  │ IN│    [METER]   │MIX│    [METER]   │OUT│        │
│  └───┘              └───┘              └───┘        │
│  Input    GR/Trans   Mix     Output     Out         │
└──────────────────────────────────────────────────────┘
```

---

## Signal Chain Position

### Recommended: End of Chain
```
[API5500] → [ButterComp2] → [Pultec] → [DynEQ] → [Transformer] → [PUNCH]
```

### Alternative: Before Transformer
```
[API5500] → [ButterComp2] → [Pultec] → [DynEQ] → [PUNCH] → [Transformer]
```

### User Choice
Leverage existing module reordering system for flexibility.

---

## Performance Budget

| Aspect | Target | Notes |
|--------|--------|-------|
| CPU (8x OS) | < 8% single core | Main cost is oversampling |
| Latency | 64-256 samples | Dependent on oversampling |
| Memory | < 1MB | Envelope states + filter buffers |

---

## Development Approach

### Phase 1: Core Clipper (Pure Rust)
**Estimated Complexity**: Low-Medium
**Tasks**:
1. Implement hard clip with threshold parameter
2. Add soft clip (tanh/cubic) with softness blend
3. Implement oversampling (use existing crate or port)
4. Add input/output gain staging

### Phase 2: Transient Detection (Pure Rust)
**Estimated Complexity**: Medium
**Tasks**:
1. Implement dual-envelope follower (fast/slow)
2. Add transient detection threshold/sensitivity
3. Calculate transient gain multiplier
4. Tune time constants based on psychoacoustic research

### Phase 3: Transient Shaping
**Estimated Complexity**: Medium
**Tasks**:
1. Apply transient gain to attack portion
2. Add sustain control
3. Implement parallel blend architecture
4. Add smoothing to prevent artifacts

### Phase 4: Integration & GUI
**Estimated Complexity**: Medium
**Tasks**:
1. Integrate into NIH-Plug parameter system
2. Add to module chain with bypass
3. Implement vizia GUI components
4. Add metering (gain reduction, transient activity)

### Phase 5: Optimization & Polish
**Estimated Complexity**: Low
**Tasks**:
1. SIMD optimization for oversampling
2. CPU profiling and optimization
3. A/B testing against reference plugins
4. Documentation and presets

---

## Agent Orchestration System

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MASTER AGENT (Opus)                              │
│                    punch-module-orchestrator                        │
│                                                                     │
│  Responsibilities:                                                  │
│  • Read this spec and manage task execution                         │
│  • Spawn child agents with appropriate models                       │
│  • Track progress and handle dependencies                           │
│  • Escalate complex issues / make architectural decisions           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
            ┌───────────────────────┼───────────────────────┐
            │                       │                       │
            ▼                       ▼                       ▼
    ┌───────────────┐     ┌───────────────┐     ┌───────────────┐
    │  Phase Agents │     │  Phase Agents │     │  Phase Agents │
    │   (Sonnet)    │     │   (Sonnet)    │     │    (Opus)     │
    │               │     │               │     │               │
    │ Implementation│     │  Integration  │     │ Optimization  │
    └───────────────┘     └───────────────┘     └───────────────┘
            │                       │                       │
            ▼                       ▼                       ▼
    ┌───────────────┐     ┌───────────────┐     ┌───────────────┐
    │  Task Agents  │     │  Task Agents  │     │  Task Agents  │
    │   (Haiku)     │     │   (Sonnet)    │     │   (Sonnet)    │
    │               │     │               │     │               │
    │ Simple tasks  │     │  GUI work     │     │  Profiling    │
    └───────────────┘     └───────────────┘     └───────────────┘
```

### Task Hierarchy with Model Assignments

#### Phase 1: Core Clipper

| Task ID | Task | Model | Parent | Dependencies | Status |
|---------|------|-------|--------|--------------|--------|
| `1.0` | Phase 1 Coordinator | **sonnet** | master | - | pending |
| `1.1` | Create `src/punch.rs` module skeleton | **haiku** | 1.0 | - | pending |
| `1.2` | Implement hard clip algorithm | **sonnet** | 1.0 | 1.1 | pending |
| `1.3` | Implement soft clip (tanh/cubic) | **sonnet** | 1.0 | 1.2 | pending |
| `1.4` | Implement oversampling | **sonnet** | 1.0 | 1.2 | pending |
| `1.5` | Add input/output gain staging | **haiku** | 1.0 | 1.2 | pending |
| `1.6` | Unit tests for clipper | **sonnet** | 1.0 | 1.2, 1.3 | pending |
| `1.7` | Phase 1 integration test | **sonnet** | 1.0 | 1.2-1.6 | pending |

#### Phase 2: Transient Detection

| Task ID | Task | Model | Parent | Dependencies | Status |
|---------|------|-------|--------|--------------|--------|
| `2.0` | Phase 2 Coordinator | **sonnet** | master | 1.7 | pending |
| `2.1` | Implement fast envelope follower | **sonnet** | 2.0 | - | pending |
| `2.2` | Implement slow envelope follower | **sonnet** | 2.0 | - | pending |
| `2.3` | Differential transient detector | **sonnet** | 2.0 | 2.1, 2.2 | pending |
| `2.4` | Add sensitivity/threshold controls | **haiku** | 2.0 | 2.3 | pending |
| `2.5` | Tune time constants (psychoacoustic) | **opus** | 2.0 | 2.3 | pending |
| `2.6` | Unit tests for detection | **sonnet** | 2.0 | 2.3-2.5 | pending |

#### Phase 3: Transient Shaping

| Task ID | Task | Model | Parent | Dependencies | Status |
|---------|------|-------|--------|--------------|--------|
| `3.0` | Phase 3 Coordinator | **sonnet** | master | 2.6 | pending |
| `3.1` | Implement attack gain shaper | **sonnet** | 3.0 | - | pending |
| `3.2` | Implement sustain control | **sonnet** | 3.0 | 3.1 | pending |
| `3.3` | Parallel blend architecture | **sonnet** | 3.0 | 3.1, 3.2 | pending |
| `3.4` | Smoothing/anti-artifact filters | **sonnet** | 3.0 | 3.3 | pending |
| `3.5` | Integration with clipper | **sonnet** | 3.0 | 1.7, 3.4 | pending |
| `3.6` | End-to-end DSP tests | **sonnet** | 3.0 | 3.5 | pending |

#### Phase 4: Plugin Integration & GUI

| Task ID | Task | Model | Parent | Dependencies | Status |
|---------|------|-------|--------|--------------|--------|
| `4.0` | Phase 4 Coordinator | **sonnet** | master | 3.6 | pending |
| `4.1` | Define NIH-Plug parameters (~14) | **sonnet** | 4.0 | - | pending |
| `4.2` | Add to `lib.rs` module chain | **sonnet** | 4.0 | 4.1 | pending |
| `4.3` | Implement bypass logic | **haiku** | 4.0 | 4.2 | pending |
| `4.4` | Create vizia GUI layout | **sonnet** | 4.0 | 4.1 | pending |
| `4.5` | Implement knob/button bindings | **sonnet** | 4.0 | 4.4 | pending |
| `4.6` | Add GR/transient metering | **sonnet** | 4.0 | 4.5 | pending |
| `4.7` | GUI styling (purple/blue theme) | **haiku** | 4.0 | 4.6 | pending |
| `4.8` | Full plugin build test | **sonnet** | 4.0 | 4.2-4.7 | pending |

#### Phase 5: Optimization & Polish

| Task ID | Task | Model | Parent | Dependencies | Status |
|---------|------|-------|--------|--------------|--------|
| `5.0` | Phase 5 Coordinator | **opus** | master | 4.8 | pending |
| `5.1` | CPU profiling analysis | **opus** | 5.0 | - | pending |
| `5.2` | SIMD optimization (oversampling) | **opus** | 5.0 | 5.1 | pending |
| `5.3` | Memory optimization | **sonnet** | 5.0 | 5.1 | pending |
| `5.4` | A/B testing vs reference plugins | **opus** | 5.0 | 5.2, 5.3 | pending |
| `5.5` | Create default presets | **haiku** | 5.0 | 5.4 | pending |
| `5.6` | Update CLAUDE.md documentation | **haiku** | 5.0 | 5.5 | pending |
| `5.7` | Final integration test | **sonnet** | 5.0 | 5.5 | pending |

### Master Agent Prompt Template

```markdown
# Punch Module Implementation - Master Agent

You are the orchestrator for implementing the Punch module in the bus_channel_strip VST plugin.

## Your Responsibilities

1. **Read** `PUNCH_MODULE_SPEC.md` for full specifications
2. **Track** task status in the Task Hierarchy tables
3. **Spawn** child agents with the correct `model` parameter
4. **Verify** each task completion before marking done
5. **Escalate** architectural decisions or blockers

## Execution Rules

1. Execute tasks in dependency order (check "Dependencies" column)
2. Use the specified model for each task (check "Model" column)
3. Update task status as: pending → in_progress → completed/blocked
4. For blocked tasks, document the blocker and attempt resolution
5. Phase coordinators (X.0) should verify all phase tasks before proceeding

## Spawning Child Agents

Use the Task tool with:
- `subagent_type`: "general-purpose"
- `model`: (from task table - "opus", "sonnet", or "haiku")
- `prompt`: Include task ID, full context, and expected deliverables

## Current State

Read the Status column in the task tables above. Begin with the first
pending task whose dependencies are satisfied.

## Key Files

- Spec: `PUNCH_MODULE_SPEC.md`
- Target: `src/punch.rs` (new file)
- Integration: `src/lib.rs`
- GUI: `src/editor.rs`
- Reference: `src/buttercomp2.rs` (similar module pattern)
```

### Child Agent Prompt Template

```markdown
# Task {TASK_ID}: {TASK_NAME}

## Context
You are implementing part of the Punch module for the bus_channel_strip VST plugin.
Read `PUNCH_MODULE_SPEC.md` for full specifications.

## Your Task
{DETAILED_TASK_DESCRIPTION}

## Dependencies Completed
{LIST_OF_COMPLETED_DEPENDENCY_OUTPUTS}

## Expected Deliverables
1. {SPECIFIC_OUTPUT_1}
2. {SPECIFIC_OUTPUT_2}
3. {etc}

## Constraints
- Follow existing code patterns in `src/buttercomp2.rs`
- Maintain lock-free, allocation-free audio processing
- Use `#[derive(Params)]` for parameter bindings

## Report Back
When complete, provide:
1. Files created/modified
2. Key implementation decisions
3. Any blockers or concerns for downstream tasks
```

### Model Selection Rationale

| Model | Cost | Speed | Best For |
|-------|------|-------|----------|
| **opus** | $$$ | Slow | Architecture, optimization, complex debugging, psychoacoustic tuning |
| **sonnet** | $$ | Medium | Implementation, integration, testing, most coding tasks |
| **haiku** | $ | Fast | Boilerplate, simple edits, documentation, styling |

### Execution Estimate

| Phase | Tasks | Estimated Agent Calls | Primary Model |
|-------|-------|----------------------|---------------|
| 1 | 8 | 10-12 | sonnet |
| 2 | 7 | 8-10 | sonnet/opus |
| 3 | 7 | 8-10 | sonnet |
| 4 | 9 | 12-15 | sonnet |
| 5 | 8 | 10-12 | opus/sonnet |
| **Total** | **39** | **~50-60** | - |

---

## References

### Psychoacoustics
- [The Times of Ira Hirsh: Multiple Ranges of Auditory Temporal Perception](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1363770/)
- [Temporal integration in human auditory cortex](https://pmc.ncbi.nlm.nih.gov/articles/PMC11463558/)
- [The impact of onset transient duration on perceived transient loudness](https://www.diva-portal.org/smash/get/diva2:1433470/FULLTEXT01.pdf)
- Fastl & Zwicker - "Psychoacoustics: Facts and Models" (2007)

### Airwindows
- [Airwindows GitHub Repository](https://github.com/airwindows/airwindows)
- [ClipOnly2 Documentation](https://www.airwindows.com/cliponly2/)
- [ADClip8 Documentation](https://www.airwindows.com/adclip8/)
- [Surge Documentation](https://www.airwindows.com/surge/)
- [Pop2 Documentation](https://www.airwindows.com/pop2/)
- [ClipSoftly Documentation](https://www.airwindows.com/clipsoftly/)

### Engineering
- "Designing Audio Effect Plugins in C++" - Will Pirkle
- [Production Expert: Top Transient Shaping Plugins](https://www.production-expert.com/production-expert-1/6-top-transient-shaping-plugins-in-2021)

### Original Insight
- "Absurdly LOUD Mixes and How The Pros Actually Do It" - Alex Emrich (YouTube)

---

## Status

- [x] Psychoacoustic research complete
- [x] Airwindows module research complete
- [x] DSP algorithm selection finalized
- [x] Development approach documented
- [x] Agent orchestration system designed
- [x] Implementation started
- [x] Phase 1: Core Clipper (Pure Rust)
  - [x] Hard clip algorithm
  - [x] Soft clip (tanh/cubic) with blend
  - [x] Oversampling (1x/4x/8x/16x)
  - [x] Input/output gain staging
  - [x] Unit tests (10 tests passing)
- [x] Phase 2: Transient Detection (Pure Rust)
  - [x] Dual-envelope follower (fast/slow)
  - [x] Differential transient detector
  - [x] Sensitivity and threshold controls
  - [x] Psychoacoustic time constants (5-30ms attack window)
  - [x] Unit tests passing
- [x] Phase 3: Transient Shaping
  - [x] Attack gain shaping (-100% to +100%)
  - [x] Sustain control
  - [x] Parallel blend architecture
  - [x] Smoothing filters
  - [x] Integration with clipper
  - [x] End-to-end DSP tests passing
- [x] Phase 4: Integration & GUI
  - [x] 14 parameters integrated into NIH-Plug
  - [x] GUI module added (5th slot, electric blue theme)
  - [x] Default parameters fixed (bypassed by default, gentle settings)
  - [x] Responsive GUI (1800x650 default, 1680x620 minimum)
  - [x] All modules visible and accessible
  - [x] Build system updated (LLVM paths for Windows)
  - [x] Documentation reorganized into docs/ directory
- [ ] Phase 5: Optimization & Polish
  - [ ] CPU profiling analysis
  - [ ] SIMD optimization for oversampling
  - [ ] Memory optimization
  - [ ] A/B testing vs reference plugins
  - [ ] Default presets creation
  - [ ] Final integration testing

## Implementation Notes (December 2025)

### Successful Testing
- ✅ Module sounds great per user feedback
- ✅ All 5 modules visible in resizable GUI
- ✅ Punch module bypassed by default (safe defaults)
- ✅ Conservative defaults prevent harsh clipping

### GUI Implementation
- Default window size: 1800x650 pixels
- Minimum size: 1680x620 pixels (ensures all modules fit)
- Responsive layout using vizia Stretch(1.0) units
- 5 module slots @ 320px each + gaps

### Module Reordering
- Backend parameters exist: `module_order_1` through `module_order_6`
- GUI not yet implemented - modules appear in fixed visual order
- Signal flow is controlled by parameters (accessible via DAW automation)
- TODO: Add dropdown selectors for visual module reordering
