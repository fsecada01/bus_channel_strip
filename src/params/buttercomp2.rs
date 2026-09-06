use nice_plug::prelude::*;

#[cfg(feature = "buttercomp2")]
use crate::buttercomp2::{ButterComp2Model, FetRatio};

#[derive(Params)]
pub struct ButterComp2Params {
    // ButterComp2 Compressor Parameters
    #[id = "comp_bypass"]
    pub comp_bypass: BoolParam,
    #[id = "comp_compress"]
    pub comp_compress: FloatParam,
    #[id = "comp_output"]
    pub comp_output: FloatParam,
    #[id = "comp_dry_wet"]
    pub comp_dry_wet: FloatParam,

    /// #18: true restores the original fixed-shape envelope follower exactly
    /// (bypasses the program-dependent release-time adaptation). Only
    /// affects the FFI-wrapped Classic model.
    #[cfg(feature = "buttercomp2")]
    #[id = "comp_adaptive_env_bypass"]
    pub comp_adaptive_env_bypass: BoolParam,

    /// Model selector — always visible; switches the active control surface.
    #[cfg(feature = "buttercomp2")]
    #[id = "comp_model"]
    pub comp_model: EnumParam<ButterComp2Model>,

    /// Sidechain HP corner (20..400 Hz). Shared across VCA and FET models —
    /// both use linked peak/RMS detection and benefit equally from removing
    /// low-frequency energy from the detector path. 20 Hz = effectively off.
    #[cfg(feature = "buttercomp2")]
    #[id = "comp_sc_hp"]
    pub comp_sc_hp_freq: FloatParam,

    // VCA model parameters
    #[id = "comp_vca_thresh"]
    pub vca_thresh: FloatParam,
    #[id = "comp_vca_ratio"]
    pub vca_ratio: FloatParam,
    #[id = "comp_vca_atk"]
    pub vca_atk: FloatParam,
    #[id = "comp_vca_rel"]
    pub vca_rel: FloatParam,

    // Optical model parameters
    #[id = "comp_opt_thresh"]
    pub opt_thresh: FloatParam,
    #[id = "comp_opt_speed"]
    pub opt_speed: FloatParam,
    #[id = "comp_opt_char"]
    pub opt_char: FloatParam,

    // 1176-style FET compressor parameters
    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_input"]
    pub fet_input_db: FloatParam,

    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_output"]
    pub fet_output_db: FloatParam,

    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_atk"]
    pub fet_attack_ms: FloatParam,

    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_rel"]
    pub fet_release_ms: FloatParam,

    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_ratio"]
    pub fet_ratio: EnumParam<FetRatio>,

    #[cfg(feature = "buttercomp2")]
    #[id = "comp_fet_auto"]
    pub fet_auto_release: BoolParam,
}

impl Default for ButterComp2Params {
    fn default() -> Self {
        Self {
            // ButterComp2 Compressor Parameters
            comp_bypass: BoolParam::new("Comp Bypass", true),

            comp_compress: FloatParam::new(
                "Compress",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            comp_output: FloatParam::new(
                "Comp Output",
                0.5, // 0.5 = unity gain
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            comp_dry_wet: FloatParam::new(
                "Comp Mix",
                1.0, // 1.0 = fully wet
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            // false = adaptive release-time scaling on by default, matching
            // #16's default-ON-with-bypass precedent.
            #[cfg(feature = "buttercomp2")]
            comp_adaptive_env_bypass: BoolParam::new("Comp Adaptive Env Bypass", false),

            #[cfg(feature = "buttercomp2")]
            comp_model: EnumParam::<ButterComp2Model>::new("Model", ButterComp2Model::default()),

            // Default 20 Hz = filter is effectively off, matching legacy
            // sessions exactly. Users crank it up to 80–160 Hz for mix-bus use.
            comp_sc_hp_freq: FloatParam::new(
                "SC HP",
                20.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            // VCA model parameters
            vca_thresh: FloatParam::new(
                "VCA Threshold",
                -18.0,
                FloatRange::Linear {
                    min: -60.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_ratio: FloatParam::new(
                "VCA Ratio",
                4.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 20.0,
                },
            )
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_atk: FloatParam::new(
                "VCA Attack",
                10.0,
                FloatRange::Linear {
                    min: 0.1,
                    max: 100.0,
                },
            )
            .with_unit(" ms")
            .with_step_size(0.1)
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_rel: FloatParam::new(
                "VCA Release",
                100.0,
                FloatRange::Linear {
                    min: 10.0,
                    max: 1000.0,
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            // Optical model parameters
            opt_thresh: FloatParam::new(
                "Opt Threshold",
                -12.0,
                FloatRange::Linear {
                    min: -60.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            opt_speed: FloatParam::new("Opt Speed", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01)
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
                .with_smoother(SmoothingStyle::Linear(5.0)),

            opt_char: FloatParam::new(
                "Opt Character",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01)
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
            .with_smoother(SmoothingStyle::Linear(5.0)),

            // 1176-style FET compressor parameters
            #[cfg(feature = "buttercomp2")]
            fet_input_db: FloatParam::new(
                "FET Input",
                0.0,
                FloatRange::Linear {
                    min: -20.0,
                    max: 40.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_output_db: FloatParam::new(
                "FET Output",
                0.0,
                FloatRange::Linear {
                    min: -20.0,
                    max: 20.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_attack_ms: FloatParam::new(
                "FET Attack",
                0.2,
                FloatRange::Skewed {
                    min: 0.02,
                    max: 0.8,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(0.01)
            .with_value_to_string(formatters::v2s_f32_rounded(2))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_release_ms: FloatParam::new(
                "FET Release",
                250.0,
                FloatRange::Skewed {
                    min: 50.0,
                    max: 1100.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_ratio: EnumParam::<FetRatio>::new("FET Ratio", FetRatio::R4),

            #[cfg(feature = "buttercomp2")]
            fet_auto_release: BoolParam::new("FET Auto Release", false),
        }
    }
}
