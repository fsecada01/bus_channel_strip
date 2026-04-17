use nih_plug::prelude::*;
use std::sync::Arc;
#[cfg(feature = "gui")]
use vizia_plug::ViziaState;
mod shaping;
mod spectral;

#[cfg(feature = "api5500")]
mod api5500;
#[cfg(feature = "api5500")]
use api5500::Api5500;

#[cfg(feature = "buttercomp2")]
mod buttercomp2;
#[cfg(feature = "buttercomp2")]
use buttercomp2::{
    ButterComp2, ButterComp2Model, FetCompressor, FetRatio, OpticalCompressor, VcaCompressor,
};

#[cfg(feature = "pultec")]
mod pultec;
#[cfg(feature = "pultec")]
use pultec::PultecEQ;

#[cfg(feature = "dynamic_eq")]
mod dynamic_eq;
#[cfg(feature = "dynamic_eq")]
use dynamic_eq::{DynamicBandParams, DynamicEQ, DynamicMode};

#[cfg(feature = "transformer")]
mod transformer;
#[cfg(feature = "transformer")]
use transformer::{TransformerModel, TransformerModule};

#[cfg(feature = "punch")]
mod punch;
#[cfg(feature = "punch")]
use punch::{ClipMode, OversamplingFactor, PunchModule};

#[cfg(feature = "gui")]
mod components;
#[cfg(feature = "gui")]
mod editor;
#[cfg(feature = "gui")]
mod styles;

/// Compute RMS across all channels from a slice-of-slices buffer view.
/// Allocation-free; safe to call on the audio thread.
fn rms_linear(channels: &[&mut [f32]]) -> f32 {
    let mut sum_sq = 0.0_f32;
    let mut n = 0_u32;
    for ch in channels.iter() {
        for &s in ch.iter() {
            sum_sq += s * s;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum_sq / n as f32).sqrt()
    }
}

/// Smoothing coefficient for auto-gain: ~5-second time constant at 86 buffers/sec.
const AUTO_GAIN_SMOOTH: f32 = 0.9975;
/// Maximum auto-gain correction: ±18 dB in linear.
const AUTO_GAIN_MAX: f32 = 8.0; // +18.06 dB
const AUTO_GAIN_MIN: f32 = 0.125; // −18.06 dB

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
    #[name = "Punch"]
    Punch,
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
    /// 1176-style FET compressor — pure Rust, no FFI
    #[cfg(feature = "buttercomp2")]
    fet_compressor: FetCompressor,
    /// VCA bus compressor — SSL G-Bus style, pure Rust, no FFI
    #[cfg(feature = "buttercomp2")]
    vca_compressor: VcaCompressor,
    /// Optical compressor — LA-2A style, pure Rust, no FFI
    #[cfg(feature = "buttercomp2")]
    optical_compressor: OpticalCompressor,
    /// Pultec-style EQ module
    #[cfg(feature = "pultec")]
    pultec: PultecEQ,
    /// Dynamic EQ module
    #[cfg(feature = "dynamic_eq")]
    dynamic_eq: DynamicEQ,
    /// Transformer coloration module
    #[cfg(feature = "transformer")]
    transformer: TransformerModule,
    /// Punch module (Clipper + Transient Shaper)
    #[cfg(feature = "punch")]
    punch: PunchModule,

    /// Buffers for module reordering
    temp_buffer_1: Vec<Vec<f32>>,
    temp_buffer_2: Vec<Vec<f32>>,

    /// Spectrum data shared lock-free with the GUI thread.
    spectrum_data: Arc<spectral::SpectrumData>,

    /// Pre-allocated FFT ring buffer — no audio-thread allocation.
    #[cfg(feature = "dynamic_eq")]
    fft_ring: Vec<f32>,
    #[cfg(feature = "dynamic_eq")]
    fft_ring_pos: usize,
    #[cfg(feature = "dynamic_eq")]
    fft_engine: Option<Arc<dyn realfft::RealToComplex<f32>>>,
    #[cfg(feature = "dynamic_eq")]
    fft_input: Vec<f32>,
    #[cfg(feature = "dynamic_eq")]
    fft_output: Vec<realfft::num_complex::Complex<f32>>,
    #[cfg(feature = "dynamic_eq")]
    fft_scratch: Vec<realfft::num_complex::Complex<f32>>,
    #[cfg(feature = "dynamic_eq")]
    fft_window: Vec<f32>,
    #[cfg(feature = "dynamic_eq")]
    fft_magnitude_smooth: Vec<f32>,

    // ── Sidechain masking analysis (Strategy A — one-shot, UI-triggered) ──────
    /// Circular ring buffer for the sidechain mono mix-down.
    #[cfg(feature = "dynamic_eq")]
    sc_ring: Vec<f32>,
    #[cfg(feature = "dynamic_eq")]
    sc_ring_pos: usize,
    /// Windowed sidechain snapshot for FFT (pre-allocated in initialize()).
    #[cfg(feature = "dynamic_eq")]
    sc_fft_input: Vec<f32>,
    /// Sidechain FFT output (pre-allocated, same size as fft_output).
    #[cfg(feature = "dynamic_eq")]
    sc_fft_output: Vec<realfft::num_complex::Complex<f32>>,
    /// Sample rate cached from initialize() for FFT bin → Hz conversion.
    #[cfg(feature = "dynamic_eq")]
    sample_rate: f32,
    /// GUI → audio: GUI sets true to request an analysis on the next FFT frame.
    analysis_requested: Arc<std::sync::atomic::AtomicBool>,
    /// audio → GUI: results of the last masking analysis.
    analysis_result: Arc<spectral::AnalysisResult>,
    /// audio → GUI: per-band gain reduction for the DynEQ spectrum display.
    gr_data: Arc<spectral::GainReductionData>,

    /// Smoothed auto-gain correction factor (linear, 1.0 = unity).
    /// Updated per buffer; reset to 1.0 when auto-gain is disabled.
    auto_gain_correction: f32,

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
    /// Global bypass — passes audio through without touching any module.
    #[id = "global_bypass"]
    pub global_bypass: BoolParam,

    /// Global auto-gain — compensates for loudness changes introduced by the chain.
    #[id = "global_auto_gain"]
    pub global_auto_gain: BoolParam,

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

    // Pultec EQ Parameters
    #[id = "pultec_bypass"]
    pub pultec_bypass: BoolParam,
    #[id = "pultec_lf_boost_freq"]
    pub pultec_lf_boost_freq: FloatParam,
    #[id = "pultec_lf_boost_gain"]
    pub pultec_lf_boost_gain: FloatParam,
    #[id = "pultec_lf_cut_freq"]
    pub pultec_lf_cut_freq: FloatParam,
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
    #[id = "dyneq_band1_solo"]
    pub dyneq_band1_solo: BoolParam,

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
    #[id = "dyneq_band2_solo"]
    pub dyneq_band2_solo: BoolParam,

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
    #[id = "dyneq_band3_solo"]
    pub dyneq_band3_solo: BoolParam,

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
    #[cfg(feature = "dynamic_eq")]
    #[id = "dyneq_band4_solo"]
    pub dyneq_band4_solo: BoolParam,

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
    #[id = "module_order_6"]
    pub module_order_6: EnumParam<ModuleType>,
}

impl Default for BusChannelStrip {
    fn default() -> Self {
        Self {
            params: Arc::new(BusChannelStripParams::default()),
            #[cfg(feature = "api5500")]
            eq_api5500: Api5500::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "buttercomp2")]
            compressor: ButterComp2::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "buttercomp2")]
            fet_compressor: FetCompressor::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "buttercomp2")]
            vca_compressor: VcaCompressor::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "buttercomp2")]
            optical_compressor: OpticalCompressor::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "pultec")]
            pultec: PultecEQ::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "dynamic_eq")]
            dynamic_eq: DynamicEQ::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "transformer")]
            transformer: TransformerModule::new(44100.0), // default sample rate; will be overwritten in initialize()
            #[cfg(feature = "punch")]
            punch: PunchModule::new(44100.0), // default sample rate; will be overwritten in initialize()
            temp_buffer_1: Vec::new(),
            temp_buffer_2: Vec::new(),
            spectrum_data: Arc::new(spectral::SpectrumData::new()),
            #[cfg(feature = "dynamic_eq")]
            fft_ring: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            fft_ring_pos: 0,
            #[cfg(feature = "dynamic_eq")]
            fft_engine: None,
            #[cfg(feature = "dynamic_eq")]
            fft_input: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            fft_output: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            fft_scratch: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            fft_window: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            fft_magnitude_smooth: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            sc_ring: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            sc_ring_pos: 0,
            #[cfg(feature = "dynamic_eq")]
            sc_fft_input: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            sc_fft_output: Vec::new(),
            #[cfg(feature = "dynamic_eq")]
            sample_rate: 44100.0,
            analysis_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            analysis_result: Arc::new(spectral::AnalysisResult::new()),
            gr_data: Arc::new(spectral::GainReductionData::new()),
            auto_gain_correction: 1.0,
            #[cfg(feature = "gui")]
            editor_state: editor::default_state(),
        }
    }
}

impl Default for BusChannelStripParams {
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            lmf_gain: FloatParam::new(
                "LMF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            mf_gain: FloatParam::new(
                "MF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            hmf_gain: FloatParam::new(
                "HMF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            hf_gain: FloatParam::new(
                "HF Gain",
                0.0,
                FloatRange::Linear { min: -15.0, max: 15.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0)),

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
                FloatRange::Linear { min: -60.0, max: 0.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_ratio: FloatParam::new(
                "VCA Ratio",
                4.0,
                FloatRange::Linear { min: 1.0, max: 20.0 },
            )
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_atk: FloatParam::new(
                "VCA Attack",
                10.0,
                FloatRange::Linear { min: 0.1, max: 100.0 },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(5.0)),

            vca_rel: FloatParam::new(
                "VCA Release",
                100.0,
                FloatRange::Linear { min: 10.0, max: 1000.0 },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(5.0)),

            // Optical model parameters
            opt_thresh: FloatParam::new(
                "Opt Threshold",
                -12.0,
                FloatRange::Linear { min: -60.0, max: 0.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            opt_speed: FloatParam::new(
                "Opt Speed",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0)),

            opt_char: FloatParam::new(
                "Opt Character",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0)),

            // 1176-style FET compressor parameters
            #[cfg(feature = "buttercomp2")]
            fet_input_db: FloatParam::new(
                "FET Input",
                0.0,
                FloatRange::Linear { min: -20.0, max: 40.0 },
            )
            .with_unit(" dB")
            .with_step_size(1.0)
            .with_value_to_string(formatters::v2s_f32_rounded(0))
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_output_db: FloatParam::new(
                "FET Output",
                0.0,
                FloatRange::Linear { min: -20.0, max: 20.0 },
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
            .with_smoother(SmoothingStyle::Linear(5.0)),

            #[cfg(feature = "buttercomp2")]
            fet_ratio: EnumParam::<FetRatio>::new("FET Ratio", FetRatio::R4),

            #[cfg(feature = "buttercomp2")]
            fet_auto_release: BoolParam::new("FET Auto Release", false),

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

            // Independent low-cut frequency enables the classic Pultec
            // "trick": boost at e.g. 60 Hz, cut at e.g. 200 Hz for a tight
            // low end. Default 100 Hz is a neutral starting point; existing
            // sessions created before v0.5 load with this default, which
            // will sound different from the old coupled (0.6*boost) behavior.
            pultec_lf_cut_freq: FloatParam::new(
                "LF Atten Freq",
                100.0,
                FloatRange::Linear { min: 20.0, max: 200.0 },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
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
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_mode: EnumParam::new("DynEQ 4 Mode", DynamicMode::CompressDownward),
            #[cfg(feature = "dynamic_eq")]
            dyneq_band4_solo: BoolParam::new("DynEQ 4 Solo", false),

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

            // Punch Module Parameters (Clipper + Transient Shaper)
            // Default: BYPASSED - user must enable intentionally
            #[cfg(feature = "punch")]
            punch_bypass: BoolParam::new("Punch Bypass", true),

            #[cfg(feature = "punch")]
            punch_threshold: FloatParam::new(
                "Clip Threshold",
                -0.1, // -0.1dB default (gentle, near 0dB ceiling)
                FloatRange::Linear { min: -12.0, max: 0.0 },
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
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_unit("")
            .with_step_size(0.01),

            #[cfg(feature = "punch")]
            punch_sustain: FloatParam::new(
                "Sustain",
                0.0, // Neutral sustain
                FloatRange::Linear { min: -1.0, max: 1.0 },
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
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            #[cfg(feature = "punch")]
            punch_output_gain: FloatParam::new(
                "Punch Output",
                0.0, // 0dB
                FloatRange::Linear { min: -12.0, max: 12.0 },
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

            // Module Ordering Parameters (default signal chain)
            // Default order matches the standard 500-series layout:
            // EQ -> Comp -> Pultec -> Transformer -> Punch  (DynamicEQ in reserve slot 6)
            module_order_1: EnumParam::new("Module Order 1", ModuleType::Api5500EQ),
            module_order_2: EnumParam::new("Module Order 2", ModuleType::ButterComp2),
            module_order_3: EnumParam::new("Module Order 3", ModuleType::PultecEQ),
            module_order_4: EnumParam::new("Module Order 4", ModuleType::Transformer),
            module_order_5: EnumParam::new("Module Order 5", ModuleType::Punch),
            module_order_6: EnumParam::new("Module Order 6", ModuleType::DynamicEQ),
        }
    }
}

/// Compact 0..6 index for ModuleType — used for duplicate-detection when
/// dispatching modules in user-chosen order. Keep in lock-step with the
/// enum definition; any reorder there requires updating this match.
fn module_type_index(mt: ModuleType) -> usize {
    match mt {
        ModuleType::Api5500EQ => 0,
        ModuleType::ButterComp2 => 1,
        ModuleType::PultecEQ => 2,
        ModuleType::DynamicEQ => 3,
        ModuleType::Transformer => 4,
        ModuleType::Punch => 5,
    }
}

impl BusChannelStrip {
    // ── Per-module processing helpers ────────────────────────────────────────
    // Each helper is idempotent-safe to call zero or one times per buffer:
    //   • update_parameters() advances smoothers/coefficients even when bypassed
    //   • the bypass gate skips the actual DSP work
    // The module_order dispatch loop in process() calls each helper at most
    // once per buffer (duplicates are deduplicated).

    #[cfg(feature = "api5500")]
    fn process_module_api5500(&mut self, buffer: &mut Buffer) {
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

    #[cfg(feature = "buttercomp2")]
    fn process_module_buttercomp(&mut self, buffer: &mut Buffer) {
        if self.params.comp_bypass.value() {
            return;
        }
        match self.params.comp_model.value() {
            ButterComp2Model::Classic => {
                self.compressor.update_parameters(
                    self.params.comp_compress.value(),
                    self.params.comp_output.value(),
                    self.params.comp_dry_wet.value(),
                );
                self.compressor.process(buffer);
            }
            ButterComp2Model::Vca => {
                self.vca_compressor.update_parameters(
                    self.params.vca_thresh.smoothed.next(),
                    self.params.vca_ratio.smoothed.next(),
                    self.params.vca_atk.smoothed.next(),
                    self.params.vca_rel.smoothed.next(),
                    self.params.comp_sc_hp_freq.value(),
                );
                self.vca_compressor.process(buffer);
            }
            ButterComp2Model::Optical => {
                let thresh = self.params.opt_thresh.smoothed.next();
                let speed = self.params.opt_speed.smoothed.next();
                let char_v = self.params.opt_char.smoothed.next();
                self.optical_compressor
                    .update_parameters(thresh, speed, char_v);
                self.optical_compressor.process(buffer, thresh);
            }
            ButterComp2Model::Fet => {
                self.fet_compressor.update_parameters(
                    self.params.fet_input_db.smoothed.next(),
                    self.params.fet_output_db.smoothed.next(),
                    self.params.fet_attack_ms.smoothed.next(),
                    self.params.fet_release_ms.smoothed.next(),
                    self.params.fet_ratio.value(),
                    self.params.fet_auto_release.value(),
                    self.params.comp_sc_hp_freq.value(),
                );
                self.fet_compressor.process(buffer);
            }
        }
    }

    #[cfg(feature = "pultec")]
    fn process_module_pultec(&mut self, buffer: &mut Buffer) {
        self.pultec.update_parameters(
            self.params.pultec_lf_boost_freq.value(),
            self.params.pultec_lf_boost_gain.value(),
            self.params.pultec_lf_cut_freq.value(),
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

    #[cfg(feature = "transformer")]
    fn process_module_transformer(&mut self, buffer: &mut Buffer) {
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

    #[cfg(feature = "dynamic_eq")]
    fn process_module_dynamic_eq(&mut self, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers) {
        // Sidechain ring accumulation — runs regardless of bypass so the
        // ANALYZE SC feature always reflects the live sidechain.
        if !aux.inputs.is_empty() {
            for channel_samples in aux.inputs[0].iter_samples() {
                let mut mono = 0.0_f32;
                let mut n = 0_usize;
                for s in channel_samples {
                    mono += *s;
                    n += 1;
                }
                if n > 0 {
                    mono /= n as f32;
                }
                self.sc_ring[self.sc_ring_pos] = mono;
                self.sc_ring_pos = (self.sc_ring_pos + 1) % spectral::FFT_SIZE;
            }
        } else {
            for _ in 0..buffer.samples() {
                self.sc_ring[self.sc_ring_pos] = 0.0;
                self.sc_ring_pos = (self.sc_ring_pos + 1) % spectral::FFT_SIZE;
            }
        }

        let dyneq_params = [
            DynamicBandParams {
                mode: self.params.dyneq_band1_mode.value(),
                detector_freq: self.params.dyneq_band1_detector_freq.value(),
                freq: self.params.dyneq_band1_freq.value(),
                q: self.params.dyneq_band1_q.value(),
                threshold_db: self.params.dyneq_band1_threshold.value(),
                ratio: self.params.dyneq_band1_ratio.value(),
                attack_ms: self.params.dyneq_band1_attack.value(),
                release_ms: self.params.dyneq_band1_release.value(),
                gain_db: self.params.dyneq_band1_gain.value(),
                enabled: self.params.dyneq_band1_enabled.value(),
                solo: self.params.dyneq_band1_solo.value(),
            },
            DynamicBandParams {
                mode: self.params.dyneq_band2_mode.value(),
                detector_freq: self.params.dyneq_band2_detector_freq.value(),
                freq: self.params.dyneq_band2_freq.value(),
                q: self.params.dyneq_band2_q.value(),
                threshold_db: self.params.dyneq_band2_threshold.value(),
                ratio: self.params.dyneq_band2_ratio.value(),
                attack_ms: self.params.dyneq_band2_attack.value(),
                release_ms: self.params.dyneq_band2_release.value(),
                gain_db: self.params.dyneq_band2_gain.value(),
                enabled: self.params.dyneq_band2_enabled.value(),
                solo: self.params.dyneq_band2_solo.value(),
            },
            DynamicBandParams {
                mode: self.params.dyneq_band3_mode.value(),
                detector_freq: self.params.dyneq_band3_detector_freq.value(),
                freq: self.params.dyneq_band3_freq.value(),
                q: self.params.dyneq_band3_q.value(),
                threshold_db: self.params.dyneq_band3_threshold.value(),
                ratio: self.params.dyneq_band3_ratio.value(),
                attack_ms: self.params.dyneq_band3_attack.value(),
                release_ms: self.params.dyneq_band3_release.value(),
                gain_db: self.params.dyneq_band3_gain.value(),
                enabled: self.params.dyneq_band3_enabled.value(),
                solo: self.params.dyneq_band3_solo.value(),
            },
            DynamicBandParams {
                mode: self.params.dyneq_band4_mode.value(),
                detector_freq: self.params.dyneq_band4_detector_freq.value(),
                freq: self.params.dyneq_band4_freq.value(),
                q: self.params.dyneq_band4_q.value(),
                threshold_db: self.params.dyneq_band4_threshold.value(),
                ratio: self.params.dyneq_band4_ratio.value(),
                attack_ms: self.params.dyneq_band4_attack.value(),
                release_ms: self.params.dyneq_band4_release.value(),
                gain_db: self.params.dyneq_band4_gain.value(),
                enabled: self.params.dyneq_band4_enabled.value(),
                solo: self.params.dyneq_band4_solo.value(),
            },
        ];
        self.dynamic_eq.update_parameters(&dyneq_params);

        if !self.params.dyneq_bypass.value() {
            self.dynamic_eq.process(buffer);
        }

        // Publish per-band gain reduction to the GUI display (Relaxed — display only).
        {
            use std::sync::atomic::Ordering;
            let gr = self.dynamic_eq.get_gain_reduction_db();
            for (i, &db) in gr.iter().enumerate() {
                self.gr_data.bands[i].store(db.to_bits(), Ordering::Relaxed);
            }
        }

        // Accumulate post-DynEQ samples into the FFT ring buffer.
        // All buffers are pre-allocated in initialize() — no audio-thread alloc.
        for channel_samples in buffer.iter_samples() {
            let mut mono = 0.0_f32;
            let mut n = 0_usize;
            for s in channel_samples {
                mono += *s;
                n += 1;
            }
            if n > 0 {
                mono /= n as f32;
            }
            self.fft_ring[self.fft_ring_pos] = mono;
            self.fft_ring_pos += 1;

            if self.fft_ring_pos >= spectral::FFT_SIZE {
                self.fft_ring_pos = 0;
                for (dst, (&src, &win)) in self
                    .fft_input
                    .iter_mut()
                    .zip(self.fft_ring.iter().zip(self.fft_window.iter()))
                {
                    *dst = src * win;
                }
                if let Some(ref fft) = self.fft_engine {
                    if fft
                        .process_with_scratch(
                            &mut self.fft_input,
                            &mut self.fft_output,
                            &mut self.fft_scratch,
                        )
                        .is_ok()
                    {
                        const SMOOTH_ALPHA: f32 = 0.8;
                        const SMOOTH_BETA: f32 = 1.0 - SMOOTH_ALPHA;
                        let scale = 2.0 / spectral::FFT_SIZE as f32;
                        for (smooth, bin) in self.fft_magnitude_smooth[..spectral::SPECTRUM_BINS]
                            .iter_mut()
                            .zip(self.fft_output[..spectral::SPECTRUM_BINS].iter())
                        {
                            let mag = bin.norm() * scale;
                            *smooth = *smooth * SMOOTH_ALPHA + mag * SMOOTH_BETA;
                        }
                        self.spectrum_data.write_from_slice(
                            &self.fft_magnitude_smooth[..spectral::SPECTRUM_BINS],
                        );

                        use std::sync::atomic::Ordering;
                        if self.analysis_requested.swap(false, Ordering::Relaxed) {
                            for i in 0..spectral::FFT_SIZE {
                                let ring_idx = (self.sc_ring_pos + i) % spectral::FFT_SIZE;
                                self.sc_fft_input[i] = self.sc_ring[ring_idx] * self.fft_window[i];
                            }
                            if fft
                                .process_with_scratch(
                                    &mut self.sc_fft_input,
                                    &mut self.sc_fft_output,
                                    &mut self.fft_scratch,
                                )
                                .is_ok()
                            {
                                let scale = 2.0 / spectral::FFT_SIZE as f32;
                                let mut peak_overlap = 0.0_f32;
                                let mut peak_bin = 1_usize;

                                for i in 1..spectral::SPECTRUM_BINS {
                                    let main_mag = self.fft_output[i].norm() * scale;
                                    let sc_mag = self.sc_fft_output[i].norm() * scale;
                                    let overlap = main_mag * sc_mag;
                                    self.analysis_result.overlap_bins[i]
                                        .store(overlap.to_bits(), Ordering::Relaxed);
                                    if overlap > peak_overlap {
                                        peak_overlap = overlap;
                                        peak_bin = i;
                                    }
                                }
                                self.analysis_result.overlap_bins[0]
                                    .store(0_u32, Ordering::Relaxed);

                                let target_freq =
                                    peak_bin as f32 * self.sample_rate / spectral::FFT_SIZE as f32;

                                let target_band: u32 = if target_freq < 500.0 {
                                    0
                                } else if target_freq < 2000.0 {
                                    1
                                } else if target_freq < 6000.0 {
                                    2
                                } else {
                                    3
                                };

                                let sc_mag_at_peak = self.sc_fft_output[peak_bin].norm() * scale;
                                let sc_db = 20.0 * sc_mag_at_peak.max(f32::MIN_POSITIVE).log10();
                                let suggested_threshold = (sc_db - 6.0).clamp(-60.0, 0.0);

                                self.analysis_result
                                    .target_band
                                    .store(target_band, Ordering::Relaxed);
                                self.analysis_result
                                    .target_freq
                                    .store(target_freq.to_bits(), Ordering::Relaxed);
                                self.analysis_result
                                    .target_threshold_db
                                    .store(suggested_threshold.to_bits(), Ordering::Relaxed);
                                self.analysis_result.ready.store(true, Ordering::Release);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "punch")]
    fn process_module_punch(&mut self, buffer: &mut Buffer) {
        self.punch.update_parameters(
            self.params.punch_threshold.value(),
            self.params.punch_clip_mode.value(),
            self.params.punch_softness.value(),
            self.params.punch_oversampling.value(),
            self.params.punch_attack.value(),
            self.params.punch_sustain.value(),
            self.params.punch_attack_time.value(),
            self.params.punch_release_time.value(),
            self.params.punch_sensitivity.value(),
            self.params.punch_input_gain.value(),
            self.params.punch_output_gain.value(),
            self.params.punch_mix.value(),
        );
        if !self.params.punch_bypass.value() {
            self.punch.process(buffer);
        }
    }

    /// Dispatch a single module by type, honoring feature flags.
    /// When a feature is disabled the corresponding arm is a no-op — the
    /// module_order_* params remain host-visible regardless of feature set,
    /// so out-of-feature selections silently pass the signal through.
    fn dispatch_module(&mut self, mt: ModuleType, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers) {
        match mt {
            ModuleType::Api5500EQ => {
                #[cfg(feature = "api5500")]
                self.process_module_api5500(buffer);
                #[cfg(not(feature = "api5500"))]
                {
                    let _ = buffer;
                }
            }
            ModuleType::ButterComp2 => {
                #[cfg(feature = "buttercomp2")]
                self.process_module_buttercomp(buffer);
                #[cfg(not(feature = "buttercomp2"))]
                {
                    let _ = buffer;
                }
            }
            ModuleType::PultecEQ => {
                #[cfg(feature = "pultec")]
                self.process_module_pultec(buffer);
                #[cfg(not(feature = "pultec"))]
                {
                    let _ = buffer;
                }
            }
            ModuleType::Transformer => {
                #[cfg(feature = "transformer")]
                self.process_module_transformer(buffer);
                #[cfg(not(feature = "transformer"))]
                {
                    let _ = buffer;
                }
            }
            ModuleType::DynamicEQ => {
                #[cfg(feature = "dynamic_eq")]
                self.process_module_dynamic_eq(buffer, aux);
                #[cfg(not(feature = "dynamic_eq"))]
                {
                    let _ = (buffer, aux);
                }
            }
            ModuleType::Punch => {
                #[cfg(feature = "punch")]
                self.process_module_punch(buffer);
                #[cfg(not(feature = "punch"))]
                {
                    let _ = buffer;
                }
            }
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
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        // Default: stereo in/out, no sidechain required (backward-compatible).
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        // Optional: stereo main + stereo sidechain for masking analysis.
        // Select this layout in Reaper via the plugin's I/O panel.
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[new_nonzero_u32(2)],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
    ];

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
        editor::create(
            self.params.clone(),
            self.editor_state.clone(),
            self.spectrum_data.clone(),
            self.analysis_requested.clone(),
            self.analysis_result.clone(),
            self.gr_data.clone(),
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
        {
            self.eq_api5500 = Api5500::new(sr);
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.compressor = ButterComp2::new(sr);
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.fet_compressor = FetCompressor::new(sr);
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.vca_compressor = VcaCompressor::new(sr);
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.optical_compressor = OpticalCompressor::new(sr);
        }
        #[cfg(feature = "pultec")]
        {
            self.pultec = PultecEQ::new(sr);
        }
        #[cfg(feature = "dynamic_eq")]
        {
            self.dynamic_eq = DynamicEQ::new(sr);
        }
        #[cfg(feature = "transformer")]
        {
            self.transformer = TransformerModule::new(sr);
        }
        #[cfg(feature = "punch")]
        {
            self.punch = PunchModule::new(sr);
        }

        // Initialize temporary buffers for module reordering
        let max_buffer_size = _buffer_config.max_buffer_size as usize;
        let num_channels = _audio_io_layout.main_output_channels.unwrap().get() as usize;

        self.temp_buffer_1 = vec![vec![0.0; max_buffer_size]; num_channels];
        self.temp_buffer_2 = vec![vec![0.0; max_buffer_size]; num_channels];

        // Pre-allocate FFT buffers — must happen here so the audio thread never allocates.
        #[cfg(feature = "dynamic_eq")]
        {
            use realfft::RealFftPlanner;
            let mut planner = RealFftPlanner::<f32>::new();
            let fft = planner.plan_fft_forward(spectral::FFT_SIZE);
            self.fft_input = fft.make_input_vec();
            self.fft_output = fft.make_output_vec();
            self.fft_scratch = fft.make_scratch_vec();
            // Sidechain analysis buffers (same FFT size, separate allocation).
            self.sc_fft_input = fft.make_input_vec();
            self.sc_fft_output = fft.make_output_vec();
            self.fft_engine = Some(fft);
            self.fft_ring = vec![0.0_f32; spectral::FFT_SIZE];
            self.fft_ring_pos = 0;
            self.sc_ring = vec![0.0_f32; spectral::FFT_SIZE];
            self.sc_ring_pos = 0;
            self.sample_rate = sr;
            // Hann window: w[n] = 0.5 * (1 - cos(2π*n / (N-1)))
            self.fft_window = (0..spectral::FFT_SIZE)
                .map(|n| {
                    0.5 * (1.0
                        - (std::f32::consts::TAU * n as f32 / (spectral::FFT_SIZE - 1) as f32)
                            .cos())
                })
                .collect();
            self.fft_magnitude_smooth = vec![0.0_f32; spectral::SPECTRUM_BINS];
        }

        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
        #[cfg(feature = "buttercomp2")]
        {
            self.compressor.reset();
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.fet_compressor.reset();
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.vca_compressor.reset();
        }
        #[cfg(feature = "buttercomp2")]
        {
            self.optical_compressor.reset();
        }
        #[cfg(feature = "dynamic_eq")]
        {
            self.dynamic_eq.reset();
        }
        #[cfg(feature = "transformer")]
        {
            self.transformer.reset();
        }
        #[cfg(feature = "punch")]
        {
            self.punch.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Global bypass — pass audio through untouched.
        if self.params.global_bypass.value() {
            return ProcessStatus::Normal;
        }

        // Auto-gain: capture input RMS before any processing.
        let auto_gain_enabled = self.params.global_auto_gain.value();
        let pre_rms = if auto_gain_enabled {
            rms_linear(buffer.as_slice())
        } else {
            0.0
        };

        // Dispatch modules in user-chosen order.
        // Each of the six module_order_N params selects which module lands
        // in slot N. Duplicates are deduplicated: if the user puts API5500
        // in two slots, the module only runs once. Any slot whose feature
        // is disabled at build time becomes a no-op inside dispatch_module.
        let order = [
            self.params.module_order_1.value(),
            self.params.module_order_2.value(),
            self.params.module_order_3.value(),
            self.params.module_order_4.value(),
            self.params.module_order_5.value(),
            self.params.module_order_6.value(),
        ];
        let mut seen = [false; 6];
        for mt in order {
            let idx = module_type_index(mt);
            if seen[idx] {
                continue;
            }
            seen[idx] = true;
            self.dispatch_module(mt, buffer, aux);
        }

        // 7) Auto-gain compensation (before master trim so it doesn't fight the user's gain knob).
        if auto_gain_enabled {
            let post_rms = rms_linear(buffer.as_slice());
            if post_rms > 1e-6 {
                let target = (pre_rms / post_rms).clamp(AUTO_GAIN_MIN, AUTO_GAIN_MAX);
                self.auto_gain_correction = self.auto_gain_correction * AUTO_GAIN_SMOOTH
                    + target * (1.0 - AUTO_GAIN_SMOOTH);
            }
            // Apply smoothed correction.
            for ch in buffer.as_slice() {
                for s in ch.iter_mut() {
                    *s *= self.auto_gain_correction;
                }
            }
        } else {
            // Reset to unity so re-enabling starts smoothly from 1.0.
            self.auto_gain_correction = 1.0;
        }

        // 8) Master output trim (intentional user gain, always last).
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
