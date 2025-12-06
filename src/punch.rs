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
        Self::X8
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
// Oversampler
// ============================================================================

/// Simple polyphase oversampler with linear interpolation and lowpass filtering
struct Oversampler {
    factor: usize,
    upsample_buffer: Vec<f32>,
    downsample_buffer: Vec<f32>,
    // Simple FIR lowpass filter state (for anti-aliasing)
    filter_state: [f32; 8],
}

impl Oversampler {
    fn new(max_factor: usize, max_block_size: usize) -> Self {
        Self {
            factor: max_factor,
            upsample_buffer: vec![0.0; max_block_size * max_factor],
            downsample_buffer: vec![0.0; max_block_size],
            filter_state: [0.0; 8],
        }
    }

    fn set_factor(&mut self, factor: usize) {
        self.factor = factor;
    }

    /// Upsample a single sample to the oversampled buffer
    /// Returns a slice of the upsampled values
    #[inline]
    fn upsample(&mut self, input: f32, idx: usize) -> &[f32] {
        let start = idx * self.factor;
        let end = start + self.factor;

        if self.factor == 1 {
            self.upsample_buffer[start] = input;
        } else {
            // Zero-stuffing with linear interpolation between samples
            let prev = if idx > 0 {
                self.downsample_buffer[idx - 1]
            } else {
                self.filter_state[0]
            };

            for i in 0..self.factor {
                let t = i as f32 / self.factor as f32;
                self.upsample_buffer[start + i] = prev * (1.0 - t) + input * t;
            }
        }

        &self.upsample_buffer[start..end]
    }

    /// Downsample from the oversampled buffer back to the original rate
    #[inline]
    fn downsample(&mut self, processed: &[f32], idx: usize) -> f32 {
        if self.factor == 1 {
            processed[0]
        } else {
            // Simple averaging with slight lowpass
            let sum: f32 = processed.iter().sum();
            let result = sum / self.factor as f32;

            // Simple IIR lowpass to reduce aliasing
            let prev = self.filter_state[0];
            let filtered = prev * 0.3 + result * 0.7;
            self.filter_state[0] = filtered;

            self.downsample_buffer[idx] = filtered;
            filtered
        }
    }

    fn reset(&mut self) {
        self.filter_state = [0.0; 8];
        self.downsample_buffer.fill(0.0);
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
        saturated * threshold
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
// Transient Shaper
// ============================================================================

/// Apply transient shaping gain
#[inline]
fn apply_transient_shaping(
    clipped: f32,
    _dry: f32,
    transient_amount: f32,
    attack_gain: f32,  // -1.0 to +1.0 (cut to boost)
    sustain_gain: f32, // -1.0 to +1.0
) -> f32 {
    // Transient component: the difference between fast and slow envelope detection
    // attack_gain > 0: boost transients, attack_gain < 0: cut transients

    // Calculate transient boost/cut with gentler scaling to avoid artifacts
    // Reduced from 1.5 to 0.8 to prevent low-mid thump at higher attack values
    let transient_mult = if attack_gain > 0.0 {
        // Boost transients - gentler scaling prevents harsh artifacts
        1.0 + transient_amount * attack_gain * 0.8
    } else {
        // Cut transients
        1.0 / (1.0 - attack_gain * transient_amount * 0.8).max(0.5)
    };

    // Calculate sustain adjustment (inverse of transient)
    let sustain_mult = if sustain_gain > 0.0 {
        // Boost sustain (non-transient portions)
        1.0 + (1.0 - transient_amount).max(0.0) * sustain_gain * 0.5
    } else {
        // Cut sustain
        1.0 - (1.0 - transient_amount).max(0.0) * sustain_gain.abs() * 0.3
    };

    // Blend transient-shaped and original clipped signal
    let transient_component = clipped * transient_mult;
    let sustain_component = clipped * sustain_mult;

    // The final output blends both components based on transient detection
    let transient_weight = transient_amount.clamp(0.0, 1.0);
    transient_component * transient_weight + sustain_component * (1.0 - transient_weight)
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
    clip_threshold: f32,      // -12dB to 0dB (stored as linear)
    clip_mode: ClipMode,      // Hard / Soft / Cubic
    softness: f32,            // 0.0 - 1.0
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

    // Smoothing for transient gain to prevent artifacts
    transient_gain_smooth_l: f32,
    transient_gain_smooth_r: f32,

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
            oversampling: OversamplingFactor::X8,

            // Default transient shaper settings
            attack: 0.2,       // +20% boost
            sustain: 0.0,      // Neutral
            attack_time: 5.0,  // 5ms
            release_time: 100.0, // 100ms
            sensitivity: 0.5,  // 50%

            // Default global controls
            input_gain: 1.0,
            output_gain: 1.0,
            mix: 1.0,

            // Initialize per-channel state
            transient_detector_l: TransientDetector::new(sample_rate),
            transient_detector_r: TransientDetector::new(sample_rate),
            oversampler_l: Oversampler::new(Self::MAX_OS_FACTOR, Self::MAX_BLOCK_SIZE),
            oversampler_r: Oversampler::new(Self::MAX_OS_FACTOR, Self::MAX_BLOCK_SIZE),

            // Smoothing state
            transient_gain_smooth_l: 1.0,
            transient_gain_smooth_r: 1.0,

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

        // Update transient detectors
        let effective_sample_rate = self.sample_rate * os_factor as f32;
        self.transient_detector_l.update_parameters(
            effective_sample_rate,
            self.attack_time,
            self.release_time,
            self.sensitivity,
        );
        self.transient_detector_r.update_parameters(
            effective_sample_rate,
            self.attack_time,
            self.release_time,
            self.sensitivity,
        );
    }

    /// Process a stereo buffer in-place
    pub fn process(&mut self, buffer: &mut Buffer) {
        let os_factor = self.oversampling.factor();
        let mut temp_os_buffer = [0.0f32; Self::MAX_OS_FACTOR];

        // Track metering
        let mut max_gr = 0.0f32;
        let mut max_transient = 0.0f32;

        for (sample_idx, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Get channel samples
            let channels: Vec<*mut f32> = channel_samples.iter_mut().map(|s| s as *mut f32).collect();

            // Process each channel
            for (ch_idx, sample_ptr) in channels.iter().enumerate() {
                let sample = unsafe { **sample_ptr };

                // 1. Apply input gain
                let gained = sample * self.input_gain;
                let dry = gained;

                // 2. Upsample
                let (oversampler, transient_detector, transient_smooth) = if ch_idx == 0 {
                    (&mut self.oversampler_l, &mut self.transient_detector_l, &mut self.transient_gain_smooth_l)
                } else {
                    (&mut self.oversampler_r, &mut self.transient_detector_r, &mut self.transient_gain_smooth_r)
                };

                let upsampled = oversampler.upsample(gained, sample_idx);

                // 3. Process each oversampled sample
                for (os_idx, &os_sample) in upsampled.iter().enumerate() {
                    // Transient detection
                    let transient_amount = transient_detector.process(os_sample);
                    max_transient = max_transient.max(transient_amount);

                    // Clipping
                    let clipped = apply_clipping(
                        os_sample,
                        self.clip_threshold,
                        self.softness,
                        self.clip_mode,
                    );

                    // Calculate gain reduction for metering
                    if os_sample.abs() > 0.0001 {
                        let gr = (os_sample.abs() - clipped.abs()) / os_sample.abs();
                        max_gr = max_gr.max(gr);
                    }

                    // Transient shaping
                    let shaped = apply_transient_shaping(
                        clipped,
                        os_sample,
                        transient_amount,
                        self.attack,
                        self.sustain,
                    );

                    // Apply smoothing to prevent abrupt gain changes (reduces low-mid thump)
                    // One-pole lowpass at oversampled rate (~1-2ms time constant)
                    let smooth_coeff = 0.05; // Adjust for sample rate
                    *transient_smooth = *transient_smooth * (1.0 - smooth_coeff) + shaped * smooth_coeff;

                    temp_os_buffer[os_idx] = *transient_smooth;
                }

                // 4. Downsample
                let processed = oversampler.downsample(&temp_os_buffer[..os_factor], sample_idx);

                // 5. Apply mix and output gain
                let mixed = dry * (1.0 - self.mix) + processed * self.mix;
                let output = mixed * self.output_gain;

                // Write output
                unsafe {
                    **sample_ptr = output;
                }
            }
        }

        // Update metering (smoothed)
        self.current_gain_reduction =
            self.current_gain_reduction * 0.9 + max_gr * 0.1;
        self.current_transient_activity =
            self.current_transient_activity * 0.9 + max_transient * 0.1;
    }

    /// Reset all internal state
    pub fn reset(&mut self) {
        self.transient_detector_l.reset();
        self.transient_detector_r.reset();
        self.oversampler_l.reset();
        self.oversampler_r.reset();
        self.transient_gain_smooth_l = 1.0;
        self.transient_gain_smooth_r = 1.0;
        self.current_gain_reduction = 0.0;
        self.current_transient_activity = 0.0;
    }

    /// Get current gain reduction (0.0 - 1.0) for metering
    pub fn get_gain_reduction(&self) -> f32 {
        self.current_gain_reduction
    }

    /// Get current transient activity (0.0 - 1.0+) for metering
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
        assert!(sustained < max_transient, "Sustained should be less than peak transient");
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
            -3.0,                       // threshold
            ClipMode::Soft,             // mode
            0.5,                        // softness
            OversamplingFactor::X4,     // oversampling
            0.5,                        // attack
            -0.2,                       // sustain
            10.0,                       // attack_time
            200.0,                      // release_time
            0.7,                        // sensitivity
            0.0,                        // input gain
            0.0,                        // output gain
            1.0,                        // mix
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
            assert!(val.abs() <= 1.5);
        }
    }
}
