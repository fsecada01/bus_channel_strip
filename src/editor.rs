// src/editor.rs
// Vizia GUI implementation for Bus Channel Strip

use nih_plug::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::{ParamButton, ParamButtonExt, ParamSlider, RawParamEvent};
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::components::{self, ModuleTheme};
use crate::spectral;
use crate::styles::COMPONENT_STYLES;
use crate::{BusChannelStripParams, ModuleType};

// ============================================================================
// App Events
// ============================================================================

/// Drop-position relative to a target slot, computed from cursor X at
/// drop time. Drives swap-vs-insert semantics:
/// - `Onto` (middle third): swap source ↔ target
/// - `Before`/`After` (left/right third): remove source, insert before/after target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPos {
    Before,
    Onto,
    After,
}

// Manual `vizia::Data` impl — the local `pub struct Data` (our model)
// shadows the prelude's `Data` trait, so the derive macro can't resolve it.
// Spelling out the trait via its absolute path sidesteps the shadowing.
impl vizia_plug::vizia::binding::Data for DropPos {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    /// Emitted from a slot's `on_drag` callback the moment vizia detects
    /// drag-start (cursor leaves source view with LMB held). Sets the
    /// reactive `drag_source` so eligible drop targets can light up.
    DragStarted(usize),
    /// Emitted from a slot's `on_drop` callback. `position` is computed
    /// from cursor X within the target slot's bounds at release time.
    /// Handler resolves the actual reorder against `drag_source`.
    DropOnSlot { target: usize, position: DropPos },
    /// Cancel any in-flight drag without committing. Wired to Esc and to
    /// a defensive `MouseLeave` on the chassis root (vizia#407 / baseview
    /// stuck-capture mitigation).
    DragCancel,
    /// Live cursor-over-target update emitted from each slot's `on_mouse_move`
    /// while a drag is in flight. Drives the directional drop indicator
    /// (left bar / full ring / right bar) and lets the user see the resolved
    /// drop intent before releasing.
    DragHover { target: usize, position: DropPos },
    /// Assign a specific ModuleType to a slot. Used by the library picker
    /// (Empty slot → fill with chosen module) and the eject action
    /// (filled slot → ModuleType::Empty).
    SetSlotModule(usize, ModuleType),
    /// Load one of the stock chain presets — writes all 7 module_order_*
    /// params to the preset's prescribed order. Bypass states and per-module
    /// parameters are left untouched (intentional: presets are a routing
    /// shortcut, not a full plugin preset).
    LoadChain(usize),
    /// Library sidebar action: if `mt` is already in the rack, focus that
    /// slot; otherwise add it to the first empty slot. No-op if the rack
    /// is full of other modules and there's no empty slot.
    AddOrFocusModule(ModuleType),
    /// Exit focus mode entirely — every slot returns to its hide-flag-driven
    /// presentation. Wired to the chassis-header EXIT FOCUS button.
    ClearFocus,
    /// Open the full-screen DynEQ back view.
    OpenDynEq,
    /// Return from DynEQ back view to the strip front view.
    CloseDynEq,
    /// Toggle the expand/collapse state of a DynEQ band (0–3). GUI-only state.
    ToggleDynEQBand(usize),
    /// Set the chassis zoom level (percentage: 75, 100, 125, 150, 200).
    /// Applied via toggle_class on the chassis root; CSS scales content widths.
    SetZoom(u8),
    /// Request a one-shot sidechain masking analysis from the audio thread.
    #[cfg(feature = "dynamic_eq")]
    RequestAnalysis,
    /// Apply analysis results to the appropriate DynEQ band parameters.
    #[cfg(feature = "dynamic_eq")]
    ApplyAnalysis {
        band: u32,
        freq: f32,
        threshold_db: f32,
    },
}

// ============================================================================
// Editor Data Model
// ============================================================================

#[derive(Lens)]
pub struct Data {
    pub params: Arc<BusChannelStripParams>,
    /// `Some(slot)` while a drag-drop is in flight from that source slot.
    /// Set by `AppEvent::DragStarted`, cleared on `DropOnSlot`/`DragCancel`.
    /// Drives the "eligible drop target" visual class on every other slot.
    pub drag_source: Option<usize>,
    /// Resolved drop site while a drag is in flight: `(target_slot, position)`.
    /// Updated continuously by per-slot `on_mouse_move` so the rack can
    /// preview the drop visually before release. Cleared on drag end.
    pub drop_target: Option<(usize, DropPos)>,
    /// Window-local cursor position, refreshed every `MouseMove` while a
    /// drag is in flight. Used to anchor the floating ghost label so the
    /// user always sees what they're dragging next to the cursor.
    pub cursor_x: f32,
    pub cursor_y: f32,
    /// When true, the DynEQ back view is shown instead of the strip.
    pub dyneq_open: bool,
    /// GUI-only expand state for each of the 4 DynEQ bands. Never accessed from audio thread.
    pub dyneq_band_expand: Arc<[AtomicBool; 4]>,
    /// Incremented on every ToggleDynEQBand — used as lens target to trigger .display() re-evaluation.
    pub dyneq_expand_gen: u32,
    /// Shared with the audio thread — GUI sets true to trigger a masking analysis.
    pub analysis_requested: Arc<AtomicBool>,
    /// Shared with the audio thread — read after analysis completes.
    pub analysis_result: Arc<spectral::AnalysisResult>,
    /// Current chassis zoom level as integer percentage. Valid: 75, 100, 125, 150, 200.
    /// Applied via toggle_class to the chassis root; CSS scales slot width + padding.
    pub zoom_level: u8,
    /// When `Some(slot)`, the rack is in focus mode: that slot renders full
    /// and every other slot collapses to its narrow tab regardless of its
    /// per-module hide flag. Set only via keyboard `1..7`; click-to-focus
    /// was removed when the slot body became the drag source.
    pub focused_slot: Option<usize>,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        // ── Window events: keyboard shortcuts + drag cancel ─────────────
        // Esc       — exit focus mode and cancel any in-flight drag
        // 1..7      — focus the corresponding real-module slot
        // MouseLeave at chassis root — defensive cancel for vizia#407
        //             baseview stuck-capture footgun (see synthesis report)
        //
        // No MouseUp listener: vizia's on_drop fires on the actually-hovered
        // target, so the hand-rolled state machine is gone.
        event.map(|win: &WindowEvent, _| match win {
            WindowEvent::KeyDown(code, _) => match code {
                Code::Escape => {
                    self.focused_slot = None;
                    self.drag_source = None;
                    self.drop_target = None;
                }
                Code::Digit1 => self.focus_if_real(0),
                Code::Digit2 => self.focus_if_real(1),
                Code::Digit3 => self.focus_if_real(2),
                Code::Digit4 => self.focus_if_real(3),
                Code::Digit5 => self.focus_if_real(4),
                Code::Digit6 => self.focus_if_real(5),
                Code::Digit7 => self.focus_if_real(6),
                _ => {}
            },
            WindowEvent::MouseLeave => {
                // Cursor left the editor window. If a drag was in flight,
                // the OS may not deliver MouseUp back to us — drop the
                // session defensively so we don't leave drag_source stuck.
                if self.drag_source.is_some() {
                    cx.emit(AppEvent::DragCancel);
                }
            }
            WindowEvent::MouseMove(x, y) => {
                // Track cursor while dragging so the floating ghost label
                // can follow. Outside a drag we ignore (saves needless lens
                // notifications since cursor_x/y feed only the ghost view).
                if self.drag_source.is_some() {
                    self.cursor_x = *x;
                    self.cursor_y = *y;
                }
            }
            _ => {}
        });

        event.map(|e: &AppEvent, _| match e {
            AppEvent::OpenDynEq => {
                self.dyneq_open = true;
            }
            AppEvent::CloseDynEq => {
                self.dyneq_open = false;
            }

            AppEvent::ToggleDynEQBand(band) => {
                let band = *band;
                if band < 4 {
                    let current = self.dyneq_band_expand[band].load(Ordering::Relaxed);
                    self.dyneq_band_expand[band].store(!current, Ordering::Relaxed);
                    self.dyneq_expand_gen = self.dyneq_expand_gen.wrapping_add(1);
                }
            }

            AppEvent::SetZoom(level) => {
                // Clamp to supported discrete levels. Unknown values fall back to 100.
                // NOTE: vizia-plug does not support runtime host-window resize
                // (cx.set_user_scale_factor / WindowEvent::SetSize aren't wired
                // into baseview), so zoom only rescales content within the
                // fixed window — slot widths grow, fonts grow via CSS classes,
                // ScrollView reveals off-screen slots.
                self.zoom_level = match *level {
                    75 | 100 | 125 | 150 | 200 => *level,
                    _ => 100,
                };
            }

            #[cfg(feature = "dynamic_eq")]
            AppEvent::RequestAnalysis => {
                self.analysis_requested.store(true, Ordering::Relaxed);
            }

            #[cfg(feature = "dynamic_eq")]
            AppEvent::ApplyAnalysis {
                band,
                freq,
                threshold_db,
            } => {
                // Clear ready so the button reflects "stale" state until next analysis.
                self.analysis_result.ready.store(false, Ordering::Relaxed);

                let (freq_ptr, thresh_ptr) = match *band {
                    0 => (
                        self.params.dyneq_band1_freq.as_ptr(),
                        self.params.dyneq_band1_threshold.as_ptr(),
                    ),
                    1 => (
                        self.params.dyneq_band2_freq.as_ptr(),
                        self.params.dyneq_band2_threshold.as_ptr(),
                    ),
                    2 => (
                        self.params.dyneq_band3_freq.as_ptr(),
                        self.params.dyneq_band3_threshold.as_ptr(),
                    ),
                    _ => (
                        self.params.dyneq_band4_freq.as_ptr(),
                        self.params.dyneq_band4_threshold.as_ptr(),
                    ),
                };

                // Safety: ParamPtr is obtained from self.params (Arc'd, outlives the editor).
                let freq_norm = unsafe { freq_ptr.preview_normalized(*freq) };
                let thresh_norm = unsafe { thresh_ptr.preview_normalized(*threshold_db) };

                cx.emit(RawParamEvent::BeginSetParameter(freq_ptr));
                cx.emit(RawParamEvent::SetParameterNormalized(freq_ptr, freq_norm));
                cx.emit(RawParamEvent::EndSetParameter(freq_ptr));

                cx.emit(RawParamEvent::BeginSetParameter(thresh_ptr));
                cx.emit(RawParamEvent::SetParameterNormalized(
                    thresh_ptr,
                    thresh_norm,
                ));
                cx.emit(RawParamEvent::EndSetParameter(thresh_ptr));
            }

            AppEvent::AddOrFocusModule(mt) => {
                if let Some(slot) = slot_containing(&self.params, *mt) {
                    // Module is already in the rack — focus that slot.
                    self.focused_slot = Some(slot);
                } else if let Some(slot) = first_empty_slot(&self.params) {
                    // Add to the leftmost empty slot, then focus it so
                    // the user can immediately tweak the new module.
                    let ptr = slot_param_ptr(&self.params, slot);
                    let norm = slot_preview_normalized(&self.params, slot, *mt);
                    cx.emit(RawParamEvent::BeginSetParameter(ptr));
                    cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
                    cx.emit(RawParamEvent::EndSetParameter(ptr));
                    self.focused_slot = Some(slot);
                } else {
                    // If no empty slot exists, silently no-op (the user
                    // would have to eject something first; baseview lacks
                    // a clean error-dialog primitive).
                }
            }

            AppEvent::ClearFocus => {
                self.focused_slot = None;
                self.drag_source = None;
                self.drop_target = None;
            }

            AppEvent::LoadChain(idx) => {
                if let Some(preset) = CHAIN_PRESETS.get(*idx) {
                    // Write all seven slots in one batch so the host sees a
                    // coherent state change. Bypasses are intentionally not
                    // touched: presets define routing, not levels.
                    for slot in 0..7 {
                        let mt = preset.chain[slot];
                        let ptr = slot_param_ptr(&self.params, slot);
                        let norm = slot_preview_normalized(&self.params, slot, mt);
                        cx.emit(RawParamEvent::BeginSetParameter(ptr));
                        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
                        cx.emit(RawParamEvent::EndSetParameter(ptr));
                    }
                    // Reset transient view state so the loaded chain shows
                    // as the overview instead of focused on whatever was
                    // there before.
                    self.drag_source = None;
                    self.drop_target = None;
                    self.focused_slot = None;
                }
            }

            AppEvent::SetSlotModule(slot, mt) => {
                // Direct param write — bypasses the swap logic so a slot can
                // become Empty (eject) or be filled from the library picker
                // without disturbing other slots. The audio dispatcher's
                // dedup loop tolerates duplicates harmlessly, but the picker
                // filters to prevent them at the UI level.
                let ptr = slot_param_ptr(&self.params, *slot);
                let norm = slot_preview_normalized(&self.params, *slot, *mt);
                cx.emit(RawParamEvent::BeginSetParameter(ptr));
                cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
                cx.emit(RawParamEvent::EndSetParameter(ptr));
                // Clear any in-flight drag — replacing a slot's contents
                // while a swap is staged would be ambiguous.
                self.drag_source = None;
                self.drop_target = None;
            }

            AppEvent::DragStarted(idx) => {
                self.drag_source = Some(*idx);
                self.drop_target = None;
            }

            AppEvent::DragCancel => {
                self.drag_source = None;
                self.drop_target = None;
            }

            AppEvent::DragHover { target, position } => {
                // Only update if the cursor is over a different slot than
                // the drag source — self-hover is meaningless and would
                // light up the source as its own target.
                if self.drag_source.is_some() && self.drag_source != Some(*target) {
                    let next = Some((*target, *position));
                    if self.drop_target != next {
                        self.drop_target = next;
                    }
                }
            }

            AppEvent::DropOnSlot { target, position } => {
                // Resolve drop into a concrete reorder operation. If
                // drag_source is None (defensive — shouldn't happen because
                // on_drop only fires after a drag started), no-op.
                if let Some(src) = self.drag_source {
                    self.reorder(cx, src, *target, *position);
                }
                self.drag_source = None;
                self.drop_target = None;
            }
        });
    }
}

impl Data {
    /// Focus a slot ONLY if it holds a real module. Empty slots silently
    /// stay unfocused — focusing one would collapse every real slot via
    /// the "any-other-focused → collapsed" render rule, leaving nothing
    /// to inspect.
    fn focus_if_real(&mut self, idx: usize) {
        if slot_module_type(&self.params, idx) != ModuleType::Empty {
            self.focused_slot = Some(idx);
            self.drag_source = None;
            self.drop_target = None;
        }
    }

    /// Apply a reorder operation against the seven `module_order_*` params
    /// in one event frame. `position` decides semantics:
    ///   • `Onto`   → swap src ↔ tgt (two slots change)
    ///   • `Before` → remove src, insert before tgt (rotation; up to 7 change)
    ///   • `After`  → remove src, insert after tgt (rotation; up to 7 change)
    ///
    /// Reads the current order, computes the new order in-memory, then
    /// writes back any slot that actually changed. This minimises host
    /// preset diff size and avoids spurious automation events.
    fn reorder(&self, cx: &mut EventContext, src: usize, tgt: usize, position: DropPos) {
        if src == tgt && position == DropPos::Onto {
            return; // self-drop, nothing to do
        }

        let mut order: [ModuleType; 7] = [
            self.params.module_order_1.value(),
            self.params.module_order_2.value(),
            self.params.module_order_3.value(),
            self.params.module_order_4.value(),
            self.params.module_order_5.value(),
            self.params.module_order_6.value(),
            self.params.module_order_7.value(),
        ];

        match position {
            DropPos::Onto => {
                order.swap(src, tgt);
            }
            DropPos::Before | DropPos::After => {
                let moving = order[src];
                // Remove src, then insert at the resolved index.
                // After removal, indices > src shift down by one — the
                // insert index must account for this.
                let raw_insert = match position {
                    DropPos::Before => tgt,
                    DropPos::After => tgt + 1,
                    DropPos::Onto => unreachable!(),
                };
                let mut tail: [ModuleType; 6] = [ModuleType::Empty; 6];
                let mut j = 0;
                for (i, &mt) in order.iter().enumerate() {
                    if i == src {
                        continue;
                    }
                    tail[j] = mt;
                    j += 1;
                }
                let insert = raw_insert
                    .min(6)
                    .saturating_sub(if raw_insert > src { 1 } else { 0 });
                let mut rebuilt = [ModuleType::Empty; 7];
                let mut k = 0;
                for (i, &mt) in tail.iter().enumerate() {
                    if i == insert {
                        rebuilt[k] = moving;
                        k += 1;
                    }
                    rebuilt[k] = mt;
                    k += 1;
                }
                if k < 7 {
                    rebuilt[k] = moving;
                }
                order = rebuilt;
            }
        }

        // Write back only the slots that actually changed.
        let before: [ModuleType; 7] = [
            self.params.module_order_1.value(),
            self.params.module_order_2.value(),
            self.params.module_order_3.value(),
            self.params.module_order_4.value(),
            self.params.module_order_5.value(),
            self.params.module_order_6.value(),
            self.params.module_order_7.value(),
        ];
        for slot in 0..7usize {
            if before[slot] == order[slot] {
                continue;
            }
            let ptr = slot_param_ptr(&self.params, slot);
            let norm = slot_preview_normalized(&self.params, slot, order[slot]);
            cx.emit(RawParamEvent::BeginSetParameter(ptr));
            cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
            cx.emit(RawParamEvent::EndSetParameter(ptr));
        }
    }
}

/// Hit-test cursor X within a slot's bounds → DropPos.
/// Left third → Before, middle third → Onto, right third → After.
fn hit_test_drop_pos(cursor_x: f32, bounds: BoundingBox) -> DropPos {
    if bounds.w <= 0.0 {
        return DropPos::Onto;
    }
    let rel = (cursor_x - bounds.x) / bounds.w;
    if rel < 0.33 {
        DropPos::Before
    } else if rel > 0.66 {
        DropPos::After
    } else {
        DropPos::Onto
    }
}

// ============================================================================
// Module Order Helpers
// ============================================================================

fn slot_module_type(params: &Arc<BusChannelStripParams>, slot: usize) -> ModuleType {
    match slot {
        0 => params.module_order_1.value(),
        1 => params.module_order_2.value(),
        2 => params.module_order_3.value(),
        3 => params.module_order_4.value(),
        4 => params.module_order_5.value(),
        5 => params.module_order_6.value(),
        _ => params.module_order_7.value(),
    }
}

fn slot_param_ptr(params: &Arc<BusChannelStripParams>, slot: usize) -> ParamPtr {
    match slot {
        0 => params.module_order_1.as_ptr(),
        1 => params.module_order_2.as_ptr(),
        2 => params.module_order_3.as_ptr(),
        3 => params.module_order_4.as_ptr(),
        4 => params.module_order_5.as_ptr(),
        5 => params.module_order_6.as_ptr(),
        _ => params.module_order_7.as_ptr(),
    }
}

/// Returns the index of the first slot whose `module_order_*` value is
/// `Empty`, or `None` if every slot is occupied.
fn first_empty_slot(params: &Arc<BusChannelStripParams>) -> Option<usize> {
    (0..7).find(|&s| slot_module_type(params, s) == ModuleType::Empty)
}

/// Returns the index of the first slot containing `mt`, or `None` if no
/// slot holds that module type. Only meaningful for non-Empty types.
fn slot_containing(params: &Arc<BusChannelStripParams>, mt: ModuleType) -> Option<usize> {
    (0..7).find(|&s| slot_module_type(params, s) == mt)
}

fn slot_preview_normalized(
    params: &Arc<BusChannelStripParams>,
    slot: usize,
    mt: ModuleType,
) -> f32 {
    // EnumParam::preview_normalized takes the enum variant directly (Plain = ModuleType)
    match slot {
        0 => params.module_order_1.preview_normalized(mt),
        1 => params.module_order_2.preview_normalized(mt),
        2 => params.module_order_3.preview_normalized(mt),
        3 => params.module_order_4.preview_normalized(mt),
        4 => params.module_order_5.preview_normalized(mt),
        5 => params.module_order_6.preview_normalized(mt),
        _ => params.module_order_7.preview_normalized(mt),
    }
}

/// Converts ModuleType to usize for use as a vizia Binding lens target.
/// vizia's `Binding::new` requires `Target: Data`; usize satisfies that.
/// Empty maps to 7 to keep indices 0..6 stable for the seven real modules.
fn module_type_to_usize(mt: ModuleType) -> usize {
    match mt {
        ModuleType::Api5500EQ => 0,
        ModuleType::ButterComp2 => 1,
        ModuleType::PultecEQ => 2,
        ModuleType::DynamicEQ => 3,
        ModuleType::Transformer => 4,
        ModuleType::Punch => 5,
        ModuleType::Haas => 6,
        ModuleType::Empty => 7,
    }
}

fn usize_to_module_type(idx: usize) -> ModuleType {
    match idx {
        0 => ModuleType::Api5500EQ,
        1 => ModuleType::ButterComp2,
        2 => ModuleType::PultecEQ,
        3 => ModuleType::DynamicEQ,
        4 => ModuleType::Transformer,
        5 => ModuleType::Punch,
        6 => ModuleType::Haas,
        _ => ModuleType::Empty,
    }
}

/// Canonical list of real (non-Empty) modules in display order. Used by the
/// library picker and the duplicate repair pass.
const ALL_REAL_MODULES: [ModuleType; 7] = [
    ModuleType::Api5500EQ,
    ModuleType::ButterComp2,
    ModuleType::PultecEQ,
    ModuleType::DynamicEQ,
    ModuleType::Transformer,
    ModuleType::Punch,
    ModuleType::Haas,
];

// ============================================================================
// Chain Presets (a.k.a. "Dream Strips")
// ============================================================================

/// A named routing snapshot. Loading a chain rewrites every `module_order_*`
/// param to match `chain[N]`. Per-module parameters and bypass states are
/// intentionally left alone — the preset is a routing shortcut, not a full
/// plugin preset. Users layer their own tone/level adjustments on top.
struct ChainPreset {
    name: &'static str,
    /// Short tag used inside the compact selector button. Aim for 3–4 chars
    /// so the button row fits across the chassis header.
    tag: &'static str,
    /// One ModuleType per slot. Use `ModuleType::Empty` for unused slots.
    chain: [ModuleType; 7],
}

/// Stock chain presets. Order follows the design doc — first entry restores
/// the plugin's shipped default, last entry is a clean slate for users who
/// prefer to start from scratch.
const CHAIN_PRESETS: &[ChainPreset] = &[
    ChainPreset {
        name: "Default",
        tag: "DEF",
        chain: [
            ModuleType::Api5500EQ,
            ModuleType::ButterComp2,
            ModuleType::PultecEQ,
            ModuleType::Transformer,
            ModuleType::Haas,
            ModuleType::Punch,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Drum Bus",
        tag: "DRM",
        chain: [
            ModuleType::Transformer,
            ModuleType::Api5500EQ,
            ModuleType::ButterComp2,
            ModuleType::Punch,
            ModuleType::Empty,
            ModuleType::Empty,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Vocal Bus",
        tag: "VOX",
        chain: [
            ModuleType::PultecEQ,
            ModuleType::Api5500EQ,
            ModuleType::DynamicEQ,
            ModuleType::ButterComp2,
            ModuleType::Empty,
            ModuleType::Empty,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Mix Glue",
        tag: "GLU",
        chain: [
            ModuleType::Api5500EQ,
            ModuleType::ButterComp2,
            ModuleType::PultecEQ,
            ModuleType::Transformer,
            ModuleType::Empty,
            ModuleType::Empty,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Master",
        tag: "MST",
        chain: [
            ModuleType::DynamicEQ,
            ModuleType::PultecEQ,
            ModuleType::ButterComp2,
            ModuleType::Punch,
            ModuleType::Empty,
            ModuleType::Empty,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Wide Bus",
        tag: "WID",
        chain: [
            ModuleType::Api5500EQ,
            ModuleType::ButterComp2,
            ModuleType::Haas,
            ModuleType::Transformer,
            ModuleType::Empty,
            ModuleType::Empty,
            ModuleType::Empty,
        ],
    },
    ChainPreset {
        name: "Empty",
        tag: "—",
        chain: [ModuleType::Empty; 7],
    },
];

fn module_type_to_theme(mt: ModuleType) -> ModuleTheme {
    match mt {
        ModuleType::Api5500EQ => ModuleTheme::Api5500,
        ModuleType::ButterComp2 => ModuleTheme::ButterComp2,
        ModuleType::PultecEQ => ModuleTheme::Pultec,
        ModuleType::DynamicEQ => ModuleTheme::DynamicEq,
        ModuleType::Transformer => ModuleTheme::Transformer,
        ModuleType::Punch => ModuleTheme::Punch,
        ModuleType::Haas => ModuleTheme::Haas,
        ModuleType::Empty => ModuleTheme::Empty,
    }
}

fn module_type_name(mt: ModuleType) -> &'static str {
    match mt {
        ModuleType::Api5500EQ => "API 550A",
        ModuleType::ButterComp2 => "ButterComp2",
        ModuleType::PultecEQ => "Pultec EQP-1A",
        ModuleType::DynamicEQ => "Dynamic EQ",
        ModuleType::Transformer => "Console/Tape",
        ModuleType::Punch => "PUNCH",
        ModuleType::Haas => "HAAS",
        ModuleType::Empty => "EMPTY SLOT",
    }
}

/// Reads the per-module-type hide flag. Hide state is keyed by the module's
/// identity, not its slot position — so moving a module around preserves its
/// visibility setting.
fn is_module_hidden(params: &Arc<BusChannelStripParams>, mt: ModuleType) -> bool {
    match mt {
        ModuleType::Api5500EQ => params.hide_api5500.value(),
        ModuleType::ButterComp2 => params.hide_buttercomp2.value(),
        ModuleType::PultecEQ => params.hide_pultec.value(),
        ModuleType::DynamicEQ => params.hide_dynamic_eq.value(),
        ModuleType::Transformer => params.hide_transformer.value(),
        ModuleType::Punch => params.hide_punch.value(),
        ModuleType::Haas => params.hide_haas.value(),
        // Empty slots are never collapsible: there is no module to hide and
        // the picker affordance must stay reachable.
        ModuleType::Empty => false,
    }
}

/// Short 3-char tag for the collapsed tab. Keeps the narrow strip legible
/// without overflowing the tab width.
fn module_type_short_name(mt: ModuleType) -> &'static str {
    match mt {
        ModuleType::Api5500EQ => "API",
        ModuleType::ButterComp2 => "BC2",
        ModuleType::PultecEQ => "PLT",
        ModuleType::DynamicEQ => "DYN",
        ModuleType::Transformer => "TRF",
        ModuleType::Punch => "PCH",
        ModuleType::Haas => "HAS",
        ModuleType::Empty => "—",
    }
}

/// Small "×" button in the module header that toggles the hide flag for this
/// module type. Bound via the same BoolParam used by the collapsed-tab expand
/// button, so clicking either one flips state in the expected direction.
fn build_hide_button_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_api5500)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::ButterComp2 => {
            ParamButton::new(cx, Data::params, |p| &p.hide_buttercomp2)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::PultecEQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_pultec)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::DynamicEQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_dynamic_eq)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::Transformer => {
            ParamButton::new(cx, Data::params, |p| &p.hide_transformer)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::Punch => {
            ParamButton::new(cx, Data::params, |p| &p.hide_punch)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        ModuleType::Haas => {
            ParamButton::new(cx, Data::params, |p| &p.hide_haas)
                .with_label("\u{00d7}")
                .class("hide-btn");
        }
        // Empty slots have nothing to hide.
        ModuleType::Empty => {}
    }
}

/// Full-tab button in the collapsed view — clicking anywhere on the tab
/// flips the same BoolParam back to false, restoring the normal slot.
fn build_expand_button_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_api5500)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::ButterComp2 => {
            ParamButton::new(cx, Data::params, |p| &p.hide_buttercomp2)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::PultecEQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_pultec)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::DynamicEQ => {
            ParamButton::new(cx, Data::params, |p| &p.hide_dynamic_eq)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::Transformer => {
            ParamButton::new(cx, Data::params, |p| &p.hide_transformer)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::Punch => {
            ParamButton::new(cx, Data::params, |p| &p.hide_punch)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        ModuleType::Haas => {
            ParamButton::new(cx, Data::params, |p| &p.hide_haas)
                .with_label("\u{25B6}")
                .class("expand-btn");
        }
        // Empty slots are never collapsed (is_module_hidden returns false).
        ModuleType::Empty => {}
    }
}

/// Eject button — shown in the header of every filled slot. Clicking writes
/// `ModuleType::Empty` to the slot's `module_order_*` param, returning the
/// slot to the picker state. Distinct from the hide button (which only
/// collapses the visual presentation) because eject removes the module from
/// the audio chain entirely.
///
/// Wrapped in an HStack because vizia's `on_press` is reliably absorbed by
/// container views; bare Labels often pass pointer events through to their
/// parent, which would cause clicks to fall back to the drag handle row.
fn build_eject_button(cx: &mut Context, slot_idx: usize) {
    HStack::new(cx, |cx| {
        Label::new(cx, "\u{2715}").class("eject-btn-glyph"); // ✕
        Label::new(cx, "REMOVE").class("eject-btn-label");
    })
    .class("eject-btn")
    .on_press(move |cx| cx.emit(AppEvent::SetSlotModule(slot_idx, ModuleType::Empty)))
    .cursor(CursorIcon::Hand);
}

/// One-shot repair for saved sessions whose `module_order_*` values now collide
/// on a real module (e.g. older schema migrations). Walks slots 0..7, finds
/// duplicated *real* module types, and rewrites later duplicates to
/// `ModuleType::Empty`. Empty slots are valid in any position and are
/// skipped by the dedup check.
fn repair_module_order(cx: &mut Context, params: &Arc<BusChannelStripParams>) {
    let raw = [
        params.module_order_1.value(),
        params.module_order_2.value(),
        params.module_order_3.value(),
        params.module_order_4.value(),
        params.module_order_5.value(),
        params.module_order_6.value(),
        params.module_order_7.value(),
    ];
    let mut seen = [false; 7]; // indices 0..6 cover real modules only
    let mut dupe_slots: Vec<usize> = Vec::new();
    for (i, mt) in raw.iter().enumerate() {
        if *mt == ModuleType::Empty {
            continue;
        }
        let idx = module_type_to_usize(*mt);
        if seen[idx] {
            dupe_slots.push(i);
        } else {
            seen[idx] = true;
        }
    }
    if dupe_slots.is_empty() {
        return;
    }

    // Replace duplicates with Empty — preserves the user's intent
    // (the first occurrence stays where it is) without inventing a new
    // module the user didn't explicitly choose.
    for dupe_slot in dupe_slots {
        let ptr = slot_param_ptr(params, dupe_slot);
        let norm = slot_preview_normalized(params, dupe_slot, ModuleType::Empty);
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }
}

fn module_type_subtitle(mt: ModuleType) -> &'static str {
    match mt {
        ModuleType::Api5500EQ => "3-BAND EQ",
        ModuleType::ButterComp2 => "COMPRESSOR",
        ModuleType::PultecEQ => "TUBE EQ",
        ModuleType::DynamicEQ => "DYN EQ",
        ModuleType::Transformer => "TRANSFORMER",
        ModuleType::Punch => "CLIP + TRANSIENT",
        ModuleType::Haas => "STEREO WIDENER",
        ModuleType::Empty => "PICK A MODULE",
    }
}

// ============================================================================
// Editor Entry Points
// ============================================================================

/// Chassis sizing constants.
///
/// Slot width is fixed at 280px per design (at zoom 100%). With exactly 4
/// slots visible + 4px gaps + paddings + the 72px library sidebar, the
/// default window fits four modules horizontally; the remaining three
/// scroll into view via the strip ScrollView.
///
/// Math (at zoom 100%):
///   4 slots × 280 px           = 1120
///   3 gaps between slots × 4px =   12
///   Library sidebar            =   72
///   Sidebar / strip gap        =    4
///   Strip horizontal padding   =   32  (16 + 16)
///   Chassis outer padding      =   28  (14 + 14, reactive: 14 × zoom/100)
///   Scrollbar gutter + margin  =   28  (scrollbar ~12 + safety 16)
///   Total                      ≈ 1296 px → rounded up to 1300
///
/// At higher zoom levels the slot width grows (BASE × zoom/100) and the
/// chassis padding grows linearly as well; the window stays at 1300 px and
/// users scroll horizontally to reveal off-screen slots. Mini-map height
/// (28 px) and chain-preset row height bumped HEIGHT to 860 to keep the
/// rack body roughly the same vertical footprint as before the redesign.
pub const DEFAULT_WINDOW_WIDTH: u32 = 1300;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 860;

pub(crate) fn default_state() -> Arc<ViziaState> {
    // new_with_default_scale_factor persists the scale across sessions and
    // multiplies window size by it. We keep the factor at 1.0 because the
    // chassis content zoom is handled via toggle_class + CSS per zoom level,
    // which keeps the window at a fixed size and lets the ScrollView reveal
    // content that overflows. Visual zoom is a pure CSS concern.
    ViziaState::new_with_default_scale_factor(|| (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT), 1.0)
}

pub(crate) fn create(
    params: Arc<BusChannelStripParams>,
    editor_state: Arc<ViziaState>,
    spectrum_data: Arc<spectral::SpectrumData>,
    analysis_requested: Arc<AtomicBool>,
    analysis_result: Arc<spectral::AnalysisResult>,
    gr_data: Arc<spectral::GainReductionData>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(COMPONENT_STYLES)
            .expect("Failed to add stylesheet");

        Data {
            params: params.clone(),
            drag_source: None,
            drop_target: None,
            cursor_x: 0.0,
            cursor_y: 0.0,
            dyneq_open: false,
            dyneq_band_expand: Arc::new([
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
            ]),
            dyneq_expand_gen: 0,
            analysis_requested: analysis_requested.clone(),
            analysis_result: analysis_result.clone(),
            zoom_level: 100,
            focused_slot: None,
        }
        .build(cx);

        // Heal duplicate module_order_* assignments left over from sessions
        // saved under an older schema (fewer slots). When slot N defaults to
        // a type already occupied by an earlier slot, overwrite it with the
        // missing module type so every slot shows a unique module.
        repair_module_order(cx, &params);

        VStack::new(cx, |cx| {
            // ── Chassis header ──────────────────────────────────────────────
            // Three-zone band: brand title (left) | signal-flow hint (center,
            // flexible) | zoom + master (right). The inner pills share the
            // same translucent fill so the whole header reads as one gradient
            // surface rather than a row of clunky boxes.
            HStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "API").class("chassis-brand");
                    Label::new(cx, "Bus Channel Strip").class("chassis-title");
                })
                .width(Auto)
                .height(Auto)
                .gap(Pixels(12.0))
                .alignment(Alignment::Center);

                // EXIT FOCUS pill — only visible while a slot is focused.
                // Returns the rack to the all-slots overview. Sized small
                // because the rest of the header is already busy; positioned
                // next to the brand so users always know where to look.
                HStack::new(cx, |cx| {
                    Label::new(cx, "\u{2715} EXIT FOCUS").class("exit-focus-label");
                })
                .class("exit-focus-btn")
                .display(Data::focused_slot.map(|f| {
                    if f.is_some() {
                        Display::Flex
                    } else {
                        Display::None
                    }
                }))
                .on_press(|cx| cx.emit(AppEvent::ClearFocus))
                .cursor(CursorIcon::Hand)
                .height(Pixels(28.0))
                .width(Auto)
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));

                // Chain preset selector — centered, takes remaining space.
                // One button per stock chain; clicking writes all 7
                // module_order_* params atomically. Replaces the old
                // signal-flow hint text (the rack itself now teaches the
                // routing model better than a hint sentence could).
                build_chain_preset_selector(cx)
                    .left(Stretch(1.0))
                    .right(Stretch(1.0));

                // Zoom control band — discrete 75/100/125/150/200 buttons.
                create_zoom_controls(cx);

                create_master_section(cx);
            })
            .class("chassis-header")
            .height(Pixels(80.0))
            .width(Stretch(1.0));

            // ── Strip view ──────────────────────────────────────────────────
            // Library sidebar (left) + scrollable rack (right). The sidebar
            // is the SOLE add affordance: clicking an available (not-in-rack)
            // module drops it into the focused empty slot if there is one,
            // otherwise the first empty slot. Clicking an in-rack row
            // focuses that slot.
            HStack::new(cx, |cx| {
                build_library_sidebar(cx);

                ScrollView::new(cx, |cx| {
                    HStack::new(cx, |cx| {
                        for slot_idx in 0..7_usize {
                            create_dynamic_module_slot(cx, slot_idx);
                        }
                    })
                    .class("lunchbox-slots")
                    .height(Stretch(1.0))
                    // Inner width is driven by the slot widths themselves
                    // (fixed 280px × N + gaps). Auto lets the HStack size to
                    // its children so ScrollView can detect overflow.
                    .width(Auto)
                    .gap(Pixels(4.0));
                })
                .class("strip-scroll")
                .height(Stretch(1.0))
                .width(Stretch(1.0));
            })
            .height(Stretch(1.0))
            .width(Stretch(1.0))
            .gap(Pixels(4.0))
            .display(Data::dyneq_open.map(|o| {
                if *o {
                    Display::None
                } else {
                    Display::Flex
                }
            }));

            // ── DynEQ back view ─────────────────────────────────────────────
            build_dyneq_back_view(
                cx,
                spectrum_data.clone(),
                analysis_result.clone(),
                gr_data.clone(),
            );

            // ── Floating drag ghost ─────────────────────────────────────────
            // While a drag is in flight, render a small pill next to the
            // cursor showing the dragged module's tag. Position-type Absolute
            // takes it out of the layout flow; left/top track cursor_x/y
            // updated by the chassis MouseMove handler. Only built (Display:
            // Flex) when drag_source is Some.
            Binding::new(cx, Data::drag_source, |cx, ds_lens| {
                let Some(slot) = ds_lens.get(cx) else {
                    return;
                };
                let params = Data::params.get(cx);
                let mt = slot_module_type(&params, slot);
                let tag = module_type_short_name(mt);
                Label::new(cx, tag)
                    .class("drag-ghost")
                    .position_type(PositionType::Absolute)
                    .left(Data::cursor_x.map(|x| Pixels(*x + 14.0)))
                    .top(Data::cursor_y.map(|y| Pixels(*y + 14.0)));
            });
        })
        .class("lunchbox-chassis")
        // Keyboard shortcuts (Esc, 1..7) are routed through
        // WindowEvent::KeyDown which vizia targets at `cx.focused`. By
        // making the chassis focusable AND seeding initial focus on it,
        // shortcuts work from the first frame — without this, KeyDown
        // events go to Entity::null() and never reach the Data model.
        // Subsequent clicks on Sliders/Buttons move focus into those
        // widgets, but events still bubble up through the chassis to
        // the model.
        .focusable(true)
        .focused(true)
        .toggle_class("zoom-75", Data::zoom_level.map(|z| *z == 75))
        .toggle_class("zoom-100", Data::zoom_level.map(|z| *z == 100))
        .toggle_class("zoom-125", Data::zoom_level.map(|z| *z == 125))
        .toggle_class("zoom-150", Data::zoom_level.map(|z| *z == 150))
        .toggle_class("zoom-200", Data::zoom_level.map(|z| *z == 200))
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .padding(Data::zoom_level.map(|z| Pixels(14.0 * (*z as f32) / 100.0)));
        // vizia-plug doesn't support runtime host-window resize
        // (set_user_scale_factor / WindowEvent::SetSize aren't wired into
        // baseview). Zoom rescales content within the fixed window: slot
        // widths scale via reactive lens, fonts scale via CSS zoom-N rules,
        // and the strip ScrollView reveals off-screen slots when content
        // grows past the window width.
    })
}

// Library sidebar — narrow vertical strip on the left edge of the rack
// area. Lists every real module with a status indicator (in rack vs
// available). Clicking an "available" row adds the module to the first
// empty slot; clicking an "in rack" row focuses the slot containing it.
//
// This is the global counterpart to the per-slot picker: that one
// answers "what can I put here?", this one answers "where is X / how
// do I get X into the rack?". Both stay because they serve different
// workflows — the per-slot picker is contextual, the sidebar is
// inventory-oriented.
fn build_library_sidebar(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "LIBRARY").class("library-sidebar-header");

        // Reactive bitset of which module types are currently in the rack.
        // Rebuilds the row list whenever any slot's contents change.
        let in_rack_lens = Data::params.map(|p| {
            let mut bits: u8 = 0;
            for s in 0..7 {
                let mt = slot_module_type(p, s);
                if mt != ModuleType::Empty {
                    bits |= 1u8 << module_type_to_usize(mt);
                }
            }
            bits
        });

        Binding::new(cx, in_rack_lens, |cx, bits_b| {
            let in_rack = bits_b.get(cx);
            for mt in ALL_REAL_MODULES {
                let theme = module_type_to_theme(mt);
                let bit = 1u8 << module_type_to_usize(mt);
                let present = in_rack & bit != 0;
                let tag = module_type_short_name(mt);

                HStack::new(cx, |cx| {
                    // Status dot — accent-colored if in rack, dim otherwise.
                    Label::new(cx, if present { "\u{25CF}" } else { "\u{25CB}" })
                        .class("library-row-dot")
                        .color(if present {
                            theme.accent_color()
                        } else {
                            Color::rgb(90, 96, 108)
                        });
                    Label::new(cx, tag)
                        .class("library-row-tag")
                        .color(if present {
                            theme.accent_color()
                        } else {
                            Color::rgb(140, 146, 158)
                        });
                })
                .class("library-row")
                .toggle_class("library-row-in-rack", present)
                .on_press(move |cx| cx.emit(AppEvent::AddOrFocusModule(mt)))
                .cursor(CursorIcon::Hand)
                .height(Pixels(28.0))
                .width(Stretch(1.0))
                .gap(Pixels(4.0))
                .alignment(Alignment::Center);
            }
        });
    })
    .class("library-sidebar")
    .height(Stretch(1.0))
    .width(Pixels(72.0))
    .gap(Pixels(2.0));
}

// Chain preset selector — horizontal row of compact buttons in the chassis
// header. Each button shows a 3-char tag and the full preset name; clicking
// emits AppEvent::LoadChain(idx) which rewrites module_order_*. Returns the
// outer Handle so the caller can attach layout modifiers (Stretch margins,
// etc.) at the call site.
fn build_chain_preset_selector(cx: &mut Context) -> Handle<'_, VStack> {
    VStack::new(cx, |cx| {
        Label::new(cx, "CHAIN PRESETS").class("signal-flow-label");
        HStack::new(cx, |cx| {
            for (i, preset) in CHAIN_PRESETS.iter().enumerate() {
                VStack::new(cx, |cx| {
                    Label::new(cx, preset.tag).class("chain-preset-tag");
                    Label::new(cx, preset.name).class("chain-preset-name");
                })
                .class("chain-preset-btn")
                .on_press(move |cx| cx.emit(AppEvent::LoadChain(i)))
                .cursor(CursorIcon::Hand)
                .width(Pixels(64.0))
                .height(Pixels(40.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));
            }
        })
        .gap(Pixels(4.0))
        .height(Pixels(40.0))
        .width(Auto)
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
    })
    .class("signal-flow-section")
    .height(Auto)
    .width(Auto)
    .gap(Pixels(4.0))
}

// Discrete zoom buttons (75/100/125/150/200%). Each button emits SetZoom on
// press; the active level is styled via a reactive `zoom-btn-active` class so
// users can see which step is current.
fn create_zoom_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "ZOOM").class("zoom-label");
        HStack::new(cx, |cx| {
            for &level in &[75_u8, 100, 125, 150, 200] {
                VStack::new(cx, |cx| {
                    Label::new(
                        cx,
                        match level {
                            75 => "75",
                            100 => "100",
                            125 => "125",
                            150 => "150",
                            _ => "200",
                        },
                    )
                    .class("zoom-btn-label");
                })
                .class("zoom-btn")
                .toggle_class(
                    "zoom-btn-active",
                    Data::zoom_level.map(move |z| *z == level),
                )
                .on_press(move |cx| cx.emit(AppEvent::SetZoom(level)))
                .cursor(CursorIcon::Hand)
                .width(Pixels(36.0))
                .height(Pixels(24.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));
            }
        })
        .gap(Pixels(2.0))
        .height(Pixels(24.0))
        .width(Auto)
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
    })
    .class("zoom-controls")
    .height(Auto)
    .width(Auto)
    .gap(Pixels(4.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn create_master_section(cx: &mut Context) {
    HStack::new(cx, |cx| {
        // Global bypass — prominently placed so it's always reachable.
        VStack::new(cx, |cx| {
            Label::new(cx, "BYPASS")
                .class("param-label")
                .height(Pixels(16.0))
                .width(Stretch(1.0));
            components::create_bypass_button(cx, "BYPASS", |p| &p.global_bypass);
        })
        .height(Auto)
        .width(Pixels(80.0))
        .gap(Pixels(4.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));

        // Auto-gain compensation toggle.
        components::create_bool_button(cx, "AUTO GAIN", Data::params, |p| &p.global_auto_gain);

        Label::new(cx, "MASTER").class("master-label");
        components::create_gain_slider(cx, "Gain", Data::params, |p| &p.gain);
    })
    .class("master-controls")
    .gap(Pixels(12.0));
}

// ============================================================================
// Dynamic Module Slot
// ============================================================================

/// Creates one 500-series slot that reactively renders whatever module is
/// currently assigned to `module_order_{slot_idx+1}`. Three layers of
/// `Binding` track independent inputs that affect what gets rendered:
///   1. `Data::focused_slot` — focus mode collapses every non-focused slot
///   2. `Data::params` (module type) — rebuild when a swap or library pick
///      changes which module lives here
///   3. `Data::params` (hide flag for that module) — collapse when hidden
///
/// The drag-source highlight is toggled separately via `toggle_class`
/// which reacts to `Data::drag_source` without a full rebuild.
fn create_dynamic_module_slot(cx: &mut Context, slot_idx: usize) {
    Binding::new(cx, Data::focused_slot, move |cx, focus_b| {
        let focus = focus_b.get(cx);
        let this_focused = focus == Some(slot_idx);
        let any_focused = focus.is_some();

        // Use usize as the Binding target because vizia requires `Target: Data`,
        // and usize satisfies that bound whereas our ModuleType enum does not.
        Binding::new(
            cx,
            Data::params.map(move |p| module_type_to_usize(slot_module_type(p, slot_idx))),
            move |cx, mt_lens| {
                let mt = usize_to_module_type(mt_lens.get(cx));
                let theme = module_type_to_theme(mt);

                // Inner binding watches the hide flag for this module type.
                // Render rule:
                //   • this_focused                       → full (focus wins)
                //   • any other slot is focused          → collapsed
                //   • nothing focused, hide flag set     → collapsed
                //   • nothing focused, not hidden        → full
                // Empty slots are ALWAYS rendered as a slim placeholder tab
                // regardless of focus or hide flags. There is no body to
                // expand — adding a module is now done via the global
                // library sidebar (which auto-targets the focused empty
                // slot when one exists, falling back to first-empty).
                if mt == ModuleType::Empty {
                    build_empty_slot(cx, slot_idx);
                    return;
                }

                let hide_lens = Data::params.map(move |p| is_module_hidden(p, mt));
                Binding::new(cx, hide_lens, move |cx, hide_binding| {
                    let hidden = hide_binding.get(cx);
                    let render_full = if this_focused {
                        true
                    } else if any_focused {
                        false
                    } else {
                        !hidden
                    };
                    if render_full {
                        build_full_slot(cx, slot_idx, mt, theme);
                    } else {
                        build_collapsed_slot(cx, slot_idx, mt, theme);
                    }
                });
            },
        );
    });
}

/// Full expanded slot — module header, bypass LED, parameter controls.
/// The slot body itself is the drag source AND drop target (per VMR
/// convention — no separate `≡` handle). Vizia's `on_drag` fires when
/// the cursor leaves this view with LMB held; `on_drop` fires on a sibling
/// when MouseUp lands there with active `drop_data`.
fn build_full_slot(cx: &mut Context, slot_idx: usize, mt: ModuleType, theme: ModuleTheme) {
    VStack::new(cx, |cx| {
        // ── Module header (name + eject + hide + LED) ────────────────
        HStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, module_type_name(mt))
                    .class("module-name")
                    .color(theme.accent_color());
                Label::new(cx, module_type_subtitle(mt)).class("module-type");
            })
            .class("module-name-target")
            .toggle_class(
                "module-name-target-focused",
                Data::focused_slot.map(move |fs| *fs == Some(slot_idx)),
            )
            .height(Auto)
            .width(Stretch(1.0));

            if mt != ModuleType::Empty {
                build_eject_button(cx, slot_idx);
            }
            build_hide_button_for_type(cx, mt);
            build_led_indicator_for_type(cx, mt);
        })
        .class("module-header")
        .top(Pixels(0.0))
        .bottom(Pixels(0.0))
        .height(Auto)
        .width(Stretch(1.0))
        .gap(Pixels(6.0));

        build_bypass_button_for_type(cx, mt);
        build_controls_for_type(cx, mt, slot_idx);
    })
    .alignment(Alignment::TopLeft)
    .gap(Pixels(4.0))
    .class("module-slot")
    .class(theme.class_name())
    // Eligible-target class: lit on every slot OTHER than the drag source
    // while a drag is in flight. CSS pairs this with `:hover` to show the
    // active drop-target outline only on the slot the cursor is over.
    .toggle_class(
        "slot-eligible-target",
        Data::drag_source.map(move |ds| ds.is_some() && *ds != Some(slot_idx)),
    )
    // Source class: visual feedback that THIS slot is being dragged.
    .toggle_class(
        "slot-drag-source",
        Data::drag_source.map(move |ds| *ds == Some(slot_idx)),
    )
    // Live drop-position indicator: which third of THIS slot is the cursor
    // currently over? Drives the directional bar (left edge / full ring /
    // right edge) so the user previews swap-vs-insert before releasing.
    .toggle_class(
        "drop-pos-before",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Before))),
    )
    .toggle_class(
        "drop-pos-onto",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Onto))),
    )
    .toggle_class(
        "drop-pos-after",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::After))),
    )
    // Drag-source: vizia auto-marks Abilities::DRAGGABLE; the closure
    // fires the moment the cursor leaves this view with LMB held.
    .on_drag(move |ex| {
        ex.set_drop_data(ex.current());
        ex.emit(AppEvent::DragStarted(slot_idx));
        ex.emit(WindowEvent::SetCursor(CursorIcon::Grabbing));
    })
    // Per-slot live cursor track. Fires while the cursor is inside this
    // slot's bounds, including during a drag. We compute DropPos from
    // window-x relative to slot bounds and emit DragHover; the model
    // gates on `drag_source.is_some()` so events outside a drag are no-ops.
    .on_mouse_move(move |ex, x, _y| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(x, bounds);
        ex.emit(AppEvent::DragHover {
            target: slot_idx,
            position: pos,
        });
    })
    // Drop target: hit-test cursor X within this slot's bounds at release
    // time → DropPos. Position decides swap vs insert-before vs insert-after.
    .on_drop(move |ex, _data| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(ex.mouse().cursor_x, bounds);
        ex.emit(AppEvent::DropOnSlot {
            target: slot_idx,
            position: pos,
        });
        ex.emit(WindowEvent::SetCursor(CursorIcon::Default));
    })
    .border_color(theme.accent_color())
    .width(Data::zoom_level.map(|z| Pixels(BASE_SLOT_WIDTH_PX * (*z as f32) / 100.0)))
    .height(Stretch(1.0))
    .border_width(Pixels(3.0))
    .background_color(Color::rgb(42, 42, 42))
    .padding(Pixels(12.0));
}

/// Narrow collapsed tab — shows the 3-char module tag plus an expand button
/// that toggles the hide flag back to false. Width is fixed regardless of
/// zoom so several collapsed tabs stack neatly next to full slots.
fn build_collapsed_slot(cx: &mut Context, slot_idx: usize, mt: ModuleType, theme: ModuleTheme) {
    VStack::new(cx, |cx| {
        Label::new(cx, module_type_short_name(mt))
            .class("collapsed-name")
            .color(theme.accent_color());
        build_expand_button_for_type(cx, mt);
    })
    .alignment(Alignment::Center)
    .gap(Pixels(6.0))
    .class("module-slot")
    .class("slot-collapsed")
    .class(theme.class_name())
    .toggle_class(
        "slot-eligible-target",
        Data::drag_source.map(move |ds| ds.is_some() && *ds != Some(slot_idx)),
    )
    .toggle_class(
        "slot-drag-source",
        Data::drag_source.map(move |ds| *ds == Some(slot_idx)),
    )
    .toggle_class(
        "drop-pos-before",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Before))),
    )
    .toggle_class(
        "drop-pos-onto",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Onto))),
    )
    .toggle_class(
        "drop-pos-after",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::After))),
    )
    .on_drag(move |ex| {
        ex.set_drop_data(ex.current());
        ex.emit(AppEvent::DragStarted(slot_idx));
        ex.emit(WindowEvent::SetCursor(CursorIcon::Grabbing));
    })
    .on_mouse_move(move |ex, x, _y| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(x, bounds);
        ex.emit(AppEvent::DragHover {
            target: slot_idx,
            position: pos,
        });
    })
    .on_drop(move |ex, _data| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(ex.mouse().cursor_x, bounds);
        ex.emit(AppEvent::DropOnSlot {
            target: slot_idx,
            position: pos,
        });
        ex.emit(WindowEvent::SetCursor(CursorIcon::Default));
    })
    .border_color(theme.accent_color())
    .width(Pixels(56.0))
    .height(Stretch(1.0))
    .border_width(Pixels(3.0))
    .background_color(Color::rgb(42, 42, 42))
    .padding(Pixels(6.0));
}

/// Base slot width at 100% zoom, in logical pixels. All other zoom levels are
/// derived as `BASE_SLOT_WIDTH_PX * zoom_level / 100` via a reactive lens.
pub const BASE_SLOT_WIDTH_PX: f32 = 280.0;

// ============================================================================
// Bypass Buttons — dispatched by module type
// ============================================================================

fn build_led_indicator_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ => {
            ParamButton::new(cx, Data::params, |p| &p.eq_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::ButterComp2 => {
            ParamButton::new(cx, Data::params, |p| &p.comp_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::PultecEQ => {
            ParamButton::new(cx, Data::params, |p| &p.pultec_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::DynamicEQ => {
            #[cfg(feature = "dynamic_eq")]
            ParamButton::new(cx, Data::params, |p| &p.dyneq_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::Transformer => {
            ParamButton::new(cx, Data::params, |p| &p.transformer_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::Punch => {
            #[cfg(feature = "punch")]
            ParamButton::new(cx, Data::params, |p| &p.punch_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        ModuleType::Haas => {
            #[cfg(feature = "haas")]
            ParamButton::new(cx, Data::params, |p| &p.haas_bypass)
                .with_label("")
                .class("module-led-indicator");
        }
        // No LED for empty slots — there is nothing to indicate.
        ModuleType::Empty => {}
    }
}

fn build_bypass_button_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ => {
            components::create_active_led_button(cx, |p| &p.eq_bypass);
        }
        ModuleType::ButterComp2 => {
            components::create_active_led_button(cx, |p| &p.comp_bypass);
        }
        ModuleType::PultecEQ => {
            components::create_active_led_button(cx, |p| &p.pultec_bypass);
        }
        ModuleType::DynamicEQ => {
            #[cfg(feature = "dynamic_eq")]
            components::create_active_led_button(cx, |p| &p.dyneq_bypass);
        }
        ModuleType::Transformer => {
            components::create_active_led_button(cx, |p| &p.transformer_bypass);
        }
        ModuleType::Punch => {
            #[cfg(feature = "punch")]
            components::create_active_led_button(cx, |p| &p.punch_bypass);
        }
        ModuleType::Haas => {
            #[cfg(feature = "haas")]
            components::create_active_led_button(cx, |p| &p.haas_bypass);
        }
        // No bypass for empty slots — pass-through is unconditional.
        ModuleType::Empty => {}
    }
}

// ============================================================================
// Parameter Controls — dispatched by module type
// ============================================================================

fn build_controls_for_type(cx: &mut Context, mt: ModuleType, slot_idx: usize) {
    let _ = slot_idx;
    match mt {
        ModuleType::Api5500EQ => build_api5500_controls(cx),
        ModuleType::ButterComp2 => build_buttercomp2_controls(cx),
        ModuleType::PultecEQ => build_pultec_controls(cx),
        ModuleType::DynamicEQ => build_dynamic_eq_controls(cx),
        ModuleType::Transformer => build_transformer_controls(cx),
        ModuleType::Punch => build_punch_controls(cx),
        ModuleType::Haas => build_haas_controls(cx),
        // Empty slots short-circuit before reaching this dispatcher
        // (see create_dynamic_module_slot — Empty renders build_empty_slot
        // directly). This arm is unreachable in practice.
        ModuleType::Empty => {}
    }
}

// ============================================================================
// Empty slot placeholder — slim dashed tab, sole add path is the sidebar
// ============================================================================

/// Renders an empty rack slot as a narrow dashed-outline tab the same width
/// as a collapsed module tab. Adding a module is done via the global library
/// sidebar. Empty slots are valid drop targets — dropping a real module here
/// is the same as inserting between the surrounding slots.
fn build_empty_slot(cx: &mut Context, slot_idx: usize) {
    let theme = ModuleTheme::Empty;
    VStack::new(cx, |cx| {
        Label::new(cx, "+").class("empty-slot-glyph");
        Label::new(cx, format!("SLOT {}", slot_idx + 1).as_str()).class("empty-slot-label");
    })
    .alignment(Alignment::Center)
    .gap(Pixels(2.0))
    .class("module-slot")
    .class("slot-collapsed")
    .class("slot-empty")
    .class(theme.class_name())
    .toggle_class(
        "slot-eligible-target",
        Data::drag_source.map(move |ds| ds.is_some() && *ds != Some(slot_idx)),
    )
    .toggle_class(
        "drop-pos-before",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Before))),
    )
    .toggle_class(
        "drop-pos-onto",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::Onto))),
    )
    .toggle_class(
        "drop-pos-after",
        Data::drop_target.map(move |dt| *dt == Some((slot_idx, DropPos::After))),
    )
    .on_mouse_move(move |ex, x, _y| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(x, bounds);
        ex.emit(AppEvent::DragHover {
            target: slot_idx,
            position: pos,
        });
    })
    .on_drop(move |ex, _data| {
        let bounds = ex.bounds();
        let pos = hit_test_drop_pos(ex.mouse().cursor_x, bounds);
        ex.emit(AppEvent::DropOnSlot {
            target: slot_idx,
            position: pos,
        });
        ex.emit(WindowEvent::SetCursor(CursorIcon::Default));
    })
    .border_color(theme.accent_color())
    .width(Pixels(56.0))
    .height(Stretch(1.0))
    .border_width(Pixels(2.0))
    .background_color(Color::rgb(36, 36, 38))
    .padding(Pixels(6.0));
}

fn build_api5500_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        // ── Shelf bands: LF and HF side-by-side ──────────────────────────────
        HStack::new(cx, |cx| {
            // Left: LF low shelf
            VStack::new(cx, |cx| {
                Label::new(cx, "LF SHELF")
                    .class("section-label")
                    .height(Pixels(16.0))
                    .width(Stretch(1.0));
                components::create_frequency_slider(cx, "FREQ", Data::params, |p| &p.lf_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lf_gain);
            })
            .gap(Pixels(4.0))
            .height(Auto)
            .width(Stretch(1.0))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0));

            // Right: HF high shelf
            VStack::new(cx, |cx| {
                Label::new(cx, "HF SHELF")
                    .class("section-label")
                    .height(Pixels(16.0))
                    .width(Stretch(1.0));
                components::create_frequency_slider(cx, "FREQ", Data::params, |p| &p.hf_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.hf_gain);
            })
            .gap(Pixels(4.0))
            .height(Auto)
            .width(Stretch(1.0))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0));
        })
        .gap(Pixels(8.0))
        .height(Auto)
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));

        // ── Parametric bands: LMF → MF → HMF (low to high) ──────────────────
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "LMF", Data::params, |p| &p.lmf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lmf_gain);
            components::create_param_slider(cx, "Q", Data::params, |p| &p.lmf_q);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "MF", Data::params, |p| &p.mf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.mf_gain);
            components::create_param_slider(cx, "Q", Data::params, |p| &p.mf_q);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "HMF", Data::params, |p| &p.hmf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.hmf_gain);
            components::create_param_slider(cx, "Q", Data::params, |p| &p.hmf_q);
        });
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_buttercomp2_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        // Model selector — always visible above the reactive control surface.
        #[cfg(feature = "buttercomp2")]
        components::create_param_slider(cx, "MODEL", Data::params, |p| &p.comp_model);

        // Reactive control surface — rebuilds when model enum changes.
        // Map the EnumParam value to usize so Binding gets a `Data`-implementing target.
        #[cfg(feature = "buttercomp2")]
        Binding::new(
            cx,
            Data::params.map(|p| p.comp_model.value() as usize),
            |cx, model_lens| {
                let model_idx = model_lens.get(cx);
                match model_idx {
                    1 => build_optical_controls(cx), // ButterComp2Model::Optical as usize == 1
                    2 => build_vca_controls(cx),     // ButterComp2Model::Vca    as usize == 2
                    3 => build_fet_controls(cx),     // ButterComp2Model::Fet    as usize == 3
                    _ => build_classic_controls(cx), // 0 = Classic; also safe fallback
                }
            },
        );

        // Fallback when buttercomp2 feature is disabled — render classic controls directly.
        #[cfg(not(feature = "buttercomp2"))]
        build_classic_controls(cx);
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// Classic ButterComp2 control surface — Compress, Output, SC HP, Dry/Wet.
fn build_classic_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::create_ratio_slider(cx, "COMPRESS", Data::params, |p| &p.comp_compress);
        components::create_gain_slider(cx, "OUTPUT", Data::params, |p| &p.comp_output);
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "SC HP", Data::params, |p| &p.comp_sc_hp_freq);
            components::create_param_slider(cx, "DRY/WET", Data::params, |p| &p.comp_dry_wet);
        });
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// VCA model control surface — Threshold, Ratio, Attack, Release, Mix.
fn build_vca_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "THRESH", Data::params, |p| &p.vca_thresh);
            components::create_ratio_slider(cx, "RATIO", Data::params, |p| &p.vca_ratio);
        });
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "ATTACK", Data::params, |p| &p.vca_atk);
            components::create_param_slider(cx, "RELEASE", Data::params, |p| &p.vca_rel);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "SC HP", Data::params, |p| &p.comp_sc_hp_freq);
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.comp_dry_wet);
        });
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// Optical model control surface — Threshold, Character, Speed, SC HP, Mix.
fn build_optical_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "THRESH", Data::params, |p| &p.opt_thresh);
            components::create_param_slider(cx, "CHAR %", Data::params, |p| &p.opt_char);
        });
        components::create_param_slider(cx, "SPEED", Data::params, |p| &p.opt_speed);
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "SC HP", Data::params, |p| &p.comp_sc_hp_freq);
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.comp_dry_wet);
        });
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// 1176-style FET compressor control surface — Input, Output, Attack, Release, Ratio, Auto-Release, Mix.
#[cfg(feature = "buttercomp2")]
fn build_fet_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::module_row(cx, |cx| {
            components::create_gain_slider(cx, "INPUT", Data::params, |p| &p.fet_input_db);
            components::create_gain_slider(cx, "OUTPUT", Data::params, |p| &p.fet_output_db);
        });
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "ATTACK", Data::params, |p| &p.fet_attack_ms);
            components::create_param_slider(cx, "RELEASE", Data::params, |p| &p.fet_release_ms);
        });
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "RATIO", Data::params, |p| &p.fet_ratio);
            components::create_bool_button(cx, "AUTO REL", Data::params, |p| &p.fet_auto_release);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "SC HP", Data::params, |p| &p.comp_sc_hp_freq);
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.comp_dry_wet);
        });
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_pultec_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        // LOW FREQUENCY: boost freq/gain on top row, independent cut
        // freq/gain on bottom row. Independent cut freq enables the classic
        // EQP-1A boost+cut trick (boost at 60 Hz, cut at 200 Hz → tight lows).
        components::module_section(cx, "LOW FREQUENCY", |cx| {
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "FREQ", Data::params, |p| {
                    &p.pultec_lf_boost_freq
                });
                components::create_gain_slider(cx, "BOOST", Data::params, |p| {
                    &p.pultec_lf_boost_gain
                });
                components::create_param_slider(cx, "BW", Data::params, |p| {
                    &p.pultec_lf_boost_bandwidth
                });
            });
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "ATTEN", Data::params, |p| {
                    &p.pultec_lf_cut_freq
                });
                components::create_gain_slider(cx, "ATTEN", Data::params, |p| {
                    &p.pultec_lf_cut_gain
                });
                components::create_param_slider(cx, "BW", Data::params, |p| {
                    &p.pultec_lf_cut_bandwidth
                });
            });
        });
        // HIGH FREQUENCY: boost and cut each on their own row (freq + gain/bw)
        components::module_section(cx, "HIGH FREQUENCY", |cx| {
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "FREQ", Data::params, |p| {
                    &p.pultec_hf_boost_freq
                });
                components::create_gain_slider(cx, "BOOST", Data::params, |p| {
                    &p.pultec_hf_boost_gain
                });
                components::create_param_slider(cx, "BW", Data::params, |p| {
                    &p.pultec_hf_boost_bandwidth
                });
            });
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "ATTEN", Data::params, |p| {
                    &p.pultec_hf_cut_freq
                });
                components::create_gain_slider(cx, "ATTEN", Data::params, |p| {
                    &p.pultec_hf_cut_gain
                });
            });
        });
        // OUTPUT: tube drive separate from the EQ bands
        components::module_section(cx, "OUTPUT", |cx| {
            components::create_param_slider(cx, "TUBE DRIVE", Data::params, |p| {
                &p.pultec_tube_drive
            });
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// Compact DynEQ card shown inside the strip slot.
/// All 4 bands are edited in the full back view — click OPEN to flip.
fn build_dynamic_eq_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "4-band dynamic equalizer")
            .class("dyneq-card-hint")
            .height(Pixels(16.0))
            .width(Stretch(1.0));
        Label::new(
            cx,
            "Real-time frequency-dependent compression with per-band threshold control",
        )
        .class("dyneq-card-desc")
        .height(Auto)
        .width(Stretch(1.0));
        // OPEN button — flips to the full DynEQ back view.
        // Uses Button::new (not VStack) so the full 40px hit area is reliably clickable;
        // VStack + on_press can have dead zones where child labels shadow parent events.
        Button::new(cx, |cx| {
            Label::new(cx, "OPEN EDITOR  \u{25B6}")
                .class("dyneq-open-label")
                .width(Stretch(1.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0))
        })
        .class("dyneq-open-btn")
        .on_press(|cx| cx.emit(AppEvent::OpenDynEq))
        .cursor(CursorIcon::Hand)
        .height(Pixels(40.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
    })
    .gap(Pixels(10.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

// ============================================================================
// Spectrum Canvas — real-time lock-free spectrum display
// ============================================================================

/// Reads magnitude bins from the audio thread's lock-free `SpectrumData` and
/// redraws each frame. Also overlays the sidechain masking analysis when available.
/// Both `display_bins` and `display_overlap` are GUI-thread-only RefCells.
struct SpectrumCanvas {
    spectrum_data: Arc<spectral::SpectrumData>,
    display_bins: RefCell<Vec<f32>>,
    analysis_result: Arc<spectral::AnalysisResult>,
    display_overlap: RefCell<Vec<f32>>,
    gr_data: Arc<spectral::GainReductionData>,
}

impl SpectrumCanvas {
    fn new(
        cx: &mut Context,
        spectrum_data: Arc<spectral::SpectrumData>,
        analysis_result: Arc<spectral::AnalysisResult>,
        gr_data: Arc<spectral::GainReductionData>,
    ) -> Handle<'_, Self> {
        Self {
            spectrum_data,
            display_bins: RefCell::new(vec![0.0_f32; spectral::SPECTRUM_BINS]),
            analysis_result,
            display_overlap: RefCell::new(vec![0.0_f32; spectral::SPECTRUM_BINS]),
            gr_data,
        }
        .build(cx, |_cx| {})
    }
}

impl View for SpectrumCanvas {
    fn element(&self) -> Option<&'static str> {
        Some("spectrum-canvas")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        use vizia_plug::vizia::vg;

        // Early-out when the canvas is hidden (display:none gives zero bounds).
        // Without this guard, cx.needs_redraw() at the end would spin the render loop
        // at 60 fps even when the DynEQ view is closed, competing with event processing
        // for all other interactions (model switching, drag handles, etc.).
        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h < 1.0 {
            return;
        }

        // Pull latest audio-thread data. Returns true if new bins arrived this frame.
        let has_new_data = {
            let mut bins = self.display_bins.borrow_mut();
            self.spectrum_data.read_into_slice(&mut bins)
        };
        // Pull overlap bins from the last analysis (Relaxed — display-only, staleness is fine).
        {
            let mut overlap = self.display_overlap.borrow_mut();
            for (i, slot) in self
                .analysis_result
                .overlap_bins
                .iter()
                .enumerate()
                .take(spectral::SPECTRUM_BINS)
            {
                overlap[i] = f32::from_bits(slot.load(Ordering::Relaxed));
            }
        }

        let bins = self.display_bins.borrow();
        let overlap = self.display_overlap.borrow();

        // ── Background ──────────────────────────────────────────────────────
        let bg_rect = vg::Rect::from_xywh(bounds.x, bounds.y, bounds.w, bounds.h);
        let mut bg_paint = vg::Paint::default();
        bg_paint.set_color(vg::Color::from_argb(255, 18, 25, 31));
        bg_paint.set_style(vg::PaintStyle::Fill);
        canvas.draw_rect(bg_rect, &bg_paint);

        let n = bins.len();
        if n == 0 {
            // No data yet — request one more frame in case audio starts soon.
            cx.needs_redraw();
            return;
        }

        let x_step = bounds.w / n as f32;

        // ── Band crossover visualization ──────────────────────────────────────
        // The spectrum covers 0..sample_rate/4 Hz across SPECTRUM_BINS bins.
        // At the 44.1 kHz reference: 512 bins = 11025 Hz.
        // x_frac = freq / 11025.0  (visual guide only — acceptable approximation).
        const SPECTRUM_TOP_HZ: f32 = 11025.0;
        const CROSSOVER_HZ: [f32; 3] = [500.0, 2000.0, 6000.0];
        // Band colors: LOW=green, LOW-MID=sky-blue, HIGH-MID=purple, HIGH=amber
        const BAND_ARGB: [(u8, u8, u8, u8); 4] = [
            (45, 80, 200, 110), // band1 LOW      — green
            (45, 60, 150, 220), // band2 LOW MID  — sky blue
            (45, 150, 90, 220), // band3 HIGH MID — purple
            (45, 220, 150, 50), // band4 HIGH     — amber
        ];

        let cx_frac: [f32; 3] = CROSSOVER_HZ.map(|f| (f / SPECTRUM_TOP_HZ).clamp(0.0, 1.0));
        let cx_x: [f32; 3] = cx_frac.map(|fr| bounds.x + fr * bounds.w);

        let band_left = [bounds.x, cx_x[0], cx_x[1], cx_x[2]];
        let band_right = [cx_x[0], cx_x[1], cx_x[2], bounds.x + bounds.w];

        // Read per-band gain reduction (Relaxed — display only, staleness fine).
        let gr_db: [f32; 4] = [
            f32::from_bits(self.gr_data.bands[0].load(Ordering::Relaxed)),
            f32::from_bits(self.gr_data.bands[1].load(Ordering::Relaxed)),
            f32::from_bits(self.gr_data.bands[2].load(Ordering::Relaxed)),
            f32::from_bits(self.gr_data.bands[3].load(Ordering::Relaxed)),
        ];

        // Draw semi-transparent band background tints + GR bars at the top.
        const MAX_GR_DB: f32 = 24.0;
        const MAX_BAR_H: f32 = 18.0;
        for b in 0..4_usize {
            let (a, r, g, bl) = BAND_ARGB[b];
            let band_w = band_right[b] - band_left[b];

            // Subtle background tint for the band region.
            let mut tint = vg::Paint::default();
            tint.set_color(vg::Color::from_argb(a, r, g, bl));
            tint.set_style(vg::PaintStyle::Fill);
            canvas.draw_rect(
                vg::Rect::from_xywh(band_left[b], bounds.y, band_w, bounds.h),
                &tint,
            );

            // GR bar: height proportional to gain reduction amount.
            let gr = gr_db[b].clamp(0.0, MAX_GR_DB);
            if gr > 0.1 {
                let bar_h = (gr / MAX_GR_DB) * MAX_BAR_H;
                let mut gr_paint = vg::Paint::default();
                gr_paint.set_color(vg::Color::from_argb(200, r, g, bl));
                gr_paint.set_style(vg::PaintStyle::Fill);
                canvas.draw_rect(
                    vg::Rect::from_xywh(band_left[b], bounds.y, band_w, bar_h),
                    &gr_paint,
                );
            }
        }

        // Draw vertical crossover lines between bands.
        let mut line_paint = vg::Paint::default();
        line_paint.set_color(vg::Color::from_argb(120, 220, 220, 220));
        line_paint.set_style(vg::PaintStyle::Stroke);
        line_paint.set_stroke_width(1.0);
        line_paint.set_anti_alias(false);
        for &cx_px in &cx_x {
            let mut vline = vg::Path::new();
            vline.move_to((cx_px, bounds.y));
            vline.line_to((cx_px, bounds.y + bounds.h));
            canvas.draw_path(&vline, &line_paint);
        }

        // ── Overlap overlay (orange) — drawn below the main spectrum fill ──
        // Normalise to the peak overlap value so relative masking is always visible.
        let max_overlap = overlap
            .iter()
            .cloned()
            .fold(0.0_f32, f32::max)
            .max(f32::MIN_POSITIVE);
        if max_overlap > f32::MIN_POSITIVE * 2.0 {
            let mut ovl_path = vg::Path::new();
            let mut ovl_started = false;
            for (i, &ov) in overlap.iter().enumerate() {
                let norm = (ov / max_overlap).clamp(0.0, 1.0);
                let x = bounds.x + i as f32 * x_step;
                let y = bounds.y + bounds.h - norm * bounds.h;
                if !ovl_started {
                    ovl_path.move_to((x, y));
                    ovl_started = true;
                } else {
                    ovl_path.line_to((x, y));
                }
            }
            if ovl_started {
                ovl_path.line_to((bounds.x + bounds.w, bounds.y + bounds.h));
                ovl_path.line_to((bounds.x, bounds.y + bounds.h));
                ovl_path.close();
                let mut ovl_paint = vg::Paint::default();
                // Semi-transparent orange — stands out clearly against the teal spectrum.
                ovl_paint.set_color(vg::Color::from_argb(90, 255, 110, 20));
                ovl_paint.set_style(vg::PaintStyle::Fill);
                ovl_paint.set_anti_alias(true);
                canvas.draw_path(&ovl_path, &ovl_paint);
            }
        }

        // ── Spectrum filled area (dBFS: −90 dB → bottom, 0 dB → top) ─────
        let mut fill = vg::Path::new();
        let mut started = false;
        for (i, &mag) in bins.iter().enumerate() {
            let db = 20.0 * mag.max(1e-9_f32).log10();
            let norm = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
            let x = bounds.x + i as f32 * x_step;
            let y = bounds.y + bounds.h - norm * bounds.h;
            if !started {
                fill.move_to((x, y));
                started = true;
            } else {
                fill.line_to((x, y));
            }
        }
        fill.line_to((bounds.x + bounds.w, bounds.y + bounds.h));
        fill.line_to((bounds.x, bounds.y + bounds.h));
        fill.close();
        let mut fill_paint = vg::Paint::default();
        fill_paint.set_color(vg::Color::from_argb(60, 50, 180, 150));
        fill_paint.set_style(vg::PaintStyle::Fill);
        fill_paint.set_anti_alias(true);
        canvas.draw_path(&fill, &fill_paint);

        // ── Stroke line ──────────────────────────────────────────────────────
        let mut line = vg::Path::new();
        let mut started = false;
        for (i, &mag) in bins.iter().enumerate() {
            let db = 20.0 * mag.max(1e-9_f32).log10();
            let norm = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
            let x = bounds.x + i as f32 * x_step;
            let y = bounds.y + bounds.h - norm * bounds.h;
            if !started {
                line.move_to((x, y));
                started = true;
            } else {
                line.line_to((x, y));
            }
        }
        let mut stroke_paint = vg::Paint::default();
        stroke_paint.set_color(vg::Color::from_argb(200, 80, 220, 180));
        stroke_paint.set_style(vg::PaintStyle::Stroke);
        stroke_paint.set_stroke_width(1.5);
        stroke_paint.set_anti_alias(true);
        canvas.draw_path(&line, &stroke_paint);

        // Always request the next frame when visible. The bounds guard above prevents
        // redraws when hidden. The has_new_data flag only tells us if the audio thread
        // wrote this frame — but skipping redraws on false would permanently stall the
        // spectrum if the GUI polls faster than the audio thread writes (which happens
        // regularly at 60fps vs ~86 buffers/sec with variable timing).
        let _ = has_new_data;
        cx.needs_redraw();
    }
}

// ============================================================================
// DynEQ Band Column — macro-based component
// ============================================================================
//
// Each of the 4 band columns has identical layout (title, ON/SOLO, 8 sliders)
// differing only in which parameter field is accessed. Because each closure
// `|p| &p.dyneq_band1_freq` is a distinct concrete type, we cannot unify them
// through generics without 10 type parameters. A macro gives us a single layout
// definition that expands per band at compile time.
//
// Dynamic spacing: the band column VStack is height(Stretch(1.0)) and fills
// the remaining height in the back view after the header and spectrum canvas.
// Each child uses top(Stretch(1.0)) so available space is distributed evenly
// above each item — controls breathe when the window is tall and compress when
// it is short, never clipping. This is morphorm's equivalent of CSS
// `justify-content: space-around` on a fixed-height flex column.
//
// dyneq_slider! inlines a compact (13px label / 16px slider) param row without
// the fixed top/bottom Pixels(0.0) that shared helpers impose. It shares the
// same .param-control class for hover styling but uses .dyneq-param-label for
// the smaller font.
//
// Usage:
//   dyneq_band_col!(cx, "BAND N — NAME",
//       band_N_enabled, band_N_solo,
//       band_N_freq, band_N_threshold, band_N_ratio,
//       band_N_q, band_N_mode, band_N_attack, band_N_release, band_N_gain);
macro_rules! dyneq_slider {
    ($cx:expr, $label:literal, $pf:expr) => {{
        VStack::new($cx, |cx| {
            Label::new(cx, $label)
                .class("dyneq-param-label")
                .height(Pixels(13.0))
                .width(Stretch(1.0));
            ParamSlider::new(cx, Data::params, $pf)
                .height(Pixels(16.0))
                .width(Stretch(1.0));
        })
        .class("param-control")
        .width(Stretch(1.0))
        .height(Auto)
        // top(Stretch) distributes free space above this item.
        // bottom(Pixels(0)) avoids double-counting (adjacent tops handle the gap).
        .top(Stretch(1.0))
        .bottom(Pixels(0.0))
    }};
}

macro_rules! dyneq_band_col {
    ($cx:expr, $title:literal,
     $enabled:ident, $solo:ident,
     $freq:ident, $thresh:ident, $ratio:ident,
     $q:ident, $mode:ident, $atk:ident, $rel:ident, $gain:ident,
     $band_idx:literal) => {
        VStack::new($cx, |cx| {
            // Band header: title + ON/SOLO buttons + chevron expand toggle
            HStack::new(cx, |cx| {
                Label::new(cx, $title)
                    .class("dyneq-band-title")
                    .height(Pixels(14.0))
                    .width(Stretch(1.0))
                    .top(Pixels(0.0))
                    .bottom(Pixels(0.0));
                components::create_on_button(cx, |p| &p.$enabled);
                components::create_bypass_button(cx, "SOLO", |p| &p.$solo);
                // Chevron toggle button — reactive label via dyneq_expand_gen lens
                {
                    let expand_arc_chevron = cx.data::<Data>().unwrap().dyneq_band_expand.clone();
                    Button::new(cx, |cx| {
                        Label::new(
                            cx,
                            Data::dyneq_expand_gen.map(move |_| {
                                if expand_arc_chevron[$band_idx].load(Ordering::Relaxed) {
                                    "▼"
                                } else {
                                    "▶"
                                }
                            }),
                        )
                    })
                    .on_press(|cx| cx.emit(AppEvent::ToggleDynEQBand($band_idx)))
                    .class("dyneq-chevron")
                    .width(Pixels(24.0))
                    .height(Auto)
                    .top(Pixels(0.0))
                    .bottom(Pixels(0.0));
                }
            })
            .top(Stretch(1.0))
            .bottom(Pixels(0.0))
            .width(Stretch(1.0))
            .height(Auto);

            // Tier 1 — always visible: MODE, FREQ, THRESH, GAIN
            dyneq_slider!(cx, "MODE", |p| &p.$mode);
            dyneq_slider!(cx, "FREQ", |p| &p.$freq);
            dyneq_slider!(cx, "THRESH", |p| &p.$thresh);
            dyneq_slider!(cx, "GAIN", |p| &p.$gain);

            // Tier 2 — conditionally built when band is expanded.
            // Uses Binding::new rather than .display() because .display(lens.map(...))
            // reliably shows elements but does not reliably re-hide them in vizia 0.3.
            // Binding destroys and rebuilds its subtree on every gen change, guaranteeing
            // the correct show/hide state in both directions.
            {
                let expand_arc_tier2 = cx.data::<Data>().unwrap().dyneq_band_expand.clone();
                Binding::new(cx, Data::dyneq_expand_gen, move |cx, _gen| {
                    if expand_arc_tier2[$band_idx].load(Ordering::Relaxed) {
                        VStack::new(cx, |cx| {
                            dyneq_slider!(cx, "RATIO", |p| &p.$ratio);
                            dyneq_slider!(cx, "Q", |p| &p.$q);
                            dyneq_slider!(cx, "ATK ms", |p| &p.$atk);
                            dyneq_slider!(cx, "REL ms", |p| &p.$rel);
                        })
                        .width(Stretch(1.0))
                        .height(Auto)
                        .top(Pixels(0.0))
                        .bottom(Pixels(0.0));
                    }
                });
            }
        })
        .class("dyneq-band-col")
        // Stretch(1.0): band column fills remaining height after header + spectrum.
        // No gap needed — spacing is entirely from top(Stretch(1.0)) on children.
        .height(Stretch(1.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0))
    };
}

// ============================================================================

/// Full-screen DynEQ back view: real-time spectral analyzer + horizontal 4-band editor.
/// Clicking "◀ STRIP VIEW" flips back to the front.
fn build_dyneq_back_view(
    cx: &mut Context,
    spectrum_data: Arc<spectral::SpectrumData>,
    analysis_result: Arc<spectral::AnalysisResult>,
    gr_data: Arc<spectral::GainReductionData>,
) {
    VStack::new(cx, |cx| {
        // ── Back-view header ──────────────────────────────────────────────────
        HStack::new(cx, |cx| {
            // Back button
            VStack::new(cx, |cx| {
                Label::new(cx, "\u{25C0} STRIP VIEW")
                    .class("dyneq-back-btn-label")
                    .height(Pixels(16.0))
                    .width(Stretch(1.0));
            })
            .class("dyneq-back-btn")
            .on_press(|cx| cx.emit(AppEvent::CloseDynEq))
            .cursor(CursorIcon::Hand)
            .height(Pixels(32.0))
            .width(Pixels(140.0))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0));

            Label::new(cx, "DYNAMIC EQ")
                .class("dyneq-back-title")
                .height(Pixels(28.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));

            #[cfg(feature = "dynamic_eq")]
            components::create_bypass_button(cx, "BYPASS", |p| &p.dyneq_bypass);

            // ── Sidechain masking analysis controls ──────────────────────────
            // ANALYZE: arms the audio thread to run one analysis on the next FFT frame.
            // APPLY:   reads the last result and programs the appropriate DynEQ band.
            // Both buttons are always visible; APPLY is a no-op if no analysis exists.
            #[cfg(feature = "dynamic_eq")]
            {
                let ar_clone = analysis_result.clone();
                VStack::new(cx, |cx| {
                    Label::new(cx, "ANALYZE SC")
                        .class("dyneq-auto-btn-label")
                        .height(Pixels(14.0))
                        .width(Stretch(1.0));
                })
                .class("dyneq-auto-btn")
                .on_press(|cx| cx.emit(AppEvent::RequestAnalysis))
                .cursor(CursorIcon::Hand)
                .height(Pixels(32.0))
                .width(Pixels(110.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));

                VStack::new(cx, |cx| {
                    Label::new(cx, "APPLY RESULT")
                        .class("dyneq-apply-btn-label")
                        .height(Pixels(14.0))
                        .width(Stretch(1.0));
                })
                .class("dyneq-apply-btn")
                .on_press(move |cx| {
                    if ar_clone.ready.load(Ordering::Acquire) {
                        let band = ar_clone.target_band.load(Ordering::Relaxed);
                        let freq = f32::from_bits(ar_clone.target_freq.load(Ordering::Relaxed));
                        let threshold_db =
                            f32::from_bits(ar_clone.target_threshold_db.load(Ordering::Relaxed));
                        cx.emit(AppEvent::ApplyAnalysis {
                            band,
                            freq,
                            threshold_db,
                        });
                    }
                })
                .cursor(CursorIcon::Hand)
                .height(Pixels(32.0))
                .width(Pixels(120.0))
                .top(Pixels(0.0))
                .bottom(Pixels(0.0));
            }
        })
        .height(Auto)
        .width(Stretch(1.0))
        .gap(Pixels(12.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));

        // ── Real-time spectral analyzer with masking overlay ──────────────────
        // Uses Stretch so the canvas grows with the back-view container as the
        // plugin window is resized by the host. SpectrumCanvas::draw already
        // reads cx.bounds() every frame, so no additional plumbing is needed.
        // min_height guards against the canvas disappearing on very short
        // windows.
        SpectrumCanvas::new(cx, spectrum_data, analysis_result, gr_data)
            .class("dyneq-spectrum")
            .height(Stretch(2.0))
            .min_height(Pixels(180.0))
            .width(Stretch(1.0))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0));

        // ── 4-band horizontal editor ──────────────────────────────────────────
        #[cfg(feature = "dynamic_eq")]
        // height(Stretch(1.0)): HStack fills remaining back-view height after
        // the header row and spectrum canvas, giving band columns a concrete
        // height to stretch into for dynamic spacing to work.
        HStack::new(cx, |cx| {
            dyneq_band_col!(
                cx,
                "BAND 1 — LOW",
                dyneq_band1_enabled,
                dyneq_band1_solo,
                dyneq_band1_freq,
                dyneq_band1_threshold,
                dyneq_band1_ratio,
                dyneq_band1_q,
                dyneq_band1_mode,
                dyneq_band1_attack,
                dyneq_band1_release,
                dyneq_band1_gain,
                0
            );

            dyneq_band_col!(
                cx,
                "BAND 2 — LOW MID",
                dyneq_band2_enabled,
                dyneq_band2_solo,
                dyneq_band2_freq,
                dyneq_band2_threshold,
                dyneq_band2_ratio,
                dyneq_band2_q,
                dyneq_band2_mode,
                dyneq_band2_attack,
                dyneq_band2_release,
                dyneq_band2_gain,
                1
            );

            dyneq_band_col!(
                cx,
                "BAND 3 — HIGH MID",
                dyneq_band3_enabled,
                dyneq_band3_solo,
                dyneq_band3_freq,
                dyneq_band3_threshold,
                dyneq_band3_ratio,
                dyneq_band3_q,
                dyneq_band3_mode,
                dyneq_band3_attack,
                dyneq_band3_release,
                dyneq_band3_gain,
                2
            );

            dyneq_band_col!(
                cx,
                "BAND 4 — HIGH",
                dyneq_band4_enabled,
                dyneq_band4_solo,
                dyneq_band4_freq,
                dyneq_band4_threshold,
                dyneq_band4_ratio,
                dyneq_band4_q,
                dyneq_band4_mode,
                dyneq_band4_attack,
                dyneq_band4_release,
                dyneq_band4_gain,
                3
            );
        })
        .height(Stretch(1.0))
        .width(Stretch(1.0))
        .gap(Pixels(12.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));

        #[cfg(not(feature = "dynamic_eq"))]
        Label::new(cx, "enable dynamic_eq feature to use this module").class("module-type");
    })
    .class("dyneq-back-view")
    .height(Stretch(1.0))
    .width(Stretch(1.0))
    .gap(Pixels(12.0))
    .padding(Pixels(16.0))
    .display(Data::dyneq_open.map(|o| if *o { Display::Flex } else { Display::None }));
}

fn build_transformer_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        // Model + compression on one row
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "MODEL", Data::params, |p| &p.transformer_model);
            components::create_ratio_slider(cx, "COMP", Data::params, |p| {
                &p.transformer_compression
            });
        });
        // Input stage: drive + saturation paired
        components::module_section(cx, "INPUT", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "DRIVE", Data::params, |p| {
                    &p.transformer_input_drive
                });
                components::create_param_slider(cx, "SAT", Data::params, |p| {
                    &p.transformer_input_saturation
                });
            });
        });
        // Output stage: drive + saturation paired
        components::module_section(cx, "OUTPUT", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "DRIVE", Data::params, |p| {
                    &p.transformer_output_drive
                });
                components::create_param_slider(cx, "SAT", Data::params, |p| {
                    &p.transformer_output_saturation
                });
            });
        });
        // Tone shaping: low/high response
        components::module_section(cx, "TONE", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "LOW", Data::params, |p| {
                    &p.transformer_low_response
                });
                components::create_param_slider(cx, "HIGH", Data::params, |p| {
                    &p.transformer_high_response
                });
            });
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_punch_controls(cx: &mut Context) {
    #[cfg(feature = "punch")]
    VStack::new(cx, |cx| {
        components::module_section(cx, "CLIPPER", |cx| {
            components::module_row(cx, |cx| {
                components::create_gain_slider(cx, "THRESH", Data::params, |p| &p.punch_threshold);
                components::create_param_slider(cx, "MODE", Data::params, |p| &p.punch_clip_mode);
            });
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "SOFT", Data::params, |p| &p.punch_softness);
                components::create_param_slider(cx, "OVSMP", Data::params, |p| {
                    &p.punch_oversampling
                });
            });
        });
        components::module_section(cx, "TRANSIENTS", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "ATTACK", Data::params, |p| &p.punch_attack);
                components::create_param_slider(cx, "SUSTAIN", Data::params, |p| &p.punch_sustain);
            });
            components::create_param_slider(cx, "SENS", Data::params, |p| &p.punch_sensitivity);
        });
        components::module_section(cx, "OUTPUT", |cx| {
            components::module_row(cx, |cx| {
                components::create_gain_slider(cx, "IN", Data::params, |p| &p.punch_input_gain);
                components::create_gain_slider(cx, "OUT", Data::params, |p| &p.punch_output_gain);
            });
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "MIX", Data::params, |p| &p.punch_mix);
                components::create_frequency_slider(cx, "WET HPF", Data::params, |p| {
                    &p.punch_wet_hpf_hz
                });
            });
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_haas_controls(cx: &mut Context) {
    #[cfg(feature = "haas")]
    VStack::new(cx, |cx| {
        components::module_section(cx, "M/S GAIN", |cx| {
            components::module_row(cx, |cx| {
                components::create_gain_slider(cx, "MID", Data::params, |p| &p.haas_mid_gain);
                components::create_gain_slider(cx, "SIDE", Data::params, |p| &p.haas_side_gain);
            });
        });
        components::module_section(cx, "COMB", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "DEPTH", Data::params, |p| &p.haas_comb_depth);
                components::create_param_slider(cx, "TIME", Data::params, |p| &p.haas_comb_time);
            });
            components::create_param_slider(cx, "MODE", Data::params, |p| &p.haas_comb_mode);
        });
        components::module_section(cx, "OUTPUT", |cx| {
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.haas_mix);
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}
