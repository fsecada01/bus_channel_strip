//! Punch Module - Combined Clipper + Transient Shaper
//!
//! A professional bus/mastering module designed to achieve louder mixes while
//! preserving perceived energy and punch. This module addresses the common problem
//! where clipping alone results in flat, lifeless mixes.
//!
//! Signal Flow:
//! ```text
//! [Input] -> [Gain] -> [Clipper] -----> [Mix] -> [Output]
//!                         |              ^
//!                    [Oversampling]      |
//!                         |              |
//!                    [Transient] --------+
//!                    Detector/Shaper
//!                    (parallel blend)
//! ```

use crate::oversampler::Oversampler;
use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// ============================================================================
// Clipping Mode Enum
// ============================================================================

/// Clipping algorithm modes
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum ClipMode {
    /// Hard clip: mathematically cleanest, most transparent for small amounts
    #[name = "Hard"]
    Hard,
    /// Soft clip (tanh): natural compression curve, warmer character
    #[name = "Soft"]
    Soft,
    /// Cubic soft clip: polynomial curve, reduced high-frequency harmonics
    #[name = "Cubic"]
    Cubic,
}

impl Default for ClipMode {
    fn default() -> Self {
        Self::Hard
    }
}

// ============================================================================
// Oversampling Factor Enum
// ============================================================================

/// Oversampling factor for anti-aliasing
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum OversamplingFactor {
    /// No oversampling (testing only)
    #[name = "1x"]
    X1,
    /// 4x oversampling - good for real-time mixing
    #[name = "4x"]
    X4,
    /// 8x oversampling - recommended default
    #[name = "8x"]
    X8,
    /// 16x oversampling - mastering quality
    #[name = "16x"]
    X16,
}

impl Default for OversamplingFactor {
    fn default() -> Self {
        Self::X4 // 4x gives good alias suppression at lower CPU cost; user can increase
    }
}

impl OversamplingFactor {
    /// Get the numeric factor
    pub fn factor(&self) -> usize {
        match self {
            Self::X1 => 1,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
        }
    }
}

// ============================================================================
// Envelope Follower for Transient Detection
// ============================================================================

/// Single-pole envelope follower with attack/release
struct EnvelopeFollower {
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl EnvelopeFollower {
    fn new(sample_rate: f32, attack_ms: f32, release_ms: f32) -> Self {
        Self {
            envelope: 0.0,
            attack_coeff: Self::time_to_coeff(attack_ms, sample_rate),
            release_coeff: Self::time_to_coeff(release_ms, sample_rate),
        }
    }

    #[inline]
    fn time_to_coeff(time_ms: f32, sample_rate: f32) -> f32 {
        if time_ms <= 0.0 {
            1.0
        } else {
            (-1.0 / (time_ms * 0.001 * sample_rate)).exp()
        }
    }

    fn update_times(&mut self, sample_rate: f32, attack_ms: f32, release_ms: f32) {
        self.attack_coeff = Self::time_to_coeff(attack_ms, sample_rate);
        self.release_coeff = Self::time_to_coeff(release_ms, sample_rate);
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let abs_input = input.abs();
        let coeff = if abs_input > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.envelope = coeff * self.envelope + (1.0 - coeff) * abs_input;
        self.envelope
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

// ============================================================================
// Transient Detector
// ============================================================================

/// Differential envelope transient detector
/// Uses fast - slow envelope to detect transients
struct TransientDetector {
    fast_envelope: EnvelopeFollower,
    slow_envelope: EnvelopeFollower,
    sensitivity: f32,
    smoothed_transient: f32,
    smoothing_coeff: f32,
}

impl TransientDetector {
    fn new(sample_rate: f32) -> Self {
        // Fast envelope: 0.5ms attack, 5ms release (captures transient onset)
        let fast_envelope = EnvelopeFollower::new(sample_rate, 0.5, 5.0);
        // Slow envelope: 20ms attack, 100ms release (captures body/sustain)
        let slow_envelope = EnvelopeFollower::new(sample_rate, 20.0, 100.0);

        Self {
            fast_envelope,
            slow_envelope,
            sensitivity: 0.5,
            smoothed_transient: 0.0,
            smoothing_coeff: Self::calc_smoothing_coeff(sample_rate, 2.0),
        }
    }

    fn calc_smoothing_coeff(sample_rate: f32, time_ms: f32) -> f32 {
        (-1.0 / (time_ms * 0.001 * sample_rate)).exp()
    }

    fn update_parameters(
        &mut self,
        sample_rate: f32,
        attack_time_ms: f32,
        release_time_ms: f32,
        sensitivity: f32,
    ) {
        // Fast envelope tracks transients
        self.fast_envelope
            .update_times(sample_rate, attack_time_ms * 0.1, attack_time_ms);
        // Slow envelope tracks body
        self.slow_envelope
            .update_times(sample_rate, release_time_ms * 0.2, release_time_ms);
        self.sensitivity = sensitivity;
        // Anti-click smoothing
        self.smoothing_coeff = Self::calc_smoothing_coeff(sample_rate, 1.0);
    }

    /// Process a sample and return transient amount (0.0 to 1.0+)
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let fast_env = self.fast_envelope.process(input);
        let slow_env = self.slow_envelope.process(input);

        // Differential: transient = how much faster is rising than slow
        let raw_transient = if slow_env > 0.0001 {
            ((fast_env - slow_env) / slow_env).max(0.0)
        } else {
            0.0
        };

        // Apply sensitivity scaling
        let scaled_transient = raw_transient * self.sensitivity * 4.0;

        // Smooth to prevent clicks
        self.smoothed_transient = self.smoothing_coeff * self.smoothed_transient
            + (1.0 - self.smoothing_coeff) * scaled_transient;

        self.smoothed_transient
    }

    fn reset(&mut self) {
        self.fast_envelope.reset();
        self.slow_envelope.reset();
        self.smoothed_transient = 0.0;
    }
}


// ============================================================================
// Clipper Algorithms
// ============================================================================

/// Hard clip: y = clamp(x, -threshold, threshold)
#[inline]
fn hard_clip(input: f32, threshold: f32) -> f32 {
    input.clamp(-threshold, threshold)
}

/// Soft clip using tanh saturation
#[inline]
fn soft_clip_tanh(input: f32, threshold: f32, softness: f32) -> f32 {
    if input.abs() < threshold * (1.0 - softness * 0.3) {
        // Below knee region - pass through
        input
    } else {
        // In saturation region
        let normalized = input / threshold;
        let drive = 1.0 + softness * 2.0;
        let saturated = (normalized * drive).tanh() / drive.tanh();
        // Clamp to [-1, 1] before scaling: tanh approaches but never reaches 1.0,
        // but with drive > 1 the division can yield values slightly above 1.0.
        saturated.clamp(-1.0, 1.0) * threshold
    }
}

/// Cubic soft clip: polynomial curve
#[inline]
fn soft_clip_cubic(input: f32, threshold: f32, softness: f32) -> f32 {
    let knee_start = threshold * (1.0 - softness * 0.5);

    if input.abs() < knee_start {
        input
    } else {
        let sign = input.signum();
        let abs_input = input.abs();

        // Cubic polynomial soft clip
        let x = (abs_input - knee_start) / (threshold - knee_start + 0.0001);
        let x_clamped = x.clamp(0.0, 2.0);

        // Cubic curve: y = x - x^3/3 (maps 0..sqrt(3) to 0..sqrt(3)*2/3)
        let cubic = if x_clamped < 1.0 {
            x_clamped - x_clamped * x_clamped * x_clamped / 3.0
        } else {
            // Beyond the knee, hard limit
            2.0 / 3.0
        };

        let output_range = threshold - knee_start;
        sign * (knee_start + cubic * output_range * 1.5)
    }
}

/// Apply clipping based on mode
#[inline]
fn apply_clipping(input: f32, threshold: f32, softness: f32, mode: ClipMode) -> f32 {
    match mode {
        ClipMode::Hard => {
            if softness > 0.01 {
                // Blend hard with soft for intermediate modes
                let hard = hard_clip(input, threshold);
                let soft = soft_clip_tanh(input, threshold, softness);
                hard * (1.0 - softness) + soft * softness
            } else {
                hard_clip(input, threshold)
            }
        }
        ClipMode::Soft => soft_clip_tanh(input, threshold, softness.max(0.5)),
        ClipMode::Cubic => soft_clip_cubic(input, threshold, softness.max(0.5)),
    }
}

// ============================================================================
// Punch Module - Main Processor
// ============================================================================

/// Professional Punch Module: Clipper + Transient Shaper
///
/// Designed to achieve louder mixes while preserving perceived energy and punch.
pub struct PunchModule {
    sample_rate: f32,

    // Clipper parameters
    clip_threshold: f32, // -12dB to 0dB (stored as linear)
    clip_mode: ClipMode, // Hard / Soft / Cubic
    softness: f32,       // 0.0 - 1.0
    oversampling: OversamplingFactor,

    // Transient shaper parameters
    attack: f32,       // -1.0 to +1.0 (cut to boost)
    sustain: f32,      // -1.0 to +1.0
    attack_time: f32,  // 0.1ms - 30ms
    release_time: f32, // 10ms - 500ms
    sensitivity: f32,  // 0.0 - 1.0

    // Global controls
    input_gain: f32,  // Linear gain
    output_gain: f32, // Linear gain
    mix: f32,         // 0.0 - 1.0 dry/wet

    // Internal state - per channel (stereo)
    transient_detector_l: TransientDetector,
    transient_detector_r: TransientDetector,
    oversampler_l: Oversampler,
    oversampler_r: Oversampler,

    // Metering (for GUI)
    current_gain_reduction: f32,
    current_transient_activity: f32,
}

impl PunchModule {
    /// Maximum oversampling factor
    const MAX_OS_FACTOR: usize = 16;
    /// Maximum block size for oversampling buffers
    const MAX_BLOCK_SIZE: usize = 8192;

    /// Create a new Punch module instance
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,

            // Default clipper settings
            clip_threshold: 0.891, // -1dB
            clip_mode: ClipMode::Hard,
            softness: 0.0,
            oversampling: OversamplingFactor::X4,

            // Default transient shaper settings
            attack: 0.2,         // +20% boost
            sustain: 0.0,        // Neutral
            attack_time: 5.0,    // 5ms
            release_time: 100.0, // 100ms
            sensitivity: 0.5,    // 50%

            // Default global controls
            input_gain: 1.0,
            output_gain: 1.0,
            mix: 1.0,

            // Initialize per-channel state
            transient_detector_l: TransientDetector::new(sample_rate),
            transient_detector_r: TransientDetector::new(sample_rate),
            oversampler_l: Oversampler::new(Self::MAX_OS_FACTOR, Self::MAX_BLOCK_SIZE),
            oversampler_r: Oversampler::new(Self::MAX_OS_FACTOR, Self::MAX_BLOCK_SIZE),

            // Metering
            current_gain_reduction: 0.0,
            current_transient_activity: 0.0,
        }
    }

    /// Update all parameters
    #[allow(clippy::too_many_arguments)]
    pub fn update_parameters(
        &mut self,
        // Clipper
        clip_threshold_db: f32,
        clip_mode: ClipMode,
        softness: f32,
        oversampling: OversamplingFactor,
        // Transient shaper
        attack: f32,
        sustain: f32,
        attack_time_ms: f32,
        release_time_ms: f32,
        sensitivity: f32,
        // Global
        input_gain_db: f32,
        output_gain_db: f32,
        mix: f32,
    ) {
        // Convert dB to linear
        self.clip_threshold = db_to_linear(clip_threshold_db);
        self.clip_mode = clip_mode;
        self.softness = softness.clamp(0.0, 1.0);
        self.oversampling = oversampling;

        self.attack = attack.clamp(-1.0, 1.0);
        self.sustain = sustain.clamp(-1.0, 1.0);
        self.attack_time = attack_time_ms.clamp(0.1, 30.0);
        self.release_time = release_time_ms.clamp(10.0, 500.0);
        self.sensitivity = sensitivity.clamp(0.0, 1.0);

        self.input_gain = db_to_linear(input_gain_db);
        self.output_gain = db_to_linear(output_gain_db);
        self.mix = mix.clamp(0.0, 1.0);

        // Update oversamplers
        let os_factor = self.oversampling.factor();
        self.oversampler_l.set_factor(os_factor);
        self.oversampler_r.set_factor(os_factor);

        // Update transient detectors at NATIVE sample rate.
        // Detection now runs pre-oversampling, so time constants are calibrated
        // to the native rate. Using oversampled rate would make them too fast
        // (e.g., 8x oversampled at 44.1kHz would be ~353kHz rate).
        self.transient_detector_l.update_parameters(
            self.sample_rate,
            self.attack_time,
            self.release_time,
            self.sensitivity,
        );
        self.transient_detector_r.update_parameters(
            self.sample_rate,
            self.attack_time,
            self.release_time,
            self.sensitivity,
        );
    }

    /// Process a stereo buffer in-place.
    ///
    /// Signal path (pumping-free design):
    ///   Input → InputGain → TransientShape → Oversample → Clip → Downsample → Mix → OutputGain
    ///
    /// The transient detector runs at the NATIVE sample rate on the pre-clip signal.
    /// Gain adjustment is applied BEFORE oversampling, so the clipper naturally
    /// limits any resulting peaks. This eliminates post-clip time-varying gain
    /// modulation, which was the root cause of the pumping artifacts.
    pub fn process(&mut self, buffer: &mut Buffer) {
        let os_factor = self.oversampling.factor();
        let mut temp_os_buffer = [0.0f32; Self::MAX_OS_FACTOR];

        let mut max_gr = 0.0f32;
        let mut max_transient = 0.0f32;

        for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Stack-allocated channel pointer array — no Vec, no heap allocation on the audio thread.
            // SAFETY: each pointer is derived from a valid `&mut f32` reference within this
            // sample block. Channels are processed sequentially, so no aliasing occurs.
            let mut channel_ptrs: [*mut f32; 2] = [std::ptr::null_mut(); 2];
            let mut num_channels = 0_usize;
            for s in channel_samples.iter_mut() {
                if num_channels < 2 {
                    channel_ptrs[num_channels] = s as *mut f32;
                    num_channels += 1;
                }
            }

            for ch_idx in 0..num_channels {
                // SAFETY: channel_ptrs[ch_idx] was assigned from a valid mutable reference
                // and remains valid for the duration of this loop body.
                let sample_ptr = channel_ptrs[ch_idx];
                let sample = unsafe { *sample_ptr };

                // 1. Apply input gain
                let gained = sample * self.input_gain;
                let dry = gained;

                let (oversampler, transient_detector) = if ch_idx == 0 {
                    (&mut self.oversampler_l, &mut self.transient_detector_l)
                } else {
                    (&mut self.oversampler_r, &mut self.transient_detector_r)
                };

                // 2. Detect transients at NATIVE sample rate on the pre-clip signal.
                //    Operating pre-clip avoids the feedback loop where clipping changes
                //    the envelope the detector is tracking.
                let transient_amount = transient_detector.process(gained);
                max_transient = max_transient.max(transient_amount);

                // 3. Apply transient shaping gain PRE-CLIP.
                //    Because the gain change happens before the clipper, any resulting
                //    peaks are naturally limited by the clipper — no pumping.
                let pre_clip = if self.attack.abs() > 0.001 || self.sustain.abs() > 0.001 {
                    let t = transient_amount.min(1.0);
                    // Transient (fast-onset) gain: boost/cut on signal attacks
                    let transient_mult = 1.0 + t * self.attack * 0.5;
                    // Sustain (slow-decay) gain: boost/cut on held portions
                    let sustain_mult = 1.0 + (1.0 - t) * self.sustain * 0.3;
                    // Blend based on transient amount; clamp to prevent extreme levels
                    let gain = (transient_mult * t + sustain_mult * (1.0 - t)).clamp(0.25, 2.0);
                    gained * gain
                } else {
                    gained
                };

                // 4. Oversample → Clip → Downsample
                let upsampled = oversampler.upsample(pre_clip, sample_idx);

                for (os_idx, &os_sample) in upsampled.iter().enumerate() {
                    let clipped = apply_clipping(
                        os_sample,
                        self.clip_threshold,
                        self.softness,
                        self.clip_mode,
                    );

                    // Gain reduction metering
                    if os_sample.abs() > 0.0001 {
                        let gr = (os_sample.abs() - clipped.abs()) / os_sample.abs();
                        max_gr = max_gr.max(gr);
                    }

                    temp_os_buffer[os_idx] = clipped;
                }

                let processed = oversampler.downsample(&temp_os_buffer[..os_factor], sample_idx);

                // 5. Mix and output
                let mixed = dry * (1.0 - self.mix) + processed * self.mix;
                let output = mixed * self.output_gain;

                // SAFETY: sample_ptr is valid and aligned (set above from NIH-plug buffer).
                unsafe {
                    *sample_ptr = output;
                }
            }
        }

        // Update metering (smoothed)
        self.current_gain_reduction = self.current_gain_reduction * 0.9 + max_gr * 0.1;
        self.current_transient_activity =
            self.current_transient_activity * 0.9 + max_transient * 0.1;
    }

    /// Reset all internal state
    pub fn reset(&mut self) {
        self.transient_detector_l.reset();
        self.transient_detector_r.reset();
        self.oversampler_l.reset();
        self.oversampler_r.reset();
        self.current_gain_reduction = 0.0;
        self.current_transient_activity = 0.0;
    }

    /// Get current gain reduction (0.0 - 1.0) for metering.
    /// Reserved for future clipper GR visualization.
    #[allow(dead_code)]
    pub fn get_gain_reduction(&self) -> f32 {
        self.current_gain_reduction
    }

    /// Get current transient activity (0.0 - 1.0+) for metering.
    /// Reserved for future transient detector visualization.
    #[allow(dead_code)]
    pub fn get_transient_activity(&self) -> f32 {
        self.current_transient_activity
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert decibels to linear gain
#[inline]
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear gain to decibels
#[inline]
#[allow(dead_code)]
fn linear_to_db(linear: f32) -> f32 {
    if linear > 0.0 {
        20.0 * linear.log10()
    } else {
        -120.0
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oversampler::{HB_NUM_TAPS, design_halfband_kaiser};

    #[test]
    fn test_hard_clip_basic() {
        // Signal below threshold - should pass through
        assert!((hard_clip(0.5, 1.0) - 0.5).abs() < 0.0001);

        // Signal at threshold - should pass through
        assert!((hard_clip(1.0, 1.0) - 1.0).abs() < 0.0001);

        // Signal above threshold - should clip
        assert!((hard_clip(1.5, 1.0) - 1.0).abs() < 0.0001);

        // Negative signal
        assert!((hard_clip(-1.5, 1.0) - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_soft_clip_tanh() {
        // Below threshold should pass through
        let result = soft_clip_tanh(0.3, 1.0, 0.5);
        assert!((result - 0.3).abs() < 0.01);

        // Above threshold should be reduced but not hard clipped
        let result = soft_clip_tanh(1.5, 1.0, 0.5);
        assert!(result < 1.5); // Should be reduced
        assert!(result > 0.8); // But not too much
    }

    #[test]
    fn test_soft_clip_cubic() {
        // Below knee should pass through
        let result = soft_clip_cubic(0.3, 1.0, 0.5);
        assert!((result - 0.3).abs() < 0.01);

        // In knee region should be smoothly limited
        let result = soft_clip_cubic(1.2, 1.0, 0.5);
        assert!(result <= 1.0);
    }

    #[test]
    fn test_envelope_follower() {
        let mut env = EnvelopeFollower::new(44100.0, 1.0, 100.0);

        // Initial state should be 0
        assert!(env.envelope < 0.0001);

        // Process a step input
        for _ in 0..100 {
            env.process(1.0);
        }

        // Envelope should have risen
        assert!(env.envelope > 0.5);
    }

    #[test]
    fn test_transient_detector() {
        let mut detector = TransientDetector::new(44100.0);

        // Process silence
        for _ in 0..100 {
            detector.process(0.0);
        }

        // Sharp transient should trigger detection
        let transient = detector.process(1.0);
        // The first sample after silence should register some transient activity
        // (may be small initially as envelopes need time to respond)

        // Continue with the transient - the peak detection comes shortly after
        let mut max_transient = transient;
        for _ in 0..50 {
            let t = detector.process(1.0);
            max_transient = max_transient.max(t);
        }

        // max_transient should be positive (transient was detected)
        assert!(max_transient > 0.0, "Transient should be detected");

        // Now process much longer sustained signal - detector should settle
        for _ in 0..2000 {
            detector.process(1.0);
        }
        let sustained = detector.process(1.0);

        // After long sustained signal, transient activity should be very low
        // because fast and slow envelopes have converged
        assert!(
            sustained < max_transient,
            "Sustained should be less than peak transient"
        );
    }

    #[test]
    fn test_db_conversion() {
        // 0dB should be 1.0
        assert!((db_to_linear(0.0) - 1.0).abs() < 0.0001);

        // -6dB should be ~0.5
        assert!((db_to_linear(-6.0) - 0.501).abs() < 0.01);

        // +6dB should be ~2.0
        assert!((db_to_linear(6.0) - 1.995).abs() < 0.01);
    }

    #[test]
    fn test_punch_module_creation() {
        let punch = PunchModule::new(44100.0);
        assert!((punch.clip_threshold - 0.891).abs() < 0.01); // -1dB
        assert_eq!(punch.clip_mode, ClipMode::Hard);
    }

    #[test]
    fn test_punch_module_update_parameters() {
        let mut punch = PunchModule::new(44100.0);

        punch.update_parameters(
            -3.0,                   // threshold
            ClipMode::Soft,         // mode
            0.5,                    // softness
            OversamplingFactor::X4, // oversampling
            0.5,                    // attack
            -0.2,                   // sustain
            10.0,                   // attack_time
            200.0,                  // release_time
            0.7,                    // sensitivity
            0.0,                    // input gain
            0.0,                    // output gain
            1.0,                    // mix
        );

        assert_eq!(punch.clip_mode, ClipMode::Soft);
        assert!((punch.softness - 0.5).abs() < 0.001);
        assert!((punch.attack - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_oversampler_factor_1() {
        let mut os = Oversampler::new(16, 1024);
        os.set_factor(1);

        let input = 0.5;
        let upsampled = os.upsample(input, 0);
        assert_eq!(upsampled.len(), 1);
        assert!((upsampled[0] - input).abs() < 0.001);

        // Copy values before calling downsample to avoid borrow conflict
        let upsampled_copy: Vec<f32> = upsampled.to_vec();
        let result = os.downsample(&upsampled_copy, 0);
        assert!((result - input).abs() < 0.001);
    }

    #[test]
    fn test_oversampler_factor_4() {
        let mut os = Oversampler::new(16, 1024);
        os.set_factor(4);

        let input = 1.0;
        let upsampled = os.upsample(input, 1); // idx 1 to test interpolation
        assert_eq!(upsampled.len(), 4);

        // All values should be reasonable
        for &val in upsampled {
            assert!(val.abs() <= 2.1);
        }
    }

    #[test]
    fn test_halfband_kaiser_design_properties() {
        let c = design_halfband_kaiser(8.0);
        // Center tap should dominate and approach 0.5 after normalization
        let center = HB_NUM_TAPS / 2;
        assert!(c[center] > 0.4 && c[center] < 0.6);
        // Halfband property: every other tap from center is zero.
        for (n, &v) in c.iter().enumerate() {
            let offset = n as i32 - center as i32;
            if offset != 0 && offset.unsigned_abs() % 2 == 0 {
                assert!(v.abs() < 1.0e-6, "tap {n} (offset {offset}) should be 0");
            }
        }
        // DC gain normalized to 1.0
        let sum: f32 = c.iter().sum();
        assert!((sum - 1.0).abs() < 1.0e-5);
        // Symmetric
        for k in 0..center {
            assert!((c[k] - c[HB_NUM_TAPS - 1 - k]).abs() < 1.0e-6);
        }
    }

    #[test]
    fn test_oversampler_dc_passthrough() {
        // After filter settles, a DC input should pass through at ~unity gain
        // through an upsample→downsample roundtrip.
        let mut os = Oversampler::new(16, 256);
        os.set_factor(4);

        let mut last = 0.0;
        for idx in 0..200 {
            let up = os.upsample(1.0, idx);
            let up_copy: [f32; 4] = [up[0], up[1], up[2], up[3]];
            last = os.downsample(&up_copy, idx);
        }
        // After steady state, unity gain at DC.
        assert!(
            (last - 1.0).abs() < 0.01,
            "DC roundtrip should be ~1.0, got {last}"
        );
    }

    #[test]
    fn test_oversampler_roundtrip_lowfreq_fidelity() {
        // A 1 kHz sine at 44.1 kHz should roundtrip with minimal attenuation.
        let mut os = Oversampler::new(16, 1024);
        os.set_factor(4);
        let sr = 44_100.0_f32;
        let freq = 1_000.0_f32;
        let two_pi = core::f32::consts::TAU;

        // Warm up filters past the FIR group delay.
        for idx in 0..200 {
            let x = (two_pi * freq * idx as f32 / sr).sin();
            let up = os.upsample(x, idx);
            let up_copy: [f32; 4] = [up[0], up[1], up[2], up[3]];
            let _ = os.downsample(&up_copy, idx);
        }

        // Measure peak magnitude of output over a full cycle.
        let mut max_out: f32 = 0.0;
        for idx in 200..400 {
            let x = (two_pi * freq * idx as f32 / sr).sin();
            let up = os.upsample(x, idx);
            let up_copy: [f32; 4] = [up[0], up[1], up[2], up[3]];
            let y = os.downsample(&up_copy, idx);
            max_out = max_out.max(y.abs());
        }
        // Passband fidelity: 1 kHz is far below the halfband cutoff (Fs/4 =
        // 11 kHz native), so magnitude should be essentially unity.
        assert!(
            max_out > 0.95 && max_out < 1.05,
            "1 kHz roundtrip magnitude out of range: {max_out}"
        );
    }

    /// Evaluate |H(e^{jw})| of the FIR at normalized radian frequency w.
    fn fir_mag(coeffs: &[f32; HB_NUM_TAPS], w: f32) -> f32 {
        let mut re = 0.0_f32;
        let mut im = 0.0_f32;
        for (n, &v) in coeffs.iter().enumerate() {
            let phase = w * n as f32;
            re += v * phase.cos();
            im -= v * phase.sin();
        }
        (re * re + im * im).sqrt()
    }

    #[test]
    fn test_halfband_freq_response() {
        let c = design_halfband_kaiser(8.0);
        let pi = core::f32::consts::PI;
        // DC: unity passband
        let h_dc = fir_mag(&c, 0.0);
        assert!(
            (h_dc - 1.0).abs() < 0.01,
            "|H(0)| should be ~1.0, got {h_dc}"
        );
        // Cutoff (π/2): halfband filters pass through at exactly 0.5
        let h_half = fir_mag(&c, pi * 0.5);
        assert!(
            (h_half - 0.5).abs() < 0.05,
            "|H(π/2)| should be ~0.5, got {h_half}"
        );
        // Nyquist (π): halfband zero
        let h_nyq = fir_mag(&c, pi);
        assert!(h_nyq < 1.0e-4, "|H(π)| should be ~0, got {h_nyq}");
        // Well into the stopband (0.8π): expect significant attenuation
        // relative to linear interpolation's ~-13 dB at Nyquist.
        let h_sb = fir_mag(&c, 0.8 * pi);
        assert!(
            h_sb < 0.1,
            "|H(0.8π)| should be in stopband (<0.1), got {h_sb}"
        );
    }
}
