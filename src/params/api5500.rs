use nice_plug::prelude::*;

#[derive(Params)]
pub struct Api5500Params {
    // API5500 EQ Parameters
    #[id = "eq_bypass"]
    pub eq_bypass: BoolParam,

    // Low Frequency (LF) - Shelving
    #[id = "lf_freq"]
    pub lf_freq: FloatParam,
    #[id = "lf_gain"]
    pub lf_gain: FloatParam,

    // Low Mid Frequency (LMF) - Parametric
    #[id = "lmf_freq"]
    pub lmf_freq: FloatParam,
    #[id = "lmf_gain"]
    pub lmf_gain: FloatParam,
    #[id = "lmf_q"]
    pub lmf_q: FloatParam,

    // Mid Frequency (MF) - Parametric
    #[id = "mf_freq"]
    pub mf_freq: FloatParam,
    #[id = "mf_gain"]
    pub mf_gain: FloatParam,
    #[id = "mf_q"]
    pub mf_q: FloatParam,

    // High Mid Frequency (HMF) - Parametric
    #[id = "hmf_freq"]
    pub hmf_freq: FloatParam,
    #[id = "hmf_gain"]
    pub hmf_gain: FloatParam,
    #[id = "hmf_q"]
    pub hmf_q: FloatParam,

    // High Frequency (HF) - Shelving
    #[id = "hf_freq"]
    pub hf_freq: FloatParam,
    #[id = "hf_gain"]
    pub hf_gain: FloatParam,
}

impl Default for Api5500Params {
    fn default() -> Self {
        Self {
            // API5500 EQ Parameters
            eq_bypass: BoolParam::new("EQ Bypass", true),

            // Low Frequency (LF) - Shelving at 100Hz
            lf_freq: FloatParam::new(
                "LF Freq",
                100.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            lf_gain: FloatParam::new(
                "LF Gain",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            // Low Mid-Frequency (LMF) - Parametric at 200Hz
            lmf_freq: FloatParam::new(
                "LMF Freq",
                200.0,
                FloatRange::Skewed {
                    min: 50.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            lmf_gain: FloatParam::new(
                "LMF Gain",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            lmf_q: FloatParam::new(
                "LMF Q",
                0.7,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01),

            // Mid Frequency (MF) - Parametric at 1kHz
            mf_freq: FloatParam::new(
                "MF Freq",
                1000.0,
                FloatRange::Skewed {
                    min: 200.0,
                    max: 8000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            mf_gain: FloatParam::new(
                "MF Gain",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            mf_q: FloatParam::new(
                "MF Q",
                0.7,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01),

            // High Mid-Frequency (HMF) - Parametric at 3kHz
            hmf_freq: FloatParam::new(
                "HMF Freq",
                3000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 15000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            hmf_gain: FloatParam::new(
                "HMF Gain",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

            hmf_q: FloatParam::new(
                "HMF Q",
                0.7,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01),

            // High Frequency (HF) - Shelving at 10kHz
            hf_freq: FloatParam::new(
                "HF Freq",
                10000.0,
                FloatRange::Skewed {
                    min: 3000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            hf_gain: FloatParam::new(
                "HF Gain",
                0.0,
                FloatRange::Linear {
                    min: -15.0,
                    max: 15.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
        }
    }
}
