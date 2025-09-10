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
        // Initialize with stable allpass filters for true bypass
        let bypass_coeff = Coefficients::<f32>::from_params(
            Type::PeakingEQ(0.0), // 0dB peaking EQ = flat response
            sample_rate.hz(),
            1000.0_f32.hz(), // Mid frequency for stability
            0.707,
        ).expect("Bypass filter parameters should be valid");
        
        let mut bypass_filter = DirectForm1::<f32>::new(bypass_coeff);
        
        // Reset filter state to ensure clean startup
        bypass_filter.run(0.0);
        bypass_filter.run(0.0);
        bypass_filter.run(0.0);
        
        Self {
            sample_rate,
            lf_boost_filter: bypass_filter,
            lf_cut_filter: bypass_filter,
            hf_boost_filter: bypass_filter,
            hf_cut_filter: bypass_filter,
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
        
        // Low Frequency Boost (shelving filter) with conservative gain
        if lf_boost_gain > 0.01 {
            let shaped_gain = lf_boost_gain * lf_boost_gain; // Quadratic curve for smooth response
            let boost_db = shaped_gain * 8.0; // Reduced to 0-8dB range to prevent instability
            let safe_freq = lf_boost_freq.clamp(30.0, 200.0); // Limit frequency range
            let coeff = Coefficients::<f32>::from_params(
                Type::LowShelf(boost_db),
                self.sample_rate.hz(),
                safe_freq.hz(),
                0.9, // Slightly wider Q for stability
            ).expect("LF boost filter parameters should be valid");
            let mut new_filter = DirectForm1::<f32>::new(coeff);
            // Prime the filter to prevent startup transients
            new_filter.run(0.0);
            new_filter.run(0.0);
            self.lf_boost_filter = new_filter;
        } else {
            // True bypass - use stable allpass
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                self.sample_rate.hz(),
                100.0_f32.hz(), // Fixed safe frequency
                0.707,
            ).expect("LF bypass parameters should be valid");
            let mut bypass_filter = DirectForm1::<f32>::new(coeff);
            bypass_filter.run(0.0);
            bypass_filter.run(0.0);
            self.lf_boost_filter = bypass_filter;
        }
        
        // Low Frequency Cut (simultaneous with boost for classic Pultec behavior)
        if lf_cut_gain > 0.01 {
            let shaped_cut = lf_cut_gain * lf_cut_gain; // Simple quadratic for smooth response
            let cut_db = -(shaped_cut * 6.0); // Reduced to 0 to -6dB cut to prevent artifacts
            let cut_freq = (lf_boost_freq * 0.6).clamp(20.0, 120.0); // Cut below boost, limit range
            let coeff = Coefficients::<f32>::from_params(
                Type::LowShelf(cut_db),
                self.sample_rate.hz(),
                cut_freq.hz(),
                1.2, // Moderate Q for smooth cut
            ).expect("LF cut filter parameters should be valid");
            let mut new_filter = DirectForm1::<f32>::new(coeff);
            new_filter.run(0.0);
            new_filter.run(0.0);
            self.lf_cut_filter = new_filter;
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                self.sample_rate.hz(),
                80.0_f32.hz(), // Fixed safe frequency
                0.707,
            ).expect("LF cut bypass parameters should be valid");
            let mut bypass_filter = DirectForm1::<f32>::new(coeff);
            bypass_filter.run(0.0);
            bypass_filter.run(0.0);
            self.lf_cut_filter = bypass_filter;
        }
        
        // High Frequency Boost (peaking filter) with conservative gain
        if hf_boost_gain > 0.01 {
            let shaped_gain = hf_boost_gain * hf_boost_gain; // Quadratic curve
            let boost_db = shaped_gain * 10.0; // Reduced to 0-10dB to prevent harshness
            let shaped_bandwidth = hf_boost_bandwidth * hf_boost_bandwidth; // Smooth Q control
            let q = 0.6 + (shaped_bandwidth * 1.4); // More conservative Q range: 0.6 to 2.0
            let safe_freq = hf_boost_freq.clamp(3000.0, 20000.0); // Limit frequency range
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(boost_db),
                self.sample_rate.hz(),
                safe_freq.hz(),
                q,
            ).expect("HF boost filter parameters should be valid");
            let mut new_filter = DirectForm1::<f32>::new(coeff);
            new_filter.run(0.0);
            new_filter.run(0.0);
            self.hf_boost_filter = new_filter;
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                self.sample_rate.hz(),
                8000.0_f32.hz(), // Fixed safe frequency
                0.707,
            ).expect("HF boost bypass parameters should be valid");
            let mut bypass_filter = DirectForm1::<f32>::new(coeff);
            bypass_filter.run(0.0);
            bypass_filter.run(0.0);
            self.hf_boost_filter = bypass_filter;
        }
        
        // High Frequency Cut (separate from boost) with conservative scaling
        if hf_cut_gain > 0.01 {
            let shaped_cut = hf_cut_gain * hf_cut_gain; // Simple quadratic
            let cut_db = -(shaped_cut * 8.0); // Reduced to 0 to -8dB cut for gentleness
            let safe_freq = hf_cut_freq.clamp(5000.0, 20000.0); // Limit frequency range
            let coeff = Coefficients::<f32>::from_params(
                Type::HighShelf(cut_db),
                self.sample_rate.hz(),
                safe_freq.hz(),
                0.9, // Slightly wider Q for smoother response
            ).expect("HF cut filter parameters should be valid");
            let mut new_filter = DirectForm1::<f32>::new(coeff);
            new_filter.run(0.0);
            new_filter.run(0.0);
            self.hf_cut_filter = new_filter;
        } else {
            let coeff = Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                self.sample_rate.hz(),
                10000.0_f32.hz(), // Fixed safe frequency
                0.707,
            ).expect("HF cut bypass parameters should be valid");
            let mut bypass_filter = DirectForm1::<f32>::new(coeff);
            bypass_filter.run(0.0);
            bypass_filter.run(0.0);
            self.hf_cut_filter = bypass_filter;
        }
    }
    
    /// Process audio buffer through Pultec EQ
    pub fn process(&mut self, buffer: &mut Buffer) {
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                
                // Apply filters in Pultec-style order with soft clipping between stages
                // 1. Low frequency boost
                s = self.lf_boost_filter.run(s);
                s = s.clamp(-2.0, 2.0); // Prevent filter overflow
                
                // 2. Low frequency cut (can be simultaneous with boost)
                s = self.lf_cut_filter.run(s);
                s = s.clamp(-2.0, 2.0);
                
                // 3. High frequency boost  
                s = self.hf_boost_filter.run(s);
                s = s.clamp(-2.0, 2.0);
                
                // 4. High frequency cut
                s = self.hf_cut_filter.run(s);
                s = s.clamp(-2.0, 2.0);
                
                // 5. Very gentle tube saturation to reduce harshness
                if self.tube_drive > 0.01 {
                    let drive_amount = self.tube_drive * 0.3; // Much gentler tube drive
                    s = s.tanh() * (1.0 + drive_amount * 0.2); // Soft tube-style saturation
                }
                
                // Final soft clipping to prevent digital clipping
                *sample = s.clamp(-1.0, 1.0);
            }
        }
    }
}

