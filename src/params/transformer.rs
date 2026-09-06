use nice_plug::prelude::*;

use crate::transformer::TransformerModel;

#[derive(Params)]
pub struct TransformerParams {
    // Transformer Module Parameters
    #[id = "transformer_bypass"]
    pub transformer_bypass: BoolParam,
    #[id = "transformer_model"]
    pub transformer_model: EnumParam<TransformerModel>,
    #[id = "transformer_input_drive"]
    pub transformer_input_drive: FloatParam,
    #[id = "transformer_input_saturation"]
    pub transformer_input_saturation: FloatParam,
    #[id = "transformer_output_drive"]
    pub transformer_output_drive: FloatParam,
    #[id = "transformer_output_saturation"]
    pub transformer_output_saturation: FloatParam,
    #[id = "transformer_low_response"]
    pub transformer_low_response: FloatParam,
    #[id = "transformer_high_response"]
    pub transformer_high_response: FloatParam,
    #[id = "transformer_compression"]
    pub transformer_compression: FloatParam,
    /// #16: true restores bit-identical v1.0 saturation (no hysteresis).
    #[id = "transformer_hysteresis_bypass"]
    pub transformer_hysteresis_bypass: BoolParam,
}

impl Default for TransformerParams {
    fn default() -> Self {
        Self {
            // Transformer Module Parameters
            transformer_bypass: BoolParam::new("Transformer Bypass", true),

            transformer_model: EnumParam::new("Transformer Model", TransformerModel::Vintage),

            transformer_input_drive: FloatParam::new(
                "Input Drive",
                0.2, // Subtle drive by default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_input_saturation: FloatParam::new(
                "Input Saturation",
                0.3, // Gentle saturation
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_output_drive: FloatParam::new(
                "Output Drive",
                0.1, // Very subtle by default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_output_saturation: FloatParam::new(
                "Output Saturation",
                0.4, // Moderate output coloration
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_low_response: FloatParam::new(
                "Low Response",
                0.0, // Flat by default
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_high_response: FloatParam::new(
                "High Response",
                0.0, // Flat by default
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_compression: FloatParam::new(
                "Transformer Compression",
                0.3, // Gentle transformer loading
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            transformer_hysteresis_bypass: BoolParam::new("Transformer Hysteresis Bypass", false),
        }
    }
}
