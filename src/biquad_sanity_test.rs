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

    /// Regression pin for biquad 0.5.0's `from_params` frequency-normalization
    /// bug. A +15 dB LowShelf with corner at 60 Hz, measured at 30 Hz (well
    /// below corner), SHOULD boost by ~+15 dB. Instead, `from_params` yields
    /// near-zero gain because it computes `f0/(2*fs)` where the cookbook calls
    /// for `f0/(fs/2)` — the filter sits 4× below its intended corner.
    ///
    /// Marked `#[should_panic]` so the test fails if the upstream crate ever
    /// fixes the bug — at which point we can retire `shaping::biquad_coeffs`.
    #[test]
    #[should_panic(expected = "should boost 30 Hz")]
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
