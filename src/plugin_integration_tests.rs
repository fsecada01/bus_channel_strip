/// Integration tests — exercise the full plugin pipeline, not just individual DSP modules.
///
/// These tests create `BusChannelStrip::default()` and catch failure modes that
/// isolated module tests cannot find:
///
///   1. Wrong parameter range — `value()` returning 0–1 instead of dB
///   2. `update_parameters` argument order mismatch
///   3. Bypass default changed unexpectedly
///   4. Module not processing audio when bypass is explicitly off
///
/// All modules default to bypass=true (the GUI bypass button activates each one).
/// Tests that need bypass OFF call `plugin.pultec.update_parameters + plugin.pultec.process`
/// directly — this exercises the plugin's real module instance (same init, same sample rate)
/// without needing NIH-plug's crate-private param setters.
#[cfg(test)]
mod plugin_integration_tests {
    use crate::BusChannelStrip;
    use nice_plug::buffer::Buffer;
    use nice_plug::prelude::Params;

    fn make_sine_buffer(freq_hz: f32, sr: f32, n: usize) -> (Vec<f32>, Vec<f32>) {
        let omega = 2.0 * core::f32::consts::PI * freq_hz / sr;
        let l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin()).collect();
        let r = l.clone();
        (l, r)
    }

    fn peak_gain_db(signal: &[f32]) -> f32 {
        let half = signal.len() / 2;
        let peak = signal[half..].iter().fold(0.0_f32, |a, &x| a.max(x.abs()));
        20.0 * peak.log10()
    }

    fn run_pultec<F: FnMut(&mut Buffer)>(l: &mut Vec<f32>, r: &mut Vec<f32>, mut f: F) {
        let n = l.len();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(l);
                ss.push(r);
            });
        }
        f(&mut buf);
    }

    // ─── Param range sanity ────────────────────────────────────────────────────

    /// The LF boost param must use a dB range (0..15), not a 0–1 normalized range.
    /// If this fails the DSP receives 1.0 instead of 15.0 at max knob position.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_lf_boost_param_range_is_db() {
        let plugin = BusChannelStrip::default();
        let range = plugin.params.pultec.pultec_lf_boost_gain.range();
        let max_db = range.unnormalize(1.0);
        let default_db = plugin.params.pultec.pultec_lf_boost_gain.value();
        assert!(
            (max_db - 18.0).abs() < 0.01,
            "LF boost param max should be 18.0 dB, got {max_db:.4}"
        );
        assert!(
            default_db.abs() < 0.01,
            "LF boost param default should be 0.0 dB, got {default_db:.4}"
        );
    }

    /// Pultec bypass defaults to true (modules are inactive until the user enables them).
    /// If this changes, modules will always process audio even when the user expects silence.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_bypass_defaults_to_true() {
        let plugin = BusChannelStrip::default();
        assert!(
            plugin.params.pultec.pultec_bypass.value(),
            "pultec_bypass must default to true (module inactive until user enables it)"
        );
    }

    /// Punch's clipper ceiling defaults to -1.0 dBTP (issue #19), leaving
    /// headroom for DAC reconstruction/intersample overshoot. Was -0.1 dB
    /// prior to true-peak detection landing — if this regresses, sessions
    /// created after this change will silently lose that headroom.
    #[cfg(feature = "punch")]
    #[test]
    fn test_punch_threshold_defaults_to_minus_one_dbtp() {
        let plugin = BusChannelStrip::default();
        let default_db = plugin.params.punch.punch_threshold.value();
        assert!(
            (default_db - (-1.0)).abs() < 0.01,
            "punch_threshold default should be -1.0 dBTP, got {default_db:.4}"
        );
    }

    // ─── Gain delivery through the plugin's pultec instance ───────────────────

    /// Zero gains through the plugin's own PultecEQ instance must be transparent.
    /// Catches coefficient initialization bugs or accidental DC offsets.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_zero_gains_are_transparent() {
        let sr = 48_000.0_f32;
        let mut plugin = BusChannelStrip::default();
        plugin.pultec = crate::pultec::PultecEQ::new(sr);
        plugin.pultec.update_parameters(
            60.0, 0.0, 0.67, 100.0, 0.0, 0.5, 10000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );

        let (mut l, mut r) = make_sine_buffer(100.0, sr, 8192);
        run_pultec(&mut l, &mut r, |buf| {
            plugin.pultec.process(buf);
        });
        let gain_db = peak_gain_db(&l);
        assert!(
            gain_db.abs() < 0.5,
            "Zero-gain Pultec must be transparent at 100 Hz, got {gain_db:.2} dB"
        );
    }

    /// LF boost +15 dB / 60 Hz through the plugin's own PultecEQ instance.
    /// If this passes but Reaper shows no effect, the bug is in param wiring —
    /// check `test_pultec_lf_boost_param_range_is_db` and the bypass GUI state.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_lf_boost_plugin_instance_30hz() {
        let sr = 48_000.0_f32;
        let mut plugin = BusChannelStrip::default();
        plugin.pultec = crate::pultec::PultecEQ::new(sr);
        plugin.pultec.update_parameters(
            60.0, 15.0, 0.67, // LF boost: 60 Hz, +15 dB, default width
            100.0, 0.0, 0.5, // LF cut: off
            10000.0, 0.0, 0.5, // HF boost: off
            10000.0, 0.0, // HF cut: off
            0.0, // tube: off
        );

        let (mut l, mut r) = make_sine_buffer(30.0, sr, 8192);
        run_pultec(&mut l, &mut r, |buf| {
            plugin.pultec.process(buf);
        });
        let gain_db = peak_gain_db(&l);
        assert!(
            gain_db > 10.0,
            "Plugin instance: Pultec LF +15 dB / 60 Hz must deliver ≥ +10 dB at 30 Hz, got {gain_db:.2} dB"
        );
    }

    /// LF resonant peak: at the shelf corner frequency the LCR bump should push
    /// total gain well above the shelf midpoint alone (~7.5 dB) — target ≥ +8 dB.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_lf_resonant_bump_at_corner() {
        let sr = 48_000.0_f32;
        let mut plugin = BusChannelStrip::default();
        plugin.pultec = crate::pultec::PultecEQ::new(sr);
        plugin.pultec.update_parameters(
            100.0, 15.0, 0.67, 100.0, 0.0, 0.5, 10000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );

        let (mut l, mut r) = make_sine_buffer(100.0, sr, 8192);
        run_pultec(&mut l, &mut r, |buf| {
            plugin.pultec.process(buf);
        });
        let gain_db = peak_gain_db(&l);
        assert!(
            gain_db > 8.0,
            "Pultec LCR resonant bump must produce ≥ +8 dB at the 100 Hz corner, got {gain_db:.2} dB"
        );
    }

    // ─── #15: session compatibility ───────────────────────────────────────────

    /// The linear-phase toggle is a new, additive param. It must default OFF
    /// so a v1.0 session (which has no value for it) loads at zero latency
    /// with the minimum-phase response it was mixed with.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_linear_phase_param_defaults_off() {
        let plugin = BusChannelStrip::default();
        assert!(
            !plugin.params.pultec.pultec_linear_phase.value(),
            "pultec_linear_phase must default to false"
        );
        let ids: Vec<String> = plugin
            .params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        assert!(ids.iter().any(|id| id == "pultec_linear_phase"));
    }

    /// Every parameter ID that shipped in v1.0.0 must still exist, unchanged.
    /// Hosts restore sessions by ID: a missing or renamed ID silently resets
    /// that control to default when a v1.0 project is opened under v2.0 —
    /// the "load failure" the #15 checklist rules out. Adding IDs is fine;
    /// removing or renaming one is a breaking change and must fail here.
    #[cfg(all(
        feature = "api5500",
        feature = "buttercomp2",
        feature = "pultec",
        feature = "transformer",
        feature = "punch",
        feature = "haas",
        feature = "dynamic_eq",
        feature = "sheen"
    ))]
    #[test]
    fn test_v1_0_param_ids_are_all_still_present() {
        const V1_0_PARAM_IDS: &[&str] = &[
            "global_bypass",
            "global_auto_gain",
            "gain",
            "eq_bypass",
            "lf_freq",
            "lf_gain",
            "lmf_freq",
            "lmf_gain",
            "lmf_q",
            "mf_freq",
            "mf_gain",
            "mf_q",
            "hmf_freq",
            "hmf_gain",
            "hmf_q",
            "hf_freq",
            "hf_gain",
            "comp_bypass",
            "comp_compress",
            "comp_output",
            "comp_dry_wet",
            "comp_model",
            "comp_sc_hp",
            "comp_vca_thresh",
            "comp_vca_ratio",
            "comp_vca_atk",
            "comp_vca_rel",
            "comp_opt_thresh",
            "comp_opt_speed",
            "comp_opt_char",
            "comp_fet_input",
            "comp_fet_output",
            "comp_fet_atk",
            "comp_fet_rel",
            "comp_fet_ratio",
            "comp_fet_auto",
            "pultec_bypass",
            "pultec_lf_boost_freq",
            "pultec_lf_boost_gain",
            "pultec_lf_bw",
            "pultec_lf_cut_freq",
            "pultec_lf_cut_gain",
            "pultec_lf_cut_bw",
            "pultec_hf_boost_freq",
            "pultec_hf_boost_gain",
            "pultec_hf_boost_bandwidth",
            "pultec_hf_cut_freq",
            "pultec_hf_cut_gain",
            "pultec_tube_drive",
            "dyneq_bypass",
            "dyneq_band1_freq",
            "dyneq_band1_threshold",
            "dyneq_band1_ratio",
            "dyneq_band1_attack",
            "dyneq_band1_release",
            "dyneq_band1_gain",
            "dyneq_band1_q",
            "dyneq_band1_enabled",
            "dyneq_band1_detector_freq",
            "dyneq_band1_mode",
            "dyneq_band1_solo",
            "dyneq_band2_freq",
            "dyneq_band2_threshold",
            "dyneq_band2_ratio",
            "dyneq_band2_attack",
            "dyneq_band2_release",
            "dyneq_band2_gain",
            "dyneq_band2_q",
            "dyneq_band2_enabled",
            "dyneq_band2_detector_freq",
            "dyneq_band2_mode",
            "dyneq_band2_solo",
            "dyneq_band3_freq",
            "dyneq_band3_threshold",
            "dyneq_band3_ratio",
            "dyneq_band3_attack",
            "dyneq_band3_release",
            "dyneq_band3_gain",
            "dyneq_band3_q",
            "dyneq_band3_enabled",
            "dyneq_band3_detector_freq",
            "dyneq_band3_mode",
            "dyneq_band3_solo",
            "dyneq_band4_freq",
            "dyneq_band4_threshold",
            "dyneq_band4_ratio",
            "dyneq_band4_attack",
            "dyneq_band4_release",
            "dyneq_band4_gain",
            "dyneq_band4_q",
            "dyneq_band4_enabled",
            "dyneq_band4_detector_freq",
            "dyneq_band4_mode",
            "dyneq_band4_solo",
            "transformer_bypass",
            "transformer_model",
            "transformer_input_drive",
            "transformer_input_saturation",
            "transformer_output_drive",
            "transformer_output_saturation",
            "transformer_low_response",
            "transformer_high_response",
            "transformer_compression",
            "punch_bypass",
            "punch_threshold",
            "punch_clip_mode",
            "punch_softness",
            "punch_oversampling",
            "punch_attack",
            "punch_sustain",
            "punch_attack_time",
            "punch_release_time",
            "punch_sensitivity",
            "punch_input_gain",
            "punch_output_gain",
            "punch_mix",
            "punch_wet_hpf",
            "haas_bypass",
            "haas_mid_gain",
            "haas_side_gain",
            "haas_comb_depth",
            "haas_comb_time",
            "haas_comb_mode",
            "haas_mix",
            "sheen_bypass",
            "sheen_body_db",
            "sheen_body_bypass",
            "sheen_presence_db",
            "sheen_presence_bypass",
            "sheen_air_db",
            "sheen_air_bypass",
            "sheen_warmth",
            "sheen_warmth_bypass",
            "sheen_width",
            "sheen_width_bypass",
            "module_order_1",
            "module_order_2",
            "module_order_3",
            "module_order_4",
            "module_order_5",
            "module_order_6",
            "module_order_7",
            "hide_api5500",
            "hide_buttercomp2",
            "hide_pultec",
            "hide_dynamic_eq",
            "hide_transformer",
            "hide_punch",
            "hide_haas",
        ];

        let plugin = BusChannelStrip::default();
        let ids: Vec<String> = plugin
            .params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();

        let missing: Vec<&&str> = V1_0_PARAM_IDS
            .iter()
            .filter(|want| !ids.iter().any(|have| have == *want))
            .collect();
        assert!(
            missing.is_empty(),
            "v1.0 parameter IDs missing (breaks session load): {missing:?}"
        );

        // IDs must also be unique — a duplicate makes host restore ambiguous.
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate parameter IDs present");
    }

    /// HF boost +10 dB / 8 kHz through the plugin's own instance.
    #[cfg(feature = "pultec")]
    #[test]
    fn test_pultec_hf_boost_plugin_instance_8khz() {
        let sr = 48_000.0_f32;
        let mut plugin = BusChannelStrip::default();
        plugin.pultec = crate::pultec::PultecEQ::new(sr);
        plugin.pultec.update_parameters(
            60.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 10.0, 0.5, 10000.0, 0.0, 0.0,
        );

        let (mut l, mut r) = make_sine_buffer(8000.0, sr, 8192);
        run_pultec(&mut l, &mut r, |buf| {
            plugin.pultec.process(buf);
        });
        let gain_db = peak_gain_db(&l);
        assert!(
            gain_db > 6.0,
            "Plugin instance: Pultec HF +10 dB / 8 kHz must deliver ≥ +6 dB at 8 kHz, got {gain_db:.2} dB"
        );
    }
}
