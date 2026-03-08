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