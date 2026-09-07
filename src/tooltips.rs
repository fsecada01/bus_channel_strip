//! Inline-help tooltip text for every automatable control in the 7 slot
//! modules (API5500, ButterComp2, Pultec, DynamicEQ, Transformer, Punch,
//! Haas) plus the pinned master-end Sheen stage (issue #25 / roadmap §4.6).
//!
//! Keyed by each parameter's stable `#[id = "..."]` string (see
//! `src/params/*.rs`), not its display label, so the mapping survives label
//! rewording. `global` and `routing` params (chassis bypass/gain and slot
//! ordering/hide-flags) are chassis-level, not "the 7-module + Sheen chain"
//! per the issue's own DoD wording, so they're intentionally not covered
//! here — GUI call sites for those pass [`NO_TOOLTIP`] instead.

/// Sentinel passed at call sites for controls outside this module's scope
/// (global/routing params). Never a real key in the table below, so
/// `get(NO_TOOLTIP)` always returns `None`.
pub const NO_TOOLTIP: &str = "";

/// Looks up the inline-help text for a control by its stable parameter ID.
/// Returns `None` for IDs outside the 7-module + Sheen chain (or unknown
/// IDs), in which case callers should render no tooltip at all.
pub fn get(param_id: &str) -> Option<&'static str> {
    match param_id {
        // ── API5500 (5-band semi-parametric EQ) ────────────────────────
        "eq_bypass" => Some("Bypasses the API5500 5-band EQ, passing audio through this slot unprocessed."),
        "lf_freq" => Some("Sets the corner frequency of the low-shelf band (20 Hz-400 Hz)."),
        "lf_gain" => Some("Boosts or cuts the low-shelf band, in dB."),
        "lmf_freq" => Some("Sets the center frequency of the low-mid parametric bell (50 Hz-2 kHz)."),
        "lmf_gain" => Some("Boosts or cuts the low-mid bell, in dB."),
        "lmf_q" => Some("Sets the bandwidth (Q) of the low-mid bell — higher values narrow the affected range."),
        "mf_freq" => Some("Sets the center frequency of the mid parametric bell (200 Hz-8 kHz)."),
        "mf_gain" => Some("Boosts or cuts the mid bell, in dB."),
        "mf_q" => Some("Sets the bandwidth (Q) of the mid bell — higher values narrow the affected range."),
        "hmf_freq" => Some("Sets the center frequency of the high-mid parametric bell (1 kHz-15 kHz)."),
        "hmf_gain" => Some("Boosts or cuts the high-mid bell, in dB."),
        "hmf_q" => Some("Sets the bandwidth (Q) of the high-mid bell — higher values narrow the affected range."),
        "hf_freq" => Some("Sets the corner frequency of the high-shelf band (3 kHz-20 kHz)."),
        "hf_gain" => Some("Boosts or cuts the high-shelf band, in dB."),

        // ── ButterComp2 (Airwindows-derived multi-model compressor) ────
        "comp_bypass" => Some("Bypasses the ButterComp2 compressor, passing audio through this slot unprocessed."),
        "comp_compress" => Some("Legacy Classic-model compression amount (0-1); higher values apply more gain reduction."),
        "comp_output" => Some("Legacy Classic-model makeup gain; 0.5 is unity, higher values add output level."),
        "comp_dry_wet" => Some("Blends compressed (wet) and unprocessed (dry) signal for parallel compression."),
        "comp_adaptive_env_bypass" => Some("When on, disables program-dependent release-time adaptation and restores the original fixed-shape Classic-model envelope follower."),
        "comp_model" => Some("Selects the active compressor topology: Classic, Optical, VCA, or 1176-style FET — switches which control surface below is live."),
        "comp_sc_hp" => Some("Sidechain high-pass corner shared by the VCA and FET detectors; removes low-frequency energy from the gain-reduction trigger so bass doesn't over-pump the compressor. 20 Hz is effectively off."),
        "comp_vca_thresh" => Some("VCA-model threshold, in dB — gain reduction begins once the input exceeds this level."),
        "comp_vca_ratio" => Some("VCA-model compression ratio — how strongly the signal is reduced above threshold."),
        "comp_vca_atk" => Some("VCA-model attack time, in ms — how quickly gain reduction engages once the threshold is crossed."),
        "comp_vca_rel" => Some("VCA-model release time, in ms — how quickly gain reduction recovers once the signal drops below threshold."),
        "comp_opt_thresh" => Some("Optical-model threshold, in dB — gain reduction begins once the input exceeds this level."),
        "comp_opt_speed" => Some("Optical-model attack/release speed — mimics the program-dependent time constants of an opto cell."),
        "comp_opt_char" => Some("Optical-model character — shapes the opto-style knee and harmonic coloration."),
        "comp_fet_input" => Some("1176-style FET input gain, in dB — drives the FET detector harder for more gain reduction and coloration."),
        "comp_fet_output" => Some("1176-style FET output (makeup) gain, in dB."),
        "comp_fet_atk" => Some("1176-style FET attack time, in ms."),
        "comp_fet_rel" => Some("1176-style FET release time, in ms."),
        "comp_fet_ratio" => Some("1176-style FET compression ratio: 4:1, 8:1, 12:1, 20:1, or ALL-buttons-in for the classic extreme-ratio FET character."),
        "comp_fet_auto" => Some("Engages 1176-style program-dependent auto-release in place of the fixed release time."),

        // ── Pultec (EQP-1A-style passive-style program EQ) ─────────────
        "pultec_bypass" => Some("Bypasses the Pultec EQ, passing audio through this slot unprocessed."),
        "pultec_lf_boost_freq" => Some("Sets the low-frequency boost center (20 Hz-300 Hz)."),
        "pultec_lf_boost_gain" => Some("Amount of low-frequency boost, in dB — up to +18 dB of professional-hardware headroom."),
        "pultec_lf_bw" => Some("Bandwidth of the low-frequency boost — low values are tight/modern, high values are wide/vintage."),
        "pultec_lf_cut_freq" => Some("Sets the low-frequency attenuator (cut) center, independent of the boost frequency — classic Pultec trick: boost low, cut higher for a tight low end."),
        "pultec_lf_cut_gain" => Some("Amount of low-frequency attenuation, in dB."),
        "pultec_lf_cut_bw" => Some("Bandwidth of the low-frequency attenuator."),
        "pultec_hf_boost_freq" => Some("Sets the high-frequency boost center (5 kHz-20 kHz)."),
        "pultec_hf_boost_gain" => Some("Amount of high-frequency boost, in dB."),
        "pultec_hf_boost_bandwidth" => Some("Bandwidth of the high-frequency boost."),
        "pultec_hf_cut_freq" => Some("Sets the high-frequency attenuator (cut) center (5 kHz-20 kHz)."),
        "pultec_hf_cut_gain" => Some("Amount of high-frequency attenuation, in dB."),
        "pultec_tube_drive" => Some("Amount of tube-style saturation applied across the Pultec stage."),
        "pultec_linear_phase" => Some("Switches the EQ core to a linear-phase (zero-phase-distortion) implementation; off reproduces the original zero-latency minimum-phase behavior."),

        // ── Dynamic EQ (4-band frequency-dependent compressor/expander) ─
        "dyneq_bypass" => Some("Bypasses the entire 4-band Dynamic EQ, passing audio through this slot unprocessed."),
        "dyneq_band1_freq" | "dyneq_band2_freq" | "dyneq_band3_freq" | "dyneq_band4_freq" =>
            Some("Center frequency this band's filter and level detector react around."),
        "dyneq_band1_threshold" | "dyneq_band2_threshold" | "dyneq_band3_threshold" | "dyneq_band4_threshold" =>
            Some("Level, in dB, at which this band's dynamic gain change begins."),
        "dyneq_band1_ratio" | "dyneq_band2_ratio" | "dyneq_band3_ratio" | "dyneq_band4_ratio" =>
            Some("How strongly this band's gain moves once past threshold — higher values react harder."),
        "dyneq_band1_attack" | "dyneq_band2_attack" | "dyneq_band3_attack" | "dyneq_band4_attack" =>
            Some("How quickly this band's gain change engages once past threshold, in ms."),
        "dyneq_band1_release" | "dyneq_band2_release" | "dyneq_band3_release" | "dyneq_band4_release" =>
            Some("How quickly this band's gain change recovers once back under threshold, in ms."),
        "dyneq_band1_gain" | "dyneq_band2_gain" | "dyneq_band3_gain" | "dyneq_band4_gain" =>
            Some("Static (non-dynamic) boost or cut applied at this band's frequency, in dB, on top of any dynamic movement."),
        "dyneq_band1_q" | "dyneq_band2_q" | "dyneq_band3_q" | "dyneq_band4_q" =>
            Some("Bandwidth (Q) of this band's filter — higher values narrow the affected range."),
        "dyneq_band1_enabled" | "dyneq_band2_enabled" | "dyneq_band3_enabled" | "dyneq_band4_enabled" =>
            Some("Enables or disables this band without resetting its settings."),
        "dyneq_band1_detector_freq" | "dyneq_band2_detector_freq" | "dyneq_band3_detector_freq" | "dyneq_band4_detector_freq" =>
            Some("Sidechain detector frequency for this band — lets the trigger listen at a different frequency than the filter itself acts on."),
        "dyneq_band1_mode" | "dyneq_band2_mode" | "dyneq_band3_mode" | "dyneq_band4_mode" =>
            Some("This band's dynamic behavior: Compress Down (reduce above threshold), Expand Up (boost above threshold), or Gate (cut below threshold)."),
        "dyneq_band1_solo" | "dyneq_band2_solo" | "dyneq_band3_solo" | "dyneq_band4_solo" =>
            Some("Solos this band's detector signal for audition — helps tune frequency and threshold by ear."),

        // ── Transformer (saturation coloration, 4 vintage models) ──────
        "transformer_bypass" => Some("Bypasses the Transformer coloration stage, passing audio through this slot unprocessed."),
        "transformer_model" => Some("Selects the transformer voicing: Vintage (Neve-style), Modern (API-style, cleaner), British (SSL-style console), or American."),
        "transformer_input_drive" => Some("Drives the input stage harder into saturation before the transformer's frequency shaping."),
        "transformer_input_saturation" => Some("Amount of input-stage harmonic saturation."),
        "transformer_output_drive" => Some("Drives the output stage harder into saturation after the transformer's frequency shaping."),
        "transformer_output_saturation" => Some("Amount of output-stage harmonic saturation."),
        "transformer_low_response" => Some("Shifts the transformer's low-frequency response; negative values roll off lows, positive values reinforce them."),
        "transformer_high_response" => Some("Shifts the transformer's high-frequency response; negative values roll off highs, positive values reinforce them."),
        "transformer_compression" => Some("Amount of transformer core loading/compression — models the gentle gain-squashing of a driven transformer."),
        "transformer_hysteresis_bypass" => Some("When on, disables the magnetic-hysteresis saturation model and restores the original v1.0 saturation curve exactly."),

        // ── Punch (clipper + transient shaper) ──────────────────────────
        "punch_bypass" => Some("Bypasses the Punch clipper/transient-shaper stage, passing audio through this slot unprocessed."),
        "punch_threshold" => Some("Ceiling, in dBTP, above which the clipper engages."),
        "punch_clip_mode" => Some("Clipping curve: Hard (cleanest, most transparent for small amounts), Soft (tanh, warmer character), or Cubic (polynomial, reduced high-frequency harmonics)."),
        "punch_softness" => Some("Knee softness of the clipper — higher values round the clip curve for a gentler onset."),
        "punch_oversampling" => Some("Internal oversampling factor used by the clipper to reduce aliasing; 8x is the recommended default, 16x is mastering quality."),
        "punch_attack" => Some("Boosts (positive) or softens (negative) the transient attack portion of detected hits."),
        "punch_sustain" => Some("Boosts (positive) or reduces (negative) the sustained portion of the signal after a transient."),
        "punch_attack_time" => Some("How long the transient shaper treats a hit as 'attack' before switching to sustain, in ms."),
        "punch_release_time" => Some("How long the transient detector takes to reset after a hit, in ms."),
        "punch_sensitivity" => Some("How easily the transient detector triggers on incoming peaks."),
        "punch_input_gain" => Some("Gain applied before the clipper/transient shaper, in dB."),
        "punch_output_gain" => Some("Gain applied after the clipper/transient shaper, in dB."),
        "punch_mix" => Some("Blends the processed (wet) Punch signal with the unprocessed (dry) signal."),
        "punch_wet_hpf" => Some("High-pass filter applied only to the wet (clipped/shaped) path, so parallel blends add attack without muddying the low end; 20 Hz is effectively off."),

        // ── Haas (M/S psychoacoustic stereo widener) ────────────────────
        "haas_bypass" => Some("Bypasses the Haas widener, passing audio through this slot unprocessed."),
        "haas_mid_gain" => Some("Gain applied to the mid (center) component after M/S encoding, in dB."),
        "haas_side_gain" => Some("Gain applied to the side (stereo difference) component after M/S encoding, in dB."),
        "haas_comb_depth" => Some("Depth of the comb-filter widening effect."),
        "haas_comb_time" => Some("Delay time used by the comb filter, in ms — sets the Haas-effect precedence delay."),
        "haas_comb_mode" => Some("Comb filter topology: Side Comb (polarity-flip, cancels on mono collapse) or Wide Comb (more diffuse, depth-limited to prevent excess peaking)."),
        "haas_mix" => Some("Blends the widened (wet) signal with the unprocessed (dry) signal."),

        // ── Sheen (pinned master-end polish coat) ────────────────────────
        "sheen_bypass" => Some("Bypasses the entire Sheen polish stage (BODY, PRESENCE, AIR, WARMTH, WIDTH), passing audio through unprocessed."),
        "sheen_body_db" => Some("Low-shelf boost/cut applied by the BODY stage, in dB."),
        "sheen_body_bypass" => Some("Bypasses just the BODY low-shelf stage."),
        "sheen_presence_db" => Some("Peak boost/cut applied by the PRESENCE stage, in dB."),
        "sheen_presence_bypass" => Some("Bypasses just the PRESENCE peak stage."),
        "sheen_air_db" => Some("High-shelf boost applied by the AIR stage, in dB."),
        "sheen_air_bypass" => Some("Bypasses just the AIR high-shelf stage."),
        "sheen_warmth" => Some("Amount of Sonnox-Inflator-style polynomial saturation applied by the WARMTH stage (processed at 2x oversampling)."),
        "sheen_warmth_bypass" => Some("Bypasses just the WARMTH saturation stage."),
        "sheen_warmth_tape_mode" => Some("Switches WARMTH to an opt-in magnetic-hysteresis (tape-style) saturation model instead of the default polynomial curve."),
        "sheen_width" => Some("Amount of mid/side stereo widening applied by the WIDTH stage (side-channel only)."),
        "sheen_width_bypass" => Some("Bypasses just the WIDTH stage."),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nice_plug::prelude::Params;

    fn assert_full_coverage<P: Params + Default>(module_name: &str) {
        let params = P::default();
        for (param_id, _ptr, _group) in params.param_map() {
            assert!(
                get(&param_id).is_some(),
                "missing tooltip entry for {module_name} param id \"{param_id}\""
            );
        }
    }

    #[test]
    fn every_api5500_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::Api5500Params>("api5500");
    }

    #[test]
    fn every_buttercomp2_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::ButterComp2Params>("buttercomp2");
    }

    #[test]
    fn every_pultec_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::PultecParams>("pultec");
    }

    #[test]
    fn every_dynamic_eq_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::DynamicEqParams>("dynamic_eq");
    }

    #[test]
    fn every_transformer_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::TransformerParams>("transformer");
    }

    #[test]
    fn every_punch_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::PunchParams>("punch");
    }

    #[test]
    fn every_haas_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::HaasParams>("haas");
    }

    #[test]
    fn every_sheen_param_has_a_tooltip() {
        assert_full_coverage::<crate::params::SheenParams>("sheen");
    }

    #[test]
    fn no_tooltip_sentinel_is_not_a_real_key() {
        assert!(get(NO_TOOLTIP).is_none());
    }
}
