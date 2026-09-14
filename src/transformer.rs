use crate::detune::{micro_detune_pair, DetuneRng};
use crate::hysteresis::HysteresisCell;
use crate::oversampler::Oversampler;
use crate::shaping::{biquad_coeffs, one_pole_coeff};
use biquad::{Biquad, DirectForm1, Type};
use nice_plug::buffer::Buffer;
use nice_plug::prelude::Enum;

/// Oversampling factor for the transformer saturation stage. 4× = 2 halfband
/// stages (23 taps each, ~16 sample delay at native rate). 4× is the sweet
/// spot: enough headroom that the 2nd and 3rd harmonics of a -3 dB signal at
/// half-Nyquist do not fold back, without the CPU cost of 8×/16×.
const TRANSFORMER_OS_FACTOR: usize = 4;

/// Transformer-loading compressor: peak level above which gain reduction starts.
const COMPRESSION_THRESHOLD: f32 = 0.7;
/// Release of the compressor's peak detector — long enough not to follow waveform cycles.
const COMPRESSION_DETECTOR_RELEASE_S: f32 = 0.15;
/// Smoothing on the compressor gain, so detector ripple cannot modulate the waveform.
const COMPRESSION_GAIN_SMOOTHING_S: f32 = 0.02;

/// Professional Transformer Coloration Module
///
/// Models input and output transformers found in classic channel strips
/// - Input transformer: Impedance loading, saturation, frequency response
/// - Output transformer: Final harmonic coloration and gentle compression
/// - Multiple vintage transformer models (Neve, API, SSL-style)
pub struct TransformerModule {
    sample_rate: f32,

    // Input transformer stage
    input_transformer: TransformerStage,

    // Output transformer stage
    output_transformer: TransformerStage,

    // Frequency response filters — updated via update_coefficients(), never
    // recreated. Per-channel (#17) so each channel's shelf corner can carry
    // an independent stereo micro-detune.
    low_shelf: [DirectForm1<f32>; 2],
    high_shelf: [DirectForm1<f32>; 2],

    // Per-channel oversamplers for anti-aliased nonlinear saturation. Input
    // and output stages need independent oversamplers because their filter
    // states are not interchangeable, and because the linear shelf filters
    // between them run at native rate.
    input_os_l: Oversampler,
    input_os_r: Oversampler,
    output_os_l: Oversampler,
    output_os_r: Oversampler,

    // Transformer model
    model: TransformerModel,

    // Cached parameter values — frequency response is only recomputed when these change.
    cached_model: TransformerModel,
    cached_low_response: f32,
    cached_high_response: f32,

    /// #16: runs both stages' hysteresis cells at zero width (exact
    /// passthrough) when true. Default is `false` (hysteresis on) — see
    /// ADR-0012 and its 2026-09-14 amendment.
    hysteresis_bypass: bool,

    /// Per-instance stereo micro-detune (#17, TMT-style) applied to the
    /// low/high shelf corners. See ADR-0013.
    detune_rng: DetuneRng,
    detune: [f32; 2],
}

/// Individual transformer stage (input or output)
struct TransformerStage {
    // Saturation state
    saturation_amount: f32,
    drive_gain: f32,

    // Harmonic generation state
    harmonic_state: f32,

    // Gentle compression (transformer loading effect), per channel for the same
    // reason as `hysteresis` below.
    compression_amount: f32,
    envelope: [f32; 2],
    compression_gain: [f32; 2],
    detector_release: f32,
    gain_smoothing: f32,

    /// Play-operator memory (#16, ADR-0012) — driven signal passes through
    /// this before the model-specific saturation curve. One cell per
    /// channel: `TransformerStage` is a single instance shared by L and R
    /// (only the oversamplers are per-channel), so a shared cell would let
    /// each channel's hysteresis state bleed into the other's.
    hysteresis: [HysteresisCell; 2],
}

/// Transformer model types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum TransformerModel {
    #[name = "Vintage"]
    Vintage, // Classic vintage sound (Neve-style)
    #[name = "Modern"]
    Modern, // Clean modern transformers (API-style)
    #[name = "British"]
    British, // British console sound (SSL-style)
    #[name = "American"]
    American, // American console sound (custom)
}

impl TransformerStage {
    fn new(sample_rate: f32) -> Self {
        let oversampled_rate = sample_rate * TRANSFORMER_OS_FACTOR as f32;
        Self {
            saturation_amount: 0.0,
            drive_gain: 1.0,
            harmonic_state: 0.0,
            compression_amount: 0.0,
            envelope: [0.0; 2],
            compression_gain: [1.0; 2],
            detector_release: one_pole_coeff(COMPRESSION_DETECTOR_RELEASE_S, sample_rate),
            gain_smoothing: one_pole_coeff(COMPRESSION_GAIN_SMOOTHING_S, sample_rate),
            hysteresis: std::array::from_fn(|_| HysteresisCell::new(oversampled_rate)),
        }
    }

    fn reset(&mut self) {
        self.envelope = [0.0; 2];
        self.compression_gain = [1.0; 2];
        self.harmonic_state = 0.0;
        for h in &mut self.hysteresis {
            h.reset();
        }
    }

    /// Process sample through transformer stage with an oversampled
    /// saturation path for anti-aliasing.
    ///
    /// The per-model curve is pointwise (memoryless), so we upsample the
    /// driven signal, run each oversampled frame through the hysteresis
    /// cell (#16, ADR-0012) — on the wet path only — then the model's
    /// nonlinearity, then downsample. The transformer loading compression
    /// runs at native rate, and runs even when saturation is off so its
    /// detector never holds stale state.
    fn process_sample(
        &mut self,
        input: f32,
        model: TransformerModel,
        ch: usize,
        hysteresis_bypass: bool,
        os: &mut Oversampler,
        scratch: &mut [f32; TRANSFORMER_OS_FACTOR],
    ) -> f32 {
        let saturated = if self.saturation_amount < 0.01 {
            // #22: decay the saturation meter toward silence while this stage
            // is effectively off, instead of freezing at its last reading.
            self.harmonic_state *= 0.99;
            input
        } else {
            let driven_signal = input * self.drive_gain;

            // Oversampled saturation: upsample → hysteresis → pointwise nonlinearity → downsample.
            let up = os.upsample(driven_signal, 0);
            for i in 0..TRANSFORMER_OS_FACTOR {
                // Bypass still calls through with amount=0 (an exact
                // passthrough, see HysteresisCell::process) so the cell keeps
                // tracking the live signal and cannot click on re-engagement.
                let hyst_amount = if hysteresis_bypass {
                    0.0
                } else {
                    self.saturation_amount
                };
                let wet_in = self.hysteresis[ch].process(up[i], hyst_amount);
                scratch[i] = saturate_by_model(up[i], wet_in, self.saturation_amount, model);
            }
            let saturated = os.downsample(&scratch[..TRANSFORMER_OS_FACTOR], 0);

            // #22: smoothed harmonic-energy proxy (the saturation stage's
            // contribution above the driven signal) for the GUI saturation meter.
            let harmonic_diff = (saturated - driven_signal).abs();
            if harmonic_diff > self.harmonic_state {
                self.harmonic_state = harmonic_diff;
            } else {
                self.harmonic_state += (harmonic_diff - self.harmonic_state) * 0.01;
            }
            saturated
        };

        self.apply_transformer_compression(saturated, ch)
    }

    /// Gentle compression that mimics transformer loading: a peak detector
    /// drives a smoothed gain, so the gain moves at envelope rate rather than
    /// following individual waveform cycles.
    fn apply_transformer_compression(&mut self, input: f32, ch: usize) -> f32 {
        let level = input.abs();
        let envelope = &mut self.envelope[ch];
        if level > *envelope {
            *envelope = level;
        } else {
            *envelope += (level - *envelope) * self.detector_release;
        }

        let over_threshold = (*envelope - COMPRESSION_THRESHOLD).max(0.0);
        let target_gain = 1.0 / (1.0 + over_threshold * self.compression_amount * 2.0);
        let gain = &mut self.compression_gain[ch];
        *gain += (target_gain - *gain) * self.gain_smoothing;
        input * *gain
    }
}

impl TransformerModule {
    /// Create new transformer module
    pub fn new(sample_rate: f32) -> Self {
        // Initialize frequency response filters (flat by default)
        let flat_coeff = biquad_coeffs(Type::LowPass, sample_rate, 20000.0, 0.707)
            .expect("LowPass filter should be valid");

        // Oversamplers are called once per sample (inline use), so
        // `max_block_size = 1` is sufficient — each upsample/downsample pair
        // writes into buffer[0..TRANSFORMER_OS_FACTOR].
        let make_os = || Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);

        let mut detune_rng = DetuneRng::seed_from_entropy();
        let detune = micro_detune_pair(&mut detune_rng);

        Self {
            sample_rate,
            input_transformer: TransformerStage::new(sample_rate),
            output_transformer: TransformerStage::new(sample_rate),
            low_shelf: [
                DirectForm1::<f32>::new(flat_coeff),
                DirectForm1::<f32>::new(flat_coeff),
            ],
            high_shelf: [
                DirectForm1::<f32>::new(flat_coeff),
                DirectForm1::<f32>::new(flat_coeff),
            ],
            input_os_l: make_os(),
            input_os_r: make_os(),
            output_os_l: make_os(),
            output_os_r: make_os(),
            model: TransformerModel::Vintage,
            cached_model: TransformerModel::Vintage,
            cached_low_response: f32::NAN, // NAN forces recompute on first call
            cached_high_response: f32::NAN,
            hysteresis_bypass: false,
            detune_rng,
            detune,
        }
    }

    /// Update transformer parameters
    #[allow(clippy::too_many_arguments)]
    pub fn update_parameters(
        &mut self,
        model: TransformerModel,
        input_drive: f32,
        input_saturation: f32,
        output_drive: f32,
        output_saturation: f32,
        low_frequency_response: f32,  // -1 to 1 (cut to boost)
        high_frequency_response: f32, // -1 to 1 (cut to boost)
        transformer_compression: f32, // Overall compression amount
        hysteresis_bypass: bool,      // #16: true restores bit-identical v1.0 output
    ) {
        self.model = model;
        self.hysteresis_bypass = hysteresis_bypass;

        // Input transformer settings - much gentler
        self.input_transformer.drive_gain = 1.0 + input_drive * 0.8; // 1x to 1.8x gain
        self.input_transformer.saturation_amount = input_saturation * 0.6; // Reduce saturation
        self.input_transformer.compression_amount = transformer_compression * 0.3; // Less compression on input

        // Output transformer settings - also gentler
        self.output_transformer.drive_gain = 1.0 + output_drive * 0.6; // 1x to 1.6x gain
        self.output_transformer.saturation_amount = output_saturation * 0.5; // Reduce saturation
        self.output_transformer.compression_amount = transformer_compression * 0.7;

        // Only recompute filter coefficients when model or response values change.
        // Comparing f32 for exact equality is valid here: we are checking whether
        // the stored parameter value (same f32 bits) has been updated by the host,
        // not comparing computed results where rounding would be an issue.
        if model != self.cached_model
            || low_frequency_response != self.cached_low_response
            || high_frequency_response != self.cached_high_response
        {
            self.cached_model = model;
            self.cached_low_response = low_frequency_response;
            self.cached_high_response = high_frequency_response;
            self.update_frequency_response(low_frequency_response, high_frequency_response);
        }
    }

    /// Update frequency response characteristics.
    ///
    /// Uses `update_coefficients()` on existing filter objects — no state reset,
    /// no heap allocation. Called only when model or response values change
    /// (guarded in `update_parameters()`), and directly from `reset()` when
    /// only the stereo micro-detune (#17) has been redrawn.
    fn update_frequency_response(&mut self, low_response: f32, high_response: f32) {
        let low_freq = match self.model {
            TransformerModel::Vintage => 80.0,
            TransformerModel::Modern => 60.0,
            TransformerModel::British => 100.0,
            TransformerModel::American => 70.0,
        };
        // Always update (even at 0 dB) so that model changes take effect immediately.
        let low_gain = low_response * 3.0; // ±3 dB
        for ch in 0..2 {
            if let Ok(coeff) = biquad_coeffs(
                Type::LowShelf(low_gain),
                self.sample_rate,
                low_freq * self.detune[ch],
                0.707,
            ) {
                self.low_shelf[ch].update_coefficients(coeff);
            }
        }

        let high_freq = match self.model {
            TransformerModel::Vintage => 8000.0,
            TransformerModel::Modern => 15000.0,
            TransformerModel::British => 12000.0,
            TransformerModel::American => 10000.0,
        };
        let high_gain = high_response * 2.0; // ±2 dB
        for ch in 0..2 {
            if let Ok(coeff) = biquad_coeffs(
                Type::HighShelf(high_gain),
                self.sample_rate,
                high_freq * self.detune[ch],
                0.707,
            ) {
                self.high_shelf[ch].update_coefficients(coeff);
            }
        }
    }

    /// Process audio buffer through transformer module
    pub fn process(&mut self, buffer: &mut Buffer) {
        // Stack scratch for the oversampled saturation path. Reused across
        // every sample; the oversampler writes `TRANSFORMER_OS_FACTOR` values
        // in and reads them back before the next call overwrites.
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        for mut samples in buffer.iter_samples() {
            for (ch, sample) in samples.iter_mut().enumerate() {
                let ch = ch.min(1);
                let mut s = *sample;

                // 1. Input transformer stage (oversampled saturation)
                let in_os = if ch == 0 {
                    &mut self.input_os_l
                } else {
                    &mut self.input_os_r
                };
                s = self.input_transformer.process_sample(
                    s,
                    self.model,
                    ch,
                    self.hysteresis_bypass,
                    in_os,
                    &mut scratch,
                );

                // 2. Frequency response modeling (native rate)
                s = self.low_shelf[ch].run(s);
                s = self.high_shelf[ch].run(s);

                // 3. Output transformer stage (oversampled saturation)
                let out_os = if ch == 0 {
                    &mut self.output_os_l
                } else {
                    &mut self.output_os_r
                };
                s = self.output_transformer.process_sample(
                    s,
                    self.model,
                    ch,
                    self.hysteresis_bypass,
                    out_os,
                    &mut scratch,
                );

                *sample = s;
            }
        }
    }

    /// Reset transformer state
    pub fn reset(&mut self) {
        // #17: redraw this instance's stereo micro-detune. `update_parameters`
        // only recomputes the shelf coefficients when model/response values
        // change, so a fresh detune wouldn't otherwise take effect — force a
        // recompute here using the last-known response values (skipped
        // before the very first `update_parameters` call, when they're
        // still the NaN sentinel).
        self.detune = micro_detune_pair(&mut self.detune_rng);
        if !self.cached_low_response.is_nan() && !self.cached_high_response.is_nan() {
            self.update_frequency_response(self.cached_low_response, self.cached_high_response);
        }

        self.input_transformer.reset();
        self.output_transformer.reset();
        self.input_os_l.reset();
        self.input_os_r.reset();
        self.output_os_l.reset();
        self.output_os_r.reset();
    }

    /// Current harmonic-content (saturation) level for the GUI meter —
    /// averages the input and output stages' smoothed harmonic-energy
    /// proxies. Not a calibrated unit, just a relative "how much coloration
    /// is happening right now" signal; 0.0 = none.
    pub fn get_saturation_level(&self) -> f32 {
        (self.input_transformer.harmonic_state + self.output_transformer.harmonic_state) * 0.5
    }

    /// Decay both stages' saturation-meter state toward zero without
    /// touching any other processing state. Call once per buffer while the
    /// module is bypassed (`process()` isn't run, so `harmonic_state`
    /// wouldn't otherwise decay) — same per-buffer decay rate `process_sample`
    /// applies per-sample when its own saturation amount is near zero, so
    /// the GUI meter still falls to zero instead of freezing (issue #22).
    pub fn decay_saturation_level(&mut self) {
        self.input_transformer.harmonic_state *= 0.99;
        self.output_transformer.harmonic_state *= 0.99;
    }
}

/// Blend `dry` with the model's saturation curve applied to `wet_in`. The two
/// are the same sample unless a memory stage (hysteresis) has shaped the wet
/// path. Pointwise (memoryless), so safe inside the oversampled block.
#[inline]
fn saturate_by_model(dry: f32, wet_in: f32, amount: f32, model: TransformerModel) -> f32 {
    if amount < 0.01 {
        return dry;
    }
    let (wet, wet_share) = match model {
        TransformerModel::Vintage => (vintage_transformer_saturation(wet_in, amount), 0.7),
        TransformerModel::Modern => (modern_transformer_saturation(wet_in, amount), 0.5),
        TransformerModel::British => (british_transformer_saturation(wet_in, amount), 0.6),
        TransformerModel::American => (american_transformer_saturation(wet_in, amount), 0.6),
    };
    let mix = amount * wet_share;
    dry * (1.0 - mix) + wet * mix
}

// Transformer saturation models (wet curves; `saturate_by_model` blends them)

/// Vintage transformer saturation (Neve-style): warm, musical, even harmonics.
fn vintage_transformer_saturation(input: f32, amount: f32) -> f32 {
    let driven = input * (1.0 + amount * 2.0);
    // Even-harmonic term. x*|x| is genuinely 2nd-order without the DC offset
    // that x² introduces — x² is always ≥ 0, so it adds a positive bias that
    // accumulates through downstream IIR stages and muddies low frequencies.
    let harmonic = driven * driven.abs() * amount * 0.1;
    driven.tanh() + harmonic
}

/// Modern transformer saturation (API-style): clean, subtle asymmetric odd
/// harmonics when pushed. `d / sqrt(1 + k·d²)` is monotonic; the earlier
/// `d / (1 + k·d²)` folded back above `1/sqrt(k)`.
fn modern_transformer_saturation(input: f32, amount: f32) -> f32 {
    let driven = input * (1.0 + amount * 1.5);
    let k = if driven > 0.0 { amount } else { amount * 0.8 };
    driven / (1.0 + driven * driven * k).sqrt()
}

/// British transformer saturation (SSL-style): tight, controlled.
fn british_transformer_saturation(input: f32, amount: f32) -> f32 {
    let driven = input * (1.0 + amount * 1.2);
    let saturated = driven / (1.0 + driven.abs() * amount * 0.8);
    let harmonic = driven.signum() * driven * driven * amount * 0.05;
    saturated + harmonic
}

/// American transformer saturation (custom balanced): soft clip above 0.5.
fn american_transformer_saturation(input: f32, amount: f32) -> f32 {
    let driven = input * (1.0 + amount * 1.8);
    let saturated = if driven.abs() > 0.5 {
        driven.signum() * (0.5 + (driven.abs() - 0.5).tanh() * 0.5)
    } else {
        driven
    };
    let harmonic = driven * driven * driven * amount * 0.08;
    saturated + harmonic
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODELS: [TransformerModel; 4] = [
        TransformerModel::Vintage,
        TransformerModel::Modern,
        TransformerModel::British,
        TransformerModel::American,
    ];
    const SR: f32 = 48_000.0;

    fn sine(amp: f32, freq: f32, i: usize) -> f32 {
        ((std::f64::consts::TAU * freq as f64 * i as f64 / SR as f64).sin() * amp as f64) as f32
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt()
    }

    fn db_to_amp(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// THD+N in dB: least-squares fit of the fundamental (window must hold a
    /// whole number of cycles); everything left over counts as distortion.
    fn thd_db(signal: &[f32], freq: f32) -> f32 {
        let n = signal.len() as f64;
        let mean = signal.iter().map(|&v| v as f64).sum::<f64>() / n;
        let w = std::f64::consts::TAU * freq as f64 / SR as f64;
        let (mut a, mut b) = (0.0, 0.0);
        for (i, &v) in signal.iter().enumerate() {
            let t = w * i as f64;
            a += (v as f64 - mean) * t.sin();
            b += (v as f64 - mean) * t.cos();
        }
        a *= 2.0 / n;
        b *= 2.0 / n;
        let (mut fundamental, mut residual) = (0.0, 0.0);
        for (i, &v) in signal.iter().enumerate() {
            let t = w * i as f64;
            let f = a * t.sin() + b * t.cos();
            fundamental += f * f;
            residual += (v as f64 - mean - f).powi(2);
        }
        (10.0 * (residual / fundamental).log10()) as f32
    }

    /// Run a sine through one stage (channel 0); returns the last `4800` samples.
    fn stage_tail(stage: &mut TransformerStage, amp: f32, freq: f32, bypass: bool) -> Vec<f32> {
        let mut os = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        let n = 24_000;
        let out: Vec<f32> = (0..n)
            .map(|i| {
                let x = sine(amp, freq, i);
                stage.process_sample(
                    x,
                    TransformerModel::Vintage,
                    0,
                    bypass,
                    &mut os,
                    &mut scratch,
                )
            })
            .collect();
        out[n - 4800..].to_vec()
    }

    // ── Saturation curves ─────────────────────────────────────────────────────

    #[test]
    fn test_saturation_zero_amount_returns_dry() {
        for model in MODELS {
            for &input in &[-1.0_f32, -0.5, 0.0, 0.5, 1.0] {
                assert_eq!(
                    saturate_by_model(input, 0.3, 0.0, model),
                    input,
                    "{model:?}"
                );
            }
        }
    }

    #[test]
    fn test_saturation_finite_and_bounded() {
        for model in MODELS {
            for &amount in &[0.1_f32, 0.5, 1.0] {
                for &input in &[-2.0_f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
                    let y = saturate_by_model(input, input, amount, model);
                    assert!(y.is_finite(), "{model:?} input={input} amount={amount}");
                    if input.abs() <= 1.0 {
                        assert!(y.abs() < 4.0, "{model:?} out of bounds: {y}");
                    }
                }
            }
        }
    }

    #[test]
    fn test_vintage_saturation_no_dc_offset() {
        // The even-harmonic term must be x*|x| not x² — x² is always ≥ 0 and
        // adds DC bias that accumulates in downstream IIR stages (audit #5).
        let amount = 0.7_f32;
        let n = 4096;
        let sum: f64 = (0..n)
            .map(|i| {
                let x = (i as f32 / n as f32 * std::f32::consts::TAU * 8.0).sin() * 0.7;
                saturate_by_model(x, x, amount, TransformerModel::Vintage) as f64
            })
            .sum();
        let mean = sum / n as f64;
        assert!(
            mean.abs() < 1e-3,
            "Vintage saturation must not inject DC; mean={mean}"
        );
    }

    #[test]
    fn test_modern_wet_curve_is_monotonic() {
        let mut prev = f32::NEG_INFINITY;
        for i in -400..=400 {
            let x = i as f32 / 100.0;
            let y = modern_transformer_saturation(x, 0.6);
            assert!(y >= prev, "Modern curve folds back at x={x}: {y} < {prev}");
            prev = y;
        }
    }

    // ── Stage level/distortion behaviour ──────────────────────────────────────

    /// Regression: the absolute hysteresis half-width silenced quiet input
    /// (below ~-29 dBFS at saturation 0.59).
    #[test]
    fn test_quiet_signal_survives_saturation() {
        let run = |bypass| {
            let mut stage = TransformerStage::new(SR);
            stage.saturation_amount = 0.59 * 0.6;
            rms(&stage_tail(&mut stage, db_to_amp(-30.0), 1000.0, bypass))
        };
        let ratio = run(false) / run(true);
        assert!(
            ratio > 0.9,
            "-30 dBFS sine lost level through hysteresis: ratio {ratio}"
        );
    }

    /// Regression: saturation 0.1 produced -27 dB THD on a -24 dBFS sine.
    #[test]
    fn test_low_saturation_is_clean_at_low_level() {
        let mut stage = TransformerStage::new(SR);
        stage.saturation_amount = 0.1 * 0.6;
        let thd = thd_db(
            &stage_tail(&mut stage, db_to_amp(-24.0), 1000.0, false),
            1000.0,
        );
        assert!(thd < -60.0, "THD at saturation 0.1, -24 dBFS: {thd} dB");
    }

    #[test]
    fn test_quiet_signals_distort_no_more_than_loud_ones() {
        let thd_at = |db| {
            let mut stage = TransformerStage::new(SR);
            stage.saturation_amount = 0.59 * 0.6;
            thd_db(
                &stage_tail(&mut stage, db_to_amp(db), 1000.0, false),
                1000.0,
            )
        };
        let (quiet, loud) = (thd_at(-24.0), thd_at(-6.0));
        assert!(
            quiet <= loud + 3.0,
            "quiet THD {quiet} dB vs loud {loud} dB"
        );
    }

    #[test]
    fn test_compression_engages_without_saturation_and_is_smooth() {
        let mut stage = TransformerStage::new(SR);
        stage.compression_amount = 0.7;
        let amp = 0.95;
        let tail = stage_tail(&mut stage, amp, 100.0, false);
        let input_rms = amp / std::f32::consts::SQRT_2;
        assert!(
            rms(&tail) < 0.9 * input_rms,
            "no gain reduction: {}",
            rms(&tail)
        );
        let thd = thd_db(&tail, 100.0);
        assert!(
            thd < -50.0,
            "compressor distorts at waveform rate: {thd} dB"
        );
    }

    #[test]
    fn test_compression_state_is_independent_per_channel() {
        let mut shared = TransformerStage::new(SR);
        let mut alone = TransformerStage::new(SR);
        for s in [&mut shared, &mut alone] {
            s.saturation_amount = 0.3;
            s.compression_amount = 0.7;
        }
        let make_os = || Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let (mut os_l, mut os_r, mut os_alone) = (make_os(), make_os(), make_os());
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        let model = TransformerModel::Vintage;
        for i in 0..4800 {
            let quiet = sine(0.2, 1000.0, i);
            shared.process_sample(
                sine(0.95, 1000.0, i),
                model,
                0,
                false,
                &mut os_l,
                &mut scratch,
            );
            let r = shared.process_sample(quiet, model, 1, false, &mut os_r, &mut scratch);
            let expected =
                alone.process_sample(quiet, model, 1, false, &mut os_alone, &mut scratch);
            assert!(
                (r - expected).abs() < 1e-6,
                "right channel compressed by left at {i}"
            );
        }
    }

    // ── TransformerModule ─────────────────────────────────────────────────────

    #[test]
    fn test_transformer_module_new_does_not_panic() {
        let _t = TransformerModule::new(44100.0);
        let _t = TransformerModule::new(48000.0);
        let _t = TransformerModule::new(96000.0);
    }

    #[test]
    fn test_transformer_module_nan_cache_forces_recompute() {
        // NaN sentinel in cached values should cause update_frequency_response on first call
        let mut t = TransformerModule::new(44100.0);
        assert!(
            t.cached_low_response.is_nan(),
            "cached_low_response should start NaN"
        );
        assert!(
            t.cached_high_response.is_nan(),
            "cached_high_response should start NaN"
        );
        // First update_parameters call should not panic
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.3,
            false,
        );
        assert!(
            !t.cached_low_response.is_nan(),
            "cached_low_response should be set after first update"
        );
    }

    #[test]
    fn test_transformer_module_model_selection() {
        let mut t = TransformerModule::new(44100.0);
        for model in [
            TransformerModel::Vintage,
            TransformerModel::Modern,
            TransformerModel::British,
            TransformerModel::American,
        ] {
            t.update_parameters(model, 0.3, 0.3, 0.3, 0.3, 0.0, 0.0, 0.3, false);
            assert_eq!(t.model, model, "Model should be updated to {:?}", model);
        }
    }

    #[test]
    fn test_transformer_module_cache_skips_filter_recompute() {
        let mut t = TransformerModule::new(44100.0);
        // First call — triggers recompute and sets cache
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.2,
            0.2,
            0.3,
            false,
        );
        let cached_low = t.cached_low_response;
        let cached_high = t.cached_high_response;
        // Same values — cache should match, no recompute
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.2,
            0.2,
            0.3,
            false,
        );
        assert_eq!(t.cached_low_response.to_bits(), cached_low.to_bits());
        assert_eq!(t.cached_high_response.to_bits(), cached_high.to_bits());
    }

    #[test]
    fn test_transformer_module_model_change_updates_cache() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.3,
            false,
        );
        assert_eq!(t.cached_model, TransformerModel::Vintage);
        // Change model — cached_model should update
        t.update_parameters(
            TransformerModel::British,
            0.3,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.3,
            false,
        );
        assert_eq!(t.cached_model, TransformerModel::British);
    }

    #[test]
    fn test_transformer_module_reset_clears_envelopes() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(
            TransformerModel::Vintage,
            0.5,
            0.8,
            0.5,
            0.8,
            0.0,
            0.0,
            0.5,
            false,
        );
        t.input_transformer.envelope = [0.9, 0.8];
        t.input_transformer.compression_gain = [0.5, 0.6];
        t.output_transformer.envelope = [0.7, 0.7];
        t.reset();
        for stage in [&t.input_transformer, &t.output_transformer] {
            assert_eq!(stage.envelope, [0.0; 2]);
            assert_eq!(stage.compression_gain, [1.0; 2]);
        }
    }

    #[test]
    fn test_transformer_input_drive_scales() {
        let mut t = TransformerModule::new(44100.0);
        // input_drive=0 → drive_gain = 1.0; input_drive=1 → drive_gain = 1.8
        t.update_parameters(
            TransformerModel::Vintage,
            0.0,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.0,
            false,
        );
        assert!(
            (t.input_transformer.drive_gain - 1.0).abs() < 1e-5,
            "drive=0 should give gain 1.0"
        );
        t.update_parameters(
            TransformerModel::Vintage,
            1.0,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.0,
            false,
        );
        assert!(
            (t.input_transformer.drive_gain - 1.8).abs() < 1e-5,
            "drive=1 should give gain 1.8"
        );
    }

    #[test]
    fn test_transformer_freq_response_per_model() {
        // Spot-check that low_freq characteristic differs per model
        let mut t44 = TransformerModule::new(44100.0);
        // Vintage → low_freq=80, Modern → low_freq=60
        // Simply verify update doesn't panic for each model
        for model in [
            TransformerModel::Vintage,
            TransformerModel::Modern,
            TransformerModel::British,
            TransformerModel::American,
        ] {
            t44.update_parameters(model, 0.3, 0.3, 0.3, 0.3, 0.5, -0.5, 0.3, false);
        }
    }

    /// With the oversampler in place, pushing a hot signal through the
    /// per-sample saturation path shouldn't blow up — verify finite output
    /// under the full nonlinear stack.
    #[test]
    fn test_transformer_saturation_oversampled_bounded() {
        let mut t = TransformerModule::new(44100.0);
        // Maximum saturation on both stages to exercise the nonlinearity.
        t.update_parameters(
            TransformerModel::Vintage,
            1.0,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
            false,
        );

        // Pass 1024 samples of hot sine through by reaching into the private
        // per-stage method. No Buffer needed — we just need to verify the
        // oversampled saturation path is numerically stable.
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        let mut os = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut stage = TransformerStage::new(44100.0);
        stage.drive_gain = 1.8;
        stage.saturation_amount = 0.6;
        stage.compression_amount = 0.3;
        for i in 0..1024 {
            let x = (2.0 * core::f32::consts::PI * 0.4 * i as f32).sin(); // ~17.6 kHz
            let y = stage.process_sample(
                x,
                TransformerModel::Vintage,
                0,
                false,
                &mut os,
                &mut scratch,
            );
            assert!(y.is_finite(), "non-finite sample {y} at i={i}");
            assert!(y.abs() < 10.0, "implausibly large sample {y} at i={i}");
        }
    }

    // ── Hysteresis integration (#16, ADR-0012) ─────────────────────────────────

    /// Bypassed and engaged hysteresis must diverge on a swept signal — proof
    /// the play operator is actually wired into the saturation path, not a
    /// dead field.
    #[test]
    fn test_hysteresis_bypass_vs_engaged_differ_on_swept_signal() {
        let mut os_bypass = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut os_engaged = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        let mut stage_bypass = TransformerStage::new(44100.0);
        let mut stage_engaged = TransformerStage::new(44100.0);
        for s in [&mut stage_bypass, &mut stage_engaged] {
            s.drive_gain = 1.8;
            s.saturation_amount = 0.6;
        }

        let n = 512;
        let mut diverged = false;
        for i in 0..n {
            let x = (2.0 * core::f32::consts::PI * 0.05 * i as f32).sin() * 0.9;
            let yb = stage_bypass.process_sample(
                x,
                TransformerModel::Vintage,
                0,
                true,
                &mut os_bypass,
                &mut scratch,
            );
            let ye = stage_engaged.process_sample(
                x,
                TransformerModel::Vintage,
                0,
                false,
                &mut os_engaged,
                &mut scratch,
            );
            assert!(yb.is_finite() && ye.is_finite());
            if (yb - ye).abs() > 1.0e-6 {
                diverged = true;
            }
        }
        assert!(
            diverged,
            "hysteresis-engaged output never differed from bypassed output"
        );
    }

    /// Regression for the reviewer-confirmed cross-channel bleed: feeding
    /// distinct L/R signals through the same `TransformerStage` must not let
    /// one channel's play-operator memory leak into the other's output.
    #[test]
    fn test_hysteresis_state_is_independent_per_channel() {
        let mut os_l = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut os_r = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        let mut stage = TransformerStage::new(44100.0);
        stage.drive_gain = 1.8;
        stage.saturation_amount = 0.6;

        // Drive L hard positive first so its play-operator rail is pinned
        // high, while R stays silent. If the cells were shared, R would
        // inherit L's rail state on its very first sample.
        for _ in 0..64 {
            stage.process_sample(
                0.9,
                TransformerModel::Vintage,
                0,
                false,
                &mut os_l,
                &mut scratch,
            );
        }
        let r_first = stage.process_sample(
            0.0,
            TransformerModel::Vintage,
            1,
            false,
            &mut os_r,
            &mut scratch,
        );
        assert!(
            r_first.abs() < 1.0e-3,
            "channel 1 output was contaminated by channel 0's hysteresis state: {r_first}"
        );
    }

    #[test]
    fn test_transformer_module_hysteresis_bypass_threads_through() {
        let mut t = TransformerModule::new(44100.0);
        assert!(!t.hysteresis_bypass, "default should be hysteresis ON");
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.3,
            true,
        );
        assert!(t.hysteresis_bypass);
        t.update_parameters(
            TransformerModel::Vintage,
            0.3,
            0.3,
            0.3,
            0.3,
            0.0,
            0.0,
            0.3,
            false,
        );
        assert!(!t.hysteresis_bypass);
    }

    /// #17: with saturation disabled (isolating the low/high shelf filters),
    /// a pinned asymmetric detune should measurably decorrelate L/R.
    #[test]
    fn test_transformer_detune_decorrelates_channels() {
        let sr = 48_000.0;
        let mut t = TransformerModule::new(sr);
        t.detune = [1.003, 0.997];
        t.update_parameters(
            TransformerModel::Vintage,
            0.0,
            0.0, // input saturation off
            0.0,
            0.0, // output saturation off
            1.0,
            1.0, // full low/high shelf boost
            0.0,
            false,
        );

        let n = 4096_usize;
        let omega = 2.0 * core::f32::consts::PI * 80.0 / sr; // near the Vintage low-shelf corner
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin()).collect();
        let mut r: Vec<f32> = l.clone();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        t.process(&mut buf);

        let max_diff = l[n / 2..]
            .iter()
            .zip(r[n / 2..].iter())
            .fold(0.0_f32, |acc, (&a, &b)| acc.max((a - b).abs()));
        assert!(
            max_diff > 1.0e-4,
            "detuned L/R shelf corners should decorrelate the response, got max_diff={max_diff:.6}"
        );
    }

    #[test]
    fn test_transformer_reset_clears_hysteresis_state() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(
            TransformerModel::Vintage,
            1.0,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
            false,
        );
        t.input_transformer.hysteresis[0].process(0.9, 0.6);
        t.reset();
        let mut fresh = HysteresisCell::new(44100.0 * TRANSFORMER_OS_FACTOR as f32);
        for &x in &[0.01_f32, 0.3, -0.2] {
            assert_eq!(
                t.input_transformer.hysteresis[0].process(x, 0.6),
                fresh.process(x, 0.6),
                "hysteresis state not cleared by reset"
            );
        }
    }

    // ── #22 saturation meter ────────────────────────────────────────────────

    #[test]
    fn test_transformer_saturation_level_zero_at_rest() {
        let t = TransformerModule::new(44100.0);
        assert_eq!(t.get_saturation_level(), 0.0);
    }

    #[test]
    fn test_transformer_saturation_level_rises_with_driven_signal() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(
            TransformerModel::Vintage,
            1.0,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
            false,
        );
        let n = 2048_usize;
        let omega = 2.0 * core::f32::consts::PI * 440.0 / 44100.0;
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin() * 0.9).collect();
        let mut r: Vec<f32> = l.clone();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        t.process(&mut buf);
        let level = t.get_saturation_level();
        assert!(
            level > 0.0,
            "Expected positive saturation level after driving a hot signal, got {level}"
        );
        assert!(level.is_finite());
    }

    /// Regression for the module-level bypass path (lib.rs calls this
    /// instead of `process()` when `transformer_bypass` is on) — without it
    /// the GUI meter freezes at its last reading forever (issue #22 review).
    #[test]
    fn test_transformer_module_decay_saturation_level_reduces_toward_zero() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(
            TransformerModel::Vintage,
            1.0,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
            false,
        );
        let n = 2048_usize;
        let omega = 2.0 * core::f32::consts::PI * 440.0 / 44100.0;
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin() * 0.9).collect();
        let mut r: Vec<f32> = l.clone();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        t.process(&mut buf);
        let level_before = t.get_saturation_level();
        assert!(level_before > 0.0);

        for _ in 0..200 {
            t.decay_saturation_level();
        }
        let level_after_200 = t.get_saturation_level();
        assert!(
            level_after_200 < level_before * 0.5,
            "expected substantial decay after 200 calls, before={level_before} after={level_after_200}"
        );

        for _ in 0..2000 {
            t.decay_saturation_level();
        }
        let level_after_2200 = t.get_saturation_level();
        assert!(
            level_after_2200 < 1e-3,
            "expected level to have decayed near zero after 2200 calls, got {level_after_2200}"
        );
    }

    #[test]
    fn test_transformer_saturation_level_decays_when_bypassed() {
        let mut stage = TransformerStage::new(44100.0);
        stage.harmonic_state = 0.5;
        stage.saturation_amount = 0.0; // effectively off
        let mut os = Oversampler::new_at_factor(TRANSFORMER_OS_FACTOR, 1);
        let mut scratch = [0.0_f32; TRANSFORMER_OS_FACTOR];
        stage.process_sample(
            0.5,
            TransformerModel::Vintage,
            0,
            false,
            &mut os,
            &mut scratch,
        );
        assert!(
            stage.harmonic_state < 0.5,
            "harmonic_state should decay when saturation is off, got {}",
            stage.harmonic_state
        );
    }
}
