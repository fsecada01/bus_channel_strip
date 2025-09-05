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
    
    // Frequency response filters
    low_shelf: DirectForm1<f32>,    // Low frequency response
    high_shelf: DirectForm1<f32>,   // High frequency response
    
    // Transformer model
    model: TransformerModel,
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
        
        // Update frequency response based on transformer model
        self.update_frequency_response(low_frequency_response, high_frequency_response);
    }
    
    /// Update frequency response characteristics
    fn update_frequency_response(&mut self, low_response: f32, high_response: f32) {
        // Low frequency shelf (transformer low-end response)
        let low_freq = match self.model {
            TransformerModel::Vintage => 80.0,   // Warmer low end
            TransformerModel::Modern => 60.0,    // Extended low end
            TransformerModel::British => 100.0,  // Tighter low end
            TransformerModel::American => 70.0,  // Balanced
        };
        
        let low_gain = low_response * 3.0; // ±3dB
        if low_gain.abs() > 0.1 {
            let low_coeff = Coefficients::<f32>::from_params(
                Type::LowShelf(low_gain),
                self.sample_rate.hz(),
                low_freq.hz(),
                0.707,
            ).expect("Low shelf should be valid");
            self.low_shelf = DirectForm1::<f32>::new(low_coeff);
        }
        
        // High frequency shelf (transformer high-end response)
        let high_freq = match self.model {
            TransformerModel::Vintage => 8000.0,  // Gentle high roll-off
            TransformerModel::Modern => 15000.0,  // Extended high end
            TransformerModel::British => 12000.0, // Crisp but controlled
            TransformerModel::American => 10000.0, // Balanced
        };
        
        let high_gain = high_response * 2.0; // ±2dB
        if high_gain.abs() > 0.1 {
            let high_coeff = Coefficients::<f32>::from_params(
                Type::HighShelf(high_gain),
                self.sample_rate.hz(),
                high_freq.hz(),
                0.707,
            ).expect("High shelf should be valid");
            self.high_shelf = DirectForm1::<f32>::new(high_coeff);
        }
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