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

/// Musical shaping functions for analog modeling
/// Based on AGENTS.md guidelines for perceptual and smooth behavior

/// Sigmoid soft saturation for musical compression and soft knees
pub fn sigmoid(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// Hyperbolic tangent for tube-style saturation
pub fn tanh_saturation(x: f32, drive: f32) -> f32 {
    let driven = x * (1.0 + drive * 2.0);
    driven.tanh() * (1.0 / (1.0 + drive * 0.5)) // Compensate level
}

/// Logarithmic gain scaling for perceptual parameter behavior
pub fn log_gain_scale(linear_param: f32, min_db: f32, max_db: f32) -> f32 {
    if linear_param <= 0.0 {
        min_db
    } else {
        min_db + (max_db - min_db) * linear_param.ln() / 1.0_f32.ln()
    }
}

/// Exponential curve for musical frequency response
pub fn exp_curve(x: f32, curve_amount: f32) -> f32 {
    let shaped = if curve_amount > 0.0 {
        // Exponential rise
        (x.powf(1.0 + curve_amount * 2.0) - x) * curve_amount + x
    } else {
        // Logarithmic curve  
        let log_curve = -curve_amount;
        x - (x - x.powf(1.0 / (1.0 + log_curve * 2.0))) * log_curve
    };
    shaped.clamp(0.0, 1.0)
}

/// Polynomial + logarithmic shaping for filter/tone controls
pub fn poly_log_curve(x: f32, poly_amount: f32, log_amount: f32) -> f32 {
    let poly_part = x + poly_amount * (x * x * x - x);
    let log_part = if x > 0.0 { 
        log_amount * (1.0 + x).ln() / 2.0_f32.ln() 
    } else { 
        0.0 
    };
    (poly_part + log_part).clamp(0.0, 1.0)
}

/// Soft knee compression curve using sigmoid
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
    let output_level = threshold + compressed_over;
    input.signum() * output_level
}
