// src/components.rs
// Reusable UI components for the Bus Channel Strip editor

use std::sync::Arc;
use nih_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;

use crate::BusChannelStripParams;

// Theme configuration for modules
#[derive(Clone, Copy)]
pub enum ModuleTheme {
    Api5500,
    ButterComp2,
    Pultec,
    DynamicEq,
    Transformer,
    Master,
}

impl ModuleTheme {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Api5500 => "api5500-theme",
            Self::ButterComp2 => "buttercomp2-theme", 
            Self::Pultec => "pultec-theme",
            Self::DynamicEq => "dynamic-eq-theme",
            Self::Transformer => "transformer-theme",
            Self::Master => "master-section",
        }
    }
    
    pub fn accent_color(self) -> Color {
        match self {
            Self::Api5500 => Color::rgb(64, 160, 208),      // #40a0d0
            Self::ButterComp2 => Color::rgb(255, 150, 64),  // #ff9640
            Self::Pultec => Color::rgb(255, 215, 0),        // #ffd700
            Self::DynamicEq => Color::rgb(102, 204, 102),   // #66cc66
            Self::Transformer => Color::rgb(204, 102, 51),  // #cc6633
            Self::Master => Color::rgb(85, 85, 85),         // #555555
        }
    }
}

// Parameter group configuration
pub struct ParamGroup<'a> {
    pub label: &'a str,
    pub params: Vec<ParamConfig<'a>>,
}

pub struct ParamConfig<'a> {
    pub label: &'a str,
    // Remove the problematic generic param function for now
    // pub param_fn: Box<dyn Fn(&Arc<BusChannelStripParams>) -> &(dyn Param<Plain = f32> + 'a)>,
}

// Reusable module section component
pub fn create_module_section<'a, F>(
    cx: &mut Context,
    title: &str,
    theme: ModuleTheme,
    bypass_param: Option<F>,
    content_builder: impl FnOnce(&mut Context) + 'a,
) where
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    VStack::new(cx, |cx| {
        // Module title
        Label::new(cx, title)
            .class("module-title");
        
        // Bypass button if provided
        if let Some(bypass_fn) = bypass_param {
            create_bypass_button(cx, "Bypass", bypass_fn);
        }
        
        // Module content
        content_builder(cx);
    })
    .class("module-section")
    .class(theme.class_name())
    .width(Stretch(1.0))
    .height(Auto);
}

// Reusable parameter group component - simplified for now
pub fn create_param_group(
    cx: &mut Context,
    label: &str,
    content_builder: impl FnOnce(&mut Context),
) {
    VStack::new(cx, |cx| {
        if !label.is_empty() {
            Label::new(cx, label)
                .font_size(12.0)
                .color(Color::rgb(200, 200, 200));
        }
        content_builder(cx);
    })
    .class("param-group");
}

pub enum ParamLayout {
    Horizontal,
    Vertical,
    Grid(usize), // number of columns
}

// Enhanced parameter slider with consistent styling
pub fn create_param_slider<P, L, F>(
    cx: &mut Context,
    label: &str,
    lens: L,
    param_map: F,
) where
    P: Param + 'static,
    L: Lens<Target = Arc<BusChannelStripParams>> + Clone + 'static,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &P,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label");
        
        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Pixels(80.0));
    })
    .class("param-control")
    .width(Pixels(90.0))
    .height(Auto);
}

// Removed problematic raw param slider function for now

// Reusable bypass button component
pub fn create_bypass_button<F>(
    cx: &mut Context,
    _label: &str,
    param_map: F,
) where
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    // Create the button with proper lens binding
    ParamButton::new(cx, crate::editor::Data::params, param_map)
        .class("bypass-button")
        .height(Pixels(30.0))
        .width(Pixels(70.0));
}

// Specialized components for common parameter types

pub fn create_frequency_slider<L, F>(
    cx: &mut Context,
    label: &str,
    lens: L,
    param_map: F,
) where
    L: Lens<Target = Arc<BusChannelStripParams>> + Clone + 'static,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label");
        
        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Pixels(80.0))
            .class("frequency-slider");
    })
    .class("param-control")
    .class("frequency-control")
    .width(Pixels(90.0));
}

pub fn create_gain_slider<L, F>(
    cx: &mut Context,
    label: &str,
    lens: L,
    param_map: F,
) where
    L: Lens<Target = Arc<BusChannelStripParams>> + Clone + 'static,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label");
        
        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Pixels(80.0))
            .class("gain-slider");
    })
    .class("param-control")
    .class("gain-control")
    .width(Pixels(90.0));
}

pub fn create_ratio_slider<L, F>(
    cx: &mut Context,
    label: &str,
    lens: L,
    param_map: F,
) where
    L: Lens<Target = Arc<BusChannelStripParams>> + Clone + 'static,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label");
        
        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Pixels(80.0))
            .class("ratio-slider");
    })
    .class("param-control")
    .class("ratio-control")
    .width(Pixels(90.0));
}

// Section builder helper for complex modules
pub struct SectionBuilder<'a> {
    cx: &'a mut Context,
    theme: ModuleTheme,
}

impl<'a> SectionBuilder<'a> {
    pub fn new(cx: &'a mut Context, theme: ModuleTheme) -> Self {
        Self { cx, theme }
    }
    
    pub fn with_title(self, title: &str) -> Self {
        Label::new(self.cx, title)
            .class("section-title")
            .color(self.theme.accent_color());
        self
    }
    
    pub fn with_horizontal_params<F>(self, builder: F) -> Self 
    where
        F: FnOnce(&mut Context),
    {
        HStack::new(self.cx, builder)
            .gap(Pixels(4.0));
        self
    }
    
    pub fn with_vertical_params<F>(self, builder: F) -> Self
    where
        F: FnOnce(&mut Context),
    {
        VStack::new(self.cx, builder)
            .gap(Pixels(4.0));
        self
    }
}
