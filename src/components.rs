// src/components.rs
// Reusable UI components for the Bus Channel Strip editor

use std::sync::Arc;
use nih_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;

use crate::BusChannelStripParams;

// ── Layout constants ──────────────────────────────────────────────────────────
// In morphorm, height(Auto) on a leaf node (no children) resolves to 0, not
// text-content height. Labels inside Auto-height VStacks MUST use explicit
// Pixels heights to avoid collapsing to 0 and overflowing onto sibling views.
const PARAM_LABEL_H: f32 = 16.0;  // height for all parameter labels (12px font + padding)

// ── Reusable structural helpers ───────────────────────────────────────────────

/// Horizontal row of parameter controls. Returns the Handle so callers can
/// chain layout modifiers (e.g. `.top(Stretch(1.0))` for dynamic spacing).
pub fn module_row(cx: &mut Context, builder: impl FnOnce(&mut Context)) -> Handle<'_, HStack> {
    HStack::new(cx, builder)
        .height(Auto)
        .width(Stretch(1.0))
        .gap(Pixels(6.0))
}

/// Titled section group: renders a section-label + vertical stack of controls.
/// Single point of failure for all labeled sections across all modules.
pub fn module_section(cx: &mut Context, title: &str, builder: impl FnOnce(&mut Context)) {
    VStack::new(cx, |cx| {
        Label::new(cx, title)
            .class("section-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));
        builder(cx);
    })
    .height(Auto)
    .width(Stretch(1.0))
    .gap(Pixels(4.0));
}

// Theme configuration for modules
#[derive(Clone, Copy)]
pub enum ModuleTheme {
    Api5500,
    ButterComp2,
    Pultec,
    DynamicEq,
    Transformer,
    Punch,
}

impl ModuleTheme {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Api5500     => "api5500-theme",
            Self::ButterComp2 => "buttercomp2-theme",
            Self::Pultec      => "pultec-theme",
            Self::DynamicEq   => "dynamic-eq-theme",
            Self::Transformer => "transformer-theme",
            Self::Punch       => "punch-theme",
        }
    }

    pub fn accent_color(self) -> Color {
        match self {
            Self::Api5500     => Color::rgb(64,  160, 208), // #40a0d0
            Self::ButterComp2 => Color::rgb(255, 150, 64),  // #ff9640
            Self::Pultec      => Color::rgb(255, 215, 0),   // #ffd700
            Self::DynamicEq   => Color::rgb(102, 204, 102), // #66cc66
            Self::Transformer => Color::rgb(204, 102, 51),  // #cc6633
            Self::Punch       => Color::rgb(0,   160, 255), // #00a0ff
        }
    }
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
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Stretch(1.0));
    })
    .class("param-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
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
        .height(Pixels(28.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
}

/// Band enable button. Uses the "on-button" CSS class which inverts the visual
/// convention: the checked/lit state (param=true = enabled) appears DARK like
/// normal operation, while the unchecked state (disabled) appears lit/red.
/// This matches the bypass button convention where dark = normal/processing.
pub fn create_on_button<F>(
    cx: &mut Context,
    param_map: F,
) where
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    ParamButton::new(cx, crate::editor::Data::params, param_map)
        .class("on-button")
        .height(Pixels(28.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
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
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Stretch(1.0))
            .class("frequency-slider");
    })
    .class("param-control")
    .class("frequency-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
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
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Stretch(1.0))
            .class("gain-slider");
    })
    .class("param-control")
    .class("gain-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
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
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        ParamSlider::new(cx, lens, param_map)
            .height(Pixels(20.0))
            .width(Stretch(1.0))
            .class("ratio-slider");
    })
    .class("param-control")
    .class("ratio-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

