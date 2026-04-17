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
        self.pos = if self.pos + 1 == HB_NUM_TAPS { 0 } else { self.pos + 1 };

        self.delay[self.pos] = 0.0;
        let y1 = self.convolve(coeffs);
        self.pos = if self.pos + 1 == HB_NUM_TAPS { 0 } else { self.pos + 1 };

        (y0 * 2.0, y1 * 2.0)
    }

    /// 2× downsample of two input samples → one output sample.
    #[inline]
    pub fn downsample_2x(&mut self, y0: f32, y1: f32, coeffs: &[f32; HB_NUM_TAPS]) -> f32 {
        self.delay[self.pos] = y0;
        let x = self.convolve(coeffs);
        self.pos = if self.pos + 1 == HB_NUM_TAPS { 0 } else { self.pos + 1 };

        self.delay[self.pos] = y1;
        self.pos = if self.pos + 1 == HB_NUM_TAPS { 0 } else { self.pos + 1 };

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
    num_stages: usize,
    hb_coeffs: [f32; HB_NUM_TAPS],
    up_stages: [HalfbandFir; MAX_OS_STAGES],
    down_stages: [HalfbandFir; MAX_OS_STAGES],
    upsample_buffer: Vec<f32>,
    downsample_buffer: Vec<f32>,
}

impl Oversampler {
    pub fn new(_max_factor: usize, max_block_size: usize) -> Self {
        Self {
            factor: 1,
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
            upsample_buffer: vec![0.0; max_block_size * (1 << MAX_OS_STAGES)],
            downsample_buffer: vec![0.0; max_block_size],
        }
    }

    pub fn set_factor(&mut self, factor: usize) {
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

    pub fn factor(&self) -> usize {
        self.factor
    }

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
