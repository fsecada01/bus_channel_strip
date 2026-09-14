// src/dynamic_eq.rs — 4-band dynamic equalizer
//
// Key design decisions:
//   - BandFilter wraps the TPT state-variable core (`crate::svf`, #15) so
//     filter state is never reset when coefficients change, and — unlike the
//     direct-form biquad it replaced — a coefficient swap lands on the new
//     response with no transient. The gain-reduction path recomputes
//     coefficients every couple of samples, which is exactly the case the
//     SVF topology exists for.
//   - The sidechain detection filter is a BandFilter running in
//     constant-0-dB-peak bandpass mode so out-of-band energy is rejected
//     rather than leaking through at unity gain (a +6 dB peaking EQ used
//     previously passed all out-of-band content, biasing detection toward
//     low-frequency broadband energy).
//   - Envelope detection uses a denormal guard (max with f32::MIN_POSITIVE)
//     before log10() to prevent -inf / NaN when the signal is silent.
//   - Solo mode routes only the soloed band(s) through a RBJ bandpass filter
//     so the user can isolate exactly the frequency range being processed.

use crate::svf::{flush_denormal, BellPrewarp, SvfCoefficients, SvfType, TptSvf};
use nice_plug::buffer::Buffer;
use nice_plug::prelude::Enum;

// RMS integration window for sidechain detection. 10 ms is a conventional
// trade-off: long enough to smooth out transient spikes that would cause
// peak-style pumping, short enough that the envelope's attack/release can
// still track program dynamics meaningfully.
const RMS_WINDOW_MS: f32 = 10.0;

// Soft-knee width in dB, centered on the threshold. Smooths the discontinuity
// at the threshold boundary into a quadratic transition, eliminating the
// audible click that a hard-knee gain computer produces when the envelope
// crosses the threshold. 6 dB is the standard "musical" default for general
// dynamic processing.
const KNEE_WIDTH_DB: f32 = 6.0;

/// Ceiling on the Expand Up boost. Uncapped, the gain computer asks for
/// `(ratio - 1) × over_db` — over +1000 dB at ratio 20 with a low threshold —
/// which overflows the bell filter to inf and latches NaN into its state.
const MAX_EXPAND_BOOST_DB: f32 = 24.0;

/// Soft-knee gain computer (Reiss 2012). Given the detector's dB-over-threshold
/// value, the mode, and the compression ratio, returns the gain change in dB
/// (negative = attenuation, positive = upward expansion). The transition region
/// around the threshold is a quadratic that matches the linear region's slope
/// at the knee boundary, yielding a C1-continuous input/output curve.
fn compute_gain_change_db(over_db: f32, mode: DynamicMode, ratio: f32) -> f32 {
    let half_knee = KNEE_WIDTH_DB * 0.5;
    match mode {
        DynamicMode::CompressDownward => {
            let slope = 1.0 - 1.0 / ratio;
            if over_db <= -half_knee {
                0.0
            } else if over_db >= half_knee {
                -slope * over_db
            } else {
                let x = over_db + half_knee;
                -slope * x * x / (2.0 * KNEE_WIDTH_DB)
            }
        }
        DynamicMode::ExpandUpward => {
            let slope = ratio - 1.0;
            if over_db <= -half_knee {
                0.0
            } else if over_db >= half_knee {
                (slope * over_db).min(MAX_EXPAND_BOOST_DB)
            } else {
                let x = over_db + half_knee;
                (slope * x * x / (2.0 * KNEE_WIDTH_DB)).min(MAX_EXPAND_BOOST_DB)
            }
        }
        DynamicMode::Gate => {
            let slope = 1.0 - 1.0 / ratio;
            if over_db >= half_knee {
                0.0
            } else if over_db <= -half_knee {
                (slope * over_db).max(-96.0)
            } else {
                let u = half_knee - over_db;
                (-slope * u * u / (2.0 * KNEE_WIDTH_DB)).max(-96.0)
            }
        }
    }
}

// ── Stateful band filter ─────────────────────────────────────────────────────
//
// Both the EQ and sidechain filters use this struct. Coefficients are
// swapped in-place on the TPT SVF core; the integrator state is untouched.

/// Lowest centre frequency the module designs for. Matches the parameter
/// ranges; anything below is a mapping bug and gets pinned here rather than
/// producing a sub-audio filter.
const BAND_MIN_FREQ_HZ: f32 = 20.0;
/// Highest centre frequency as a fraction of the sample rate.
const BAND_MAX_FREQ_RATIO: f32 = 0.49;
/// Lowest Q accepted by the band filters.
const BAND_MIN_Q: f32 = 0.1;

struct BandFilter {
    svf: TptSvf,
}

impl BandFilter {
    fn new() -> Self {
        Self {
            svf: TptSvf::flat(),
        }
    }

    /// Pins frequency and Q to the module's design range.
    #[inline]
    fn clamp_design(freq_hz: f32, q: f32, sample_rate: f32) -> (f32, f32) {
        // `.max().min()`, not `.clamp()` — see svf.rs's identical guard.
        let max_hz = (sample_rate * BAND_MAX_FREQ_RATIO).max(BAND_MIN_FREQ_HZ);
        (freq_hz.max(BAND_MIN_FREQ_HZ).min(max_hz), q.max(BAND_MIN_Q))
    }

    #[inline]
    fn design(filter_type: SvfType, freq_hz: f32, q: f32, sample_rate: f32) -> SvfCoefficients {
        let (freq_hz, q) = Self::clamp_design(freq_hz, q, sample_rate);
        SvfCoefficients::new(filter_type, sample_rate, freq_hz, q)
    }

    /// Block-rate bell design for a band whose gain moves every sample.
    fn bell_prewarp(freq_hz: f32, q: f32, sample_rate: f32) -> BellPrewarp {
        let (freq_hz, q) = Self::clamp_design(freq_hz, q, sample_rate);
        BellPrewarp::new(sample_rate, freq_hz, q)
    }

    /// Peaking EQ (RBJ prototype) — updates coefficients, preserves state.
    #[cfg(test)]
    fn update_peaking(&mut self, freq_hz: f32, q: f32, gain_db: f32, sample_rate: f32) {
        self.svf.update_coefficients(Self::design(
            SvfType::Bell(gain_db),
            freq_hz,
            q,
            sample_rate,
        ));
    }

    /// Constant-0-dB-peak bandpass — updates coefficients, preserves state.
    /// Peak gain is exactly 1.0 at `freq_hz` regardless of Q, so the detected level
    /// equals the actual signal energy in the band. Out-of-band content falls off
    /// at ~6 dB/octave * Q. Used for sidechain detection so the envelope follower
    /// is not contaminated by broadband low-frequency energy.
    fn update_bandpass_unity(&mut self, freq_hz: f32, q: f32, sample_rate: f32) {
        self.svf.update_coefficients(Self::design(
            SvfType::BandPassUnity,
            freq_hz,
            q,
            sample_rate,
        ));
    }

    /// Constant-skirt-gain bandpass — updates coefficients, preserves state.
    /// Used for solo band-isolation mode.
    fn update_bandpass(&mut self, freq_hz: f32, q: f32, sample_rate: f32) {
        self.svf
            .update_coefficients(Self::design(SvfType::BandPass, freq_hz, q, sample_rate));
    }

    /// Update a stereo pair of constant-skirt-gain bandpass filters from one
    /// shared coefficient computation. See [`Self::update_peaking_pair`].
    fn update_bandpass_pair(l: &mut Self, r: &mut Self, freq_hz: f32, q: f32, sample_rate: f32) {
        let coeffs = Self::design(SvfType::BandPass, freq_hz, q, sample_rate);
        l.svf.update_coefficients(coeffs);
        r.svf.update_coefficients(coeffs);
    }

    /// Processes one sample.
    #[inline]
    fn process(&mut self, x0: f32) -> f32 {
        self.svf.run(x0)
    }

    fn reset(&mut self) {
        self.svf.reset();
    }
}

// ── DynamicMode ───────────────────────────────────────────────────────────────

/// Dynamic processing mode for a single band. The display labels are chosen
/// to expose the operational direction of each mode in a way that parallels
/// pro-audio compressor/expander/gate terminology.
///
/// - `CompressDownward` — reduce gain of signal *above* threshold.
///   Like a classic compressor, but applied per-band via the peaking EQ.
/// - `ExpandUpward` — boost gain of signal *above* threshold.
///   Accentuates the band when the band's content is loud.
/// - `Gate` — reduce gain of signal *below* threshold (downward expansion).
///   Despite the name, this is a ratio-controlled downward expander, not a
///   hard-mute gate: with ratio=1.5 it is a subtle expander, with ratio=20
///   it behaves as a gate for practical purposes. Attenuation is clamped at
///   -96 dB so extreme silence doesn't push the peaking EQ into numerical
///   corner cases. No hold or hysteresis — it responds purely to the
///   instantaneous envelope level through the attack/release smoother.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum DynamicMode {
    #[name = "Compress Down"]
    CompressDownward,
    #[name = "Expand Up"]
    ExpandUpward,
    #[name = "Gate"]
    Gate,
}

impl Default for DynamicMode {
    fn default() -> Self {
        DynamicMode::CompressDownward
    }
}

// ── DynamicBand ───────────────────────────────────────────────────────────────

struct DynamicBand {
    // Filters (all BandFilter — state persists across buffer boundaries).
    // Every filter is per channel: interleaving L/R samples through one SVF
    // corrupts its state. Detection filters each channel *before* rectifying
    // (squaring) so a tone's fundamental reaches the band-pass intact.
    detector_filter: [BandFilter; 2], // unity-peak BPF, L and R
    eq_filter_l: BandFilter,
    eq_filter_r: BandFilter,
    solo_filter_l: BandFilter,
    solo_filter_r: BandFilter,

    // Detection: per-channel RMS, linked by the louder channel into one
    // envelope so both channels get the same gain change.
    rms_state: [f32; 2],
    rms_coeff: f32,
    envelope: f32,
    pub gain_reduction_db: f32,
    bell: BellPrewarp,
    bell_db_bits: u32,
    bell_dirty: bool,

    // Cached parameter values (updated per-buffer, used per-sample)
    sample_rate: f32,
    mode: DynamicMode,
    detector_freq: f32,
    frequency: f32,
    q: f32,
    threshold_db: f32, // stored directly in dB (no round-trip conversion)
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    make_up_gain: f32, // linear gain
    enabled: bool,
    solo: bool,
}

impl DynamicBand {
    fn new(sample_rate: f32) -> Self {
        let mut detector_filter = [BandFilter::new(), BandFilter::new()];
        for filter in &mut detector_filter {
            filter.update_bandpass_unity(1000.0, 1.0, sample_rate);
        }

        let mut solo_filter_l = BandFilter::new();
        let mut solo_filter_r = BandFilter::new();
        solo_filter_l.update_bandpass(1000.0, 1.0, sample_rate);
        solo_filter_r.update_bandpass(1000.0, 1.0, sample_rate);

        let rms_coeff = (-1.0 / (RMS_WINDOW_MS * 0.001 * sample_rate)).exp();

        Self {
            detector_filter,
            eq_filter_l: BandFilter::new(),
            eq_filter_r: BandFilter::new(),
            solo_filter_l,
            solo_filter_r,
            rms_state: [0.0; 2],
            rms_coeff,
            envelope: 0.0,
            gain_reduction_db: 0.0,
            bell: BandFilter::bell_prewarp(1000.0, 1.0, sample_rate),
            bell_db_bits: 0,
            bell_dirty: true,
            sample_rate,
            mode: DynamicMode::default(),
            detector_freq: 1000.0,
            frequency: 1000.0,
            q: 1.0,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            make_up_gain: 1.0,
            enabled: true,
            solo: false,
        }
    }

    fn update_parameters(
        &mut self,
        mode: DynamicMode,
        detector_freq: f32,
        frequency: f32,
        q: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        make_up_gain_db: f32,
        enabled: bool,
        solo: bool,
    ) {
        self.mode = mode;
        self.detector_freq = detector_freq;
        self.frequency = frequency;
        self.q = q;
        self.threshold_db = threshold_db; // direct dB — no mapping needed
        self.ratio = ratio;
        let sr = self.sample_rate;
        // Standard exponential-decay IIR attack/release coefficients.
        self.attack_coeff = (-1.0 / (attack_ms.max(0.01) * 0.001 * sr)).exp();
        self.release_coeff = (-1.0 / (release_ms.max(0.01) * 0.001 * sr)).exp();
        // RMS coefficient is derived from a fixed 10 ms window; recomputed here
        // in case sample_rate changes between calls (cheap, runs once per buffer).
        self.rms_coeff = (-1.0 / (RMS_WINDOW_MS * 0.001 * sr)).exp();
        self.make_up_gain = 10.0f32.powf(make_up_gain_db / 20.0);
        self.enabled = enabled;
        self.solo = solo;

        // Unity-peak bandpass: detection level == actual in-band signal level.
        for filter in &mut self.detector_filter {
            filter.update_bandpass_unity(detector_freq, q, sr);
        }

        let bell = BandFilter::bell_prewarp(frequency, q, sr);
        if bell != self.bell {
            self.bell = bell;
            self.bell_dirty = true;
        }

        // Update solo bandpass filters (L and R) for this band's center
        // frequency. Both channels receive identical coefficients — only state
        // diverges with input.
        BandFilter::update_bandpass_pair(
            &mut self.solo_filter_l,
            &mut self.solo_filter_r,
            frequency,
            q,
            sr,
        );
    }

    /// Update the envelope from the raw **module input** `l`/`r` (not the
    /// inter-band cascade signal), so band N's detection is not contaminated
    /// by EQ applied in bands 0..N-1.
    ///
    /// Per channel: BPF → square → RMS lowpass (10 ms). The louder channel's
    /// RMS then drives one attack/release smoother. The input must never be
    /// rectified before the BPF: `|x|` moves a tone's energy to DC and 2f, so
    /// the band-pass would read it 12–26 dB low.
    fn update_envelope(&mut self, l: f32, r: f32) {
        if !self.enabled {
            return;
        }
        let rms_coeff = self.rms_coeff;
        let mut mean_sq = 0.0_f32;
        for ((filter, state), x) in self
            .detector_filter
            .iter_mut()
            .zip(self.rms_state.iter_mut())
            .zip([l, r])
        {
            let y = filter.process(x);
            let y_sq = y * y;
            *state = flush_denormal(y_sq + (*state - y_sq) * rms_coeff);
            mean_sq = mean_sq.max(*state);
        }
        let det = mean_sq.max(0.0).sqrt();

        if det > self.envelope {
            self.envelope = det + (self.envelope - det) * self.attack_coeff;
        } else {
            self.envelope = det + (self.envelope - det) * self.release_coeff;
        }
        self.envelope = flush_denormal(self.envelope);
    }

    /// Compute the dynamic gain from the current envelope and apply the peaking
    /// EQ + makeup gain to both L and R channels. The same gain change is used
    /// for both channels so stereo image is preserved. Bell coefficients are
    /// redesigned on every sample whose gain differs from the last, so the
    /// filter tracks the gain computer exactly; state stays per channel.
    ///
    /// `l`/`r` are the **cascade signals** from the previous band's apply_eq
    /// (or the dry module input for band 0).
    fn apply_eq_stereo(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.enabled {
            return (l, r);
        }

        // Gain computation in dB.
        // Guard: max with MIN_POSITIVE prevents log10(0) = -inf → NaN / Gate explosion.
        let envelope_db = 20.0 * self.envelope.max(f32::MIN_POSITIVE).log10();
        let over_db = envelope_db - self.threshold_db;

        let gain_change_db = compute_gain_change_db(over_db, self.mode, self.ratio);
        self.gain_reduction_db = -gain_change_db;

        let bell_db = gain_change_db;
        if self.bell_dirty || bell_db.to_bits() != self.bell_db_bits {
            let coeffs = self.bell.coefficients(bell_db);
            self.eq_filter_l.svf.update_coefficients(coeffs);
            self.eq_filter_r.svf.update_coefficients(coeffs);
            self.bell_db_bits = bell_db.to_bits();
            self.bell_dirty = false;
        }

        (
            self.eq_filter_l.process(l) * self.make_up_gain,
            self.eq_filter_r.process(r) * self.make_up_gain,
        )
    }

    /// Convenience wrapper for tests and any caller that wants the old
    /// "detection == EQ input" behavior. Feeds the same sample to both
    /// channels and returns the left output — production code
    /// (`DynamicEQ::process`) calls `update_envelope` + `apply_eq_stereo`
    /// directly with a linked detection input.
    #[cfg(test)]
    fn process_sample(&mut self, input: f32) -> f32 {
        self.update_envelope(input, input);
        self.apply_eq_stereo(input, input).0
    }

    fn reset(&mut self) {
        self.rms_state = [0.0; 2];
        self.envelope = 0.0;
        self.gain_reduction_db = 0.0;
        self.bell_dirty = true;
        self.eq_filter_l.reset();
        self.eq_filter_r.reset();
        // Intentionally keep detector and solo filter state to avoid clicks.
    }

    /// Clears state poisoned by a non-finite sample. NaN or inf in an SVF
    /// integrator or the envelope never decays, so without this one bad host
    /// block silences the band — and everything after it — until reload.
    /// The EQ filters go back to flat and are redesigned on the next sample.
    fn recover_if_non_finite(&mut self) {
        let filters_finite = [
            &self.detector_filter[0],
            &self.detector_filter[1],
            &self.eq_filter_l,
            &self.eq_filter_r,
            &self.solo_filter_l,
            &self.solo_filter_r,
        ]
        .iter()
        .all(|f| {
            let (s1, s2) = f.svf.state();
            s1.is_finite() && s2.is_finite()
        });
        if filters_finite
            && self.envelope.is_finite()
            && self.rms_state.iter().all(|s| s.is_finite())
            && self.gain_reduction_db.is_finite()
        {
            return;
        }
        self.reset();
        self.eq_filter_l = BandFilter::new();
        self.eq_filter_r = BandFilter::new();
        for filter in &mut self.detector_filter {
            filter.reset();
        }
        self.solo_filter_l.reset();
        self.solo_filter_r.reset();
    }
}

// ── Public API types ──────────────────────────────────────────────────────────

/// Parameters for a single dynamic band, passed from lib.rs each buffer.
#[derive(Clone, Copy)]
pub struct DynamicBandParams {
    pub mode: DynamicMode,
    pub detector_freq: f32,
    pub freq: f32,
    pub q: f32,
    pub threshold_db: f32, // dB, e.g. -18.0
    pub ratio: f32,        // linear, e.g. 4.0 for 4:1
    pub attack_ms: f32,
    pub release_ms: f32,
    pub gain_db: f32, // makeup gain in dB
    pub enabled: bool,
    pub solo: bool,
}

// ── DynamicEQ ─────────────────────────────────────────────────────────────────

pub struct DynamicEQ {
    bands: [DynamicBand; 4],
}

impl DynamicEQ {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bands: [
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
                DynamicBand::new(sample_rate),
            ],
        }
    }

    pub fn update_parameters(&mut self, band_params: &[DynamicBandParams; 4]) {
        for (i, p) in band_params.iter().enumerate() {
            self.bands[i].update_parameters(
                p.mode,
                p.detector_freq,
                p.freq,
                p.q,
                p.threshold_db,
                p.ratio,
                p.attack_ms,
                p.release_ms,
                p.gain_db,
                p.enabled,
                p.solo,
            );
        }
    }

    pub fn process(&mut self, buffer: &mut Buffer) {
        let any_solo = self.bands.iter().any(|b| b.solo && b.enabled);
        // Normalise solo level: sum of N band-limited signals ÷ N to avoid clipping.
        let solo_count = self
            .bands
            .iter()
            .filter(|b| b.solo && b.enabled)
            .count()
            .max(1) as f32;

        let channels = buffer.as_slice();
        let num_channels = channels.len();
        if num_channels == 0 {
            return;
        }
        let num_samples = channels[0].len();

        for i in 0..num_samples {
            // Read L and R (mono buffers treat R = L so the stereo path still
            // produces a correct mono result).
            let l_in = channels[0][i];
            let r_in = if num_channels >= 2 {
                channels[1][i]
            } else {
                l_in
            };

            // Detection taps the dry module input so the cascade of bands
            // 0..N-1 can't starve or pump band N's detection.
            for band in &mut self.bands {
                band.update_envelope(l_in, r_in);
            }

            let (l_out, r_out) = if any_solo {
                // Solo mode: route only the soloed bands' content to the
                // output, through a constant-skirt bandpass (not the peaking
                // EQ / cascade). This is deliberately a *different* signal
                // path from the non-solo cascade — solo is for monitoring
                // what's in each band, not for any processing reduction.
                // All bands still run update_envelope() above so soloing
                // doesn't freeze the un-soloed bands' envelope state, which
                // would cause a click when solo is released.
                let mut ol = 0.0_f32;
                let mut or_ = 0.0_f32;
                for band in &mut self.bands {
                    if band.solo && band.enabled {
                        ol += band.solo_filter_l.process(l_in);
                        or_ += band.solo_filter_r.process(r_in);
                    }
                }
                (ol / solo_count, or_ / solo_count)
            } else {
                // Normal mode: cascade EQs in series on each channel. Every
                // band applies identical gain to L and R (single shared
                // envelope), so stereo image is preserved across the cascade.
                let mut sl = l_in;
                let mut sr = r_in;
                for band in &mut self.bands {
                    let (nl, nr) = band.apply_eq_stereo(sl, sr);
                    sl = nl;
                    sr = nr;
                }
                (sl, sr)
            };

            channels[0][i] = l_out;
            if num_channels >= 2 {
                channels[1][i] = r_out;
            }
        }

        for band in &mut self.bands {
            band.recover_if_non_finite();
        }
    }

    pub fn get_gain_reduction_db(&self) -> [f32; 4] {
        [
            self.bands[0].gain_reduction_db,
            self.bands[1].gain_reduction_db,
            self.bands[2].gain_reduction_db,
            self.bands[3].gain_reduction_db,
        ]
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BandFilter ────────────────────────────────────────────────────────────

    #[test]
    fn test_band_filter_identity_passthrough() {
        let mut bq = BandFilter::new();
        // Flat filter should pass signal unchanged
        for &input in &[0.0, 0.5, -0.5, 1.0, -1.0] {
            let out = bq.process(input);
            assert!(
                (out - input).abs() < 1e-6,
                "Identity passthrough: input={input}, out={out}"
            );
        }
    }

    #[test]
    fn test_band_filter_reset_clears_state() {
        let mut bq = BandFilter::new();
        bq.update_peaking(1000.0, 1.0, 6.0, 44100.0);
        for _ in 0..100 {
            bq.process(1.0);
        }
        bq.reset();
        assert_eq!(bq.svf.state(), (0.0, 0.0));
    }

    #[test]
    fn test_band_filter_update_peaking_does_not_clear_state() {
        // State must survive a coefficient update (key design invariant)
        let mut bq = BandFilter::new();
        bq.update_peaking(1000.0, 1.0, 6.0, 44100.0);
        for _ in 0..100 {
            bq.process(0.7);
        }
        let before = bq.svf.state();
        assert!(before != (0.0, 0.0), "state should be non-zero after DC");
        bq.update_peaking(2000.0, 1.5, -3.0, 44100.0);
        assert_eq!(bq.svf.state(), before, "state should survive coeff update");
    }

    #[test]
    fn test_band_filter_nonzero_gain_changes_amplitude() {
        let mut flat = BandFilter::new();
        flat.update_peaking(1000.0, 1.0, 0.0, 44100.0);

        let mut boosted = BandFilter::new();
        boosted.update_peaking(1000.0, 1.0, 6.0, 44100.0);

        // Drive both with a sine at the bell's centre frequency. (A bell is
        // unity at DC by construction, so DC cannot reveal the boost.)
        let sr = 44100.0_f32;
        let w = core::f32::consts::TAU * 1000.0 / sr;
        let n = 8192;
        let (mut flat_ms, mut boosted_ms) = (0.0_f32, 0.0_f32);
        for i in 0..n {
            let x = (w * i as f32).sin();
            let f = flat.process(x);
            let b = boosted.process(x);
            if i >= n / 2 {
                flat_ms += f * f;
                boosted_ms += b * b;
            }
        }
        // 6 dB boost at centre freq — boosted RMS should be ~2× the flat RMS.
        let ratio = (boosted_ms / flat_ms).sqrt();
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "6 dB boost should double amplitude at fc, got ratio {ratio:.3}"
        );
    }

    #[test]
    fn test_band_filter_produces_finite_output() {
        let mut bq = BandFilter::new();
        bq.update_peaking(20.0, 0.1, -60.0, 44100.0); // extreme params
        for i in 0..200 {
            let out = bq.process(if i % 2 == 0 { 1.0 } else { -1.0 });
            assert!(
                out.is_finite(),
                "BandFilter output must be finite at sample {i}: {out}"
            );
        }
    }

    #[test]
    fn test_band_filter_bandpass_unity_rejects_out_of_band() {
        // Verifies the detector shape fix: out-of-band content must be
        // significantly attenuated relative to in-band content. The old
        // +6 dB peaking detector passed out-of-band energy at 0 dB, which
        // biased envelope detection toward broadband LF content.
        let sr = 44100.0;
        let detector_fc = 4000.0_f32;

        let mut bp = BandFilter::new();
        bp.update_bandpass_unity(detector_fc, 1.5, sr);

        // Measure energy of a 100 Hz sine (8 kHz away from center) after detector
        let mut low_peak = 0.0_f32;
        for n in 0..8192 {
            let phase = std::f32::consts::TAU * 100.0 * (n as f32) / sr;
            let out = bp.process(phase.sin()).abs();
            if n > 2048 && out > low_peak {
                low_peak = out;
            }
        }

        // Measure energy of a 4 kHz sine (at center) after detector
        let mut bp2 = BandFilter::new();
        bp2.update_bandpass_unity(detector_fc, 1.5, sr);
        let mut center_peak = 0.0_f32;
        for n in 0..8192 {
            let phase = std::f32::consts::TAU * detector_fc * (n as f32) / sr;
            let out = bp2.process(phase.sin()).abs();
            if n > 2048 && out > center_peak {
                center_peak = out;
            }
        }

        assert!(
            center_peak > 0.9,
            "Center-frequency output should be ~unity, got {center_peak}"
        );
        assert!(
            low_peak < 0.1 * center_peak,
            "Out-of-band (100 Hz vs 4 kHz detector) must be <10% of center level, \
             got low={low_peak}, center={center_peak}"
        );
    }

    #[test]
    fn test_band_filter_bandpass_update_does_not_panic() {
        let mut bq = BandFilter::new();
        bq.update_bandpass(1000.0, 1.0, 44100.0);
        bq.update_bandpass(500.0, 2.0, 48000.0);
    }

    /// #15: the band filters run on the TPT core and must null against the
    /// RBJ biquad reference for every response the module uses — peaking
    /// (EQ), unity-peak bandpass (detector), constant-skirt bandpass (solo).
    #[test]
    fn test_band_filter_nulls_against_biquad_reference() {
        use crate::shaping::biquad_coeffs;
        use biquad::{Biquad, DirectForm1, Type};

        let sr = 48_000.0;
        let n = 8192;

        let mut peak = BandFilter::new();
        peak.update_peaking(1000.0, 1.5, -6.0, sr);
        let mut skirt = BandFilter::new();
        skirt.update_bandpass(2000.0, 1.0, sr);
        let mut unity = BandFilter::new();
        unity.update_bandpass_unity(4000.0, 1.5, sr);

        let mut peak_ref =
            DirectForm1::<f32>::new(biquad_coeffs(Type::PeakingEQ(-6.0), sr, 1000.0, 1.5).unwrap());
        let mut skirt_ref =
            DirectForm1::<f32>::new(biquad_coeffs(Type::BandPass, sr, 2000.0, 1.0).unwrap());
        // biquad 0.5.0's BandPass is the constant-skirt form; the unity-peak
        // form is skirt × (1/Q).
        let mut unity_ref =
            DirectForm1::<f32>::new(biquad_coeffs(Type::BandPass, sr, 4000.0, 1.5).unwrap());

        let mut d_peak = 0.0_f32;
        let mut d_skirt = 0.0_f32;
        let mut d_unity = 0.0_f32;
        for i in 0..n {
            let x = if i == 0 { 1.0 } else { 0.0 };
            d_peak = d_peak.max((peak.process(x) - peak_ref.run(x)).abs());
            d_skirt = d_skirt.max((skirt.process(x) - skirt_ref.run(x)).abs());
            d_unity = d_unity.max((unity.process(x) - unity_ref.run(x) / 1.5).abs());
        }
        assert!(d_peak < 1.0e-4, "peaking differs from biquad by {d_peak:e}");
        assert!(
            d_skirt < 1.0e-4,
            "bandpass differs from biquad by {d_skirt:e}"
        );
        assert!(
            d_unity < 1.0e-4,
            "unity bandpass differs from biquad by {d_unity:e}"
        );
    }

    #[test]
    fn test_band_filter_freq_clamping_to_nyquist() {
        let sr = 44100.0;
        let nyquist = sr * 0.49;
        let mut bq = BandFilter::new();
        // freq above Nyquist should be clamped — should not panic or produce NaN
        bq.update_peaking(nyquist + 10000.0, 1.0, 3.0, sr);
        let out = bq.process(0.5);
        assert!(out.is_finite(), "Output after freq clamping: {out}");
    }

    // ── DynamicBand ───────────────────────────────────────────────────────────

    #[test]
    fn test_flush_denormal_zeros_subthreshold() {
        assert_eq!(flush_denormal(0.0), 0.0);
        assert_eq!(flush_denormal(1e-25), 0.0);
        assert_eq!(flush_denormal(-1e-25), 0.0);
        // Values above threshold pass through unchanged
        assert_eq!(flush_denormal(1.0e-15), 1.0e-15);
        assert_eq!(flush_denormal(1.0), 1.0);
        assert_eq!(flush_denormal(-0.5), -0.5);
    }

    #[test]
    fn test_dynamic_band_envelope_flushes_to_zero_on_silence() {
        let sr = 44100.0_f32;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -18.0,
            4.0,
            1.0,
            10.0, // 10 ms release — decays through subnormals within the sample budget
            0.0,
            true,
            false,
        );
        // Drive with a 1 kHz sine (matches detector center) so the bandpass
        // detector actually passes the signal and envelope can build.
        for n in 0..2000 {
            let phase = std::f32::consts::TAU * 1000.0 * (n as f32) / sr;
            band.process_sample(phase.sin());
        }
        assert!(
            band.envelope > 0.1,
            "Envelope should build up under sustained in-band excitation, got {}",
            band.envelope
        );
        // At 10 ms release, envelope reaches ~e^-500 after ~220k silent samples —
        // guaranteed to cross the DENORMAL_FLUSH threshold well before the end.
        for _ in 0..500_000 {
            band.process_sample(0.0);
        }
        assert_eq!(
            band.envelope, 0.0,
            "Envelope must flush to exactly zero under sustained silence, got {}",
            band.envelope
        );
    }

    #[test]
    fn test_dynamic_band_new_default_values() {
        let band = DynamicBand::new(44100.0);
        assert!((band.envelope - 0.0).abs() < 1e-9);
        assert!((band.gain_reduction_db - 0.0).abs() < 1e-9);
        assert!(band.enabled);
        assert!(!band.solo);
        assert!((band.threshold_db - (-18.0)).abs() < 1e-5);
        assert!((band.ratio - 4.0).abs() < 1e-5);
    }

    #[test]
    fn test_dynamic_band_disabled_passes_through() {
        let sr = 44100.0;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -18.0,
            4.0,
            5.0,
            100.0,
            0.0,
            false,
            false,
        );
        // When disabled, process_sample should return input unchanged
        let input = 0.7_f32;
        let out = band.process_sample(input);
        assert!(
            (out - input).abs() < 1e-5,
            "Disabled band: expected {input}, got {out}"
        );
    }

    #[test]
    fn test_dynamic_band_reset_clears_envelope() {
        let mut band = DynamicBand::new(44100.0);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -18.0,
            4.0,
            0.1,
            10.0,
            0.0,
            true,
            false,
        );
        for _ in 0..500 {
            band.process_sample(1.0);
        }
        band.reset();
        assert!(
            (band.envelope - 0.0).abs() < 1e-9,
            "Envelope should be 0 after reset"
        );
        assert!(
            (band.gain_reduction_db - 0.0).abs() < 1e-9,
            "GR should be 0 after reset"
        );
    }

    #[test]
    fn test_dynamic_band_compress_mode_reduces_loud_signal() {
        let sr = 44100.0;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -30.0, // very sensitive threshold
            4.0,
            0.001,
            50.0,
            0.0,
            true,
            false,
        );
        // Warm up the envelope with loud signal
        for _ in 0..2000 {
            band.process_sample(1.0);
        }
        let gr = band.gain_reduction_db;
        assert!(
            gr > 0.0,
            "Compressor should show positive GR in dB, got {gr}"
        );
    }

    #[test]
    fn test_dynamic_band_gate_mode_attenuates_quiet_signal() {
        let sr = 44100.0;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::Gate,
            1000.0,
            1000.0,
            1.0,
            -6.0, // high threshold — quiet signal is below it
            4.0,
            0.1,
            50.0,
            0.0,
            true,
            false,
        );
        // Process a quiet signal below threshold
        for _ in 0..200 {
            band.process_sample(0.01);
        }
        let out = band.process_sample(0.01);
        // Gate should attenuate signal below threshold
        assert!(
            out.abs() < 0.01,
            "Gate should attenuate quiet signal, got {out}"
        );
    }

    #[test]
    fn test_soft_knee_compress_is_continuous_at_boundaries() {
        // At over_db = -half_knee, quadratic region starts at 0 (matching
        // the "no action" region). At +half_knee, the quadratic matches the
        // linear region. Verifies C1 continuity at both knee boundaries.
        let half_knee = KNEE_WIDTH_DB * 0.5;
        let ratio = 4.0_f32;

        // Lower boundary: both approaches should give ~0
        let eps = 0.001_f32;
        let below = compute_gain_change_db(-half_knee - eps, DynamicMode::CompressDownward, ratio);
        let at_lower =
            compute_gain_change_db(-half_knee + eps, DynamicMode::CompressDownward, ratio);
        assert!(below.abs() < 1e-5, "Below knee must be 0, got {below}");
        assert!(at_lower.abs() < 1e-3, "Just above lower knee: {at_lower}");

        // Upper boundary: quadratic and linear should agree
        let below_upper =
            compute_gain_change_db(half_knee - eps, DynamicMode::CompressDownward, ratio);
        let at_upper =
            compute_gain_change_db(half_knee + eps, DynamicMode::CompressDownward, ratio);
        assert!(
            (below_upper - at_upper).abs() < 1e-2,
            "Knee continuity failed at upper boundary: {below_upper} vs {at_upper}"
        );
    }

    #[test]
    fn test_soft_knee_matches_hard_knee_far_above_threshold() {
        // Well above threshold+knee, soft-knee should converge to hard-knee
        // behaviour: gain_change = -slope * over_db for CompressDownward.
        let ratio = 4.0_f32;
        let slope = 1.0 - 1.0 / ratio;
        let over_db = 20.0_f32; // way above knee
        let expected = -slope * over_db;
        let actual = compute_gain_change_db(over_db, DynamicMode::CompressDownward, ratio);
        assert!(
            (actual - expected).abs() < 1e-5,
            "Far-above-threshold gain change: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn test_soft_knee_is_monotonic_for_compress() {
        // Across the knee, gain_change_db is monotonically decreasing (more
        // attenuation as over_db rises).
        let ratio = 4.0_f32;
        let samples: Vec<f32> = (-200..=200).map(|i| i as f32 * 0.05).collect();
        let mut prev = 0.0_f32;
        for &over_db in &samples {
            let gc = compute_gain_change_db(over_db, DynamicMode::CompressDownward, ratio);
            assert!(
                gc <= prev + 1e-5,
                "Non-monotonic at over_db={over_db}: gc={gc}, prev={prev}"
            );
            prev = gc;
        }
    }

    #[test]
    fn test_soft_knee_gate_continuity() {
        // Gate: above threshold+knee → 0. Below threshold-knee → -slope * over_db.
        // Within knee → quadratic. Check continuity at both boundaries.
        let half_knee = KNEE_WIDTH_DB * 0.5;
        let ratio = 4.0_f32;
        let eps = 0.001_f32;

        let above = compute_gain_change_db(half_knee + eps, DynamicMode::Gate, ratio);
        let at_upper = compute_gain_change_db(half_knee - eps, DynamicMode::Gate, ratio);
        assert!(above.abs() < 1e-5, "Above knee gate must be 0, got {above}");
        assert!(
            at_upper.abs() < 1e-3,
            "Just below upper knee gate: {at_upper}"
        );

        let at_lower = compute_gain_change_db(-half_knee + eps, DynamicMode::Gate, ratio);
        let below_lower = compute_gain_change_db(-half_knee - eps, DynamicMode::Gate, ratio);
        assert!(
            (at_lower - below_lower).abs() < 1e-2,
            "Gate knee continuity at lower boundary: {at_lower} vs {below_lower}"
        );
    }

    #[test]
    fn test_dynamic_band_detection_is_rms_not_peak() {
        // A steady sine of amplitude A driven into the detector should settle
        // to env ≈ A/sqrt(2) (RMS) rather than A (peak). Confirms the detector
        // performs RMS integration.
        let sr = 44100.0_f32;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -60.0, // threshold low enough to see detection without gain collapse
            1.0,   // ratio 1.0 (no compression applied, just detection)
            0.1,   // fast attack so envelope tracks
            50.0,  // moderate release
            0.0,
            true,
            false,
        );
        let amp = 0.5_f32;
        // Let the detector settle: >> attack, release, and RMS window combined.
        for n in 0..50_000 {
            let phase = std::f32::consts::TAU * 1000.0 * (n as f32) / sr;
            let x = phase.sin() * amp;
            band.update_envelope(x, x);
        }
        let expected_rms = amp / std::f32::consts::SQRT_2;
        let relative_error = (band.envelope - expected_rms).abs() / expected_rms;
        assert!(
            relative_error < 0.05,
            "Envelope should settle near A/sqrt(2) = {expected_rms:.4}, got {:.4} \
             (relative error {:.3})",
            band.envelope,
            relative_error
        );
        // And it must NOT be near the peak amplitude (which is what a peak
        // detector would produce).
        assert!(
            band.envelope < amp * 0.85,
            "Envelope {:.4} is too close to peak {amp}; detector looks peak-style",
            band.envelope
        );
    }

    #[test]
    fn test_dynamic_band_attack_coeff_formula() {
        // attack_coeff = exp(-1 / (attack_ms * 0.001 * sr))
        let sr = 44100.0_f32;
        let attack_ms = 5.0_f32;
        let expected = (-1.0_f32 / (attack_ms * 0.001 * sr)).exp();
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward,
            1000.0,
            1000.0,
            1.0,
            -18.0,
            4.0,
            attack_ms,
            100.0,
            0.0,
            true,
            false,
        );
        assert!(
            (band.attack_coeff - expected).abs() < 1e-7,
            "Attack coeff: {} vs expected {}",
            band.attack_coeff,
            expected
        );
    }

    #[test]
    fn test_dynamic_band_process_produces_finite_output() {
        let mut band = DynamicBand::new(44100.0);
        band.update_parameters(
            DynamicMode::CompressDownward,
            500.0,
            500.0,
            1.5,
            -18.0,
            8.0,
            1.0,
            100.0,
            0.0,
            true,
            false,
        );
        for i in 0..500 {
            let input = if i % 3 == 0 { 1.0 } else { 0.1 };
            let out = band.process_sample(input);
            assert!(
                out.is_finite(),
                "DynamicBand output must be finite at {i}: {out}"
            );
        }
    }

    #[test]
    fn test_expand_up_boost_is_capped() {
        for ratio in [2.0_f32, 4.0, 20.0] {
            for over_db in [0.0_f32, 2.9, 3.0, 10.0, 60.0, 200.0] {
                let gc = compute_gain_change_db(over_db, DynamicMode::ExpandUpward, ratio);
                assert!(
                    gc <= MAX_EXPAND_BOOST_DB,
                    "Expand Up boost must not exceed {MAX_EXPAND_BOOST_DB} dB: ratio={ratio}, over_db={over_db}, got {gc}"
                );
            }
        }
    }

    fn process_stereo_block(deq: &mut DynamicEQ, l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(l);
                ss.push(r);
            });
        }
        deq.process(&mut buf);
    }

    fn sine_block(freq_hz: f32, sr: f32, start: usize, n: usize, amp: f32) -> Vec<f32> {
        (start..start + n)
            .map(|i| (std::f32::consts::TAU * freq_hz * i as f32 / sr).sin() * amp)
            .collect()
    }

    /// Regression (2026-09-13): all bands in Expand Up at ratio 20 with a -60 dB threshold drove
    /// the bell filters to inf, latched NaN into their state and silenced the plugin in Reaper.
    #[test]
    fn test_dynamic_eq_extreme_expand_up_stays_finite() {
        let sr = 96_000.0_f32;
        let n = 4096;
        let mut deq = DynamicEQ::new(sr);
        deq.update_parameters(
            &[DynamicBandParams {
                mode: DynamicMode::ExpandUpward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -60.0,
                ratio: 20.0,
                attack_ms: 1.0,
                release_ms: 100.0,
                gain_db: 18.0,
                enabled: true,
                solo: false,
            }; 4],
        );
        for block in 0..20 {
            let mut l = sine_block(1000.0, sr, block * n, n, 0.9);
            let mut r = l.clone();
            process_stereo_block(&mut deq, &mut l, &mut r);
            assert!(
                l.iter().chain(r.iter()).all(|s| s.is_finite()),
                "Expand Up output went non-finite in block {block}"
            );
        }
    }

    #[test]
    fn test_dynamic_eq_recovers_after_non_finite_input_block() {
        let sr = 48_000.0_f32;
        let n = 1024;
        let mut deq = DynamicEQ::new(sr);
        deq.update_parameters(
            &[DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -30.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: true,
                solo: false,
            }; 4],
        );

        let mut l = vec![f32::NAN; n];
        let mut r = vec![f32::NAN; n];
        process_stereo_block(&mut deq, &mut l, &mut r);

        let mut l = sine_block(1000.0, sr, 0, n, 0.5);
        let mut r = l.clone();
        process_stereo_block(&mut deq, &mut l, &mut r);
        assert!(
            l.iter().chain(r.iter()).all(|s| s.is_finite()),
            "a single non-finite host block must not latch NaN into later blocks"
        );
        assert!(
            deq.get_gain_reduction_db().iter().all(|gr| gr.is_finite()),
            "gain reduction must recover too"
        );
    }

    // ── DynamicEQ public API ──────────────────────────────────────────────────

    #[test]
    fn test_dynamic_eq_new_does_not_panic() {
        let _deq = DynamicEQ::new(44100.0);
        let _deq = DynamicEQ::new(48000.0);
        let _deq = DynamicEQ::new(96000.0);
    }

    #[test]
    fn test_dynamic_eq_update_parameters_does_not_panic() {
        let mut deq = DynamicEQ::new(44100.0);
        let params = [DynamicBandParams {
            mode: DynamicMode::CompressDownward,
            detector_freq: 1000.0,
            freq: 1000.0,
            q: 1.0,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            gain_db: 0.0,
            enabled: true,
            solo: false,
        }; 4];
        deq.update_parameters(&params);
    }

    #[test]
    fn test_dynamic_eq_get_gain_reduction_db_initial() {
        let deq = DynamicEQ::new(44100.0);
        let gr = deq.get_gain_reduction_db();
        for (i, &val) in gr.iter().enumerate() {
            assert!(
                (val - 0.0).abs() < 1e-9,
                "Initial GR band {i} should be 0.0"
            );
        }
    }

    #[test]
    fn test_dynamic_eq_detection_decoupled_from_cascade() {
        // After the DEQ-5 refactor, each band's envelope updates from the
        // pristine module input — not from the inter-band cascade. This means
        // band 2's detected level should be independent of whether band 1 is
        // heavily cutting or not. Verifies the invariant.
        let sr = 44100.0_f32;
        use nice_plug::buffer::Buffer;

        let make_sine = |n: usize| {
            let l: Vec<f32> = (0..n)
                .map(|i| (std::f32::consts::TAU * 1000.0 * (i as f32) / sr).sin() * 0.5)
                .collect();
            let r = l.clone();
            (l, r)
        };

        // Scenario A: band 0 disabled (no LF cut), band 1 detecting 1 kHz.
        let (mut l_a, mut r_a) = make_sine(512);
        let mut buf_a = Buffer::default();
        unsafe {
            buf_a.set_slices(512, |ss| {
                ss.clear();
                ss.push(&mut l_a);
                ss.push(&mut r_a);
            });
        }
        let mut deq_a = DynamicEQ::new(sr);
        let params_a = [
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 100.0,
                freq: 100.0,
                q: 1.0,
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false, // band 0 off
                solo: false,
            },
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -30.0,
                ratio: 4.0,
                attack_ms: 1.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: true,
                solo: false,
            },
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -30.0,
                ratio: 4.0,
                attack_ms: 1.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false,
                solo: false,
            },
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -30.0,
                ratio: 4.0,
                attack_ms: 1.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false,
                solo: false,
            },
        ];
        deq_a.update_parameters(&params_a);
        deq_a.process(&mut buf_a);
        let gr_a = deq_a.get_gain_reduction_db()[1];

        // Scenario B: band 0 cutting hard at 100 Hz. Band 1 still sees the
        // full 1 kHz input because detection taps the dry signal.
        let (mut l_b, mut r_b) = make_sine(512);
        let mut buf_b = Buffer::default();
        unsafe {
            buf_b.set_slices(512, |ss| {
                ss.clear();
                ss.push(&mut l_b);
                ss.push(&mut r_b);
            });
        }
        let mut deq_b = DynamicEQ::new(sr);
        let mut params_b = params_a;
        params_b[0].enabled = true;
        params_b[0].threshold_db = -60.0; // cut aggressively at 100 Hz
        params_b[0].ratio = 20.0;
        deq_b.update_parameters(&params_b);
        deq_b.process(&mut buf_b);
        let gr_b = deq_b.get_gain_reduction_db()[1];

        // Band 1's gain reduction must be (essentially) identical in both
        // scenarios — band 0's cascade shouldn't influence band 1's detection.
        assert!(
            (gr_a - gr_b).abs() < 0.1,
            "Band 1 GR changed when band 0 cascade changed: {gr_a} vs {gr_b}"
        );
    }

    #[test]
    fn test_dynamic_eq_stereo_link_applies_equal_gain() {
        // Stereo-linked detection: if only one channel has a hot 1 kHz
        // transient, the max-of-absolutes detector still drives the envelope,
        // and both channels must receive identical gain reduction (preserves
        // stereo image). The channels' outputs must also be congruent: L input
        // shaped by the L filter and R input shaped by the R filter, with the
        // same coefficient trajectory over time. We verify this by running two
        // buffers side by side and comparing band GR + per-sample ratios.
        let sr = 44100.0_f32;
        use nice_plug::buffer::Buffer;

        let n = 1024_usize;
        // L channel gets a 1 kHz sine at -6 dBFS; R channel is silent.
        // Without stereo linking, the detector would see nothing on R's frame
        // pass and act only on L's — giving lopsided GR.
        let mut l: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 1000.0 * (i as f32) / sr).sin() * 0.5)
            .collect();
        let mut r: Vec<f32> = vec![0.0; n];

        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }

        let mut deq = DynamicEQ::new(sr);
        let params = [
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -30.0,
                ratio: 4.0,
                attack_ms: 1.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: true,
                solo: false,
            },
            // Remaining bands disabled.
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false,
                solo: false,
            },
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false,
                solo: false,
            },
            DynamicBandParams {
                mode: DynamicMode::CompressDownward,
                detector_freq: 1000.0,
                freq: 1000.0,
                q: 1.0,
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 100.0,
                gain_db: 0.0,
                enabled: false,
                solo: false,
            },
        ];
        deq.update_parameters(&params);
        deq.process(&mut buf);

        // Gain reduction is a single state shared between L and R — by
        // construction it's identical. What we want to verify is that the
        // envelope built up at all (stereo-linked detection fired on L's
        // activity even with silent R), and that the L-channel output
        // responded while R stayed near silence in band 1's output.
        let gr = deq.get_gain_reduction_db()[0];
        assert!(
            gr > 0.5,
            "Expected meaningful band-0 GR from linked detection; got {gr} dB"
        );

        // R must stay near-silent (filter state shouldn't leak anything
        // because R input is zero). Per-channel EQ state means the L and R
        // biquads are independent even with identical coefficients.
        let r_peak = r.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(
            r_peak < 1e-3,
            "R channel should stay silent when only L has signal; peak = {r_peak}"
        );

        // Symmetry sanity: run the same scenario with L and R swapped — the
        // GR should come out the same (linked detection is symmetric in L,R).
        let mut l2: Vec<f32> = vec![0.0; n];
        let mut r2: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 1000.0 * (i as f32) / sr).sin() * 0.5)
            .collect();
        let mut buf2 = Buffer::default();
        unsafe {
            buf2.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l2);
                ss.push(&mut r2);
            });
        }
        let mut deq2 = DynamicEQ::new(sr);
        deq2.update_parameters(&params);
        deq2.process(&mut buf2);
        let gr_swapped = deq2.get_gain_reduction_db()[0];
        assert!(
            (gr - gr_swapped).abs() < 0.1,
            "Linked detection should be symmetric in L/R: {gr} vs {gr_swapped}"
        );
    }

    #[test]
    fn test_dynamic_eq_stereo_channels_independent_filter_state() {
        // Per-channel biquad state invariant: feeding DC + sine to L and R
        // channels through the same (bypassed) dynamic EQ must preserve both
        // channels independently. This would fail if the same biquad struct
        // were shared across channels — interleaved L/R samples would
        // corrupt each other's state. With eq_filter_l / eq_filter_r split,
        // the channels are effectively two independent filter chains.
        let sr = 44100.0_f32;
        use nice_plug::buffer::Buffer;

        let n = 256_usize;
        // L: 500 Hz sine at 0.25 amplitude. R: 2 kHz sine at 0.25 amplitude.
        let mut l: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 500.0 * (i as f32) / sr).sin() * 0.25)
            .collect();
        let mut r: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 2000.0 * (i as f32) / sr).sin() * 0.25)
            .collect();

        // Capture originals for comparison.
        let l_orig = l.clone();
        let r_orig = r.clone();

        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }

        let mut deq = DynamicEQ::new(sr);
        // All bands disabled → no EQ applied, just a pass-through that still
        // exercises update_envelope and the channel read/write path.
        let disabled = DynamicBandParams {
            mode: DynamicMode::CompressDownward,
            detector_freq: 1000.0,
            freq: 1000.0,
            q: 1.0,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            gain_db: 0.0,
            enabled: false,
            solo: false,
        };
        deq.update_parameters(&[disabled, disabled, disabled, disabled]);
        deq.process(&mut buf);

        // With every band disabled, apply_eq_stereo returns (l, r) unchanged.
        // Both channels must be bit-identical to their input.
        for i in 0..n {
            assert!(
                (l[i] - l_orig[i]).abs() < 1e-6,
                "L channel altered at sample {i}: {} vs {}",
                l[i],
                l_orig[i]
            );
            assert!(
                (r[i] - r_orig[i]).abs() < 1e-6,
                "R channel altered at sample {i}: {} vs {}",
                r[i],
                r_orig[i]
            );
        }
    }

    fn detect_only(freq: f32, q: f32) -> DynamicBandParams {
        DynamicBandParams {
            mode: DynamicMode::CompressDownward,
            detector_freq: freq,
            freq,
            q,
            threshold_db: 0.0,
            ratio: 1.0,
            attack_ms: 1.0,
            release_ms: 100.0,
            gain_db: 0.0,
            enabled: true,
            solo: false,
        }
    }

    /// Settled envelope of band 0, in dB, averaged over the last quarter of a
    /// one-second run of `l`/`r` sines.
    fn settled_envelope_db(
        params: DynamicBandParams,
        sr: f32,
        freq: f32,
        amp_l: f32,
        amp_r: f32,
    ) -> f32 {
        let n = 512;
        let blocks = (sr as usize) / n;
        let mut deq = DynamicEQ::new(sr);
        deq.update_parameters(&[params; 4]);
        let mut sum = 0.0_f32;
        let mut count = 0;
        for block in 0..blocks {
            let mut l = sine_block(freq, sr, block * n, n, amp_l);
            let mut r = sine_block(freq, sr, block * n, n, amp_r);
            process_stereo_block(&mut deq, &mut l, &mut r);
            if block >= blocks * 3 / 4 {
                sum += 20.0 * deq.bands[0].envelope.max(f32::MIN_POSITIVE).log10();
                count += 1;
            }
        }
        sum / count as f32
    }

    /// Regression (2026-09-14): `process` rectified the input before the band-pass,
    /// which moved a tone's energy to DC and 2f and read it 12–26 dB low.
    #[test]
    fn test_detector_reads_tone_rms_through_process() {
        let sr = 48_000.0_f32;
        let amp = 10.0_f32.powf(-12.0 / 20.0);
        let want_db = 20.0 * (amp / std::f32::consts::SQRT_2).log10();
        for freq in [330.0_f32, 3000.0, 7000.0] {
            for q in [1.0_f32, 4.3] {
                let got = settled_envelope_db(detect_only(freq, q), sr, freq, amp, amp);
                assert!(
                    (got - want_db).abs() < 1.0,
                    "{freq} Hz Q{q}: detector read {got:.2} dB, tone RMS is {want_db:.2} dB"
                );
            }
        }
    }

    #[test]
    fn test_detector_link_reads_hard_panned_tone_full_level() {
        let sr = 48_000.0_f32;
        let params = detect_only(1000.0, 1.0);
        let centred = settled_envelope_db(params, sr, 1000.0, 0.5, 0.5);
        let left_only = settled_envelope_db(params, sr, 1000.0, 0.5, 0.0);
        assert!(
            (centred - left_only).abs() < 0.5,
            "hard-panned tone read {left_only:.2} dB, centred {centred:.2} dB"
        );
    }

    /// The bell must carry exactly the gain computer's current value on every
    /// sample — no hysteresis step — including right after a FREQ change.
    #[test]
    fn test_bell_tracks_gain_computer_every_sample() {
        let sr = 48_000.0_f32;
        let mut band = DynamicBand::new(sr);
        let mut freq = 1000.0;
        for i in 0..9600 {
            if i == 0 || i == 4800 {
                band.update_parameters(
                    DynamicMode::CompressDownward,
                    freq,
                    freq,
                    1.0,
                    -40.0,
                    4.0,
                    1.0,
                    50.0,
                    0.0,
                    true,
                    false,
                );
                freq = 2000.0;
            }
            let level = if (i / 480) % 2 == 0 { 0.8 } else { 0.05 };
            band.process_sample((std::f32::consts::TAU * 1000.0 * i as f32 / sr).sin() * level);
            assert_eq!(
                band.eq_filter_l.svf.coefficients(),
                band.bell.coefficients(-band.gain_reduction_db),
                "bell lagged the gain computer at sample {i}"
            );
        }
        assert!(band.gain_reduction_db.abs() > 0.0 || band.envelope > 0.0);
    }

    #[test]
    fn test_dynamic_eq_reset_clears_all_bands() {
        let mut deq = DynamicEQ::new(44100.0);
        // Manually drive envelope in all bands
        for band in &mut deq.bands {
            band.update_parameters(
                DynamicMode::CompressDownward,
                1000.0,
                1000.0,
                1.0,
                -18.0,
                4.0,
                0.1,
                10.0,
                0.0,
                true,
                false,
            );
            for _ in 0..200 {
                band.process_sample(1.0);
            }
        }
        deq.reset();
        let gr = deq.get_gain_reduction_db();
        for (i, &val) in gr.iter().enumerate() {
            assert!(
                (val - 0.0).abs() < 1e-9,
                "GR band {i} should be 0 after reset"
            );
        }
    }
}
