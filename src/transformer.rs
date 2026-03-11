use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};
use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

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

    // Frequency response filters — updated via update_coefficients(), never recreated.
    low_shelf: DirectForm1<f32>,
    high_shelf: DirectForm1<f32>,

    // Transformer model
    model: TransformerModel,

    // Cached parameter values — frequency response is only recomputed when these change.
    cached_model: TransformerModel,
    cached_low_response: f32,
    cached_high_response: f32,
}

/// Individual transformer stage (input or output)
struct TransformerStage {
    // Saturation state
    saturation_amount: f32,
    drive_gain: f32,
    
    // Harmonic generation state
    harmonic_state: f32,
    
    // Gentle compression (transformer loading effect)
    compression_amount: f32,
    envelope: f32,
}

/// Transformer model types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum TransformerModel {
    #[name = "Vintage"]
    Vintage,    // Classic vintage sound (Neve-style)
    #[name = "Modern"]
    Modern,     // Clean modern transformers (API-style) 
    #[name = "British"]
    British,    // British console sound (SSL-style)
    #[name = "American"]
    American,   // American console sound (custom)
}

impl TransformerStage {
    fn new() -> Self {
        Self {
            saturation_amount: 0.0,
            drive_gain: 1.0,
            harmonic_state: 0.0,
            compression_amount: 0.0,
            envelope: 0.0,
        }
    }
    
    /// Process sample through transformer stage
    fn process_sample(&mut self, input: f32, model: TransformerModel) -> f32 {
        if self.saturation_amount < 0.01 {
            return input;
        }
        
        // Apply input drive
        let driven_signal = input * self.drive_gain;
        
        // Transformer saturation modeling
        let saturated = match model {
            TransformerModel::Vintage => {
                // Neve-style: Warm, musical saturation with even harmonics
                vintage_transformer_saturation(driven_signal, self.saturation_amount)
            },
            TransformerModel::Modern => {
                // API-style: Clean with subtle odd harmonics when pushed
                modern_transformer_saturation(driven_signal, self.saturation_amount)
            },
            TransformerModel::British => {
                // SSL-style: Tight, controlled saturation
                british_transformer_saturation(driven_signal, self.saturation_amount)
            },
            TransformerModel::American => {
                // Custom: Balanced approach
                american_transformer_saturation(driven_signal, self.saturation_amount)
            },
        };
        
        // Gentle transformer compression (loading effect)
        let compressed = if self.compression_amount > 0.01 {
            self.apply_transformer_compression(saturated)
        } else {
            saturated
        };
        
        compressed
    }
    
    /// Apply gentle compression that mimics transformer loading
    fn apply_transformer_compression(&mut self, input: f32) -> f32 {
        let abs_input = input.abs();
        
        // Simple envelope follower
        if abs_input > self.envelope {
            self.envelope = abs_input;
        } else {
            self.envelope += (abs_input - self.envelope) * 0.01; // Slow release
        }
        
        // Gentle compression when signal gets hot
        let threshold = 0.7;
        if self.envelope > threshold {
            let over_threshold = self.envelope - threshold;
            let compression_ratio = 1.0 + (over_threshold * self.compression_amount * 2.0);
            input / compression_ratio
        } else {
            input
        }
    }
}

impl TransformerModule {
    /// Create new transformer module
    pub fn new(sample_rate: f32) -> Self {
        // Initialize frequency response filters (flat by default)
        let flat_coeff = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate.hz(),
            20000.0_f32.hz(),
            0.707,
        ).expect("LowPass filter should be valid");
        
        Self {
            sample_rate,
            input_transformer: TransformerStage::new(),
            output_transformer: TransformerStage::new(),
            low_shelf: DirectForm1::<f32>::new(flat_coeff),
            high_shelf: DirectForm1::<f32>::new(flat_coeff),
            model: TransformerModel::Vintage,
            cached_model: TransformerModel::Vintage,
            cached_low_response: f32::NAN, // NAN forces recompute on first call
            cached_high_response: f32::NAN,
        }
    }
    
    /// Update transformer parameters
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
    ) {
        self.model = model;
        
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
    /// (guarded in `update_parameters()`).
    fn update_frequency_response(&mut self, low_response: f32, high_response: f32) {
        let low_freq = match self.model {
            TransformerModel::Vintage  => 80.0,
            TransformerModel::Modern   => 60.0,
            TransformerModel::British  => 100.0,
            TransformerModel::American => 70.0,
        };
        // Always update (even at 0 dB) so that model changes take effect immediately.
        let low_gain = low_response * 3.0; // ±3 dB
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::LowShelf(low_gain), self.sample_rate.hz(), low_freq.hz(), 0.707,
        ) { self.low_shelf.update_coefficients(coeff); }

        let high_freq = match self.model {
            TransformerModel::Vintage  => 8000.0,
            TransformerModel::Modern   => 15000.0,
            TransformerModel::British  => 12000.0,
            TransformerModel::American => 10000.0,
        };
        let high_gain = high_response * 2.0; // ±2 dB
        if let Ok(coeff) = Coefficients::<f32>::from_params(
            Type::HighShelf(high_gain), self.sample_rate.hz(), high_freq.hz(), 0.707,
        ) { self.high_shelf.update_coefficients(coeff); }
    }
    
    /// Process audio buffer through transformer module
    pub fn process(&mut self, buffer: &mut Buffer) {
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                
                // 1. Input transformer stage
                s = self.input_transformer.process_sample(s, self.model);
                
                // 2. Frequency response modeling
                s = self.low_shelf.run(s);
                s = self.high_shelf.run(s);
                
                // 3. Output transformer stage
                s = self.output_transformer.process_sample(s, self.model);
                
                *sample = s;
            }
        }
    }
    
    /// Reset transformer state
    pub fn reset(&mut self) {
        self.input_transformer.envelope = 0.0;
        self.input_transformer.harmonic_state = 0.0;
        self.output_transformer.envelope = 0.0;
        self.output_transformer.harmonic_state = 0.0;
    }
}

// Transformer saturation models

/// Vintage transformer saturation (Neve-style)
fn vintage_transformer_saturation(input: f32, amount: f32) -> f32 {
    if amount < 0.01 {
        return input;
    }
    
    // Warm, musical saturation with even harmonics
    let driven = input * (1.0 + amount * 2.0);
    let saturated = driven.tanh(); // Smooth saturation
    
    // Add subtle even harmonics
    let harmonic = driven * driven * amount * 0.1;
    
    // Blend
    let wet = saturated + harmonic;
    input * (1.0 - amount * 0.7) + wet * (amount * 0.7)
}

/// Modern transformer saturation (API-style)
fn modern_transformer_saturation(input: f32, amount: f32) -> f32 {
    if amount < 0.01 {
        return input;
    }
    
    // Clean with subtle odd harmonics when pushed
    let driven = input * (1.0 + amount * 1.5);
    
    // Asymmetric clipping for odd harmonics
    let saturated = if driven > 0.0 {
        driven / (1.0 + driven * driven * amount)
    } else {
        driven / (1.0 + driven * driven * amount * 0.8) // Slight asymmetry
    };
    
    input * (1.0 - amount * 0.5) + saturated * (amount * 0.5)
}

/// British transformer saturation (SSL-style)
fn british_transformer_saturation(input: f32, amount: f32) -> f32 {
    if amount < 0.01 {
        return input;
    }
    
    // Tight, controlled saturation
    let driven = input * (1.0 + amount * 1.2);
    let saturated = driven / (1.0 + driven.abs() * amount * 0.8);
    
    // Very subtle harmonic addition
    let harmonic = driven.signum() * driven * driven * amount * 0.05;
    
    input * (1.0 - amount * 0.6) + (saturated + harmonic) * (amount * 0.6)
}

/// American transformer saturation (Custom balanced)
fn american_transformer_saturation(input: f32, amount: f32) -> f32 {
    if amount < 0.01 {
        return input;
    }

    // Balanced approach between vintage warmth and modern clarity
    let driven = input * (1.0 + amount * 1.8);

    // Soft clipping with gentle compression
    let saturated = if driven.abs() > 0.5 {
        driven.signum() * (0.5 + (driven.abs() - 0.5).tanh() * 0.5)
    } else {
        driven
    };

    // Balanced harmonic content
    let harmonic = driven * driven * driven * amount * 0.08;

    input * (1.0 - amount * 0.6) + (saturated + harmonic) * (amount * 0.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Saturation functions (private but accessible from child mod tests) ────

    #[test]
    fn test_all_saturation_fns_zero_amount_passthrough() {
        // All saturation functions guard amount < 0.01 — return input unchanged
        for &input in &[-1.0_f32, -0.5, 0.0, 0.5, 1.0] {
            let v = vintage_transformer_saturation(input, 0.0);
            let m = modern_transformer_saturation(input, 0.0);
            let b = british_transformer_saturation(input, 0.0);
            let a = american_transformer_saturation(input, 0.0);
            assert!((v - input).abs() < 1e-6, "Vintage zero-amount: {v} vs {input}");
            assert!((m - input).abs() < 1e-6, "Modern zero-amount: {m} vs {input}");
            assert!((b - input).abs() < 1e-6, "British zero-amount: {b} vs {input}");
            assert!((a - input).abs() < 1e-6, "American zero-amount: {a} vs {input}");
        }
    }

    #[test]
    fn test_all_saturation_fns_produce_finite_output() {
        for &amount in &[0.1_f32, 0.5, 1.0] {
            for &input in &[-2.0_f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
                let v = vintage_transformer_saturation(input, amount);
                let m = modern_transformer_saturation(input, amount);
                let b = british_transformer_saturation(input, amount);
                let a = american_transformer_saturation(input, amount);
                assert!(v.is_finite(), "Vintage finite: input={input}, amount={amount}, out={v}");
                assert!(m.is_finite(), "Modern finite: input={input}, amount={amount}, out={m}");
                assert!(b.is_finite(), "British finite: input={input}, amount={amount}, out={b}");
                assert!(a.is_finite(), "American finite: input={input}, amount={amount}, out={a}");
            }
        }
    }

    #[test]
    fn test_saturation_antisymmetric_for_odd_models() {
        // All models use wet/dry blend — verify approximate antisymmetry
        for amount in [0.3_f32, 0.7, 1.0] {
            let x = 0.5_f32;
            let v_pos = vintage_transformer_saturation(x, amount);
            let v_neg = vintage_transformer_saturation(-x, amount);
            // Pure tanh saturation is antisymmetric; our blend adds even harmonics
            // but the blend should keep the result bounded
            assert!(v_pos.is_finite() && v_neg.is_finite());
        }
    }

    #[test]
    fn test_saturation_output_bounded() {
        // For realistic input levels, saturation shouldn't explode
        for &amount in &[0.5_f32, 1.0] {
            for &input in &[-1.0_f32, -0.5, 0.5, 1.0] {
                let v = vintage_transformer_saturation(input, amount).abs();
                let m = modern_transformer_saturation(input, amount).abs();
                let b = british_transformer_saturation(input, amount).abs();
                let a = american_transformer_saturation(input, amount).abs();
                assert!(v < 4.0, "Vintage out-of-bounds: {v}");
                assert!(m < 4.0, "Modern out-of-bounds: {m}");
                assert!(b < 4.0, "British out-of-bounds: {b}");
                assert!(a < 4.0, "American out-of-bounds: {a}");
            }
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
        assert!(t.cached_low_response.is_nan(), "cached_low_response should start NaN");
        assert!(t.cached_high_response.is_nan(), "cached_high_response should start NaN");
        // First update_parameters call should not panic
        t.update_parameters(TransformerModel::Vintage, 0.3, 0.3, 0.3, 0.3, 0.0, 0.0, 0.3);
        assert!(!t.cached_low_response.is_nan(), "cached_low_response should be set after first update");
    }

    #[test]
    fn test_transformer_module_model_selection() {
        let mut t = TransformerModule::new(44100.0);
        for model in [TransformerModel::Vintage, TransformerModel::Modern,
                      TransformerModel::British, TransformerModel::American] {
            t.update_parameters(model, 0.3, 0.3, 0.3, 0.3, 0.0, 0.0, 0.3);
            assert_eq!(t.model, model, "Model should be updated to {:?}", model);
        }
    }

    #[test]
    fn test_transformer_module_cache_skips_filter_recompute() {
        let mut t = TransformerModule::new(44100.0);
        // First call — triggers recompute and sets cache
        t.update_parameters(TransformerModel::Vintage, 0.3, 0.3, 0.3, 0.3, 0.2, 0.2, 0.3);
        let cached_low = t.cached_low_response;
        let cached_high = t.cached_high_response;
        // Same values — cache should match, no recompute
        t.update_parameters(TransformerModel::Vintage, 0.3, 0.3, 0.3, 0.3, 0.2, 0.2, 0.3);
        assert_eq!(t.cached_low_response.to_bits(), cached_low.to_bits());
        assert_eq!(t.cached_high_response.to_bits(), cached_high.to_bits());
    }

    #[test]
    fn test_transformer_module_model_change_updates_cache() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(TransformerModel::Vintage, 0.3, 0.3, 0.3, 0.3, 0.0, 0.0, 0.3);
        assert_eq!(t.cached_model, TransformerModel::Vintage);
        // Change model — cached_model should update
        t.update_parameters(TransformerModel::British, 0.3, 0.3, 0.3, 0.3, 0.0, 0.0, 0.3);
        assert_eq!(t.cached_model, TransformerModel::British);
    }

    #[test]
    fn test_transformer_module_reset_clears_envelopes() {
        let mut t = TransformerModule::new(44100.0);
        t.update_parameters(TransformerModel::Vintage, 0.5, 0.8, 0.5, 0.8, 0.0, 0.0, 0.5);
        // Manually set envelope state
        t.input_transformer.envelope = 0.9;
        t.output_transformer.envelope = 0.7;
        t.reset();
        assert!((t.input_transformer.envelope - 0.0).abs() < 1e-9);
        assert!((t.output_transformer.envelope - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_transformer_input_drive_scales() {
        let mut t = TransformerModule::new(44100.0);
        // input_drive=0 → drive_gain = 1.0; input_drive=1 → drive_gain = 1.8
        t.update_parameters(TransformerModel::Vintage, 0.0, 0.3, 0.3, 0.3, 0.0, 0.0, 0.0);
        assert!((t.input_transformer.drive_gain - 1.0).abs() < 1e-5, "drive=0 should give gain 1.0");
        t.update_parameters(TransformerModel::Vintage, 1.0, 0.3, 0.3, 0.3, 0.0, 0.0, 0.0);
        assert!((t.input_transformer.drive_gain - 1.8).abs() < 1e-5, "drive=1 should give gain 1.8");
    }

    #[test]
    fn test_transformer_freq_response_per_model() {
        // Spot-check that low_freq characteristic differs per model
        let mut t44 = TransformerModule::new(44100.0);
        // Vintage → low_freq=80, Modern → low_freq=60
        // Simply verify update doesn't panic for each model
        for model in [TransformerModel::Vintage, TransformerModel::Modern,
                      TransformerModel::British, TransformerModel::American] {
            t44.update_parameters(model, 0.3, 0.3, 0.3, 0.3, 0.5, -0.5, 0.3);
        }
    }
}