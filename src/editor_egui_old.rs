use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, *, CornerRadius};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::sync::Arc;

use crate::{BusChannelStripParams, TransformerModel};

/// GUI width and height
const GUI_WIDTH: f32 = 1000.0;
const GUI_HEIGHT: f32 = 600.0;

/// Module colors (from AGENTS.md guidelines)
const EQ_COLOR: Color32 = Color32::from_rgb(60, 80, 100);           // Blue-gray background
const EQ_ACCENT: Color32 = Color32::from_rgb(0, 200, 255);          // Cyan accents
const COMP_COLOR: Color32 = Color32::from_rgb(40, 40, 40);          // Slate/black
const COMP_ACCENT: Color32 = Color32::from_rgb(255, 140, 0);        // Orange knobs
const PULTEC_COLOR: Color32 = Color32::from_rgb(120, 100, 60);      // Brass tones
const PULTEC_ACCENT: Color32 = Color32::from_rgb(255, 215, 0);      // Gold highlights
const DYNEQ_COLOR: Color32 = Color32::from_rgb(70, 90, 120);        // Steel blue
const DYNEQ_ACCENT: Color32 = Color32::from_rgb(0, 255, 100);       // Green accents
const TRANSFORMER_COLOR: Color32 = Color32::from_rgb(60, 45, 45);   // Charcoal
const TRANSFORMER_ACCENT: Color32 = Color32::from_rgb(200, 80, 60); // Oxide red

/// Create the plugin editor
pub fn create_editor(
    params: Arc<BusChannelStripParams>,
    editor_state: Arc<EguiState>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        editor_state,
        (),
        |ctx, _| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_min_size([GUI_WIDTH, GUI_HEIGHT].into());
                ui.set_max_size([GUI_WIDTH, GUI_HEIGHT].into());
                
                // Main background
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    CornerRadius::same(4),
                    Color32::from_rgb(25, 25, 30), // Dark background
                );
                
                // Title bar
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("BUS CHANNEL STRIP")
                            .size(24.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("v0.1.0")
                                .size(12.0)
                                .color(Color32::LIGHT_GRAY),
                        );
                    });
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Main module layout - horizontal strip
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    
                    // Module 1: API5500 EQ
                    draw_api5500_module(ui, &params);
                    ui.add_space(5.0);
                    
                    // Module 2: ButterComp2
                    draw_buttercomp_module(ui, &params);
                    ui.add_space(5.0);
                    
                    // Module 3: Pultec EQ
                    draw_pultec_module(ui, &params);
                    ui.add_space(5.0);
                    
                    // Module 4: Dynamic EQ
                    draw_dynamic_eq_module(ui, &params);
                    ui.add_space(5.0);
                    
                    // Module 5: Transformer
                    draw_transformer_module(ui, &params);
                    ui.add_space(5.0);
                });
                
                ui.add_space(10.0);
                
                // Master section at bottom
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    
                    // Master gain
                    ui.vertical(|ui| {
                        ui.label(RichText::new("MASTER").size(14.0).strong());
                        ui.add_space(5.0);
                        draw_knob(ui, &params.gain, "GAIN", Color32::WHITE, 50.0);
                    });
                    
                    ui.add_space(20.0);
                    
                    // Signal flow visualization
                    draw_signal_flow(ui);
                });
            });
        },
        move |ctx, setter, _state| {
            // Handle parameter updates
            // This is where we'd process parameter changes from the GUI
        },
    )
}

/// Draw API5500 EQ module
fn draw_api5500_module(ui: &mut Ui, params: &BusChannelStripParams) {
    let module_rect = Rect::from_min_size(ui.cursor().min, [180.0, 380.0].into());
    
    // Module background
    ui.painter().rect_filled(
        module_rect,
        CornerRadius::same(6),
        EQ_COLOR,
    );
    
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
        ui.add_space(8.0);
        
        // Module title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("API 5500 EQ").size(16.0).color(EQ_ACCENT).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                draw_bypass_button(ui, &params.eq_bypass, EQ_ACCENT);
            });
        });
        
        ui.add_space(10.0);
        
        // EQ bands in vertical layout
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                
                // LF and HF (shelving)
                ui.vertical(|ui| {
                    ui.label(RichText::new("LF").size(12.0).color(EQ_ACCENT));
                    draw_knob(ui, &params.lf_freq, "FREQ", EQ_ACCENT, 35.0);
                    draw_knob(ui, &params.lf_gain, "GAIN", EQ_ACCENT, 35.0);
                });
                
                ui.add_space(10.0);
                
                ui.vertical(|ui| {
                    ui.label(RichText::new("HF").size(12.0).color(EQ_ACCENT));
                    draw_knob(ui, &params.hf_freq, "FREQ", EQ_ACCENT, 35.0);
                    draw_knob(ui, &params.hf_gain, "GAIN", EQ_ACCENT, 35.0);
                });
            });
            
            ui.add_space(15.0);
            
            // Parametric bands
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                
                // LMF
                ui.vertical(|ui| {
                    ui.label(RichText::new("LMF").size(10.0).color(EQ_ACCENT));
                    draw_knob(ui, &params.lmf_freq, "F", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.lmf_gain, "G", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.lmf_q, "Q", EQ_ACCENT, 25.0);
                });
                
                // MF
                ui.vertical(|ui| {
                    ui.label(RichText::new("MF").size(10.0).color(EQ_ACCENT));
                    draw_knob(ui, &params.mf_freq, "F", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.mf_gain, "G", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.mf_q, "Q", EQ_ACCENT, 25.0);
                });
                
                // HMF
                ui.vertical(|ui| {
                    ui.label(RichText::new("HMF").size(10.0).color(EQ_ACCENT));
                    draw_knob(ui, &params.hmf_freq, "F", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.hmf_gain, "G", EQ_ACCENT, 25.0);
                    draw_knob(ui, &params.hmf_q, "Q", EQ_ACCENT, 25.0);
                });
            });
        });
    });
}

/// Draw ButterComp2 module
fn draw_buttercomp_module(ui: &mut Ui, params: &BusChannelStripParams) {
    let module_rect = Rect::from_min_size(ui.cursor().min, [140.0, 380.0].into());
    
    // Module background
    ui.painter().rect_filled(
        module_rect,
        CornerRadius::same(6),
        COMP_COLOR,
    );
    
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
        ui.add_space(8.0);
        
        // Module title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("BUTTERCOMP2").size(14.0).color(COMP_ACCENT).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                draw_bypass_button(ui, &params.comp_bypass, COMP_ACCENT);
            });
        });
        
        ui.add_space(20.0);
        
        // Compressor controls
        ui.vertical(|ui| {
            ui.add_space(10.0);
            draw_knob(ui, &params.comp_compress, "COMPRESS", COMP_ACCENT, 50.0);
            ui.add_space(15.0);
            draw_knob(ui, &params.comp_output, "OUTPUT", COMP_ACCENT, 50.0);
            ui.add_space(15.0);
            draw_knob(ui, &params.comp_dry_wet, "MIX", COMP_ACCENT, 50.0);
            
            ui.add_space(20.0);
            
            // Gain reduction meter placeholder
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("GR").size(10.0).color(COMP_ACCENT));
                    draw_meter(ui, 0.3, COMP_ACCENT); // Placeholder value
                });
            });
        });
    });
}

/// Draw Pultec EQ module
fn draw_pultec_module(ui: &mut Ui, params: &BusChannelStripParams) {
    let module_rect = Rect::from_min_size(ui.cursor().min, [160.0, 380.0].into());
    
    // Module background
    ui.painter().rect_filled(
        module_rect,
        CornerRadius::same(6),
        PULTEC_COLOR,
    );
    
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
        ui.add_space(8.0);
        
        // Module title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("PULTEC EQ").size(14.0).color(PULTEC_ACCENT).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                draw_bypass_button(ui, &params.pultec_bypass, PULTEC_ACCENT);
            });
        });
        
        ui.add_space(15.0);
        
        // Pultec controls
        ui.vertical(|ui| {
            // Low frequency section
            ui.label(RichText::new("LOW FREQUENCY").size(11.0).color(PULTEC_ACCENT));
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    draw_knob(ui, &params.pultec_lf_boost_freq, "FREQ", PULTEC_ACCENT, 30.0);
                    draw_knob(ui, &params.pultec_lf_boost_gain, "BOOST", PULTEC_ACCENT, 30.0);
                });
                ui.vertical(|ui| {
                    ui.add_space(30.0); // Align with boost
                    draw_knob(ui, &params.pultec_lf_cut_gain, "ATTEN", PULTEC_ACCENT, 30.0);
                });
            });
            
            ui.add_space(15.0);
            
            // High frequency section
            ui.label(RichText::new("HIGH FREQUENCY").size(11.0).color(PULTEC_ACCENT));
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    draw_knob(ui, &params.pultec_hf_boost_freq, "FREQ", PULTEC_ACCENT, 25.0);
                    draw_knob(ui, &params.pultec_hf_boost_gain, "BOOST", PULTEC_ACCENT, 25.0);
                    draw_knob(ui, &params.pultec_hf_boost_bandwidth, "BW", PULTEC_ACCENT, 25.0);
                });
                ui.vertical(|ui| {
                    draw_knob(ui, &params.pultec_hf_cut_freq, "CUT F", PULTEC_ACCENT, 25.0);
                    draw_knob(ui, &params.pultec_hf_cut_gain, "ATTEN", PULTEC_ACCENT, 25.0);
                });
            });
            
            ui.add_space(10.0);
            
            // Tube drive
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                draw_knob(ui, &params.pultec_tube_drive, "TUBE", PULTEC_ACCENT, 35.0);
            });
        });
    });
}

/// Draw Dynamic EQ module
fn draw_dynamic_eq_module(ui: &mut Ui, params: &BusChannelStripParams) {
    let module_rect = Rect::from_min_size(ui.cursor().min, [200.0, 380.0].into());
    
    // Module background
    ui.painter().rect_filled(
        module_rect,
        CornerRadius::same(6),
        DYNEQ_COLOR,
    );
    
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
        ui.add_space(8.0);
        
        // Module title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("DYNAMIC EQ").size(14.0).color(DYNEQ_ACCENT).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                ui.label("Dynamic EQ - Feature Disabled");
            });
        });
        
        ui.add_space(10.0);
        
        // 4 bands in compact layout
        ui.horizontal(|ui| {
            ui.add_space(5.0);
            ui.label("Dynamic EQ - Not Available (Feature Disabled)");
        });
    });
}

// Dynamic EQ band drawing removed - feature disabled

/// Draw Transformer module
fn draw_transformer_module(ui: &mut Ui, params: &BusChannelStripParams) {
    let module_rect = Rect::from_min_size(ui.cursor().min, [160.0, 380.0].into());
    
    // Module background
    ui.painter().rect_filled(
        module_rect,
        CornerRadius::same(6),
        TRANSFORMER_COLOR,
    );
    
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::LEFT), |ui| {
        ui.add_space(8.0);
        
        // Module title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("TRANSFORMER").size(13.0).color(TRANSFORMER_ACCENT).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                draw_bypass_button(ui, &params.transformer_bypass, TRANSFORMER_ACCENT);
            });
        });
        
        ui.add_space(10.0);
        
        // Transformer controls
        ui.vertical(|ui| {
            // Model selector
            draw_enum_selector(ui, &params.transformer_model, "MODEL", TRANSFORMER_ACCENT);
            
            ui.add_space(10.0);
            
            // Input stage
            ui.label(RichText::new("INPUT").size(11.0).color(TRANSFORMER_ACCENT));
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_input_drive, "DRIVE", TRANSFORMER_ACCENT, 30.0);
                });
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_input_saturation, "SAT", TRANSFORMER_ACCENT, 30.0);
                });
            });
            
            ui.add_space(10.0);
            
            // Output stage
            ui.label(RichText::new("OUTPUT").size(11.0).color(TRANSFORMER_ACCENT));
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_output_drive, "DRIVE", TRANSFORMER_ACCENT, 30.0);
                });
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_output_saturation, "SAT", TRANSFORMER_ACCENT, 30.0);
                });
            });
            
            ui.add_space(10.0);
            
            // Frequency response
            ui.label(RichText::new("FREQ RESP").size(10.0).color(TRANSFORMER_ACCENT));
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_low_response, "LOW", TRANSFORMER_ACCENT, 25.0);
                });
                ui.vertical(|ui| {
                    draw_knob(ui, &params.transformer_high_response, "HIGH", TRANSFORMER_ACCENT, 25.0);
                });
            });
            
            ui.add_space(10.0);
            
            // Compression
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                draw_knob(ui, &params.transformer_compression, "COMP", TRANSFORMER_ACCENT, 35.0);
            });
        });
    });
}

/// Draw a parameter knob
fn draw_knob(ui: &mut Ui, param: &impl Param, label: &str, color: Color32, size: f32) {
    ui.vertical_centered(|ui| {
        // Parameter control (NIH-plug will handle the actual knob rendering)
        let response = ui.add_sized([size, size], 
            egui::Slider::new(&mut 0.5f32, 0.0..=1.0)
                .show_value(false)
        );
        
        // Draw custom knob appearance
        let rect = response.rect;
        let center = rect.center();
        let radius = rect.width() * 0.4;
        
        // Knob base
        ui.painter().circle_filled(center, radius, Color32::from_rgb(40, 40, 40));
        ui.painter().circle_stroke(center, radius, Stroke::new(1.0, color));
        
        // Knob indicator
        let angle = (0.5f32 - 0.5) * 2.0 * std::f32::consts::PI * 0.75;
        let indicator_end = center + Vec2::new(
            angle.cos() * radius * 0.7,
            angle.sin() * radius * 0.7,
        );
        ui.painter().line_segment([center, indicator_end], Stroke::new(2.0, color));
        
        ui.add_space(2.0);
        ui.label(RichText::new(label).size(9.0).color(Color32::LIGHT_GRAY));
    });
}

/// Draw a mini knob for compact layouts
fn draw_mini_knob(ui: &mut Ui, param: &impl Param, label: &str, color: Color32) {
    draw_knob(ui, param, label, color, 20.0);
}

/// Draw a bypass button
fn draw_bypass_button(ui: &mut Ui, param: &BoolParam, color: Color32) {
    let is_active = !param.value(); // Bypass is inverted (true = bypassed)
    let button_color = if is_active { color } else { Color32::DARK_GRAY };
    
    if ui.add_sized([30.0, 20.0], 
        egui::Button::new(RichText::new("ON").size(10.0))
            .fill(button_color)
    ).clicked() {
        // Toggle bypass
        // param.set_normalized_value(if param.value() { 0.0 } else { 1.0 });
    }
}

/// Draw a mini button
fn draw_mini_button(ui: &mut Ui, param: &BoolParam, label: &str, color: Color32) {
    let is_active = param.value();
    let button_color = if is_active { color } else { Color32::DARK_GRAY };
    
    if ui.add_sized([20.0, 15.0], 
        egui::Button::new(RichText::new(label).size(8.0))
            .fill(button_color)
    ).clicked() {
        // Toggle parameter
        // param.set_normalized_value(if param.value() { 0.0 } else { 1.0 });
    }
}

/// Draw an enum selector (for transformer model)
fn draw_enum_selector(ui: &mut Ui, param: &EnumParam<TransformerModel>, label: &str, color: Color32) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(label).size(9.0).color(Color32::LIGHT_GRAY));
        
        let current_value = param.value();
        let model_name = match current_value {
            TransformerModel::Vintage => "Vintage",
            TransformerModel::Modern => "Modern", 
            TransformerModel::British => "British",
            TransformerModel::American => "American",
        };
        
        if ui.add_sized([70.0, 20.0],
            egui::Button::new(RichText::new(model_name).size(10.0))
                .fill(Color32::from_rgb(60, 60, 60))
        ).clicked() {
            // Cycle through transformer models
            // Implementation would cycle to next model
        }
    });
}

/// Draw a gain reduction meter
fn draw_meter(ui: &mut Ui, level: f32, color: Color32) {
    let meter_rect = Rect::from_min_size(ui.cursor().min, [15.0, 80.0].into());
    
    // Meter background
    ui.painter().rect_filled(
        meter_rect,
        CornerRadius::same(2),
        Color32::from_rgb(20, 20, 20),
    );
    
    // Meter level
    let level_height = meter_rect.height() * level;
    let level_rect = Rect::from_min_size(
        [meter_rect.min.x, meter_rect.max.y - level_height].into(),
        [meter_rect.width(), level_height].into(),
    );
    
    ui.painter().rect_filled(
        level_rect,
        CornerRadius::same(1),
        color,
    );
    
    ui.allocate_rect(meter_rect, Sense::hover());
}

/// Draw signal flow visualization
fn draw_signal_flow(ui: &mut Ui) {
    ui.vertical(|ui| {
        ui.label(RichText::new("SIGNAL FLOW").size(12.0).color(Color32::LIGHT_GRAY));
        ui.add_space(5.0);
        
        ui.horizontal(|ui| {
            let modules = ["EQ", "COMP", "PULTEC", "DYN EQ", "XFRM"];
            let colors = [EQ_ACCENT, COMP_ACCENT, PULTEC_ACCENT, DYNEQ_ACCENT, TRANSFORMER_ACCENT];
            
            for (i, (module, color)) in modules.iter().zip(colors.iter()).enumerate() {
                // Module indicator
                ui.add_sized([40.0, 15.0], 
                    egui::Button::new(RichText::new(*module).size(8.0))
                        .fill(*color)
                );
                
                // Arrow (except for last module)
                if i < modules.len() - 1 {
                    ui.label(RichText::new("→").size(12.0).color(Color32::WHITE));
                }
            }
        });
    });
}