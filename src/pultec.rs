use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use nih_plug::buffer::Buffer;

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
}

impl PultecEQ {
    /// Create a new Pultec EQ with the given sample rate
    pub fn new(sample_rate: f32) -> Self {
        // Initialize with flat response filters
        let flat_coeff = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate.hz(),
            20000.0_f32.hz(),
            0.707,
        ).expect("AllPass filter parameters should be valid");
        
        let flat_filter = DirectForm1::<f32>::new(flat_coeff);
        
        Self {
            sample_rate,
            lf_boost_filter: flat_filter,
            lf_cut_filter: flat_filter,
            hf_boost_filter: flat_filter,
            hf_cut_filter: flat_filter,
            tube_drive: 0.0,
        }
    }
    
    /// Update Pultec parameters
    /// 
    /// # Arguments
    /// * `lf_boost_freq` - Low frequency boost center (20, 30, 60, 100 Hz)
    /// * `lf_boost_gain` - Low frequency boost amount (0.0 to 1.0)
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
        lf_cut_gain: f32,
        hf_boost_freq: f32,
        hf_boost_gain: f32,
        hf_boost_bandwidth: f32,
        hf_cut_freq: f32,
        hf_cut_gain: f32,
        tube_drive: f32,
    ) {
        // Update tube drive
        self.tube_drive = tube_drive.clamp(0.0, 1.0);
        
        // Low Frequency Boost (shelving filter)
        if lf_boost_gain > 0.01 {
            let boost_db = lf_boost_gain * 15.0; // 0-15dB range
            let coeff = Coefficients::<f32>::from_params(
                Type::LowShelf,
                self.sample_rate.hz(),
                lf_boost_freq.hz(),
                0.707, // Classic Pultec Q
            ).expect("LF boost filter parameters should be valid");
            self.lf_boost_filter = DirectForm1::<f32>::new(coeff.set_gain(boost_db));
        } else {
            // Flat response when no boost
            let coeff = Coefficients::<f32>::from_params(
                Type::LowPass,
                self.sample_rate.hz(),
                lf_boost_freq.hz(),
                0.707,
            ).expect("LF allpass parameters should be valid");
            self.lf_boost_filter = DirectForm1::<f32>::new(coeff);
        }
        
        // Low Frequency Cut (simultaneous with boost for classic Pultec behavior)
        if lf_cut_gain > 0.01 {
            let cut_db = -(lf_cut_gain * 12.0); // 0 to -12dB cut
            let cut_freq = lf_boost_freq * 0.5; // Cut below boost frequency
            let coeff = Coefficients::<f32>::from_params(
                Type::LowShelf,
                self.sample_rate.hz(),
                cut_freq.hz(),
                1.4, // Wider Q for cut
            ).expect("LF cut filter parameters should be valid");
            self.lf_cut_filter = DirectForm1::<f32>::new(coeff.set_gain(cut_db));
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::LowPass,
                self.sample_rate.hz(),
                lf_boost_freq.hz(),
                0.707,
            ).expect("LF cut allpass parameters should be valid");
            self.lf_cut_filter = DirectForm1::<f32>::new(coeff);
        }
        
        // High Frequency Boost (peaking filter)
        if hf_boost_gain > 0.01 {
            let boost_db = hf_boost_gain * 18.0; // 0-18dB range (Pultec can be generous)
            let q = 0.5 + (hf_boost_bandwidth * 2.0); // 0.5 to 2.5 Q range
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ,
                self.sample_rate.hz(),
                hf_boost_freq.hz(),
                q,
            ).expect("HF boost filter parameters should be valid");
            self.hf_boost_filter = DirectForm1::<f32>::new(coeff.set_gain(boost_db));
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::LowPass,
                self.sample_rate.hz(),
                hf_boost_freq.hz(),
                0.707,
            ).expect("HF boost allpass parameters should be valid");
            self.hf_boost_filter = DirectForm1::<f32>::new(coeff);
        }
        
        // High Frequency Cut (separate from boost)
        if hf_cut_gain > 0.01 {
            let cut_db = -(hf_cut_gain * 15.0); // 0 to -15dB cut
            let coeff = Coefficients::<f32>::from_params(
                Type::HighShelf,
                self.sample_rate.hz(),
                hf_cut_freq.hz(),
                0.707,
            ).expect("HF cut filter parameters should be valid");
            self.hf_cut_filter = DirectForm1::<f32>::new(coeff.set_gain(cut_db));
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::LowPass,
                self.sample_rate.hz(),
                hf_cut_freq.hz(),
                0.707,
            ).expect("HF cut allpass parameters should be valid");
            self.hf_cut_filter = DirectForm1::<f32>::new(coeff);
        }
    }
    
    /// Process audio buffer through Pultec EQ
    pub fn process(&mut self, buffer: &mut Buffer) {
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                
                // Apply filters in Pultec-style order
                // 1. Low frequency boost
                s = self.lf_boost_filter.run(s);
                
                // 2. Low frequency cut (can be simultaneous with boost)
                s = self.lf_cut_filter.run(s);
                
                // 3. High frequency boost  
                s = self.hf_boost_filter.run(s);
                
                // 4. High frequency cut
                s = self.hf_cut_filter.run(s);
                
                // 5. Tube saturation modeling (soft clipping with harmonics)
                if self.tube_drive > 0.01 {
                    s = tube_saturation(s, self.tube_drive);
                }
                
                *sample = s;
            }
        }
    }
}

/// Tube saturation modeling
/// 
/// Models the harmonic distortion and soft clipping characteristics of tube circuits
fn tube_saturation(input: f32, drive: f32) -> f32 {
    if drive < 0.01 {
        return input;
    }
    
    // Scale input by drive amount
    let driven = input * (1.0 + drive * 3.0);
    
    // Soft clipping with asymmetric characteristics (tube-like)
    let output = if driven > 0.0 {
        // Positive half-cycle (slightly harder clipping)
        driven / (1.0 + driven.abs().powf(0.8))
    } else {
        // Negative half-cycle (softer clipping)
        driven / (1.0 + driven.abs().powf(0.6))
    };
    
    // Add subtle even harmonics (tube characteristic)
    let harmonics = output * output * drive * 0.1;
    
    // Mix back to appropriate level
    let wet = output + harmonics;
    let dry = input;
    
    // Blend based on drive amount
    dry * (1.0 - drive * 0.5) + wet * (drive * 0.5)
}