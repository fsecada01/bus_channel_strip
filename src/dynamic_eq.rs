
use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// NOTE: This implementation recalculates filter coefficients for every sample,
// which is inefficient and can cause audio artifacts. For a production-ready
// plugin, the filter math should be implemented directly to allow for
// efficient gain modulation.

/// The mode of operation for a dynamic band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum DynamicMode {
    /// Reduces gain when the signal is above the threshold.
    #[name = "Compress Down"]
    CompressDownward,
    /// Increases gain when the signal is above the threshold.
    #[name = "Expand Up"]
    ExpandUpward,
    /// Reduces gain when the signal is below the threshold.
    #[name = "Gate"]
    Gate,
}

impl Default for DynamicMode {
    fn default() -> Self {
        DynamicMode::CompressDownward
    }
}

/// A single band of dynamic equalization.
struct DynamicBand {
    // Filters
    sidechain_filter: DirectForm1<f32>,
    eq_filter: DirectForm1<f32>,

    // State
    envelope: f32,
    pub gain_reduction_db: f32,

    // Parameters
    sample_rate: f32,
    mode: DynamicMode,
    detector_freq: f32,
    frequency: f32,
    q: f32,
    threshold: f32, // Linear threshold from dB
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    make_up_gain: f32, // Linear gain from dB
    enabled: bool,
}

impl DynamicBand {
    fn new(sample_rate: f32) -> Self {
        let flat_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ(0.0), // Start flat
            sample_rate.hz(),
            1000.0f32.hz(),
            0.707,
        )
        .unwrap();

        Self {
            sidechain_filter: DirectForm1::<f32>::new(flat_coeff),
            eq_filter: DirectForm1::<f32>::new(flat_coeff),
            envelope: 0.0,
            gain_reduction_db: 0.0,
            sample_rate,
            mode: DynamicMode::default(),
            detector_freq: 1000.0,
            frequency: 1000.0,
            q: 0.707,
            threshold: 1.0, // 0 dB
            ratio: 1.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            make_up_gain: 1.0,
            enabled: false,
        }
    }

    fn update_parameters(
        &mut self,
        mode: DynamicMode,
        detector_freq: f32,
        frequency: f32,
        q: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        make_up_gain_db: f32,
        enabled: bool,
    ) {
        self.mode = mode;
        self.detector_freq = detector_freq;
        self.frequency = frequency;
        self.q = q;
        self.threshold = 10.0f32.powf(threshold_db / 20.0);
        self.ratio = ratio;
        self.attack_coeff = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();
        self.make_up_gain = 10.0f32.powf(make_up_gain_db / 20.0);
        self.enabled = enabled;

        // Update sidechain filter (for detection)
        let sidechain_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ(6.0), // Detection boost
            self.sample_rate.hz(),
            self.detector_freq.hz(),
            self.q,
        )
        .unwrap();
        self.sidechain_filter = DirectForm1::<f32>::new(sidechain_coeff);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        // 1. Sidechain detection and envelope following
        let sidechain_signal = self.sidechain_filter.run(input);
        let detection_level = sidechain_signal.abs();

        if detection_level > self.envelope {
            self.envelope = detection_level + (self.envelope - detection_level) * self.attack_coeff;
        } else {
            self.envelope = detection_level + (self.envelope - detection_level) * self.release_coeff;
        }

        // 2. Calculate gain adjustment in dB
        let threshold_db = 20.0 * self.threshold.log10();
        let envelope_db = 20.0 * self.envelope.log10();
        let over_db = envelope_db - threshold_db;

        let mut gain_change_db = 0.0;
        match self.mode {
            DynamicMode::CompressDownward => {
                if over_db > 0.0 {
                    gain_change_db = -over_db * (1.0 - 1.0 / self.ratio);
                }
            }
            DynamicMode::ExpandUpward => {
                if over_db > 0.0 {
                    gain_change_db = over_db * (self.ratio - 1.0);
                }
            }
            DynamicMode::Gate => {
                if over_db < 0.0 {
                    gain_change_db = over_db * (1.0 - 1.0 / self.ratio);
                }
            }
        }
        self.gain_reduction_db = -gain_change_db;

        // 3. Apply dynamic gain to EQ filter
        let eq_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ(gain_change_db),
            self.sample_rate.hz(),
            self.frequency.hz(),
            self.q,
        )
        .unwrap();

        self.eq_filter = DirectForm1::<f32>::new(eq_coeff);

        // 4. Process signal and apply makeup gain
        self.eq_filter.run(input) * self.make_up_gain
    }
    
    fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain_reduction_db = 0.0;
    }
}


/// Professional 4-band Dynamic EQ
pub struct DynamicEQ {
    sample_rate: f32,
    bands: [DynamicBand; 4],
}

// To avoid a huge parameter list, we'll create a struct for band parameters
#[derive(Clone, Copy)]
pub struct DynamicBandParams {
    pub mode: DynamicMode,
    pub detector_freq: f32,
    pub freq: f32,
    pub q: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub gain_db: f32,
    pub enabled: bool,
}

impl DynamicEQ {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            bands: [
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
            ],
        }
    }

    pub fn update_parameters(&mut self, band_params: &[DynamicBandParams; 4]) {
        for (i, params) in band_params.iter().enumerate() {
            self.bands[i].update_parameters(
                params.mode,
                params.detector_freq,
                params.freq,
                params.q,
                params.threshold_db,
                params.ratio,
                params.attack_ms,
                params.release_ms,
                params.gain_db,
                params.enabled,
            );
        }
    }

    pub fn process(&mut self, buffer: &mut Buffer) {
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                for band in &mut self.bands {
                    s = band.process_sample(s);
                }
                *sample = s;
            }
        }
    }

    pub fn get_gain_reduction_db(&self) -> [f32; 4] {
        [
            self.bands[0].gain_reduction_db,
            self.bands[1].gain_reduction_db,
            self.bands[2].gain_reduction_db,
            self.bands[3].gain_reduction_db,
        ]
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}
