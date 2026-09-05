use crate::shaping::{Filter, FilterType};
use biquad::Q_BUTTERWORTH_F32;
use nih_plug::buffer::Buffer;

pub struct Api5500 {
    sample_rate: f32,
    lf: Filter,
    lmf: Filter,
    mf: Filter,
    hmf: Filter,
    hf: Filter,
}

impl Api5500 {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            lf: Filter::new(
                sample_rate,
                FilterType::LowShelf,
                20000.0,
                Q_BUTTERWORTH_F32,
                0.0,
            ),
            lmf: Filter::new(
                sample_rate,
                FilterType::Bell,
                20000.0,
                Q_BUTTERWORTH_F32,
                0.0,
            ),
            mf: Filter::new(
                sample_rate,
                FilterType::Bell,
                20000.0,
                Q_BUTTERWORTH_F32,
                0.0,
            ),
            hmf: Filter::new(
                sample_rate,
                FilterType::Bell,
                20000.0,
                Q_BUTTERWORTH_F32,
                0.0,
            ),
            hf: Filter::new(
                sample_rate,
                FilterType::HighShelf,
                20000.0,
                Q_BUTTERWORTH_F32,
                0.0,
            ),
        }
    }

    pub fn update_parameters(
        &mut self,
        lf_freq: f32,
        lf_gain: f32,
        lmf_freq: f32,
        lmf_gain: f32,
        lmf_q: f32,
        mf_freq: f32,
        mf_gain: f32,
        mf_q: f32,
        hmf_freq: f32,
        hmf_gain: f32,
        hmf_q: f32,
        hf_freq: f32,
        hf_gain: f32,
    ) {
        // Limit gains to prevent instability and distortion
        let safe_lf_gain = lf_gain.clamp(-12.0, 12.0);
        let safe_lmf_gain = lmf_gain.clamp(-12.0, 12.0);
        let safe_mf_gain = mf_gain.clamp(-12.0, 12.0);
        let safe_hmf_gain = hmf_gain.clamp(-12.0, 12.0);
        let safe_hf_gain = hf_gain.clamp(-12.0, 12.0);

        // Update filters with safe gains
        self.lf.update_parameters(
            self.sample_rate,
            FilterType::LowShelf,
            lf_freq,
            Q_BUTTERWORTH_F32,
            safe_lf_gain,
        );
        self.lmf.update_parameters(
            self.sample_rate,
            FilterType::Bell,
            lmf_freq,
            lmf_q,
            safe_lmf_gain,
        );
        self.mf.update_parameters(
            self.sample_rate,
            FilterType::Bell,
            mf_freq,
            mf_q,
            safe_mf_gain,
        );
        self.hmf.update_parameters(
            self.sample_rate,
            FilterType::Bell,
            hmf_freq,
            hmf_q,
            safe_hmf_gain,
        );
        self.hf.update_parameters(
            self.sample_rate,
            FilterType::HighShelf,
            hf_freq,
            Q_BUTTERWORTH_F32,
            safe_hf_gain,
        );
    }

    pub fn process(&mut self, buffer: &mut Buffer) {
        for mut samples in buffer.iter_samples() {
            for (ch, sample) in samples.iter_mut().enumerate() {
                let ch = ch.min(1);
                let mut s = *sample;
                s = self.lf.run_ch(s, ch);
                s = self.lmf.run_ch(s, ch);
                s = self.mf.run_ch(s, ch);
                s = self.hmf.run_ch(s, ch);
                s = self.hf.run_ch(s, ch);
                *sample = s;
            }
        }
    }

    /// Zero every band's SVF integrator state. Call on transport reset — the
    /// other four EQ modules already clear their filter state there, and
    /// (unlike Sheen's deliberately-left-to-settle EQ, see its own `reset()`)
    /// API5500 had no reset path at all before #15 added `Filter::reset()`.
    pub fn reset(&mut self) {
        self.lf.reset();
        self.lmf.reset();
        self.mf.reset();
        self.hmf.reset();
        self.hf.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api5500_new_does_not_panic() {
        let _eq = Api5500::new(44100.0);
        let _eq = Api5500::new(48000.0);
        let _eq = Api5500::new(96000.0);
    }

    #[test]
    fn test_api5500_update_parameters_does_not_panic() {
        let mut eq = Api5500::new(44100.0);
        // Nominal in-range values
        eq.update_parameters(
            100.0,   // lf_freq
            3.0,     // lf_gain
            300.0,   // lmf_freq
            2.0,     // lmf_gain
            0.7,     // lmf_q
            1000.0,  // mf_freq
            -2.0,    // mf_gain
            1.0,     // mf_q
            5000.0,  // hmf_freq
            1.5,     // hmf_gain
            1.2,     // hmf_q
            12000.0, // hf_freq
            -1.0,    // hf_gain
        );
    }

    #[test]
    fn test_api5500_gain_clamping_positive() {
        // Passing gains > 12 dB should silently clamp — no panic, no NaN
        let mut eq = Api5500::new(44100.0);
        eq.update_parameters(
            100.0, 100.0, // lf +100 dB — must be clamped to +12
            300.0, 100.0, 0.7, 1000.0, 100.0, 1.0, 5000.0, 100.0, 1.2, 12000.0, 100.0,
        );
        // Processing a sample should not produce NaN or ±inf
        // We cannot call process() without a Buffer, so we verify the update didn't crash.
    }

    #[test]
    fn test_api5500_gain_clamping_negative() {
        let mut eq = Api5500::new(44100.0);
        eq.update_parameters(
            100.0, -100.0, 300.0, -100.0, 0.7, 1000.0, -100.0, 1.0, 5000.0, -100.0, 1.2, 12000.0,
            -100.0,
        );
    }

    /// Run a stereo sine through `Api5500::process` and return the left
    /// channel output.
    fn process_sine(eq: &mut Api5500, freq_hz: f32, sr: f32, n: usize) -> Vec<f32> {
        let omega = core::f32::consts::TAU * freq_hz / sr;
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin()).collect();
        let mut r = l.clone();
        let mut buf = Buffer::default();
        // SAFETY: `l` and `r` are each length `n` and live for the duration
        // of this function, so the slices `set_slices` hands to `ss` are
        // valid and correctly sized for the whole call.
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        eq.process(&mut buf);
        l
    }

    /// Checklist item 2 (#15): API5500 is the first module validated on the
    /// TPT core. With one band boosted and the other four flat, the module
    /// output must null against a single RBJ biquad of the same design —
    /// proving both that the TPT core matches the old topology and that the
    /// flat stages are transparent.
    #[test]
    fn test_api5500_tpt_core_nulls_against_biquad_reference() {
        use crate::shaping::biquad_coeffs;
        use biquad::{Biquad, DirectForm1, Type};

        let sr = 48_000.0;
        let n = 8192;
        let cases: [(&str, f32, f32, f32, Type<f32>); 3] = [
            ("mf bell", 1000.0, 6.0, 1.0, Type::PeakingEQ(6.0)),
            (
                "lf shelf",
                100.0,
                4.0,
                Q_BUTTERWORTH_F32,
                Type::LowShelf(4.0),
            ),
            (
                "hf shelf",
                10000.0,
                -5.0,
                Q_BUTTERWORTH_F32,
                Type::HighShelf(-5.0),
            ),
        ];
        for (name, freq, gain, q, bq_type) in cases {
            let mut eq = Api5500::new(sr);
            // Every band flat except the one under test.
            let (lf_g, mf_g, hf_g) = match name {
                "lf shelf" => (gain, 0.0, 0.0),
                "mf bell" => (0.0, gain, 0.0),
                _ => (0.0, 0.0, gain),
            };
            eq.update_parameters(
                if name == "lf shelf" { freq } else { 80.0 },
                lf_g,
                300.0,
                0.0,
                0.7,
                if name == "mf bell" { freq } else { 1000.0 },
                mf_g,
                q,
                5000.0,
                0.0,
                1.0,
                if name == "hf shelf" { freq } else { 12000.0 },
                hf_g,
            );
            let out = process_sine(&mut eq, freq, sr, n);

            let mut reference =
                DirectForm1::<f32>::new(biquad_coeffs(bq_type, sr, freq, q).unwrap());
            let omega = core::f32::consts::TAU * freq / sr;
            let expected: Vec<f32> = (0..n)
                .map(|i| reference.run((omega * i as f32).sin()))
                .collect();

            let diff = out
                .iter()
                .zip(&expected)
                .fold(0.0_f32, |a, (x, y)| a.max((x - y).abs()));
            assert!(
                diff < 1.0e-3,
                "{name}: API5500 output differs from biquad reference by {diff:e}"
            );
        }
    }

    #[test]
    fn test_api5500_multiple_sample_rates() {
        for &sr in &[22050.0, 44100.0, 48000.0, 88200.0, 96000.0_f32] {
            let mut eq = Api5500::new(sr);
            eq.update_parameters(
                200.0, 3.0, 500.0, 2.0, 0.7, 2000.0, -1.0, 1.0, 8000.0, 1.0, 1.0, 15000.0, -2.0,
            );
        }
    }
}
