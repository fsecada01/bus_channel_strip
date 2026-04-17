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
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                s = self.lf.run(s);
                s = self.lmf.run(s);
                s = self.mf.run(s);
                s = self.hmf.run(s);
                s = self.hf.run(s);
                *sample = s;
            }
        }
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
