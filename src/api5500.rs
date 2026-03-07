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
            lf:  Filter::new(sample_rate, FilterType::LowShelf,  20000.0, Q_BUTTERWORTH_F32, 0.0),
            lmf: Filter::new(sample_rate, FilterType::Bell,       20000.0, Q_BUTTERWORTH_F32, 0.0),
            mf:  Filter::new(sample_rate, FilterType::Bell,       20000.0, Q_BUTTERWORTH_F32, 0.0),
            hmf: Filter::new(sample_rate, FilterType::Bell,       20000.0, Q_BUTTERWORTH_F32, 0.0),
            hf:  Filter::new(sample_rate, FilterType::HighShelf,  20000.0, Q_BUTTERWORTH_F32, 0.0),
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
        self.lf.update_parameters(self.sample_rate, FilterType::LowShelf, lf_freq, Q_BUTTERWORTH_F32, safe_lf_gain);
        self.lmf.update_parameters(self.sample_rate, FilterType::Bell, lmf_freq, lmf_q, safe_lmf_gain);
        self.mf.update_parameters(self.sample_rate, FilterType::Bell, mf_freq, mf_q, safe_mf_gain);
        self.hmf.update_parameters(self.sample_rate, FilterType::Bell, hmf_freq, hmf_q, safe_hmf_gain);
        self.hf.update_parameters(self.sample_rate, FilterType::HighShelf, hf_freq, Q_BUTTERWORTH_F32, safe_hf_gain);
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
