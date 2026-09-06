use nice_plug::prelude::*;

#[derive(Params)]
pub struct GlobalParams {
    /// Global bypass — passes audio through without touching any module.
    #[id = "global_bypass"]
    pub global_bypass: BoolParam,

    /// Global auto-gain — compensates for loudness changes introduced by the chain.
    #[id = "global_auto_gain"]
    pub global_auto_gain: BoolParam,

    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for GlobalParams {
    fn default() -> Self {
        Self {
            global_bypass: BoolParam::new("Bypass", false),
            global_auto_gain: BoolParam::new("Auto Gain", false),

            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}
