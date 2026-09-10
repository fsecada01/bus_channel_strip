use nice_plug::prelude::*;

#[derive(Params)]
pub struct PultecParams {
    // Pultec EQ Parameters
    #[id = "pultec_bypass"]
    pub pultec_bypass: BoolParam,
    #[id = "pultec_lf_boost_freq"]
    pub pultec_lf_boost_freq: FloatParam,
    #[id = "pultec_lf_boost_gain"]
    pub pultec_lf_boost_gain: FloatParam,
    #[id = "pultec_lf_bw"]
    pub pultec_lf_boost_bandwidth: FloatParam,
    #[id = "pultec_lf_cut_freq"]
    pub pultec_lf_cut_freq: FloatParam,
    #[id = "pultec_lf_cut_gain"]
    pub pultec_lf_cut_gain: FloatParam,
    #[id = "pultec_lf_cut_bw"]
    pub pultec_lf_cut_bandwidth: FloatParam,
    #[id = "pultec_hf_boost_freq"]
    pub pultec_hf_boost_freq: FloatParam,
    #[id = "pultec_hf_boost_gain"]
    pub pultec_hf_boost_gain: FloatParam,
    #[id = "pultec_hf_boost_bandwidth"]
    pub pultec_hf_boost_bandwidth: FloatParam,
    #[id = "pultec_hf_cut_freq"]
    pub pultec_hf_cut_freq: FloatParam,
    #[id = "pultec_hf_cut_gain"]
    pub pultec_hf_cut_gain: FloatParam,
    #[id = "pultec_tube_drive"]
    pub pultec_tube_drive: FloatParam,
    /// Linear-phase Pultec (v2.0, #15). Off by default so v1.0 sessions keep
    /// their zero-latency minimum-phase behaviour on load.
    #[id = "pultec_linear_phase"]
    pub pultec_linear_phase: BoolParam,
}

impl Default for PultecParams {
    fn default() -> Self {
        Self {
            // Pultec EQ Parameters
            pultec_bypass: BoolParam::new("Pultec Bypass", true),

            pultec_lf_boost_freq: FloatParam::new(
                "LF Boost Freq",
                60.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 300.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            // Extended to ±18 dB to match professional hardware headroom.
            pultec_lf_boost_gain: FloatParam::new(
                "LF Boost",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 18.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            // BW=0 → Q=1.0 (tight/modern), BW=1 → Q=0.25 (very wide/vintage).
            // Default 0.67 reproduces the current warm-sounding Q=0.5 shelf.
            pultec_lf_boost_bandwidth: FloatParam::new(
                "LF Boost BW",
                0.67,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Independent low-cut frequency enables the classic Pultec
            // "trick": boost at e.g. 60 Hz, cut at e.g. 200 Hz for a tight
            // low end. Extended to 400 Hz so users can target guitar mud range.
            pultec_lf_cut_freq: FloatParam::new(
                "LF Atten Freq",
                100.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            pultec_lf_cut_gain: FloatParam::new(
                "LF Atten",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 18.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            pultec_lf_cut_bandwidth: FloatParam::new(
                "LF Atten BW",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            pultec_hf_boost_freq: FloatParam::new(
                "HF Boost Freq",
                10000.0,
                FloatRange::Skewed {
                    min: 5000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            pultec_hf_boost_gain: FloatParam::new(
                "HF Boost",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            pultec_hf_boost_bandwidth: FloatParam::new(
                "HF Bandwidth",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            pultec_hf_cut_freq: FloatParam::new(
                "HF Atten Freq",
                10000.0,
                FloatRange::Skewed {
                    min: 5000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            pultec_hf_cut_gain: FloatParam::new(
                "HF Atten",
                0.0,
                FloatRange::Linear { min: 0.0, max: 8.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            pultec_tube_drive: FloatParam::new(
                "Tube Drive",
                0.2, // Subtle tube character by default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            pultec_linear_phase: BoolParam::new("Linear Phase", false),
        }
    }
}
