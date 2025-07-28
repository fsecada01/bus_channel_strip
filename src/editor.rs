// src/editor.rs
// Temporarily disable the iced GUI implementation. Returns None to skip GUI.
use std::sync::Arc;
use nih_plug::prelude::*;
use nih_plug_iced::IcedState;
use crate::BusChannelStripParams;
// use std::sync::Arc;
// use nih_plug_iced::IcedState;

/// Stubbed GUI: not yet implemented
/// The default GUI size
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(1000, 600)
}

/// Stubbed GUI: not yet implemented
pub(crate) fn create(
    _params: Arc<BusChannelStripParams>,
    _editor_state: Arc<IcedState>,
) -> Option<Box<dyn Editor>> {
    None
}
