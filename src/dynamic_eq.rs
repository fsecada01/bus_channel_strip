// src/dynamic_eq.rs — 4-band dynamic equalizer
//
// Key design decisions:
//   - BiquadPeak replaces biquad::DirectForm1 everywhere so filter state
//     is never reset when coefficients change (DirectForm1::new() zeroed state).
//   - The sidechain detection filter is also a BiquadPeak (+6 dB peak at the
//     detector frequency) so its state persists across buffer boundaries.
//   - Envelope detection uses a denormal guard (max with f32::MIN_POSITIVE)
//     before log10() to prevent -inf / NaN when the signal is silent.
//   - Solo mode routes only the soloed band(s) through a RBJ bandpass filter
//     so the user can isolate exactly the frequency range being processed.

use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// ── Stateful biquad ──────────────────────────────────────────────────────────
//
// Both the EQ and sidechain filters use this struct. Coefficient fields
// (b0‥a2) are updated in-place without touching the state fields (x1,x2,y1,y2).

struct BiquadPeak {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadPeak {
    fn new() -> Self {
        // Identity (flat): b0=1, all others 0.
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0,
               x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// RBJ Cookbook peaking EQ — updates coefficients, preserves state.
    fn update_peaking(&mut self, freq_hz: f32, q: f32, gain_db: f32, sample_rate: f32) {
        let freq_hz = freq_hz.clamp(20.0, sample_rate * 0.49);
        let q = q.max(0.1);
        let a = 10.0f32.powf(gain_db / 40.0); // sqrt of linear gain
        let w0 = std::f32::consts::TAU * freq_hz / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let inv_a0 = 1.0 / (1.0 + alpha / a);
        self.b0 = (1.0 + alpha * a) * inv_a0;
        self.b1 = (-2.0 * cos_w0) * inv_a0;
        self.b2 = (1.0 - alpha * a) * inv_a0;
        self.a1 = (-2.0 * cos_w0) * inv_a0;
        self.a2 = (1.0 - alpha / a) * inv_a0;
    }

    /// RBJ Cookbook constant-skirt-gain bandpass — updates coefficients, preserves state.
    /// Used for solo band-isolation mode.
    fn update_bandpass(&mut self, freq_hz: f32, q: f32, sample_rate: f32) {
        let freq_hz = freq_hz.clamp(20.0, sample_rate * 0.49);
        let q = q.max(0.1);
        let w0 = std::f32::consts::TAU * freq_hz / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let inv_a0 = 1.0 / (1.0 + alpha);
        self.b0 =  (sin_w0 / 2.0) * inv_a0;
        self.b1 =  0.0;
        self.b2 = -(sin_w0 / 2.0) * inv_a0;
        self.a1 = (-2.0 * cos_w0) * inv_a0;
        self.a2 = (1.0 - alpha) * inv_a0;
    }

    /// Direct Form 1 — processes one sample.
    #[inline]
    fn process(&mut self, x0: f32) -> f32 {
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2
                              - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x0;
        self.y2 = self.y1; self.y1 = y0;
        y0
    }

    fn reset(&mut self) {
        self.x1 = 0.0; self.x2 = 0.0;
        self.y1 = 0.0; self.y2 = 0.0;
    }
}

// ── DynamicMode ───────────────────────────────────────────────────────────────

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
    fn default() -> Self { DynamicMode::CompressDownward }
}

// ── DynamicBand ───────────────────────────────────────────────────────────────

struct DynamicBand {
    // Filters (all BiquadPeak — state persists across buffer boundaries)
    sidechain_filter: BiquadPeak, // detection: +6 dB peak at detector_freq
    eq_filter: BiquadPeak,        // dynamic EQ: gain changes with envelope
    solo_filter: BiquadPeak,      // bandpass for band-isolation solo mode

    // Envelope
    envelope: f32,
    pub gain_reduction_db: f32,
    last_gain_change_db: f32, // hysteresis cache — avoids per-sample trig recompute

    // Cached parameter values (updated per-buffer, used per-sample)
    sample_rate: f32,
    mode: DynamicMode,
    detector_freq: f32,
    frequency: f32,
    q: f32,
    threshold_db: f32,   // stored directly in dB (no round-trip conversion)
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    make_up_gain: f32,   // linear gain
    enabled: bool,
    solo: bool,
}

impl DynamicBand {
    fn new(sample_rate: f32) -> Self {
        let mut sidechain_filter = BiquadPeak::new();
        sidechain_filter.update_peaking(1000.0, 1.0, 6.0, sample_rate);

        let mut solo_filter = BiquadPeak::new();
        solo_filter.update_bandpass(1000.0, 1.0, sample_rate);

        Self {
            sidechain_filter,
            eq_filter: BiquadPeak::new(),
            solo_filter,
            envelope: 0.0,
            gain_reduction_db: 0.0,
            last_gain_change_db: 0.0,
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
        self.attack_coeff  = (-1.0 / (attack_ms.max(0.01)  * 0.001 * sr)).exp();
        self.release_coeff = (-1.0 / (release_ms.max(0.01) * 0.001 * sr)).exp();
        self.make_up_gain = 10.0f32.powf(make_up_gain_db / 20.0);
        self.enabled = enabled;
        self.solo = solo;

        // Update sidechain detection filter — state preserved, no reset.
        self.sidechain_filter.update_peaking(detector_freq, q, 6.0, sr);

        // Update solo bandpass filter for this band's center frequency.
        self.solo_filter.update_bandpass(frequency, q, sr);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        // 1. Sidechain detection — frequency-weighted envelope follower.
        let sidechain_signal = self.sidechain_filter.process(input);
        let detection_level = sidechain_signal.abs();

        if detection_level > self.envelope {
            self.envelope = detection_level + (self.envelope - detection_level) * self.attack_coeff;
        } else {
            self.envelope = detection_level + (self.envelope - detection_level) * self.release_coeff;
        }

        // 2. Gain computation in dB.
        // Guard: max with MIN_POSITIVE prevents log10(0) = -inf → NaN / Gate explosion.
        let envelope_db = 20.0 * self.envelope.max(f32::MIN_POSITIVE).log10();
        let over_db = envelope_db - self.threshold_db;

        let mut gain_change_db = 0.0_f32;
        match self.mode {
            DynamicMode::CompressDownward => {
                if over_db > 0.0 {
                    gain_change_db = -over_db * (1.0 - 1.0 / self.ratio);
                }
            }
            DynamicMode::ExpandUpward => {
                if over_db > 0.0 {
                    gain_change_db = over_db * (self.ratio - 1.0);
                }
            }
            DynamicMode::Gate => {
                if over_db < 0.0 {
                    // Clamp so we never apply more attenuation than threshold allows.
                    gain_change_db = (over_db * (1.0 - 1.0 / self.ratio)).max(-96.0);
                }
            }
        }
        self.gain_reduction_db = -gain_change_db;

        // 3. Update EQ coefficients only when gain changes significantly.
        // update_peaking() runs cos()/sin()/powf() — expensive transcendental math.
        // With typical attack/release times, the envelope changes <0.025 dB/sample,
        // so a 0.05 dB hysteresis threshold means we recompute every ~2 samples
        // during active compression and never during silence — substantial savings
        // with at most 0.05 dB of GR tracking error (inaudible).
        const GR_HYSTERESIS_DB: f32 = 0.05;
        if (gain_change_db - self.last_gain_change_db).abs() > GR_HYSTERESIS_DB {
            self.eq_filter.update_peaking(self.frequency, self.q, gain_change_db, self.sample_rate);
            self.last_gain_change_db = gain_change_db;
        }

        // 4. Apply dynamic EQ and makeup gain.
        self.eq_filter.process(input) * self.make_up_gain
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain_reduction_db = 0.0;
        self.last_gain_change_db = 0.0;
        self.eq_filter.reset();
        // Intentionally keep sidechain_filter and solo_filter state to avoid clicks.
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
    pub gain_db: f32,      // makeup gain in dB
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
        let solo_count = self.bands.iter()
            .filter(|b| b.solo && b.enabled)
            .count()
            .max(1) as f32;

        for samples in buffer.iter_samples() {
            for sample in samples {
                if any_solo {
                    // Band-isolation mode: sum bandpass outputs of soloed bands.
                    // Non-soloed bands are still needed for envelope updates so
                    // they respond instantly when solo is released.
                    let dry = *sample;
                    let mut out = 0.0_f32;
                    for band in &mut self.bands {
                        if band.solo && band.enabled {
                            out += band.solo_filter.process(dry);
                        }
                        // Keep sidechain/envelope alive for non-soloed bands.
                        // (process_sample would apply EQ too; we just update envelope.)
                        else if band.enabled {
                            let sc = band.sidechain_filter.process(dry);
                            let det = sc.abs();
                            if det > band.envelope {
                                band.envelope = det + (band.envelope - det) * band.attack_coeff;
                            } else {
                                band.envelope = det + (band.envelope - det) * band.release_coeff;
                            }
                        }
                    }
                    *sample = out / solo_count;
                } else {
                    // Normal mode: cascade all enabled bands in series.
                    let mut s = *sample;
                    for band in &mut self.bands {
                        s = band.process_sample(s);
                    }
                    *sample = s;
                }
            }
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

    // ── BiquadPeak ────────────────────────────────────────────────────────────

    #[test]
    fn test_biquad_peak_identity_passthrough() {
        let mut bq = BiquadPeak::new();
        // Identity filter (b0=1, all others 0) should pass signal unchanged
        for &input in &[0.0, 0.5, -0.5, 1.0, -1.0] {
            let out = bq.process(input);
            assert!((out - input).abs() < 1e-6, "Identity passthrough: input={input}, out={out}");
        }
    }

    #[test]
    fn test_biquad_peak_reset_clears_state() {
        let mut bq = BiquadPeak::new();
        bq.update_peaking(1000.0, 1.0, 6.0, 44100.0);
        for _ in 0..100 { bq.process(1.0); }
        bq.reset();
        assert!((bq.x1 - 0.0).abs() < 1e-9);
        assert!((bq.x2 - 0.0).abs() < 1e-9);
        assert!((bq.y1 - 0.0).abs() < 1e-9);
        assert!((bq.y2 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_biquad_peak_update_peaking_does_not_clear_state() {
        // State fields must survive a coefficient update (key design invariant)
        let mut bq = BiquadPeak::new();
        bq.update_peaking(1000.0, 1.0, 6.0, 44100.0);
        for _ in 0..100 { bq.process(0.7); }
        let y1_before = bq.y1;
        bq.update_peaking(2000.0, 1.5, -3.0, 44100.0);
        assert!((bq.y1 - y1_before).abs() < 1e-9, "y1 state should survive coeff update");
    }

    #[test]
    fn test_biquad_peak_nonzero_gain_changes_amplitude() {
        let mut flat = BiquadPeak::new();
        flat.update_peaking(1000.0, 1.0, 0.0, 44100.0);

        let mut boosted = BiquadPeak::new();
        boosted.update_peaking(1000.0, 1.0, 6.0, 44100.0);

        // Warm up both with a DC signal
        for _ in 0..1000 {
            flat.process(0.5);
            boosted.process(0.5);
        }
        let flat_out = flat.process(0.5);
        let boosted_out = boosted.process(0.5);
        // 6 dB boost at center freq — boosted should produce higher output
        assert!(boosted_out.abs() > flat_out.abs(), "6 dB boost should increase amplitude");
    }

    #[test]
    fn test_biquad_peak_produces_finite_output() {
        let mut bq = BiquadPeak::new();
        bq.update_peaking(20.0, 0.1, -60.0, 44100.0); // extreme params
        for i in 0..200 {
            let out = bq.process(if i % 2 == 0 { 1.0 } else { -1.0 });
            assert!(out.is_finite(), "BiquadPeak output must be finite at sample {i}: {out}");
        }
    }

    #[test]
    fn test_biquad_bandpass_update_does_not_panic() {
        let mut bq = BiquadPeak::new();
        bq.update_bandpass(1000.0, 1.0, 44100.0);
        bq.update_bandpass(500.0, 2.0, 48000.0);
    }

    #[test]
    fn test_biquad_freq_clamping_to_nyquist() {
        let sr = 44100.0;
        let nyquist = sr * 0.49;
        let mut bq = BiquadPeak::new();
        // freq above Nyquist should be clamped — should not panic or produce NaN
        bq.update_peaking(nyquist + 10000.0, 1.0, 3.0, sr);
        let out = bq.process(0.5);
        assert!(out.is_finite(), "Output after freq clamping: {out}");
    }

    // ── DynamicBand ───────────────────────────────────────────────────────────

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
            DynamicMode::CompressDownward, 1000.0, 1000.0, 1.0,
            -18.0, 4.0, 5.0, 100.0, 0.0, false, false,
        );
        // When disabled, process_sample should return input unchanged
        let input = 0.7_f32;
        let out = band.process_sample(input);
        assert!((out - input).abs() < 1e-5, "Disabled band: expected {input}, got {out}");
    }

    #[test]
    fn test_dynamic_band_reset_clears_envelope() {
        let mut band = DynamicBand::new(44100.0);
        band.update_parameters(
            DynamicMode::CompressDownward, 1000.0, 1000.0, 1.0,
            -18.0, 4.0, 0.1, 10.0, 0.0, true, false,
        );
        for _ in 0..500 { band.process_sample(1.0); }
        band.reset();
        assert!((band.envelope - 0.0).abs() < 1e-9, "Envelope should be 0 after reset");
        assert!((band.gain_reduction_db - 0.0).abs() < 1e-9, "GR should be 0 after reset");
    }

    #[test]
    fn test_dynamic_band_compress_mode_reduces_loud_signal() {
        let sr = 44100.0;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward, 1000.0, 1000.0, 1.0,
            -30.0, // very sensitive threshold
            4.0, 0.001, 50.0, 0.0, true, false,
        );
        // Warm up the envelope with loud signal
        for _ in 0..2000 { band.process_sample(1.0); }
        let gr = band.gain_reduction_db;
        assert!(gr > 0.0, "Compressor should show positive GR in dB, got {gr}");
    }

    #[test]
    fn test_dynamic_band_gate_mode_attenuates_quiet_signal() {
        let sr = 44100.0;
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::Gate, 1000.0, 1000.0, 1.0,
            -6.0, // high threshold — quiet signal is below it
            4.0, 0.1, 50.0, 0.0, true, false,
        );
        // Process a quiet signal below threshold
        for _ in 0..200 { band.process_sample(0.01); }
        let out = band.process_sample(0.01);
        // Gate should attenuate signal below threshold
        assert!(out.abs() < 0.01, "Gate should attenuate quiet signal, got {out}");
    }

    #[test]
    fn test_dynamic_band_attack_coeff_formula() {
        // attack_coeff = exp(-1 / (attack_ms * 0.001 * sr))
        let sr = 44100.0_f32;
        let attack_ms = 5.0_f32;
        let expected = (-1.0_f32 / (attack_ms * 0.001 * sr)).exp();
        let mut band = DynamicBand::new(sr);
        band.update_parameters(
            DynamicMode::CompressDownward, 1000.0, 1000.0, 1.0,
            -18.0, 4.0, attack_ms, 100.0, 0.0, true, false,
        );
        assert!(
            (band.attack_coeff - expected).abs() < 1e-7,
            "Attack coeff: {} vs expected {}",
            band.attack_coeff, expected
        );
    }

    #[test]
    fn test_dynamic_band_process_produces_finite_output() {
        let mut band = DynamicBand::new(44100.0);
        band.update_parameters(
            DynamicMode::CompressDownward, 500.0, 500.0, 1.5,
            -18.0, 8.0, 1.0, 100.0, 0.0, true, false,
        );
        for i in 0..500 {
            let input = if i % 3 == 0 { 1.0 } else { 0.1 };
            let out = band.process_sample(input);
            assert!(out.is_finite(), "DynamicBand output must be finite at {i}: {out}");
        }
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
            detector_freq: 1000.0, freq: 1000.0, q: 1.0,
            threshold_db: -18.0, ratio: 4.0,
            attack_ms: 5.0, release_ms: 100.0,
            gain_db: 0.0, enabled: true, solo: false,
        }; 4];
        deq.update_parameters(&params);
    }

    #[test]
    fn test_dynamic_eq_get_gain_reduction_db_initial() {
        let deq = DynamicEQ::new(44100.0);
        let gr = deq.get_gain_reduction_db();
        for (i, &val) in gr.iter().enumerate() {
            assert!((val - 0.0).abs() < 1e-9, "Initial GR band {i} should be 0.0");
        }
    }

    #[test]
    fn test_dynamic_eq_reset_clears_all_bands() {
        let mut deq = DynamicEQ::new(44100.0);
        // Manually drive envelope in all bands
        for band in &mut deq.bands {
            band.update_parameters(
                DynamicMode::CompressDownward, 1000.0, 1000.0, 1.0,
                -18.0, 4.0, 0.1, 10.0, 0.0, true, false,
            );
            for _ in 0..200 { band.process_sample(1.0); }
        }
        deq.reset();
        let gr = deq.get_gain_reduction_db();
        for (i, &val) in gr.iter().enumerate() {
            assert!((val - 0.0).abs() < 1e-9, "GR band {i} should be 0 after reset");
        }
    }
}
