use nice_plug::prelude::*;

#[cfg(feature = "haas")]
use crate::haas::CombMode;

#[derive(Params)]
pub struct HaasParams {
    #[cfg(feature = "haas")]
    #[id = "haas_bypass"]
    pub haas_bypass: BoolParam,
    #[cfg(feature = "haas")]
    #[id = "haas_mid_gain"]
    pub haas_mid_gain: FloatParam,
    #[cfg(feature = "haas")]
    #[id = "haas_side_gain"]
    pub haas_side_gain: FloatParam,
    #[cfg(feature = "haas")]
    #[id = "haas_comb_depth"]
    pub haas_comb_depth: FloatParam,
    #[cfg(feature = "haas")]
    #[id = "haas_comb_time"]
    pub haas_comb_time: FloatParam,
    #[cfg(feature = "haas")]
    #[id = "haas_comb_mode"]
    pub haas_comb_mode: EnumParam<CombMode>,
    #[cfg(feature = "haas")]
    #[id = "haas_mix"]
    pub haas_mix: FloatParam,
}

impl Default for HaasParams {
    fn default() -> Self {
        Self {
            // Default: BYPASSED so the chain remains audibly unchanged on
            // first load. User must engage Haas intentionally.
            #[cfg(feature = "haas")]
            haas_bypass: BoolParam::new("Haas Bypass", true),
            #[cfg(feature = "haas")]
            haas_mid_gain: FloatParam::new(
                "Haas Mid",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 6.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit(" dB")
            .with_step_size(0.1)
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            #[cfg(feature = "haas")]
            haas_side_gain: FloatParam::new(
                "Haas Side",
                0.0,
                FloatRange::Linear {
                    min: -6.0,
                    max: 6.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit(" dB")
            .with_step_size(0.1)
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            #[cfg(feature = "haas")]
            haas_comb_depth: FloatParam::new(
                "Haas Depth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit("")
            .with_step_size(0.01),
            #[cfg(feature = "haas")]
            haas_comb_time: FloatParam::new(
                "Haas Time",
                7.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(0.1)
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            #[cfg(feature = "haas")]
            haas_comb_mode: EnumParam::new("Haas Mode", CombMode::SideComb),
            #[cfg(feature = "haas")]
            haas_mix: FloatParam::new("Haas Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(5.0))
                .with_unit("")
                .with_step_size(0.01),
        }
    }
}
