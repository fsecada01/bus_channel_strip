use nice_plug::prelude::*;

#[cfg(feature = "punch")]
use crate::punch::{ClipMode, OversamplingFactor};

#[derive(Params)]
pub struct PunchParams {
    // Punch Module Parameters (Clipper + Transient Shaper)
    #[cfg(feature = "punch")]
    #[id = "punch_bypass"]
    pub punch_bypass: BoolParam,
    // Clipper section
    #[cfg(feature = "punch")]
    #[id = "punch_threshold"]
    pub punch_threshold: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_clip_mode"]
    pub punch_clip_mode: EnumParam<ClipMode>,
    #[cfg(feature = "punch")]
    #[id = "punch_softness"]
    pub punch_softness: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_oversampling"]
    pub punch_oversampling: EnumParam<OversamplingFactor>,
    // Transient shaper section
    #[cfg(feature = "punch")]
    #[id = "punch_attack"]
    pub punch_attack: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_sustain"]
    pub punch_sustain: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_attack_time"]
    pub punch_attack_time: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_release_time"]
    pub punch_release_time: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_sensitivity"]
    pub punch_sensitivity: FloatParam,
    // Global controls
    #[cfg(feature = "punch")]
    #[id = "punch_input_gain"]
    pub punch_input_gain: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_output_gain"]
    pub punch_output_gain: FloatParam,
    #[cfg(feature = "punch")]
    #[id = "punch_mix"]
    pub punch_mix: FloatParam,

    /// Wet-path HPF cutoff (Hz). Applies only to the clipped/shaped signal,
    /// not the dry, so parallel drum blends add attack/punch without muddying
    /// the low end. 20 Hz = effectively off; 120–400 Hz suits drum submix.
    #[id = "punch_wet_hpf"]
    pub punch_wet_hpf_hz: FloatParam,
}

impl Default for PunchParams {
    fn default() -> Self {
        Self {
            // Punch Module Parameters (Clipper + Transient Shaper)
            // Default: BYPASSED - user must enable intentionally
            #[cfg(feature = "punch")]
            punch_bypass: BoolParam::new("Punch Bypass", true),

            #[cfg(feature = "punch")]
            punch_threshold: FloatParam::new(
                "Clip Threshold",
                -0.1, // -0.1dB default (gentle, near 0dB ceiling)
                FloatRange::Linear {
                    min: -12.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            #[cfg(feature = "punch")]
            punch_clip_mode: EnumParam::new("Clip Mode", ClipMode::Soft),

            #[cfg(feature = "punch")]
            punch_softness: FloatParam::new(
                "Softness",
                0.3, // Gentle soft clip knee by default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "punch")]
            punch_oversampling: EnumParam::new("Oversampling", OversamplingFactor::X8),

            #[cfg(feature = "punch")]
            punch_attack: FloatParam::new(
                "Attack",
                0.0, // Neutral by default - user adds punch as needed
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "punch")]
            punch_sustain: FloatParam::new(
                "Sustain",
                0.0, // Neutral sustain
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "punch")]
            punch_attack_time: FloatParam::new(
                "Attack Time",
                5.0, // 5ms default
                FloatRange::Skewed {
                    min: 0.1,
                    max: 30.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(0.1),

            #[cfg(feature = "punch")]
            punch_release_time: FloatParam::new(
                "Release Time",
                100.0, // 100ms default
                FloatRange::Skewed {
                    min: 10.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0),

            #[cfg(feature = "punch")]
            punch_sensitivity: FloatParam::new(
                "Sensitivity",
                0.5, // 50% default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "punch")]
            punch_input_gain: FloatParam::new(
                "Punch Input",
                0.0, // 0dB
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            #[cfg(feature = "punch")]
            punch_output_gain: FloatParam::new(
                "Punch Output",
                0.0, // 0dB
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            #[cfg(feature = "punch")]
            punch_mix: FloatParam::new(
                "Punch Mix",
                1.0, // Fully wet
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            // Not feature-gated: this HPF applies to the wet path regardless
            // of whether the "punch" DSP feature is compiled in, matching the
            // struct field above (fixes a pre-existing --no-default-features
            // build break where this cfg attribute didn't match the field's).
            punch_wet_hpf_hz: FloatParam::new(
                "Punch Wet HPF",
                20.0, // Off by default — full-range parallel
                FloatRange::Skewed {
                    min: 20.0,
                    max: 1000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
        }
    }
}
