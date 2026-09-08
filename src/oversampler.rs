//! Cascaded halfband FIR oversampler — shared by the Punch clipper and by
//! the saturation-bearing modules (Transformer, Pultec tube stage, FET
//! all-buttons). Each 2× stage is a 23-tap Kaiser-windowed halfband FIR
//! (β=8.0). Cascaded log₂(factor) times for 2×/4×/8×/16×.
//!
//! The implementation is strictly audio-thread safe: filter state is
//! fixed-size `[f32; HB_NUM_TAPS]`, and the only heap usage is a pair of
//! `Vec<f32>` scratch buffers pre-allocated at construction time.

pub const HB_NUM_TAPS: usize = 23;
pub const MAX_OS_STAGES: usize = 4; // 2^4 = 16× max

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
    let mut coeffs = [0.0_f32; HB_NUM_TAPS];
    let m = (HB_NUM_TAPS - 1) as f32;
    let center = (HB_NUM_TAPS - 1) / 2;
    let denom = bessel_i0(beta);
    let pi = core::f32::consts::PI;

    for n in 0..HB_NUM_TAPS {
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

/// Single halfband FIR stage: holds a circular delay line over HB_NUM_TAPS
/// samples at the filter's operating rate (the higher of the two rates the
/// stage bridges).
#[derive(Clone)]
pub struct HalfbandFir {
    delay: [f32; HB_NUM_TAPS],
    pos: usize,
}

impl HalfbandFir {
    pub fn new() -> Self {
        Self {
            delay: [0.0; HB_NUM_TAPS],
            pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.delay = [0.0; HB_NUM_TAPS];
        self.pos = 0;
    }

    #[inline]
    fn convolve(&self, coeffs: &[f32; HB_NUM_TAPS]) -> f32 {
        let mut sum = 0.0_f32;
        let mut read = self.pos;
        for k in 0..HB_NUM_TAPS {
            sum += coeffs[k] * self.delay[read];
            read = if read == 0 { HB_NUM_TAPS - 1 } else { read - 1 };
        }
        sum
    }

    /// 2× upsample of one input sample → two output samples.
    /// Zero-stuff + FIR filter + ×2 gain compensation for the zero-stuff energy loss.
    #[inline]
    pub fn upsample_2x(&mut self, x: f32, coeffs: &[f32; HB_NUM_TAPS]) -> (f32, f32) {
        self.delay[self.pos] = x;
        let y0 = self.convolve(coeffs);
        self.pos = if self.pos + 1 == HB_NUM_TAPS {
            0
        } else {
            self.pos + 1
        };

        self.delay[self.pos] = 0.0;
        let y1 = self.convolve(coeffs);
        self.pos = if self.pos + 1 == HB_NUM_TAPS {
            0
        } else {
            self.pos + 1
        };

        (y0 * 2.0, y1 * 2.0)
    }

    /// 2× downsample of two input samples → one output sample.
    #[inline]
    pub fn downsample_2x(&mut self, y0: f32, y1: f32, coeffs: &[f32; HB_NUM_TAPS]) -> f32 {
        self.delay[self.pos] = y0;
        let x = self.convolve(coeffs);
        self.pos = if self.pos + 1 == HB_NUM_TAPS {
            0
        } else {
            self.pos + 1
        };

        self.delay[self.pos] = y1;
        self.pos = if self.pos + 1 == HB_NUM_TAPS {
            0
        } else {
            self.pos + 1
        };

        x
    }
}

impl Default for HalfbandFir {
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
            for s in &mut self.up_stages {
                s.reset();
            }
            for s in &mut self.down_stages {
                s.reset();
            }
        }
        self.factor = factor;
        self.num_stages = new_num_stages;
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
            let stage = &mut self.up_stages[stage_idx];
            if stage_idx % 2 == 0 {
                for i in 0..count {
                    let (y0, y1) = stage.upsample_2x(buf_a[i], &self.hb_coeffs);
                    buf_b[2 * i] = y0;
                    buf_b[2 * i + 1] = y1;
                }
            } else {
                for i in 0..count {
                    let (y0, y1) = stage.upsample_2x(buf_b[i], &self.hb_coeffs);
                    buf_a[2 * i] = y0;
                    buf_a[2 * i + 1] = y1;
                }
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
            let stage = &mut self.down_stages[self.num_stages - 1 - stage_idx];
            let new_count = count / 2;
            if stage_idx % 2 == 0 {
                for i in 0..new_count {
                    let y0 = buf_a[2 * i];
                    let y1 = buf_a[2 * i + 1];
                    buf_b[i] = stage.downsample_2x(y0, y1, &self.hb_coeffs);
                }
            } else {
                for i in 0..new_count {
                    let y0 = buf_b[2 * i];
                    let y1 = buf_b[2 * i + 1];
                    buf_a[i] = stage.downsample_2x(y0, y1, &self.hb_coeffs);
                }
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
        for s in &mut self.up_stages {
            s.reset();
        }
        for s in &mut self.down_stages {
            s.reset();
        }
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
