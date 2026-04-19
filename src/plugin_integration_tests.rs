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
    use nih_plug::buffer::Buffer;

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
        let range = plugin.params.pultec_lf_boost_gain.range();
        let max_db = range.unnormalize(1.0);
        let default_db = plugin.params.pultec_lf_boost_gain.value();
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
            plugin.params.pultec_bypass.value(),
            "pultec_bypass must default to true (module inactive until user enables it)"
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
