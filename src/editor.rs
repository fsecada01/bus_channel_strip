// src/editor.rs
// Vizia GUI implementation for Bus Channel Strip

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::RefCell;
use nih_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use vizia_plug::widgets::{ParamSlider, RawParamEvent};

use crate::{BusChannelStripParams, ModuleType};
use crate::spectral;
use crate::components::{self, ModuleTheme};
use crate::styles::COMPONENT_STYLES;

// ============================================================================
// App Events
// ============================================================================

/// Click-to-select/swap module reordering events.
/// - First click on a slot: selects it (highlights it).
/// - Click on a different slot: swaps the two modules, clears selection.
/// - Click the same slot again: cancels the selection.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    SelectSlot(usize),
    /// Open the full-screen DynEQ back view.
    OpenDynEq,
    /// Return from DynEQ back view to the strip front view.
    CloseDynEq,
    /// Request a one-shot sidechain masking analysis from the audio thread.
    #[cfg(feature = "dynamic_eq")]
    RequestAnalysis,
    /// Apply analysis results to the appropriate DynEQ band parameters.
    #[cfg(feature = "dynamic_eq")]
    ApplyAnalysis { band: u32, freq: f32, threshold_db: f32 },
}

// ============================================================================
// Editor Data Model
// ============================================================================

#[derive(Lens)]
pub struct Data {
    pub params: Arc<BusChannelStripParams>,
    /// The slot index currently selected for swapping, or None.
    pub drag_slot: Option<usize>,
    /// When true, the DynEQ back view is shown instead of the strip.
    pub dyneq_open: bool,
    /// Shared with the audio thread — GUI sets true to trigger a masking analysis.
    pub analysis_requested: Arc<AtomicBool>,
    /// Shared with the audio thread — read after analysis completes.
    pub analysis_result: Arc<spectral::AnalysisResult>,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e: &AppEvent, _| match e {
            AppEvent::OpenDynEq  => { self.dyneq_open = true;  }
            AppEvent::CloseDynEq => { self.dyneq_open = false; }

            #[cfg(feature = "dynamic_eq")]
            AppEvent::RequestAnalysis => {
                self.analysis_requested.store(true, Ordering::Relaxed);
            }

            #[cfg(feature = "dynamic_eq")]
            AppEvent::ApplyAnalysis { band, freq, threshold_db } => {
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
                let freq_norm   = unsafe { freq_ptr.preview_normalized(*freq) };
                let thresh_norm = unsafe { thresh_ptr.preview_normalized(*threshold_db) };

                cx.emit(RawParamEvent::BeginSetParameter(freq_ptr));
                cx.emit(RawParamEvent::SetParameterNormalized(freq_ptr, freq_norm));
                cx.emit(RawParamEvent::EndSetParameter(freq_ptr));

                cx.emit(RawParamEvent::BeginSetParameter(thresh_ptr));
                cx.emit(RawParamEvent::SetParameterNormalized(thresh_ptr, thresh_norm));
                cx.emit(RawParamEvent::EndSetParameter(thresh_ptr));
            }

            AppEvent::SelectSlot(idx) => {
            let idx = *idx;
            match self.drag_slot {
                None => {
                    // Select this slot as the reorder source
                    self.drag_slot = Some(idx);
                }
                Some(src) if src == idx => {
                    // Click the same slot again = cancel
                    self.drag_slot = None;
                }
                Some(src) => {
                    // Swap the modules at `src` and `idx`
                    let src_mt = slot_module_type(&self.params, src);
                    let tgt_mt = slot_module_type(&self.params, idx);
                    let src_ptr = slot_param_ptr(&self.params, src);
                    let tgt_ptr = slot_param_ptr(&self.params, idx);

                    // src slot receives tgt_mt; tgt slot receives src_mt
                    let src_norm = slot_preview_normalized(&self.params, src, tgt_mt);
                    let tgt_norm = slot_preview_normalized(&self.params, idx, src_mt);

                    // Safety: ParamPtr is valid as long as params lives, which outlives the editor.
                    cx.emit(RawParamEvent::BeginSetParameter(src_ptr));
                    cx.emit(RawParamEvent::SetParameterNormalized(src_ptr, src_norm));
                    cx.emit(RawParamEvent::EndSetParameter(src_ptr));

                    cx.emit(RawParamEvent::BeginSetParameter(tgt_ptr));
                    cx.emit(RawParamEvent::SetParameterNormalized(tgt_ptr, tgt_norm));
                    cx.emit(RawParamEvent::EndSetParameter(tgt_ptr));

                    self.drag_slot = None;
                }
            }
            } // AppEvent::SelectSlot
        });
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
        _ => params.module_order_6.value(),
    }
}

fn slot_param_ptr(params: &Arc<BusChannelStripParams>, slot: usize) -> ParamPtr {
    match slot {
        0 => params.module_order_1.as_ptr(),
        1 => params.module_order_2.as_ptr(),
        2 => params.module_order_3.as_ptr(),
        3 => params.module_order_4.as_ptr(),
        4 => params.module_order_5.as_ptr(),
        _ => params.module_order_6.as_ptr(),
    }
}

fn slot_preview_normalized(params: &Arc<BusChannelStripParams>, slot: usize, mt: ModuleType) -> f32 {
    // EnumParam::preview_normalized takes the enum variant directly (Plain = ModuleType)
    match slot {
        0 => params.module_order_1.preview_normalized(mt),
        1 => params.module_order_2.preview_normalized(mt),
        2 => params.module_order_3.preview_normalized(mt),
        3 => params.module_order_4.preview_normalized(mt),
        4 => params.module_order_5.preview_normalized(mt),
        _ => params.module_order_6.preview_normalized(mt),
    }
}

/// Converts ModuleType to usize for use as a vizia Binding lens target.
/// vizia's `Binding::new` requires `Target: Data`; usize satisfies that.
fn module_type_to_usize(mt: ModuleType) -> usize {
    match mt {
        ModuleType::Api5500EQ   => 0,
        ModuleType::ButterComp2 => 1,
        ModuleType::PultecEQ    => 2,
        ModuleType::DynamicEQ   => 3,
        ModuleType::Transformer => 4,
        ModuleType::Punch       => 5,
    }
}

fn usize_to_module_type(idx: usize) -> ModuleType {
    match idx {
        0 => ModuleType::Api5500EQ,
        1 => ModuleType::ButterComp2,
        2 => ModuleType::PultecEQ,
        3 => ModuleType::DynamicEQ,
        4 => ModuleType::Transformer,
        _ => ModuleType::Punch,
    }
}

fn module_type_to_theme(mt: ModuleType) -> ModuleTheme {
    match mt {
        ModuleType::Api5500EQ   => ModuleTheme::Api5500,
        ModuleType::ButterComp2 => ModuleTheme::ButterComp2,
        ModuleType::PultecEQ    => ModuleTheme::Pultec,
        ModuleType::DynamicEQ   => ModuleTheme::DynamicEq,
        ModuleType::Transformer => ModuleTheme::Transformer,
        ModuleType::Punch       => ModuleTheme::Punch,
    }
}

fn module_type_name(mt: ModuleType) -> &'static str {
    match mt {
        ModuleType::Api5500EQ   => "API 550A",
        ModuleType::ButterComp2 => "ButterComp2",
        ModuleType::PultecEQ    => "Pultec EQP-1A",
        ModuleType::DynamicEQ   => "Dynamic EQ",
        ModuleType::Transformer => "Console/Tape",
        ModuleType::Punch       => "PUNCH",
    }
}

fn module_type_subtitle(mt: ModuleType) -> &'static str {
    match mt {
        ModuleType::Api5500EQ   => "3-BAND EQ",
        ModuleType::ButterComp2 => "COMPRESSOR",
        ModuleType::PultecEQ    => "TUBE EQ",
        ModuleType::DynamicEQ   => "DYN EQ",
        ModuleType::Transformer => "TRANSFORMER",
        ModuleType::Punch       => "CLIP + TRANSIENT",
    }
}

// ============================================================================
// Editor Entry Points
// ============================================================================

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1820, 820))
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
        cx.add_stylesheet(COMPONENT_STYLES).expect("Failed to add stylesheet");

        Data {
            params: params.clone(),
            drag_slot: None,
            dyneq_open: false,
            analysis_requested: analysis_requested.clone(),
            analysis_result: analysis_result.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            // Chassis header
            HStack::new(cx, |cx| {
                Label::new(cx, "API")
                    .class("chassis-brand");
                Label::new(cx, "Bus Channel Strip")
                    .class("chassis-title");

                // Signal flow / reorder hint
                VStack::new(cx, |cx| {
                    Label::new(cx, "SIGNAL FLOW")
                        .class("signal-flow-label");
                    Label::new(cx, "Click \u{2261} on a module to select, then click another to swap")
                        .class("signal-flow-hint");
                    Label::new(cx, "DSP order: module_order_1 \u{2192} module_order_5")
                        .class("signal-flow-params");
                })
                .class("signal-flow-section");

                create_master_section(cx);
            })
            .class("chassis-header")
            .height(Pixels(80.0))
            .width(Stretch(1.0));

            // Strip view — hidden when DynEQ back view is open.
            HStack::new(cx, |cx| {
                for slot_idx in 0..6_usize {
                    create_dynamic_module_slot(cx, slot_idx);
                }
            })
            .class("lunchbox-slots")
            .height(Stretch(1.0))
            .width(Stretch(1.0))
            .min_width(Pixels(1720.0))
            .gap(Pixels(4.0))
            .display(Data::dyneq_open.map(|o| if *o { Display::None } else { Display::Flex }));

            // DynEQ back view — hidden when strip is shown.
            build_dyneq_back_view(cx, spectrum_data.clone(), analysis_result.clone(), gr_data.clone());
        })
        .class("lunchbox-chassis")
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .min_width(Pixels(1780.0))
        .min_height(Pixels(780.0))
        .padding(Pixels(20.0));
    })
}

fn create_master_section(cx: &mut Context) {
    HStack::new(cx, |cx| {
        Label::new(cx, "MASTER")
            .class("master-label");
        components::create_gain_slider(cx, "Gain", Data::params, |p| &p.gain);
    })
    .class("master-controls")
    .gap(Pixels(12.0));
}

// ============================================================================
// Dynamic Module Slot
// ============================================================================

/// Creates one 500-series slot that reactively renders whatever module is
/// currently assigned to `module_order_{slot_idx+1}`. The entire slot content
/// is wrapped in a `Binding` so it rebuilds when the module type changes
/// (i.e. after a swap). The drag-source highlight is toggled separately via
/// `toggle_class` which reacts to `Data::drag_slot` without a full rebuild.
fn create_dynamic_module_slot(cx: &mut Context, slot_idx: usize) {
    // Use usize as the Binding target because vizia requires `Target: Data`,
    // and usize satisfies that bound whereas our ModuleType enum does not.
    Binding::new(
        cx,
        Data::params.map(move |p| module_type_to_usize(slot_module_type(p, slot_idx))),
        move |cx, mt_lens| {
            let mt = usize_to_module_type(mt_lens.get(cx));
            let theme = module_type_to_theme(mt);

            VStack::new(cx, |cx| {
                // ── Drag handle ─────────────────────────────────────────────
                HStack::new(cx, |cx| {
                    Label::new(cx, "\u{2261}")  // ≡ hamburger icon
                        .class("drag-handle-icon");
                    // Reactive label: context changes based on drag state
                    Label::new(cx, Data::drag_slot.map(move |ds| {
                        match *ds {
                            Some(src) if src == slot_idx => "CANCEL",
                            Some(_) => "SWAP HERE",
                            None => "MOVE",
                        }
                    }))
                    .class("drag-handle-label");
                    // "SELECTED" indicator — only visible when this slot is
                    // the active swap source.
                    Label::new(cx, "\u{25CF} SELECTED")
                        .class("drag-selected-indicator")
                        .display(Data::drag_slot.map(move |ds| {
                            if *ds == Some(slot_idx) { Display::Flex } else { Display::None }
                        }));
                })
                .class("drag-handle")
                .toggle_class(
                    "drag-handle-active",
                    Data::drag_slot.map(move |ds| *ds == Some(slot_idx)),
                )
                .on_press(move |cx| cx.emit(AppEvent::SelectSlot(slot_idx)))
                .cursor(CursorIcon::Hand)
                .top(Pixels(0.0))
                .bottom(Pixels(0.0))
                .height(Auto)
                .width(Stretch(1.0));

                // ── Module header ────────────────────────────────────────────
                VStack::new(cx, |cx| {
                    Label::new(cx, module_type_name(mt))
                        .class("module-name")
                        // Name turns bright yellow when this slot is selected.
                        .color(Data::drag_slot.map(move |ds| {
                            if *ds == Some(slot_idx) {
                                Color::rgb(255, 220, 50)
                            } else {
                                theme.accent_color()
                            }
                        }));
                    Label::new(cx, module_type_subtitle(mt))
                        .class("module-type");
                })
                .class("module-header")
                .top(Pixels(0.0))
                .bottom(Pixels(0.0))
                .height(Auto)
                .width(Stretch(1.0));

                // ── Bypass button ────────────────────────────────────────────
                build_bypass_button_for_type(cx, mt);

                // ── Parameter controls ───────────────────────────────────────
                build_controls_for_type(cx, mt);
            })
            // alignment(TopLeft) packs children to the top-left instead of
            // the default center which distributes Stretch space around items.
            .alignment(Alignment::TopLeft)
            .gap(Pixels(4.0))
            .class("module-slot")
            .class(theme.class_name())
            // Border turns bright white when this slot is selected for swap.
            .border_color(Data::drag_slot.map(move |ds| {
                if *ds == Some(slot_idx) {
                    Color::rgba(255, 255, 255, 230)
                } else {
                    theme.accent_color()
                }
            }))
            .width(Pixels(280.0))
            .height(Stretch(1.0))
            .border_width(Pixels(3.0))
            .background_color(Color::rgb(42, 42, 42))
            .padding(Pixels(12.0));
        },
    );
}

// ============================================================================
// Bypass Buttons — dispatched by module type
// ============================================================================

fn build_bypass_button_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ => {
            components::create_bypass_button(cx, "BYPASS", |p| &p.eq_bypass);
        }
        ModuleType::ButterComp2 => {
            components::create_bypass_button(cx, "BYPASS", |p| &p.comp_bypass);
        }
        ModuleType::PultecEQ => {
            components::create_bypass_button(cx, "BYPASS", |p| &p.pultec_bypass);
        }
        ModuleType::DynamicEQ => {
            #[cfg(feature = "dynamic_eq")]
            components::create_bypass_button(cx, "BYPASS", |p| &p.dyneq_bypass);
        }
        ModuleType::Transformer => {
            components::create_bypass_button(cx, "BYPASS", |p| &p.transformer_bypass);
        }
        ModuleType::Punch => {
            #[cfg(feature = "punch")]
            components::create_bypass_button(cx, "BYPASS", |p| &p.punch_bypass);
        }
    }
}

// ============================================================================
// Parameter Controls — dispatched by module type
// ============================================================================

fn build_controls_for_type(cx: &mut Context, mt: ModuleType) {
    match mt {
        ModuleType::Api5500EQ   => build_api5500_controls(cx),
        ModuleType::ButterComp2 => build_buttercomp2_controls(cx),
        ModuleType::PultecEQ    => build_pultec_controls(cx),
        ModuleType::DynamicEQ   => build_dynamic_eq_controls(cx),
        ModuleType::Transformer => build_transformer_controls(cx),
        ModuleType::Punch       => build_punch_controls(cx),
    }
}

fn build_api5500_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "HF", Data::params, |p| &p.hf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.hf_gain);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "MF", Data::params, |p| &p.lmf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lmf_gain);
            components::create_param_slider(cx, "Q", Data::params, |p| &p.lmf_q);
        });
        components::module_row(cx, |cx| {
            components::create_frequency_slider(cx, "LF", Data::params, |p| &p.lf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lf_gain);
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_buttercomp2_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::create_ratio_slider(cx, "COMPRESS", Data::params, |p| &p.comp_compress);
        components::create_gain_slider(cx, "OUTPUT", Data::params, |p| &p.comp_output);
        components::create_param_slider(cx, "DRY/WET", Data::params, |p| &p.comp_dry_wet);
    })
    .gap(Pixels(6.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

fn build_pultec_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::module_section(cx, "LOW FREQUENCY", |cx| {
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "BOOST", Data::params, |p| &p.pultec_lf_boost_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.pultec_lf_boost_gain);
            });
            components::create_gain_slider(cx, "ATTEN", Data::params, |p| &p.pultec_lf_cut_gain);
        });
        components::module_section(cx, "HIGH FREQUENCY", |cx| {
            components::module_row(cx, |cx| {
                components::create_frequency_slider(cx, "BOOST", Data::params, |p| &p.pultec_hf_boost_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.pultec_hf_boost_gain);
            });
            components::create_param_slider(cx, "TUBE", Data::params, |p| &p.pultec_tube_drive);
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
        Label::new(cx, "Real-time frequency-dependent compression with per-band threshold control")
            .class("dyneq-card-desc")
            .height(Auto)
            .width(Stretch(1.0));
        // OPEN button — flips to the full DynEQ back view
        VStack::new(cx, |cx| {
            Label::new(cx, "OPEN EDITOR  \u{25B6}")
                .class("dyneq-open-label")
                .height(Pixels(18.0))
                .width(Stretch(1.0));
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

        // Pull latest audio-thread data (lock-free: AtomicU32 bins, Acquire on dirty flag).
        {
            let mut bins = self.display_bins.borrow_mut();
            self.spectrum_data.read_into_slice(&mut bins);
        }
        // Pull overlap bins from the last analysis (Relaxed — display-only, staleness is fine).
        {
            let mut overlap = self.display_overlap.borrow_mut();
            for (i, slot) in self.analysis_result.overlap_bins.iter()
                .enumerate()
                .take(spectral::SPECTRUM_BINS)
            {
                overlap[i] = f32::from_bits(slot.load(Ordering::Relaxed));
            }
        }

        let bins    = self.display_bins.borrow();
        let overlap = self.display_overlap.borrow();
        let bounds  = cx.bounds();

        // ── Background ──────────────────────────────────────────────────────
        let bg_rect = vg::Rect::from_xywh(bounds.x, bounds.y, bounds.w, bounds.h);
        let mut bg_paint = vg::Paint::default();
        bg_paint.set_color(vg::Color::from_argb(255, 18, 25, 31));
        bg_paint.set_style(vg::PaintStyle::Fill);
        canvas.draw_rect(bg_rect, &bg_paint);

        let n = bins.len();
        if n == 0 {
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
            (45, 80,  200, 110),  // band1 LOW      — green
            (45, 60,  150, 220),  // band2 LOW MID  — sky blue
            (45, 150,  90, 220),  // band3 HIGH MID — purple
            (45, 220, 150,  50),  // band4 HIGH     — amber
        ];

        let cx_frac: [f32; 3] = CROSSOVER_HZ.map(|f| (f / SPECTRUM_TOP_HZ).clamp(0.0, 1.0));
        let cx_x: [f32; 3] = cx_frac.map(|fr| bounds.x + fr * bounds.w);

        let band_left  = [bounds.x, cx_x[0], cx_x[1], cx_x[2]];
        let band_right = [cx_x[0],  cx_x[1], cx_x[2], bounds.x + bounds.w];

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
        let max_overlap = overlap.iter().cloned().fold(0.0_f32, f32::max).max(f32::MIN_POSITIVE);
        if max_overlap > f32::MIN_POSITIVE * 2.0 {
            let mut ovl_path = vg::Path::new();
            let mut ovl_started = false;
            for (i, &ov) in overlap.iter().enumerate() {
                let norm = (ov / max_overlap).clamp(0.0, 1.0);
                let x = bounds.x + i as f32 * x_step;
                let y = bounds.y + bounds.h - norm * bounds.h;
                if !ovl_started { ovl_path.move_to((x, y)); ovl_started = true; }
                else { ovl_path.line_to((x, y)); }
            }
            if ovl_started {
                ovl_path.line_to((bounds.x + bounds.w, bounds.y + bounds.h));
                ovl_path.line_to((bounds.x,            bounds.y + bounds.h));
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
            let db   = 20.0 * mag.max(1e-9_f32).log10();
            let norm = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
            let x    = bounds.x + i as f32 * x_step;
            let y    = bounds.y + bounds.h - norm * bounds.h;
            if !started { fill.move_to((x, y)); started = true; } else { fill.line_to((x, y)); }
        }
        fill.line_to((bounds.x + bounds.w, bounds.y + bounds.h));
        fill.line_to((bounds.x,            bounds.y + bounds.h));
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
            let db   = 20.0 * mag.max(1e-9_f32).log10();
            let norm = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
            let x    = bounds.x + i as f32 * x_step;
            let y    = bounds.y + bounds.h - norm * bounds.h;
            if !started { line.move_to((x, y)); started = true; } else { line.line_to((x, y)); }
        }
        let mut stroke_paint = vg::Paint::default();
        stroke_paint.set_color(vg::Color::from_argb(200, 80, 220, 180));
        stroke_paint.set_style(vg::PaintStyle::Stroke);
        stroke_paint.set_stroke_width(1.5);
        stroke_paint.set_anti_alias(true);
        canvas.draw_path(&line, &stroke_paint);

        // Continuous redraw — keep the spectrum animated at the GUI frame rate.
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
     $q:ident, $mode:ident, $atk:ident, $rel:ident, $gain:ident) => {
        VStack::new($cx, |cx| {
            Label::new(cx, $title)
                .class("dyneq-band-title")
                .height(Pixels(14.0))
                .width(Stretch(1.0))
                // Title participates in stretch distribution like the sliders.
                .top(Stretch(1.0))
                .bottom(Pixels(0.0));
            components::module_row(cx, |cx| {
                components::create_on_button(cx, |p| &p.$enabled);
                components::create_bypass_button(cx, "SOLO", |p| &p.$solo);
            })
            .top(Stretch(1.0))
            .bottom(Pixels(0.0));
            dyneq_slider!(cx, "FREQ",   |p| &p.$freq);
            dyneq_slider!(cx, "THRESH", |p| &p.$thresh);
            dyneq_slider!(cx, "RATIO",  |p| &p.$ratio);
            dyneq_slider!(cx, "Q",      |p| &p.$q);
            dyneq_slider!(cx, "MODE",   |p| &p.$mode);
            dyneq_slider!(cx, "ATK ms", |p| &p.$atk);
            dyneq_slider!(cx, "REL ms", |p| &p.$rel);
            dyneq_slider!(cx, "GAIN",   |p| &p.$gain);
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
                        let freq = f32::from_bits(
                            ar_clone.target_freq.load(Ordering::Relaxed)
                        );
                        let threshold_db = f32::from_bits(
                            ar_clone.target_threshold_db.load(Ordering::Relaxed)
                        );
                        cx.emit(AppEvent::ApplyAnalysis { band, freq, threshold_db });
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
        SpectrumCanvas::new(cx, spectrum_data, analysis_result, gr_data)
            .class("dyneq-spectrum")
            .height(Pixels(220.0))
            .width(Stretch(1.0))
            .top(Pixels(0.0))
            .bottom(Pixels(0.0));

        // ── 4-band horizontal editor ──────────────────────────────────────────
        #[cfg(feature = "dynamic_eq")]
        // height(Stretch(1.0)): HStack fills remaining back-view height after
        // the header row and spectrum canvas, giving band columns a concrete
        // height to stretch into for dynamic spacing to work.
        HStack::new(cx, |cx| {
            dyneq_band_col!(cx, "BAND 1 — LOW",
                dyneq_band1_enabled, dyneq_band1_solo,
                dyneq_band1_freq, dyneq_band1_threshold, dyneq_band1_ratio,
                dyneq_band1_q, dyneq_band1_mode, dyneq_band1_attack,
                dyneq_band1_release, dyneq_band1_gain);

            dyneq_band_col!(cx, "BAND 2 — LOW MID",
                dyneq_band2_enabled, dyneq_band2_solo,
                dyneq_band2_freq, dyneq_band2_threshold, dyneq_band2_ratio,
                dyneq_band2_q, dyneq_band2_mode, dyneq_band2_attack,
                dyneq_band2_release, dyneq_band2_gain);

            dyneq_band_col!(cx, "BAND 3 — HIGH MID",
                dyneq_band3_enabled, dyneq_band3_solo,
                dyneq_band3_freq, dyneq_band3_threshold, dyneq_band3_ratio,
                dyneq_band3_q, dyneq_band3_mode, dyneq_band3_attack,
                dyneq_band3_release, dyneq_band3_gain);

            dyneq_band_col!(cx, "BAND 4 — HIGH",
                dyneq_band4_enabled, dyneq_band4_solo,
                dyneq_band4_freq, dyneq_band4_threshold, dyneq_band4_ratio,
                dyneq_band4_q, dyneq_band4_mode, dyneq_band4_attack,
                dyneq_band4_release, dyneq_band4_gain);
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
        components::module_row(cx, |cx| {
            components::create_param_slider(cx, "MODEL", Data::params, |p| &p.transformer_model);
            components::create_ratio_slider(cx, "COMP", Data::params, |p| &p.transformer_compression);
        });
        components::module_section(cx, "DRIVE", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "INPUT", Data::params, |p| &p.transformer_input_drive);
                components::create_param_slider(cx, "OUTPUT", Data::params, |p| &p.transformer_output_drive);
            });
        });
        components::module_section(cx, "CHARACTER", |cx| {
            components::module_row(cx, |cx| {
                components::create_param_slider(cx, "SAT", Data::params, |p| &p.transformer_input_saturation);
                components::create_param_slider(cx, "LOW", Data::params, |p| &p.transformer_low_response);
                components::create_param_slider(cx, "HIGH", Data::params, |p| &p.transformer_high_response);
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
                components::create_param_slider(cx, "OVSMP", Data::params, |p| &p.punch_oversampling);
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
            components::create_param_slider(cx, "MIX", Data::params, |p| &p.punch_mix);
        });
    })
    .gap(Pixels(4.0))
    .height(Auto)
    .width(Stretch(1.0))
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}
