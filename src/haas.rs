//! Haas Module — Psychoacoustic stereo widener.
//!
//! M/S widening + side-only comb (SideComb) OR delayed L/R wide-comb
//! (WideComb) with honest naming. Not a transaural XTC. Named after Helmut
//! Haas and the precedence effect.
//!
//! Signal flow:
//! ```text
//! [In L/R] -> [M/S encode + gains]
//!           -> [Side-only comb delay line]          (SideComb)
//!              or
//!              [Raw L/R delay line -> (L-R) wide]   (WideComb)
//!           -> [Decode]
//!           -> [Output trim (RMS safety)]
//!           -> [Dry/Wet linear blend] -> [Out L/R]
//! ```
//!
//! No EQ, no saturation, no bass enhancement. Those belong to API5500,
//! Pultec, and Transformer respectively. Haas is a clean spatial tool.

use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// ============================================================================
// Constants
// ============================================================================

/// Ring-buffer length: next power of two above 20 ms @ 192 kHz plus 4-sample
/// Hermite-interpolation headroom (3840 + 4 = 3844, round up to next pow2 =
/// 4096). Power-of-two allows mask-based wrap.
pub const DELAY_BUF_LEN: usize = 4096;
/// Mask for power-of-two ring-buffer wrap — ALWAYS prefer `& DELAY_MASK` over
/// `% DELAY_BUF_LEN` on the hotpath (non-const `%` on a divisor is far slower).
pub const DELAY_MASK: usize = DELAY_BUF_LEN - 1;

/// Anti-denormal dither magnitude written into the delay line on every
/// sample. Alternates sign per sample so it averages to zero.
/// Airwindows convention.
const DENORMAL_DITHER: f32 = 1.0e-20;

/// Safety headroom reserved for Hermite4 interpolation (needs x-1, x0, x+1,
/// x+2 around the read point).
const HERMITE_HEADROOM: usize = 4;

/// Maximum safe delay in samples — keeps Hermite reads inside the ring.
const MAX_DELAY_SAMPLES: f32 = (DELAY_BUF_LEN - HERMITE_HEADROOM) as f32;

/// One-pole LPF time constant for comb-time parameter smoothing (seconds).
/// 20 ms is slow enough to eliminate zipper noise on automation sweeps but
/// fast enough that the user hears the delay time move.
const DELAY_SMOOTH_TAU_S: f32 = 0.020;

// ============================================================================
// CombMode
// ============================================================================

/// Comb-filter variant. Both variants are mono-compatible at the sum level;
/// `WideComb`'s internal 0.5 depth clamp prevents L-channel peaks that would
/// otherwise exceed the RMS-preservation budget.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Enum)]
pub enum CombMode {
    /// Side-only polarity-flip comb (WOW-Thing style). Comb contribution
    /// sums to zero on mono collapse.
    #[name = "Side Comb"]
    SideComb,
    /// Wide comb: delayed (L−R) injected with opposing signs. More diffuse
    /// than SideComb. Depth is internally clamped to 0.5 to cap L-channel
    /// peaking at +3.5 dB worst case.
    #[name = "Wide Comb"]
    WideComb,
}

impl Default for CombMode {
    fn default() -> Self {
        Self::SideComb
    }
}

// ============================================================================
// HaasModule
// ============================================================================

/// Heap-allocated power-of-two delay line.
type DelayLine = Box<[f32; DELAY_BUF_LEN]>;

fn make_delay_line() -> DelayLine {
    // `Box::new([0.0; N])` can blow the stack in debug builds for large N;
    // vec → into_boxed_slice → try_into sidesteps that and is heap-only.
    vec![0.0_f32; DELAY_BUF_LEN]
        .into_boxed_slice()
        .try_into()
        .expect("DelayLine vec length matches array length")
}

pub struct HaasModule {
    sample_rate: f32,

    // Delay buffers. 4 × 4096 × 4 B = 64 KB — heap-allocated in `new()`.
    delay_side_l: DelayLine, // SideComb: +side written
    delay_side_r: DelayLine, // SideComb: −side written
    delay_in_l: DelayLine,   // WideComb: raw L history
    delay_in_r: DelayLine,   // WideComb: raw R history
    write_pos: usize,

    // Smoothed delay state (samples, fractional). Two smoothers track the
    // same target so mode-switches inherit the correct per-path state
    // without a discontinuity.
    smoothed_delay_samples: f32,
    smoothed_xtalk_samples: f32,
    target_delay_samples: f32,
    target_xtalk_samples: f32,
    delay_smooth_coeff: f32,

    // Cached parameter state.
    mid_gain: f32,   // linear
    side_gain: f32,  // linear
    comb_depth: f32, // 0..1
    comb_mode: CombMode,
    mix: f32, // 0..1

    // Pre-computed per-buffer so process() is multiply-free on this.
    output_trim: f32,

    // Anti-denormal dither sign flip per sample.
    denormal_sign: f32,
}

impl HaasModule {
    pub fn new(sample_rate: f32) -> Self {
        let smooth_coeff = delay_smooth_coeff(sample_rate);
        let default_delay = clamp_delay(7.0_f32 * 0.001 * sample_rate);
        Self {
            sample_rate,
            delay_side_l: make_delay_line(),
            delay_side_r: make_delay_line(),
            delay_in_l: make_delay_line(),
            delay_in_r: make_delay_line(),
            write_pos: 0,
            smoothed_delay_samples: default_delay,
            smoothed_xtalk_samples: default_delay,
            target_delay_samples: default_delay,
            target_xtalk_samples: default_delay,
            delay_smooth_coeff: smooth_coeff,
            mid_gain: 1.0,
            side_gain: 1.0,
            comb_depth: 0.0,
            comb_mode: CombMode::SideComb,
            mix: 1.0,
            output_trim: 1.0,
            denormal_sign: 1.0,
        }
    }

    /// Update user-facing parameters. Called once per buffer before
    /// `process()`. Gains are **linear amplitude**, already converted from
    /// dB at the lib.rs boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn update_parameters(
        &mut self,
        mid_gain: f32,
        side_gain: f32,
        comb_depth: f32,
        comb_time_ms: f32,
        comb_mode: CombMode,
        mix: f32,
    ) {
        self.mid_gain = mid_gain;
        self.side_gain = side_gain;
        self.comb_depth = comb_depth.clamp(0.0, 1.0);
        self.comb_mode = comb_mode;
        self.mix = mix.clamp(0.0, 1.0);

        let target = clamp_delay(comb_time_ms * 0.001 * self.sample_rate);
        self.target_delay_samples = target;
        self.target_xtalk_samples = target;

        // Output compensation. Conservative upper bound on the L-channel
        // peak relative to a unity-RMS input: 1 + |side_gain| * depth. Use
        // sqrt to move toward equal-RMS rather than equal-peak; floor at
        // 1.0 so we never *boost* the wet path.
        //
        //   out_l_peak ≤ mid + side + side_delay*depth + xcomb
        //   using |mid| ≤ 1, |side| ≤ side_gain, worst case sums:
        let peak_budget = 1.0_f32 + side_gain.abs() * self.comb_depth;
        self.output_trim = 1.0 / peak_budget.max(1.0).sqrt();
    }

    /// Process a stereo buffer in place. Lock-free, allocation-free.
    pub fn process(&mut self, buffer: &mut Buffer) {
        // Flush-to-zero + denormals-are-zero for this thread. The named
        // `_MM_SET_*` helpers are deprecated / absent in recent `core::arch`,
        // so we OR the bits into MXCSR directly:
        //   FTZ = bit 15 (0x8000)
        //   DAZ = bit 6  (0x0040)
        //
        // SAFETY: These are per-thread CPU control flags. The audio callback
        // is the only caller; rewriting on every buffer is idempotent.
        // Leaving them set between callbacks is safe — FTZ/DAZ affect only
        // denormal precision, never correctness of well-scaled audio.
        #[cfg(target_arch = "x86_64")]
        #[allow(deprecated)]
        // `_mm_getcsr`/`_mm_setcsr` are soft-deprecated in favour of inline
        // asm, but they remain the portable, stable way to toggle FTZ/DAZ
        // on stable Rust. Allowed locally to avoid polluting the crate.
        unsafe {
            use core::arch::x86_64::{_mm_getcsr, _mm_setcsr};
            const FTZ_DAZ: u32 = 0x8040;
            _mm_setcsr(_mm_getcsr() | FTZ_DAZ);
        }

        for mut frame in buffer.iter_samples() {
            let mut iter = frame.iter_mut();
            // Stereo bus — 2 channels are guaranteed by the plugin layout
            // declaration. Any mono or surround layout is ignored here.
            let (l_ref, r_ref) = match (iter.next(), iter.next()) {
                (Some(l), Some(r)) => (l, r),
                _ => continue,
            };

            let in_l = *l_ref;
            let in_r = *r_ref;

            self.smoothed_delay_samples +=
                (self.target_delay_samples - self.smoothed_delay_samples) * self.delay_smooth_coeff;
            self.smoothed_xtalk_samples +=
                (self.target_xtalk_samples - self.smoothed_xtalk_samples) * self.delay_smooth_coeff;

            // M/S encode with user-facing gains baked in so downstream math
            // treats mid/side as the processed quantities, not the raw
            // mid/side components.
            let mid = (in_l + in_r) * 0.5 * self.mid_gain;
            let side = (in_l - in_r) * 0.5 * self.side_gain;

            // Write current frame into all four delay lines. The denormal
            // dither alternates sign per sample so it DC-averages to zero
            // yet keeps the filter state out of denormal territory during
            // silence.
            self.denormal_sign = -self.denormal_sign;
            let dither = DENORMAL_DITHER * self.denormal_sign;
            self.delay_side_l[self.write_pos] = side + dither;
            self.delay_side_r[self.write_pos] = -side + dither;
            self.delay_in_l[self.write_pos] = in_l + dither;
            self.delay_in_r[self.write_pos] = in_r + dither;

            let (comb_l, comb_r) = match self.comb_mode {
                CombMode::SideComb => {
                    let side_delayed_l = hermite4_read(
                        &self.delay_side_l,
                        self.write_pos,
                        self.smoothed_delay_samples,
                    );
                    let side_delayed_r = hermite4_read(
                        &self.delay_side_r,
                        self.write_pos,
                        self.smoothed_delay_samples,
                    );
                    (
                        side_delayed_l * self.comb_depth,
                        side_delayed_r * self.comb_depth,
                    )
                }
                CombMode::WideComb => {
                    // Hard-clamp depth for this mode. At depth=1.0 the
                    // per-channel peak would scale by 2× worst case; the
                    // 0.5 cap holds it at 1.5× (≈ +3.5 dB) before
                    // `output_trim` scales it back.
                    let effective_depth = self.comb_depth.min(0.5);
                    let x_l = hermite4_read(
                        &self.delay_in_l,
                        self.write_pos,
                        self.smoothed_xtalk_samples,
                    );
                    let x_r = hermite4_read(
                        &self.delay_in_r,
                        self.write_pos,
                        self.smoothed_xtalk_samples,
                    );
                    let cancel = (x_l - x_r) * effective_depth * 0.5;
                    (cancel, -cancel)
                }
            };

            // Decode M/S back to L/R with the comb contribution added.
            let wide_l = mid + side + comb_l;
            let wide_r = mid - side + comb_r;

            // Linear dry/wet blend. Equal-power would over-boost correlated
            // material near mix=0.5.
            let trimmed_l = wide_l * self.output_trim;
            let trimmed_r = wide_r * self.output_trim;
            *l_ref = in_l + (trimmed_l - in_l) * self.mix;
            *r_ref = in_r + (trimmed_r - in_r) * self.mix;

            // Advance the write pointer. Mask wrap is cheaper than `%` on
            // a non-const divisor and works because DELAY_BUF_LEN is a
            // power of two.
            self.write_pos = (self.write_pos + 1) & DELAY_MASK;
        }
    }

    /// Zero all delay buffers and reset smoothed state. Safe from the audio
    /// thread — no allocation.
    pub fn reset(&mut self) {
        for s in self.delay_side_l.iter_mut() {
            *s = 0.0;
        }
        for s in self.delay_side_r.iter_mut() {
            *s = 0.0;
        }
        for s in self.delay_in_l.iter_mut() {
            *s = 0.0;
        }
        for s in self.delay_in_r.iter_mut() {
            *s = 0.0;
        }
        self.write_pos = 0;
        self.smoothed_delay_samples = self.target_delay_samples;
        self.smoothed_xtalk_samples = self.target_xtalk_samples;
        self.denormal_sign = 1.0;
    }

    /// Current module latency in samples. Haas uses a feed-forward delay
    /// line; latency = floor of the smoothed delay length while the comb
    /// is audible. The host needs this for plugin-delay compensation.
    #[allow(dead_code)]
    pub fn latency_samples(&self) -> u32 {
        // Only report latency when the wet branch is actually audible.
        // At mix=0 or comb_depth=0 the output is bit-identical to the
        // input on the decode side, but the delay still affects the comb
        // contribution — so latency is purely informational anyway.
        if self.mix <= 0.0 {
            0
        } else {
            self.smoothed_delay_samples.max(0.0).floor() as u32
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// One-pole LPF coefficient for the given sample rate, targeting a time
/// constant of `DELAY_SMOOTH_TAU_S` seconds.
#[inline]
fn delay_smooth_coeff(sample_rate: f32) -> f32 {
    if sample_rate <= 0.0 {
        return 1.0;
    }
    1.0 - (-1.0 / (DELAY_SMOOTH_TAU_S * sample_rate)).exp()
}

/// Clamp a raw delay-in-samples value to a safe Hermite-compatible range.
#[inline]
fn clamp_delay(delay: f32) -> f32 {
    delay.clamp(0.0, MAX_DELAY_SAMPLES)
}

/// 4-point Hermite interpolation for fractional delay reads.
/// Reference: Moorer, "The Manifold Joys of Conformal Mapping," JAES 1983.
#[inline]
fn hermite4_read(buf: &[f32; DELAY_BUF_LEN], write_pos: usize, delay: f32) -> f32 {
    let delay = delay.clamp(0.0, MAX_DELAY_SAMPLES);
    let di = delay.floor();
    let frac = delay - di;
    let i = di as usize;
    let base = (write_pos + DELAY_BUF_LEN - i) & DELAY_MASK;
    let xm1 = buf[(base + 1) & DELAY_MASK];
    let x0 = buf[base];
    let x1 = buf[(base + DELAY_BUF_LEN - 1) & DELAY_MASK];
    let x2 = buf[(base + DELAY_BUF_LEN - 2) & DELAY_MASK];
    let c0 = x0;
    let c1 = 0.5 * (x1 - xm1);
    let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
    ((c3 * frac + c2) * frac + c1) * frac + c0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Build an allocated `Buffer` backed by owned stereo slices. Returns
    /// the `Buffer` plus the two backing vecs so the test retains
    /// ownership of the memory.
    struct StereoBuf {
        data_l: Vec<f32>,
        data_r: Vec<f32>,
    }

    impl StereoBuf {
        fn new(n: usize) -> Self {
            Self {
                data_l: vec![0.0; n],
                data_r: vec![0.0; n],
            }
        }

        fn fill_mono(&mut self, samples: &[f32]) {
            for (i, &v) in samples.iter().enumerate() {
                self.data_l[i] = v;
                self.data_r[i] = v;
            }
        }

        fn process_through(&mut self, haas: &mut HaasModule) {
            // Build a real nih_plug Buffer that points at our two vecs.
            let mut buffer = Buffer::default();
            // SAFETY: set_slices is the supported way to construct a Buffer
            // pointing at external owned storage. We pass lifetimes bound
            // to `self` so `buffer` cannot outlive the Vecs.
            unsafe {
                let len = self.data_l.len();
                buffer.set_slices(len, |slices| {
                    slices.clear();
                    slices.push(&mut self.data_l);
                    slices.push(&mut self.data_r);
                });
            }
            haas.process(&mut buffer);
        }
    }

    #[test]
    fn identity_mono_input_passes_through_after_warmup() {
        let mut haas = HaasModule::new(SR);
        // Unity M/S gains, no comb, fully wet.
        haas.update_parameters(1.0, 1.0, 0.0, 7.0, CombMode::SideComb, 1.0);

        let n = 4096;
        let mut buf = StereoBuf::new(n);
        // Deterministic mono content.
        for i in 0..n {
            let v = ((i as f32) * 0.01).sin();
            buf.data_l[i] = v;
            buf.data_r[i] = v;
        }

        // Save input for comparison.
        let in_l = buf.data_l.clone();
        let in_r = buf.data_r.clone();

        buf.process_through(&mut haas);

        // Comb depth = 0, output_trim = 1.0, mono input → side = 0.
        // out_l = mid + side + 0 = in_l; out_r = mid - side + 0 = in_r.
        // Allow for the anti-denormal dither (±1e-20).
        let eps = 1.0e-6;
        for i in 0..n {
            assert!(
                (buf.data_l[i] - in_l[i]).abs() < eps,
                "L identity failed at {i}"
            );
            assert!(
                (buf.data_r[i] - in_r[i]).abs() < eps,
                "R identity failed at {i}"
            );
        }
    }

    #[test]
    fn widening_increases_side_energy() {
        let mut haas = HaasModule::new(SR);
        // +6 dB on side, no comb, fully wet.
        haas.update_parameters(1.0, 2.0, 0.0, 7.0, CombMode::SideComb, 1.0);

        let n = 1024;
        let mut buf = StereoBuf::new(n);
        for i in 0..n {
            buf.data_l[i] = ((i as f32) * 0.1).sin();
            buf.data_r[i] = 0.0;
        }
        let in_side: f32 = buf
            .data_l
            .iter()
            .zip(buf.data_r.iter())
            .map(|(l, r)| (l - r).abs())
            .sum();

        buf.process_through(&mut haas);

        let out_side: f32 = buf
            .data_l
            .iter()
            .zip(buf.data_r.iter())
            .map(|(l, r)| (l - r).abs())
            .sum();

        assert!(
            out_side > in_side,
            "side energy did not grow: in={in_side}, out={out_side}"
        );
    }

    #[test]
    fn mix_zero_is_bit_exact_passthrough() {
        let mut haas = HaasModule::new(SR);
        // Extreme settings — output must still equal input when mix=0.
        haas.update_parameters(2.0, 4.0, 1.0, 20.0, CombMode::WideComb, 0.0);

        let n = 512;
        let mut buf = StereoBuf::new(n);
        for i in 0..n {
            buf.data_l[i] = ((i as f32) * 0.05).sin();
            buf.data_r[i] = ((i as f32) * 0.07).cos();
        }
        let in_l = buf.data_l.clone();
        let in_r = buf.data_r.clone();

        buf.process_through(&mut haas);

        for i in 0..n {
            assert_eq!(
                buf.data_l[i], in_l[i],
                "L must be bit-exact at mix=0 (i={i})"
            );
            assert_eq!(
                buf.data_r[i], in_r[i],
                "R must be bit-exact at mix=0 (i={i})"
            );
        }
    }

    #[test]
    fn sidecomb_mono_sum_null_across_parameter_space() {
        // Mono input must sum to 2 * mid_gain * input across any
        // comb_depth / comb_time — within one dither epsilon.
        for &depth in &[0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            for &time in &[1.0_f32, 7.0, 20.0] {
                let mut haas = HaasModule::new(SR);
                haas.update_parameters(1.0, 1.0, depth, time, CombMode::SideComb, 1.0);

                let n = 2048;
                let mut buf = StereoBuf::new(n);
                let impulse: Vec<f32> = (0..n).map(|i| if i == 100 { 1.0 } else { 0.0 }).collect();
                buf.fill_mono(&impulse);
                let expected_sum: Vec<f32> = impulse
                    .iter()
                    .map(|&v| 2.0 * 1.0 * v * haas.output_trim)
                    .collect();

                buf.process_through(&mut haas);

                for i in 0..n {
                    let sum = buf.data_l[i] + buf.data_r[i];
                    let err = (sum - expected_sum[i]).abs();
                    assert!(
                        err < 1.0e-4,
                        "mono-sum null failed at i={i} depth={depth} time={time}: sum={sum} expected={} err={err}",
                        expected_sum[i]
                    );
                }
            }
        }
    }

    #[test]
    fn widecomb_mono_sum_bounded() {
        // Mono input + max depth — verify the internal clamp keeps the
        // mono sum well above the catastrophic notch region. Because
        // the wide-comb contribution is anti-symmetric it cancels in
        // the sum, so the assertion is actually stronger than a notch
        // bound: the sum equals 2*mid*trim exactly.
        let mut haas = HaasModule::new(SR);
        haas.update_parameters(1.0, 1.0, 1.0, 5.0, CombMode::WideComb, 1.0);

        let n = 1024;
        let mut buf = StereoBuf::new(n);
        let sine: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.1).sin()).collect();
        buf.fill_mono(&sine);

        buf.process_through(&mut haas);

        // Sum energy should be close to 2*mid*trim * sum_energy_of_input.
        let input_energy: f32 = sine.iter().map(|v| v * v).sum();
        let sum_energy: f32 = (0..n)
            .map(|i| {
                let s = buf.data_l[i] + buf.data_r[i];
                s * s
            })
            .sum();
        let expected = 4.0 * haas.output_trim * haas.output_trim * input_energy;
        let rel_err = (sum_energy - expected).abs() / expected.max(1e-9);
        assert!(
            rel_err < 0.01,
            "WideComb mono-sum deviates too much: sum={sum_energy}, expected={expected}, rel_err={rel_err}"
        );
    }

    #[test]
    fn correlation_decreases_as_side_gain_increases() {
        // Pearson(L,R) on a stereo-correlated input should monotonically
        // decrease as side_gain grows.
        let n = 2048;
        let input_l: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.1).sin()).collect();
        // Highly correlated (0.8 * L + 0.2 * decorrelated noise).
        let input_r: Vec<f32> = (0..n)
            .map(|i| 0.8 * ((i as f32) * 0.1).sin() + 0.2 * ((i as f32) * 0.13).sin())
            .collect();

        let mut correlations = Vec::new();
        for &sg in &[0.5_f32, 1.0, 1.5, 2.0] {
            let mut haas = HaasModule::new(SR);
            haas.update_parameters(1.0, sg, 0.0, 7.0, CombMode::SideComb, 1.0);

            let mut buf = StereoBuf::new(n);
            buf.data_l.copy_from_slice(&input_l);
            buf.data_r.copy_from_slice(&input_r);

            buf.process_through(&mut haas);
            correlations.push(pearson(&buf.data_l, &buf.data_r));
        }

        for w in correlations.windows(2) {
            assert!(
                w[1] < w[0] + 1.0e-3,
                "correlation did not decrease: {:?}",
                correlations
            );
        }
    }

    #[test]
    fn dc_preservation() {
        // DC in → DC out at unity mid_gain.
        let mut haas = HaasModule::new(SR);
        haas.update_parameters(1.0, 1.0, 0.5, 7.0, CombMode::SideComb, 1.0);
        let n = 512;
        let mut buf = StereoBuf::new(n);
        for i in 0..n {
            buf.data_l[i] = 0.5;
            buf.data_r[i] = 0.5;
        }
        buf.process_through(&mut haas);
        // Allow small tolerance for denormal dither + output_trim.
        // Trim = 1/sqrt(1 + 1*0.5) = 1/sqrt(1.5) ≈ 0.8165.
        let expected = 0.5 * haas.output_trim;
        for i in 100..n {
            assert!(
                (buf.data_l[i] - expected).abs() < 1.0e-4,
                "L DC drift at {i}: {} vs {expected}",
                buf.data_l[i]
            );
            assert!(
                (buf.data_r[i] - expected).abs() < 1.0e-4,
                "R DC drift at {i}: {} vs {expected}",
                buf.data_r[i]
            );
        }
    }

    #[test]
    fn latency_report_matches_smoothed_delay() {
        let mut haas = HaasModule::new(SR);
        haas.update_parameters(1.0, 1.0, 0.0, 10.0, CombMode::SideComb, 1.0);

        // Force smoothed delay to catch up to target by running lots of
        // samples through.
        let n = 48_000;
        let mut buf = StereoBuf::new(n);
        buf.process_through(&mut haas);

        let expected = (10.0_f32 * 0.001 * SR) as u32;
        let got = haas.latency_samples();
        assert!(
            got.abs_diff(expected) <= 1,
            "latency mismatch: got={got}, expected={expected}"
        );
    }

    #[test]
    fn no_click_on_automation_sweep() {
        // Sweep comb_time from 1 ms to 20 ms over 100 ms of audio.
        // Output max-abs should stay bounded (< 2.0) — any discontinuity
        // would spike above the signal envelope.
        let mut haas = HaasModule::new(SR);
        let sweep_samples = (0.100 * SR) as usize;
        let mut buf = StereoBuf::new(sweep_samples);
        for i in 0..sweep_samples {
            let v = 0.5 * ((i as f32) * 0.05).sin();
            buf.data_l[i] = v;
            buf.data_r[i] = 0.5 * ((i as f32) * 0.05).cos();
        }

        // Process in small sub-buffers so update_parameters gets called
        // repeatedly as the user would see from automation.
        let chunk = 64;
        let mut pos = 0;
        while pos < sweep_samples {
            let t = pos as f32 / sweep_samples as f32;
            let comb_time = 1.0 + 19.0 * t;
            haas.update_parameters(1.0, 1.0, 0.5, comb_time, CombMode::SideComb, 1.0);

            let end = (pos + chunk).min(sweep_samples);
            let mut sub = StereoBuf::new(end - pos);
            sub.data_l.copy_from_slice(&buf.data_l[pos..end]);
            sub.data_r.copy_from_slice(&buf.data_r[pos..end]);
            sub.process_through(&mut haas);
            buf.data_l[pos..end].copy_from_slice(&sub.data_l);
            buf.data_r[pos..end].copy_from_slice(&sub.data_r);
            pos = end;
        }

        let max_abs = buf
            .data_l
            .iter()
            .chain(buf.data_r.iter())
            .fold(0.0_f32, |a, &v| a.max(v.abs()));
        assert!(
            max_abs < 2.0,
            "sweep produced suspicious spike: max_abs={max_abs}"
        );
    }

    #[test]
    fn denormal_survival() {
        // After a loud burst and a long tail of silence, buffers must
        // flush to exact zero — FTZ should have cleaned up.
        let mut haas = HaasModule::new(SR);
        haas.update_parameters(1.0, 1.0, 1.0, 7.0, CombMode::SideComb, 1.0);

        let n = (0.100 * SR) as usize; // 100 ms
        let mut buf = StereoBuf::new(n);
        // Loud burst in first 10 ms.
        let burst = (0.010 * SR) as usize;
        for i in 0..burst {
            let v = ((i as f32) * 0.5).sin();
            buf.data_l[i] = v;
            buf.data_r[i] = -v;
        }
        buf.process_through(&mut haas);

        // Now 10 s of silence.
        let silence_len = (10.0 * SR) as usize;
        let mut silence = StereoBuf::new(silence_len);
        silence.process_through(&mut haas);

        // Final buffer must be bounded — no denormal explosion.
        let max_abs = silence
            .data_l
            .iter()
            .chain(silence.data_r.iter())
            .fold(0.0_f32, |a, &v| a.max(v.abs()));
        assert!(
            max_abs < 1.0e-9,
            "denormal tail did not settle: max_abs={max_abs}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut haas = HaasModule::new(SR);
        haas.update_parameters(1.0, 1.0, 1.0, 7.0, CombMode::SideComb, 1.0);

        // Run a loud burst.
        let mut buf = StereoBuf::new(4096);
        for i in 0..4096 {
            buf.data_l[i] = 1.0;
            buf.data_r[i] = -1.0;
        }
        buf.process_through(&mut haas);

        haas.reset();

        // After reset, a fresh mono impulse should propagate without any
        // leftover signal: the comb lines are zero.
        let mut post = StereoBuf::new(512);
        post.data_l[0] = 1.0;
        post.data_r[0] = 1.0;
        post.process_through(&mut haas);

        // At t=0 (mono impulse), side=0, comb contribution = 0 →
        // out_l = out_r = mid_gain * 1.0 * output_trim.
        let expected = 1.0 * haas.output_trim;
        assert!(
            (post.data_l[0] - expected).abs() < 1.0e-4,
            "post-reset L at t=0: {} vs {expected}",
            post.data_l[0]
        );
        assert!(
            (post.data_r[0] - expected).abs() < 1.0e-4,
            "post-reset R at t=0: {} vs {expected}",
            post.data_r[0]
        );
    }

    // ----- Helpers ---------------------------------------------------------

    fn pearson(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        let n = a.len() as f32;
        let mean_a = a.iter().sum::<f32>() / n;
        let mean_b = b.iter().sum::<f32>() / n;
        let mut num = 0.0_f32;
        let mut den_a = 0.0_f32;
        let mut den_b = 0.0_f32;
        for (&av, &bv) in a.iter().zip(b.iter()) {
            let da = av - mean_a;
            let db = bv - mean_b;
            num += da * db;
            den_a += da * da;
            den_b += db * db;
        }
        let denom = (den_a * den_b).sqrt().max(1.0e-12);
        num / denom
    }
}
