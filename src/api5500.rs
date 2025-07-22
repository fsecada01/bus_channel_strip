//! API 5500–style 5‑band semi‑parametric EQ module stub.
//!
//! This struct will host the DSP filters and state for an API 5500–style channel EQ.

use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type, Q_BUTTERWORTH_F32};
use nih_plug::buffer::Buffer;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// API 5500–style 5‑band EQ stub.
/// API 5500–style 5‑band semi‑parametric EQ stub.
///
/// Contains one biquad per band: LF, LMF, MF, HMF, and HF.
/// API 5500–style 5‑band semi‑parametric EQ stub with optional spectral analysis.
pub struct Api5500 {
    #[allow(dead_code)]
    sample_rate: f32,
    lf: DirectForm1<f32>,
    lmf: DirectForm1<f32>,
    mf: DirectForm1<f32>,
    hmf: DirectForm1<f32>,
    hf: DirectForm1<f32>,
    // FFT state for spectral analysis or future FFT‑based filters
    fft_size: usize,
    fft: Arc<dyn RealToComplex<f32>>,
    #[allow(dead_code)]
    ifft: Arc<dyn ComplexToReal<f32>>,
    fft_input: Vec<f32>,
    fft_spectrum: Vec<Complex<f32>>,
    #[allow(dead_code)]
    fft_output: Vec<f32>,
}

impl Api5500 {
    /// Create a new API 5500 EQ with the given sample rate.
    pub fn new(sample_rate: f32) -> Self {
        // Start with all-pass (flat) filters at 1 kHz
        let coeff = Coefficients::<f32>::from_params(
            Type::AllPass,
            sample_rate.hz(),
            1000.0_f32.hz(),
            Q_BUTTERWORTH_F32,
        )
        .expect("AllPass filter parameters should be valid");
        let df = DirectForm1::<f32>::new(coeff);
        // Initialize FFT for spectral analysis (1024-point)
        let fft_size = 1024;
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);
        let fft_input = vec![0.0; fft_size];
        let fft_spectrum = fft.make_output_vec();
        let fft_output = ifft.make_output_vec();
        Self {
            sample_rate,
            lf: df,
            lmf: df,
            mf: df,
            hmf: df,
            hf: df,
            fft_size,
            fft,
            ifft,
            fft_input,
            fft_spectrum,
            fft_output,
        }
    }

    /// Process the EQ on the audio buffer (in place).
    pub fn process(&mut self, buffer: &mut Buffer) {
        // 1) Input-stage biquads in series:
        for samples in buffer.iter_samples() {
            for sample in samples {
                let mut s = *sample;
                s = self.lf.run(s);
                s = self.lmf.run(s);
                s = self.mf.run(s);
                s = self.hmf.run(s);
                s = self.hf.run(s);
                *sample = s;
            }
        }
        // 2) Optional spectral analysis
        let _spectrum = self.analyze_spectrum(buffer);
    }

    /// Compute magnitude spectrum of channel 0 via FFT.
    pub fn analyze_spectrum(&mut self, buffer: &mut Buffer) -> Vec<f32> {
        // Copy first channel into FFT input (zero-pad/truncate)
        let mut idx = 0;
        for mut frame in buffer.iter_samples() {
            if idx >= self.fft_size {
                break;
            }
            // grab first channel sample or zero if unavailable
            self.fft_input[idx] = frame.iter_mut().next().map(|s| *s).unwrap_or(0.0);
            idx += 1;
        }
        self.fft_input[idx..].fill(0.0);
        // Forward FFT
        self.fft
            .process(&mut self.fft_input, &mut self.fft_spectrum)
            .unwrap();
        // Magnitude (half-spectrum)
        self.fft_spectrum
            .iter()
            .take(self.fft_spectrum.len())
            .map(|c| c.norm())
            .collect()
    }
}
