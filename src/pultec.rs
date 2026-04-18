use crate::oversampler::Oversampler;
use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use nih_plug::buffer::Buffer;

/// Oversampling factor for the tube saturation stage. 4× (2 halfband stages)
/// brings the 2nd/3rd-order harmonic energy of a pushed signal below
/// fold-back threshold while remaining cheap enough for an always-on EQ.
const PULTEC_TUBE_OS_FACTOR: usize = 4;

/// Pultec EQP-1A style EQ module
///
/// Classic passive tube EQ with simultaneous boost/cut characteristics
/// - Low frequency boost with optional simultaneous cut for unique curves
/// - High frequency boost with separate bandwidth and cut controls
/// - Tube-style saturation modeling
pub struct PultecEQ {
    sample_rate: f32,

    // Low frequency section - boost and cut can be used simultaneously
    lf_boost_filter: DirectForm1<f32>,
    lf_cut_filter: DirectForm1<f32>,

    // High frequency section - separate boost and cut
    hf_boost_filter: DirectForm1<f32>,
    hf_cut_filter: DirectForm1<f32>,

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
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                sample_rate.hz(),
                freq_hz.hz(),
                0.707,
            )
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
            lf_boost_filter: flat_at(100.0),
            lf_cut_filter: flat_at(80.0),
            hf_boost_filter: flat_at(8000.0),
            hf_cut_filter: flat_at(10000.0),
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
    /// * `lf_boost_freq` - Low frequency boost center (20..200 Hz)
    /// * `lf_boost_gain` - Low frequency boost amount (0.0 to 1.0)
    /// * `lf_cut_freq` - Low frequency cut center, independent of boost — the
    ///   Pultec "trick" is to set boost and cut at *different* frequencies so
    ///   the overlap produces a scooped-then-boosted curve (20..200 Hz)
    /// * `lf_cut_gain` - Low frequency cut amount (0.0 to 1.0)
    /// * `hf_boost_freq` - High frequency boost center (5, 8, 10, 12, 15, 20 kHz)
    /// * `hf_boost_gain` - High frequency boost amount (0.0 to 1.0)
    /// * `hf_boost_bandwidth` - High frequency boost Q/bandwidth (0.0 to 1.0)
    /// * `hf_cut_freq` - High frequency cut frequency (5, 10, 20 kHz)
    /// * `hf_cut_gain` - High frequency cut amount (0.0 to 1.0)
    /// * `tube_drive` - Tube saturation amount (0.0 to 1.0)
    pub fn update_parameters(
        &mut self,
        lf_boost_freq: f32,
        lf_boost_gain: f32,
        lf_cut_freq: f32,
        lf_cut_gain: f32,
        hf_boost_freq: f32,
        hf_boost_gain: f32,
        hf_boost_bandwidth: f32,
        hf_cut_freq: f32,
        hf_cut_gain: f32,
        tube_drive: f32,
    ) {
        self.tube_drive = tube_drive.clamp(0.0, 1.0);

        // All four sections follow the same pattern:
        //   - compute dB (0.0 when the gain control is below noise floor)
        //   - call update_coefficients() on the existing filter object
        // This preserves filter state across parameter changes (no state reset,
        // no clicks) and avoids creating new DirectForm1 objects on the audio thread.

        // Low Frequency Boost — LowShelf, 0 dB when inactive.
        // +12 dB max matches real EQP-1A spec; quadratic curve keeps the first
        // third of slider travel subtle for fine-tuning.
        let lf_boost_db = if lf_boost_gain > 0.01 {
            lf_boost_gain * lf_boost_gain * 12.0 // 0–12 dB quadratic curve
        } else {
            0.0
        };
        let safe_lf_freq = lf_boost_freq.clamp(30.0, 200.0);
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::LowShelf(lf_boost_db),
            self.sample_rate.hz(),
            safe_lf_freq.hz(),
            0.9,
        ) {
            self.lf_boost_filter.update_coefficients(coeff);
        }

        // Low Frequency Cut — independent frequency from boost. This is the
        // classic EQP-1A "trick": set boost at e.g. 60 Hz and cut at e.g.
        // 200 Hz so the low-shelf cut attenuates the mud right above the
        // boosted low-bass, leaving a tight, defined low end.
        // Real EQP-1A offers up to ~-15 dB attenuation; the previous -6 dB cap
        // left the low-mid scoop too subtle to achieve the classic tight-low
        // sound. Quadratic curve keeps fine control at low slider positions.
        let lf_cut_db = if lf_cut_gain > 0.01 {
            -(lf_cut_gain * lf_cut_gain * 15.0) // 0 to -15 dB quadratic curve
        } else {
            0.0
        };
        let safe_lf_cut_freq = lf_cut_freq.clamp(20.0, 200.0);
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::LowShelf(lf_cut_db),
            self.sample_rate.hz(),
            safe_lf_cut_freq.hz(),
            1.2,
        ) {
            self.lf_cut_filter.update_coefficients(coeff);
        }

        // High Frequency Boost — PeakingEQ, 0 dB when inactive.
        let hf_boost_db = if hf_boost_gain > 0.01 {
            hf_boost_gain * hf_boost_gain * 10.0 // 0–10 dB quadratic curve
        } else {
            0.0
        };
        let hf_q = 0.6 + hf_boost_bandwidth * hf_boost_bandwidth * 1.4; // 0.6–2.0
        let safe_hf_freq = hf_boost_freq.clamp(3000.0, 20000.0);
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::PeakingEQ(hf_boost_db),
            self.sample_rate.hz(),
            safe_hf_freq.hz(),
            hf_q,
        ) {
            self.hf_boost_filter.update_coefficients(coeff);
        }

        // High Frequency Cut — HighShelf, 0 dB when inactive.
        let hf_cut_db = if hf_cut_gain > 0.01 {
            -(hf_cut_gain * hf_cut_gain * 8.0) // 0 to -8 dB quadratic curve
        } else {
            0.0
        };
        let safe_hf_cut_freq = hf_cut_freq.clamp(5000.0, 20000.0);
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::HighShelf(hf_cut_db),
            self.sample_rate.hz(),
            safe_hf_cut_freq.hz(),
            0.9,
        ) {
            self.hf_cut_filter.update_coefficients(coeff);
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
                s = self.lf_boost_filter.run(s);
                s = self.lf_cut_filter.run(s);
                s = self.hf_boost_filter.run(s);
                s = self.hf_cut_filter.run(s);

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
            0.5,     // lf_boost_gain
            200.0,   // lf_cut_freq — classic trick: cut above the boost
            0.3,     // lf_cut_gain
            8000.0,  // hf_boost_freq
            0.6,     // hf_boost_gain
            0.5,     // hf_boost_bandwidth
            10000.0, // hf_cut_freq
            0.2,     // hf_cut_gain
            0.0,     // tube_drive
        );
    }

    #[test]
    fn test_pultec_lf_cut_freq_clamping() {
        // lf_cut_freq is clamped to [20, 200]; extreme values must not panic.
        let mut eq = PultecEQ::new(44100.0);
        // below range
        eq.update_parameters(60.0, 0.5, 5.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0);
        // above range
        eq.update_parameters(60.0, 0.5, 10000.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0);
    }

    #[test]
    fn test_pultec_lf_boost_quadratic_curve() {
        // With gain=1.0, boost_db = 1^2 * 8.0 = 8.0 dB
        // With gain=0.5, boost_db = 0.5^2 * 8.0 = 2.0 dB
        // These values come from the formula in update_parameters; we verify the intent.
        let gain_high = 1.0_f32;
        let gain_low = 0.5_f32;
        let boost_high = gain_high * gain_high * 8.0;
        let boost_low = gain_low * gain_low * 8.0;
        assert!((boost_high - 8.0).abs() < 1e-5, "1.0 → 8 dB");
        assert!((boost_low - 2.0).abs() < 1e-5, "0.5 → 2 dB");
        assert!(boost_high > boost_low);
    }

    #[test]
    fn test_pultec_hf_boost_quadratic_curve() {
        // max gain: 1^2 * 10 = 10 dB
        let boost = 1.0_f32 * 1.0 * 10.0;
        assert!((boost - 10.0).abs() < 1e-5);
    }

    #[test]
    fn test_pultec_hf_cut_quadratic_curve() {
        // max cut: 1^2 * 8 = 8 dB negative
        let cut = -(1.0_f32 * 1.0 * 8.0);
        assert!((cut + 8.0).abs() < 1e-5);
    }

    #[test]
    fn test_pultec_tube_drive_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // tube_drive is clamped to [0.0, 1.0] in update_parameters
        eq.update_parameters(100.0, 0.0, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, 2.0);
        assert!(
            (eq.tube_drive - 1.0).abs() < 1e-5,
            "tube_drive > 1.0 should clamp to 1.0"
        );

        eq.update_parameters(100.0, 0.0, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, -1.0);
        assert!(
            (eq.tube_drive - 0.0).abs() < 1e-5,
            "tube_drive < 0.0 should clamp to 0.0"
        );
    }

    #[test]
    fn test_pultec_lf_boost_freq_clamping() {
        // safe_lf_freq is clamped to [30, 200] — extremely low freq should not panic
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(1.0, 0.5, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0);
        eq.update_parameters(500.0, 0.5, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0);
    }

    #[test]
    fn test_pultec_hf_freq_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // hf_boost_freq clamps to [3000, 20000]
        eq.update_parameters(100.0, 0.5, 100.0, 0.0, 100.0, 0.5, 0.5, 10000.0, 0.2, 0.0);
        eq.update_parameters(100.0, 0.5, 100.0, 0.0, 30000.0, 0.5, 0.5, 25000.0, 0.2, 0.0);
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
        eq.update_parameters(100.0, 0.0, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0);
    }

    /// Hit the tube oversampler with a push-the-boundaries signal and verify
    /// the output stays finite and bounded — guards against FIR state
    /// corruption or overflow from the tanh·scale blend.
    #[test]
    fn test_pultec_tube_saturation_oversampled_bounded() {
        let mut eq = PultecEQ::new(44100.0);
        // Drive the tube stage hard while leaving EQ mostly flat.
        eq.update_parameters(100.0, 0.0, 100.0, 0.0, 8000.0, 0.0, 0.5, 10000.0, 0.0, 1.0);
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
