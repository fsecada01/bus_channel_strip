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

    /// Process a single sample.
    pub fn run(&mut self, sample: f32) -> f32 {
        self.filter.run(sample)
    }
}
