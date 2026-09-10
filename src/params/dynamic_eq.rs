use nice_plug::prelude::*;

#[cfg(feature = "dynamic_eq")]
use crate::dynamic_eq::DynamicMode;

#[derive(Params)]
pub struct DynamicEqParams {
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_bypass"]
    pub dyneq_bypass: BoolParam,

    // Band 1 (Low) - 200Hz default
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_freq"]
    pub dyneq_band1_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_threshold"]
    pub dyneq_band1_threshold: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_ratio"]
    pub dyneq_band1_ratio: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_attack"]
    pub dyneq_band1_attack: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_release"]
    pub dyneq_band1_release: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_gain"]
    pub dyneq_band1_gain: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_q"]
    pub dyneq_band1_q: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_enabled"]
    pub dyneq_band1_enabled: BoolParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_detector_freq"]
    pub dyneq_band1_detector_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_mode"]
    pub dyneq_band1_mode: EnumParam<DynamicMode>,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band1_solo"]
    pub dyneq_band1_solo: BoolParam,

    // Band 2 (Low-Mid) - 800Hz default
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_freq"]
    pub dyneq_band2_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_threshold"]
    pub dyneq_band2_threshold: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_ratio"]
    pub dyneq_band2_ratio: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_attack"]
    pub dyneq_band2_attack: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_release"]
    pub dyneq_band2_release: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_gain"]
    pub dyneq_band2_gain: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_q"]
    pub dyneq_band2_q: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_enabled"]
    pub dyneq_band2_enabled: BoolParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_detector_freq"]
    pub dyneq_band2_detector_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_mode"]
    pub dyneq_band2_mode: EnumParam<DynamicMode>,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band2_solo"]
    pub dyneq_band2_solo: BoolParam,

    // Band 3 (High-Mid) - 3kHz default
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_freq"]
    pub dyneq_band3_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_threshold"]
    pub dyneq_band3_threshold: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_ratio"]
    pub dyneq_band3_ratio: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_attack"]
    pub dyneq_band3_attack: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_release"]
    pub dyneq_band3_release: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_gain"]
    pub dyneq_band3_gain: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_q"]
    pub dyneq_band3_q: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_enabled"]
    pub dyneq_band3_enabled: BoolParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_detector_freq"]
    pub dyneq_band3_detector_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_mode"]
    pub dyneq_band3_mode: EnumParam<DynamicMode>,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band3_solo"]
    pub dyneq_band3_solo: BoolParam,

    // Band 4 (High) - 8kHz default
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_freq"]
    pub dyneq_band4_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_threshold"]
    pub dyneq_band4_threshold: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_ratio"]
    pub dyneq_band4_ratio: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_attack"]
    pub dyneq_band4_attack: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_release"]
    pub dyneq_band4_release: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_gain"]
    pub dyneq_band4_gain: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_q"]
    pub dyneq_band4_q: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_enabled"]
    pub dyneq_band4_enabled: BoolParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_detector_freq"]
    pub dyneq_band4_detector_freq: FloatParam,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_mode"]
    pub dyneq_band4_mode: EnumParam<DynamicMode>,
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_solo"]
    pub dyneq_band4_solo: BoolParam,
}

impl Default for DynamicEqParams {
    fn default() -> Self {
        Self {
            #[cfg(feature = "dynamic_eq")]
            // Dynamic EQ Parameters
            dyneq_bypass: BoolParam::new("DynEQ Bypass", true),

            #[cfg(feature = "dynamic_eq")]
            // Band 1 (Low) - 200Hz
            dyneq_band1_freq: FloatParam::new(
                "DynEQ 1 Freq",
                200.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_threshold: FloatParam::new(
                "DynEQ 1 Thresh",
                -18.0,
                FloatRange::Linear { min: -60.0, max: 0.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_ratio: FloatParam::new(
                "DynEQ 1 Ratio",
                4.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_attack: FloatParam::new(
                "DynEQ 1 Attack",
                10.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 200.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_release: FloatParam::new(
                "DynEQ 1 Release",
                100.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_gain: FloatParam::new(
                "DynEQ 1 Gain",
                0.0,
                FloatRange::Linear { min: -18.0, max: 18.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_q: FloatParam::new(
                "DynEQ 1 Q",
                1.0,
                FloatRange::Skewed {
                    min: 0.3,
                    max: 8.0,
                    factor: FloatRange::skew_factor(0.5),
                },
            )
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_enabled: BoolParam::new("DynEQ 1 On", true),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_detector_freq: FloatParam::new(
                "DynEQ 1 Detector Freq",
                200.0, // Same as main frequency by default
                FloatRange::Skewed {
                    min: 20.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_mode: EnumParam::new("DynEQ 1 Mode", DynamicMode::CompressDownward),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_solo: BoolParam::new("DynEQ 1 Solo", false),

            #[cfg(feature = "dynamic_eq")]
            // Band 2 (Low-Mid) - 800Hz (similar pattern, different defaults)
            dyneq_band2_freq: FloatParam::new(
                "DynEQ 2 Freq",
                800.0,
                FloatRange::Skewed {
                    min: 200.0,
                    max: 5000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_threshold: FloatParam::new("DynEQ 2 Thresh", -18.0, FloatRange::Linear { min: -60.0, max: 0.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_ratio: FloatParam::new("DynEQ 2 Ratio", 4.0, FloatRange::Skewed { min: 1.0, max: 20.0, factor: FloatRange::skew_factor(-1.5) }).with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_attack: FloatParam::new("DynEQ 2 Attack", 10.0, FloatRange::Skewed { min: 0.1, max: 200.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_release: FloatParam::new("DynEQ 2 Release", 100.0, FloatRange::Skewed { min: 1.0, max: 2000.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_gain: FloatParam::new("DynEQ 2 Gain", 0.0, FloatRange::Linear { min: -18.0, max: 18.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_q: FloatParam::new("DynEQ 2 Q", 1.0, FloatRange::Skewed { min: 0.3, max: 8.0, factor: FloatRange::skew_factor(0.5) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_enabled: BoolParam::new("DynEQ 2 On", true),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_detector_freq: FloatParam::new(
                "DynEQ 2 Detector Freq",
                800.0, // Same as main frequency by default
                FloatRange::Skewed {
                    min: 200.0,
                    max: 5000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_mode: EnumParam::new("DynEQ 2 Mode", DynamicMode::CompressDownward),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_solo: BoolParam::new("DynEQ 2 Solo", false),

            #[cfg(feature = "dynamic_eq")]
            // Band 3 (High-Mid) - 3kHz
            dyneq_band3_freq: FloatParam::new(
                "DynEQ 3 Freq",
                3000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 15000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_threshold: FloatParam::new("DynEQ 3 Thresh", -18.0, FloatRange::Linear { min: -60.0, max: 0.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_ratio: FloatParam::new("DynEQ 3 Ratio", 4.0, FloatRange::Skewed { min: 1.0, max: 20.0, factor: FloatRange::skew_factor(-1.5) }).with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_attack: FloatParam::new("DynEQ 3 Attack", 5.0, FloatRange::Skewed { min: 0.1, max: 200.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_release: FloatParam::new("DynEQ 3 Release", 60.0, FloatRange::Skewed { min: 1.0, max: 2000.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_gain: FloatParam::new("DynEQ 3 Gain", 0.0, FloatRange::Linear { min: -18.0, max: 18.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_q: FloatParam::new("DynEQ 3 Q", 1.0, FloatRange::Skewed { min: 0.3, max: 8.0, factor: FloatRange::skew_factor(0.5) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_enabled: BoolParam::new("DynEQ 3 On", true),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_detector_freq: FloatParam::new(
                "DynEQ 3 Det Freq",
                3000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 15000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_mode: EnumParam::new("DynEQ 3 Mode", DynamicMode::CompressDownward),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_solo: BoolParam::new("DynEQ 3 Solo", false),

            #[cfg(feature = "dynamic_eq")]
            // Band 4 (High) - 8kHz
            dyneq_band4_freq: FloatParam::new(
                "DynEQ 4 Freq",
                8000.0,
                FloatRange::Skewed {
                    min: 3000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_threshold: FloatParam::new("DynEQ 4 Thresh", -18.0, FloatRange::Linear { min: -60.0, max: 0.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_ratio: FloatParam::new("DynEQ 4 Ratio", 4.0, FloatRange::Skewed { min: 1.0, max: 20.0, factor: FloatRange::skew_factor(-1.5) }).with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_attack: FloatParam::new("DynEQ 4 Attack", 2.0, FloatRange::Skewed { min: 0.1, max: 200.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_release: FloatParam::new("DynEQ 4 Release", 30.0, FloatRange::Skewed { min: 1.0, max: 2000.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_gain: FloatParam::new("DynEQ 4 Gain", 0.0, FloatRange::Linear { min: -18.0, max: 18.0 }).with_unit(" dB").with_step_size(1.0).with_value_to_string(formatters::v2s_f32_rounded(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_q: FloatParam::new("DynEQ 4 Q", 1.0, FloatRange::Skewed { min: 0.3, max: 8.0, factor: FloatRange::skew_factor(0.5) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_enabled: BoolParam::new("DynEQ 4 On", true),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_detector_freq: FloatParam::new(
                "DynEQ 4 Det Freq",
                8000.0,
                FloatRange::Skewed {
                    min: 3000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_mode: EnumParam::new("DynEQ 4 Mode", DynamicMode::CompressDownward),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_solo: BoolParam::new("DynEQ 4 Solo", false),
        }
    }
}
