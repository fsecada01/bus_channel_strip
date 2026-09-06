//! The `.bcs` preset file format: a versioned wrapper around a parameter-ID
//! keyed value map (issue #21). Deliberately does NOT carry nice-plug's
//! `PluginState::fields` map — that's where `#[persist]` GUI state like
//! `editor_state` (window size / HiDPI zoom, issue #20) lives, and a preset
//! must never touch it.

use std::collections::BTreeMap;

use nice_plug::plugin::ParamValue;
use serde::{Deserialize, Serialize};

/// Bumped only if the on-disk `.bcs` shape itself changes in a
/// backwards-incompatible way (not on every new parameter — nice-plug's own
/// `set_state()` already skips unknown IDs and leaves missing ones alone).
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// File extension (without the dot) for saved presets.
pub const PRESET_EXTENSION: &str = "bcs";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub schema_version: u32,
    pub name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Every automatable parameter's plain value, keyed by its stable
    /// `#[id]` string. Always contains every parameter the plugin defines
    /// at save time so loading a preset is a full, deterministic reset —
    /// never a partial merge onto whatever was already dialed in.
    pub params: BTreeMap<String, ParamValue>,
}

impl Preset {
    pub fn new(
        name: impl Into<String>,
        category: impl Into<String>,
        params: BTreeMap<String, ParamValue>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: name.into(),
            category: category.into(),
            author: None,
            params,
        }
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> BTreeMap<String, ParamValue> {
        let mut m = BTreeMap::new();
        m.insert("gain".to_string(), ParamValue::F32(0.5));
        m.insert("global_bypass".to_string(), ParamValue::Bool(false));
        m.insert(
            "punch_clip_mode".to_string(),
            ParamValue::String("Soft".to_string()),
        );
        m
    }

    #[test]
    fn round_trips_through_json() {
        let preset = Preset::new("Rock Drum Bus", "Drums", sample_params());
        let json = serde_json::to_string_pretty(&preset).expect("serialize");
        let restored: Preset = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(restored.name, "Rock Drum Bus");
        assert_eq!(restored.category, "Drums");
        assert_eq!(restored.params.len(), 3);
    }

    #[test]
    fn author_is_omitted_from_json_when_absent() {
        let preset = Preset::new("Pop Vocal Bus", "Vocals", sample_params());
        let json = serde_json::to_string(&preset).expect("serialize");
        assert!(!json.contains("author"));
    }

    #[test]
    fn author_round_trips_when_present() {
        let preset = Preset::new("Podcast Voice", "Voice", sample_params()).with_author("Factory");
        let json = serde_json::to_string(&preset).expect("serialize");
        let restored: Preset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.author.as_deref(), Some("Factory"));
    }
}
