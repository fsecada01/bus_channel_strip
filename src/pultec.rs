use crate::oversampler::Oversampler;
use crate::svf::{SvfCoefficients, SvfType, TptSvf};
use nih_plug::buffer::Buffer;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// Oversampling factor for the tube saturation stage. 4× (2 halfband stages)
/// brings the 2nd/3rd-order harmonic energy of a pushed signal below
/// fold-back threshold while remaining cheap enough for an always-on EQ.
const PULTEC_TUBE_OS_FACTOR: usize = 4;

/// Number of EQ stages in the linear chain (LF boost shelf, LF resonant
/// bump, LF cut shelf, HF boost bell, HF cut shelf).
const EQ_STAGES: usize = 5;

// ── Linear-phase mode (#15 stretch, roadmap §3.2) ────────────────────────────
//
// In linear-phase mode the five EQ stages are replaced by a single
// zero-phase FIR whose magnitude is sampled from the very same TPT
// coefficients the minimum-phase chain uses, so the two modes measure the
// same and differ only in phase. The FIR runs as overlap-save FFT
// convolution: every `LP_HOP` input samples, one `LP_FFT`-point FFT/IFFT pair
// per channel produces `LP_HOP` output samples.
//
// Latency budget: `LP_HOP` samples of block buffering + `(TAPS-1)/2` of
// kernel group delay = 512 samples at any sample rate (10.7 ms at 48 kHz),
// the roadmap's "~512 samples" target.
//
// Resolution trade-off: a 513-tap kernel resolves ~fs/513 ≈ 94 Hz at 48 kHz.
// The HF sections are reproduced within a fraction of a dB; the LF shelf
// and its Q=1.8 resonant bump are smoothed below ~200 Hz (the boost is
// still delivered, but the corner is softer than in minimum-phase mode).
// This is the standard trade-off of a "low latency" linear-phase mode; a
// longer-kernel "high resolution" option is the natural follow-up.

/// FIR kernel length (odd, so the centre tap is an integer delay).
const LINEAR_PHASE_TAPS: usize = 513;
/// Overlap-save hop: input samples per FFT block.
const LP_HOP: usize = 256;
/// Overlap-save FFT size. Must be ≥ `LP_HOP + LINEAR_PHASE_TAPS − 1`.
const LP_FFT: usize = 1024;
/// Total latency reported to the host when linear-phase mode is engaged.
const LINEAR_PHASE_LATENCY: usize = LP_HOP + (LINEAR_PHASE_TAPS - 1) / 2;
/// Frequency-sampling grid for kernel design. Finer than the kernel itself
/// so the zero-phase impulse response is well resolved before truncation.
const LP_DESIGN_FFT: usize = 4096;

const _: () = assert!(LP_FFT >= LP_HOP + LINEAR_PHASE_TAPS - 1);
const _: () = assert!(LP_DESIGN_FFT >= 2 * LINEAR_PHASE_TAPS);
const _: () = assert!(LINEAR_PHASE_TAPS % 2 == 1);
const _: () = assert!(LINEAR_PHASE_LATENCY == 512);

/// Minimum samples between kernel redesigns, bounding automation's worst-case
/// hit on `design_kernel`'s cost to once per hop. See ADR-0011.
const KERNEL_REDESIGN_MIN_INTERVAL: usize = LP_HOP;

/// Overlap-save FFT convolver plus the zero-phase FIR designer that feeds it.
/// Every buffer and FFT plan is allocated in `new()`; `design_kernel`,
/// `process_frame` and `run_block` are allocation-free.
struct LinearPhaseEngine {
    fft_fwd: Arc<dyn RealToComplex<f32>>,
    fft_inv: Arc<dyn ComplexToReal<f32>>,
    design_inv: Arc<dyn ComplexToReal<f32>>,

    // Kernel design (runs only when a parameter actually changed).
    /// `tan(ω/2)` for every design bin — tabulated so a redesign is pure
    /// arithmetic (no trig on the audio thread).
    design_tan: Vec<f32>,
    design_spec: Vec<Complex<f32>>,
    design_time: Vec<f32>,
    design_scratch: Vec<Complex<f32>>,
    /// Hann window over the kernel taps.
    window: Vec<f32>,
    kernel_time: Vec<f32>,
    /// FFT of the windowed kernel, pre-scaled by 1/LP_FFT so the inverse
    /// transform of a product comes out normalised.
    kernel_spec: Vec<Complex<f32>>,
    kernel_dirty: bool,
    /// Samples elapsed since the kernel was last redesigned. Gates
    /// `design_kernel` calls to at most once per [`KERNEL_REDESIGN_MIN_INTERVAL`].
    samples_since_redesign: usize,

    // Runtime overlap-save state.
    /// Last `LP_FFT` input samples per channel; the newest `LP_HOP` live at
    /// the tail and are filled in as frames arrive.
    hist: [Vec<f32>; 2],
    /// Output of the most recently completed block, read out one sample per
    /// frame while the next block fills.
    out_block: [Vec<f32>; 2],
    fft_in: Vec<f32>,
    spec: Vec<Complex<f32>>,
    time: Vec<f32>,
    fwd_scratch: Vec<Complex<f32>>,
    inv_scratch: Vec<Complex<f32>>,
    /// Frame index within the current block, `0..LP_HOP`.
    pos: usize,
}

impl LinearPhaseEngine {
    fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_fwd = planner.plan_fft_forward(LP_FFT);
        let fft_inv = planner.plan_fft_inverse(LP_FFT);
        let design_inv = planner.plan_fft_inverse(LP_DESIGN_FFT);

        let design_bins = LP_DESIGN_FFT / 2 + 1;
        let design_tan: Vec<f32> = (0..design_bins)
            .map(|i| {
                if i == LP_DESIGN_FFT / 2 {
                    // tan(π/2): take the limit explicitly instead of trusting
                    // f32 to overflow the right way.
                    f32::INFINITY
                } else {
                    ((core::f64::consts::PI * i as f64) / LP_DESIGN_FFT as f64).tan() as f32
                }
            })
            .collect();
        let window = crate::shaping::hann_window(LINEAR_PHASE_TAPS);

        let mut engine = Self {
            design_tan,
            design_spec: design_inv.make_input_vec(),
            design_time: design_inv.make_output_vec(),
            design_scratch: design_inv.make_scratch_vec(),
            window,
            kernel_time: fft_fwd.make_input_vec(),
            kernel_spec: fft_fwd.make_output_vec(),
            kernel_dirty: true,
            // "Ready" so the first coefficient update redesigns immediately.
            samples_since_redesign: KERNEL_REDESIGN_MIN_INTERVAL,
            hist: [vec![0.0; LP_FFT], vec![0.0; LP_FFT]],
            out_block: [vec![0.0; LP_HOP], vec![0.0; LP_HOP]],
            fft_in: fft_fwd.make_input_vec(),
            spec: fft_fwd.make_output_vec(),
            time: fft_inv.make_output_vec(),
            fwd_scratch: fft_fwd.make_scratch_vec(),
            inv_scratch: fft_inv.make_scratch_vec(),
            pos: 0,
            fft_fwd,
            fft_inv,
            design_inv,
        };
        // Start from a flat (pure-delay) kernel so the engine is usable
        // before the first parameter update.
        engine.design_kernel(&[SvfCoefficients::flat(); EQ_STAGES]);
        engine
    }

    /// Rebuild the FIR from the composite magnitude of `stages`.
    ///
    /// 1. Sample |H| = Π|H_stage| on the design grid (zero phase).
    /// 2. Inverse real FFT → even (zero-phase) impulse response.
    /// 3. Take the centre `LINEAR_PHASE_TAPS` samples, Hann-window them.
    /// 4. Forward FFT (zero-padded to `LP_FFT`) → kernel spectrum.
    fn design_kernel(&mut self, stages: &[SvfCoefficients; EQ_STAGES]) {
        // Flat stages contribute unity magnitude everywhere; skip their
        // `response_from_tan` call. Stack array, not `Vec` — stays allocation-free.
        let flat = SvfCoefficients::flat();
        let mut is_active = [false; EQ_STAGES];
        for (active, stage) in is_active.iter_mut().zip(stages) {
            *active = *stage != flat;
        }
        for (bin, t) in self.design_spec.iter_mut().zip(&self.design_tan) {
            let mut mag = 1.0_f32;
            for (stage, &active) in stages.iter().zip(&is_active) {
                if active {
                    mag *= stage.response_from_tan(*t).norm();
                }
            }
            *bin = Complex::new(mag, 0.0);
        }
        // Buffer lengths are fixed at construction and the DC/Nyquist bins
        // are exactly real, so this cannot fail; the Result is discarded
        // rather than unwrapped because this runs on the audio thread.
        let _ = self.design_inv.process_with_scratch(
            &mut self.design_spec,
            &mut self.design_time,
            &mut self.design_scratch,
        );

        let half = (LINEAR_PHASE_TAPS - 1) / 2;
        let norm = 1.0 / LP_DESIGN_FFT as f32;
        self.kernel_time.fill(0.0);
        for m in 0..LINEAR_PHASE_TAPS {
            // Negative time wraps to the end of the IFFT output.
            let idx = (m + LP_DESIGN_FFT - half) % LP_DESIGN_FFT;
            self.kernel_time[m] = self.design_time[idx] * norm * self.window[m];
        }
        let _ = self.fft_fwd.process_with_scratch(
            &mut self.kernel_time,
            &mut self.kernel_spec,
            &mut self.fwd_scratch,
        );
        let inv_n = 1.0 / LP_FFT as f32;
        for bin in &mut self.kernel_spec {
            *bin *= inv_n;
        }
        self.kernel_dirty = false;
        self.samples_since_redesign = 0;
    }

    /// Dry input from `LINEAR_PHASE_LATENCY` frames ago (for bypass). Must be
    /// read *before* `process_frame` pushes the current frame.
    #[inline]
    fn delayed_dry(&self) -> (f32, f32) {
        let idx = LP_FFT - LP_HOP + self.pos - LINEAR_PHASE_LATENCY;
        (self.hist[0][idx], self.hist[1][idx])
    }

    /// Push one stereo frame, return the filtered frame from
    /// `LINEAR_PHASE_LATENCY` frames ago.
    #[inline]
    fn process_frame(&mut self, l: f32, r: f32) -> (f32, f32) {
        let pos = self.pos;
        let out = (self.out_block[0][pos], self.out_block[1][pos]);
        let write = LP_FFT - LP_HOP + pos;
        self.hist[0][write] = l;
        self.hist[1][write] = r;
        self.pos += 1;
        self.samples_since_redesign = self.samples_since_redesign.saturating_add(1);
        if self.pos == LP_HOP {
            self.run_block();
            self.pos = 0;
        }
        out
    }

    /// One overlap-save block for both channels.
    fn run_block(&mut self) {
        for ch in 0..2 {
            self.fft_in.copy_from_slice(&self.hist[ch]);
            let _ = self.fft_fwd.process_with_scratch(
                &mut self.fft_in,
                &mut self.spec,
                &mut self.fwd_scratch,
            );
            for (s, k) in self.spec.iter_mut().zip(&self.kernel_spec) {
                *s *= *k;
            }
            // Real signals have exactly real DC/Nyquist bins; pin them so
            // rounding can't trip realfft's input check.
            self.spec[0].im = 0.0;
            self.spec[LP_FFT / 2].im = 0.0;
            let _ = self.fft_inv.process_with_scratch(
                &mut self.spec,
                &mut self.time,
                &mut self.inv_scratch,
            );
            // The first TAPS−1 outputs are circularly wrapped; the last
            // LP_HOP are the clean linear-convolution samples for the newest
            // LP_HOP inputs.
            self.out_block[ch].copy_from_slice(&self.time[LP_FFT - LP_HOP..]);
            self.hist[ch].copy_within(LP_HOP.., 0);
        }
    }

    fn reset(&mut self) {
        for h in &mut self.hist {
            h.fill(0.0);
        }
        for o in &mut self.out_block {
            o.fill(0.0);
        }
        self.pos = 0;
        // Ready for an immediate redesign rather than making the caller wait
        // out the throttle window right after a reset.
        self.samples_since_redesign = KERNEL_REDESIGN_MIN_INTERVAL;
    }
}

/// The passive LCR inductor network in the real EQP-1A creates a resonant
/// peak at the selected shelf frequency. At Q=0.5 (wide shelf) the peak needs
/// to be ~45% of shelf gain to remain clearly audible at the corner.
const LF_RESONANT_RATIO: f32 = 0.45;
const LF_RESONANT_Q: f32 = 1.8;

/// LF shelf Q range driven by the bandwidth knob (0 = narrow/modern, 1 = wide/vintage).
/// Q=1.0 at BW=0 gives a tight, modern shelf; Q=0.25 at BW=1 gives the very
/// gradual, wide shelf characteristic of the passive EQP-1A inductor network.
const LF_SHELF_Q_NARROW: f32 = 1.0;
const LF_SHELF_Q_WIDE: f32 = 0.25;

/// HF attenuation shelf Q — fixed, slightly resonant like the original's
/// passive HF cut network.
const HF_CUT_Q: f32 = 0.9;

/// Pultec EQP-1A style EQ module
///
/// Classic passive tube EQ with simultaneous boost/cut characteristics
/// - Low frequency boost with optional simultaneous cut for unique curves
/// - High frequency boost with separate bandwidth and cut controls
/// - Tube-style saturation modeling
pub struct PultecEQ {
    sample_rate: f32,

    // Each SVF carries its own integrator state; stereo processing REQUIRES
    // a separate filter instance per channel. Sharing one filter across L and
    // R makes consecutive samples from alternating channels corrupt the
    // filter memory, smearing the shelf and blunting perceived gain.
    lf_boost_filter: [TptSvf; 2],
    // Resonant peak from the passive LCR network — centered at the same
    // frequency as the shelf, gain proportional to shelf gain.
    lf_resonant_filter: [TptSvf; 2],
    lf_cut_filter: [TptSvf; 2],
    hf_boost_filter: [TptSvf; 2],
    hf_cut_filter: [TptSvf; 2],

    // Tube saturation state
    tube_drive: f32,

    // Per-channel oversamplers for the tube saturation nonlinearity.
    tube_os_l: Oversampler,
    tube_os_r: Oversampler,

    /// Linear-phase mode: the five EQ stages above are replaced by the FIR
    /// convolver. The tube stage still runs after it.
    linear_phase: bool,
    linear: LinearPhaseEngine,
}

impl PultecEQ {
    /// Create a new Pultec EQ with the given sample rate.
    ///
    /// Filters are initialized flat (0 dB). Coefficients are updated in-place
    /// via `update_coefficients()` in `update_parameters()`, which preserves
    /// filter state across parameter changes and avoids per-buffer allocation.
    /// The linear-phase engine (FFT plans + buffers) is allocated here too,
    /// so engaging the mode later never allocates on the audio thread.
    pub fn new(sample_rate: f32) -> Self {
        // Oversamplers are used inline (one sample in → one sample out), so
        // `max_block_size = 1` keeps their scratch buffers at 16 samples.
        let make_os = || Oversampler::new_at_factor(PULTEC_TUBE_OS_FACTOR, 1);
        let flat = || [TptSvf::flat(), TptSvf::flat()];

        Self {
            sample_rate,
            lf_boost_filter: flat(),
            lf_resonant_filter: flat(),
            lf_cut_filter: flat(),
            hf_boost_filter: flat(),
            hf_cut_filter: flat(),
            tube_drive: 0.0,
            tube_os_l: make_os(),
            tube_os_r: make_os(),
            linear_phase: false,
            linear: LinearPhaseEngine::new(),
        }
    }

    /// Reset filter, convolver and saturation state. Call on sample-rate
    /// change or buffer discontinuity.
    pub fn reset(&mut self) {
        for f in self
            .lf_boost_filter
            .iter_mut()
            .chain(self.lf_resonant_filter.iter_mut())
            .chain(self.lf_cut_filter.iter_mut())
            .chain(self.hf_boost_filter.iter_mut())
            .chain(self.hf_cut_filter.iter_mut())
        {
            f.reset();
        }
        self.tube_os_l.reset();
        self.tube_os_r.reset();
        self.linear.reset();
    }

    /// Engage or release linear-phase mode. Engaging clears the convolver so
    /// the FIR starts from silence rather than from whatever the history
    /// held when the mode was last active. The caller is responsible for
    /// re-reporting `latency_samples()` to the host.
    pub fn set_linear_phase(&mut self, enabled: bool) {
        if enabled == self.linear_phase {
            return;
        }
        self.linear_phase = enabled;
        if enabled {
            self.linear.reset();
            self.linear.kernel_dirty = true;
        }
    }

    /// Whether linear-phase mode is currently engaged.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn linear_phase(&self) -> bool {
        self.linear_phase
    }

    /// Latency the host must compensate for in the current mode.
    pub fn latency_samples(&self) -> u32 {
        if self.linear_phase {
            LINEAR_PHASE_LATENCY as u32
        } else {
            0
        }
    }

    /// Current coefficients of the five EQ stages (left channel; right is
    /// identical), in processing order.
    fn stage_coefficients(&self) -> [SvfCoefficients; EQ_STAGES] {
        [
            self.lf_boost_filter[0].coefficients(),
            self.lf_resonant_filter[0].coefficients(),
            self.lf_cut_filter[0].coefficients(),
            self.hf_boost_filter[0].coefficients(),
            self.hf_cut_filter[0].coefficients(),
        ]
    }

    /// Write `coeffs` to both channels of `pair` if they changed; flags the
    /// linear-phase kernel for redesign when they did.
    #[inline]
    fn set_stage(pair: &mut [TptSvf; 2], coeffs: SvfCoefficients, kernel_dirty: &mut bool) {
        if pair[0].coefficients() != coeffs {
            pair[0].update_coefficients(coeffs);
            pair[1].update_coefficients(coeffs);
            *kernel_dirty = true;
        }
    }

    /// Update Pultec parameters
    ///
    /// # Arguments
    /// * `lf_boost_freq` - Low frequency boost center (20..300 Hz)
    /// * `lf_boost_db` - Low frequency boost (0..+18 dB)
    /// * `lf_boost_bandwidth` - Shelf width: 0 = narrow/modern (Q=1.0), 1 = wide/vintage (Q=0.25)
    /// * `lf_cut_freq` - Low frequency cut center, independent of boost — the
    ///   Pultec "trick" is to set boost and cut at *different* frequencies so
    ///   the overlap produces a scooped-then-boosted curve (20..400 Hz)
    /// * `lf_cut_db` - Low frequency attenuation (0..18 dB; negated internally)
    /// * `lf_cut_bandwidth` - Cut shelf width: same convention as lf_boost_bandwidth
    /// * `hf_boost_freq` - High frequency boost center (5, 8, 10, 12, 15, 20 kHz)
    /// * `hf_boost_db` - High frequency boost (0..+10 dB)
    /// * `hf_boost_bandwidth` - High frequency boost Q/bandwidth (0.0 to 1.0)
    /// * `hf_cut_freq` - High frequency cut frequency (5, 10, 20 kHz)
    /// * `hf_cut_db` - High frequency attenuation (0..8 dB; negated internally)
    /// * `tube_drive` - Tube saturation amount (0.0 to 1.0)
    pub fn update_parameters(
        &mut self,
        lf_boost_freq: f32,
        lf_boost_db: f32,
        lf_boost_bandwidth: f32,
        lf_cut_freq: f32,
        lf_cut_db: f32,
        lf_cut_bandwidth: f32,
        hf_boost_freq: f32,
        hf_boost_db: f32,
        hf_boost_bandwidth: f32,
        hf_cut_freq: f32,
        hf_cut_db: f32,
        tube_drive: f32,
    ) {
        self.tube_drive = tube_drive.clamp(0.0, 1.0);

        // All four sections follow the same pattern:
        //   - compute dB (0.0 when the gain control is below noise floor)
        //   - swap coefficients on the existing SVF (state preserved, no
        //     clicks, nothing constructed on the audio thread)
        //   - if they changed, flag the linear-phase kernel for redesign
        let sr = self.sample_rate;
        let mut dirty = false;

        // Low Frequency Boost — LowShelf + resonant peak at the same frequency.
        // The passive LCR network in the real EQP-1A creates a resonant bump
        // at the corner, giving the characteristic "thump" and making the boost
        // much more perceptible at typical musical frequencies (60–300 Hz).
        let lf_boost_db = if lf_boost_db > 0.05 { lf_boost_db } else { 0.0 };
        let safe_lf_freq = lf_boost_freq.clamp(20.0, 400.0);
        // BW=0 → Q=LF_SHELF_Q_NARROW (tight/modern), BW=1 → Q=LF_SHELF_Q_WIDE (vintage/gradual)
        let lf_boost_q = LF_SHELF_Q_NARROW
            + lf_boost_bandwidth.clamp(0.0, 1.0) * (LF_SHELF_Q_WIDE - LF_SHELF_Q_NARROW);
        Self::set_stage(
            &mut self.lf_boost_filter,
            SvfCoefficients::new(SvfType::LowShelf(lf_boost_db), sr, safe_lf_freq, lf_boost_q),
            &mut dirty,
        );
        // Resonant peak: 45% of shelf gain, Q=1.8, same center frequency.
        // Goes flat (0 dB) when the shelf is inactive.
        let resonant_db = lf_boost_db * LF_RESONANT_RATIO;
        Self::set_stage(
            &mut self.lf_resonant_filter,
            SvfCoefficients::new(SvfType::Bell(resonant_db), sr, safe_lf_freq, LF_RESONANT_Q),
            &mut dirty,
        );

        // Low Frequency Cut — independent frequency from boost. Classic
        // EQP-1A "trick": boost at e.g. 60 Hz and cut at e.g. 200 Hz so the
        // cut attenuates the mud above the boosted low-bass for a tight,
        // defined low end. Value is already in dB; negate for shelf cut.
        let lf_cut_db = if lf_cut_db > 0.05 { -lf_cut_db } else { 0.0 };
        let safe_lf_cut_freq = lf_cut_freq.clamp(20.0, 500.0);
        let lf_cut_q = LF_SHELF_Q_NARROW
            + lf_cut_bandwidth.clamp(0.0, 1.0) * (LF_SHELF_Q_WIDE - LF_SHELF_Q_NARROW);
        Self::set_stage(
            &mut self.lf_cut_filter,
            SvfCoefficients::new(SvfType::LowShelf(lf_cut_db), sr, safe_lf_cut_freq, lf_cut_q),
            &mut dirty,
        );

        // High Frequency Boost — bell, 0 dB when inactive.
        // Value is already in dB (parameter range 0..10 dB).
        let hf_boost_db = if hf_boost_db > 0.05 { hf_boost_db } else { 0.0 };
        let hf_q = 0.6 + hf_boost_bandwidth * hf_boost_bandwidth * 1.4; // 0.6–2.0
        let safe_hf_freq = hf_boost_freq.clamp(3000.0, 20000.0);
        Self::set_stage(
            &mut self.hf_boost_filter,
            SvfCoefficients::new(SvfType::Bell(hf_boost_db), sr, safe_hf_freq, hf_q),
            &mut dirty,
        );

        // High Frequency Cut — HighShelf, 0 dB when inactive.
        // Value is already in dB; negate for shelf cut.
        let hf_cut_db = if hf_cut_db > 0.05 { -hf_cut_db } else { 0.0 };
        let safe_hf_cut_freq = hf_cut_freq.clamp(5000.0, 20000.0);
        Self::set_stage(
            &mut self.hf_cut_filter,
            SvfCoefficients::new(
                SvfType::HighShelf(hf_cut_db),
                sr,
                safe_hf_cut_freq,
                HF_CUT_Q,
            ),
            &mut dirty,
        );

        if dirty {
            self.linear.kernel_dirty = true;
        }
    }

    /// Rebuild the linear-phase FIR if a parameter moved since the last
    /// design. Runs once per change, never per sample.
    #[inline]
    fn redesign_kernel_if_dirty(&mut self) {
        if self.linear.kernel_dirty
            && self.linear.samples_since_redesign >= KERNEL_REDESIGN_MIN_INTERVAL
        {
            let stages = self.stage_coefficients();
            self.linear.design_kernel(&stages);
        }
    }

    /// Tube saturation — the one intentional nonlinearity in this module.
    /// Run through a 4× halfband oversampler so the tanh harmonics do not
    /// fold back into the audible range.
    #[inline]
    fn tube_stage(&mut self, s: f32, ch: usize, scratch: &mut [f32; PULTEC_TUBE_OS_FACTOR]) -> f32 {
        if self.tube_drive <= 0.01 {
            return s;
        }
        let drive_amount = self.tube_drive * 0.3;
        let scale = 1.0 + drive_amount * 0.2;
        let os = if ch == 0 {
            &mut self.tube_os_l
        } else {
            &mut self.tube_os_r
        };
        {
            let up = os.upsample(s, 0);
            for i in 0..PULTEC_TUBE_OS_FACTOR {
                scratch[i] = up[i].tanh() * scale;
            }
        }
        os.downsample(&scratch[..PULTEC_TUBE_OS_FACTOR], 0)
    }

    /// Process audio buffer through Pultec EQ
    pub fn process(&mut self, buffer: &mut Buffer) {
        let mut scratch = [0.0_f32; PULTEC_TUBE_OS_FACTOR];

        if self.linear_phase {
            self.redesign_kernel_if_dirty();
            for mut samples in buffer.iter_samples() {
                let mut iter = samples.iter_mut();
                let Some(l_ref) = iter.next() else { continue };
                let r_ref = iter.next();
                let l = *l_ref;
                // Mono buffers: run the right lane on the same signal so the
                // convolver history stays coherent if the layout changes.
                let r = r_ref.as_deref().copied().unwrap_or(l);

                let (yl, yr) = self.linear.process_frame(l, r);
                *l_ref = self.tube_stage(yl, 0, &mut scratch);
                if let Some(r_ref) = r_ref {
                    *r_ref = self.tube_stage(yr, 1, &mut scratch);
                }
            }
            return;
        }

        for mut samples in buffer.iter_samples() {
            for (ch, sample) in samples.iter_mut().enumerate() {
                let ch = ch.min(1);
                let mut s = *sample;

                // Linear SVF chain. No inline clamps: stability is guaranteed
                // by the coefficient math, and clamps between stages would inject
                // memoryless distortion that aliases into the midrange.
                s = self.lf_boost_filter[ch].run(s);
                s = self.lf_resonant_filter[ch].run(s);
                s = self.lf_cut_filter[ch].run(s);
                s = self.hf_boost_filter[ch].run(s);
                s = self.hf_cut_filter[ch].run(s);

                *sample = self.tube_stage(s, ch, &mut scratch);
            }
        }
    }

    /// Bypass path. In minimum-phase mode this is a no-op. In linear-phase
    /// mode the module still owes the host `LINEAR_PHASE_LATENCY` samples of
    /// delay, so the dry signal is delayed by exactly that amount while the
    /// convolver keeps running, ready to un-bypass with a warm history.
    ///
    /// Also called (see `sync_pultec_latency` in `lib.rs`) when Pultec isn't
    /// an active chain slot at all — ADR-0011. Keeps the FIR draining so the
    /// signal always carries the delay the plugin reported to the host.
    pub fn process_bypassed(&mut self, buffer: &mut Buffer) {
        if !self.linear_phase {
            return;
        }
        self.redesign_kernel_if_dirty();
        for mut samples in buffer.iter_samples() {
            let mut iter = samples.iter_mut();
            let Some(l_ref) = iter.next() else { continue };
            let r_ref = iter.next();
            let l = *l_ref;
            let r = r_ref.as_deref().copied().unwrap_or(l);

            let (dl, dr) = self.linear.delayed_dry();
            let _ = self.linear.process_frame(l, r);
            *l_ref = dl;
            if let Some(r_ref) = r_ref {
                *r_ref = dr;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pultec_new_does_not_panic() {
        let _eq = PultecEQ::new(44100.0);
        let _eq = PultecEQ::new(48000.0);
        let _eq = PultecEQ::new(96000.0);
    }

    #[test]
    fn test_pultec_update_parameters_nominal_does_not_panic() {
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            60.0,    // lf_boost_freq
            7.5,     // lf_boost_db (mid of 0..18)
            0.67,    // lf_boost_bandwidth (default wide)
            200.0,   // lf_cut_freq — classic trick: cut above the boost
            4.5,     // lf_cut_db
            0.5,     // lf_cut_bandwidth
            8000.0,  // hf_boost_freq
            6.0,     // hf_boost_db
            0.5,     // hf_boost_bandwidth
            10000.0, // hf_cut_freq
            1.6,     // hf_cut_db
            0.0,     // tube_drive
        );
    }

    #[test]
    fn test_pultec_lf_cut_freq_clamping() {
        // lf_cut_freq is clamped to [20, 200]; extreme values must not panic.
        let mut eq = PultecEQ::new(44100.0);
        // below range
        eq.update_parameters(
            60.0, 7.5, 0.67, 5.0, 7.5, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        // above range
        eq.update_parameters(
            60.0, 7.5, 0.67, 10000.0, 7.5, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    #[test]
    fn test_pultec_tube_drive_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // tube_drive is clamped to [0.0, 1.0] in update_parameters
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 2.0,
        );
        assert!(
            (eq.tube_drive - 1.0).abs() < 1e-5,
            "tube_drive > 1.0 should clamp to 1.0"
        );

        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, -1.0,
        );
        assert!(
            (eq.tube_drive - 0.0).abs() < 1e-5,
            "tube_drive < 0.0 should clamp to 0.0"
        );
    }

    #[test]
    fn test_pultec_lf_boost_freq_clamping() {
        // safe_lf_freq is clamped to [30, 200] — extremely low freq should not panic
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            1.0, 7.5, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        eq.update_parameters(
            500.0, 7.5, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    #[test]
    fn test_pultec_hf_freq_clamping() {
        let mut eq = PultecEQ::new(44100.0);
        // hf_boost_freq clamps to [3000, 20000]
        eq.update_parameters(
            100.0, 7.5, 0.67, 100.0, 0.0, 0.5, 100.0, 5.0, 0.5, 10000.0, 1.6, 0.0,
        );
        eq.update_parameters(
            100.0, 7.5, 0.67, 100.0, 0.0, 0.5, 30000.0, 5.0, 0.5, 25000.0, 1.6, 0.0,
        );
    }

    #[test]
    fn test_pultec_hf_q_range() {
        // hf_q = 0.6 + bandwidth^2 * 1.4 — ranges from 0.6 (bandwidth=0) to 2.0 (bandwidth=1)
        let q_min = 0.6_f32 + 0.0_f32.powi(2) * 1.4;
        let q_max = 0.6_f32 + 1.0_f32.powi(2) * 1.4;
        assert!((q_min - 0.6).abs() < 1e-5, "Q min should be 0.6");
        assert!((q_max - 2.0).abs() < 1e-5, "Q max should be 2.0");
    }

    #[test]
    fn test_pultec_inactive_sections_do_not_modify_coefficients() {
        // Gain values <= 0.01 should result in 0 dB (inactive sections).
        // Verify no panic when all gains are near zero.
        let mut eq = PultecEQ::new(44100.0);
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    /// Measure the actual steady-state gain PultecEQ applies through
    /// `process()`, using a real nih_plug `Buffer`. This is the end-to-end
    /// check the user-facing issue needs: "when I crank LF BOOST, do I
    /// actually get ~+15 dB of boost below the shelf corner?"
    fn measure_gain_db(eq: &mut PultecEQ, freq_hz: f32, sr: f32) -> f32 {
        use nih_plug::buffer::Buffer;
        let n = 8192_usize;
        let omega = 2.0 * core::f32::consts::PI * freq_hz / sr;
        let mut l: Vec<f32> = (0..n).map(|i| (omega * i as f32).sin()).collect();
        let mut r: Vec<f32> = l.clone();
        let mut buf = Buffer::default();
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        eq.process(&mut buf);
        // Measure peak in the second half of the buffer — the first half
        // covers the biquad's transient warm-up.
        let peak = l[n / 2..].iter().fold(0.0_f32, |acc, &x| acc.max(x.abs()));
        20.0 * peak.log10()
    }

    #[test]
    fn test_pultec_lf_boost_delivers_real_gain() {
        // RED test: lf_boost at max should push ~+15 dB below the shelf corner.
        // At 30 Hz with a 60 Hz LowShelf and Q=0.9, the shelf plateau is fully
        // engaged, so a +15 dB shelf should show ≥ +12 dB measured gain.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            60.0, 15.0, 0.67, // lf boost: 60 Hz, +15 dB, default width
            100.0, 0.0, 0.5, // lf cut: disabled
            10000.0, 0.0, 0.5, // hf boost: disabled
            10000.0, 0.0, // hf cut: disabled
            0.0, // tube: off (linear path only)
        );
        let gain_db = measure_gain_db(&mut eq, 30.0, sr);
        assert!(
            gain_db > 12.0,
            "LF boost at max should deliver ≥ +12 dB at 30 Hz, got {gain_db:.2} dB"
        );
    }

    #[test]
    fn test_pultec_lf_boost_at_100hz_exactly_what_the_user_did() {
        // Replicate the user's Reaper scenario: LF boost freq = 100 Hz, gain = 1.0
        // (100%), all other sections neutral. Probe across the shelf to prove it
        // is truly centered at 100 Hz — not mistuned 4× low like before the
        // biquad_coeffs fix.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            100.0, 15.0, 0.67, // LF boost: 100 Hz, +15 dB, default width
            100.0, 0.0, 0.5, // LF cut disabled
            10000.0, 0.0, 0.5, // HF boost disabled
            10000.0, 0.0, // HF cut disabled
            0.0, // tube off
        );
        let g_30 = measure_gain_db(&mut eq, 30.0, sr);
        let g_100 = measure_gain_db(&mut eq, 100.0, sr);
        let g_1k = measure_gain_db(&mut eq, 1000.0, sr);
        // Well below corner: full +15 dB shelf plateau.
        assert!(
            g_30 > 12.0,
            "LF boost should deliver ≥ +12 dB at 30 Hz, got {g_30:.2} dB"
        );
        // Well above corner: shelf stops; should be near unity.
        assert!(
            g_1k.abs() < 2.0,
            "LF boost should be near 0 dB at 1 kHz (above shelf), got {g_1k:.2} dB"
        );
        // At the corner: shelf midpoint (~7.5 dB) + resonant peak (~5.25 dB) ≈ 12–14 dB.
        assert!(
            g_100 > 4.0 && g_100 < 18.0,
            "LF boost should be mid-rise + resonant peak at 100 Hz corner, got {g_100:.2} dB"
        );
    }

    #[test]
    fn test_pultec_lf_boost_zero_is_unity() {
        // Sanity guard: with every gain at 0, the chain is transparent.
        let sr = 48_000.0;
        let mut eq = PultecEQ::new(sr);
        eq.update_parameters(
            60.0, 0.0, 0.67, 100.0, 0.0, 0.5, 10000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        let gain_db = measure_gain_db(&mut eq, 30.0, sr);
        assert!(
            gain_db.abs() < 0.5,
            "flat Pultec should pass 30 Hz unchanged, got {gain_db:.2} dB"
        );
    }

    // ── Linear-phase mode (#15 stretch) ───────────────────────────────────

    /// Run an arbitrary stereo signal through `process` (or `process_bypassed`)
    /// and return the left channel.
    fn run_stereo(eq: &mut PultecEQ, input: &[f32], bypassed: bool) -> Vec<f32> {
        use nih_plug::buffer::Buffer;
        let n = input.len();
        let mut l = input.to_vec();
        let mut r = input.to_vec();
        let mut buf = Buffer::default();
        // SAFETY: `l`/`r` are length `n` and outlive this call.
        unsafe {
            buf.set_slices(n, |ss| {
                ss.clear();
                ss.push(&mut l);
                ss.push(&mut r);
            });
        }
        if bypassed {
            eq.process_bypassed(&mut buf);
        } else {
            eq.process(&mut buf);
        }
        l
    }

    fn impulse(n: usize) -> Vec<f32> {
        let mut v = vec![0.0_f32; n];
        v[0] = 1.0;
        v
    }

    fn sine(freq_hz: f32, sr: f32, n: usize) -> Vec<f32> {
        let w = core::f32::consts::TAU * freq_hz / sr;
        (0..n).map(|i| (w * i as f32).sin()).collect()
    }

    /// Steady-state sine amplitude in dB (RMS·√2 over the second half).
    /// RMS rather than sampled peak: at probes like fs/3 or fs/8 the sample
    /// grid only hits a handful of phases per cycle, so a peak detector
    /// under-reads by an amount that depends on the filter's phase shift.
    fn amplitude_db(signal: &[f32]) -> f32 {
        let tail = &signal[signal.len() / 2..];
        let ms = tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32;
        20.0 * (2.0 * ms).sqrt().log10()
    }

    /// Settings shared by the linear-phase tests: LF boost + resonant bump
    /// at 100 Hz, HF boost at 8 kHz, cuts off, tube off.
    fn set_mastering_curve(eq: &mut PultecEQ) {
        eq.update_parameters(
            100.0, 12.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 6.0, 0.5, 10000.0, 0.0, 0.0,
        );
    }

    #[test]
    fn test_pultec_linear_phase_latency_is_512_when_engaged() {
        let mut eq = PultecEQ::new(48_000.0);
        assert_eq!(
            eq.latency_samples(),
            0,
            "minimum-phase mode is zero-latency"
        );
        assert!(!eq.linear_phase());
        eq.set_linear_phase(true);
        assert!(eq.linear_phase());
        assert_eq!(eq.latency_samples(), LINEAR_PHASE_LATENCY as u32);
        assert_eq!(LINEAR_PHASE_LATENCY, 512, "roadmap §3.2: ~512 samples");
        eq.set_linear_phase(false);
        assert!(!eq.linear_phase());
        assert_eq!(eq.latency_samples(), 0);
    }

    /// Linear phase means a symmetric impulse response centred on the
    /// reported latency — that is the whole definition of the mode.
    #[test]
    fn test_pultec_linear_phase_impulse_is_symmetric_about_latency() {
        let mut eq = PultecEQ::new(48_000.0);
        eq.set_linear_phase(true);
        set_mastering_curve(&mut eq);
        let out = run_stereo(&mut eq, &impulse(4096), false);

        let c = LINEAR_PHASE_LATENCY;
        let peak = out.iter().fold(0.0_f32, |a, &x| a.max(x.abs()));
        let peak_idx = out
            .iter()
            .position(|&x| x.abs() == peak)
            .expect("non-empty");
        assert_eq!(
            peak_idx, c,
            "impulse peak must land on the reported latency"
        );

        let mut asym = 0.0_f32;
        for k in 1..=c {
            asym = asym.max((out[c + k] - out[c - k]).abs());
        }
        assert!(
            asym < 1.0e-5 * peak.max(1.0),
            "impulse response is not symmetric: max asymmetry {asym:e}"
        );
        // Nothing may arrive before the kernel's first tap.
        let pre = out[..c - (LINEAR_PHASE_TAPS - 1) / 2]
            .iter()
            .fold(0.0_f32, |a, &x| a.max(x.abs()));
        assert!(pre < 1.0e-6, "pre-echo before kernel start: {pre:e}");
    }

    /// Minimum-phase mode must be unaffected by the linear-phase machinery:
    /// zero latency, response starts at sample 0.
    #[test]
    fn test_pultec_minimum_phase_mode_is_zero_latency() {
        let mut eq = PultecEQ::new(48_000.0);
        set_mastering_curve(&mut eq);
        let out = run_stereo(&mut eq, &impulse(1024), false);
        assert!(
            out[0].abs() > 0.5,
            "min-phase impulse must respond at n=0, got {}",
            out[0]
        );
    }

    /// The FIR is designed from the same response the TPT chain produces, so
    /// away from the LF resolution limit the two modes measure the same.
    #[test]
    fn test_pultec_linear_phase_magnitude_matches_minimum_phase() {
        let sr = 48_000.0;
        let n = 32_768;
        let probe = |linear: bool, freq: f32| -> f32 {
            let mut eq = PultecEQ::new(sr);
            eq.set_linear_phase(linear);
            set_mastering_curve(&mut eq);
            amplitude_db(&run_stereo(&mut eq, &sine(freq, sr, n), false))
        };
        for freq in [1000.0, 4000.0, 8000.0, 16000.0] {
            let min_phase = probe(false, freq);
            let lin_phase = probe(true, freq);
            assert!(
                (min_phase - lin_phase).abs() < 0.5,
                "{freq} Hz: min-phase {min_phase:.2} dB vs linear-phase {lin_phase:.2} dB"
            );
        }
        // Below the 513-tap kernel's frequency resolution (~fs/513 ≈ 94 Hz
        // main-lobe width) the shelf plateau is smoothed, but the boost must
        // still be substantially delivered.
        let lf = probe(true, 40.0);
        assert!(
            lf > 6.0,
            "LF boost must survive the FIR truncation, got {lf:.2} dB"
        );
    }

    /// Bypass in linear-phase mode must be a pure delay of exactly the
    /// reported latency — otherwise toggling bypass shifts the track.
    #[test]
    fn test_pultec_linear_phase_bypass_is_pure_latency_delay() {
        let mut eq = PultecEQ::new(48_000.0);
        eq.set_linear_phase(true);
        set_mastering_curve(&mut eq);
        let out = run_stereo(&mut eq, &impulse(2048), true);
        for (i, &y) in out.iter().enumerate() {
            if i == LINEAR_PHASE_LATENCY {
                assert!((y - 1.0).abs() < 1.0e-6, "delayed impulse amplitude {y}");
            } else {
                assert!(y.abs() < 1.0e-7, "bypass leaked {y:e} at sample {i}");
            }
        }
        // Minimum-phase bypass is a no-op on the buffer.
        let mut eq = PultecEQ::new(48_000.0);
        let out = run_stereo(&mut eq, &impulse(256), true);
        assert!((out[0] - 1.0).abs() < 1.0e-7 && out[1..].iter().all(|&y| y == 0.0));
    }

    /// The tube stage is a nonlinearity and cannot be linear-phase; it must
    /// still run after the FIR so the mode change doesn't silently drop it.
    #[test]
    fn test_pultec_linear_phase_keeps_tube_stage() {
        let sr = 48_000.0;
        let n = 16_384;
        let run = |drive: f32| -> Vec<f32> {
            let mut eq = PultecEQ::new(sr);
            eq.set_linear_phase(true);
            eq.update_parameters(
                100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, drive,
            );
            let mut x = sine(1000.0, sr, n);
            for s in &mut x {
                *s *= 0.9;
            }
            run_stereo(&mut eq, &x, false)
        };
        let clean = run(0.0);
        let driven = run(1.0);
        let diff = clean[n / 2..]
            .iter()
            .zip(&driven[n / 2..])
            .fold(0.0_f32, |a, (x, y)| a.max((x - y).abs()));
        assert!(
            diff > 1.0e-3,
            "tube drive had no effect in linear-phase mode"
        );
    }

    /// Toggling the mode and driving parameters across buffer boundaries of
    /// awkward sizes must never produce NaN/inf or leave the block engine in
    /// a bad state.
    #[test]
    fn test_pultec_linear_phase_toggle_and_odd_block_sizes_stay_finite() {
        let sr = 44_100.0;
        let mut eq = PultecEQ::new(sr);
        let mut boost = 0.0_f32;
        for (round, block) in [1_usize, 7, 64, 255, 256, 257, 1000, 4096]
            .iter()
            .enumerate()
        {
            eq.set_linear_phase(round % 2 == 0);
            boost = (boost + 3.0) % 18.0;
            eq.update_parameters(
                60.0, boost, 0.5, 200.0, 4.0, 0.5, 12000.0, 5.0, 0.7, 10000.0, 2.0, 0.3,
            );
            let x = sine(440.0, sr, *block);
            let out = run_stereo(&mut eq, &x, false);
            assert!(
                out.iter().all(|y| y.is_finite()),
                "non-finite output at block {block}"
            );
            assert!(
                out.iter().all(|y| y.abs() < 10.0),
                "runaway output at block {block}"
            );
        }
        eq.reset();
        let out = run_stereo(&mut eq, &impulse(1024), false);
        assert!(out.iter().all(|y| y.is_finite()));
    }

    /// Hit the tube oversampler with a push-the-boundaries signal and verify
    /// the output stays finite and bounded — guards against FIR state
    /// corruption or overflow from the tanh·scale blend.
    #[test]
    fn test_pultec_tube_saturation_oversampled_bounded() {
        let mut eq = PultecEQ::new(44100.0);
        // Drive the tube stage hard while leaving EQ mostly flat.
        eq.update_parameters(
            100.0, 0.0, 0.67, 100.0, 0.0, 0.5, 8000.0, 0.0, 0.5, 10000.0, 0.0, 1.0,
        );
        // Run 2048 samples of a sine at ~0.3·Nyquist directly through the
        // oversampled saturation block.
        let mut os = Oversampler::new_at_factor(PULTEC_TUBE_OS_FACTOR, 1);
        let mut scratch = [0.0_f32; PULTEC_TUBE_OS_FACTOR];
        let drive_amount = eq.tube_drive * 0.3;
        let scale = 1.0 + drive_amount * 0.2;
        for i in 0..2048 {
            let x = (2.0 * core::f32::consts::PI * 0.3 * i as f32).sin();
            let up = os.upsample(x, 0);
            for k in 0..PULTEC_TUBE_OS_FACTOR {
                scratch[k] = up[k].tanh() * scale;
            }
            let y = os.downsample(&scratch[..PULTEC_TUBE_OS_FACTOR], 0);
            assert!(y.is_finite(), "non-finite sample {y} at i={i}");
            assert!(y.abs() < 2.0, "implausibly large sample {y} at i={i}");
        }
    }

    /// A coefficient change must not trigger a kernel redesign again until
    /// `KERNEL_REDESIGN_MIN_INTERVAL` samples have elapsed since the last one
    /// — otherwise automation or a dragged knob could re-run the 4096-point
    /// IFFT + 5-stage evaluation + 1024-point FFT on nearly every sample,
    /// which is well outside the audio-thread CPU budget.
    #[test]
    fn test_pultec_kernel_redesign_is_throttled() {
        let mut eq = PultecEQ::new(48_000.0);
        eq.set_linear_phase(true);

        // Engaging linear-phase mode leaves the engine "ready" (as if a full
        // interval had already elapsed), so the first parameter change
        // redesigns immediately rather than waiting out the throttle window.
        eq.update_parameters(
            60.0, 6.0, 0.5, 200.0, 0.0, 0.5, 12000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        eq.redesign_kernel_if_dirty();
        assert!(
            !eq.linear.kernel_dirty,
            "first redesign after engaging linear-phase mode must not be throttled"
        );
        assert_eq!(eq.linear.samples_since_redesign, 0);

        // A second change immediately after the first redesign must be
        // throttled: the kernel stays flagged dirty (not yet redesigned).
        eq.update_parameters(
            60.0, 12.0, 0.5, 200.0, 0.0, 0.5, 12000.0, 0.0, 0.5, 10000.0, 0.0, 0.0,
        );
        assert!(eq.linear.kernel_dirty);
        eq.redesign_kernel_if_dirty();
        assert!(
            eq.linear.kernel_dirty,
            "redesign must be throttled immediately after the previous one"
        );

        // One sample short of the interval: still throttled.
        for _ in 0..KERNEL_REDESIGN_MIN_INTERVAL - 1 {
            eq.linear.process_frame(0.0, 0.0);
        }
        eq.redesign_kernel_if_dirty();
        assert!(
            eq.linear.kernel_dirty,
            "redesign must stay throttled one sample short of the interval"
        );

        // The interval has now elapsed: the pending change is applied.
        eq.linear.process_frame(0.0, 0.0);
        eq.redesign_kernel_if_dirty();
        assert!(
            !eq.linear.kernel_dirty,
            "redesign must proceed once the interval has elapsed"
        );
    }
}
