// src/editor.rs
// Vizia GUI implementation for Bus Channel Strip

use std::sync::Arc;
use nih_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
// use vizia_plug::widgets::*; // Not used directly here; components uses widgets
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::BusChannelStripParams;
use crate::components::{self, ModuleTheme};
use crate::styles::COMPONENT_STYLES;

pub const NOTO_SANS: &str = "Noto Sans";

// Bypass param accessors to avoid closure lifetime issues
fn get_eq_bypass(p: &Arc<BusChannelStripParams>) -> &BoolParam { &p.eq_bypass }
fn get_comp_bypass(p: &Arc<BusChannelStripParams>) -> &BoolParam { &p.comp_bypass }
fn get_pultec_bypass(p: &Arc<BusChannelStripParams>) -> &BoolParam { &p.pultec_bypass }
fn get_transformer_bypass(p: &Arc<BusChannelStripParams>) -> &BoolParam { &p.transformer_bypass }
#[cfg(feature = "punch")]
fn get_punch_bypass(p: &Arc<BusChannelStripParams>) -> &BoolParam { &p.punch_bypass }


#[derive(Lens)]
pub struct Data {
    pub params: Arc<BusChannelStripParams>,
}

impl Model for Data {}

/// Create default editor state with 500 series lunchbox dimensions
/// Minimum width: 5 modules × 320px + gaps + padding = ~1680px
/// Default size provides comfortable spacing for all 5 modules
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1800, 650))
}

/// Create the vizia editor
pub(crate) fn create(
    params: Arc<BusChannelStripParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_stylesheet(COMPONENT_STYLES).expect("Failed to add stylesheet");
        
        Data {
            params: params.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            // Lunchbox chassis header
            HStack::new(cx, |cx| {
                Label::new(cx, "API")
                    .class("chassis-brand");
                Label::new(cx, "Bus Channel Strip")
                    .class("chassis-title");

                // Signal flow indicator
                VStack::new(cx, |cx| {
                    Label::new(cx, "SIGNAL FLOW")
                        .class("signal-flow-label");
                    Label::new(cx, "Change module order via DAW automation →")
                        .class("signal-flow-hint");
                    Label::new(cx, "Parameters: 'Module Order 1' through 'Module Order 6'")
                        .class("signal-flow-params");
                })
                .class("signal-flow-section");

                create_master_section(cx);
            })
            .class("chassis-header")
            .height(Pixels(80.0))
            .width(Stretch(1.0));

            // 500 Series module slots in horizontal layout
            // NOTE: Module ordering is controlled by parameters module_order_1 through module_order_6
            // TODO: Implement UI for module reordering (dropdown selectors or drag-and-drop)
            HStack::new(cx, |cx| {
                // Slot 1: API5500 EQ
                create_api5500_module_slot(cx);

                // Slot 2: ButterComp2
                create_buttercomp2_module_slot(cx);

                // Slot 3: Pultec EQ
                create_pultec_module_slot(cx);

                // Slot 4: Transformer
                create_transformer_module_slot(cx);

                // Slot 5: Punch (Clipper + Transient Shaper)
                #[cfg(feature = "punch")]
                create_punch_module_slot(cx);
            })
            .class("lunchbox-slots")
            .height(Stretch(1.0))
            .width(Stretch(1.0))
            .min_width(Pixels(1620.0))  // 5 modules × 320px + gaps
            .gap(Pixels(4.0));
        })
        .class("lunchbox-chassis")
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .min_width(Pixels(1680.0))  // Total minimum including padding
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

fn create_api5500_module_slot(cx: &mut Context) {
    create_500_series_module(
        cx,
        "API 550A",
        "3-BAND EQ",
        ModuleTheme::Api5500,
        Some(get_eq_bypass),
        |cx| {
            VStack::new(cx, |cx| {
                // Top row: HF controls
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "HF", Data::params, |p| &p.hf_freq);
                    components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.hf_gain);
                })
                .gap(Pixels(8.0));
                
                // Middle row: MF controls  
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "MF", Data::params, |p| &p.lmf_freq);
                    components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lmf_gain);
                    components::create_param_slider(cx, "Q", Data::params, |p| &p.lmf_q);
                })
                .gap(Pixels(8.0));
                
                // Bottom row: LF controls
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "LF", Data::params, |p| &p.lf_freq);
                    components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.lf_gain);
                })
                .gap(Pixels(8.0));
            })
            .gap(Pixels(6.0));
        }
    );
}

// Helper function to create 500 series module slots
fn create_500_series_module<F>(
    cx: &mut Context,
    module_name: &str,
    module_type: &str,
    theme: ModuleTheme,
    bypass_param: Option<F>,
    content_builder: impl FnOnce(&mut Context),
) where
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    VStack::new(cx, |cx| {
        // Module faceplate header
        VStack::new(cx, |cx| {
            Label::new(cx, module_name)
                .class("module-name");
            Label::new(cx, module_type)
                .class("module-type");
        })
        .class("module-header");
        
        // Bypass LED and button
        if let Some(bypass_fn) = bypass_param {
            components::create_bypass_button(cx, "BYPASS", bypass_fn);
        }
        
        // Module controls
        content_builder(cx);
    })
    .class("module-slot")
    .class(theme.class_name())
    .width(Pixels(320.0))
    .height(Stretch(1.0));
}

fn create_buttercomp2_module_slot(cx: &mut Context) {
    create_500_series_module(
        cx,
        "ButterComp2",
        "COMPRESSOR",
        ModuleTheme::ButterComp2,
        Some(get_comp_bypass),
        |cx| {
            VStack::new(cx, |cx| {
                components::create_ratio_slider(cx, "COMPRESS", Data::params, |p| &p.comp_compress);
                components::create_gain_slider(cx, "OUTPUT", Data::params, |p| &p.comp_output);
                components::create_param_slider(cx, "DRY/WET", Data::params, |p| &p.comp_dry_wet);
            })
            .gap(Pixels(12.0));
        }
    );
}

fn create_pultec_module_slot(cx: &mut Context) {
    create_500_series_module(
        cx,
        "Pultec EQP-1A",
        "TUBE EQ",
        ModuleTheme::Pultec,
        Some(get_pultec_bypass),
        |cx| {
            VStack::new(cx, |cx| {
                // Low boost/cut section
                VStack::new(cx, |cx| {
                    Label::new(cx, "LOW FREQUENCY")
                        .class("section-label");
                    HStack::new(cx, |cx| {
                        components::create_frequency_slider(cx, "BOOST", Data::params, |p| &p.pultec_lf_boost_freq);
                        components::create_gain_slider(cx, "GAIN", Data::params, |p| &p.pultec_lf_boost_gain);
                    })
                    .gap(Pixels(8.0));
                    components::create_gain_slider(cx, "ATTEN", Data::params, |p| &p.pultec_lf_cut_gain);
                })
                .gap(Pixels(4.0));
                
                // High boost section  
                VStack::new(cx, |cx| {
                    Label::new(cx, "HIGH FREQUENCY")
                        .class("section-label");
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
    );
}

fn create_transformer_module_slot(cx: &mut Context) {
    create_500_series_module(
        cx,
        "Console/Tape",
        "TRANSFORMER",
        ModuleTheme::Transformer,
        Some(get_transformer_bypass),
        |cx| {
            VStack::new(cx, |cx| {
                // Model selector
                HStack::new(cx, |cx| {
                    components::create_param_slider(cx, "MODEL", Data::params, |p| &p.transformer_model);
                    components::create_ratio_slider(cx, "COMP", Data::params, |p| &p.transformer_compression);
                })
                .gap(Pixels(8.0));

                // Drive controls
                VStack::new(cx, |cx| {
                    Label::new(cx, "DRIVE")
                        .class("section-label");
                    HStack::new(cx, |cx| {
                        components::create_param_slider(cx, "INPUT", Data::params, |p| &p.transformer_input_drive);
                        components::create_param_slider(cx, "OUTPUT", Data::params, |p| &p.transformer_output_drive);
                    })
                    .gap(Pixels(8.0));
                })
                .gap(Pixels(4.0));

                // Saturation and Response
                VStack::new(cx, |cx| {
                    Label::new(cx, "CHARACTER")
                        .class("section-label");
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
    );
}

#[cfg(feature = "punch")]
fn create_punch_module_slot(cx: &mut Context) {
    create_500_series_module(
        cx,
        "PUNCH",
        "CLIP + TRANSIENT",
        ModuleTheme::Punch,
        Some(get_punch_bypass),
        |cx| {
            VStack::new(cx, |cx| {
                // Clipper section
                VStack::new(cx, |cx| {
                    Label::new(cx, "CLIPPER")
                        .class("section-label");
                    HStack::new(cx, |cx| {
                        components::create_gain_slider(cx, "THRESH", Data::params, |p| &p.punch_threshold);
                        components::create_param_slider(cx, "SOFT", Data::params, |p| &p.punch_softness);
                    })
                    .gap(Pixels(8.0));
                })
                .gap(Pixels(4.0));

                // Transient shaper section
                VStack::new(cx, |cx| {
                    Label::new(cx, "TRANSIENTS")
                        .class("section-label");
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

                // Output section
                VStack::new(cx, |cx| {
                    Label::new(cx, "OUTPUT")
                        .class("section-label");
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
    );
}

fn create_api5500_section(cx: &mut Context) {
    components::create_module_section(
        cx,
        "API5500 EQ",
        ModuleTheme::Api5500,
        Some(get_eq_bypass),
        |cx| {
            // Compact horizontal layout for all EQ bands
            HStack::new(cx, |cx| {
                // Low Shelf
                VStack::new(cx, |cx| {
                    Label::new(cx, "Low Shelf")
                        .class("section-title");
                    components::create_frequency_slider(cx, "LF Freq", Data::params, |p| &p.lf_freq);
                    components::create_gain_slider(cx, "LF Gain", Data::params, |p| &p.lf_gain);
                })
                .class("param-group");
                
                // Low Mid
                VStack::new(cx, |cx| {
                    Label::new(cx, "Low Mid")
                        .class("section-title");
                    components::create_frequency_slider(cx, "LMF Freq", Data::params, |p| &p.lmf_freq);
                    components::create_gain_slider(cx, "LMF Gain", Data::params, |p| &p.lmf_gain);
                    components::create_param_slider(cx, "LMF Q", Data::params, |p| &p.lmf_q);
                })
                .class("param-group");
                
                // High Shelf
                VStack::new(cx, |cx| {
                    Label::new(cx, "High Shelf")
                        .class("section-title");
                    components::create_frequency_slider(cx, "HF Freq", Data::params, |p| &p.hf_freq);
                    components::create_gain_slider(cx, "HF Gain", Data::params, |p| &p.hf_gain);
                })
                .class("param-group");
            })
            .gap(Pixels(8.0));
        }
    );
}

fn create_buttercomp2_section(cx: &mut Context) {
    components::create_module_section(
        cx,
        "ButterComp2",
        ModuleTheme::ButterComp2,
        Some(get_comp_bypass),
        |cx| {
            // Compact horizontal layout for compressor controls
            HStack::new(cx, |cx| {
                components::create_ratio_slider(cx, "Compress", Data::params, |p| &p.comp_compress);
                components::create_gain_slider(cx, "Output", Data::params, |p| &p.comp_output);
                components::create_param_slider(cx, "Dry/Wet", Data::params, |p| &p.comp_dry_wet);
            })
            .class("param-group")
            .gap(Pixels(6.0));
        }
    );
}

fn create_pultec_section(cx: &mut Context) {
    components::create_module_section(
        cx,
        "Pultec EQ",
        ModuleTheme::Pultec,
        Some(get_pultec_bypass),
        |cx| {
            // Horizontal layout for low and high sections
            HStack::new(cx, |cx| {
                // Low section
                VStack::new(cx, |cx| {
                    Label::new(cx, "Low Section")
                        .class("section-title");
                    components::create_frequency_slider(cx, "Boost Freq", Data::params, |p| &p.pultec_lf_boost_freq);
                    components::create_gain_slider(cx, "Boost Gain", Data::params, |p| &p.pultec_lf_boost_gain);
                    components::create_gain_slider(cx, "Cut Gain", Data::params, |p| &p.pultec_lf_cut_gain);
                })
                .class("param-group");
                
                // High section
                VStack::new(cx, |cx| {
                    Label::new(cx, "High Section")
                        .class("section-title");
                    components::create_frequency_slider(cx, "Boost Freq", Data::params, |p| &p.pultec_hf_boost_freq);
                    components::create_gain_slider(cx, "Boost Gain", Data::params, |p| &p.pultec_hf_boost_gain);
                    components::create_param_slider(cx, "Tube Drive", Data::params, |p| &p.pultec_tube_drive);
                })
                .class("param-group");
            })
            .gap(Pixels(8.0));
        }
    );
}

fn create_transformer_section(cx: &mut Context) {
    components::create_module_section(
        cx,
        "Transformer",
        ModuleTheme::Transformer,
        Some(get_transformer_bypass),
        |cx| {
            // Efficient horizontal layout for transformer controls
            VStack::new(cx, |cx| {
                // Model selection and compression on one row
                HStack::new(cx, |cx| {
                    components::create_param_slider(cx, "Model", Data::params, |p| &p.transformer_model);
                    components::create_ratio_slider(cx, "Compression", Data::params, |p| &p.transformer_compression);
                })
                .gap(Pixels(8.0))
                .class("param-group");
                
                // Input, Output, and Frequency sections in columns
                HStack::new(cx, |cx| {
                    // Input section
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Input")
                            .class("section-title");
                        components::create_param_slider(cx, "Drive", Data::params, |p| &p.transformer_input_drive);
                        components::create_param_slider(cx, "Saturation", Data::params, |p| &p.transformer_input_saturation);
                    })
                    .class("param-group");
                    
                    // Output section
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Output")
                            .class("section-title");
                        components::create_param_slider(cx, "Drive", Data::params, |p| &p.transformer_output_drive);
                        components::create_param_slider(cx, "Saturation", Data::params, |p| &p.transformer_output_saturation);
                    })
                    .class("param-group");
                    
                    // Frequency response section
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Frequency Response")
                            .class("section-title");
                        components::create_param_slider(cx, "Low", Data::params, |p| &p.transformer_low_response);
                        components::create_param_slider(cx, "High", Data::params, |p| &p.transformer_high_response);
                    })
                    .class("param-group");
                })
                .gap(Pixels(8.0));
            })
            .gap(Pixels(6.0));
        }
    );
}

// Duplicate helpers removed in favor of components::*
