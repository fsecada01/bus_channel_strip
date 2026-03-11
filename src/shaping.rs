use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};

/// Enum for the type of filter to use.
pub enum FilterType {
    Bell,
    LowShelf,
    HighShelf,
}

/// A single biquad filter with parameters.
pub struct Filter {
    filter: DirectForm1<f32>,
}

impl Filter {
    /// Create a new filter with the given parameters.
    pub fn new(sample_rate: f32, filter_type: FilterType, freq: f32, q: f32, gain: f32) -> Self {
        let filter_type = match filter_type {
            FilterType::Bell => Type::PeakingEQ(gain),
            FilterType::LowShelf => Type::LowShelf(gain),
            FilterType::HighShelf => Type::HighShelf(gain),
        };

        let coeff = Coefficients::<f32>::from_params(
            filter_type,
            sample_rate.hz(),
            freq.hz(),
            q,
        )
        .expect("Failed to create filter coefficients");

        Self {
            filter: DirectForm1::<f32>::new(coeff),
        }
    }

    /// Update filter parameters without recreating the filter structure
    pub fn update_parameters(&mut self, sample_rate: f32, filter_type: FilterType, freq: f32, q: f32, gain: f32) {
        let filter_type = match filter_type {
            FilterType::Bell => Type::PeakingEQ(gain),
            FilterType::LowShelf => Type::LowShelf(gain),
            FilterType::HighShelf => Type::HighShelf(gain),
        };

        let coeff = Coefficients::<f32>::from_params(
            filter_type,
            sample_rate.hz(),
            freq.hz(),
            q,
        )
        .expect("Failed to create filter coefficients");

        // Update coefficients without clearing filter memory
        self.filter.update_coefficients(coeff);
    }

    /// Process a single sample with soft clipping to prevent harsh distortion.
    pub fn run(&mut self, sample: f32) -> f32 {
        let output = self.filter.run(sample);
        // Apply soft clipping using tanh for musical distortion behavior
        if output.abs() > 0.95 {
            output.signum() * (1.0 - (-3.0 * output.abs()).exp())
        } else {
            output
        }
    }
}

/// Musical shaping functions for analog modeling.
/// These are DSP building blocks available to all modules.
#[allow(dead_code)]
pub mod shaping_fns {
    /// Sigmoid soft saturation — smooth soft-knee compression curve.
    pub fn sigmoid(x: f32) -> f32 {
        x / (1.0 + x.abs())
    }

    /// Hyperbolic tangent tube-style saturation with level compensation.
    pub fn tanh_saturation(x: f32, drive: f32) -> f32 {
        let driven = x * (1.0 + drive * 2.0);
        driven.tanh() * (1.0 / (1.0 + drive * 0.5))
    }

    /// Exponential curve for musical frequency response shaping.
    pub fn exp_curve(x: f32, curve_amount: f32) -> f32 {
        let shaped = if curve_amount > 0.0 {
            (x.powf(1.0 + curve_amount * 2.0) - x) * curve_amount + x
        } else {
            let log_curve = -curve_amount;
            x - (x - x.powf(1.0 / (1.0 + log_curve * 2.0))) * log_curve
        };
        shaped.clamp(0.0, 1.0)
    }

    /// Polynomial + logarithmic shaping for filter/tone controls.
    pub fn poly_log_curve(x: f32, poly_amount: f32, log_amount: f32) -> f32 {
        let poly_part = x + poly_amount * (x * x * x - x);
        let log_part = if x > 0.0 {
            log_amount * (1.0 + x).ln() / 2.0_f32.ln()
        } else {
            0.0
        };
        (poly_part + log_part).clamp(0.0, 1.0)
    }

    /// Soft knee compression using sigmoid for smooth gain reduction.
    pub fn soft_knee_compress(input: f32, threshold: f32, ratio: f32, knee_width: f32) -> f32 {
        let over_threshold = (input.abs() - threshold).max(0.0);
        if over_threshold <= 0.0 {
            return input;
        }
        let knee_ratio = if knee_width > 0.0 {
            let knee_pos = (over_threshold / knee_width).clamp(0.0, 1.0);
            1.0 + (ratio - 1.0) * sigmoid(knee_pos * 4.0 - 2.0) * 0.5 + 0.5
        } else {
            ratio
        };
        let compressed_over = over_threshold / knee_ratio;
        input.signum() * (threshold + compressed_over)
    }
}

#[cfg(test)]
mod tests {
    use super::shaping_fns::*;
    use super::{Filter, FilterType};

    // ── sigmoid ───────────────────────────────────────────────────────────────

    #[test]
    fn test_sigmoid_zero() {
        assert!((sigmoid(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_sigmoid_bounded() {
        // sigmoid(x) = x / (1 + |x|) — stays strictly inside (-1, 1)
        assert!(sigmoid(1000.0).abs() < 1.0);
        assert!(sigmoid(-1000.0).abs() < 1.0);
    }

    #[test]
    fn test_sigmoid_antisymmetric() {
        // sigmoid(-x) == -sigmoid(x)
        for &x in &[0.1, 1.0, 5.0, 50.0] {
            assert!((sigmoid(x) + sigmoid(-x)).abs() < 1e-6, "sigmoid not antisymmetric at {x}");
        }
    }

    #[test]
    fn test_sigmoid_monotone() {
        // Larger |x| → larger |sigmoid(x)|
        assert!(sigmoid(2.0) > sigmoid(1.0));
        assert!(sigmoid(-2.0) < sigmoid(-1.0));
    }

    // ── tanh_saturation ───────────────────────────────────────────────────────

    #[test]
    fn test_tanh_saturation_zero_input() {
        assert!((tanh_saturation(0.0, 0.0)).abs() < 1e-6);
        assert!((tanh_saturation(0.0, 1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_tanh_saturation_no_drive() {
        // drive=0: output = tanh(x) * 1.0
        let x = 0.5_f32;
        let expected = x.tanh();
        let result = tanh_saturation(x, 0.0);
        assert!((result - expected).abs() < 1e-5, "drive=0 expected {expected}, got {result}");
    }

    #[test]
    fn test_tanh_saturation_drive_saturates() {
        // For LARGE inputs, higher drive limits output more (saturation dominates over drive boost).
        // drive=0: tanh(10 * 1) * 1.0 ≈ 1.0
        // drive=1: tanh(10 * 3) * (1/1.5) ≈ 1.0 * 0.667 = 0.667
        let x = 10.0_f32;
        let no_drive = tanh_saturation(x, 0.0).abs();
        let full_drive = tanh_saturation(x, 1.0).abs();
        // At large amplitude the drive denominator (1 + drive*0.5) dominates, reducing output
        assert!(full_drive < no_drive, "At large amplitude, high drive should reduce output via 1/(1+drive*0.5)");
    }

    #[test]
    fn test_tanh_saturation_bounded() {
        // Output should be bounded for extreme inputs
        let result = tanh_saturation(100.0, 1.0);
        assert!(result.is_finite(), "Output must be finite");
        assert!(result.abs() < 2.0, "Output should be bounded: {result}");
    }

    // ── exp_curve ─────────────────────────────────────────────────────────────

    #[test]
    fn test_exp_curve_zero_amount_is_identity() {
        for &x in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let result = exp_curve(x, 0.0);
            assert!((result - x).abs() < 1e-5, "exp_curve({x}, 0) should equal {x}, got {result}");
        }
    }

    #[test]
    fn test_exp_curve_output_clamped() {
        // Output should always be in [0, 1]
        for &x in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            for &curve in &[-1.0, -0.5, 0.0, 0.5, 1.0] {
                let result = exp_curve(x, curve);
                assert!(
                    result >= 0.0 && result <= 1.0,
                    "exp_curve({x}, {curve}) = {result} out of [0,1]"
                );
            }
        }
    }

    // ── poly_log_curve ────────────────────────────────────────────────────────

    #[test]
    fn test_poly_log_curve_zero_params_identity() {
        let result = poly_log_curve(0.5, 0.0, 0.0);
        assert!((result - 0.5).abs() < 1e-5, "zero params should be identity");
    }

    #[test]
    fn test_poly_log_curve_output_clamped() {
        for &x in &[0.0, 0.5, 1.0] {
            let result = poly_log_curve(x, 0.5, 0.5);
            assert!(
                result >= 0.0 && result <= 1.0,
                "poly_log_curve({x}, 0.5, 0.5) = {result} out of [0,1]"
            );
        }
    }

    #[test]
    fn test_poly_log_curve_zero_input() {
        // log part is guarded by x > 0.0 check — should not NaN
        let result = poly_log_curve(0.0, 0.5, 0.5);
        assert!(result.is_finite(), "poly_log_curve(0) should be finite");
    }

    // ── soft_knee_compress ────────────────────────────────────────────────────

    #[test]
    fn test_soft_knee_below_threshold_passes_through() {
        let input = 0.3_f32;
        let result = soft_knee_compress(input, 0.5, 4.0, 0.1);
        assert!((result - input).abs() < 1e-5, "below threshold: expected {input}, got {result}");
    }

    #[test]
    fn test_soft_knee_above_threshold_reduces_signal() {
        let input = 0.8_f32;
        let result = soft_knee_compress(input, 0.5, 4.0, 0.1);
        assert!(result < input, "above threshold: signal should be compressed");
        assert!(result > 0.0, "compressed signal should be positive");
    }

    #[test]
    fn test_soft_knee_preserves_sign() {
        let pos = soft_knee_compress(0.8, 0.5, 4.0, 0.1);
        let neg = soft_knee_compress(-0.8, 0.5, 4.0, 0.1);
        assert!(pos > 0.0, "positive input should give positive output");
        assert!(neg < 0.0, "negative input should give negative output");
        assert!((pos + neg).abs() < 1e-5, "should be antisymmetric");
    }

    #[test]
    fn test_soft_knee_zero_knee_is_hard_ratio() {
        // With knee_width=0 the function uses hard ratio
        let input = 0.8_f32;
        let threshold = 0.5_f32;
        let ratio = 4.0_f32;
        let result = soft_knee_compress(input, threshold, ratio, 0.0);
        let expected = threshold + (input - threshold) / ratio;
        assert!((result - expected).abs() < 1e-5, "zero knee: expected {expected}, got {result}");
    }

    // ── Filter ────────────────────────────────────────────────────────────────

    #[test]
    fn test_filter_bell_creation_does_not_panic() {
        let _f = Filter::new(44100.0, FilterType::Bell, 1000.0, 0.707, 0.0);
    }

    #[test]
    fn test_filter_low_shelf_creation_does_not_panic() {
        let _f = Filter::new(44100.0, FilterType::LowShelf, 200.0, 0.707, 0.0);
    }

    #[test]
    fn test_filter_high_shelf_creation_does_not_panic() {
        let _f = Filter::new(44100.0, FilterType::HighShelf, 8000.0, 0.707, 0.0);
    }

    #[test]
    fn test_filter_zero_gain_steady_state() {
        // A 0 dB filter should reach steady-state output equal to its DC input
        let mut f = Filter::new(44100.0, FilterType::Bell, 1000.0, 0.707, 0.0);
        // Warm up with DC
        for _ in 0..2000 {
            f.run(0.5);
        }
        let out = f.run(0.5);
        assert!((out - 0.5).abs() < 0.01, "0 dB Bell steady-state: {out}");
    }

    #[test]
    fn test_filter_update_parameters_does_not_panic() {
        let mut f = Filter::new(44100.0, FilterType::Bell, 1000.0, 0.707, 0.0);
        f.update_parameters(44100.0, FilterType::Bell, 2000.0, 1.0, 6.0);
        f.update_parameters(48000.0, FilterType::LowShelf, 200.0, 0.707, -3.0);
    }

    #[test]
    fn test_filter_soft_clip_prevents_extreme_output() {
        // Peaking filter at high gain — soft clipping should keep output <= 1.0
        let mut f = Filter::new(44100.0, FilterType::Bell, 1000.0, 0.707, 12.0);
        for _ in 0..200 {
            f.run(2.0); // hot input
        }
        let out = f.run(2.0);
        assert!(out.abs() <= 1.0, "Soft clip must limit output to [-1, 1]; got {out}");
    }
}
