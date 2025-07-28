use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use nih_plug::buffer::Buffer;

/// Professional 4-band Dynamic EQ
/// 
/// Frequency-dependent compression that only applies EQ when signal exceeds threshold
/// Each band has independent frequency, threshold, ratio, attack, release, and gain
pub struct DynamicEQ {
    sample_rate: f32,
    bands: [DynamicBand; 4],
}

/// Individual dynamic EQ band
struct DynamicBand {
    // Filter components
    sidechain_filter: DirectForm1<f32>,  // For detection
    eq_filter: DirectForm1<f32>,         // For processing
    
    // Dynamic processing state
    envelope: f32,
    gain_reduction: f32,
    
    // Parameters (stored internally for efficiency)
    frequency: f32,
    threshold: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    make_up_gain: f32,
    enabled: bool,
}

impl DynamicBand {
    fn new(sample_rate: f32) -> Self {
        let flat_coeff = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate.hz(),
            20000.0_f32.hz(),
            0.707,
        ).expect("AllPass filter should be valid");
        
        Self {
            sidechain_filter: DirectForm1::<f32>::new(flat_coeff),
            eq_filter: DirectForm1::<f32>::new(flat_coeff),
            envelope: 0.0,
            gain_reduction: 0.0,
            frequency: 1000.0,
            threshold: 0.5, // -6dB
            ratio: 3.0,
            attack_coeff: 0.001,
            release_coeff: 0.0001,
            make_up_gain: 0.0,
            enabled: false,
        }
    }
    
    /// Update band parameters
    fn update_parameters(
        &mut self,
        sample_rate: f32,
        frequency: f32,
        threshold: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        make_up_gain: f32,
        q: f32,
        enabled: bool,
    ) {
        self.frequency = frequency;
        self.threshold = threshold;
        self.ratio = ratio;
        self.make_up_gain = make_up_gain;
        self.enabled = enabled;
        
        // Calculate attack/release coefficients
        self.attack_coeff = (-1.0 / (attack_ms * 0.001 * sample_rate)).exp();
        self.release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();
        
        // Update sidechain filter (for detection)
        let sidechain_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ(6.0),
            sample_rate.hz(),
            frequency.hz(),
            q,
        ).expect("Sidechain filter should be valid");
        self.sidechain_filter = DirectForm1::<f32>::new(sidechain_coeff);
        
        // Update EQ filter (for processing) - this will be dynamically modulated
        let eq_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ,
            sample_rate.hz(),
            frequency.hz(),
            q,
        ).expect("EQ filter should be valid");
        self.eq_filter = DirectForm1::<f32>::new(eq_coeff);
    }
    
    /// Process a single sample through the dynamic band
    fn process_sample(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }
        
        // 1. Sidechain detection
        let sidechain_signal = self.sidechain_filter.run(input);
        let detection_level = sidechain_signal.abs();
        
        // 2. Envelope following
        let target_envelope = detection_level;
        if target_envelope > self.envelope {
            // Attack
            self.envelope = target_envelope + (self.envelope - target_envelope) * self.attack_coeff;
        } else {
            // Release
            self.envelope = target_envelope + (self.envelope - target_envelope) * self.release_coeff;
        }
        
        // 3. Calculate gain reduction
        if self.envelope > self.threshold {
            let over_threshold = self.envelope - self.threshold;
            let compressed = over_threshold / self.ratio;
            self.gain_reduction = over_threshold - compressed;
        } else {
            self.gain_reduction = 0.0;
        }
        
        // 4. Apply dynamic gain to EQ filter
        let dynamic_gain = -self.gain_reduction * 12.0 + self.make_up_gain; // Convert to dB
        let eq_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ,
            44100.0_f32.hz(), // Use stored sample rate
            self.frequency.hz(),
            0.707, // Q stored separately
        ).expect("Dynamic EQ filter should be valid");
        self.eq_filter = DirectForm1::<f32>::new(eq_coeff.set_gain(dynamic_gain));
        
        // 5. Process signal through EQ
        self.eq_filter.run(input)
    }
}

impl DynamicEQ {
    /// Create new 4-band Dynamic EQ
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            bands: [
                DynamicBand::new(sample_rate), // Band 1: Low
                DynamicBand::new(sample_rate), // Band 2: Low-Mid
                DynamicBand::new(sample_rate), // Band 3: High-Mid
                DynamicBand::new(sample_rate), // Band 4: High
            ],
        }
    }
    
    /// Update parameters for all bands
    pub fn update_parameters(
        &mut self,
        // Band 1 parameters
        band1_freq: f32,
        band1_threshold: f32,
        band1_ratio: f32,
        band1_attack: f32,
        band1_release: f32,
        band1_gain: f32,
        band1_q: f32,
        band1_enabled: bool,
        
        // Band 2 parameters
        band2_freq: f32,
        band2_threshold: f32,
        band2_ratio: f32,
        band2_attack: f32,
        band2_release: f32,
        band2_gain: f32,
        band2_q: f32,
        band2_enabled: bool,
        
        // Band 3 parameters
        band3_freq: f32,
        band3_threshold: f32,
        band3_ratio: f32,
        band3_attack: f32,
        band3_release: f32,
        band3_gain: f32,
        band3_q: f32,
        band3_enabled: bool,
        
        // Band 4 parameters
        band4_freq: f32,
        band4_threshold: f32,
        band4_ratio: f32,
        band4_attack: f32,
        band4_release: f32,
        band4_gain: f32,
        band4_q: f32,
        band4_enabled: bool,
    ) {
        let params = [
            (band1_freq, band1_threshold, band1_ratio, band1_attack, band1_release, band1_gain, band1_q, band1_enabled),
            (band2_freq, band2_threshold, band2_ratio, band2_attack, band2_release, band2_gain, band2_q, band2_enabled),
            (band3_freq, band3_threshold, band3_ratio, band3_attack, band3_release, band3_gain, band3_q, band3_enabled),
            (band4_freq, band4_threshold, band4_ratio, band4_attack, band4_release, band4_gain, band4_q, band4_enabled),
        ];
        
        for (i, &(freq, thresh, ratio, attack, release, gain, q, enabled)) in params.iter().enumerate() {
            self.bands[i].update_parameters(
                self.sample_rate,
                freq,
                thresh,
                ratio,
                attack,
                release,
                gain,
                q,
                enabled,
            );
        }
    }
    
    /// Process audio buffer through dynamic EQ
    pub fn process(&mut self, buffer: &mut Buffer) {
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                
                // Process through each band in series
                for band in &mut self.bands {
                    s = band.process_sample(s);
                }
                
                *sample = s;
            }
        }
    }
    
    /// Get gain reduction for visualization (returns [band1_gr, band2_gr, band3_gr, band4_gr])
    pub fn get_gain_reduction(&self) -> [f32; 4] {
        [
            self.bands[0].gain_reduction,
            self.bands[1].gain_reduction,
            self.bands[2].gain_reduction,
            self.bands[3].gain_reduction,
        ]
    }
    
    /// Reset all band states
    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.envelope = 0.0;
            band.gain_reduction = 0.0;
        }
    }
}