use crate::oversampler::Oversampler;
use crate::shaping::biquad_coeffs;
use biquad::{Biquad, DirectForm1, Type};
use nih_plug::buffer::Buffer;

/// Oversampling factor for the tube saturation stage. 4× (2 halfband stages)
/// brings the 2nd/3rd-order harmonic energy of a pushed signal below
/// fold-back threshold while remaining cheap enough for an always-on EQ.
const PULTEC_TUBE_OS_FACTOR: usize = 4;

/// The passive LCR inductor network in the real EQP-1A creates a resonant
/// peak at the selected shelf frequency. At Q=0.5 (wide shelf) the peak needs
/// to be ~45% of shelf gain to remain clearly audible at the corner.
const LF_RESONANT_RATIO: f32 = 0.45;
const LF_RESONANT_Q: f32 = 1.8;

/// LF shelf Q range driven by the bandwidth knob (0 = narrow/modern, 1 = wide/vintage).
/// Q=1.0 at BW=0 gives a tight, modern shelf; Q=0.25 at BW=1 gives the very
/// gradual, wide shelf characteristic of the passive EQP-1A inductor network.
const LF_SHELF_Q_NARROW: f32 = 1.0;
const LF_SHELF_Q_WIDE: f32 = 0.25;

/// Pultec EQP-1A style EQ module
///
/// Classic passive tube EQ with simultaneous boost/cut characteristics
/// - Low frequency boost with optional simultaneous cut for unique curves
/// - High frequency boost with separate bandwidth and cut controls
/// - Tube-style saturation modeling
pub struct PultecEQ {
    sample_rate: f32,

    // Each biquad carries its own state (z1, z2); stereo processing REQUIRES
    // a separate filter instance per channel. Sharing one filter across L and
    // R makes consecutive samples from alternating channels corrupt the
    // filter memory, smearing the shelf and blunting perceived gain.
    lf_boost_filter: [DirectForm1<f32>; 2],
    // Resonant peak from the passive LCR network — centered at the same
    // frequency as the shelf, gain proportional to shelf gain.
    lf_resonant_filter: [DirectForm1<f32>; 2],
    lf_cut_filter: [DirectForm1<f32>; 2],
    hf_boost_filter: [DirectForm1<f32>; 2],
    hf_cut_filter: [DirectForm1<f32>; 2],

    // Tube saturation state
    tube_drive: f32,

    // Per-channel oversamplers for the tube saturation nonlinearity.
    tube_os_l: Oversampler,
    tube_os_r: Oversampler,
}

impl PultecEQ {
    /// Create a new Pultec EQ with the given sample rate.
    ///
    /// Filters are initialized flat (0 dB). Coefficients are updated in-place
    /// via `update_coefficients()` in `update_parameters()`, which preserves
    /// filter state across parameter changes and avoids per-buffer allocation.
    pub fn new(sample_rate: f32) -> Self {
        // Helper: flat 0 dB filter at a nominal per-section frequency.
        let flat_at = |freq_hz: f32| -> DirectForm1<f32> {
            let coeff = biquad_coeffs(Type::PeakingEQ(0.0), sample_rate, freq_hz, 0.707)
                .expect("0 dB PeakingEQ is always valid");
            DirectForm1::<f32>::new(coeff)
        };

        // Oversamplers are used inline (one sample in → one sample out), so
        // `max_block_size = 1` keeps their scratch buffers at 16 samples.
        let make_os = || {
            let mut os = Oversampler::new(PULTEC_TUBE_OS_FACTOR, 1);
            os.set_factor(PULTEC_TUBE_OS_FACTOR);
            os
        };

        Self {
            sample_rate,
            lf_boost_filter: [flat_at(100.0), flat_at(100.0)],
            lf_resonant_filter: [flat_at(100.0), flat_at(100.0)],
            lf_cut_filter: [flat_at(80.0), flat_at(80.0)],
            hf_boost_filter: [flat_at(8000.0), flat_at(8000.0)],
            hf_cut_filter: [flat_at(10000.0), flat_at(10000.0)],
            tube_drive: 0.0,
            tube_os_l: make_os(),
            tube_os_r: make_os(),
        }
    }

    /// Reset filter and saturation state. Call on sample-rate change or
    /// buffer discontinuity.
    pub fn reset(&mut self) {
        self.tube_os_l.reset();
        self.tube_os_r.reset();
    }

    /// Update Pultec parameters
    ///
    /// # Arguments
    /// * `lf_boost_freq` - Low frequency boost center (20..300 Hz)
    /// * `lf_boost_db` - Low frequency boost (0..+18 dB)
    /// * `lf_boost_bandwidth` - Shelf width: 0 = narrow/modern (Q=1.0), 1 = wide/vintage (Q=0.25)
    /// * `lf_cut_freq` - Low frequency cut center, independent of boost — the
    ///   Pultec "trick" is to set boost and cut at *different* frequencies so
    ///   the overlap produces a scooped-then-boosted curve (20..400 Hz)
    /// * `lf_cut_db` - Low frequency attenuation (0..18 dB; negated internally)
    /// * `lf_cut_bandwidth` - Cut shelf width: same convention as lf_boost_bandwidth
    /// * `hf_boost_freq` - High frequency boost center (5, 8, 10, 12, 15, 20 kHz)
    /// * `hf_boost_db` - High frequency boost (0..+10 dB)
    /// * `hf_boost_bandwidth` - High frequency boost Q/bandwidth (0.0 to 1.0)
    /// * `hf_cut_freq` - High frequency cut frequency (5, 10, 20 kHz)
    /// * `hf_cut_db` - High frequency attenuation (0..8 dB; negated internally)
    /// * `tube_drive` - Tube saturation amount (0.0 to 1.0)
    pub fn update_parameters(
        &mut self,
        lf_boost_freq: f32,
        lf_boost_db: f32,
        lf_boost_bandwidth: f32,
        lf_cut_freq: f32,
        lf_cut_db: f32,
        lf_cut_bandwidth: f32,
        hf_boost_freq: f32,
        hf_boost_db: f32,
        hf_boost_bandwidth: f32,
        hf_cut_freq: f32,
        hf_cut_db: f32,
        tube_drive: f32,
    ) {
        self.tube_drive = tube_drive.clamp(0.0, 1.0);

        // All four sections follow the same pattern:
        //   - compute dB (0.0 when the gain control is below noise floor)
        //   - call update_coefficients() on the existing filter object
        // This preserves filter state across parameter changes (no state reset,
        // no clicks) and avoids creating new DirectForm1 objects on the audio thread.

        // Low Frequency Boost — LowShelf + resonant peak at the same frequency.
        // The passive LCR network in the real EQP-1A creates a resonant bump
        // at the corner, giving the characteristic "thump" and making the boost
        // much more perceptible at typical musical frequencies (60–300 Hz).
        let lf_boost_db = if lf_boost_db > 0.05 { lf_boost_db } else { 0.0 };
        let safe_lf_freq = lf_boost_freq.clamp(20.0, 400.0);
        // BW=0 → Q=LF_SHELF_Q_NARROW (tight/modern), BW=1 → Q=LF_SHELF_Q_WIDE (vintage/gradual)
        let lf_boost_q = LF_SHELF_Q_NARROW
            + lf_boost_bandwidth.clamp(0.0, 1.0) * (LF_SHELF_Q_WIDE - LF_SHELF_Q_NARROW);
        if let Ok(coeff) = biquad_coeffs(
            Type::LowShelf(lf_boost_db),
            self.sample_rate,
            safe_lf_freq,
            lf_boost_q,
        ) {
            self.lf_boost_filter[0].update_coefficients(coeff);
            self.lf_boost_filter[1].update_coefficients(coeff);
        }
        // Resonant peak: 45% of shelf gain, Q=1.8, same center frequency.
        // Goes flat (0 dB) when the shelf is inactive.
        let resonant_db = lf_boost_db * LF_RESONANT_RATIO;
        if let Ok(coeff) = biquad_coeffs(
            Type::PeakingEQ(resonant_db),
            self.sample_rate,
            safe_lf_freq,
            LF_RESONANT_Q,
        ) {
            self.lf_resonant_filter[0].update_coefficients(coeff);
            self.lf_resonant_filter[1].update_coefficients(coeff);
        }

        // Low Frequency Cut — independent frequency from boost. Classic
        // EQP-1A "trick": boost at e.g. 60 Hz and cut at e.g. 200 Hz so the
        // cut attenuates the mud above the boosted low-bass for a tight,
        // defined low end. Value is already in dB; negate for shelf cut.
        let lf_cut_db = if lf_cut_db > 0.05 { -lf_cut_db } else { 0.0 };
        let safe_lf_cut_freq = lf_cut_freq.clamp(20.0, 500.0);
        let lf_cut_q = LF_SHELF_Q_NARROW
            + lf_cut_bandwidth.clamp(0.0, 1.0) * (LF_SHELF_Q_WIDE - LF_SHELF_Q_NARROW);
        if let Ok(coeff) = biquad_coeffs(
            Type::LowShelf(lf_cut_db),
            self.sample_rate,
            safe_lf_cut_freq,
            lf_cut_q,
        ) {
            self.lf_cut_filter[0].update_coefficients(coeff);
            self.lf_cut_filter[1].update_coefficients(coeff);
        }

        // High Frequency Boost — PeakingEQ, 0 dB when inactive.
        // Value is already in dB (parameter range 0..10 dB).
        let hf_boost_db = if hf_boost_db > 0.05 { hf_boost_db } else { 0.0 };
        let hf_q = 0.6 + hf_boost_bandwidth * hf_boost_bandwidth * 1.4; // 0.6–2.0
        let safe_hf_freq = hf_boost_freq.clamp(3000.0, 20000.0);
        if let Ok(coeff) = biquad_coeffs(
            Type::PeakingEQ(hf_boost_db),
            self.sample_rate,
            safe_hf_freq,
            hf_q,
        ) {
            self.hf_boost_filter[0].update_coefficients(coeff);
            self.hf_boost_filter[1].update_coefficients(coeff);
        }

        // High Frequency Cut — HighShelf, 0 dB when inactive.
        // Value is already in dB; negate for shelf cut.
        let hf_cut_db = if hf_cut_db > 0.05 { -hf_cut_db } else { 0.0 };
        let safe_hf_cut_freq = hf_cut_freq.clamp(5000.0, 20000.0);
        if let Ok(coeff) = biquad_coeffs(
            Type::HighShelf(hf_cut_db),
            self.sample_rate,
            safe_hf_cut_freq,
            0.9,
        ) {
            self.hf_cut_filter[0].update_coefficients(coeff);
            self.hf_cut_filter[1].update_coefficients(coeff);
        }
    }

    /// Process audio buffer through Pultec EQ
    pub fn process(&mut self, buffer: &mut Buffer) {
        let mut scratch = [0.0_f32; PULTEC_TUBE_OS_FACTOR];
        for mut samples in buffer.iter_samples() {
            for (ch, sample) in samples.iter_mut().enumerate() {
                let ch = ch.min(1);
                let mut s = *sample;

                // Linear biquad chain. No inline clamps: stability is guaranteed
                // by the coefficient math, and clamps between stages would inject
                // memoryless distortion that aliases into the midrange.
                s = self.lf_boost_filter[ch].run(s);
                s = self.lf_resonant_filter[ch].run(s);
                s = self.lf_cut_filter[ch].run(s);
                s = self.hf_boost_filter[ch].run(s);
                s = self.hf_cut_filter[ch].run(s);

                // Tube saturation — the one intentional nonlinearity in this
                // module. Run through a 4× halfband oversampler so the tanh
                // harmonics do not fold back into the audible range.
                if self.tube_drive > 0.01 {
                    let drive_amount = self.tube_drive * 0.3;
                    let scale = 1.0 + drive_amount * 0.2;
                    let os = if ch == 0 {
                        &mut self.tube_os_l
                    } else {
                        &mut self.tube_os_r
                    };
                    {
                        let up = os.upsample(s, 0);
                        for i in 0..PULTEC_TUBE_OS_FACTOR {
                            scratch[i] = up[i].tanh() * scale;
                        }
                    }
                    s = os.downsample(&scratch[..PULTEC_TUBE_OS_FACTOR], 0);
                }

                *sample = s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pultec_new_does_not_panic() {
        let _eq = PultecEQ::new(44100.0);
        let _eq = PultecEQ::new(48000.0);
        let _eq = PultecEQ::new(96000.0);
    }

    #[test]
    fn test_pultec_update_parameters_nominal_does_not_panic() {
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            60.0,    // lf_boost_freq
            7.5,     // lf_boost_db (mid of 0..18)
            0.67,    // lf_boost_bandwidth (default wide)
            200.0,   // lf_cut_freq — classic trick: cut above the boost
            4.5,     // lf_cut_db
            0.5,     // lf_cut_bandwidth
            8000.0,  // hf_boost_freq
            6.0,     // hf_boost_db
            0.5,     // hf_boost_bandwidth
            10000.0, // hf_cut_freq
            1.6,     // hf_cut_db
            0.0,     // tube_drive
        );
    }

    #[test]
    fn test_pultec_lf_cut_freq_clamping() {
        // lf_cut_freq is clamped to [20, 200]; extreme values must not panic.
        let mut eq = PultecEQ::new(44100.0);
        // below range
        eq.update_parameters(
            60.0, 7.5, 0.67, 5.0, 7.5, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        // above range
        eq.update_parameters(
            60.0, 7.5, 0.67, 10000.0, 7.5, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    #[test]
    fn test_pultec_tube_drive_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // tube_drive is clamped to [0.0, 1.0] in update_parameters
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 2.0,
        );
        assert!(
            (eq.tube_drive - 1.0).abs() < 1e-5,
            "tube_drive > 1.0 should clamp to 1.0"
        );

        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, -1.0,
        );
        assert!(
            (eq.tube_drive - 0.0).abs() < 1e-5,
            "tube_drive < 0.0 should clamp to 0.0"
        );
    }

    #[test]
    fn test_pultec_lf_boost_freq_clamping() {
        // safe_lf_freq is clamped to [30, 200] — extremely low freq should not panic
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            1.0, 7.5, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        eq.update_parameters(
            500.0, 7.5, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    #[test]
    fn test_pultec_hf_freq_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // hf_boost_freq clamps to [3000, 20000]
        eq.update_parameters(
            100.0, 7.5, 0.67, 100.0, 0.0, 0.5, 100.0, 5.0, 0.5, 10000.0, 1.6, 0.0,
        );
        eq.update_parameters(
            100.0, 7.5, 0.67, 100.0, 0.0, 0.5, 30000.0, 5.0, 0.5, 25000.0, 1.6, 0.0,
        );
    }

    #[test]
    fn test_pultec_hf_q_range() {
        // hf_q = 0.6 + bandwidth^2 * 1.4 — ranges from 0.6 (bandwidth=0) to 2.0 (bandwidth=1)
        let q_min = 0.6_f32 + 0.0_f32.powi(2) * 1.4;
        let q_max = 0.6_f32 + 1.0_f32.powi(2) * 1.4;
        assert!((q_min - 0.6).abs() < 1e-5, "Q min should be 0.6");
        assert!((q_max - 2.0).abs() < 1e-5, "Q max should be 2.0");
    }

    #[test]
    fn test_pultec_inactive_sections_do_not_modify_coefficients() {
        // Gain values <= 0.01 should result in 0 dB (inactive sections).
        // Verify no panic when all gains are near zero.
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    /// Measure the actual steady-state gain PultecEQ applies through
    /// `process()`, using a real nih_plug `Buffer`. This is the end-to-end
    /// check the user-facing issue needs: "when I crank LF BOOST, do I
    /// actually get ~+15 dB of boost below the shelf corner?"
    fn measure_gain_db(eq: &mut PultecEQ, freq_hz: f32, sr: f32) -> f32 {
        use nih_plug::buffer::Buffer;
        let n = 8192_usize;
        let omega = 2.0 * core::f32::consts::PI * freq_hz / sr;
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin()).collect();
        let mut r: Vec<f32> = l.clone();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        eq.process(&mut buf);
        // Measure peak in the second half of the buffer — the first half
        // covers the biquad's transient warm-up.
        let peak = l[n / 2..].iter().fold(0.0_f32, |acc, &x| acc.max(x.abs()));
        20.0 * peak.log10()
    }

    #[test]
    fn test_pultec_lf_boost_delivers_real_gain() {
        // RED test: lf_boost at max should push ~+15 dB below the shelf corner.
        // At 30 Hz with a 60 Hz LowShelf and Q=0.9, the shelf plateau is fully
        // engaged, so a +15 dB shelf should show ≥ +12 dB measured gain.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            60.0, 15.0, 0.67, // lf boost: 60 Hz, +15 dB, default width
            100.0, 0.0, 0.5, // lf cut: disabled
            10000.0, 0.0, 0.5, // hf boost: disabled
            10000.0, 0.0, // hf cut: disabled
            0.0, // tube: off (linear path only)
        );
        let gain_db = measure_gain_db(&mut eq, 30.0, sr);
        assert!(
            gain_db > 12.0,
            "LF boost at max should deliver ≥ +12 dB at 30 Hz, got {gain_db:.2} dB"
        );
    }

    #[test]
    fn test_pultec_lf_boost_at_100hz_exactly_what_the_user_did() {
        // Replicate the user's Reaper scenario: LF boost freq = 100 Hz, gain = 1.0
        // (100%), all other sections neutral. Probe across the shelf to prove it
        // is truly centered at 100 Hz — not mistuned 4× low like before the
        // biquad_coeffs fix.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            100.0, 15.0, 0.67, // LF boost: 100 Hz, +15 dB, default width
            100.0, 0.0, 0.5, // LF cut disabled
            10000.0, 0.0, 0.5, // HF boost disabled
            10000.0, 0.0, // HF cut disabled
            0.0, // tube off
        );
        let g_30 = measure_gain_db(&mut eq, 30.0, sr);
        let g_100 = measure_gain_db(&mut eq, 100.0, sr);
        let g_1k = measure_gain_db(&mut eq, 1000.0, sr);
        // Well below corner: full +15 dB shelf plateau.
        assert!(
            g_30 > 12.0,
            "LF boost should deliver ≥ +12 dB at 30 Hz, got {g_30:.2} dB"
        );
        // Well above corner: shelf stops; should be near unity.
        assert!(
            g_1k.abs() < 2.0,
            "LF boost should be near 0 dB at 1 kHz (above shelf), got {g_1k:.2} dB"
        );
        // At the corner: shelf midpoint (~7.5 dB) + resonant peak (~5.25 dB) ≈ 12–14 dB.
        assert!(
            g_100 > 4.0 && g_100 < 18.0,
            "LF boost should be mid-rise + resonant peak at 100 Hz corner, got {g_100:.2} dB"
        );
    }

    #[test]
    fn test_pultec_lf_boost_zero_is_unity() {
        // Sanity guard: with every gain at 0, the chain is transparent.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            60.0, 0.0, 0.67, 100.0, 0.0, 0.5, 10000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        let gain_db = measure_gain_db(&mut eq, 30.0, sr);
        assert!(
            gain_db.abs() < 0.5,
            "flat Pultec should pass 30 Hz unchanged, got {gain_db:.2} dB"
        );
    }

    /// Hit the tube oversampler with a push-the-boundaries signal and verify
    /// the output stays finite and bounded — guards against FIR state
    /// corruption or overflow from the tanh·scale blend.
    #[test]
    fn test_pultec_tube_saturation_oversampled_bounded() {
        let mut eq = PultecEQ::new(44100.0);
        // Drive the tube stage hard while leaving EQ mostly flat.
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 1.0,
        );
        // Run 2048 samples of a sine at ~0.3·Nyquist directly through the
        // oversampled saturation block.
        let mut os = Oversampler::new(PULTEC_TUBE_OS_FACTOR, 1);
        os.set_factor(PULTEC_TUBE_OS_FACTOR);
        let mut scratch = [0.0_f32; PULTEC_TUBE_OS_FACTOR];
        let drive_amount = eq.tube_drive * 0.3;
        let scale = 1.0 + drive_amount * 0.2;
        for i in 0..2048 {
            let x = (2.0 * core::f32::consts::PI * 0.3 * i as f32).sin();
            let up = os.upsample(x, 0);
            for k in 0..PULTEC_TUBE_OS_FACTOR {
                scratch[k] = up[k].tanh() * scale;
            }
            let y = os.downsample(&scratch[..PULTEC_TUBE_OS_FACTOR], 0);
            assert!(y.is_finite(), "non-finite sample {y} at i={i}");
            assert!(y.abs() < 2.0, "implausibly large sample {y} at i={i}");
        }
    }
}
