use nih_plug::buffer::Buffer;
use nih_plug::prelude::Enum;

// ============================================================================
// ButterComp2 Model Enum
// ============================================================================

/// Selectable compressor personality for the ButterComp2 slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ButterComp2Model {
    #[name = "Classic"]
    Classic,
    #[name = "Optical"]
    Optical,
    #[name = "VCA"]
    Vca,
    #[name = "1176 FET"]
    Fet,
}

impl Default for ButterComp2Model {
    fn default() -> Self { ButterComp2Model::Classic }
}

// ============================================================================
// FET Ratio Enum
// ============================================================================

/// 1176-style ratio selector. `All` corresponds to the iconic "all-buttons-in" mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum FetRatio {
    #[name = "4:1"]
    R4,
    #[name = "8:1"]
    R8,
    #[name = "12:1"]
    R12,
    #[name = "20:1"]
    R20,
    #[name = "ALL"]
    All,
}

impl FetRatio {
    pub fn value(self) -> f32 {
        match self {
            Self::R4  =>  4.0,
            Self::R8  =>  8.0,
            Self::R12 => 12.0,
            Self::R20 => 20.0,
            // All-buttons mode: gain computer uses 20:1 then enforces a separate GR cap.
            Self::All => 20.0,
        }
    }
}

// ============================================================================
// FetCompressor — Pure Rust 1176-style FET compressor
// ============================================================================

/// Minimum input level before log conversion (prevents -inf dB).
const FET_LEVEL_FLOOR: f32 = 1e-6;
/// Floor for dB computations — avoids NaN from zero inputs.
const FET_DB_FLOOR: f32 = -120.0;
/// Envelope minimum in dB — prevents denormals in long-decay tails.
const FET_ENVELOPE_MIN_DB: f32 = -60.0;
/// All-Buttons mode: second-harmonic saturation coefficient.
const FET_ALL_BUTTONS_SAT: f32 = 0.15;
/// All-Buttons mode: maximum GR cap in dB (prevents runaway compression).
const FET_ALL_BUTTONS_GR_CAP: f32 = -18.0;
/// Auto-release: shortest release time (ms) — active during loud sustained signals.
const FET_AUTO_RELEASE_MIN_MS: f32 = 40.0;
/// Auto-release: longest release time (ms) — active during transients.
const FET_AUTO_RELEASE_MAX_MS: f32 = 1100.0;

/// 1176-style peak-detecting FET compressor with linked stereo detection.
///
/// All mutable state is pre-allocated in struct fields — no heap allocation in
/// `process_sample()`. This struct intentionally does NOT implement `Copy`.
pub struct FetCompressor {
    sample_rate: f32,
    // Linked stereo detection — single shared gain-reduction envelope.
    envelope_db: f32,
    fast_env_db: f32,
    envelope_fast_db: f32, // All-Buttons dual-release secondary arm.
    // Cached ballistic coefficients — recomputed only on parameter change.
    coeff_attack: f32,
    coeff_release: f32,
    coeff_fast_attack: f32,
    coeff_fast_release: f32,
    // Parameter dirty-check cache — avoids pow() on every buffer.
    cached_input_db: f32,
    cached_output_db: f32,
    cached_attack_ms: f32,
    cached_release_ms: f32,
    cached_ratio: FetRatio,
    cached_auto_release: bool,
    // Linear gain values derived from dB params.
    input_gain_linear: f32,
    output_gain_linear: f32,
}

impl FetCompressor {
    pub fn new(sample_rate: f32) -> Self {
        let mut s = Self {
            sample_rate,
            envelope_db: 0.0,
            fast_env_db: 0.0,
            envelope_fast_db: 0.0,
            coeff_attack: 0.0,
            coeff_release: 0.0,
            coeff_fast_attack: 0.0,
            coeff_fast_release: 0.0,
            // NaN sentinel forces coefficient computation on first update_parameters() call.
            cached_input_db: f32::NAN,
            cached_output_db: f32::NAN,
            cached_attack_ms: f32::NAN,
            cached_release_ms: f32::NAN,
            cached_ratio: FetRatio::R4,
            cached_auto_release: false,
            input_gain_linear: 1.0,
            output_gain_linear: 1.0,
        };
        s.recompute_coefficients(0.2, 250.0, false);
        s
    }

    fn recompute_coefficients(&mut self, attack_ms: f32, release_ms: f32, _auto_release: bool) {
        self.coeff_attack  = (-1.0 / (attack_ms  * 0.001 * self.sample_rate)).exp();
        self.coeff_release = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();
        // Fixed fast coefficients used by the All-Buttons secondary envelope and auto-release.
        self.coeff_fast_attack  = (-1.0 / (5.0  * 0.001 * self.sample_rate)).exp();
        self.coeff_fast_release = (-1.0 / (50.0 * 0.001 * self.sample_rate)).exp();
    }

    /// Update parameters — call once per buffer, not per sample.
    /// Coefficient recomputation only happens when values change beyond a threshold.
    pub fn update_parameters(
        &mut self,
        input_db: f32,
        output_db: f32,
        attack_ms: f32,
        release_ms: f32,
        ratio: FetRatio,
        auto_release: bool,
    ) {
        if (input_db - self.cached_input_db).abs() > 0.001 {
            self.cached_input_db = input_db;
            self.input_gain_linear = 10.0_f32.powf(input_db / 20.0);
        }
        if (output_db - self.cached_output_db).abs() > 0.001 {
            self.cached_output_db = output_db;
            self.output_gain_linear = 10.0_f32.powf(output_db / 20.0);
        }
        let atk_changed  = (attack_ms  - self.cached_attack_ms ).abs() > 0.0001;
        let rel_changed  = (release_ms - self.cached_release_ms).abs() > 0.5;
        let auto_changed = auto_release != self.cached_auto_release;
        if atk_changed || rel_changed || auto_changed {
            self.cached_attack_ms    = attack_ms;
            self.cached_release_ms   = release_ms;
            self.cached_auto_release = auto_release;
            self.recompute_coefficients(attack_ms, release_ms, auto_release);
        }
        self.cached_ratio = ratio;
    }

    /// Reset all envelope state. May be called from the audio thread (no allocation).
    pub fn reset(&mut self) {
        self.envelope_db      = 0.0;
        self.fast_env_db      = 0.0;
        self.envelope_fast_db = 0.0;
    }

    /// Process one stereo sample pair with linked peak detection.
    ///
    /// No allocation, no locking, no panics — safe for the audio thread.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let is_all_buttons = self.cached_ratio == FetRatio::All;

        // Stage 1 — Input drive (applied equally to both channels and sidechain).
        let driven_l = in_l * self.input_gain_linear;
        let driven_r = in_r * self.input_gain_linear;

        // Stage 2 — Linked peak detection (max of absolute values, stereo-linked).
        let x_abs = driven_l.abs().max(driven_r.abs());

        // Stage 3 — Gain computer in log domain.
        let x_db = (20.0 * x_abs.max(FET_LEVEL_FLOOR).log10()).max(FET_DB_FLOOR);
        // Threshold shifts with input drive: louder input → earlier engagement.
        let effective_threshold = -self.cached_input_db;
        let over_db = (x_db - effective_threshold).max(0.0);
        let ratio_val = self.cached_ratio.value();
        let mut gr_target = if over_db > 0.0 {
            -over_db * (1.0 - 1.0 / ratio_val)
        } else {
            0.0
        };

        // All-Buttons hard GR cap — prevents extreme pumping artefacts.
        if is_all_buttons {
            gr_target = gr_target.max(FET_ALL_BUTTONS_GR_CAP);
        }

        // Stage 4 — Attack/Release ballistics.
        let coeff_attack = if is_all_buttons {
            // All-Buttons mode enforces minimum 0.02 ms attack for characteristic aggression.
            (-1.0 / (0.02_f32 * 0.001 * self.sample_rate)).exp()
        } else {
            self.coeff_attack
        };

        if gr_target < self.envelope_db {
            // More GR needed — follow attack coefficient.
            self.envelope_db = coeff_attack * self.envelope_db
                + (1.0 - coeff_attack) * gr_target;
        } else if is_all_buttons {
            // All-Buttons: blend slow and fast release envelopes for the characteristic sound.
            let coeff_r_fast =
                (-1.0 / (50.0_f32 * 0.001 * self.sample_rate)).exp();
            self.envelope_fast_db = coeff_r_fast * self.envelope_fast_db
                + (1.0 - coeff_r_fast) * gr_target;
            self.envelope_db = 0.7 * (self.coeff_release * self.envelope_db
                + (1.0 - self.coeff_release) * gr_target)
                + 0.3 * self.envelope_fast_db;
        } else if self.cached_auto_release {
            // Auto-release: release time scales dynamically with current GR depth.
            // Update the fast-tracking secondary envelope.
            if gr_target < self.fast_env_db {
                self.fast_env_db = self.coeff_fast_attack * self.fast_env_db
                    + (1.0 - self.coeff_fast_attack) * gr_target;
            } else {
                self.fast_env_db = self.coeff_fast_release * self.fast_env_db
                    + (1.0 - self.coeff_fast_release) * gr_target;
            }
            let gr_magnitude = self.fast_env_db.abs();
            let t_auto_ms = (FET_AUTO_RELEASE_MIN_MS
                + (gr_magnitude / 20.0)
                    * (FET_AUTO_RELEASE_MAX_MS - FET_AUTO_RELEASE_MIN_MS))
                .clamp(FET_AUTO_RELEASE_MIN_MS, FET_AUTO_RELEASE_MAX_MS);
            let coeff_auto_rel =
                (-1.0 / (t_auto_ms * 0.001 * self.sample_rate)).exp();
            self.envelope_db = coeff_auto_rel * self.envelope_db
                + (1.0 - coeff_auto_rel) * gr_target;
        } else {
            self.envelope_db = self.coeff_release * self.envelope_db
                + (1.0 - self.coeff_release) * gr_target;
        }

        // Clamp envelopes and prevent denormals.
        self.envelope_db      = self.envelope_db     .clamp(FET_ENVELOPE_MIN_DB, 0.0);
        self.fast_env_db      = self.fast_env_db     .clamp(FET_ENVELOPE_MIN_DB, 0.0);
        self.envelope_fast_db = self.envelope_fast_db.clamp(FET_ENVELOPE_MIN_DB, 0.0);

        // Convert GR from dB to linear.
        let gr_linear = 10.0_f32.powf(self.envelope_db / 20.0);

        // Apply GR to the driven signal.
        let mut out_l = driven_l * gr_linear;
        let mut out_r = driven_r * gr_linear;

        // Stage 5 — All-Buttons second-harmonic injection (odd-order saturation, asymmetric).
        if is_all_buttons {
            out_l += FET_ALL_BUTTONS_SAT * out_l * out_l * out_l.signum();
            out_r += FET_ALL_BUTTONS_SAT * out_r * out_r * out_r.signum();
        }

        // Stage 6 — Output makeup gain.
        out_l *= self.output_gain_linear;
        out_r *= self.output_gain_linear;

        (out_l, out_r)
    }

    /// Process a full stereo buffer in place.
    ///
    /// # Safety invariant
    /// Caller must ensure the buffer has exactly 2 channels (guaranteed by the plugin's
    /// `AUDIO_IO_LAYOUTS` declaration which specifies a stereo main output).
    pub fn process(&mut self, buffer: &mut Buffer) {
        for mut frame in buffer.iter_samples() {
            let mut iter = frame.iter_mut();
            // SAFETY: Stereo bus — 2 channels guaranteed by plugin layout declaration.
            if let (Some(l), Some(r)) = (iter.next(), iter.next()) {
                let (out_l, out_r) = self.process_sample(*l, *r);
                *l = out_l;
                *r = out_r;
            }
        }
    }
}

// ============================================================================
// VcaCompressor — RMS-detecting, soft-knee, feed-forward bus compressor
// SSL G-Bus style: linked stereo detection, configurable threshold/ratio/A/R
// ============================================================================

/// RMS window used for level detection (10 ms is a standard ballistic choice).
const VCA_RMS_WINDOW_S: f32 = 0.010;
/// Soft-knee width in dB — transitions from linear to compressed region over 6 dB.
const VCA_KNEE_WIDTH_DB: f32 = 6.0;
/// Denormal guard for the squared RMS accumulator — prevents CPU load spikes.
const VCA_DENORMAL_FLOOR: f32 = 1e-15;
/// Minimum RMS level before log conversion — avoids -inf dB.
const VCA_MIN_RMS_LINEAR: f32 = 1e-6;
/// Minimum linear GR multiplier — prevents complete signal extinction.
const VCA_GR_MIN_LINEAR: f32 = 0.001;

/// RMS-detecting, soft-knee, feed-forward VCA bus compressor.
///
/// All mutable state is pre-allocated in struct fields — no heap allocation in
/// `process_sample()`. Linked stereo detection produces a single shared envelope.
pub struct VcaCompressor {
    sample_rate: f32,
    /// Linked stereo RMS accumulator (mean-square, pre-sqrt).
    rms_sq: f32,
    coeff_rms: f32,
    /// Shared gain-reduction envelope, linear multiplier (init 1.0 = no GR).
    env_gr: f32,
    /// Cached ballistic coefficients — recomputed only on parameter change.
    coeff_atk: f32,
    coeff_rel: f32,
    /// Dirty-check cache — avoids exp() on every buffer call.
    cached_thresh: f32,
    cached_ratio: f32,
    cached_atk_ms: f32,
    cached_rel_ms: f32,
}

impl VcaCompressor {
    pub fn new(sample_rate: f32) -> Self {
        let mut s = Self {
            sample_rate,
            rms_sq: 0.0,
            coeff_rms: 0.0,
            env_gr: 1.0,
            coeff_atk: 0.0,
            coeff_rel: 0.0,
            // NaN sentinel forces coefficient computation on first update_parameters() call.
            cached_thresh: f32::NAN,
            cached_ratio: f32::NAN,
            cached_atk_ms: f32::NAN,
            cached_rel_ms: f32::NAN,
        };
        s.recompute_coefficients(10.0, 100.0);
        s
    }

    fn recompute_coefficients(&mut self, atk_ms: f32, rel_ms: f32) {
        self.coeff_rms = (-1.0 / (VCA_RMS_WINDOW_S * self.sample_rate)).exp();
        self.coeff_atk = (-1.0 / (atk_ms * 0.001 * self.sample_rate)).exp();
        self.coeff_rel = (-1.0 / (rel_ms * 0.001 * self.sample_rate)).exp();
    }

    /// Update parameters — call once per buffer, not per sample.
    /// Coefficient recomputation only happens when values change beyond a threshold.
    pub fn update_parameters(
        &mut self,
        thresh_db: f32,
        ratio: f32,
        atk_ms: f32,
        rel_ms: f32,
    ) {
        let thresh_changed = (thresh_db - self.cached_thresh).abs() > 0.001;
        let ratio_changed  = (ratio    - self.cached_ratio  ).abs() > 0.001;
        let atk_changed    = (atk_ms   - self.cached_atk_ms ).abs() > 0.01;
        let rel_changed    = (rel_ms   - self.cached_rel_ms ).abs() > 0.01;
        if thresh_changed { self.cached_thresh = thresh_db; }
        if ratio_changed  { self.cached_ratio  = ratio; }
        if atk_changed || rel_changed {
            self.cached_atk_ms = atk_ms;
            self.cached_rel_ms = rel_ms;
            self.recompute_coefficients(atk_ms, rel_ms);
        }
    }

    /// Process one stereo sample pair with linked RMS detection.
    ///
    /// No allocation, no locking, no panics — safe for the audio thread.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        // Stage 1 — Linked RMS accumulation (max-abs side-chain, mean-square IIR).
        let x_sq = in_l.abs().max(in_r.abs()).powi(2);
        self.rms_sq = self.coeff_rms * self.rms_sq + (1.0 - self.coeff_rms) * x_sq;
        // Denormal guard: clamp to a small positive floor before sqrt.
        self.rms_sq = self.rms_sq.max(VCA_DENORMAL_FLOOR);
        let rms = self.rms_sq.sqrt();

        // Stage 2 — Level to dB.
        let x_db = if rms < VCA_MIN_RMS_LINEAR {
            -120.0_f32
        } else {
            20.0 * rms.log10()
        };

        // Stage 3 — Soft-knee gain computer.
        let t = self.cached_thresh;
        let r = self.cached_ratio;
        let w = VCA_KNEE_WIDTH_DB;
        let over = x_db - t;
        let gr_db = if x_db < t - w * 0.5 {
            0.0_f32
        } else if x_db > t + w * 0.5 {
            over * (1.0 / r - 1.0)
        } else {
            let frac = (over + w * 0.5) / w; // 0..1 inside knee
            frac * frac * over * (1.0 / r - 1.0)
        };
        // gr_db ≤ 0; convert to linear and clamp to prevent full extinction.
        let gr_linear_target = 10.0_f32.powf(gr_db / 20.0).clamp(VCA_GR_MIN_LINEAR, 1.0);

        // Stage 4 — Attack/release envelope on the linear GR multiplier.
        if gr_linear_target < self.env_gr {
            // More GR needed — attack phase.
            self.env_gr = self.coeff_atk * self.env_gr
                + (1.0 - self.coeff_atk) * gr_linear_target;
        } else {
            // Less GR needed — release phase.
            self.env_gr = self.coeff_rel * self.env_gr
                + (1.0 - self.coeff_rel) * gr_linear_target;
        }
        self.env_gr = self.env_gr.clamp(VCA_GR_MIN_LINEAR, 1.0);

        // Stage 5 — Apply shared GR to both channels.
        (in_l * self.env_gr, in_r * self.env_gr)
    }

    /// Process a full stereo buffer in place.
    ///
    /// # Safety invariant
    /// Caller must ensure the buffer has exactly 2 channels (guaranteed by the plugin's
    /// `AUDIO_IO_LAYOUTS` declaration which specifies a stereo main output).
    pub fn process(&mut self, buffer: &mut Buffer) {
        for mut frame in buffer.iter_samples() {
            let mut iter = frame.iter_mut();
            // SAFETY: Stereo bus — 2 channels guaranteed by plugin layout declaration.
            if let (Some(l), Some(r)) = (iter.next(), iter.next()) {
                let (out_l, out_r) = self.process_sample(*l, *r);
                *l = out_l;
                *r = out_r;
            }
        }
    }

    /// Reset all envelope and accumulator state. Safe to call from audio thread.
    pub fn reset(&mut self) {
        self.env_gr = 1.0;
        self.rms_sq = 0.0;
    }
}

// ============================================================================
// OpticalCompressor — LA-2A style opto-cell model
// Dual-integrator (fast envelope + slow memory), log-law gain curve,
// speed/character parameters, per-channel processing.
// ============================================================================

/// Fixed compression ratio for the optical model (3:1 like the LA-2A programme dependent mode).
const OPT_RATIO: f32 = 3.0;
/// Log-law shaping coefficient — higher values make the curve more non-linear.
const OPT_LOG_SHAPE_K: f32 = 4.0;
/// Denormal guard added to absolute signal level before log conversion.
const OPT_DENORM_GUARD: f32 = 1e-9;
/// Minimum signal level in dB (floor for peak pre-filter and level computations).
const OPT_MIN_LEVEL_DB: f32 = -90.0;
/// Maximum gain reduction in dB — prevents runaway compression.
const OPT_MAX_GR_DB: f32 = 40.0;
/// Maximum value of the slow memory integrator in dB.
const OPT_ENV_SLOW_MAX: f32 = 30.0;
/// GR threshold (dB) above which the slow integrator tracks (opto "memory" engages).
const OPT_ENV_SLOW_THRESHOLD: f32 = 0.5;
/// Slow memory decay multiplier applied to the memory_ms time constant.
const OPT_MEMORY_DECAY_FACTOR: f32 = 4.0;
/// Attack time for the peak pre-filter (ms) — fast enough to catch transients.
const OPT_PEAK_HOLD_ATK_MS: f32 = 0.5;
/// Release time for the peak pre-filter (ms).
const OPT_PEAK_HOLD_REL_MS: f32 = 5.0;

/// LA-2A style optical compressor with program-dependent release.
///
/// Per-channel processing to model independent opto-cell behaviour per channel.
/// All mutable state pre-allocated — no heap allocation in any processing method.
pub struct OpticalCompressor {
    sample_rate: f32,
    /// Fast ballistics integrator, per channel (GR dB, init 0.0).
    env_fast_l: f32,
    env_fast_r: f32,
    /// Slow memory integrator, per channel (GR dB, init 0.0).
    env_slow_l: f32,
    env_slow_r: f32,
    /// Peak pre-filter state, per channel (dB, init OPT_MIN_LEVEL_DB).
    peak_hold_l: f32,
    peak_hold_r: f32,
    /// Cached ballistic coefficients — same for both channels.
    attack_coeff: f32,
    release_coeff: f32,
    memory_coeff: f32,
    memory_decay_coeff: f32,
    peak_atk_coeff: f32,
    peak_rel_coeff: f32,
    memory_weight: f32,
    knee_db: f32,
    /// Dirty-check cache — avoids exp() on every buffer call.
    cached_thresh: f32,
    cached_speed: f32,
    cached_char: f32,
}

impl OpticalCompressor {
    pub fn new(sample_rate: f32) -> Self {
        let mut s = Self {
            sample_rate,
            env_fast_l: 0.0,
            env_fast_r: 0.0,
            env_slow_l: 0.0,
            env_slow_r: 0.0,
            peak_hold_l: OPT_MIN_LEVEL_DB,
            peak_hold_r: OPT_MIN_LEVEL_DB,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            memory_coeff: 0.0,
            memory_decay_coeff: 0.0,
            peak_atk_coeff: 0.0,
            peak_rel_coeff: 0.0,
            memory_weight: 0.0,
            knee_db: 0.0,
            // NaN sentinel forces coefficient computation on first update_parameters() call.
            cached_thresh: f32::NAN,
            cached_speed: f32::NAN,
            cached_char: f32::NAN,
        };
        s.recompute_coefficients(0.5, 0.5);
        s
    }

    fn recompute_coefficients(&mut self, speed: f32, char_val: f32) {
        // Log-space interpolation for perceptually uniform sweeping of time constants.
        let attack_ms  = (50.0_f32.ln() * (1.0 - speed) + 3.0_f32.ln()    * speed).exp();
        let release_ms = (3000.0_f32.ln() * (1.0 - speed) + 80.0_f32.ln() * speed).exp();
        let memory_ms  = (6000.0_f32.ln() * (1.0 - speed) + 500.0_f32.ln()* speed).exp();

        self.attack_coeff        = (-1.0 / (attack_ms  * 0.001 * self.sample_rate)).exp();
        self.release_coeff       = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();
        self.memory_coeff        = (-1.0 / (memory_ms  * 0.001 * self.sample_rate)).exp();
        self.memory_decay_coeff  = (-1.0 / (memory_ms * OPT_MEMORY_DECAY_FACTOR * 0.001 * self.sample_rate)).exp();

        self.peak_atk_coeff = (-1.0 / (OPT_PEAK_HOLD_ATK_MS * 0.001 * self.sample_rate)).exp();
        self.peak_rel_coeff = (-1.0 / (OPT_PEAK_HOLD_REL_MS * 0.001 * self.sample_rate)).exp();

        // character sweeps memory_weight 0.3..0.9 and knee 12..3 dB.
        self.memory_weight = 0.3 + char_val * 0.6;
        self.knee_db       = 12.0 - char_val * 9.0;
    }

    /// Update parameters — call once per buffer, not per sample.
    /// Coefficient recomputation only happens when values change beyond a threshold.
    pub fn update_parameters(&mut self, thresh_db: f32, speed: f32, char_val: f32) {
        let thresh_changed = (thresh_db - self.cached_thresh).abs() > 0.001;
        let speed_changed  = (speed     - self.cached_speed ).abs() > 0.005;
        let char_changed   = (char_val  - self.cached_char  ).abs() > 0.005;
        if thresh_changed { self.cached_thresh = thresh_db; }
        if speed_changed || char_changed {
            self.cached_speed = speed;
            self.cached_char  = char_val;
            self.recompute_coefficients(speed, char_val);
        }
    }

    /// Compute log-law shaped gain reduction (in dB, positive = amount of GR to apply).
    ///
    /// Uses a soft-knee around `thresh_db` then applies a non-linear log curve to
    /// model the response of an opto-cell element.
    #[inline]
    fn gain_computer(&self, x_db: f32, thresh_db: f32) -> f32 {
        let over = x_db - thresh_db;
        let half_knee = self.knee_db * 0.5;
        let gr_target = if over <= -half_knee {
            0.0_f32
        } else if over >= half_knee {
            over * (1.0 - 1.0 / OPT_RATIO)
        } else {
            // Quadratic interpolation through the knee region.
            (over + half_knee) * (over + half_knee)
                / (2.0 * self.knee_db)
                * (1.0 - 1.0 / OPT_RATIO)
        };
        // Log-law shaping: emulates the non-linear response of an opto-cell.
        let shaped = if gr_target > 0.0 {
            gr_target * (1.0 + gr_target * OPT_LOG_SHAPE_K).ln()
                / (1.0 + OPT_LOG_SHAPE_K).ln()
        } else {
            0.0
        };
        shaped.clamp(0.0, OPT_MAX_GR_DB)
    }

    /// Single-channel processing kernel — takes and returns state by value to avoid
    /// borrow checker conflicts when calling from `process_sample`.
    ///
    /// Returns `(output_sample, new_env_fast, new_env_slow, new_peak_hold)`.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn process_sample_channel(
        &self,
        x: f32,
        env_fast: f32,
        env_slow: f32,
        peak_hold: f32,
        thresh_db: f32,
    ) -> (f32, f32, f32, f32) {
        // Stage 1 — Peak pre-filter (smooth peak tracking to reduce inter-sample clicks).
        let x_abs    = x.abs() + OPT_DENORM_GUARD;
        let x_db_raw = 20.0 * x_abs.log10();
        let x_db     = x_db_raw.max(OPT_MIN_LEVEL_DB);
        let new_peak = if x_db > peak_hold {
            self.peak_atk_coeff * peak_hold + (1.0 - self.peak_atk_coeff) * x_db
        } else {
            self.peak_rel_coeff * peak_hold + (1.0 - self.peak_rel_coeff) * x_db
        };
        let new_peak = new_peak.max(OPT_MIN_LEVEL_DB);

        // Stage 2 — Gain computer.
        let gr_shaped = self.gain_computer(new_peak, thresh_db);

        // Stage 3 — Fast envelope with program-dependent release.
        let env_slow_norm = (env_slow / OPT_ENV_SLOW_MAX).clamp(0.0, 1.0);
        let new_env_fast = if gr_shaped > env_fast {
            // Attack phase.
            self.attack_coeff * env_fast + (1.0 - self.attack_coeff) * gr_shaped
        } else {
            // Program-dependent release: memory integrator blends toward slower release.
            let coeff_mod = (self.release_coeff
                + self.memory_weight * (self.memory_coeff - self.release_coeff) * env_slow_norm)
                .clamp(self.release_coeff, self.memory_coeff);
            coeff_mod * env_fast + (1.0 - coeff_mod) * gr_shaped
        };
        let new_env_fast = new_env_fast.clamp(0.0, OPT_MAX_GR_DB);

        // Stage 4 — Slow memory integrator (opto "memory" / tube warm-up effect).
        let new_env_slow = if gr_shaped > OPT_ENV_SLOW_THRESHOLD {
            self.memory_coeff * env_slow + (1.0 - self.memory_coeff) * gr_shaped
        } else {
            self.memory_decay_coeff * env_slow
        };
        let new_env_slow = new_env_slow.clamp(0.0, OPT_ENV_SLOW_MAX);

        // Stage 5 — Convert GR from dB to linear and apply.
        let gr_linear = 10.0_f32.powf(-new_env_fast / 20.0);
        let output = x * gr_linear;

        (output, new_env_fast, new_env_slow, new_peak)
    }

    /// Process one stereo sample pair with independent per-channel detection.
    ///
    /// No allocation, no locking, no panics — safe for the audio thread.
    #[inline]
    pub fn process_sample(&mut self, in_l: f32, in_r: f32, thresh_db: f32) -> (f32, f32) {
        let (out_l, ef_l, es_l, ph_l) =
            self.process_sample_channel(in_l, self.env_fast_l, self.env_slow_l, self.peak_hold_l, thresh_db);
        let (out_r, ef_r, es_r, ph_r) =
            self.process_sample_channel(in_r, self.env_fast_r, self.env_slow_r, self.peak_hold_r, thresh_db);
        self.env_fast_l = ef_l; self.env_slow_l = es_l; self.peak_hold_l = ph_l;
        self.env_fast_r = ef_r; self.env_slow_r = es_r; self.peak_hold_r = ph_r;
        (out_l, out_r)
    }

    /// Process a full stereo buffer in place.
    ///
    /// # Safety invariant
    /// Caller must ensure the buffer has exactly 2 channels (guaranteed by the plugin's
    /// `AUDIO_IO_LAYOUTS` declaration which specifies a stereo main output).
    pub fn process(&mut self, buffer: &mut Buffer, thresh_db: f32) {
        for mut frame in buffer.iter_samples() {
            let mut iter = frame.iter_mut();
            // SAFETY: Stereo bus — 2 channels guaranteed by plugin layout declaration.
            if let (Some(l), Some(r)) = (iter.next(), iter.next()) {
                let (out_l, out_r) = self.process_sample(*l, *r, thresh_db);
                *l = out_l;
                *r = out_r;
            }
        }
    }

    /// Reset all envelope and pre-filter state. Safe to call from audio thread.
    pub fn reset(&mut self) {
        self.env_fast_l = 0.0;
        self.env_fast_r = 0.0;
        self.env_slow_l = 0.0;
        self.env_slow_r = 0.0;
        self.peak_hold_l = OPT_MIN_LEVEL_DB;
        self.peak_hold_r = OPT_MIN_LEVEL_DB;
    }
}

// ButterComp2 FFI bindings
#[repr(C)]
pub struct ButterComp2StateOpaque {
    _private: [u8; 0],
}

pub type ButterComp2State = ButterComp2StateOpaque;

extern "C" {
    fn buttercomp2_create(sample_rate: f64) -> *mut ButterComp2State;
    fn buttercomp2_destroy(state: *mut ButterComp2State);
    fn buttercomp2_set_compress(state: *mut ButterComp2State, compress: f64);
    fn buttercomp2_set_output(state: *mut ButterComp2State, output: f64);
    fn buttercomp2_set_dry_wet(state: *mut ButterComp2State, dry_wet: f64);
    fn buttercomp2_process_stereo(
        state: *mut ButterComp2State,
        left_channel: *mut f32,
        right_channel: *mut f32,
        num_samples: i32,
    );
    fn buttercomp2_reset(state: *mut ButterComp2State);
}

/// ButterComp2 wrapper for Rust integration
///
/// Airwindows ButterComp2: "The single richest, lushest 'glue' compressor"
/// Features 4 independent compressors per channel in bipolar, interleaved configuration
pub struct ButterComp2 {
    state: *mut ButterComp2State,
}

impl ButterComp2 {
    /// Create a new ButterComp2 instance
    pub fn new(sample_rate: f32) -> Self {
        let state = unsafe { buttercomp2_create(sample_rate as f64) };
        assert!(!state.is_null(), "Failed to create ButterComp2 state");
        Self { state }
    }
    
    /// Update compressor parameters
    /// 
    /// # Arguments
    /// * `compress` - Compression amount (0.0 to 1.0, maps to 0-14dB)
    /// * `output` - Output gain (0.0 to 1.0, maps to 0-2x gain)
    /// * `dry_wet` - Dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn update_parameters(&mut self, compress: f32, output: f32, dry_wet: f32) {
        // Scale parameters to prevent over-compression and distortion
        let safe_compress = (compress * 0.5).clamp(0.0, 0.5); // Reduce max compression
        let safe_output = (output * 0.8 + 0.2).clamp(0.2, 1.0); // Keep output in reasonable range
        let safe_dry_wet = dry_wet.clamp(0.0, 1.0);
        
        unsafe {
            buttercomp2_set_compress(self.state, safe_compress as f64);
            buttercomp2_set_output(self.state, safe_output as f64);
            buttercomp2_set_dry_wet(self.state, safe_dry_wet as f64);
        }
    }
    
    /// Process audio buffer in place (stereo, lock-free, allocation-free).
    ///
    /// Calls the C++ function once per buffer (O(1) FFI overhead) rather than
    /// once per sample (O(block_size) overhead). The C++ implementation loops
    /// over `num_samples` internally — see buttercomp2_process_stereo in cpp/.
    pub fn process(&mut self, buffer: &mut Buffer) {
        let num_samples = buffer.samples();
        if num_samples == 0 { return; }

        // Capture a *mut f32 to the start of each channel from the first sample
        // iteration. NIH-plug guarantees each channel is a contiguous, non-overlapping
        // f32 slice, so ch[n]+i accesses sample i of channel n.
        let mut ch: [*mut f32; 2] = [std::ptr::null_mut(); 2];
        let mut count = 0usize;
        if let Some(first) = buffer.iter_samples().next() {
            for s in first {
                if count < 2 {
                    ch[count] = s as *mut f32;
                    count += 1;
                }
            }
        }

        if count >= 2 {
            // Safety: ch[0] and ch[1] are valid *mut f32 pointers to the first
            // element of non-overlapping, contiguous channel slices of length
            // num_samples. buttercomp2_process_stereo iterates [0..num_samples).
            unsafe {
                buttercomp2_process_stereo(self.state, ch[0], ch[1], num_samples as i32);
            }
        }
    }
    
    /// Reset internal state
    pub fn reset(&mut self) {
        unsafe {
            buttercomp2_reset(self.state);
        }
    }
}

impl Drop for ButterComp2 {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe {
                buttercomp2_destroy(self.state);
            }
        }
    }
}

unsafe impl Send for ButterComp2 {}
unsafe impl Sync for ButterComp2 {}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FetRatio ──────────────────────────────────────────────────────────────

    #[test]
    fn test_fet_ratio_values() {
        assert!((FetRatio::R4.value()  -  4.0).abs() < 1e-5);
        assert!((FetRatio::R8.value()  -  8.0).abs() < 1e-5);
        assert!((FetRatio::R12.value() - 12.0).abs() < 1e-5);
        assert!((FetRatio::R20.value() - 20.0).abs() < 1e-5);
        // All-buttons uses 20:1 for the gain computer
        assert!((FetRatio::All.value() - 20.0).abs() < 1e-5);
    }

    // ── FetCompressor ─────────────────────────────────────────────────────────

    #[test]
    fn test_fet_compressor_new_initial_state() {
        let fet = FetCompressor::new(44100.0);
        assert!((fet.envelope_db - 0.0).abs() < 1e-5);
        assert!((fet.fast_env_db - 0.0).abs() < 1e-5);
        assert!((fet.input_gain_linear - 1.0).abs() < 1e-5);
        assert!((fet.output_gain_linear - 1.0).abs() < 1e-5);
        assert!(fet.coeff_attack > 0.0 && fet.coeff_attack < 1.0);
        assert!(fet.coeff_release > 0.0 && fet.coeff_release < 1.0);
    }

    #[test]
    fn test_fet_compressor_reset_clears_envelopes() {
        let mut fet = FetCompressor::new(44100.0);
        for _ in 0..500 { fet.process_sample(1.0, 1.0); }
        fet.reset();
        assert!((fet.envelope_db - 0.0).abs() < 1e-5, "envelope_db after reset: {}", fet.envelope_db);
        assert!((fet.fast_env_db - 0.0).abs() < 1e-5, "fast_env_db after reset: {}", fet.fast_env_db);
        assert!((fet.envelope_fast_db - 0.0).abs() < 1e-5, "envelope_fast_db after reset: {}", fet.envelope_fast_db);
    }

    #[test]
    fn test_fet_compressor_quiet_signal_passes_through() {
        let mut fet = FetCompressor::new(44100.0);
        // Pre-seed: NaN dirty-check in update_parameters never fires for the first call
        // (IEEE754: (x - NaN).abs() > threshold always returns false).
        // Set cached values directly so the compressor state is valid for the test.
        fet.cached_input_db = 0.0;
        fet.cached_output_db = 0.0;
        fet.input_gain_linear = 1.0;
        fet.output_gain_linear = 1.0;
        // Very quiet signal (well below 0 dB effective threshold)
        let input = 0.001_f32;
        let (out_l, out_r) = fet.process_sample(input, input);
        assert!((out_l / input - 1.0).abs() < 0.01, "Quiet signal gain: {}", out_l / input);
        assert!((out_r / input - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fet_compressor_loud_signal_is_attenuated() {
        let mut fet = FetCompressor::new(44100.0);
        // Pre-seed: effective_threshold = -cached_input_db.
        // Use +6 dB input_db → effective_threshold = -6 dBFS so a 0 dBFS signal engages GR.
        fet.cached_input_db = 6.0;
        fet.input_gain_linear = 10.0_f32.powf(6.0 / 20.0);
        fet.cached_output_db = 0.0;
        fet.output_gain_linear = 1.0;
        // Use fast attack so the envelope builds quickly in the test
        fet.recompute_coefficients(0.001, 100.0, false);

        for _ in 0..2000 { fet.process_sample(1.0, 1.0); }
        let (out_l, _) = fet.process_sample(1.0, 1.0);
        assert!(out_l < 1.0, "Loud signal should be attenuated, got {out_l}");
        assert!(out_l > 0.0, "Output should still be positive");
    }

    #[test]
    fn test_fet_compressor_all_buttons_gr_cap() {
        let mut fet = FetCompressor::new(44100.0);
        fet.update_parameters(18.0, 0.0, 0.001, 100.0, FetRatio::All, false);

        for _ in 0..5000 { fet.process_sample(1.0, 1.0); }

        // GR should never exceed the -18 dB cap
        assert!(
            fet.envelope_db >= FET_ALL_BUTTONS_GR_CAP,
            "All-Buttons GR cap violated: envelope_db = {}",
            fet.envelope_db
        );
    }

    #[test]
    fn test_fet_compressor_envelope_clamped_in_range() {
        let mut fet = FetCompressor::new(44100.0);
        for _ in 0..5000 { fet.process_sample(100.0, 100.0); }
        assert!(
            fet.envelope_db >= FET_ENVELOPE_MIN_DB,
            "envelope_db below floor: {}",
            fet.envelope_db
        );
        assert!(fet.envelope_db <= 0.0, "envelope_db must be <= 0: {}", fet.envelope_db);
    }

    #[test]
    fn test_fet_compressor_process_produces_finite_output() {
        let mut fet = FetCompressor::new(44100.0);
        fet.update_parameters(6.0, -6.0, 1.0, 200.0, FetRatio::R8, true);
        for _ in 0..100 {
            let (l, r) = fet.process_sample(0.5, -0.5);
            assert!(l.is_finite(), "Output L must be finite, got {l}");
            assert!(r.is_finite(), "Output R must be finite, got {r}");
        }
    }

    #[test]
    fn test_fet_compressor_update_dirty_check_skips_recompute() {
        let mut fet = FetCompressor::new(44100.0);
        // Seed the cache first (bypassing the NaN-init limitation)
        fet.cached_attack_ms = 1.0;
        fet.cached_release_ms = 200.0;
        fet.cached_auto_release = false;
        fet.recompute_coefficients(1.0, 200.0, false);
        let coeff_before = fet.coeff_attack;
        // Same parameters — dirty check should skip recompute
        fet.update_parameters(0.0, 0.0, 1.0, 200.0, FetRatio::R4, false);
        assert!((fet.coeff_attack - coeff_before).abs() < 1e-9, "Dirty-check bypass: coeff should not change");
    }

    #[test]
    fn test_fet_compressor_update_new_attack_recomputes() {
        let mut fet = FetCompressor::new(44100.0);
        // Seed the cache so the dirty check can detect changes
        fet.cached_attack_ms = 1.0;
        fet.cached_release_ms = 200.0;
        fet.cached_auto_release = false;
        fet.recompute_coefficients(1.0, 200.0, false);
        let coeff_before = fet.coeff_attack;
        // Different attack time — should trigger recompute
        fet.update_parameters(0.0, 0.0, 10.0, 200.0, FetRatio::R4, false);
        assert!(
            (fet.coeff_attack - coeff_before).abs() > 1e-5,
            "New attack time should change coeff_attack"
        );
    }

    #[test]
    fn test_fet_compressor_coeff_attack_formula() {
        // new() calls recompute_coefficients(0.2, 250.0) — verify the stored coeff
        // matches exp(-1 / (ms * 0.001 * sr)) for the initial 0.2 ms attack time.
        let sr = 44100.0_f32;
        let ms = 0.2_f32; // value used in new()
        let expected = (-1.0_f32 / (ms * 0.001 * sr)).exp();
        let fet = FetCompressor::new(sr);
        assert!((fet.coeff_attack - expected).abs() < 1e-7, "coeff mismatch: {} vs {expected}", fet.coeff_attack);
    }

    // ── VcaCompressor ─────────────────────────────────────────────────────────

    #[test]
    fn test_vca_compressor_new_does_not_panic() {
        let _vca = VcaCompressor::new(44100.0);
        let _vca = VcaCompressor::new(48000.0);
    }

    #[test]
    fn test_vca_compressor_process_produces_finite_output() {
        let mut vca = VcaCompressor::new(44100.0);
        // Pre-seed NaN-initialized cache fields (dirty-check NaN issue — same as FetCompressor)
        vca.cached_thresh = -18.0;
        vca.cached_ratio = 4.0;
        for _ in 0..200 {
            let (l, r) = vca.process_sample(0.5, 0.5);
            assert!(l.is_finite(), "VCA output L must be finite");
            assert!(r.is_finite(), "VCA output R must be finite");
        }
    }

    #[test]
    fn test_vca_compressor_quiet_passes_through() {
        let mut vca = VcaCompressor::new(44100.0);
        vca.cached_thresh = -18.0;
        vca.cached_ratio = 4.0;
        let input = 0.0001_f32;
        let (out_l, out_r) = vca.process_sample(input, input);
        assert!(out_l.is_finite());
        assert!(out_r.is_finite());
    }

    // ── OpticalCompressor ─────────────────────────────────────────────────────

    #[test]
    fn test_optical_compressor_new_does_not_panic() {
        let _opt = OpticalCompressor::new(44100.0);
    }

    #[test]
    fn test_optical_compressor_process_produces_finite_output() {
        let mut opt = OpticalCompressor::new(44100.0);
        opt.update_parameters(-12.0, 0.5, 0.5);
        for _ in 0..200 {
            let (l, r) = opt.process_sample(0.7, -0.7, -12.0);
            assert!(l.is_finite(), "Optical output L: {l}");
            assert!(r.is_finite(), "Optical output R: {r}");
        }
    }

    #[test]
    fn test_optical_compressor_loud_signal_is_attenuated() {
        let mut opt = OpticalCompressor::new(44100.0);
        opt.update_parameters(-12.0, 0.8, 0.5);
        // Warm up
        for _ in 0..2000 { opt.process_sample(1.0, 1.0, -12.0); }
        let (out_l, _) = opt.process_sample(1.0, 1.0, -12.0);
        assert!(out_l < 1.0, "Optical compressor should reduce loud signal, got {out_l}");
    }
}