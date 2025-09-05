use nih_plug::buffer::Buffer;
use realfft::num_complex::Complex;
use realfft::{RealToComplex};
use std::sync::Arc;

pub fn analyze_spectrum(
    buffer: &mut Buffer,
    fft_size: usize,
    fft: &Arc<dyn RealToComplex<f32>>,
    fft_input: &mut Vec<f32>,
    fft_spectrum: &mut Vec<Complex<f32>>,
) -> Vec<f32> {
    let mut idx = 0;
    for mut frame in buffer.iter_samples() {
        if idx >= fft_size {
            break;
        }
        fft_input[idx] = frame.iter_mut().next().map(|s| *s).unwrap_or(0.0);
        idx += 1;
    }
    fft_input[idx..].fill(0.0);
    fft.process(fft_input, fft_spectrum).unwrap();
    fft_spectrum.iter().take(fft_spectrum.len()).map(|c| c.norm()).collect()
}
