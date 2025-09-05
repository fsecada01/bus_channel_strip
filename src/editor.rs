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


#[derive(Lens)]
pub struct Data {
    pub params: Arc<BusChannelStripParams>,
}

impl Model for Data {}

/// Create default editor state with appropriate size for comprehensive GUI
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1200, 700))
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
            // Plugin title
            Label::new(cx, "Bus Channel Strip")
                .class("plugin-title");

            // Master section
            create_master_section(cx);
            
            // Main modules row
            HStack::new(cx, |cx| {
                create_api5500_section(cx);
                create_buttercomp2_section(cx);
            })
            .height(Auto)
            .gap(Stretch(1.0));
            
            // Second row with remaining modules
            HStack::new(cx, |cx| {
                create_pultec_section(cx);
                create_transformer_section(cx);
            })
            .height(Auto)
            .gap(Stretch(1.0));
        })
        .padding(Pixels(16.0))
        .gap(Pixels(12.0));
    })
}

fn create_master_section(cx: &mut Context) {
    components::create_module_section(
        cx,
        "Master", 
        ModuleTheme::Master,
        None::<fn(&Arc<BusChannelStripParams>) -> &BoolParam>,
        |cx| {
            HStack::new(cx, |cx| {
                components::create_gain_slider(cx, "Master Gain", Data::params, |p| &p.gain);
            })
            .gap(Pixels(8.0));
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
            // Low Shelf section
            VStack::new(cx, |cx| {
                Label::new(cx, "Low Shelf")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "LF Freq", Data::params, |p| &p.lf_freq);
                    components::create_gain_slider(cx, "LF Gain", Data::params, |p| &p.lf_gain);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
            
            // Low Mid section
            VStack::new(cx, |cx| {
                Label::new(cx, "Low Mid")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "LMF Freq", Data::params, |p| &p.lmf_freq);
                    components::create_gain_slider(cx, "LMF Gain", Data::params, |p| &p.lmf_gain);
                    components::create_param_slider(cx, "LMF Q", Data::params, |p| &p.lmf_q);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
            
            // High Shelf section
            VStack::new(cx, |cx| {
                Label::new(cx, "High Shelf")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "HF Freq", Data::params, |p| &p.hf_freq);
                    components::create_gain_slider(cx, "HF Gain", Data::params, |p| &p.hf_gain);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
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
            VStack::new(cx, |cx| {
                components::create_ratio_slider(cx, "Compress", Data::params, |p| &p.comp_compress);
                components::create_gain_slider(cx, "Output", Data::params, |p| &p.comp_output);
                components::create_param_slider(cx, "Dry/Wet", Data::params, |p| &p.comp_dry_wet);
            })
            .class("param-group")
            .gap(Pixels(4.0));
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
            // Low section
            VStack::new(cx, |cx| {
                Label::new(cx, "Low Section")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "Boost Freq", Data::params, |p| &p.pultec_lf_boost_freq);
                    components::create_gain_slider(cx, "Boost Gain", Data::params, |p| &p.pultec_lf_boost_gain);
                })
                .gap(Pixels(4.0));
                components::create_gain_slider(cx, "Cut Gain", Data::params, |p| &p.pultec_lf_cut_gain);
            })
            .class("param-group");
            
            // High section
            VStack::new(cx, |cx| {
                Label::new(cx, "High Section")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_frequency_slider(cx, "Boost Freq", Data::params, |p| &p.pultec_hf_boost_freq);
                    components::create_gain_slider(cx, "Boost Gain", Data::params, |p| &p.pultec_hf_boost_gain);
                })
                .gap(Pixels(4.0));
                components::create_param_slider(cx, "Tube Drive", Data::params, |p| &p.pultec_tube_drive);
            })
            .class("param-group");
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
            // Model selection
            components::create_param_slider(cx, "Model", Data::params, |p| &p.transformer_model);
            
            // Input section
            VStack::new(cx, |cx| {
                Label::new(cx, "Input")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_param_slider(cx, "Drive", Data::params, |p| &p.transformer_input_drive);
                    components::create_param_slider(cx, "Saturation", Data::params, |p| &p.transformer_input_saturation);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
            
            // Output section
            VStack::new(cx, |cx| {
                Label::new(cx, "Output")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_param_slider(cx, "Drive", Data::params, |p| &p.transformer_output_drive);
                    components::create_param_slider(cx, "Saturation", Data::params, |p| &p.transformer_output_saturation);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
            
            // Frequency response
            VStack::new(cx, |cx| {
                Label::new(cx, "Frequency Response")
                    .class("section-title");
                HStack::new(cx, |cx| {
                    components::create_param_slider(cx, "Low", Data::params, |p| &p.transformer_low_response);
                    components::create_param_slider(cx, "High", Data::params, |p| &p.transformer_high_response);
                })
                .gap(Pixels(4.0));
            })
            .class("param-group");
            
            components::create_ratio_slider(cx, "Compression", Data::params, |p| &p.transformer_compression);
        }
    );
}

// Duplicate helpers removed in favor of components::*
