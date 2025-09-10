use nih_plug::prelude::*;
#[cfg(feature = "gui")]
use vizia_plug::ViziaState;
use std::sync::Arc;
mod shaping;
mod spectral;

#[cfg(feature = "api5500")]
mod api5500;
#[cfg(feature = "api5500")]
use api5500::Api5500;

#[cfg(feature = "buttercomp2")]
mod buttercomp2;
#[cfg(feature = "buttercomp2")]
use buttercomp2::ButterComp2;

#[cfg(feature = "pultec")]
mod pultec;
#[cfg(feature = "pultec")]
use pultec::PultecEQ;

#[cfg(feature = "dynamic_eq")]
mod dynamic_eq;
#[cfg(feature = "dynamic_eq")]
use dynamic_eq::{DynamicEQ, DynamicBandParams, DynamicMode};

#[cfg(feature = "transformer")]
mod transformer;
#[cfg(feature = "transformer")]
use transformer::{TransformerModule, TransformerModel};

#[cfg(feature = "gui")]
mod editor;
#[cfg(feature = "gui")]
mod components;
#[cfg(feature = "gui")]
mod styles;

/// Module identifiers for reordering
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum ModuleType {
    #[name = "API5500 EQ"]
    Api5500EQ,
    #[name = "ButterComp2"]
    ButterComp2,
    #[name = "Pultec EQ"]
    PultecEQ,
    #[name = "Dynamic EQ"]
    DynamicEQ,
    #[name = "Transformer"]
    Transformer,
}

impl Default for ModuleType {
    fn default() -> Self {
        Self::Api5500EQ
    }
}

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct BusChannelStrip {
    params: Arc<BusChannelStripParams>,
    /// API 5500–style input EQ module
    #[cfg(feature = "api5500")]
    eq_api5500: Api5500,
    /// ButterComp2 compressor module
    #[cfg(feature = "buttercomp2")]
    compressor: ButterComp2,
    /// Pultec-style EQ module
    #[cfg(feature = "pultec")]
    pultec: PultecEQ,
    /// Dynamic EQ module
    #[cfg(feature = "dynamic_eq")]
    dynamic_eq: DynamicEQ,
    /// Transformer coloration module
    #[cfg(feature = "transformer")]
    transformer: TransformerModule,
    
    /// Buffers for module reordering
    temp_buffer_1: Vec<Vec<f32>>,
    temp_buffer_2: Vec<Vec<f32>>,
    
    /// GUI state  
    #[cfg(feature = "gui")]
    editor_state: Arc<ViziaState>,
}

#[derive(Params)]
pub struct BusChannelStripParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

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

    // ButterComp2 Compressor Parameters
    #[id = "comp_bypass"]
    pub comp_bypass: BoolParam,
    #[id = "comp_compress"]
    pub comp_compress: FloatParam,
    #[id = "comp_output"]
    pub comp_output: FloatParam,
    #[id = "comp_dry_wet"]
    pub comp_dry_wet: FloatParam,

    // Pultec EQ Parameters
    #[id = "pultec_bypass"]
    pub pultec_bypass: BoolParam,
    #[id = "pultec_lf_boost_freq"]
    pub pultec_lf_boost_freq: FloatParam,
    #[id = "pultec_lf_boost_gain"]
    pub pultec_lf_boost_gain: FloatParam,
    #[id = "pultec_lf_cut_gain"]
    pub pultec_lf_cut_gain: FloatParam,
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

    #[cfg(feature = "dynamic_eq")]
    // Dynamic EQ Parameters
    #[id = "dyneq_bypass"]
    pub dyneq_bypass: BoolParam,

    #[cfg(feature = "dynamic_eq")]
    // Band 1 (Low) - 200Hz default
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
    // Band 2 (Low-Mid) - 800Hz default
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
    // Band 3 (High-Mid) - 3kHz default
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
    // Band 4 (High) - 8kHz default
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

    // Module Ordering Parameters
    #[id = "module_order_1"]
    pub module_order_1: EnumParam<ModuleType>,
    #[id = "module_order_2"]
    pub module_order_2: EnumParam<ModuleType>,
    #[id = "module_order_3"]
    pub module_order_3: EnumParam<ModuleType>,
    #[id = "module_order_4"]
    pub module_order_4: EnumParam<ModuleType>,
    #[id = "module_order_5"]
    pub module_order_5: EnumParam<ModuleType>,
}

impl Default for BusChannelStrip {
    fn default() -> Self {
        Self {
            params: Arc::new(BusChannelStripParams::default()),
            #[cfg(feature = "api5500")]
            eq_api5500: Api5500::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "buttercomp2")]
            compressor: ButterComp2::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "pultec")]
            pultec: PultecEQ::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "dynamic_eq")]
            dynamic_eq: DynamicEQ::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "transformer")]
            transformer: TransformerModule::new(44100.0), // default sample rate; will be overwritten in initialize()
            temp_buffer_1: Vec::new(),
            temp_buffer_2: Vec::new(),
            #[cfg(feature = "gui")]
            editor_state: editor::default_state(),
        }
    }
}

impl Default for BusChannelStripParams {
    fn default() -> Self {
        Self {
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

            // API5500 EQ Parameters
            eq_bypass: BoolParam::new("EQ Bypass", false),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            lf_gain: FloatParam::new(
                "LF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            lmf_gain: FloatParam::new(
                "LMF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            mf_gain: FloatParam::new(
                "MF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            hmf_gain: FloatParam::new(
                "HMF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            hf_gain: FloatParam::new(
                "HF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            // ButterComp2 Compressor Parameters
            comp_bypass: BoolParam::new("Comp Bypass", false),
            
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

            // Pultec EQ Parameters
            pultec_bypass: BoolParam::new("Pultec Bypass", false),
            
            pultec_lf_boost_freq: FloatParam::new(
                "LF Boost Freq",
                60.0,
                FloatRange::Linear { min: 20.0, max: 100.0 },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            pultec_lf_boost_gain: FloatParam::new(
                "LF Boost",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),
            
            pultec_lf_cut_gain: FloatParam::new(
                "LF Atten", // "Attenuation" like the original
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),
            
            pultec_hf_boost_freq: FloatParam::new(
                "HF Boost Freq",
                10000.0,
                FloatRange::Skewed {
                    min: 5000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            pultec_hf_boost_gain: FloatParam::new(
                "HF Boost",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            
            pultec_hf_cut_gain: FloatParam::new(
                "HF Atten",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),
            
            pultec_tube_drive: FloatParam::new(
                "Tube Drive",
                0.2, // Subtle tube character by default
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            // Dynamic EQ Parameters
            dyneq_bypass: BoolParam::new("DynEQ Bypass", false),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_threshold: FloatParam::new(
                "DynEQ 1 Thresh",
                0.7, // -3dB
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_ratio: FloatParam::new(
                "DynEQ 1 Ratio",
                0.5, // 3:1 ratio
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_attack: FloatParam::new(
                "DynEQ 1 Attack",
                0.3, // ~10ms
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_release: FloatParam::new(
                "DynEQ 1 Release",
                0.4, // ~100ms
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_gain: FloatParam::new(
                "DynEQ 1 Gain",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_q: FloatParam::new(
                "DynEQ 1 Q",
                0.5, // Q = 1.0
                FloatRange::Linear { min: 0.1, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_enabled: BoolParam::new("DynEQ 1 On", false),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band1_mode: EnumParam::new("DynEQ 1 Mode", DynamicMode::CompressDownward),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_threshold: FloatParam::new("DynEQ 2 Thresh", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_ratio: FloatParam::new("DynEQ 2 Ratio", 0.5, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_attack: FloatParam::new("DynEQ 2 Attack", 0.3, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-2.0) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_release: FloatParam::new("DynEQ 2 Release", 0.4, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_gain: FloatParam::new("DynEQ 2 Gain", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_q: FloatParam::new("DynEQ 2 Q", 0.5, FloatRange::Linear { min: 0.1, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_enabled: BoolParam::new("DynEQ 2 On", false),
            
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band2_mode: EnumParam::new("DynEQ 2 Mode", DynamicMode::CompressDownward),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_threshold: FloatParam::new("DynEQ 3 Thresh", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_ratio: FloatParam::new("DynEQ 3 Ratio", 0.5, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_attack: FloatParam::new("DynEQ 3 Attack", 0.2, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-2.0) }).with_step_size(0.01), // Faster for highs
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_release: FloatParam::new("DynEQ 3 Release", 0.3, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01), // Faster for highs
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_gain: FloatParam::new("DynEQ 3 Gain", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_q: FloatParam::new("DynEQ 3 Q", 0.5, FloatRange::Linear { min: 0.1, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_enabled: BoolParam::new("DynEQ 3 On", false),
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band3_mode: EnumParam::new("DynEQ 3 Mode", DynamicMode::CompressDownward),

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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_threshold: FloatParam::new("DynEQ 4 Thresh", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_ratio: FloatParam::new("DynEQ 4 Ratio", 0.5, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_attack: FloatParam::new("DynEQ 4 Attack", 0.1, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-2.0) }).with_step_size(0.01), // Very fast for highs
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_release: FloatParam::new("DynEQ 4 Release", 0.2, FloatRange::Skewed { min: 0.0, max: 1.0, factor: FloatRange::skew_factor(-1.0) }).with_step_size(0.01), // Fast for highs
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_gain: FloatParam::new("DynEQ 4 Gain", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_q: FloatParam::new("DynEQ 4 Q", 0.5, FloatRange::Linear { min: 0.1, max: 1.0 }).with_step_size(0.01),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_enabled: BoolParam::new("DynEQ 4 On", false),
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_mode: EnumParam::new("DynEQ 4 Mode", DynamicMode::CompressDownward),
            
            

            // Transformer Module Parameters
            transformer_bypass: BoolParam::new("Transformer Bypass", false),
            
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
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),
            
            transformer_high_response: FloatParam::new(
                "High Response",
                0.0, // Flat by default
                FloatRange::Linear { min: -1.0, max: 1.0 },
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

            // Module Ordering Parameters (default signal chain)
            module_order_1: EnumParam::new("Module Order 1", ModuleType::Api5500EQ),
            module_order_2: EnumParam::new("Module Order 2", ModuleType::ButterComp2),
            module_order_3: EnumParam::new("Module Order 3", ModuleType::PultecEQ),
            module_order_4: EnumParam::new("Module Order 4", ModuleType::DynamicEQ),
            module_order_5: EnumParam::new("Module Order 5", ModuleType::Transformer),
        }
    }
}

impl Plugin for BusChannelStrip {
    const NAME: &'static str = "Bus Channel Strip";
    const VENDOR: &'static str = "Francis Secada";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "your@email.com";

    // Version with build date suffix provided by build.rs
    const VERSION: &'static str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("BUILD_DATE"));

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default, these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    #[cfg(feature = "gui")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Create the vizia GUI editor
        editor::create(
            self.params.clone(),
            self.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Reinitialize the API5500 EQ with real sample rate once context is available.
        // TODO: query actual sample rate from _context or BufferConfig
        // Reinitialize modules with the actual sample rate
        let sr = _buffer_config.sample_rate;
        #[cfg(feature = "api5500")]
        { self.eq_api5500 = Api5500::new(sr); }
        #[cfg(feature = "buttercomp2")]
        { self.compressor = ButterComp2::new(sr); }
        #[cfg(feature = "pultec")]
        { self.pultec = PultecEQ::new(sr); }
        #[cfg(feature = "dynamic_eq")]
        { self.dynamic_eq = DynamicEQ::new(sr); }
        #[cfg(feature = "transformer")]
        { self.transformer = TransformerModule::new(sr); }
        
        // Initialize temporary buffers for module reordering
        let max_buffer_size = _buffer_config.max_buffer_size as usize;
        let num_channels = _audio_io_layout.main_output_channels.unwrap().get() as usize;
        
        self.temp_buffer_1 = vec![vec![0.0; max_buffer_size]; num_channels];
        self.temp_buffer_2 = vec![vec![0.0; max_buffer_size]; num_channels];
        
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
        #[cfg(feature = "buttercomp2")]
        { self.compressor.reset(); }
        #[cfg(feature = "dynamic_eq")]
        { self.dynamic_eq.reset(); }
        #[cfg(feature = "transformer")]
        { self.transformer.reset(); }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Process enabled modules based on feature flags
        
        // 1) API5500 EQ Module
        #[cfg(feature = "api5500")]
        {
            self.eq_api5500.update_parameters(
                self.params.lf_freq.value(),
                self.params.lf_gain.value(),
                self.params.lmf_freq.value(),
                self.params.lmf_gain.value(),
                self.params.lmf_q.value(),
                self.params.mf_freq.value(),
                self.params.mf_gain.value(),
                self.params.mf_q.value(),
                self.params.hmf_freq.value(),
                self.params.hmf_gain.value(),
                self.params.hmf_q.value(),
                self.params.hf_freq.value(),
                self.params.hf_gain.value(),
            );

            if !self.params.eq_bypass.value() {
                self.eq_api5500.process(buffer);
            }
        }

        // 2) ButterComp2 Compressor Module
        #[cfg(feature = "buttercomp2")]
        {
            self.compressor.update_parameters(
                self.params.comp_compress.value(),
                self.params.comp_output.value(),
                self.params.comp_dry_wet.value(),
            );
            
            if !self.params.comp_bypass.value() {
                self.compressor.process(buffer);
            }
        }

        // 3) Pultec EQ Module
        #[cfg(feature = "pultec")]
        {
            self.pultec.update_parameters(
                self.params.pultec_lf_boost_freq.value(),
                self.params.pultec_lf_boost_gain.value(),
                self.params.pultec_lf_cut_gain.value(),
                self.params.pultec_hf_boost_freq.value(),
                self.params.pultec_hf_boost_gain.value(),
                self.params.pultec_hf_boost_bandwidth.value(),
                self.params.pultec_hf_cut_freq.value(),
                self.params.pultec_hf_cut_gain.value(),
                self.params.pultec_tube_drive.value(),
            );
            
            if !self.params.pultec_bypass.value() {
                self.pultec.process(buffer);
            }
        }

        // 4) Transformer Module  
        #[cfg(feature = "transformer")]
        {
            self.transformer.update_parameters(
                self.params.transformer_model.value(),
                self.params.transformer_input_drive.value(),
                self.params.transformer_input_saturation.value(),
                self.params.transformer_output_drive.value(),
                self.params.transformer_output_saturation.value(),
                self.params.transformer_low_response.value(),
                self.params.transformer_high_response.value(),
                self.params.transformer_compression.value(),
            );
            
            if !self.params.transformer_bypass.value() {
                self.transformer.process(buffer);
            }
        }

        // 5) Dynamic EQ Module (disabled for now)
        #[cfg(feature = "dynamic_eq")]
        {
            let dyneq_params = [
                DynamicBandParams {
                    mode: self.params.dyneq_band1_mode.value(),
                    detector_freq: self.params.dyneq_band1_detector_freq.value(),
                    freq: self.params.dyneq_band1_freq.value(),
                    q: map_q_param(self.params.dyneq_band1_q.value()),
                    threshold_db: self.params.dyneq_band1_threshold.value() * 24.0 - 24.0, // Map 0-1 to -24 to 0 dB
                    ratio: map_ratio_param(self.params.dyneq_band1_ratio.value()),
                    attack_ms: map_time_param(self.params.dyneq_band1_attack.value(), 1.0, 100.0),
                    release_ms: map_time_param(self.params.dyneq_band1_release.value(), 10.0, 1000.0),
                    gain_db: self.params.dyneq_band1_gain.value() * 12.0, // ±12dB
                    enabled: self.params.dyneq_band1_enabled.value(),
                },
                DynamicBandParams {
                    mode: self.params.dyneq_band2_mode.value(),
                    detector_freq: self.params.dyneq_band2_detector_freq.value(),
                    freq: self.params.dyneq_band2_freq.value(),
                    q: map_q_param(self.params.dyneq_band2_q.value()),
                    threshold_db: self.params.dyneq_band2_threshold.value() * 24.0 - 24.0,
                    ratio: map_ratio_param(self.params.dyneq_band2_ratio.value()),
                    attack_ms: map_time_param(self.params.dyneq_band2_attack.value(), 1.0, 100.0),
                    release_ms: map_time_param(self.params.dyneq_band2_release.value(), 10.0, 1000.0),
                    gain_db: self.params.dyneq_band2_gain.value() * 12.0,
                    enabled: self.params.dyneq_band2_enabled.value(),
                },
                DynamicBandParams {
                    mode: self.params.dyneq_band3_mode.value(),
                    detector_freq: self.params.dyneq_band3_detector_freq.value(),
                    freq: self.params.dyneq_band3_freq.value(),
                    q: map_q_param(self.params.dyneq_band3_q.value()),
                    threshold_db: self.params.dyneq_band3_threshold.value() * 24.0 - 24.0,
                    ratio: map_ratio_param(self.params.dyneq_band3_ratio.value()),
                    attack_ms: map_time_param(self.params.dyneq_band3_attack.value(), 0.5, 50.0),
                    release_ms: map_time_param(self.params.dyneq_band3_release.value(), 5.0, 500.0),
                    gain_db: self.params.dyneq_band3_gain.value() * 12.0,
                    enabled: self.params.dyneq_band3_enabled.value(),
                },
                DynamicBandParams {
                    mode: self.params.dyneq_band4_mode.value(),
                    detector_freq: self.params.dyneq_band4_detector_freq.value(),
                    freq: self.params.dyneq_band4_freq.value(),
                    q: map_q_param(self.params.dyneq_band4_q.value()),
                    threshold_db: self.params.dyneq_band4_threshold.value() * 24.0 - 24.0,
                    ratio: map_ratio_param(self.params.dyneq_band4_ratio.value()),
                    attack_ms: map_time_param(self.params.dyneq_band4_attack.value(), 0.1, 20.0),
                    release_ms: map_time_param(self.params.dyneq_band4_release.value(), 1.0, 200.0),
                    gain_db: self.params.dyneq_band4_gain.value() * 12.0,
                    enabled: self.params.dyneq_band4_enabled.value(),
                },
            ];
            self.dynamic_eq.update_parameters(&dyneq_params);
            
            if !self.params.dyneq_bypass.value() {
                self.dynamic_eq.process(buffer);
            }
        }

        // 6) Output gain
        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
    
    
    
    
}

impl ClapPlugin for BusChannelStrip {
    const CLAP_ID: &'static str = "com.your-domain.your-plugin-name (use underscores)";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for BusChannelStrip {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(BusChannelStrip);
nih_export_vst3!(BusChannelStrip);

// Helper functions for parameter mapping

/// Map 0-1 ratio parameter to 1:1 to 10:1 compression ratio
fn map_ratio_param(param: f32) -> f32 {
    1.0 + param * 9.0  // 1.0 to 10.0
}

/// Map 0-1 time parameter to milliseconds with min/max range
fn map_time_param(param: f32, min_ms: f32, max_ms: f32) -> f32 {
    min_ms + param * (max_ms - min_ms)
}

/// Map 0-1 Q parameter to 0.5 to 4.0 Q range
fn map_q_param(param: f32) -> f32 {
    0.5 + param * 3.5  // 0.5 to 4.0
}
