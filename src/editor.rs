// src/editor.rs
// Vizia GUI implementation for Bus Channel Strip

use std::sync::Arc;
use nih_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use vizia_plug::widgets::RawParamEvent;

use crate::{BusChannelStripParams, ModuleType};
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
}

// ============================================================================
// Editor Data Model
// ============================================================================

#[derive(Lens)]
pub struct Data {
    pub params: Arc<BusChannelStripParams>,
    /// The slot index currently selected for swapping, or None.
    pub drag_slot: Option<usize>,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e: &AppEvent, _| {
            let AppEvent::SelectSlot(idx) = e;
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
    ViziaState::new(|| (1800, 650))
}

pub(crate) fn create(
    params: Arc<BusChannelStripParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(COMPONENT_STYLES).expect("Failed to add stylesheet");

        Data {
            params: params.clone(),
            drag_slot: None,
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

            // 5 dynamic module slots — order driven by module_order_1..5 params
            HStack::new(cx, |cx| {
                for slot_idx in 0..5_usize {
                    create_dynamic_module_slot(cx, slot_idx);
                }
            })
            .class("lunchbox-slots")
            .height(Stretch(1.0))
            .width(Stretch(1.0))
            .min_width(Pixels(1620.0))
            .gap(Pixels(4.0));
        })
        .class("lunchbox-chassis")
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .min_width(Pixels(1680.0))
        .min_height(Pixels(620.0))
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
                    Label::new(cx, "MOVE")
                        .class("drag-handle-label");
                })
                .class("drag-handle")
                .toggle_class(
                    "drag-handle-active",
                    Data::drag_slot.map(move |ds| *ds == Some(slot_idx)),
                )
                .on_press(move |cx| cx.emit(AppEvent::SelectSlot(slot_idx)))
                .cursor(CursorIcon::Hand);

                // ── Module header ────────────────────────────────────────────
                VStack::new(cx, |cx| {
                    Label::new(cx, module_type_name(mt))
                        .class("module-name")
                        .color(theme.accent_color());
                    Label::new(cx, module_type_subtitle(mt))
                        .class("module-type");
                })
                .class("module-header");

                // ── Bypass button ────────────────────────────────────────────
                build_bypass_button_for_type(cx, mt);

                // ── Parameter controls ───────────────────────────────────────
                build_controls_for_type(cx, mt);
            })
            .class("module-slot")
            .class(theme.class_name())
            .toggle_class(
                "drag-source",
                Data::drag_slot.map(move |ds| *ds == Some(slot_idx)),
            )
            .width(Pixels(320.0))
            .height(Stretch(1.0))
            .border_width(Pixels(3.0))
            .border_color(theme.accent_color())
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
        HStack::new(cx, |cx| {
            components::create_frequency_slider(cx, "HF", Data::params, |p| &p.hf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.hf_gain);
        })
        .gap(Pixels(8.0));

        HStack::new(cx, |cx| {
            components::create_frequency_slider(cx, "MF", Data::params, |p| &p.lmf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lmf_gain);
            components::create_param_slider(cx, "Q", Data::params, |p| &p.lmf_q);
        })
        .gap(Pixels(8.0));

        HStack::new(cx, |cx| {
            components::create_frequency_slider(cx, "LF", Data::params, |p| &p.lf_freq);
            components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lf_gain);
        })
        .gap(Pixels(8.0));
    })
    .gap(Pixels(6.0));
}

fn build_buttercomp2_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        components::create_ratio_slider(cx, "COMPRESS", Data::params, |p| &p.comp_compress);
        components::create_gain_slider(cx, "OUTPUT", Data::params, |p| &p.comp_output);
        components::create_param_slider(cx, "DRY/WET", Data::params, |p| &p.comp_dry_wet);
    })
    .gap(Pixels(12.0));
}

fn build_pultec_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "LOW FREQUENCY").class("section-label");
            HStack::new(cx, |cx| {
                components::create_frequency_slider(cx, "BOOST", Data::params, |p| &p.pultec_lf_boost_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.pultec_lf_boost_gain);
            })
            .gap(Pixels(8.0));
            components::create_gain_slider(cx, "ATTEN", Data::params, |p| &p.pultec_lf_cut_gain);
        })
        .gap(Pixels(4.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "HIGH FREQUENCY").class("section-label");
            HStack::new(cx, |cx| {
                components::create_frequency_slider(cx, "BOOST", Data::params, |p| &p.pultec_hf_boost_freq);
                components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.pultec_hf_boost_gain);
            })
            .gap(Pixels(8.0));
            components::create_param_slider(cx, "TUBE", Data::params, |p| &p.pultec_tube_drive);
        })
        .gap(Pixels(4.0));
    })
    .gap(Pixels(8.0));
}

fn build_dynamic_eq_controls(cx: &mut Context) {
    #[cfg(feature = "dynamic_eq")]
    {
        Label::new(cx, "DYNAMIC EQ").class("section-label");
        // Full dynamic EQ controls can be added here when the feature is enabled
    }
    #[cfg(not(feature = "dynamic_eq"))]
    {
        Label::new(cx, "build with dynamic_eq feature").class("module-type");
    }
}

fn build_transformer_controls(cx: &mut Context) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            components::create_param_slider(cx, "MODEL", Data::params, |p| &p.transformer_model);
            components::create_ratio_slider(cx, "COMP", Data::params, |p| &p.transformer_compression);
        })
        .gap(Pixels(8.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "DRIVE").class("section-label");
            HStack::new(cx, |cx| {
                components::create_param_slider(cx, "INPUT", Data::params, |p| &p.transformer_input_drive);
                components::create_param_slider(cx, "OUTPUT", Data::params, |p| &p.transformer_output_drive);
            })
            .gap(Pixels(8.0));
        })
        .gap(Pixels(4.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "CHARACTER").class("section-label");
            HStack::new(cx, |cx| {
                components::create_param_slider(cx, "SAT", Data::params, |p| &p.transformer_input_saturation);
                components::create_param_slider(cx, "LOW", Data::params, |p| &p.transformer_low_response);
                components::create_param_slider(cx, "HIGH", Data::params, |p| &p.transformer_high_response);
            })
            .gap(Pixels(8.0));
        })
        .gap(Pixels(4.0));
    })
    .gap(Pixels(6.0));
}

fn build_punch_controls(cx: &mut Context) {
    #[cfg(feature = "punch")]
    VStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "CLIPPER").class("section-label");
            HStack::new(cx, |cx| {
                components::create_gain_slider(cx, "THRESH", Data::params, |p| &p.punch_threshold);
                components::create_param_slider(cx, "SOFT", Data::params, |p| &p.punch_softness);
            })
            .gap(Pixels(8.0));
        })
        .gap(Pixels(4.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "TRANSIENTS").class("section-label");
            HStack::new(cx, |cx| {
                components::create_param_slider(cx, "ATTACK", Data::params, |p| &p.punch_attack);
                components::create_param_slider(cx, "SUSTAIN", Data::params, |p| &p.punch_sustain);
            })
            .gap(Pixels(8.0));
            HStack::new(cx, |cx| {
                components::create_param_slider(cx, "SENS", Data::params, |p| &p.punch_sensitivity);
            })
            .gap(Pixels(8.0));
        })
        .gap(Pixels(4.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "OUTPUT").class("section-label");
            HStack::new(cx, |cx| {
                components::create_gain_slider(cx, "IN", Data::params, |p| &p.punch_input_gain);
                components::create_gain_slider(cx, "OUT", Data::params, |p| &p.punch_output_gain);
                components::create_param_slider(cx, "MIX", Data::params, |p| &p.punch_mix);
            })
            .gap(Pixels(8.0));
        })
        .gap(Pixels(4.0));
    })
    .gap(Pixels(6.0));
}
