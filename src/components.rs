// src/components.rs
// Reusable UI components for the Bus Channel Strip editor

use nice_plug::prelude::*;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::*;

use crate::icons::{Icon, IconKind};
use crate::BusChannelStripParams;

// ── Layout constants ──────────────────────────────────────────────────────────
// In morphorm, height(Auto) on a leaf node (no children) resolves to 0, not
// text-content height. Labels inside Auto-height VStacks MUST use explicit
// Pixels heights to avoid collapsing to 0 and overflowing onto sibling views.
const PARAM_LABEL_H: f32 = 16.0; // height for all parameter labels (12px font + padding)

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
pub fn module_section(cx: &mut Context, title: &'static str, builder: impl FnOnce(&mut Context)) {
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

// Theme configuration for modules. `Empty` is the unoccupied-slot variant —
// rendered as a muted dashed placeholder with the library picker inside.
#[derive(Clone, Copy)]
pub enum ModuleTheme {
    Api5500,
    ButterComp2,
    Pultec,
    DynamicEq,
    Transformer,
    Punch,
    Haas,
    Empty,
}

impl ModuleTheme {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Api5500 => "api5500-theme",
            Self::ButterComp2 => "buttercomp2-theme",
            Self::Pultec => "pultec-theme",
            Self::DynamicEq => "dynamic-eq-theme",
            Self::Transformer => "transformer-theme",
            Self::Punch => "punch-theme",
            Self::Haas => "haas-theme",
            Self::Empty => "empty-theme",
        }
    }

    pub fn accent_color(self) -> Color {
        match self {
            Self::Api5500 => Color::rgb(64, 160, 208),     // #40a0d0
            Self::ButterComp2 => Color::rgb(255, 150, 64), // #ff9640
            Self::Pultec => Color::rgb(255, 215, 0),       // #ffd700
            Self::DynamicEq => Color::rgb(102, 204, 102),  // #66cc66
            Self::Transformer => Color::rgb(204, 102, 51), // #cc6633
            Self::Punch => Color::rgb(255, 51, 68),        // #ff3344 (red/orange per spec)
            Self::Haas => Color::rgb(140, 160, 210),       // #8ca0d2 (muted blue-lavender)
            Self::Empty => Color::rgb(110, 116, 128),      // #6e7480 (neutral steel)
        }
    }
}

/// Wraps a `ParamSlider` with a floating value tooltip that follows the
/// cursor during drag (roadmap v2.0 §4.4 "Knob micro-interactions").
/// `ParamSlider` is a sealed external widget (vizia_plug) with no override
/// hooks, so this observes `MouseMove` bubbling out of it rather than
/// hooking into its internal drag state — its own `WindowEvent::MouseMove`
/// handler never calls `meta.consume()`. Drag-vs-hover is inferred from the
/// left mouse button being held, since `ParamSlider` exposes no drag-state
/// API to read directly.
pub(crate) fn param_slider_with_tooltip<'c, 'p, P, F>(
    cx: &'c mut Context,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) -> Handle<'c, ZStack>
where
    'p: 'c,
    P: Param + 'static,
    F: 'static + Clone + Copy + Send + Sync + Fn(&Arc<BusChannelStripParams>) -> &P,
{
    let tooltip_visible = SyncSignal::new(false);
    let tooltip_text: SyncSignal<String> = SyncSignal::new(String::new());
    let tooltip_x = SyncSignal::new(0.0_f32);
    let params_owned = params.clone();

    ZStack::new(cx, |cx| {
        ParamSlider::new(cx, param_map(params))
            .height(Stretch(1.0))
            .width(Stretch(1.0));
        Label::new(cx, tooltip_text)
            .class("param-drag-tooltip")
            .position_type(PositionType::Absolute)
            .display(tooltip_visible.map(|v| if *v { Display::Flex } else { Display::None }))
            .left(tooltip_x.map(|x| Pixels(*x)))
            .hoverable(false);
    })
    .on_mouse_move(move |cx, x, _y| {
        if cx.mouse().left.state == MouseButtonState::Pressed {
            let bounds = cx.bounds();
            let p = param_map(&params_owned);
            let value_text = p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
            tooltip_text.set(value_text);
            tooltip_x.set((x - bounds.x - 20.0).max(0.0));
            tooltip_visible.set(true);
        } else if tooltip_visible.get() {
            tooltip_visible.set(false);
        }
    })
    .height(Pixels(20.0))
    .width(Stretch(1.0))
}

// Enhanced parameter slider with consistent styling
pub fn create_param_slider<'c, 'p, P, F>(
    cx: &'c mut Context,
    label: &'static str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    P: Param + 'static,
    F: 'static + Clone + Copy + Send + Sync + Fn(&Arc<BusChannelStripParams>) -> &P,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        param_slider_with_tooltip(cx, params, param_map);
    })
    .class("param-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

// Removed problematic raw param slider function for now

// Reusable bypass button component
pub fn create_bypass_button<'c, 'p, F>(
    cx: &'c mut Context,
    _label: &str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    ParamButton::new(cx, param_map(params))
        .class("bypass-button")
        .height(Pixels(28.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
}

/// Hardware-LED-style bypass button. Visual convention is inverted from the
/// underlying BoolParam: when the module is ACTIVE (bypass=false) the button
/// is lit green; when BYPASSED (bypass=true, i.e. ParamButton :checked) it
/// appears dark/off. Label reads "ACTIVE" in both states — users read the
/// color, not the text, matching how outboard gear works.
pub fn create_active_led_button<'c, 'p, F>(
    cx: &'c mut Context,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    ParamButton::new(cx, param_map(params))
        .with_label("ACTIVE")
        .class("active-led-button")
        .height(Pixels(28.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
}

/// Band enable button. Uses the "on-button" CSS class which inverts the visual
/// convention: the checked/lit state (param=true = enabled) appears DARK like
/// normal operation, while the unchecked state (disabled) appears lit/red.
/// This matches the bypass button convention where dark = normal/processing.
pub fn create_on_button<'c, 'p, F>(
    cx: &'c mut Context,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    ParamButton::new(cx, param_map(params))
        .class("on-button")
        .height(Pixels(28.0))
        .width(Stretch(1.0))
        .top(Pixels(0.0))
        .bottom(Pixels(0.0));
}

/// Inline labeled toggle button for BoolParam controls inside a module's control surface.
/// Renders a label above a full-width button, matching the slider layout so heights
/// stay consistent when mixed with param sliders in the same row.
pub fn create_bool_button<'c, 'p, F>(
    cx: &'c mut Context,
    label: &'static str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));
        ParamButton::new(cx, param_map(params))
            .class("bool-button")
            .height(Pixels(20.0))
            .width(Stretch(1.0));
    })
    .class("param-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

/// Collapsed-tab expand button: a full-size clickable `ParamButton` (empty
/// label) with a `ChevronRight` [`Icon`] overlaid on top. Replaces the
/// previous `"\u{25B6}"` glyph label — see `docs/adr` and roadmap v2.0 §4.4.
pub fn create_expand_button<'c, 'p, F>(
    cx: &'c mut Context,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Fn(&Arc<BusChannelStripParams>) -> &BoolParam,
{
    ZStack::new(cx, |cx| {
        ParamButton::new(cx, param_map(params))
            .with_label("")
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        Icon::new(
            cx,
            IconKind::ChevronRight,
            vg::Color::from_argb(255, 224, 224, 224),
        )
        .hoverable(false)
        .width(Pixels(12.0))
        .height(Pixels(12.0));
    })
    .class("expand-btn");
}

// Specialized components for common parameter types

pub fn create_frequency_slider<'c, 'p, F>(
    cx: &'c mut Context,
    label: &'static str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Send + Sync + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        param_slider_with_tooltip(cx, params, param_map).class("frequency-slider");
    })
    .class("param-control")
    .class("frequency-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

pub fn create_gain_slider<'c, 'p, F>(
    cx: &'c mut Context,
    label: &'static str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Send + Sync + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        param_slider_with_tooltip(cx, params, param_map).class("gain-slider");
    })
    .class("param-control")
    .class("gain-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}

pub fn create_ratio_slider<'c, 'p, F>(
    cx: &'c mut Context,
    label: &'static str,
    params: &'p Arc<BusChannelStripParams>,
    param_map: F,
) where
    'p: 'c,
    F: 'static + Clone + Copy + Send + Sync + Fn(&Arc<BusChannelStripParams>) -> &FloatParam,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .class("param-label")
            .height(Pixels(PARAM_LABEL_H))
            .width(Stretch(1.0));

        param_slider_with_tooltip(cx, params, param_map).class("ratio-slider");
    })
    .class("param-control")
    .class("ratio-control")
    .width(Stretch(1.0))
    .height(Auto)
    .top(Pixels(0.0))
    .bottom(Pixels(0.0));
}
