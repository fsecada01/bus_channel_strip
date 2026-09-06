//! The ~20 factory presets shipped with the plugin (issue #21 §DoD). Each
//! preset is built by taking a full snapshot of every parameter's default
//! value (via [`default_param_values`]) and overlaying a small, curated list
//! of overrides — so loading any factory preset is always a complete,
//! deterministic reset of all ~152 parameters, never a partial merge onto
//! whatever was already dialed in.

use std::collections::BTreeMap;

use nice_plug::plugin::ParamValue;
use nice_plug::util;

use super::schema::Preset;
use super::value_ext::extract_param_values;

const FACTORY_AUTHOR: &str = "Bus Channel Strip Factory";

/// Every parameter's default value, keyed by its stable `#[id]` string. The
/// base every factory (and user) preset overlays its overrides onto.
pub fn default_param_values() -> BTreeMap<String, ParamValue> {
    extract_param_values(&crate::params::BusChannelStripParams::default())
}

fn build_preset(name: &str, category: &str, overrides: &[(&str, ParamValue)]) -> Preset {
    let mut params = default_param_values();
    for (id, value) in overrides {
        params.insert((*id).to_string(), value.clone());
    }
    Preset::new(name, category, params).with_author(FACTORY_AUTHOR)
}

fn db(value: f32) -> ParamValue {
    ParamValue::F32(util::db_to_gain(value))
}

fn f32v(value: f32) -> ParamValue {
    ParamValue::F32(value)
}

fn boolv(value: bool) -> ParamValue {
    ParamValue::Bool(value)
}

fn strv(value: &str) -> ParamValue {
    ParamValue::String(value.to_string())
}

/// The full factory preset library, grouped by category.
pub fn factory_presets() -> Vec<Preset> {
    vec![
        // ── Drums ────────────────────────────────────────────────────────
        build_preset(
            "Rock Drum Bus",
            "Drums",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(3.0)),
                ("hf_gain", f32v(2.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-14.0)),
                ("comp_vca_ratio", f32v(4.0)),
                ("comp_vca_atk", f32v(5.0)),
                ("comp_vca_rel", f32v(80.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Soft")),
                ("punch_attack", f32v(0.4)),
                ("punch_sustain", f32v(0.1)),
            ],
        ),
        build_preset(
            "Punchy Pop Drums",
            "Drums",
            &[
                ("eq_bypass", boolv(false)),
                ("hf_gain", f32v(3.0)),
                ("hmf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("1176 FET")),
                ("comp_fet_ratio", strv("4:1")),
                ("comp_fet_atk", f32v(0.1)),
                ("comp_fet_rel", f32v(150.0)),
                ("comp_fet_input", f32v(6.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Soft")),
                ("punch_attack", f32v(0.5)),
                ("punch_sustain", f32v(0.2)),
            ],
        ),
        build_preset(
            "Trap Drum Bus",
            "Drums",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_freq", f32v(60.0)),
                ("lf_gain", f32v(4.0)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-20.0)),
                ("comp_vca_ratio", f32v(6.0)),
                ("comp_vca_atk", f32v(2.0)),
                ("comp_vca_rel", f32v(60.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Hard")),
                ("punch_threshold", f32v(-1.0)),
                ("punch_attack", f32v(0.6)),
            ],
        ),
        build_preset(
            "Jazz Brushes Kit",
            "Drums",
            &[
                ("eq_bypass", boolv(false)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-16.0)),
                ("comp_opt_speed", f32v(0.3)),
                ("comp_opt_char", f32v(0.6)),
                ("sheen_air_db", f32v(2.5)),
            ],
        ),
        // ── Vocals ───────────────────────────────────────────────────────
        build_preset(
            "Pop Vocal Bus",
            "Vocals",
            &[
                ("eq_bypass", boolv(false)),
                ("hmf_gain", f32v(2.0)),
                ("hf_gain", f32v(1.5)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-18.0)),
                ("comp_opt_speed", f32v(0.6)),
                ("comp_opt_char", f32v(0.5)),
                ("pultec_bypass", boolv(false)),
                ("pultec_hf_boost_gain", f32v(2.0)),
                ("dyneq_bypass", boolv(false)),
                ("dyneq_band4_threshold", f32v(-24.0)),
                ("dyneq_band4_ratio", f32v(4.0)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("American")),
                ("transformer_input_drive", f32v(0.15)),
            ],
        ),
        build_preset(
            "Rock Lead Vocal",
            "Vocals",
            &[
                ("eq_bypass", boolv(false)),
                ("lmf_gain", f32v(-2.0)),
                ("hmf_gain", f32v(2.5)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("1176 FET")),
                ("comp_fet_ratio", strv("8:1")),
                ("comp_fet_atk", f32v(0.1)),
                ("comp_fet_rel", f32v(250.0)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("British")),
                ("transformer_input_drive", f32v(0.3)),
                ("transformer_input_saturation", f32v(0.4)),
            ],
        ),
        build_preset(
            "R&B Vocal Silk",
            "Vocals",
            &[
                ("eq_bypass", boolv(false)),
                ("hf_gain", f32v(2.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-20.0)),
                ("comp_opt_speed", f32v(0.4)),
                ("comp_opt_char", f32v(0.7)),
                ("pultec_bypass", boolv(false)),
                ("pultec_hf_boost_gain", f32v(3.0)),
                ("pultec_hf_boost_freq", f32v(12000.0)),
                ("sheen_air_db", f32v(2.5)),
                ("sheen_warmth", f32v(0.3)),
            ],
        ),
        build_preset(
            "Rap Vocal Bus",
            "Vocals",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(-3.0)),
                ("hmf_gain", f32v(3.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("1176 FET")),
                ("comp_fet_ratio", strv("4:1")),
                ("comp_fet_input", f32v(8.0)),
                ("comp_fet_output", f32v(-2.0)),
                ("dyneq_bypass", boolv(false)),
                ("dyneq_band4_threshold", f32v(-20.0)),
                ("dyneq_band4_ratio", f32v(5.0)),
            ],
        ),
        // ── Voice ────────────────────────────────────────────────────────
        build_preset(
            "Podcast Voice",
            "Voice",
            &[
                ("global_auto_gain", boolv(true)),
                ("eq_bypass", boolv(false)),
                ("lf_freq", f32v(100.0)),
                ("lf_gain", f32v(-2.0)),
                ("hmf_gain", f32v(1.5)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-24.0)),
                ("comp_vca_ratio", f32v(3.0)),
                ("comp_vca_atk", f32v(15.0)),
                ("comp_vca_rel", f32v(150.0)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("Modern")),
                ("transformer_input_drive", f32v(0.1)),
            ],
        ),
        build_preset(
            "Radio Voice Broadcast",
            "Voice",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(-3.0)),
                ("hmf_gain", f32v(2.5)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-20.0)),
                ("comp_vca_ratio", f32v(8.0)),
                ("comp_vca_atk", f32v(5.0)),
                ("comp_vca_rel", f32v(80.0)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("Vintage")),
                ("transformer_compression", f32v(0.5)),
            ],
        ),
        // ── Mastering ────────────────────────────────────────────────────
        build_preset(
            "Hip-Hop Master",
            "Mastering",
            &[
                ("gain", db(1.0)),
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(2.0)),
                ("hf_gain", f32v(1.5)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-10.0)),
                ("comp_vca_ratio", f32v(2.0)),
                ("comp_vca_atk", f32v(30.0)),
                ("comp_vca_rel", f32v(200.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Hard")),
                ("punch_threshold", f32v(-0.3)),
                ("punch_attack", f32v(0.3)),
                ("sheen_body_db", f32v(2.0)),
                ("sheen_air_db", f32v(2.5)),
            ],
        ),
        build_preset(
            "EDM Master",
            "Mastering",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(2.5)),
                ("hf_gain", f32v(2.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-8.0)),
                ("comp_vca_ratio", f32v(3.0)),
                ("comp_vca_atk", f32v(1.0)),
                ("comp_vca_rel", f32v(100.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Cubic")),
                ("punch_threshold", f32v(-0.5)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("Modern")),
                ("transformer_compression", f32v(0.2)),
                ("sheen_width", f32v(0.7)),
                ("sheen_air_db", f32v(3.0)),
            ],
        ),
        build_preset(
            "Jazz Acoustic Master",
            "Mastering",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(0.5)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-12.0)),
                ("comp_opt_speed", f32v(0.2)),
                ("comp_opt_char", f32v(0.4)),
                ("pultec_bypass", boolv(false)),
                ("pultec_lf_boost_gain", f32v(1.0)),
                ("pultec_hf_boost_gain", f32v(1.5)),
                ("sheen_warmth", f32v(0.15)),
            ],
        ),
        build_preset(
            "Classical Orchestra",
            "Mastering",
            &[
                ("global_auto_gain", boolv(false)),
                ("eq_bypass", boolv(false)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-10.0)),
                ("comp_opt_speed", f32v(0.15)),
                ("comp_opt_char", f32v(0.3)),
                ("sheen_bypass", boolv(true)),
            ],
        ),
        build_preset(
            "Pop Master Loud",
            "Mastering",
            &[
                ("gain", db(2.0)),
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(1.5)),
                ("hf_gain", f32v(2.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-6.0)),
                ("comp_vca_ratio", f32v(4.0)),
                ("comp_vca_atk", f32v(10.0)),
                ("comp_vca_rel", f32v(100.0)),
                ("punch_bypass", boolv(false)),
                ("punch_clip_mode", strv("Soft")),
                ("punch_threshold", f32v(-0.2)),
                ("punch_softness", f32v(0.5)),
                ("sheen_body_db", f32v(2.0)),
                ("sheen_air_db", f32v(3.0)),
            ],
        ),
        build_preset(
            "Lo-Fi Master",
            "Mastering",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(1.0)),
                ("hf_gain", f32v(-3.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-14.0)),
                ("comp_opt_speed", f32v(0.5)),
                ("comp_opt_char", f32v(0.8)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("Vintage")),
                ("transformer_input_drive", f32v(0.6)),
                ("transformer_input_saturation", f32v(0.7)),
                ("transformer_hysteresis_bypass", boolv(false)),
                ("sheen_warmth", f32v(0.6)),
                ("sheen_warmth_tape_mode", boolv(true)),
                ("sheen_air_db", f32v(0.0)),
            ],
        ),
        // ── Instruments ──────────────────────────────────────────────────
        build_preset(
            "Acoustic Guitar Bus",
            "Instruments",
            &[
                ("eq_bypass", boolv(false)),
                ("lmf_gain", f32v(-2.0)),
                ("hf_gain", f32v(1.5)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-16.0)),
                ("comp_opt_speed", f32v(0.3)),
                ("comp_opt_char", f32v(0.5)),
                ("pultec_bypass", boolv(false)),
                ("pultec_hf_boost_gain", f32v(2.0)),
            ],
        ),
        build_preset(
            "Electric Guitar Bus",
            "Instruments",
            &[
                ("eq_bypass", boolv(false)),
                ("lmf_gain", f32v(2.0)),
                ("hf_gain", f32v(-1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("1176 FET")),
                ("comp_fet_ratio", strv("12:1")),
                ("comp_fet_atk", f32v(0.3)),
                ("comp_fet_rel", f32v(200.0)),
                ("transformer_bypass", boolv(false)),
                ("transformer_model", strv("British")),
                ("transformer_input_drive", f32v(0.4)),
                ("transformer_input_saturation", f32v(0.5)),
            ],
        ),
        build_preset(
            "Bass Guitar Bus",
            "Instruments",
            &[
                ("eq_bypass", boolv(false)),
                ("lf_gain", f32v(3.0)),
                ("lf_freq", f32v(80.0)),
                ("lmf_gain", f32v(-1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("VCA")),
                ("comp_vca_thresh", f32v(-16.0)),
                ("comp_vca_ratio", f32v(5.0)),
                ("comp_vca_atk", f32v(10.0)),
                ("comp_vca_rel", f32v(120.0)),
                ("dyneq_bypass", boolv(false)),
                ("dyneq_band1_threshold", f32v(-20.0)),
                ("dyneq_band1_ratio", f32v(3.0)),
            ],
        ),
        build_preset(
            "Piano Bus",
            "Instruments",
            &[
                ("eq_bypass", boolv(false)),
                ("lmf_gain", f32v(-1.0)),
                ("hf_gain", f32v(1.0)),
                ("comp_bypass", boolv(false)),
                ("comp_model", strv("Optical")),
                ("comp_opt_thresh", f32v(-18.0)),
                ("comp_opt_speed", f32v(0.25)),
                ("comp_opt_char", f32v(0.4)),
                ("pultec_bypass", boolv(false)),
                ("pultec_lf_boost_gain", f32v(1.0)),
                ("pultec_hf_boost_gain", f32v(1.5)),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ships_at_least_twenty_presets() {
        assert!(factory_presets().len() >= 20);
    }

    #[test]
    fn every_preset_specifies_every_parameter() {
        let full_count = default_param_values().len();
        for preset in factory_presets() {
            assert_eq!(
                preset.params.len(),
                full_count,
                "preset {:?} does not specify every parameter",
                preset.name
            );
        }
    }

    #[test]
    fn preset_names_are_unique() {
        let presets = factory_presets();
        let mut names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), presets.len(), "duplicate preset name found");
    }

    #[test]
    fn every_preset_has_a_recognized_category() {
        const CATEGORIES: &[&str] = &["Drums", "Vocals", "Voice", "Mastering", "Instruments"];
        for preset in factory_presets() {
            assert!(
                CATEGORIES.contains(&preset.category.as_str()),
                "preset {:?} has unrecognized category {:?}",
                preset.name,
                preset.category
            );
        }
    }

    #[test]
    fn every_override_key_is_a_real_parameter_id() {
        let known_ids = default_param_values();
        for preset in factory_presets() {
            for id in preset.params.keys() {
                assert!(
                    known_ids.contains_key(id),
                    "preset {:?} references unknown parameter id {:?}",
                    preset.name,
                    id
                );
            }
        }
    }
}
