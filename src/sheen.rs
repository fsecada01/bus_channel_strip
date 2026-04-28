//! Sheen Module — master-end "polish coat".
//!
//! Pinned at the end of the chain (post-Punch, pre-master-gain). Always present
//! in the signal flow regardless of slot order; not user-reorderable. The brass
//! "API" plate in the chassis header is the only front-panel surface; the
//! five sliders below live on a hidden back view.
//!
//! Five stages, in processing order:
//!
//! ```text
//! [in] -> BODY (low shelf) -> PRESENCE (peak) -> AIR (high shelf)
//!      -> WARMTH (Inflator polynomial @ 2x oversample)
//!      -> WIDTH (M/S side-only HPF + high shelf)
//!      -> [out]
//! ```
//!
//! Stage rationale and citations live in `docs/SHEEN_MODULE_SPEC.md`.

use crate::shaping::{biquad_coeffs, Filter, FilterType};
use biquad::{Biquad, DirectForm1, Type};
use nih_plug::buffer::Buffer;

// ============================================================================
// Stage constants — fixed frequencies / Qs from the spec
// ============================================================================

/// BODY: low shelf at 100 Hz, conservative Q for a smooth contour.
const BODY_FREQ_HZ: f32 = 100.0;
const BODY_Q: f32 = 0.707;

/// PRESENCE: peak EQ at 3 kHz, Q=1.0 for a focused but musical bell.
const PRESENCE_FREQ_HZ: f32 = 3000.0;
const PRESENCE_Q: f32 = 1.0;

/// AIR: high shelf at 14 kHz with a low Q. Low Q matters here — high-Q air
/// shelves create a resonant peak just below the corner that sounds glassy.
const AIR_FREQ_HZ: f32 = 14000.0;
const AIR_Q: f32 = 0.5;

/// WIDTH stage operates only on the side signal of an M/S decomposition.
/// The HPF kills bass below 150 Hz (mono lows protect the low end), the
/// shelf adds optional brilliance above 500 Hz.
const WIDTH_HPF_HZ: f32 = 150.0;
const WIDTH_HPF_Q: f32 = 0.707;
const WIDTH_SHELF_HZ: f32 = 500.0;
const WIDTH_SHELF_Q: f32 = 0.707;

/// At width_param = 1.0 the side gets +25% above 500 Hz (≈ +1.94 dB on the
/// side energy). Held intentionally subtle — width slamming sounds gimmicky.
const MAX_WIDTH_GAIN: f32 = 0.25;

// ============================================================================
// Inflator polynomial — Sonnox Curve = 0
// ============================================================================
//
// Public-domain reverse-engineered transfer function (RCJacH JSFX, nulls the
// original Sonnox at every Curve setting):
//
//   f(x) = A·x + B·x² + C·x³ - D·(x² - 2x³ + x⁴)
//
// At Curve = 0 (the most-loved setting per the polish-plugin synthesis):
//   A = 1.5, B = 0, C = -0.5, D = 0.0625
//
// Expanded into a single quartic in x:
//   f(x) = 1.5·x + (-0.0625)·x² + (-0.5 + 0.125)·x³ + (-0.0625)·x⁴
//        = 1.5·x  -  0.0625·x²  -  0.375·x³  -  0.0625·x⁴
//
// f(0) = 0 and f(1) = 1 by construction; f(-1) = -1.25 (asymmetric — adds
// even-order content). The Effect mix scales the asymmetry back at the
// 20% factory default, where the worst-case excursion is ≈ -1.05.

const INFLATOR_X1: f32 = 1.5;
const INFLATOR_X2: f32 = -0.0625;
const INFLATOR_X3: f32 = -0.375;
const INFLATOR_X4: f32 = -0.0625;

/// 2× oversampling for the warmth shaper. The polynomial generates 4th-order
/// products (cubic+quartic) so 2× pushes the worst-case alias above 22 kHz at
/// 44.1 kHz host rate. Cheap enough to justify; if measurable aliasing turns
/// up in the tuning pass we can bump to 4×.
const OS_FACTOR: usize = 2;

// ============================================================================
// SheenModule
// ============================================================================

pub struct SheenModule {
    sample_rate: f32,

    // EQ stages — `Filter` keeps per-channel state so stereo processing stays
    // phase-coherent (per `feedback_stereo_biquad_state.md`).
    body: Filter,
    presence: Filter,
    air: Filter,

    // WIDTH side-channel filters. These see ONE signal (the M/S-derived
    // side), so a single DirectForm1 — not a `Filter` — is correct.
    width_hpf: DirectForm1<f32>,
    width_shelf: DirectForm1<f32>,

    // WARMTH oversampler state — one previous-input and one previous-output
    // sample per channel. The previous-input is used by linear interpolation
    // upsampling; the previous-output by a 1-pole IIR for downsampling.
    warmth_prev_in: [f32; 2],
    warmth_prev_out: [f32; 2],

    // Cached parameter values. Compared against incoming params each buffer
    // so coefficients only regenerate when a slider actually moves —
    // sin/cos in `biquad_coeffs` is the most expensive op in this module.
    body_db: f32,
    presence_db: f32,
    air_db: f32,
    warmth_effect: f32,
    width_param: f32,

    // Per-stage bypasses. Each one is a flat boolean check at the top of its
    // stage block — no allocation, no indirection.
    body_bypass: bool,
    presence_bypass: bool,
    air_bypass: bool,
    warmth_bypass: bool,
    width_bypass: bool,

    /// Master Sheen bypass. When true, `process()` returns immediately
    /// without touching the buffer.
    sheen_bypass: bool,

    // Coefficient-regen flags. Set in `update_parameters` when the cached
    // value disagrees with the new value; consumed and cleared by
    // `regen_coeffs_if_dirty` at the top of `process()`.
    dirty_body: bool,
    dirty_presence: bool,
    dirty_air: bool,
    dirty_width: bool,
}

impl SheenModule {
    /// Construct a new SheenModule at the given host sample rate. Initial
    /// parameter cache matches the spec's factory defaults so the first
    /// processed buffer already has the right tonality even before the
    /// host pushes its first parameter update.
    pub fn new(sample_rate: f32) -> Self {
        let body = Filter::new(
            sample_rate,
            FilterType::LowShelf,
            BODY_FREQ_HZ,
            BODY_Q,
            1.0, // factory default body_db
        );
        let presence = Filter::new(
            sample_rate,
            FilterType::Bell,
            PRESENCE_FREQ_HZ,
            PRESENCE_Q,
            0.0, // factory default presence_db (transparent)
        );
        let air = Filter::new(
            sample_rate,
            FilterType::HighShelf,
            AIR_FREQ_HZ,
            AIR_Q,
            1.8, // factory default air_db
        );

        let hpf_coeff = biquad_coeffs(Type::HighPass, sample_rate, WIDTH_HPF_HZ, WIDTH_HPF_Q)
            .expect("Sheen width HPF coefficient build failed at construction");
        let shelf_coeff = biquad_coeffs(
            Type::HighShelf(width_shelf_db_for(0.5)), // factory default width=0.5
            sample_rate,
            WIDTH_SHELF_HZ,
            WIDTH_SHELF_Q,
        )
        .expect("Sheen width shelf coefficient build failed at construction");

        Self {
            sample_rate,
            body,
            presence,
            air,
            width_hpf: DirectForm1::<f32>::new(hpf_coeff),
            width_shelf: DirectForm1::<f32>::new(shelf_coeff),
            warmth_prev_in: [0.0; 2],
            warmth_prev_out: [0.0; 2],
            body_db: 1.0,
            presence_db: 0.0,
            air_db: 1.8,
            warmth_effect: 0.20,
            width_param: 0.5,
            body_bypass: false,
            presence_bypass: false,
            air_bypass: false,
            warmth_bypass: false,
            width_bypass: false,
            sheen_bypass: false,
            dirty_body: false,
            dirty_presence: false,
            dirty_air: false,
            dirty_width: false,
        }
    }

    /// Update cached parameter state. Called once per buffer from the audio
    /// callback before `process()`. Coefficient regeneration is deferred to
    /// `process()` and skipped entirely when no value changed — a cheap
    /// no-op on the steady-state path where nothing is being automated.
    #[allow(clippy::too_many_arguments)]
    pub fn update_parameters(
        &mut self,
        sheen_bypass: bool,
        body_db: f32,
        body_bypass: bool,
        presence_db: f32,
        presence_bypass: bool,
        air_db: f32,
        air_bypass: bool,
        warmth_effect: f32,
        warmth_bypass: bool,
        width_param: f32,
        width_bypass: bool,
    ) {
        self.sheen_bypass = sheen_bypass;

        // Float compare with a small epsilon — sliders smoothed via
        // SmoothingStyle::Linear bounce within ~1e-4 of the target each
        // frame, so a hard `!=` would treat every settled buffer as dirty.
        if (body_db - self.body_db).abs() > 1.0e-4 {
            self.body_db = body_db;
            self.dirty_body = true;
        }
        if (presence_db - self.presence_db).abs() > 1.0e-4 {
            self.presence_db = presence_db;
            self.dirty_presence = true;
        }
        if (air_db - self.air_db).abs() > 1.0e-4 {
            self.air_db = air_db;
            self.dirty_air = true;
        }
        if (width_param - self.width_param).abs() > 1.0e-4 {
            self.width_param = width_param;
            self.dirty_width = true;
        }
        // Warmth Effect doesn't drive a biquad; clamp and stash directly.
        self.warmth_effect = warmth_effect.clamp(0.0, 1.0);

        self.body_bypass = body_bypass;
        self.presence_bypass = presence_bypass;
        self.air_bypass = air_bypass;
        self.warmth_bypass = warmth_bypass;
        self.width_bypass = width_bypass;
    }

    /// Process a stereo buffer in place. Lock-free, allocation-free.
    pub fn process(&mut self, buffer: &mut Buffer) {
        if self.sheen_bypass {
            return;
        }

        self.regen_coeffs_if_dirty();

        for mut frame in buffer.iter_samples() {
            let mut iter = frame.iter_mut();
            let (l_ref, r_ref) = match (iter.next(), iter.next()) {
                (Some(l), Some(r)) => (l, r),
                // Mono or surround layouts: skip rather than corrupt.
                _ => continue,
            };

            let mut l = *l_ref;
            let mut r = *r_ref;

            // ── BODY ─ low shelf @ 100 Hz ───────────────────────────────
            if !self.body_bypass {
                l = self.body.run_ch(l, 0);
                r = self.body.run_ch(r, 1);
            }

            // ── PRESENCE ─ peaking @ 3 kHz ──────────────────────────────
            if !self.presence_bypass {
                l = self.presence.run_ch(l, 0);
                r = self.presence.run_ch(r, 1);
            }

            // ── AIR ─ high shelf @ 14 kHz ───────────────────────────────
            if !self.air_bypass {
                l = self.air.run_ch(l, 0);
                r = self.air.run_ch(r, 1);
            }

            // ── WARMTH ─ Inflator-style polynomial @ 2× oversample ──────
            // Skip the whole stage when effect is at-or-below noise floor;
            // saves the polynomial and the oversampler hop on the dry path.
            if !self.warmth_bypass && self.warmth_effect > 1.0e-6 {
                l = self.process_warmth(l, 0);
                r = self.process_warmth(r, 1);
            }

            // ── WIDTH ─ M/S side-only HPF + shelf ───────────────────────
            // Side channel sees a HPF (mono-fy bass) and a high shelf
            // (subtly lift sides above 500 Hz). Mid passes through clean.
            if !self.width_bypass {
                let mid = (l + r) * 0.5;
                let mut side = (l - r) * 0.5;
                side = self.width_hpf.run(side);
                side = self.width_shelf.run(side);
                l = mid + side;
                r = mid - side;
            }

            *l_ref = l;
            *r_ref = r;
        }
    }

    /// Zero out the warmth oversampler state. Biquad state is left to
    /// settle naturally with silence input — DirectForm1 has no public
    /// reset and rebuilding the filters mid-process would require
    /// re-running `biquad_coeffs` (sin/cos), which we'd rather avoid.
    /// In practice the host calls `reset()` on transport start where the
    /// buffer leading edge is silence anyway, so any residual filter
    /// energy decays within ~100 samples for our chosen Q values.
    pub fn reset(&mut self) {
        self.warmth_prev_in = [0.0; 2];
        self.warmth_prev_out = [0.0; 2];
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Regenerate any dirty filter coefficients. Each branch only runs when
    /// its corresponding slider actually moved since last buffer.
    fn regen_coeffs_if_dirty(&mut self) {
        if self.dirty_body {
            self.body.update_parameters(
                self.sample_rate,
                FilterType::LowShelf,
                BODY_FREQ_HZ,
                BODY_Q,
                self.body_db,
            );
            self.dirty_body = false;
        }
        if self.dirty_presence {
            self.presence.update_parameters(
                self.sample_rate,
                FilterType::Bell,
                PRESENCE_FREQ_HZ,
                PRESENCE_Q,
                self.presence_db,
            );
            self.dirty_presence = false;
        }
        if self.dirty_air {
            self.air.update_parameters(
                self.sample_rate,
                FilterType::HighShelf,
                AIR_FREQ_HZ,
                AIR_Q,
                self.air_db,
            );
            self.dirty_air = false;
        }
        if self.dirty_width {
            let shelf_db = width_shelf_db_for(self.width_param);
            if let Ok(coeff) = biquad_coeffs(
                Type::HighShelf(shelf_db),
                self.sample_rate,
                WIDTH_SHELF_HZ,
                WIDTH_SHELF_Q,
            ) {
                self.width_shelf.update_coefficients(coeff);
            }
            self.dirty_width = false;
        }
    }

    /// 2× oversampled Inflator pass for one channel. Linear interpolation
    /// up, polynomial at 2× rate, average + 1-pole IIR down. The IIR pole
    /// at 0.5 matches the punch.rs convention for cheap halfband-ish
    /// downsampling — light enough not to pull HF, strong enough to keep
    /// the alias below the noise floor at typical playback levels.
    #[inline]
    fn process_warmth(&mut self, x: f32, ch: usize) -> f32 {
        let prev_in = self.warmth_prev_in[ch];
        // First inserted (interpolated) sample of the pair.
        let interp = (prev_in + x) * 0.5;
        self.warmth_prev_in[ch] = x;

        // Apply the Inflator transfer function at the 2× rate.
        let shaped_a = inflator(interp);
        let shaped_b = inflator(x);

        // Wet/dry mix is per-sample so the mix knob is exactly equivalent
        // to operator-applied parallel processing.
        let mix = self.warmth_effect;
        let dry = 1.0 - mix;
        let out_a = dry * interp + mix * shaped_a;
        let out_b = dry * x + mix * shaped_b;

        // Downsample: average the pair, then a 1-pole IIR for extra
        // alias suppression above Nyquist.
        let avg = (out_a + out_b) * 0.5;
        let prev_out = self.warmth_prev_out[ch];
        let smoothed = prev_out + (avg - prev_out) * 0.5;
        self.warmth_prev_out[ch] = smoothed;
        smoothed
    }
}

/// Map the `width_param` (0..=1) to dB for the side-channel high shelf.
/// At width = 0.0 → 0 dB (no width change). At width = 1.0 → +1.94 dB.
#[inline]
fn width_shelf_db_for(width_param: f32) -> f32 {
    let w = width_param.clamp(0.0, 1.0);
    20.0 * (1.0 + MAX_WIDTH_GAIN * w).log10()
}

/// Sonnox Inflator transfer function at Curve = 0. Inlined hot path.
#[inline]
fn inflator(x: f32) -> f32 {
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    INFLATOR_X1 * x + INFLATOR_X2 * x2 + INFLATOR_X3 * x3 + INFLATOR_X4 * x4
}

// Compile-time assertion that the oversample factor stays the value the
// algorithm above assumes. If we bump to 4× this constant guards the
// math elsewhere in the file.
const _: () = assert!(OS_FACTOR == 2, "WARMTH oversampler is hard-coded for 2×");

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// inflator() must satisfy the two anchoring values that define the
    /// Inflator @ Curve = 0 transfer: f(0) = 0 and f(1) = 1.
    #[test]
    fn inflator_anchors_at_0_and_1() {
        assert!(inflator(0.0).abs() < 1.0e-7);
        assert!((inflator(1.0) - 1.0).abs() < 1.0e-6);
    }

    /// inflator() must produce its documented asymmetry at -1: f(-1) = -1.25.
    /// This is the source of the even-order harmonic content.
    #[test]
    fn inflator_asymmetry_at_minus_one() {
        let want = -1.25_f32;
        assert!(
            (inflator(-1.0) - want).abs() < 1.0e-6,
            "got {}, want {want}",
            inflator(-1.0)
        );
    }

    /// inflator() small-signal gain at the origin must be the documented
    /// A coefficient (1.5×). Approximate with a tiny derivative.
    #[test]
    fn inflator_small_signal_gain_is_one_point_five() {
        let dx = 1.0e-4_f32;
        let slope = (inflator(dx) - inflator(-dx)) / (2.0 * dx);
        assert!(
            (slope - 1.5).abs() < 1.0e-3,
            "small-signal slope {slope}, want 1.5"
        );
    }

    /// width_shelf_db_for(0) must produce 0 dB so the WIDTH stage is
    /// transparent at the minimum slider position (other than the HPF
    /// removing bass-side energy).
    #[test]
    fn width_zero_is_zero_db() {
        assert!(width_shelf_db_for(0.0).abs() < 1.0e-6);
    }

    /// width_shelf_db_for(1) must produce the expected +1.94 dB upper
    /// bound on the side-channel boost.
    #[test]
    fn width_one_is_one_point_nine_four_db() {
        let want = 20.0_f32 * (1.0_f32 + MAX_WIDTH_GAIN).log10();
        assert!((width_shelf_db_for(1.0) - want).abs() < 1.0e-6);
    }

    /// Master sheen_bypass must leave every input sample bit-identical.
    /// This is the only path that lets a user A/B against the dry signal.
    #[test]
    fn master_bypass_is_bit_exact() {
        let mut sheen = SheenModule::new(SR);
        sheen.update_parameters(
            true, // sheen_bypass = true
            3.0, false, 3.0, false, 4.0, false, 1.0, false, 1.0, false,
        );

        let n = 1024;
        let mut data_l: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.1).sin()).collect();
        let mut data_r: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.13).cos()).collect();
        let in_l = data_l.clone();
        let in_r = data_r.clone();

        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut data_l);
                slices.push(&mut data_r);
            });
        }
        sheen.process(&mut buffer);

        for i in 0..n {
            assert_eq!(data_l[i], in_l[i], "L drifted at {i} under bypass");
            assert_eq!(data_r[i], in_r[i], "R drifted at {i} under bypass");
        }
    }

    /// All five per-stage bypasses on + master ON must also be bit-exact.
    /// This catches stages that accidentally mutate the buffer even when
    /// their per-stage bypass should skip the work.
    #[test]
    fn all_stages_bypassed_is_bit_exact() {
        let mut sheen = SheenModule::new(SR);
        sheen.update_parameters(
            false, // sheen master ON
            3.0, true, // body bypassed
            3.0, true, // presence bypassed
            4.0, true, // air bypassed
            1.0, true, // warmth bypassed
            1.0, true, // width bypassed
        );

        let n = 512;
        let mut data_l: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.07).sin()).collect();
        let mut data_r: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.11).cos()).collect();
        let in_l = data_l.clone();
        let in_r = data_r.clone();

        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut data_l);
                slices.push(&mut data_r);
            });
        }
        sheen.process(&mut buffer);

        for i in 0..n {
            assert_eq!(data_l[i], in_l[i], "L drifted at {i} all-stages-bypassed");
            assert_eq!(data_r[i], in_r[i], "R drifted at {i} all-stages-bypassed");
        }
    }

    /// Factory defaults must produce finite output (no NaN, no infinity)
    /// across a noisy multi-frequency input — guards against denormal
    /// blow-up in the cascaded biquads.
    #[test]
    fn factory_defaults_produce_finite_output() {
        let mut sheen = SheenModule::new(SR);
        // Spec defaults: body+1 dB, presence 0, air +1.8, warmth 0.2, width 0.5
        sheen.update_parameters(
            false, 1.0, false, 0.0, false, 1.8, false, 0.20, false, 0.50, false,
        );

        let n = 4096;
        let mut data_l: Vec<f32> = (0..n)
            .map(|i| 0.5 * ((i as f32) * 0.05).sin() + 0.2 * ((i as f32) * 0.31).cos())
            .collect();
        let mut data_r: Vec<f32> = (0..n)
            .map(|i| 0.5 * ((i as f32) * 0.07).cos() + 0.2 * ((i as f32) * 0.29).sin())
            .collect();

        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut data_l);
                slices.push(&mut data_r);
            });
        }
        sheen.process(&mut buffer);

        for (i, (&l, &r)) in data_l.iter().zip(data_r.iter()).enumerate() {
            assert!(l.is_finite(), "L not finite at {i}: {l}");
            assert!(r.is_finite(), "R not finite at {i}: {r}");
            assert!(l.abs() < 4.0, "L exploded at {i}: {l}");
            assert!(r.abs() < 4.0, "R exploded at {i}: {r}");
        }
    }
}
