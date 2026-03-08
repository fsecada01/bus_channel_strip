use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// ============================================================================
// ButterComp2 Model Enum
// ============================================================================

/// Selectable compressor personality for the ButterComp2 slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ButterComp2Model {
    #[name = "Classic"]
    Classic,
    #[name = "Optical"]
    Optical,
    #[name = "VCA"]
    Vca,
}

impl Default for ButterComp2Model {
    fn default() -> Self { ButterComp2Model::Classic }
}

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
}

impl ButterComp2 {
    /// Create a new ButterComp2 instance
    pub fn new(sample_rate: f32) -> Self {
        let state = unsafe { buttercomp2_create(sample_rate as f64) };
        assert!(!state.is_null(), "Failed to create ButterComp2 state");
        Self { state }
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
    
    /// Process audio buffer in place (stereo, lock-free, allocation-free).
    ///
    /// Calls the C++ function once per buffer (O(1) FFI overhead) rather than
    /// once per sample (O(block_size) overhead). The C++ implementation loops
    /// over `num_samples` internally — see buttercomp2_process_stereo in cpp/.
    pub fn process(&mut self, buffer: &mut Buffer) {
        let num_samples = buffer.samples();
        if num_samples == 0 { return; }

        // Capture a *mut f32 to the start of each channel from the first sample
        // iteration. NIH-plug guarantees each channel is a contiguous, non-overlapping
        // f32 slice, so ch[n]+i accesses sample i of channel n.
        let mut ch: [*mut f32; 2] = [std::ptr::null_mut(); 2];
        let mut count = 0usize;
        if let Some(first) = buffer.iter_samples().next() {
            for s in first {
                if count < 2 {
                    ch[count] = s as *mut f32;
                    count += 1;
                }
            }
        }

        if count >= 2 {
            // Safety: ch[0] and ch[1] are valid *mut f32 pointers to the first
            // element of non-overlapping, contiguous channel slices of length
            // num_samples. buttercomp2_process_stereo iterates [0..num_samples).
            unsafe {
                buttercomp2_process_stereo(self.state, ch[0], ch[1], num_samples as i32);
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