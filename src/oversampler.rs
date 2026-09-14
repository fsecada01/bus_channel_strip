//! Cascaded halfband FIR oversampler — shared by the Punch clipper and by
//! the saturation-bearing modules (Transformer, Pultec tube stage, FET
//! all-buttons). Each 2× stage is a 23-tap Kaiser-windowed halfband FIR
//! (β=8.0). Cascaded log₂(factor) times for 2×/4×/8×/16×.
//! [`Oversampler::new_steep`] swaps the first stage for a 127-tap filter.
//!
//! The implementation is strictly audio-thread safe: filter state is
//! fixed-size arrays, and the only heap usage is a pair of
//! `Vec<f32>` scratch buffers pre-allocated at construction time.

pub const HB_NUM_TAPS: usize = 23;
pub const MAX_OS_STAGES: usize = 4; // 2^4 = 16× max

/// Taps of the first (base ↔ 2×) stage of a [`Oversampler::new_steep`] cascade. That stage alone
/// sets the passband edge and the rejection of images folding back into the audio band; 127 taps
/// at β=9 keep 44.1 kHz flat to 20 kHz with images landing in 0–20 kHz attenuated ~90 dB, where
/// the 23-tap stage is −6 dB at 20 kHz and lets images through at −11 dB.
pub const STEEP_HB_NUM_TAPS: usize = 127;
const STEEP_HB_BETA: f32 = 9.0;

/// Modified Bessel function of the first kind, order 0.
/// Series expansion — called at init time only.
fn bessel_i0(x: f32) -> f32 {
    let mut sum = 1.0_f32;
    let mut term = 1.0_f32;
    let q = (x * x) * 0.25;
    for k in 1..60 {
        let k_f = k as f32;
        term *= q / (k_f * k_f);
        sum += term;
        if sum.abs() > 0.0 && term.abs() / sum.abs() < 1.0e-9 {
            break;
        }
    }
    sum
}

/// Design a Kaiser-windowed halfband FIR (HB_NUM_TAPS, odd).
/// β=8.0 → ~-40 dB stopband for 23 taps (a big upgrade from linear-interp
/// / boxcar which rejects barely anything). Coefficients are normalized to
/// unity DC gain.
pub fn design_halfband_kaiser(beta: f32) -> [f32; HB_NUM_TAPS] {
    design_halfband_kaiser_taps(beta)
}

/// [`design_halfband_kaiser`] for any odd tap count `N`.
pub fn design_halfband_kaiser_taps<const N: usize>(beta: f32) -> [f32; N] {
    let mut coeffs = [0.0_f32; N];
    let m = (N - 1) as f32;
    let center = (N - 1) / 2;
    let denom = bessel_i0(beta);
    let pi = core::f32::consts::PI;

    for n in 0..N {
        let offset = n as i32 - center as i32;

        let ideal = if offset == 0 {
            0.5
        } else if offset.unsigned_abs() % 2 == 0 {
            0.0
        } else {
            let arg = offset as f32 * pi * 0.5;
            arg.sin() / (offset as f32 * pi)
        };

        let normalized = (2.0 * n as f32 - m) / m;
        let w_arg = 1.0 - normalized * normalized;
        let window = if w_arg >= 0.0 {
            bessel_i0(beta * w_arg.sqrt()) / denom
        } else {
            0.0
        };

        coeffs[n] = ideal * window;
    }

    let sum: f32 = coeffs.iter().sum();
    if sum.abs() > f32::MIN_POSITIVE {
        for c in &mut coeffs {
            *c /= sum;
        }
    }
    coeffs
}

/// Single halfband FIR stage: holds a circular delay line over `N`
/// samples at the filter's operating rate (the higher of the two rates the
/// stage bridges).
#[derive(Clone)]
pub struct HalfbandFir<const N: usize = HB_NUM_TAPS> {
    delay: [f32; N],
    pos: usize,
}

impl<const N: usize> HalfbandFir<N> {
    pub fn new() -> Self {
        Self {
            delay: [0.0; N],
            pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.delay = [0.0; N];
        self.pos = 0;
    }

    #[inline]
    fn convolve(&self, coeffs: &[f32; N]) -> f32 {
        let mut sum = 0.0_f32;
        let mut read = self.pos;
        for k in 0..N {
            sum += coeffs[k] * self.delay[read];
            read = if read == 0 { N - 1 } else { read - 1 };
        }
        sum
    }

    #[inline]
    fn advance(&mut self) {
        self.pos = if self.pos + 1 == N { 0 } else { self.pos + 1 };
    }

    /// 2× upsample of one input sample → two output samples.
    /// Zero-stuff + FIR filter + ×2 gain compensation for the zero-stuff energy loss.
    #[inline]
    pub fn upsample_2x(&mut self, x: f32, coeffs: &[f32; N]) -> (f32, f32) {
        self.delay[self.pos] = x;
        let y0 = self.convolve(coeffs);
        self.advance();

        self.delay[self.pos] = 0.0;
        let y1 = self.convolve(coeffs);
        self.advance();

        (y0 * 2.0, y1 * 2.0)
    }

    /// 2× downsample of two input samples → one output sample.
    #[inline]
    pub fn downsample_2x(&mut self, y0: f32, y1: f32, coeffs: &[f32; N]) -> f32 {
        self.delay[self.pos] = y0;
        let x = self.convolve(coeffs);
        self.advance();

        self.delay[self.pos] = y1;
        self.advance();

        x
    }
}

impl<const N: usize> Default for HalfbandFir<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Cascaded halfband FIR oversampler.
/// `up_stages[0]` operates at the base rate (1× → 2×), `up_stages[n-1]` at the
/// highest rate (factor/2 → factor). `down_stages` mirror this.
pub struct Oversampler {
    factor: usize,
    /// Upper bound on `factor` for this instance's lifetime; `upsample_buffer`
    /// is sized to this instead of the global `MAX_OS_STAGES` ceiling.
    max_factor: usize,
    num_stages: usize,
    hb_coeffs: [f32; HB_NUM_TAPS],
    up_stages: [HalfbandFir; MAX_OS_STAGES],
    down_stages: [HalfbandFir; MAX_OS_STAGES],
    /// When set, `steep_up`/`steep_down` replace `up_stages[0]`/`down_stages[0]`.
    steep_first_stage: bool,
    /// All zeros unless `steep_first_stage`.
    steep_coeffs: [f32; STEEP_HB_NUM_TAPS],
    steep_up: HalfbandFir<STEEP_HB_NUM_TAPS>,
    steep_down: HalfbandFir<STEEP_HB_NUM_TAPS>,
    upsample_buffer: Vec<f32>,
    /// Empty (no allocation) for an `Oversampler` built via
    /// `new_upsample_only` — such an instance must never call `downsample`.
    downsample_buffer: Vec<f32>,
}

impl Oversampler {
    fn new_inner(max_factor: usize, max_block_size: usize, needs_downsample: bool) -> Self {
        let max_factor = max_factor.clamp(1, 1 << MAX_OS_STAGES);
        Self {
            factor: 1,
            max_factor,
            num_stages: 0,
            hb_coeffs: design_halfband_kaiser(8.0),
            steep_first_stage: false,
            steep_coeffs: [0.0; STEEP_HB_NUM_TAPS],
            steep_up: HalfbandFir::new(),
            steep_down: HalfbandFir::new(),
            up_stages: [
                HalfbandFir::new(),
                HalfbandFir::new(),
                HalfbandFir::new(),
                HalfbandFir::new(),
            ],
            down_stages: [
                HalfbandFir::new(),
                HalfbandFir::new(),
                HalfbandFir::new(),
                HalfbandFir::new(),
            ],
            upsample_buffer: vec![0.0; max_block_size * max_factor],
            downsample_buffer: if needs_downsample {
                vec![0.0; max_block_size]
            } else {
                Vec::new()
            },
        }
    }

    /// `max_factor` should be the largest factor this instance will ever be
    /// switched to via `set_factor`, not just its starting factor.
    pub fn new(max_factor: usize, max_block_size: usize) -> Self {
        Self::new_inner(max_factor, max_block_size, true)
    }

    /// Like [`Oversampler::new`], but with a [`STEEP_HB_NUM_TAPS`]-tap first stage: flat to
    /// 20 kHz and far stronger image rejection, for ~52 extra base-rate samples of latency.
    pub fn new_steep(max_factor: usize, max_block_size: usize) -> Self {
        let mut os = Self::new_inner(max_factor, max_block_size, true);
        os.steep_first_stage = true;
        os.steep_coeffs = design_halfband_kaiser_taps(STEEP_HB_BETA);
        os
    }

    /// Construct an `Oversampler` already set to `factor`. Equivalent to
    /// `Oversampler::new(factor, max_block_size)` followed by
    /// `set_factor(factor)`, but removes the two-step footgun: `new()` alone
    /// leaves `factor` at 1×/passthrough, so a call site that forgets the
    /// follow-up `set_factor` call silently gets no oversampling at all.
    pub fn new_at_factor(factor: usize, max_block_size: usize) -> Self {
        let mut os = Self::new(factor, max_block_size);
        os.set_factor(factor);
        os
    }

    /// Like `new_at_factor`, but for a caller that will only ever call
    /// `upsample()` (e.g. a metering tap) — skips the downsample buffer.
    /// Calling `downsample()` on the result panics (empty buffer).
    pub fn new_upsample_only(factor: usize, max_block_size: usize) -> Self {
        let mut os = Self::new_inner(factor, max_block_size, false);
        os.set_factor(factor);
        os
    }

    pub fn set_factor(&mut self, factor: usize) {
        let factor = factor.min(self.max_factor);
        let new_num_stages = match factor {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            16 => 4,
            _ => 0,
        };
        if new_num_stages != self.num_stages {
            self.reset_stages();
        }
        self.factor = factor;
        self.num_stages = new_num_stages;
    }

    fn reset_stages(&mut self) {
        for s in self.up_stages.iter_mut().chain(self.down_stages.iter_mut()) {
            s.reset();
        }
        self.steep_up.reset();
        self.steep_down.reset();
    }

    /// Group delay of an `upsample` → `downsample` round trip at the current factor, in
    /// base-rate samples. Fractional at 4× and above (e.g. 16.5 for the standard 4× cascade).
    pub fn latency_samples(&self) -> f32 {
        if self.num_stages == 0 {
            return 0.0;
        }
        let first_taps = if self.steep_first_stage {
            STEEP_HB_NUM_TAPS
        } else {
            HB_NUM_TAPS
        };
        // Each 2× stage delays by (taps − 1) / 2 samples at its upper rate on the way up and
        // again on the way down.
        (first_taps - 1) as f32 * 0.5
            + (1..self.num_stages)
                .map(|stage| (HB_NUM_TAPS - 1) as f32 / (1 << (stage + 1)) as f32)
                .sum::<f32>()
    }

    #[allow(dead_code)]
    pub fn factor(&self) -> usize {
        self.factor
    }

    #[allow(dead_code)]
    pub fn num_stages(&self) -> usize {
        self.num_stages
    }

    /// Upsample a single input sample to `factor` output samples. Writes them
    /// into `upsample_buffer[idx*factor .. (idx+1)*factor]` and returns that
    /// slice.
    #[inline]
    pub fn upsample(&mut self, input: f32, idx: usize) -> &[f32] {
        let start = idx * self.factor;
        let end = start + self.factor;

        if self.num_stages == 0 {
            self.upsample_buffer[start] = input;
            return &self.upsample_buffer[start..end];
        }

        let mut buf_a = [0.0_f32; 1 << MAX_OS_STAGES];
        let mut buf_b = [0.0_f32; 1 << MAX_OS_STAGES];
        buf_a[0] = input;
        let mut count = 1_usize;

        for stage_idx in 0..self.num_stages {
            let steep = stage_idx == 0 && self.steep_first_stage;
            let (src, dst) = if stage_idx % 2 == 0 {
                (&buf_a, &mut buf_b)
            } else {
                (&buf_b, &mut buf_a)
            };
            for i in 0..count {
                let (y0, y1) = if steep {
                    self.steep_up.upsample_2x(src[i], &self.steep_coeffs)
                } else {
                    self.up_stages[stage_idx].upsample_2x(src[i], &self.hb_coeffs)
                };
                dst[2 * i] = y0;
                dst[2 * i + 1] = y1;
            }
            count *= 2;
        }

        let out = if self.num_stages % 2 == 0 {
            &buf_a[..count]
        } else {
            &buf_b[..count]
        };
        self.upsample_buffer[start..end].copy_from_slice(out);
        &self.upsample_buffer[start..end]
    }

    /// Downsample `factor` input samples to a single output sample.
    #[inline]
    pub fn downsample(&mut self, processed: &[f32], idx: usize) -> f32 {
        if self.num_stages == 0 {
            let r = processed[0];
            self.downsample_buffer[idx] = r;
            return r;
        }

        let mut buf_a = [0.0_f32; 1 << MAX_OS_STAGES];
        let mut buf_b = [0.0_f32; 1 << MAX_OS_STAGES];
        let mut count = processed.len();
        buf_a[..count].copy_from_slice(processed);

        for stage_idx in 0..self.num_stages {
            let stage = self.num_stages - 1 - stage_idx;
            let steep = stage == 0 && self.steep_first_stage;
            let (src, dst) = if stage_idx % 2 == 0 {
                (&buf_a, &mut buf_b)
            } else {
                (&buf_b, &mut buf_a)
            };
            let new_count = count / 2;
            for i in 0..new_count {
                let (y0, y1) = (src[2 * i], src[2 * i + 1]);
                dst[i] = if steep {
                    self.steep_down.downsample_2x(y0, y1, &self.steep_coeffs)
                } else {
                    self.down_stages[stage].downsample_2x(y0, y1, &self.hb_coeffs)
                };
            }
            count = new_count;
        }

        let result = if self.num_stages % 2 == 0 {
            buf_a[0]
        } else {
            buf_b[0]
        };
        self.downsample_buffer[idx] = result;
        result
    }

    pub fn reset(&mut self) {
        self.reset_stages();
        self.downsample_buffer.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_upsample_only_matches_new_at_factor_output() {
        let mut os_full = Oversampler::new_at_factor(4, 64);
        let mut os_lean = Oversampler::new_upsample_only(4, 64);

        for i in 0..64 {
            let x = (i as f32 * 0.1).sin();
            let full = os_full.upsample(x, i).to_vec();
            let lean = os_lean.upsample(x, i).to_vec();
            assert_eq!(
                full, lean,
                "new_upsample_only output diverged from new_at_factor at sample {i}"
            );
        }
    }

    #[test]
    fn test_set_factor_clamps_to_max_factor() {
        let mut os = Oversampler::new_upsample_only(4, 16);
        os.set_factor(16);
        assert_eq!(os.factor(), 4, "set_factor should clamp to max_factor");

        for i in 0..16 {
            os.upsample(0.0, i); // must not panic (OOB if the clamp were missing)
        }
    }

    #[test]
    fn test_bessel_i0_at_zero_is_one() {
        // I0(0) = 1 by definition (the series' only surviving term).
        assert!((bessel_i0(0.0) - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn test_halfband_kaiser_has_unity_dc_gain() {
        let coeffs = design_halfband_kaiser(8.0);
        let sum: f32 = coeffs.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1.0e-5,
            "halfband FIR must be normalized to unity DC gain, got {sum}"
        );
    }

    #[test]
    fn test_halfband_kaiser_is_linear_phase_symmetric() {
        // A Kaiser-windowed FIR is symmetric about its center tap (linear
        // phase) — required so the cascade doesn't introduce frequency-
        // dependent phase distortion into the oversampled signal.
        let coeffs = design_halfband_kaiser(8.0);
        for n in 0..HB_NUM_TAPS {
            assert!(
                (coeffs[n] - coeffs[HB_NUM_TAPS - 1 - n]).abs() < 1.0e-7,
                "tap {n} ({}) should mirror tap {} ({})",
                coeffs[n],
                HB_NUM_TAPS - 1 - n,
                coeffs[HB_NUM_TAPS - 1 - n]
            );
        }
    }

    #[test]
    fn test_halfband_fir_reset_clears_delay_line() {
        let coeffs = design_halfband_kaiser(8.0);
        let mut warmed = HalfbandFir::new();
        for _ in 0..HB_NUM_TAPS {
            warmed.upsample_2x(1.0, &coeffs);
        }
        warmed.reset();

        let mut fresh = HalfbandFir::new();
        for _ in 0..HB_NUM_TAPS {
            let a = warmed.upsample_2x(0.3, &coeffs);
            let b = fresh.upsample_2x(0.3, &coeffs);
            assert_eq!(
                a, b,
                "reset() should make state identical to a fresh instance"
            );
        }
    }

    #[test]
    fn test_oversampler_factor_1_is_passthrough() {
        let mut os = Oversampler::new_at_factor(1, 8);
        assert_eq!(os.num_stages(), 0);

        let up = os.upsample(0.42, 0).to_vec();
        assert_eq!(
            up,
            vec![0.42],
            "factor=1 upsample must pass the sample through unchanged"
        );

        let down = os.downsample(&up, 0);
        assert_eq!(
            down, 0.42,
            "factor=1 downsample must pass the sample through unchanged"
        );
    }

    #[test]
    fn test_oversampler_dc_round_trip_converges_to_input() {
        // Feeding a constant DC value through upsample -> downsample with no
        // processing in between should reconstruct that DC value once the
        // FIR cascade's delay lines have filled (steady state) — the whole
        // point of a unity-DC-gain halfband cascade.
        for factor in [2usize, 4, 8, 16] {
            let mut os = Oversampler::new_at_factor(factor, 1);
            let dc = 0.5_f32;
            let mut last = 0.0_f32;
            for i in 0..(HB_NUM_TAPS * 4) {
                let up = os.upsample(dc, 0).to_vec();
                last = os.downsample(&up, 0);
                let _ = i;
            }
            assert!(
                (last - dc).abs() < 1.0e-3,
                "factor={factor}: steady-state DC round trip should converge to {dc}, got {last}"
            );
        }
    }

    #[test]
    fn test_oversampler_all_factors_produce_finite_output() {
        // Exercises both the even (factor 4, 16) and odd (factor 2, 8)
        // num_stages ping-pong buffer paths in upsample()/downsample().
        for factor in [1usize, 2, 4, 8, 16] {
            let mut os = Oversampler::new_at_factor(factor, 4);
            for i in 0..4 {
                let x = (i as f32 * 0.7).sin();
                let up = os.upsample(x, i).to_vec();
                assert_eq!(up.len(), factor);
                assert!(
                    up.iter().all(|s| s.is_finite()),
                    "factor={factor}: NaN/Inf in upsample output"
                );

                let down = os.downsample(&up, i);
                assert!(
                    down.is_finite(),
                    "factor={factor}: NaN/Inf in downsample output"
                );
            }
        }
    }

    #[test]
    fn test_oversampler_reset_clears_filter_state() {
        let mut os = Oversampler::new_at_factor(4, 1);
        for _ in 0..HB_NUM_TAPS {
            let up = os.upsample(1.0, 0).to_vec();
            os.downsample(&up, 0);
        }
        os.reset();

        let mut fresh = Oversampler::new_at_factor(4, 1);
        let up_reset = os.upsample(0.25, 0).to_vec();
        let up_fresh = fresh.upsample(0.25, 0).to_vec();
        assert_eq!(
            up_reset, up_fresh,
            "reset() should make state identical to a fresh instance"
        );
    }

    fn round_trip_impulse_response(os: &mut Oversampler, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| {
                let up = os.upsample(if n == 0 { 1.0 } else { 0.0 }, 0).to_vec();
                os.downsample(&up, 0)
            })
            .collect()
    }

    /// |H| in dB of an impulse response at `freq_hz`.
    fn response_db(ir: &[f32], freq_hz: f32, sample_rate: f32) -> f32 {
        let w = core::f32::consts::TAU * freq_hz / sample_rate;
        let (re, im) = ir
            .iter()
            .enumerate()
            .fold((0.0_f64, 0.0_f64), |(re, im), (n, &h)| {
                let phase = (w * n as f32) as f64;
                (re + h as f64 * phase.cos(), im - h as f64 * phase.sin())
            });
        (10.0 * (re * re + im * im).log10()) as f32
    }

    #[test]
    fn test_latency_samples_matches_round_trip_group_delay() {
        for steep in [false, true] {
            for factor in [1usize, 2, 4, 8, 16] {
                let mut os = if steep {
                    Oversampler::new_steep(factor, 1)
                } else {
                    Oversampler::new(factor, 1)
                };
                os.set_factor(factor);
                // A linear-phase cascade's impulse response is symmetric about its group delay,
                // so its centroid is exactly that delay.
                let ir = round_trip_impulse_response(&mut os, 400);
                let centroid = ir
                    .iter()
                    .enumerate()
                    .map(|(n, &h)| n as f32 * h)
                    .sum::<f32>()
                    / ir.iter().sum::<f32>();
                assert!(
                    (centroid - os.latency_samples()).abs() < 0.01,
                    "steep={steep} {factor}x: latency_samples() = {} but group delay is {centroid}",
                    os.latency_samples()
                );
            }
        }
    }

    #[test]
    fn test_steep_first_stage_is_flat_to_20khz_at_44k() {
        let sr = 44_100.0;
        for factor in [2usize, 4, 16] {
            let mut os = Oversampler::new_steep(factor, 1);
            os.set_factor(factor);
            let ir = round_trip_impulse_response(&mut os, 400);
            for freq in [1000.0, 16_000.0, 19_000.0, 20_000.0] {
                let db = response_db(&ir, freq, sr);
                assert!(db.abs() < 0.1, "{factor}x at {freq} Hz: {db:.3} dB");
            }
        }
    }

    #[test]
    fn test_steep_first_stage_rejects_images_that_would_fold_into_the_audio_band() {
        // At 2× of 44.1 kHz, 26 kHz sits above base Nyquist and would fold to 18.1 kHz.
        let rate_2x = 88_200.0_f32;
        let tone = 26_000.0_f32;
        let settle = STEEP_HB_NUM_TAPS * 2;
        let mut os = Oversampler::new_steep(2, 1);
        os.set_factor(2);
        let mut sum_sq = 0.0_f32;
        let mut count = 0;
        for n in 0..4000 {
            let phase = |k: usize| (core::f32::consts::TAU * tone * k as f32 / rate_2x).sin();
            let y = os.downsample(&[phase(2 * n), phase(2 * n + 1)], 0);
            if n >= settle {
                sum_sq += y * y;
                count += 1;
            }
        }
        let rms_db = 10.0 * (sum_sq / count as f32).log10() + 3.01;
        assert!(rms_db < -70.0, "image leaked through at {rms_db:.1} dB");
    }

    #[test]
    #[should_panic]
    fn test_upsample_only_downsample_panics() {
        // Documented invariant: an Oversampler built via new_upsample_only
        // never allocates a downsample buffer, so calling downsample() on it
        // must panic rather than silently doing nothing.
        let mut os = Oversampler::new_upsample_only(4, 8);
        let up = os.upsample(0.1, 0).to_vec();
        os.downsample(&up, 0);
    }
}
