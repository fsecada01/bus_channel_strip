//! Trapezoidal-integrated (TPT / ZDF) state-variable filter core.
//!
//! The v2.0 EQ filter topology (#15), replacing `biquad::DirectForm1` in
//! every EQ stage (API5500, Pultec, DynamicEQ, Sheen). Rationale and null
//! tests against the old cores: [ADR-0011](../../docs/adr/0011-tpt-svf-and-pultec-linear-phase.md).
//!
//! Reference: Andrew Simper (Cytomic), "Solving the continuous SVF equations
//! using trapezoidal integration and equivalent currents" —
//! `SvfLinearTrapOptimised2`. Every response type shares the same
//! two-integrator core plus an output mix `(m0, m1, m2)` over
//! `(input, band, low)`.

use realfft::num_complex::Complex;

/// Below this magnitude, state is flushed to exactly zero. IIR state that
/// decays through the f32 subnormal range costs ~100× the normal multiply
/// latency on x86 without FTZ; flushing at 1e-20 is ~-400 dBFS.
///
/// Shared with `dynamic_eq`'s envelope follower, which asymptotes to zero
/// through the same subnormal range and hits the same stall.
pub(crate) const DENORMAL_FLUSH: f32 = 1.0e-20;

/// Highest corner frequency accepted, as a fraction of the sample rate.
/// `tan(π·0.499)` is ~318 — well inside f32 range and still a sane filter.
const MAX_FREQ_RATIO: f32 = 0.499;

/// Lowest corner frequency accepted. Below 1 Hz the prewarped `g` becomes
/// small enough that the shelf/bell gain terms lose precision in f32.
const MIN_FREQ_HZ: f32 = 1.0;

/// Lowest Q accepted. `k = 1/Q` — 40 is already an implausibly wide filter;
/// clamping keeps `k` finite for a zero/negative Q handed in from a bad
/// parameter mapping.
const MIN_Q: f32 = 0.025;

/// RBJ cookbook's sqrt-of-linear-gain convention: `A = 10^(dB/RBJ_GAIN_DIVISOR)`.
const RBJ_GAIN_DIVISOR: f32 = 40.0;

#[inline(always)]
pub(crate) fn flush_denormal(x: f32) -> f32 {
    if x.abs() < DENORMAL_FLUSH {
        0.0
    } else {
        x
    }
}

/// Response type for [`SvfCoefficients::new`]. Gain arguments are in dB.
///
/// Not every variant has a production caller yet (`LowPass`/`Notch` are
/// exercised by the null tests and kept as part of the complete SVF family).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum SvfType {
    LowPass,
    HighPass,
    /// Constant-skirt-gain bandpass (RBJ "BPF, peak gain = Q").
    BandPass,
    /// Constant-0-dB-peak bandpass (RBJ "BPF, constant 0 dB peak gain").
    BandPassUnity,
    Notch,
    Bell(f32),
    LowShelf(f32),
    HighShelf(f32),
}

/// Precomputed SVF coefficients. `a1..a3` drive the integrator core;
/// `m0..m2` mix `(input, band, low)` into the output. `g` and `k` are kept
/// so the analytic response can be evaluated (used by the Pultec
/// linear-phase FIR designer and by the tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvfCoefficients {
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    m0: f32,
    m1: f32,
    m2: f32,
}

impl SvfCoefficients {
    /// Identity (flat, 0 dB) coefficients. The integrator core is left at a
    /// benign `g = 1, k = 1` so a filter constructed flat and later updated
    /// starts from a well-defined state.
    pub fn flat() -> Self {
        Self {
            g: 1.0,
            k: 1.0,
            a1: 1.0 / 3.0,
            a2: 1.0 / 3.0,
            a3: 1.0 / 3.0,
            m0: 1.0,
            m1: 0.0,
            m2: 0.0,
        }
    }

    /// Design coefficients for `filter_type` at `freq_hz` / `q`. Frequency
    /// is clamped to `[MIN_FREQ_HZ, 0.499·fs]` and Q to `>= MIN_Q`, so this
    /// never fails and never produces NaN — callers pass parameter values
    /// straight through.
    pub fn new(filter_type: SvfType, sample_rate: f32, freq_hz: f32, q: f32) -> Self {
        // `.max().min()`, not `.clamp()`: a degenerate sample rate must
        // never invert the clamp bounds and panic on the audio thread.
        let sample_rate = sample_rate.max(2.0 * MIN_FREQ_HZ);
        let max_hz = (sample_rate * MAX_FREQ_RATIO).max(MIN_FREQ_HZ);
        let freq_hz = freq_hz.max(MIN_FREQ_HZ).min(max_hz);
        let q = q.max(MIN_Q);

        // Prewarped integrator gain — tan() of the normalised corner maps
        // the analog prototype's corner exactly onto the digital one.
        let mut g = (core::f32::consts::PI * freq_hz / sample_rate).tan();
        let mut k = 1.0 / q;

        // Output mix over (input, band, low); reproduces the RBJ cookbook
        // prototypes term for term (substitute lp = 1/D, bp = s/D, D = s²+k·s+1).
        let (m0, m1, m2) = match filter_type {
            SvfType::LowPass => (0.0, 0.0, 1.0),
            SvfType::BandPass => (0.0, 1.0, 0.0),
            // band has peak gain Q = 1/k; scaling by k pins the peak to 0 dB.
            SvfType::BandPassUnity => (0.0, k, 0.0),
            SvfType::HighPass => (1.0, -k, -1.0),
            SvfType::Notch => (1.0, -k, 0.0),
            SvfType::Bell(db) => {
                let a = 10.0_f32.powf(db / RBJ_GAIN_DIVISOR);
                k /= a;
                (1.0, k * (a * a - 1.0), 0.0)
            }
            SvfType::LowShelf(db) => {
                let a = 10.0_f32.powf(db / RBJ_GAIN_DIVISOR);
                g /= a.sqrt();
                (1.0, k * (a - 1.0), a * a - 1.0)
            }
            SvfType::HighShelf(db) => {
                let a = 10.0_f32.powf(db / RBJ_GAIN_DIVISOR);
                g *= a.sqrt();
                (a * a, k * (1.0 - a) * a, 1.0 - a * a)
            }
        };

        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        Self {
            g,
            k,
            a1,
            a2,
            a3,
            m0,
            m1,
            m2,
        }
    }

    /// Complex frequency response `H(e^{jω})` at `w` radians/sample.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn response(&self, w: f32) -> Complex<f32> {
        self.response_from_tan(((w * 0.5) as f64).tan() as f32)
    }

    /// Response given `t = tan(ω/2)` directly. The Pultec linear-phase FIR
    /// designer evaluates thousands of bins per redesign and tabulates `t`
    /// once at construction, so the hot loop is arithmetic only.
    ///
    /// The trapezoidal SVF is the bilinear transform of the analog prototype
    /// with `s = (1/g)·(1 − z⁻¹)/(1 + z⁻¹) = j·tan(ω/2)/g`.
    pub fn response_from_tan(&self, t: f32) -> Complex<f32> {
        // At Nyquist tan(ω/2) → ∞: both integrator outputs vanish and only
        // the direct path survives. Take the limit explicitly rather than
        // letting inf/inf produce NaN in the kernel design.
        if !t.is_finite() {
            return Complex::new(self.m0, 0.0);
        }
        let s_im = t / self.g; // s = j·s_im
                               // D = s² + k·s + 1 = (1 − s_im²) + j·k·s_im
        let d = Complex::new(1.0 - s_im * s_im, self.k * s_im);
        let lp = d.inv();
        let bp = Complex::new(0.0, s_im) * lp;
        Complex::new(self.m0, 0.0) + bp * self.m1 + lp * self.m2
    }

    /// Magnitude response in dB at `w` radians/sample.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn magnitude_db(&self, w: f32) -> f32 {
        20.0 * self.response(w).norm().max(f32::MIN_POSITIVE).log10()
    }
}

/// One channel of TPT state-variable filter. Stereo callers keep one per
/// channel (`[TptSvf; 2]`) — sharing a single instance across interleaved
/// L/R samples corrupts the integrator state exactly like it did for DF1.
#[derive(Clone, Copy, Debug)]
pub struct TptSvf {
    coeffs: SvfCoefficients,
    ic1eq: f32,
    ic2eq: f32,
}

impl TptSvf {
    /// Construct with the given coefficients and zeroed state.
    pub fn new(coeffs: SvfCoefficients) -> Self {
        Self {
            coeffs,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    /// Construct a flat (identity) filter.
    pub fn flat() -> Self {
        Self::new(SvfCoefficients::flat())
    }

    /// Replace the coefficients, preserving state. This is the whole point of
    /// the topology: safe to call every sample under automation.
    #[inline]
    pub fn update_coefficients(&mut self, coeffs: SvfCoefficients) {
        self.coeffs = coeffs;
    }

    /// Current coefficients (used by callers that evaluate the response).
    pub fn coefficients(&self) -> SvfCoefficients {
        self.coeffs
    }

    /// Integrator state `(ic1eq, ic2eq)` — exposed for state-preservation
    /// tests; production code has no reason to read it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn state(&self) -> (f32, f32) {
        (self.ic1eq, self.ic2eq)
    }

    /// Process one sample.
    #[inline]
    pub fn run(&mut self, v0: f32) -> f32 {
        let c = &self.coeffs;
        // Trapezoidal (zero-delay-feedback) tick. v1 = band, v2 = low.
        let v3 = v0 - self.ic2eq;
        let v1 = c.a1 * self.ic1eq + c.a2 * v3;
        let v2 = self.ic2eq + c.a2 * self.ic1eq + c.a3 * v3;
        // "Equivalent current" state update: the integrators hold
        // 2·v − previous, which is what makes a coefficient swap land on the
        // new response without a transient.
        self.ic1eq = flush_denormal(2.0 * v1 - self.ic1eq);
        self.ic2eq = flush_denormal(2.0 * v2 - self.ic2eq);
        // Flush the output too: types with a direct-path term (m0) can pass
        // a subnormal `v0` straight through even once this filter's own
        // state has settled to zero, re-triggering the stall downstream.
        flush_denormal(c.m0 * v0 + c.m1 * v1 + c.m2 * v2)
    }

    /// Zero the integrator state.
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shaping::biquad_coeffs;
    use biquad::{Biquad, DirectForm1, Type};

    const SR: f32 = 48_000.0;

    /// Impulse response of a filter over `n` samples.
    fn impulse_svf(f: &mut TptSvf, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| f.run(if i == 0 { 1.0 } else { 0.0 }))
            .collect()
    }

    fn impulse_df1(f: &mut DirectForm1<f32>, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| f.run(if i == 0 { 1.0 } else { 0.0 }))
            .collect()
    }

    /// Steady-state peak gain (dB) of a sine at `freq_hz` through `run`.
    /// Steady-state gain of `run` for a unit sine, in dB, from the RMS of the
    /// second half of the output. RMS rather than sampled peak: at probes
    /// like fs/8 the sample grid only hits eight phases per cycle, so a peak
    /// detector under-reads by up to 0.7 dB depending on the filter's phase.
    fn sine_gain_db<F: FnMut(f32) -> f32>(mut run: F, freq_hz: f32, sr: f32) -> f32 {
        let n = 16_384;
        let w = core::f32::consts::TAU * freq_hz / sr;
        let mut sum_sq = 0.0_f64;
        for i in 0..n {
            let y = run((w * i as f32).sin());
            if i >= n / 2 {
                sum_sq += (y as f64) * (y as f64);
            }
        }
        let rms = (sum_sq / (n / 2) as f64).sqrt();
        20.0 * (rms * core::f64::consts::SQRT_2).log10() as f32
    }

    fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b)
            .fold(0.0_f32, |acc, (x, y)| acc.max((x - y).abs()))
    }

    /// (SVF type, equivalent biquad type, freq, Q) pairs covering every
    /// response the EQ modules use, at moderate settings.
    fn reference_pairs() -> Vec<(SvfType, Type<f32>, f32, f32)> {
        vec![
            (SvfType::Bell(6.0), Type::PeakingEQ(6.0), 1000.0, 0.707),
            (SvfType::Bell(-9.0), Type::PeakingEQ(-9.0), 3000.0, 2.0),
            (SvfType::LowShelf(-4.0), Type::LowShelf(-4.0), 200.0, 0.707),
            (SvfType::LowShelf(15.0), Type::LowShelf(15.0), 100.0, 0.5),
            (SvfType::HighShelf(3.0), Type::HighShelf(3.0), 8000.0, 0.5),
            (
                SvfType::HighShelf(-8.0),
                Type::HighShelf(-8.0),
                10000.0,
                0.9,
            ),
            (SvfType::HighPass, Type::HighPass, 150.0, 0.707),
            (SvfType::LowPass, Type::LowPass, 5000.0, 0.707),
            (SvfType::BandPass, Type::BandPass, 1000.0, 1.0),
            (SvfType::Notch, Type::Notch, 2000.0, 1.0),
        ]
    }

    // ── Null tests against the biquad (RBJ) reference ─────────────────────

    /// Checklist item 2: the TPT core must null against the DirectForm1
    /// reference. TPT is the bilinear transform of the same analog
    /// prototype, so the impulse responses match to f32 rounding — far
    /// tighter than the roadmap's 0.1 dB budget.
    #[test]
    fn svf_impulse_response_nulls_against_biquad_reference() {
        for (svf_ty, bq_ty, freq, q) in reference_pairs() {
            let mut svf = TptSvf::new(SvfCoefficients::new(svf_ty, SR, freq, q));
            let mut bq = DirectForm1::<f32>::new(biquad_coeffs(bq_ty, SR, freq, q).unwrap());
            let a = impulse_svf(&mut svf, 8192);
            let b = impulse_df1(&mut bq, 8192);
            let diff = max_abs_diff(&a, &b);
            assert!(
                diff < 1.0e-4,
                "{svf_ty:?} @ {freq} Hz Q={q}: impulse response differs from biquad by {diff:e}"
            );
        }
    }

    /// Same null, measured as steady-state sine gain at, below, and above
    /// each corner — the way a listener would hear it.
    #[test]
    fn svf_steady_state_gain_nulls_against_biquad_reference() {
        for (svf_ty, bq_ty, freq, q) in reference_pairs() {
            for probe in [freq * 0.25, freq, freq * 4.0] {
                if probe >= SR * 0.45 {
                    continue;
                }
                let mut svf = TptSvf::new(SvfCoefficients::new(svf_ty, SR, freq, q));
                let mut bq = DirectForm1::<f32>::new(biquad_coeffs(bq_ty, SR, freq, q).unwrap());
                let g_svf = sine_gain_db(|x| svf.run(x), probe, SR);
                let g_bq = sine_gain_db(|x| bq.run(x), probe, SR);
                // Skip the notch's own centre — both are ~-inf there and the
                // dB difference of two near-zero peaks is meaningless.
                if g_bq < -60.0 {
                    continue;
                }
                assert!(
                    (g_svf - g_bq).abs() < 0.02,
                    "{svf_ty:?} @ {freq} Hz Q={q}, probe {probe} Hz: svf {g_svf:.3} dB vs biquad {g_bq:.3} dB"
                );
            }
        }
    }

    /// A 0 dB bell/shelf must be bit-transparent, not merely close — the EQ
    /// modules park inactive sections at 0 dB and expect a clean pass.
    #[test]
    fn svf_zero_db_stages_are_transparent() {
        for ty in [
            SvfType::Bell(0.0),
            SvfType::LowShelf(0.0),
            SvfType::HighShelf(0.0),
        ] {
            let mut svf = TptSvf::new(SvfCoefficients::new(ty, SR, 1000.0, 0.707));
            let ir = impulse_svf(&mut svf, 2048);
            assert!((ir[0] - 1.0).abs() < 1.0e-6, "{ty:?}: ir[0] = {}", ir[0]);
            let tail = ir[1..].iter().fold(0.0_f32, |a, &x| a.max(x.abs()));
            assert!(
                tail < 1.0e-6,
                "{ty:?}: 0 dB stage leaves a tail of {tail:e}"
            );
        }
    }

    #[test]
    fn svf_flat_is_identity() {
        let mut svf = TptSvf::flat();
        for &x in &[0.0, 0.5, -0.5, 1.0, -1.0, 0.123_456] {
            assert_eq!(svf.run(x), x);
        }
    }

    // ── Analytic response ─────────────────────────────────────────────────

    /// The analytic `response()` drives the Pultec linear-phase FIR design,
    /// so it must agree with what the recursion actually does.
    #[test]
    fn svf_analytic_response_matches_measured_gain() {
        for (svf_ty, _, freq, q) in reference_pairs() {
            let coeffs = SvfCoefficients::new(svf_ty, SR, freq, q);
            for probe in [freq * 0.5, freq, freq * 2.0] {
                if probe >= SR * 0.45 {
                    continue;
                }
                let predicted = coeffs.magnitude_db(core::f32::consts::TAU * probe / SR);
                if predicted < -60.0 {
                    continue;
                }
                let mut svf = TptSvf::new(coeffs);
                let measured = sine_gain_db(|x| svf.run(x), probe, SR);
                assert!(
                    (predicted - measured).abs() < 0.05,
                    "{svf_ty:?} @ {freq} Hz Q={q}, probe {probe} Hz: analytic {predicted:.3} dB vs measured {measured:.3} dB"
                );
            }
        }
    }

    #[test]
    fn svf_bell_response_peaks_at_requested_gain() {
        let coeffs = SvfCoefficients::new(SvfType::Bell(6.0), SR, 1000.0, 1.0);
        let w0 = core::f32::consts::TAU * 1000.0 / SR;
        assert!((coeffs.magnitude_db(w0) - 6.0).abs() < 0.01);
        // Far away from the bell the response is back at 0 dB.
        assert!(coeffs.magnitude_db(w0 * 0.02).abs() < 0.05);
        assert!(coeffs.magnitude_db(w0 * 20.0).abs() < 0.2);
    }

    // ── Topology properties that motivated the migration ─────────────────

    /// Extreme Q must stay finite and bounded — the DF1 recursion can blow
    /// up from coefficient rounding here; the trapezoidal one cannot.
    #[test]
    fn svf_is_stable_at_extreme_q() {
        for ty in [SvfType::Bell(18.0), SvfType::LowPass, SvfType::BandPass] {
            let mut svf = TptSvf::new(SvfCoefficients::new(ty, SR, 5000.0, 40.0));
            let mut peak = 0.0_f32;
            for i in 0..200_000 {
                // Deterministic broadband excitation.
                let x = ((i as f32 * 12.9898).sin() * 43758.547).fract() - 0.5;
                let y = svf.run(x);
                assert!(y.is_finite(), "{ty:?}: non-finite output at sample {i}");
                peak = peak.max(y.abs());
            }
            // Q=40 at +18 dB is at most ~+18 dB of resonant gain on a
            // half-scale signal; anything past 1e3 means the recursion ran away.
            assert!(peak < 1.0e3, "{ty:?}: peak {peak} at Q=40");
        }
        // A high-Q resonator must also ring *down*, not sustain.
        let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::LowPass, SR, 1000.0, 50.0));
        let ir = impulse_svf(&mut svf, 100_000);
        let late = ir[90_000..].iter().fold(0.0_f32, |a, &x| a.max(x.abs()));
        assert!(late < 1.0e-6, "Q=50 lowpass still ringing at {late:e}");
    }

    /// Updating coefficients must not touch the integrator state — the
    /// DynamicEQ invariant (previously enforced on BiquadPeak's x1/y1).
    #[test]
    fn svf_update_coefficients_preserves_state() {
        let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::Bell(6.0), SR, 1000.0, 1.0));
        for _ in 0..100 {
            svf.run(0.7);
        }
        let before = svf.state();
        assert!(
            before.0 != 0.0 || before.1 != 0.0,
            "state should be non-zero after DC"
        );
        svf.update_coefficients(SvfCoefficients::new(SvfType::Bell(-3.0), SR, 2000.0, 1.5));
        assert_eq!(svf.state(), before);
    }

    /// Re-applying identical coefficients mid-stream must be a bit-exact
    /// no-op on the output.
    #[test]
    fn svf_reapplying_same_coefficients_is_a_noop() {
        let coeffs = SvfCoefficients::new(SvfType::LowShelf(4.0), SR, 120.0, 0.707);
        let mut a = TptSvf::new(coeffs);
        let mut b = TptSvf::new(coeffs);
        for i in 0..4096 {
            let x = (i as f32 * 0.05).sin();
            let ya = a.run(x);
            b.update_coefficients(coeffs);
            let yb = b.run(x);
            assert_eq!(ya, yb, "sample {i}");
        }
    }

    /// Per-sample coefficient modulation (the DynamicEQ case) must not
    /// produce transient bursts: a 200 Hz → 8 kHz lowpass sweep with Q=8 on
    /// a full-scale sine stays under the filter's own resonant gain.
    #[test]
    fn svf_per_sample_modulation_is_bounded() {
        let q = 8.0_f32;
        let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::LowPass, SR, 200.0, q));
        let n = 48_000;
        let mut peak = 0.0_f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let fc = 200.0 * (8000.0_f32 / 200.0).powf(t);
            svf.update_coefficients(SvfCoefficients::new(SvfType::LowPass, SR, fc, q));
            let x = (core::f32::consts::TAU * 1000.0 * i as f32 / SR).sin();
            let y = svf.run(x);
            assert!(y.is_finite(), "non-finite at sample {i}");
            peak = peak.max(y.abs());
        }
        // Resonant gain of a Q=8 lowpass is 8× (18 dB); anything well past
        // that is a modulation artefact rather than the filter's response.
        assert!(peak < q * 1.25, "modulation burst: peak {peak} (Q={q})");
    }

    #[test]
    fn svf_reset_zeroes_state() {
        let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::Bell(6.0), SR, 1000.0, 1.0));
        for _ in 0..100 {
            svf.run(1.0);
        }
        svf.reset();
        assert_eq!(svf.state(), (0.0, 0.0));
    }

    // ── Input hygiene ─────────────────────────────────────────────────────

    #[test]
    fn svf_clamps_out_of_range_frequency_and_q() {
        for (freq, q) in [
            (30_000.0, 1.0),
            (0.0, 1.0),
            (-50.0, 1.0),
            (1000.0, 0.0),
            (1000.0, -3.0),
        ] {
            let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::Bell(6.0), 44_100.0, freq, q));
            for i in 0..1000 {
                let y = svf.run(if i % 2 == 0 { 1.0 } else { -1.0 });
                assert!(y.is_finite(), "freq={freq} q={q}: non-finite output {y}");
            }
        }
    }

    #[test]
    fn svf_denormal_tail_flushes_to_zero() {
        let mut svf = TptSvf::new(SvfCoefficients::new(SvfType::LowPass, SR, 100.0, 0.707));
        svf.run(1.0);
        for _ in 0..500_000 {
            svf.run(0.0);
        }
        let (s1, s2) = svf.state();
        assert!(
            s1 == 0.0 && s2 == 0.0,
            "state did not flush: ({s1:e}, {s2:e})"
        );
    }
}
