use nih_plug::buffer::Buffer;


// ButterComp2 FFI bindings
#[repr(C)]
pub struct ButterComp2StateOpaque {
    _private: [u8; 0],
}

pub type ButterComp2State = ButterComp2StateOpaque;

extern "C" {
    fn buttercomp2_create(sample_rate: f64) -> *mut ButterComp2State;
    fn buttercomp2_destroy(state: *mut ButterComp2State);
    fn buttercomp2_set_compress(state: *mut ButterComp2State, compress: f64);
    fn buttercomp2_set_output(state: *mut ButterComp2State, output: f64);
    fn buttercomp2_set_dry_wet(state: *mut ButterComp2State, dry_wet: f64);
    fn buttercomp2_process_stereo(
        state: *mut ButterComp2State,
        left_channel: *mut f32,
        right_channel: *mut f32,
        num_samples: i32,
    );
    fn buttercomp2_reset(state: *mut ButterComp2State);
}

/// ButterComp2 wrapper for Rust integration
/// 
/// Airwindows ButterComp2: "The single richest, lushest 'glue' compressor" 
/// Features 4 independent compressors per channel in bipolar, interleaved configuration
pub struct ButterComp2 {
    state: *mut ButterComp2State,
    sample_rate: f32,
}

impl ButterComp2 {
    /// Create a new ButterComp2 instance
    pub fn new(sample_rate: f32) -> Self {
        let state = unsafe { buttercomp2_create(sample_rate as f64) };
        assert!(!state.is_null(), "Failed to create ButterComp2 state");
        
        Self {
            state,
            sample_rate,
        }
    }
    
    /// Update compressor parameters
    /// 
    /// # Arguments
    /// * `compress` - Compression amount (0.0 to 1.0, maps to 0-14dB)
    /// * `output` - Output gain (0.0 to 1.0, maps to 0-2x gain)
    /// * `dry_wet` - Dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn update_parameters(&mut self, compress: f32, output: f32, dry_wet: f32) {
        // Scale parameters to prevent over-compression and distortion
        let safe_compress = (compress * 0.5).clamp(0.0, 0.5); // Reduce max compression
        let safe_output = (output * 0.8 + 0.2).clamp(0.2, 1.0); // Keep output in reasonable range
        let safe_dry_wet = dry_wet.clamp(0.0, 1.0);
        
        unsafe {
            buttercomp2_set_compress(self.state, safe_compress as f64);
            buttercomp2_set_output(self.state, safe_output as f64);
            buttercomp2_set_dry_wet(self.state, safe_dry_wet as f64);
        }
    }
    
    /// Process audio buffer in place
    /// 
    /// Requires stereo input (2 channels)
    pub fn process(&mut self, buffer: &mut Buffer) {
        // Process buffer sample by sample for stereo
        for samples in buffer.iter_samples() {
            let mut channels: Vec<&mut f32> = samples.into_iter().collect();
            if channels.len() >= 2 {
                let mut left = *channels[0];
                let mut right = *channels[1];
                
                unsafe {
                    buttercomp2_process_stereo(
                        self.state,
                        &mut left,
                        &mut right,
                        1,
                    );
                }
                
                *channels[0] = left;
                *channels[1] = right;
            }
        }
    }
    
    /// Reset internal state
    pub fn reset(&mut self) {
        unsafe {
            buttercomp2_reset(self.state);
        }
    }
}

impl Drop for ButterComp2 {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe {
                buttercomp2_destroy(self.state);
            }
        }
    }
}

unsafe impl Send for ButterComp2 {}
unsafe impl Sync for ButterComp2 {}