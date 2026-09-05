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
//!      -> WARMTH (Inflator polynomial @ 4x oversample)
//!      -> WIDTH (M/S side-only HPF + high shelf)
//!      -> [out]
//! ```
//!
//! Stage rationale and citations live in `docs/adr/0006-sheen-pinned-master-end-polish.md`.

use crate::oversampler::Oversampler;
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

/// 4× oversampling for the warmth shaper, routed through the same
/// Kaiser-windowed halfband FIR cascade (`crate::oversampler::Oversampler`)
/// used by ButterComp2/Pultec/Transformer — the v2.0 DSP-track floor for
/// every nonlinear module (see `docs/V2_ROADMAP.md` §3.1, tracked in #14).
/// The polynomial generates 4th-order products (cubic+quartic); 4× keeps
/// their alias well clear of the audible band with real stopband rejection
/// instead of the legacy 2×/linear-interp scheme's crude suppression.
const OS_FACTOR: usize = 4;

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

    // WARMTH oversamplers — one halfband-FIR Oversampler per channel, run
    // inline (one sample in → OS_FACTOR samples out) like Pultec's tube
    // stage and Transformer's saturation core.
    warmth_os: [Oversampler; 2],
    /// Tracks whether the WARMTH stage actually ran on the previous buffer,
    /// so `process()` can flush `warmth_os`'s FIR delay lines exactly on the
    /// active→skipped transition rather than leaving them holding stale
    /// pre-gap content for the stage's eventual re-entry (see #27 review).
    warmth_was_active: bool,

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

        // Oversamplers are used inline (one sample in → OS_FACTOR samples
        // out), so `max_block_size = 1` keeps their scratch buffers minimal
        // — same convention as Pultec's tube stage and Transformer's core.
        let make_warmth_os = || Oversampler::new_at_factor(OS_FACTOR, 1);

        Self {
            sample_rate,
            body,
            presence,
            air,
            width_hpf: DirectForm1::<f32>::new(hpf_coeff),
            width_shelf: DirectForm1::<f32>::new(shelf_coeff),
            warmth_os: [make_warmth_os(), make_warmth_os()],
            warmth_was_active: false,
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

        // Computed once per buffer (params are buffer-granular, like every
        // other cached value here) so the per-sample loop below doesn't
        // re-evaluate it, and so the active→skipped transition can be
        // detected exactly once rather than per-sample.
        let warmth_active = !self.warmth_bypass && self.warmth_effect > 1.0e-6;
        if !warmth_active && self.warmth_was_active {
            // WARMTH just stopped running. Flush the FIR delay lines now so
            // that when the stage re-activates, it resumes from silence
            // instead of convolving fresh input against several-sample-old
            // pre-gap content — the latter reads as an audible click on
            // re-entry with this cascade's ~46-tap history depth.
            for os in &mut self.warmth_os {
                os.reset();
            }
        }
        self.warmth_was_active = warmth_active;

        let mut warmth_scratch = [0.0_f32; OS_FACTOR];

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

            // ── WARMTH ─ Inflator-style polynomial @ 4× oversample ──────
            // Skip the whole stage when effect is at-or-below noise floor;
            // saves the polynomial and the oversampler hop on the dry path.
            if warmth_active {
                l = self.process_warmth(l, 0, &mut warmth_scratch);
                r = self.process_warmth(r, 1, &mut warmth_scratch);
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

    /// Reset the warmth oversamplers' halfband-FIR delay lines. Biquad
    /// state is left to settle naturally with silence input — DirectForm1
    /// has no public reset and rebuilding the filters mid-process would
    /// require re-running `biquad_coeffs` (sin/cos), which we'd rather
    /// avoid. In practice the host calls `reset()` on transport start
    /// where the buffer leading edge is silence anyway, so any residual
    /// filter energy decays within ~100 samples for our chosen Q values.
    pub fn reset(&mut self) {
        for os in &mut self.warmth_os {
            os.reset();
        }
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

    /// 4× oversampled Inflator pass for one channel. Upsamples through the
    /// shared Kaiser-windowed halfband FIR cascade, applies the Inflator
    /// transfer function (with per-sample wet/dry mix) at the 4× rate, then
    /// downsamples back — same pattern as Pultec's tube stage and
    /// Transformer's saturation core, replacing the legacy 2×
    /// linear-interpolation-up / 1-pole-IIR-down scheme.
    #[inline]
    fn process_warmth(&mut self, x: f32, ch: usize, scratch: &mut [f32; OS_FACTOR]) -> f32 {
        let mix = self.warmth_effect;
        let dry = 1.0 - mix;

        {
            let up = self.warmth_os[ch].upsample(x, 0);
            for i in 0..OS_FACTOR {
                scratch[i] = dry * up[i] + mix * inflator(up[i]);
            }
        }
        self.warmth_os[ch].downsample(&scratch[..OS_FACTOR], 0)
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

// Compile-time assertion that the oversample factor stays a value the
// Oversampler cascade (crate::oversampler) actually supports (1/2/4/8/16).
const _: () = assert!(
    OS_FACTOR == 4,
    "WARMTH oversampler is the v2.0 4x floor (#14)"
);

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

    /// v2.0 DSP track (#14): WARMTH must run at the 4× internal-oversampling
    /// floor like every other nonlinear module (ButterComp2/Pultec/Transformer
    /// already do), not the legacy 2× rate.
    #[test]
    fn warmth_oversample_factor_is_four() {
        assert_eq!(OS_FACTOR, 4, "WARMTH must run at the v2.0 4x floor (#14)");
    }

    /// Hit the WARMTH oversampler with a hot, near-Nyquist, full-effect
    /// signal and verify the output stays finite and bounded — guards
    /// against FIR state corruption or overflow from the 4× halfband
    /// cascade, mirroring the equivalent tests in pultec.rs/transformer.rs.
    #[test]
    fn warmth_saturation_oversampled_bounded() {
        let mut sheen = SheenModule::new(SR);
        // Flat EQ stages, WARMTH pushed to full effect.
        sheen.update_parameters(
            false, 0.0, true, 0.0, true, 0.0, true, 1.0, false, 0.0, true,
        );

        let n = 2048;
        let mut data_l: Vec<f32> = (0..n)
            .map(|i| (2.0 * core::f32::consts::PI * 0.45 * i as f32).sin())
            .collect();
        let mut data_r: Vec<f32> = (0..n)
            .map(|i| (2.0 * core::f32::consts::PI * 0.47 * i as f32).sin())
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
            assert!(l.is_finite(), "L non-finite at {i}: {l}");
            assert!(r.is_finite(), "R non-finite at {i}: {r}");
            assert!(l.abs() < 4.0, "L implausibly large at {i}: {l}");
            assert!(r.abs() < 4.0, "R implausibly large at {i}: {r}");
        }
    }

    /// Regression test for the PR #27 review finding: skipping WARMTH
    /// (bypass or near-zero effect) must flush `warmth_os`'s FIR delay
    /// lines, not just stop pushing new samples into them. Fill the stage
    /// with a hot signal, bypass it for one buffer (the active→skipped
    /// transition that must trigger the flush), then re-activate on pure
    /// silence: with `dry = 1.0 - mix = 0.0` (full-wet), a correctly-reset
    /// oversampler convolving zero input against a zeroed delay line must
    /// produce bit-exact silence. Stale pre-bypass delay-line content
    /// would instead leak through as nonzero output on re-activation.
    #[test]
    fn warmth_bypass_reactivation_flushes_stale_oversampler_state() {
        let mut sheen = SheenModule::new(SR);
        let n = 256;

        // Phase 1: WARMTH active, hot near-Nyquist signal — fills the FIR
        // delay lines with substantial energy.
        sheen.update_parameters(
            false, 0.0, true, 0.0, true, 0.0, true, 1.0, false, 0.0, true,
        );
        let mut hot_l: Vec<f32> = (0..n)
            .map(|i| (2.0 * core::f32::consts::PI * 0.45 * i as f32).sin())
            .collect();
        let mut hot_r = hot_l.clone();
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut hot_l);
                slices.push(&mut hot_r);
            });
        }
        sheen.process(&mut buffer);

        // Phase 2: bypass WARMTH for one buffer — this is the
        // active→skipped transition that must trigger the flush.
        sheen.update_parameters(false, 0.0, true, 0.0, true, 0.0, true, 1.0, true, 0.0, true);
        let mut silent_l = vec![0.0_f32; n];
        let mut silent_r = vec![0.0_f32; n];
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut silent_l);
                slices.push(&mut silent_r);
            });
        }
        sheen.process(&mut buffer);

        // Phase 3: re-activate WARMTH on pure silence.
        sheen.update_parameters(
            false, 0.0, true, 0.0, true, 0.0, true, 1.0, false, 0.0, true,
        );
        let mut check_l = vec![0.0_f32; n];
        let mut check_r = vec![0.0_f32; n];
        let mut buffer = Buffer::default();
        unsafe {
            buffer.set_slices(n, |slices| {
                slices.clear();
                slices.push(&mut check_l);
                slices.push(&mut check_r);
            });
        }
        sheen.process(&mut buffer);

        for (i, (&l, &r)) in check_l.iter().zip(check_r.iter()).enumerate() {
            assert_eq!(l, 0.0, "L leaked stale oversampler energy at {i}: {l}");
            assert_eq!(r, 0.0, "R leaked stale oversampler energy at {i}: {r}");
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
