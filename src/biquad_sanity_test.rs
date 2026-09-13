#[cfg(test)]
mod biquad_sanity {
    use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type};

    fn measure_gain_db(coeff: Coefficients<f32>, freq_hz: f32, sr: f32) -> f32 {
        let mut f = DirectForm1::<f32>::new(coeff);
        let n = 16_384_usize;
        let omega = 2.0 * core::f32::consts::PI * freq_hz / sr;
        let mut peak = 0.0_f32;
        for i in 0..n {
            let x = (omega * i as f32).sin();
            let y = f.run(x);
            if i > n / 2 {
                peak = peak.max(y.abs());
            }
        }
        20.0 * peak.log10()
    }

    /// biquad 0.5.0's `from_params` computed `f0/(2*fs)` instead of the
    /// cookbook `f0/(fs/2)`, placing every corner 4× too low. 0.6.0 fixed it;
    /// this guards against a regression. A +15 dB LowShelf at 60 Hz must boost
    /// 30 Hz by ≥ +12 dB.
    #[test]
    fn biquad_from_params_lowshelf_15db_at_30hz() {
        let sr = 48_000.0_f32;
        let coeff =
            Coefficients::<f32>::from_params(Type::LowShelf(15.0), sr.hz(), 60.0_f32.hz(), 0.9)
                .unwrap();
        let gain = measure_gain_db(coeff, 30.0, sr);
        assert!(
            gain > 12.0,
            "LowShelf +15 dB at 60 Hz should boost 30 Hz by ≥ +12 dB, got {gain:.2} dB"
        );
    }

    /// Same filter measured using from_normalized_params with the "Nyquist = 1"
    /// convention. If THIS passes but from_params fails, we've pinpointed the
    /// frequency-normalization bug in from_params.
    #[test]
    fn biquad_from_normalized_params_lowshelf_15db_at_30hz() {
        let sr = 48_000.0_f32;
        // Nyquist = 1 convention: 60 Hz / (48000/2) = 0.0025
        let normalized = 60.0_f32 / (sr / 2.0);
        let coeff =
            Coefficients::<f32>::from_normalized_params(Type::LowShelf(15.0), normalized, 0.9)
                .unwrap();
        let gain = measure_gain_db(coeff, 30.0, sr);
        assert!(
            gain > 12.0,
            "from_normalized_params LowShelf should boost 30 Hz ≥ +12 dB, got {gain:.2} dB"
        );
    }
}
