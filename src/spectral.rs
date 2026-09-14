// src/spectral.rs — Lock-free spectrum analysis pipeline.
//
// SpectrumData is written by the audio thread and read by the GUI thread
// with no locks. Safety is achieved by:
//   - Storing f32 magnitude values as their raw u32 bits in AtomicU32.
//   - Using Relaxed ordering for individual bin reads/writes (torn reads
//     of a single f32 produce a valid f32 — worst case a slightly stale value).
//   - Using Release/Acquire ordering on `dirty` to establish happens-before
//     between the audio thread write and the GUI thread read.

use realfft::num_complex::Complex32;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

/// Number of frequency bins published to the GUI: every positive-frequency bin of
/// [`FFT_SIZE`], so the analyzer covers 0 … Nyquist at any sample rate.
pub const SPECTRUM_BINS: usize = FFT_SIZE / 2;

/// FFT size used in lib.rs for accumulation. Declared here so both the
/// audio thread and spectral.rs agree on the constant.
pub const FFT_SIZE: usize = 2048;

/// Lock-free spectrum data shared between the audio thread (writer)
/// and the GUI thread (reader).
pub struct SpectrumData {
    /// Magnitude bins stored as f32 bits for lock-free access.
    bins: Vec<AtomicU32>,
    /// Audio thread sets this after writing; GUI clears it after reading.
    dirty: AtomicBool,
    /// Opt-in runtime probe; carried here because this is the one channel already shared
    /// between the audio thread and the DynEQ view that writes the log.
    #[cfg(feature = "diagnostics")]
    pub probe: crate::diagnostics::Probe,
}

impl SpectrumData {
    pub fn new() -> Self {
        Self {
            bins: (0..SPECTRUM_BINS).map(|_| AtomicU32::new(0)).collect(),
            dirty: AtomicBool::new(false),
            #[cfg(feature = "diagnostics")]
            probe: crate::diagnostics::Probe::new(),
        }
    }

    /// **Audio thread only.** Publish a slice of magnitude values.
    /// Length is silently clamped to SPECTRUM_BINS.
    pub fn write_from_slice(&self, magnitudes: &[f32]) {
        let len = magnitudes.len().min(SPECTRUM_BINS);
        for (i, &mag) in magnitudes.iter().take(len).enumerate() {
            // Safety: mag is a valid f32; storing its bits is always defined.
            self.bins[i].store(mag.to_bits(), Ordering::Relaxed);
        }
        #[cfg(feature = "diagnostics")]
        self.probe.note_frame_written();
        // Release fence: all bin stores above are visible before this store.
        self.dirty.store(true, Ordering::Release);
    }

    /// **GUI thread only.** Copy magnitude values into `out` if new data
    /// is available. Returns `false` when no update was pending.
    pub fn read_into_slice(&self, out: &mut [f32]) -> bool {
        // Acquire fence: makes all bin stores from the audio thread visible.
        if !self.dirty.swap(false, Ordering::Acquire) {
            return false;
        }
        let len = out.len().min(SPECTRUM_BINS);
        for (i, out_bin) in out.iter_mut().take(len).enumerate() {
            *out_bin = f32::from_bits(self.bins[i].load(Ordering::Relaxed));
        }
        true
    }
}

impl Default for SpectrumData {
    fn default() -> Self {
        Self::new()
    }
}

// ── Analyzer display axis ─────────────────────────────────────────────────────

/// Lowest frequency on the DynEQ analyzer's log-frequency axis.
pub const DISPLAY_MIN_HZ: f32 = 20.0;
/// Highest frequency on the analyzer's axis, when Nyquist allows it.
pub const DISPLAY_MAX_HZ: f32 = 20_000.0;
/// Sample rate the analyzer assumes before `initialize()` publishes the real one.
pub const REFERENCE_SAMPLE_RATE: f32 = 44_100.0;

/// The published sample rate, or [`REFERENCE_SAMPLE_RATE`] while none is published yet.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn display_sample_rate(published: f32) -> f32 {
    if published > 0.0 {
        published
    } else {
        REFERENCE_SAMPLE_RATE
    }
}

/// Top of the analyzer axis: Nyquist, capped at [`DISPLAY_MAX_HZ`].
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn display_top_hz(sample_rate: f32) -> f32 {
    (sample_rate * 0.5).min(DISPLAY_MAX_HZ)
}

/// Horizontal position (0 = left edge, 1 = right edge) of `freq_hz` on the log axis.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn freq_to_x_frac(freq_hz: f32, sample_rate: f32) -> f32 {
    let span = (display_top_hz(sample_rate) / DISPLAY_MIN_HZ).ln();
    ((freq_hz.max(DISPLAY_MIN_HZ) / DISPLAY_MIN_HZ).ln() / span).clamp(0.0, 1.0)
}

/// Inverse of [`freq_to_x_frac`].
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn x_frac_to_freq(x_frac: f32, sample_rate: f32) -> f32 {
    DISPLAY_MIN_HZ * (display_top_hz(sample_rate) / DISPLAY_MIN_HZ).powf(x_frac.clamp(0.0, 1.0))
}

/// Fractional FFT bin index of `freq_hz`.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn hz_to_bin(freq_hz: f32, sample_rate: f32) -> f32 {
    freq_hz * FFT_SIZE as f32 / sample_rate
}

/// Magnitude to draw for a display column spanning `lo_hz..hi_hz`: the loudest bin inside the
/// span, or the value interpolated at the span's centre when the span falls between two bins
/// (the low end of the log axis, where one bin covers many columns).
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn column_magnitude(bins: &[f32], lo_hz: f32, hi_hz: f32, sample_rate: f32) -> f32 {
    let Some(last) = bins.len().checked_sub(1) else {
        return 0.0;
    };
    let lo = hz_to_bin(lo_hz, sample_rate);
    let hi = hz_to_bin(hi_hz, sample_rate);
    let first = lo.max(0.0).ceil() as usize;
    let end = hi.max(0.0).floor() as usize;
    if first <= end && first <= last {
        return bins[first..=end.min(last)]
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);
    }
    let pos = (0.5 * (lo + hi)).clamp(0.0, last as f32);
    let i = pos.floor() as usize;
    let j = (i + 1).min(last);
    bins[i] + (bins[j] - bins[i]) * (pos - i as f32)
}

/// Lower and upper edges of a bell band's region: `freq_hz / q` wide and geometrically
/// centred on `freq_hz` (the standard bandwidth definition of a peaking filter's Q).
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn bell_band_edges_hz(freq_hz: f32, q: f32) -> (f32, f32) {
    let k = 0.5 / q.max(1.0e-3);
    let s = (1.0 + k * k).sqrt();
    (freq_hz * (s - k), freq_hz * (s + k))
}

// ── Sidechain masking analysis ────────────────────────────────────────────────
//
// ANALYZE SC runs a `MaskingLearner` on the audio thread for LEARN_SECONDS, then publishes
// up to four band suggestions into `AnalysisResult`. Protocol: the audio thread writes every
// suggestion field with Relaxed ordering, then stores `status` with Release ordering. The GUI
// loads `status` with Acquire ordering before reading the other fields, establishing the
// happens-before relationship.

/// Progress of a sidechain masking analysis, shared between the GUI and the audio thread.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    /// No analysis requested yet.
    Idle = 0,
    /// The GUI requested an analysis the audio thread has not started yet.
    Pending = 1,
    /// Suggestions are available in [`AnalysisResult::suggestions`].
    Ready = 2,
    /// The last analysis found too little signal on the sidechain input.
    NoSidechain = 3,
    /// The sidechain carried signal but the main input was silent at those frequencies.
    NoOverlap = 4,
    /// At least one suggestion was written to the DynEQ band parameters.
    Applied = 5,
    /// The audio thread is accumulating frames; see [`AnalysisResult::progress`].
    Learning = 6,
    /// Learning finished and the suggestions are being computed.
    Finalizing = 7,
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
impl AnalysisStatus {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Pending,
            2 => Self::Ready,
            3 => Self::NoSidechain,
            4 => Self::NoOverlap,
            5 => Self::Applied,
            6 => Self::Learning,
            7 => Self::Finalizing,
            _ => Self::Idle,
        }
    }
}

/// Lock-free mirror of one band's `Option<BandSuggestion>`.
pub struct SuggestionSlot {
    pub active: AtomicBool,
    pub freq_hz: AtomicU32,
    pub q: AtomicU32,
    pub threshold_db: AtomicU32,
    pub score_db: AtomicU32,
}

impl SuggestionSlot {
    fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            freq_hz: AtomicU32::new(0),
            q: AtomicU32::new(0),
            threshold_db: AtomicU32::new(0),
            score_db: AtomicU32::new(0),
        }
    }

    fn store(&self, suggestion: Option<BandSuggestion>) {
        if let Some(s) = suggestion {
            self.freq_hz.store(s.freq_hz.to_bits(), Ordering::Relaxed);
            self.q.store(s.q.to_bits(), Ordering::Relaxed);
            self.threshold_db
                .store(s.threshold_db.to_bits(), Ordering::Relaxed);
            self.score_db.store(s.score_db.to_bits(), Ordering::Relaxed);
        }
        self.active.store(suggestion.is_some(), Ordering::Relaxed);
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub fn load(&self) -> Option<BandSuggestion> {
        self.active.load(Ordering::Relaxed).then(|| BandSuggestion {
            freq_hz: f32::from_bits(self.freq_hz.load(Ordering::Relaxed)),
            q: f32::from_bits(self.q.load(Ordering::Relaxed)),
            threshold_db: f32::from_bits(self.threshold_db.load(Ordering::Relaxed)),
            score_db: f32::from_bits(self.score_db.load(Ordering::Relaxed)),
        })
    }
}

/// Lock-free results of the sidechain masking analysis.
pub struct AnalysisResult {
    /// [`AnalysisStatus`] as `u8`; see [`AnalysisResult::status`].
    status: AtomicU8,
    /// Completed fraction (0–1) of the learning window, as raw f32 bits.
    pub learn_progress: AtomicU32,
    /// Per-band suggestion from the last finished analysis (index 0 = band 1).
    pub suggestions: [SuggestionSlot; 4],
    /// Bit `b` is set once the GUI has applied band `b`'s suggestion.
    pub applied_bands: AtomicU8,
    /// Smoothed overlap score per bin from the last analysis, for the GUI overlay.
    /// Raw values — normalise for display.
    pub overlap_bins: Vec<AtomicU32>,
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(AnalysisStatus::Idle as u8),
            learn_progress: AtomicU32::new(0),
            suggestions: std::array::from_fn(|_| SuggestionSlot::new()),
            applied_bands: AtomicU8::new(0),
            overlap_bins: (0..SPECTRUM_BINS).map(|_| AtomicU32::new(0)).collect(),
        }
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub fn status(&self) -> AnalysisStatus {
        AnalysisStatus::from_u8(self.status.load(Ordering::Acquire))
    }

    pub fn set_status(&self, status: AnalysisStatus) {
        self.status.store(status as u8, Ordering::Release);
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.learn_progress.load(Ordering::Relaxed))
    }

    pub fn set_progress(&self, progress: f32) {
        self.learn_progress
            .store(progress.to_bits(), Ordering::Relaxed);
    }

    /// **Audio thread only.** Clear the previous analysis and report Learning.
    pub fn begin_learning(&self) {
        for slot in &self.suggestions {
            slot.store(None);
        }
        self.applied_bands.store(0, Ordering::Relaxed);
        self.set_progress(0.0);
        self.set_status(AnalysisStatus::Learning);
    }

    /// **Audio thread only.** Publish a finished analysis; `status` is written last.
    pub fn publish(&self, outcome: &LearnOutcome, overlap_score: &[f32]) {
        for (slot, &score) in self.overlap_bins.iter().zip(overlap_score) {
            slot.store(score.to_bits(), Ordering::Relaxed);
        }
        let (suggestions, status) = match outcome {
            LearnOutcome::NoSidechain => ([None; 4], AnalysisStatus::NoSidechain),
            LearnOutcome::NoOverlap => ([None; 4], AnalysisStatus::NoOverlap),
            LearnOutcome::Suggestions(s) => (*s, AnalysisStatus::Ready),
        };
        for (slot, suggestion) in self.suggestions.iter().zip(suggestions) {
            slot.store(suggestion);
        }
        self.set_progress(1.0);
        self.set_status(status);
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

/// FFT-normalised sidechain magnitude below which the sidechain counts as silent (≈ -80 dBFS).
pub const SIDECHAIN_SILENCE_MAG: f32 = 1.0e-4;
/// Length of the window ANALYZE SC listens to.
pub const LEARN_SECONDS: f32 = 3.0;
/// Fewest sidechain-active frames a learning window needs before it can suggest anything.
pub const MIN_ACTIVE_FRAMES: usize = 8;
/// Number of 1/12-octave bands in the main-level history (20 Hz – 20.48 kHz).
pub const LEVEL_BANDS: usize = 120;
/// Overlap peaks further than this below the strongest one are not suggested.
pub const SCORE_FLOOR_DB: f32 = 24.0;

const LEVEL_BANDS_MIN_HZ: f32 = 20.0;
const LEVEL_BANDS_PER_OCTAVE: f32 = 12.0;
const NO_LEVEL_BAND: u8 = u8::MAX;
const SMOOTH_HALF_WIDTH_OCTAVES: f32 = 1.0 / 6.0;
const PEAK_EXCLUSION_OCTAVES: f32 = 1.0 / 3.0;
const MIN_SUGGESTED_Q: f32 = 0.7;
const MAX_SUGGESTED_Q: f32 = 4.0;
/// Compress Down: loud frames (this percentile) should sit `COMPRESS_OFFSET_DB` over threshold.
const COMPRESS_PERCENTILE: f32 = 0.9;
const COMPRESS_OFFSET_DB: f32 = -6.0;
/// Expand Up / Gate: the median frame should sit `EXPAND_OFFSET_DB` under threshold.
const EXPAND_PERCENTILE: f32 = 0.5;
const EXPAND_OFFSET_DB: f32 = 3.0;
const MIN_SUGGESTED_THRESHOLD_DB: f32 = -60.0;
const MAX_SUGGESTED_THRESHOLD_DB: f32 = 0.0;
/// Final step index of [`MaskingLearner::finalize_step`] (step 0 smooths, 1 picks, 2–5 thresholds).
const LAST_FINALIZE_STEP: u8 = 5;

/// A DynEQ band as the learner sees it: the span its FREQ can cover, where FREQ is now, and
/// whether its mode compresses (`downward`) or expands/gates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandRange {
    pub min_hz: f32,
    pub max_hz: f32,
    pub current_hz: f32,
    pub downward: bool,
}

impl BandRange {
    fn contains(&self, freq_hz: f32) -> bool {
        (self.min_hz..=self.max_hz).contains(&freq_hz)
    }
}

/// FREQ, Q and THRESH the learner proposes for one band. `score_db` is the overlap peak's level
/// relative to the strongest peak (0 dB for the strongest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandSuggestion {
    pub freq_hz: f32,
    pub q: f32,
    pub threshold_db: f32,
    pub score_db: f32,
}

/// Result of one learning window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LearnOutcome {
    NoSidechain,
    NoOverlap,
    Suggestions([Option<BandSuggestion>; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LearnPhase {
    Idle,
    Learning,
    Finalizing(u8),
}

#[derive(Clone, Copy, Debug)]
struct OverlapPeak {
    freq_hz: f32,
    q: f32,
    score: f32,
}

/// Learns where the sidechain masks the main input over [`LEARN_SECONDS`] of FFT frames and
/// proposes up to four DynEQ band settings. All buffers are sized in [`MaskingLearner::new`];
/// `arm`, `push_frame` and `finalize_step` never allocate, so they run on the audio thread.
///
/// Frames must be Hann-windowed [`FFT_SIZE`] spectra (`shaping::hann_window`), unnormalised.
/// Per-frame main power per 1/12-octave band is kept so thresholds can be read through each
/// suggested band's unity-peak band-pass, on the same RMS scale as the band detector.
pub struct MaskingLearner {
    sample_rate: f32,
    /// Σw² of the analysis window.
    window_power: f32,
    /// 1/12-octave history band of each FFT bin, or [`NO_LEVEL_BAND`].
    level_band_of_bin: Vec<u8>,
    /// Σ |main|·|sidechain| per bin over active frames; the smoothed score after step 0.
    overlap_sum: Vec<f32>,
    /// Σ |sidechain| per bin over active frames, for locating each peak's centre frequency.
    sidechain_sum: Vec<f32>,
    smoothed: Vec<f32>,
    prefix: Vec<f64>,
    consumed: Vec<bool>,
    /// `frames_target × LEVEL_BANDS` main power per active frame.
    main_level_hist: Vec<f32>,
    band_weights: Vec<f32>,
    scratch_levels: Vec<f32>,
    frames_target: usize,
    frames_seen: usize,
    frames_active: usize,
    phase: LearnPhase,
    suggestions: [Option<BandSuggestion>; 4],
}

impl MaskingLearner {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(1.0);
        let frames_target =
            ((LEARN_SECONDS * sample_rate / FFT_SIZE as f32).ceil() as usize).max(1);
        let bin_hz = sample_rate / FFT_SIZE as f32;
        Self {
            sample_rate,
            window_power: crate::shaping::hann_window(FFT_SIZE)
                .iter()
                .map(|w| w * w)
                .sum(),
            level_band_of_bin: (0..SPECTRUM_BINS)
                .map(|k| level_band_of(k as f32 * bin_hz).map_or(NO_LEVEL_BAND, |j| j as u8))
                .collect(),
            overlap_sum: vec![0.0; SPECTRUM_BINS],
            sidechain_sum: vec![0.0; SPECTRUM_BINS],
            smoothed: vec![0.0; SPECTRUM_BINS],
            prefix: vec![0.0; SPECTRUM_BINS + 1],
            consumed: vec![false; SPECTRUM_BINS],
            main_level_hist: vec![0.0; frames_target * LEVEL_BANDS],
            band_weights: vec![0.0; LEVEL_BANDS],
            scratch_levels: vec![0.0; frames_target],
            frames_target,
            frames_seen: 0,
            frames_active: 0,
            phase: LearnPhase::Idle,
            suggestions: [None; 4],
        }
    }

    /// Start a new learning window, discarding any previous one.
    pub fn arm(&mut self) {
        self.overlap_sum.fill(0.0);
        self.sidechain_sum.fill(0.0);
        self.frames_seen = 0;
        self.frames_active = 0;
        self.suggestions = [None; 4];
        self.phase = LearnPhase::Learning;
    }

    pub fn is_learning(&self) -> bool {
        self.phase == LearnPhase::Learning
    }

    pub fn is_finalizing(&self) -> bool {
        matches!(self.phase, LearnPhase::Finalizing(_))
    }

    /// Completed fraction (0–1) of the learning window.
    pub fn progress(&self) -> f32 {
        (self.frames_seen as f32 / self.frames_target as f32).min(1.0)
    }

    /// Smoothed overlap score per bin, valid once finalization has produced an outcome.
    pub fn overlap_score(&self) -> &[f32] {
        &self.overlap_sum
    }

    /// Accumulate one frame of main and sidechain spectra. Frames where the sidechain is
    /// silent advance the window but are not scored.
    pub fn push_frame(&mut self, main: &[Complex32], sidechain: &[Complex32]) {
        if self.phase != LearnPhase::Learning {
            return;
        }
        let bins = SPECTRUM_BINS.min(main.len()).min(sidechain.len());
        let sidechain_peak = sidechain
            .iter()
            .take(bins)
            .skip(1)
            .map(|c| c.norm_sqr())
            .fold(0.0_f32, f32::max)
            .sqrt()
            * 2.0
            / FFT_SIZE as f32;

        if sidechain_peak >= SIDECHAIN_SILENCE_MAG && self.frames_active < self.frames_target {
            let row_start = self.frames_active * LEVEL_BANDS;
            let row = &mut self.main_level_hist[row_start..row_start + LEVEL_BANDS];
            row.fill(0.0);
            for k in 1..bins {
                let main_power = main[k].norm_sqr();
                let sidechain_mag = sidechain[k].norm();
                self.overlap_sum[k] += main_power.sqrt() * sidechain_mag;
                self.sidechain_sum[k] += sidechain_mag;
                if let Some(level) = row.get_mut(usize::from(self.level_band_of_bin[k])) {
                    *level += main_power;
                }
            }
            self.frames_active += 1;
        }

        self.frames_seen += 1;
        if self.frames_seen >= self.frames_target {
            self.phase = LearnPhase::Finalizing(0);
        }
    }

    /// Do one bounded unit of finalization work (call once per audio block). Returns the
    /// outcome when finalization is complete, after at most six calls.
    pub fn finalize_step(&mut self, bands: &[BandRange; 4]) -> Option<LearnOutcome> {
        let LearnPhase::Finalizing(step) = self.phase else {
            return None;
        };
        let outcome = match step {
            0 => self.smooth_overlap(),
            1 => self.pick_bands(bands),
            _ => {
                self.set_threshold(usize::from(step - 2), bands);
                (step >= LAST_FINALIZE_STEP).then_some(LearnOutcome::Suggestions(self.suggestions))
            }
        };
        self.phase = if outcome.is_some() {
            LearnPhase::Idle
        } else {
            LearnPhase::Finalizing(step + 1)
        };
        outcome
    }

    /// Step 0: average the overlap over active frames and smooth it over ±1/6 octave.
    fn smooth_overlap(&mut self) -> Option<LearnOutcome> {
        if self.frames_active < MIN_ACTIVE_FRAMES {
            self.overlap_sum.fill(0.0);
            return Some(LearnOutcome::NoSidechain);
        }
        let inv_frames = 1.0 / self.frames_active as f32;
        for value in &mut self.overlap_sum {
            *value *= inv_frames;
        }
        smooth_log_frequency(&self.overlap_sum, &mut self.smoothed, &mut self.prefix);
        smooth_log_frequency(&self.smoothed, &mut self.overlap_sum, &mut self.prefix);
        None
    }

    /// Step 1: find up to four overlap peaks within [`SCORE_FLOOR_DB`] of the strongest, then
    /// assign them to bands.
    fn pick_bands(&mut self, bands: &[BandRange; 4]) -> Option<LearnOutcome> {
        let bin_hz = self.sample_rate / FFT_SIZE as f32;
        let top_hz = (0.5 * self.sample_rate).min(level_band_edge_hz(LEVEL_BANDS));
        let first = ((LEVEL_BANDS_MIN_HZ / bin_hz).ceil() as usize).max(1);
        let last = ((top_hz / bin_hz).floor() as usize).min(SPECTRUM_BINS - 2);
        if first >= last {
            return Some(LearnOutcome::NoOverlap);
        }

        let score = &self.overlap_sum;
        let strongest = score[first..=last].iter().copied().fold(0.0_f32, f32::max);
        if !(strongest.is_finite() && strongest > 0.0) {
            return Some(LearnOutcome::NoOverlap);
        }
        let floor = strongest * 10.0_f32.powf(-SCORE_FLOOR_DB / 10.0);

        self.consumed.fill(false);
        let mut peaks = [None::<OverlapPeak>; 4];
        let mut found = 0;
        while found < peaks.len() {
            let consumed = &self.consumed;
            let best = (first..=last)
                .filter(|&k| {
                    !consumed[k]
                        && score[k] > 0.0
                        && score[k] >= floor
                        && score[k] >= score[k - 1]
                        && score[k] >= score[k + 1]
                })
                .max_by(|&a, &b| score[a].total_cmp(&score[b]));
            let Some(k) = best else {
                break;
            };
            let peak = describe_peak(score, &self.sidechain_sum, k, first, last, bin_hz);
            let exclusion = 2.0_f32.powf(PEAK_EXCLUSION_OCTAVES);
            let lo =
                (((peak.freq_hz / exclusion) / bin_hz).floor() as usize).min(k.saturating_sub(1));
            let hi = (((peak.freq_hz * exclusion) / bin_hz).ceil() as usize).max(k + 1);
            for flag in &mut self.consumed[lo..=hi.min(SPECTRUM_BINS - 1)] {
                *flag = true;
            }
            peaks[found] = Some(peak);
            found += 1;
        }

        let assignment = assign_peaks_to_bands(&peaks[..found], bands);
        for (suggestion, peak) in self.suggestions.iter_mut().zip(assignment) {
            *suggestion = peak.and_then(|p| peaks[p]).map(|p| BandSuggestion {
                freq_hz: p.freq_hz,
                q: p.q,
                threshold_db: 0.0,
                score_db: 10.0 * (p.score / strongest).log10(),
            });
        }
        self.suggestions
            .iter()
            .all(Option::is_none)
            .then_some(LearnOutcome::NoOverlap)
    }

    /// Steps 2–5: set `band`'s threshold from the main level its band-pass read in each active
    /// frame — the 90th percentile minus 6 dB when compressing, the median plus 3 dB otherwise.
    fn set_threshold(&mut self, band: usize, bands: &[BandRange; 4]) {
        let (Some(slot), Some(range)) = (self.suggestions.get_mut(band), bands.get(band)) else {
            return;
        };
        let Some(mut suggestion) = *slot else {
            return;
        };
        let frames = self.frames_active.min(self.frames_target);
        if frames == 0 {
            return;
        }

        for (j, weight) in self.band_weights.iter_mut().enumerate() {
            let centre = level_band_edge_hz(j) * 2.0_f32.powf(0.5 / LEVEL_BANDS_PER_OCTAVE);
            let detune = suggestion.q * (centre / suggestion.freq_hz - suggestion.freq_hz / centre);
            *weight = 1.0 / (1.0 + detune * detune);
        }
        let power_to_mean_square = 2.0 / (FFT_SIZE as f32 * self.window_power);
        for (frame, level_db) in self.scratch_levels[..frames].iter_mut().enumerate() {
            let row = &self.main_level_hist[frame * LEVEL_BANDS..(frame + 1) * LEVEL_BANDS];
            let power: f32 = row.iter().zip(&self.band_weights).map(|(p, w)| p * w).sum();
            *level_db = 10.0 * (power * power_to_mean_square).max(1.0e-12).log10();
        }

        let (percentile, offset_db) = if range.downward {
            (COMPRESS_PERCENTILE, COMPRESS_OFFSET_DB)
        } else {
            (EXPAND_PERCENTILE, EXPAND_OFFSET_DB)
        };
        let index = ((frames - 1) as f32 * percentile).round() as usize;
        let (_, level_db, _) =
            self.scratch_levels[..frames].select_nth_unstable_by(index, f32::total_cmp);
        suggestion.threshold_db =
            (*level_db + offset_db).clamp(MIN_SUGGESTED_THRESHOLD_DB, MAX_SUGGESTED_THRESHOLD_DB);
        *slot = Some(suggestion);
    }
}

impl Default for MaskingLearner {
    fn default() -> Self {
        Self::new(REFERENCE_SAMPLE_RATE)
    }
}

/// Lower edge of 1/12-octave history band `j`.
fn level_band_edge_hz(j: usize) -> f32 {
    LEVEL_BANDS_MIN_HZ * 2.0_f32.powf(j as f32 / LEVEL_BANDS_PER_OCTAVE)
}

fn level_band_of(freq_hz: f32) -> Option<usize> {
    if freq_hz < LEVEL_BANDS_MIN_HZ {
        return None;
    }
    let j = (LEVEL_BANDS_PER_OCTAVE * (freq_hz / LEVEL_BANDS_MIN_HZ).log2()) as usize;
    (j < LEVEL_BANDS).then_some(j)
}

/// Box-average each bin of `src` over ±[`SMOOTH_HALF_WIDTH_OCTAVES`] into `dst`, using `prefix`
/// (one longer than `src`) as running-sum workspace.
fn smooth_log_frequency(src: &[f32], dst: &mut [f32], prefix: &mut [f64]) {
    let n = src.len().min(dst.len()).min(prefix.len().saturating_sub(1));
    if n == 0 {
        return;
    }
    prefix[0] = 0.0;
    for k in 0..n {
        prefix[k + 1] = prefix[k] + f64::from(src[k]);
    }
    let ratio = 2.0_f32.powf(SMOOTH_HALF_WIDTH_OCTAVES);
    for (k, out) in dst.iter_mut().enumerate().take(n) {
        let lo = ((k as f32 / ratio).round() as usize).min(k);
        let hi = ((k as f32 * ratio).round() as usize).clamp(k, n - 1);
        *out = ((prefix[hi + 1] - prefix[lo]) / (hi - lo + 1) as f64) as f32;
    }
}

/// Centre frequency and −3 dB bandwidth Q of the score peak at bin `k` (`first <= k <= last`).
///
/// The score picks the region and sets its width, but the centre comes from the sidechain
/// spectrum alone (its loudest bin within ±1 of `k`, refined by parabolic interpolation in dB):
/// the main input's spectral tilt would otherwise drag low peaks, where a bin spans a large
/// fraction of an octave, toward the louder side.
fn describe_peak(
    score: &[f32],
    sidechain: &[f32],
    k: usize,
    first: usize,
    last: usize,
    bin_hz: f32,
) -> OverlapPeak {
    let centre_bin = (k.saturating_sub(1).max(first)..=(k + 1).min(last))
        .max_by(|&a, &b| sidechain[a].total_cmp(&sidechain[b]))
        .unwrap_or(k);
    let db = |i: usize| 20.0 * sidechain[i].max(f32::MIN_POSITIVE).log10();
    let (below, centre, above) = (db(centre_bin - 1), db(centre_bin), db(centre_bin + 1));
    let curvature = below - 2.0 * centre + above;
    let offset = if curvature < -1.0e-6 {
        (0.5 * (below - above) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let freq_hz = (centre_bin as f32 + offset) * bin_hz;

    let half = 0.5 * score[k];
    let mut lo = k;
    while lo > first && score[lo - 1] > half {
        lo -= 1;
    }
    let lo_edge = if score[lo - 1] <= half && score[lo] > score[lo - 1] {
        (lo - 1) as f32 + (half - score[lo - 1]) / (score[lo] - score[lo - 1])
    } else {
        lo as f32
    };
    let mut hi = k;
    while hi < last && score[hi + 1] > half {
        hi += 1;
    }
    let hi_edge = if score[hi + 1] <= half && score[hi] > score[hi + 1] {
        hi as f32 + (score[hi] - half) / (score[hi] - score[hi + 1])
    } else {
        hi as f32
    };
    let width_hz = ((hi_edge - lo_edge) * bin_hz).max(bin_hz);
    OverlapPeak {
        freq_hz,
        q: (freq_hz / width_hz).clamp(MIN_SUGGESTED_Q, MAX_SUGGESTED_Q),
        score: score[k],
    }
}

/// Band → peak index. Tries every assignment of peaks to distinct bands whose FREQ range
/// contains them (at most 5⁴ = 625), preferring the most peaks placed, then the strongest,
/// then the least total octave distance from each band's current FREQ.
fn assign_peaks_to_bands(
    peaks: &[Option<OverlapPeak>],
    bands: &[BandRange; 4],
) -> [Option<usize>; 4] {
    const UNASSIGNED: usize = 4;
    let mut best = [None; 4];
    let mut best_key = (0_usize, 0.0_f32, f32::INFINITY);
    for code in 0..(UNASSIGNED + 1).pow(peaks.len() as u32) {
        let mut digits = code;
        let mut candidate = [None; 4];
        let mut key = (0_usize, 0.0_f32, 0.0_f32);
        let mut valid = true;
        for (index, peak) in peaks.iter().enumerate() {
            let band = digits % (UNASSIGNED + 1);
            digits /= UNASSIGNED + 1;
            let Some(peak) = peak else {
                continue;
            };
            if band == UNASSIGNED {
                continue;
            }
            if candidate[band].is_some() || !bands[band].contains(peak.freq_hz) {
                valid = false;
                break;
            }
            candidate[band] = Some(index);
            key.0 += 1;
            key.1 += peak.score;
            key.2 += (peak.freq_hz / bands[band].current_hz.max(1.0))
                .log2()
                .abs();
        }
        let better = key.0 > best_key.0
            || (key.0 == best_key.0
                && (key.1 > best_key.1 || (key.1 == best_key.1 && key.2 < best_key.2)));
        if valid && better {
            best = candidate;
            best_key = key;
        }
    }
    best
}

/// Status line shown beside the ANALYZE SC / APPLY ALL buttons. `timed_out` marks a request the
/// audio thread never started (Dynamic EQ not in the rack, or no audio processing).
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn analysis_status_text(
    status: AnalysisStatus,
    timed_out: bool,
    progress: f32,
    suggested: usize,
    applied_bands: u8,
) -> String {
    match status {
        AnalysisStatus::Idle => "Route a sidechain to inputs 3/4, play, then ANALYZE SC".into(),
        AnalysisStatus::Pending if timed_out => {
            "No response: Dynamic EQ must be in the rack with audio playing".into()
        }
        AnalysisStatus::Pending | AnalysisStatus::Finalizing => "Analyzing…".into(),
        AnalysisStatus::Learning => {
            format!("Learning… {:.0}%", progress.clamp(0.0, 1.0) * 100.0)
        }
        AnalysisStatus::Ready if suggested == 1 => "1 band suggested".into(),
        AnalysisStatus::Ready => format!("{suggested} bands suggested"),
        AnalysisStatus::NoSidechain => "No sidechain signal: route audio to inputs 3/4".into(),
        AnalysisStatus::NoOverlap => "Sidechain found, but the main input is silent there".into(),
        AnalysisStatus::Applied => {
            let bands: Vec<String> = (0..4)
                .filter(|b| applied_bands & (1 << b) != 0)
                .map(|b| (b + 1).to_string())
                .collect();
            match bands.len() {
                0 => "Applied".into(),
                1 => format!("Applied to band {}", bands[0]),
                _ => format!("Applied to bands {}", bands.join(", ")),
            }
        }
    }
}

/// One-line summary of a suggestion for a band column's suggestion strip.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn suggestion_text(suggestion: &BandSuggestion) -> String {
    let freq = if suggestion.freq_hz >= 1000.0 {
        format!("{:.1} kHz", suggestion.freq_hz / 1000.0)
    } else {
        format!("{:.0} Hz", suggestion.freq_hz)
    };
    format!(
        "{freq} · Q {:.1} · {:.0} dB",
        suggestion.q, suggestion.threshold_db
    )
}

#[cfg(test)]
mod learner_tests {
    use super::*;
    use realfft::RealFftPlanner;

    const SR: f32 = 48_000.0;

    fn default_bands(downward: bool) -> [BandRange; 4] {
        let band = |min_hz, max_hz, current_hz| BandRange {
            min_hz,
            max_hz,
            current_hz,
            downward,
        };
        [
            band(20.0, 2000.0, 200.0),
            band(200.0, 5000.0, 800.0),
            band(1000.0, 15_000.0, 3000.0),
            band(3000.0, 20_000.0, 8000.0),
        ]
    }

    /// Feed consecutive Hann-windowed frames of `main`/`sidechain` until the window is full,
    /// then finalize.
    fn learn(
        learner: &mut MaskingLearner,
        main: &[f32],
        sidechain: &[f32],
        bands: &[BandRange; 4],
    ) -> LearnOutcome {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let window = crate::shaping::hann_window(FFT_SIZE);
        let (mut main_in, mut main_out) = (fft.make_input_vec(), fft.make_output_vec());
        let (mut sc_in, mut sc_out) = (fft.make_input_vec(), fft.make_output_vec());
        let mut scratch = fft.make_scratch_vec();
        learner.arm();
        for (main_frame, sc_frame) in main
            .as_chunks::<FFT_SIZE>()
            .0
            .iter()
            .zip(sidechain.as_chunks::<FFT_SIZE>().0)
        {
            if !learner.is_learning() {
                break;
            }
            for i in 0..FFT_SIZE {
                main_in[i] = main_frame[i] * window[i];
                sc_in[i] = sc_frame[i] * window[i];
            }
            fft.process_with_scratch(&mut main_in, &mut main_out, &mut scratch)
                .unwrap();
            fft.process_with_scratch(&mut sc_in, &mut sc_out, &mut scratch)
                .unwrap();
            learner.push_frame(&main_out, &sc_out);
        }
        assert!(
            learner.is_finalizing(),
            "signal shorter than the learning window"
        );
        for _ in 0..=LAST_FINALIZE_STEP {
            if let Some(outcome) = learner.finalize_step(bands) {
                return outcome;
            }
        }
        panic!(
            "finalization did not finish within {} steps",
            LAST_FINALIZE_STEP + 1
        );
    }

    fn seconds(s: f32) -> usize {
        (s * SR) as usize
    }

    fn sine(freq_hz: f32, amp: f32, start: usize, len: usize) -> Vec<f32> {
        (start..start + len)
            .map(|i| amp * (core::f32::consts::TAU * freq_hz * i as f32 / SR).sin())
            .collect()
    }

    /// Deterministic pink noise (Kellett's economy filter over an LCG), about −20 dBFS RMS.
    fn pink_noise(len: usize) -> Vec<f32> {
        let mut seed = 0x2545_f491_u32;
        let (mut b0, mut b1, mut b2) = (0.0_f32, 0.0_f32, 0.0_f32);
        (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let white = (seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
                b0 = 0.99765 * b0 + white * 0.099_046;
                b1 = 0.963 * b1 + white * 0.296_516_4;
                b2 = 0.57 * b2 + white * 1.052_691_3;
                (b0 + b1 + b2 + white * 0.1848) * 0.05
            })
            .collect()
    }

    /// Kick (60 Hz) on the beat and snare (200 Hz body + 1.5 kHz) on the off-beat, 120 BPM
    /// eighth notes, starting `start` samples into the pattern.
    fn kick_snare(start: usize, len: usize) -> Vec<f32> {
        const BEAT_S: f32 = 0.5;
        (start..start + len)
            .map(|i| {
                let t = i as f32 / SR;
                let in_beat = t % BEAT_S;
                let kick =
                    0.8 * (-in_beat / 0.08).exp() * (core::f32::consts::TAU * 60.0 * t).sin();
                let snare_t = (t + BEAT_S * 0.5) % BEAT_S;
                let snare = (-snare_t / 0.06).exp()
                    * (0.5 * (core::f32::consts::TAU * 200.0 * t).sin()
                        + 0.35 * (core::f32::consts::TAU * 1500.0 * t).sin());
                kick + snare
            })
            .collect()
    }

    fn octaves_between(a: f32, b: f32) -> f32 {
        (a / b).log2().abs()
    }

    fn suggestions(outcome: LearnOutcome) -> [Option<BandSuggestion>; 4] {
        match outcome {
            LearnOutcome::Suggestions(s) => s,
            other => panic!("expected suggestions, got {other:?}"),
        }
    }

    #[test]
    fn learner_suggests_two_bands_for_kick_plus_snare_sc() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let mut learner = MaskingLearner::new(SR);
        let outcome = learn(
            &mut learner,
            &pink_noise(len),
            &kick_snare(0, len),
            &default_bands(true),
        );
        let found = suggestions(outcome);
        let sources = [60.0, 200.0, 1500.0];
        let active: Vec<_> = found.iter().flatten().collect();
        assert!(active.len() >= 2, "{found:?}");
        for s in &active {
            assert!(
                sources
                    .iter()
                    .any(|&f| octaves_between(s.freq_hz, f) <= 1.0 / 6.0),
                "{:.1} Hz is not near a source: {found:?}",
                s.freq_hz
            );
        }
    }

    #[test]
    fn learner_is_stable_across_start_offsets() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let noise = pink_noise(seconds(1.1) + len);
        let bands = default_bands(true);
        let mut learner = MaskingLearner::new(SR);
        let runs: Vec<_> = [0.0, 0.3, 1.1]
            .iter()
            .map(|&offset| {
                let start = seconds(offset);
                suggestions(learn(
                    &mut learner,
                    &noise[start..start + len],
                    &kick_snare(start, len),
                    &bands,
                ))
            })
            .collect();
        for run in &runs[1..] {
            for (band, (a, b)) in runs[0].iter().zip(run).enumerate() {
                match (a, b) {
                    (Some(a), Some(b)) => assert!(
                        octaves_between(a.freq_hz, b.freq_hz) <= 1.0 / 12.0,
                        "band {band}: {a:?} vs {b:?}"
                    ),
                    (None, None) => {}
                    _ => panic!("band {band} differs between runs: {runs:?}"),
                }
            }
        }
    }

    #[test]
    fn learner_threshold_calibration() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let main = sine(1000.0, 0.1 * core::f32::consts::SQRT_2, 0, len);
        let sidechain = sine(1000.0, 0.3, 0, len);
        for (downward, expected_db) in [(true, -26.0), (false, -17.0)] {
            let mut learner = MaskingLearner::new(SR);
            let found = suggestions(learn(
                &mut learner,
                &main,
                &sidechain,
                &default_bands(downward),
            ));
            let s = found
                .iter()
                .flatten()
                .find(|s| (s.freq_hz - 1000.0).abs() < SR / FFT_SIZE as f32)
                .unwrap_or_else(|| panic!("no 1 kHz suggestion: {found:?}"));
            assert!(
                (s.threshold_db - expected_db).abs() <= 1.5,
                "downward={downward}: threshold {:.2} dB, expected {expected_db}",
                s.threshold_db
            );
        }
    }

    #[test]
    fn learner_assigns_1khz_to_the_nearest_containing_band() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let mut learner = MaskingLearner::new(SR);
        let found = suggestions(learn(
            &mut learner,
            &sine(1000.0, 0.5, 0, len),
            &sine(1000.0, 0.3, 0, len),
            &default_bands(true),
        ));
        assert!(found[1].is_some(), "{found:?}");
        assert_eq!(found.iter().flatten().count(), 1, "{found:?}");
    }

    #[test]
    fn learner_silent_sc_is_no_sidechain() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let mut learner = MaskingLearner::new(SR);
        let outcome = learn(
            &mut learner,
            &sine(1000.0, 0.5, 0, len),
            &vec![0.0; len],
            &default_bands(true),
        );
        assert_eq!(outcome, LearnOutcome::NoSidechain);
    }

    #[test]
    fn learner_silent_main_is_no_overlap() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let mut learner = MaskingLearner::new(SR);
        let outcome = learn(
            &mut learner,
            &vec![0.0; len],
            &sine(1000.0, 0.3, 0, len),
            &default_bands(true),
        );
        assert_eq!(outcome, LearnOutcome::NoOverlap);
    }

    #[test]
    fn learner_no_alloc_after_new() {
        let len = seconds(LEARN_SECONDS + 0.5);
        let mut learner = MaskingLearner::new(SR);
        let buffers = |l: &MaskingLearner| {
            [
                (l.overlap_sum.as_ptr() as usize, l.overlap_sum.capacity()),
                (
                    l.sidechain_sum.as_ptr() as usize,
                    l.sidechain_sum.capacity(),
                ),
                (l.smoothed.as_ptr() as usize, l.smoothed.capacity()),
                (l.prefix.as_ptr() as usize, l.prefix.capacity()),
                (l.consumed.as_ptr() as usize, l.consumed.capacity()),
                (
                    l.main_level_hist.as_ptr() as usize,
                    l.main_level_hist.capacity(),
                ),
                (l.band_weights.as_ptr() as usize, l.band_weights.capacity()),
                (
                    l.scratch_levels.as_ptr() as usize,
                    l.scratch_levels.capacity(),
                ),
            ]
        };
        let before = buffers(&learner);
        learn(
            &mut learner,
            &pink_noise(len),
            &kick_snare(0, len),
            &default_bands(true),
        );
        assert_eq!(before, buffers(&learner));
    }

    #[test]
    fn publish_mirrors_the_outcome_and_sets_status_last() {
        let result = AnalysisResult::new();
        result.begin_learning();
        assert_eq!(result.status(), AnalysisStatus::Learning);
        let suggestion = BandSuggestion {
            freq_hz: 2400.0,
            q: 2.1,
            threshold_db: -22.0,
            score_db: -3.0,
        };
        result.publish(
            &LearnOutcome::Suggestions([None, None, Some(suggestion), None]),
            &[0.5; SPECTRUM_BINS],
        );
        assert_eq!(result.status(), AnalysisStatus::Ready);
        assert_eq!(result.suggestions[2].load(), Some(suggestion));
        assert!(result.suggestions[0].load().is_none());
        assert_eq!(result.progress(), 1.0);

        result.begin_learning();
        assert!(result.suggestions.iter().all(|s| s.load().is_none()));
        result.publish(&LearnOutcome::NoSidechain, &[0.0; SPECTRUM_BINS]);
        assert_eq!(result.status(), AnalysisStatus::NoSidechain);
    }

    #[test]
    fn status_round_trips_through_the_atomic() {
        let result = AnalysisResult::new();
        assert_eq!(result.status(), AnalysisStatus::Idle);
        for status in [
            AnalysisStatus::Pending,
            AnalysisStatus::Ready,
            AnalysisStatus::NoSidechain,
            AnalysisStatus::NoOverlap,
            AnalysisStatus::Applied,
            AnalysisStatus::Learning,
            AnalysisStatus::Finalizing,
        ] {
            result.set_status(status);
            assert_eq!(result.status(), status);
        }
    }

    #[test]
    fn status_text_reports_progress_count_and_applied_bands() {
        let pending = analysis_status_text(AnalysisStatus::Pending, false, 0.0, 0, 0);
        let timed_out = analysis_status_text(AnalysisStatus::Pending, true, 0.0, 0, 0);
        assert_ne!(pending, timed_out);
        assert_eq!(
            analysis_status_text(AnalysisStatus::Learning, false, 0.45, 0, 0),
            "Learning… 45%"
        );
        assert_eq!(
            analysis_status_text(AnalysisStatus::Ready, false, 1.0, 3, 0),
            "3 bands suggested"
        );
        assert_eq!(
            analysis_status_text(AnalysisStatus::Applied, false, 1.0, 3, 0b1011),
            "Applied to bands 1, 2, 4"
        );
        assert_eq!(
            analysis_status_text(AnalysisStatus::Applied, false, 1.0, 1, 0b0100),
            "Applied to band 3"
        );
    }

    #[test]
    fn suggestion_text_formats_hz_and_khz() {
        let s = |freq_hz| BandSuggestion {
            freq_hz,
            q: 2.14,
            threshold_db: -22.4,
            score_db: 0.0,
        };
        assert_eq!(suggestion_text(&s(2400.0)), "2.4 kHz · Q 2.1 · -22 dB");
        assert_eq!(suggestion_text(&s(62.0)), "62 Hz · Q 2.1 · -22 dB");
    }
}

// ── GainReductionData ─────────────────────────────────────────────────────────
//
// Lock-free per-band gain reduction written by the audio thread and read by
// the GUI thread for the spectrum overlay. Relaxed ordering is sufficient —
// the GUI only uses these values for display; a stale read is acceptable.

/// Quietest level, in dB, a DynEQ band's trigger meter reports.
pub const DYNEQ_TRIGGER_FLOOR_DB: f32 = -90.0;

/// Lock-free per-band gain reduction (dB) shared with the GUI thread.
pub struct GainReductionData {
    /// Gain reduction amount in dB for each of the 4 DynEQ bands, as raw f32
    /// bits. 0.0 = no reduction; positive values = attenuation amount.
    pub bands: [AtomicU32; 4],
    /// Loudest detector level per band over the last audio block, in dB, as
    /// raw f32 bits. Floored at [`DYNEQ_TRIGGER_FLOOR_DB`].
    pub trigger_db: [AtomicU32; 4],
}

impl GainReductionData {
    pub fn new() -> Self {
        Self {
            bands: std::array::from_fn(|_| AtomicU32::new(0)),
            trigger_db: std::array::from_fn(|_| AtomicU32::new(DYNEQ_TRIGGER_FLOOR_DB.to_bits())),
        }
    }
}

impl Default for GainReductionData {
    fn default() -> Self {
        Self::new()
    }
}

// ── TruePeakData ──────────────────────────────────────────────────────────────
//
// Lock-free per-channel true-peak (ITU-R BS.1770-4 intersample-peak) reading
// written by the audio thread and read by the GUI thread for Punch's
// true-peak meter. Relaxed ordering is sufficient — display only.

/// Lock-free stereo true-peak reading (dBTP) shared with the GUI thread.
pub struct TruePeakData {
    /// Left/right true-peak level in dBTP, as raw f32 bits.
    pub channels: [AtomicU32; 2],
}

/// Floor value (dBTP) a fresh/reset `TruePeakData` reports before any audio
/// has been processed.
pub const TRUE_PEAK_FLOOR_DB: f32 = -120.0;

impl TruePeakData {
    pub fn new() -> Self {
        Self {
            channels: [
                AtomicU32::new(TRUE_PEAK_FLOOR_DB.to_bits()),
                AtomicU32::new(TRUE_PEAK_FLOOR_DB.to_bits()),
            ],
        }
    }
}

impl Default for TruePeakData {
    fn default() -> Self {
        Self::new()
    }
}

/// One-pole envelope-follower step for meter display ballistics (roadmap
/// v2.0 §4.4 "smooth needle ballistics"). Applied purely at the display
/// layer — decouples visible meter motion from per-frame data jitter,
/// independent of any ballistics already implemented by the underlying DSP
/// detector (e.g. Punch's true-peak hold/decay). Uses a fast time constant
/// while rising toward `target` (attack) and a slower one while falling
/// (release), matching standard VU/PPM meter behavior.
pub fn meter_ballistics_step(
    displayed: f32,
    target: f32,
    dt_seconds: f32,
    attack_tc_seconds: f32,
    release_tc_seconds: f32,
) -> f32 {
    let tc = if target > displayed {
        attack_tc_seconds
    } else {
        release_tc_seconds
    };
    if tc <= 0.0 {
        return target;
    }
    let coeff = (-dt_seconds / tc).exp();
    target + (displayed - target) * coeff
}

// ── LevelMeterData ────────────────────────────────────────────────────────────
//
// Generic lock-free scalar meter reading written by the audio thread and read
// by the GUI thread (issue #22 inline metering). One instance per module
// meter (ButterComp2 GR, Transformer/Punch/Sheen saturation) — each writer
// documents its own unit/convention at the call site, e.g. positive dB of
// gain reduction, or a 0.0-1.0 saturation proxy. Relaxed ordering is
// sufficient — display only, a stale read is acceptable.

/// Lock-free single-scalar meter reading shared with the GUI thread.
pub struct LevelMeterData {
    /// Current meter reading, as raw f32 bits. Unit/convention is documented
    /// by each writer (see module doc comment above).
    pub value: AtomicU32,
}

impl LevelMeterData {
    pub fn new() -> Self {
        Self {
            value: AtomicU32::new(0.0_f32.to_bits()),
        }
    }
}

impl Default for LevelMeterData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    // ── SpectrumData ──────────────────────────────────────────────────────────

    #[test]
    fn test_spectrum_data_new_not_dirty() {
        let sd = SpectrumData::new();
        let mut out = vec![0.0_f32; SPECTRUM_BINS];
        assert!(
            !sd.read_into_slice(&mut out),
            "Fresh SpectrumData must not be dirty"
        );
    }

    #[test]
    fn test_spectrum_data_write_read_roundtrip() {
        let sd = SpectrumData::new();
        let input: Vec<f32> = (0..SPECTRUM_BINS).map(|i| i as f32 * 0.001).collect();
        sd.write_from_slice(&input);

        let mut out = vec![0.0_f32; SPECTRUM_BINS];
        assert!(
            sd.read_into_slice(&mut out),
            "Should detect new data after write"
        );
        for (i, (&expected, &actual)) in input.iter().zip(out.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-7,
                "Bin {i} mismatch: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_spectrum_data_dirty_clears_after_read() {
        let sd = SpectrumData::new();
        sd.write_from_slice(&vec![1.0_f32; SPECTRUM_BINS]);

        let mut out = vec![0.0_f32; SPECTRUM_BINS];
        let first = sd.read_into_slice(&mut out);
        let second = sd.read_into_slice(&mut out);

        assert!(first, "First read should see pending data");
        assert!(
            !second,
            "Second read should see no new data (dirty flag cleared)"
        );
    }

    #[test]
    fn test_spectrum_data_oversized_write_does_not_panic() {
        let sd = SpectrumData::new();
        let input = vec![0.5_f32; SPECTRUM_BINS + 100]; // more bins than SPECTRUM_BINS
        sd.write_from_slice(&input); // should clamp silently
        let mut out = vec![0.0_f32; SPECTRUM_BINS];
        assert!(sd.read_into_slice(&mut out));
        for &v in &out {
            assert!((v - 0.5).abs() < 1e-6, "All written bins should be 0.5");
        }
    }

    #[test]
    fn test_spectrum_data_undersized_read_does_not_panic() {
        let sd = SpectrumData::new();
        sd.write_from_slice(&vec![0.25_f32; SPECTRUM_BINS]);
        let mut out = vec![0.0_f32; 16]; // read fewer than SPECTRUM_BINS
        assert!(sd.read_into_slice(&mut out));
        for &v in &out {
            assert!((v - 0.25).abs() < 1e-6, "Read bins should be 0.25");
        }
    }

    #[test]
    fn test_spectrum_data_zero_slice_write_read() {
        let sd = SpectrumData::new();
        sd.write_from_slice(&[]); // empty write still sets dirty flag
        let mut out = vec![0.0_f32; 4];
        // dirty flag is set even on empty write
        let updated = sd.read_into_slice(&mut out);
        assert!(updated, "Empty write should still set dirty");
    }

    // ── f32 bit-packing ───────────────────────────────────────────────────────

    #[test]
    fn test_f32_bit_roundtrip_normal_values() {
        for &v in &[-1.0_f32, -0.5, 0.0, f32::MIN_POSITIVE, 0.5, 1.0, 100.0] {
            let recovered = f32::from_bits(v.to_bits());
            assert_eq!(
                v.to_bits(),
                recovered.to_bits(),
                "bit roundtrip failed for {v}"
            );
        }
    }

    // ── AnalysisResult ────────────────────────────────────────────────────────

    #[test]
    fn test_analysis_result_default_not_ready() {
        let ar = AnalysisResult::new();
        assert_eq!(ar.status(), AnalysisStatus::Idle);
    }

    #[test]
    fn test_analysis_result_default_has_no_suggestions() {
        let ar = AnalysisResult::new();
        assert!(ar.suggestions.iter().all(|s| s.load().is_none()));
        assert_eq!(ar.progress(), 0.0);
    }

    #[test]
    fn test_analysis_result_overlap_bins_length() {
        let ar = AnalysisResult::new();
        assert_eq!(ar.overlap_bins.len(), SPECTRUM_BINS);
    }

    // ── GainReductionData ─────────────────────────────────────────────────────

    #[test]
    fn test_gain_reduction_data_initialized_zero() {
        let grd = GainReductionData::new();
        for (i, band) in grd.bands.iter().enumerate() {
            let val = f32::from_bits(band.load(Ordering::Relaxed));
            assert!(val == 0.0, "Band {i} should be 0.0 dB at init, got {val}");
        }
    }

    #[test]
    fn test_gain_reduction_data_write_read() {
        let grd = GainReductionData::new();
        let test_db = 3.5_f32;
        grd.bands[2].store(test_db.to_bits(), Ordering::Relaxed);
        let recovered = f32::from_bits(grd.bands[2].load(Ordering::Relaxed));
        assert!(
            (recovered - test_db).abs() < 1e-6,
            "GR write/read: expected {test_db}, got {recovered}"
        );
    }

    // ── TruePeakData ──────────────────────────────────────────────────────────

    #[test]
    fn test_true_peak_data_initialized_to_floor() {
        let tpd = TruePeakData::new();
        for (i, ch) in tpd.channels.iter().enumerate() {
            let val = f32::from_bits(ch.load(Ordering::Relaxed));
            assert!(
                (val - TRUE_PEAK_FLOOR_DB).abs() < 1e-6,
                "Channel {i} should init at the floor ({TRUE_PEAK_FLOOR_DB} dBTP), got {val}"
            );
        }
    }

    #[test]
    fn test_true_peak_data_write_read() {
        let tpd = TruePeakData::new();
        let test_db = -0.8_f32;
        tpd.channels[1].store(test_db.to_bits(), Ordering::Relaxed);
        let recovered = f32::from_bits(tpd.channels[1].load(Ordering::Relaxed));
        assert!(
            (recovered - test_db).abs() < 1e-6,
            "True-peak write/read: expected {test_db}, got {recovered}"
        );
    }

    // ── meter_ballistics_step ────────────────────────────────────────────────

    #[test]
    fn test_meter_ballistics_rises_toward_target_on_attack() {
        let next = meter_ballistics_step(-60.0, 0.0, 0.02, 0.02, 0.3);
        assert!(
            next > -60.0 && next < 0.0,
            "A rising target should move the displayed value partway toward it, got {next}"
        );
    }

    #[test]
    fn test_meter_ballistics_falls_toward_target_on_release() {
        let next = meter_ballistics_step(0.0, -60.0, 0.02, 0.02, 0.3);
        assert!(
            next < 0.0 && next > -60.0,
            "A falling target should move the displayed value partway toward it, got {next}"
        );
    }

    #[test]
    fn test_meter_ballistics_attack_is_faster_than_release() {
        let dt = 0.02;
        let attack_step = meter_ballistics_step(-60.0, 0.0, dt, 0.02, 0.3);
        let release_step = meter_ballistics_step(0.0, -60.0, dt, 0.02, 0.3);
        let attack_progress = attack_step - (-60.0);
        let release_progress = 0.0 - release_step;
        assert!(
            attack_progress > release_progress,
            "Attack (tc=0.02s) should close more distance per step than release (tc=0.3s): \
             attack_progress={attack_progress}, release_progress={release_progress}"
        );
    }

    #[test]
    fn test_meter_ballistics_converges_to_target_over_many_steps() {
        let mut displayed = -60.0_f32;
        for _ in 0..500 {
            displayed = meter_ballistics_step(displayed, 0.0, 0.02, 0.02, 0.3);
        }
        assert!(
            (displayed - 0.0).abs() < 0.01,
            "Repeated steps should converge to the target, got {displayed}"
        );
    }

    #[test]
    fn test_meter_ballistics_zero_dt_holds_displayed_value() {
        let next = meter_ballistics_step(-12.0, 0.0, 0.0, 0.02, 0.3);
        assert!(
            (next - -12.0).abs() < 1e-6,
            "Zero elapsed time should not move the displayed value, got {next}"
        );
    }

    // ── LevelMeterData ────────────────────────────────────────────────────────

    #[test]
    fn test_level_meter_data_initialized_zero() {
        let lmd = LevelMeterData::new();
        let val = f32::from_bits(lmd.value.load(Ordering::Relaxed));
        assert!(val == 0.0, "LevelMeterData should init at 0.0, got {val}");
    }

    #[test]
    fn test_level_meter_data_write_read() {
        let lmd = LevelMeterData::new();
        let test_val = 6.2_f32;
        lmd.value.store(test_val.to_bits(), Ordering::Relaxed);
        let recovered = f32::from_bits(lmd.value.load(Ordering::Relaxed));
        assert!(
            (recovered - test_val).abs() < 1e-6,
            "LevelMeterData write/read: expected {test_val}, got {recovered}"
        );
    }

    // ── Constants ─────────────────────────────────────────────────────────────

    #[test]
    fn test_spectrum_constants_sane() {
        assert_eq!(FFT_SIZE, 2048);
        assert_eq!(SPECTRUM_BINS, FFT_SIZE / 2);
    }

    // ── Analyzer display axis (editor.rs SpectrumCanvas) ─────────────────────

    const SAMPLE_RATES: [f32; 5] = [44_100.0, 48_000.0, 88_200.0, 96_000.0, 192_000.0];

    #[test]
    fn test_display_axis_round_trips() {
        for &sr in &SAMPLE_RATES {
            for &f in &[20.0_f32, 63.0, 200.0, 1000.0, 5000.0, 15_000.0, 20_000.0] {
                let back = x_frac_to_freq(freq_to_x_frac(f, sr), sr);
                assert!(
                    (back / f - 1.0).abs() < 1e-3,
                    "{f} Hz at {sr} Hz: round trip gave {back}"
                );
            }
        }
    }

    #[test]
    fn test_display_axis_spans_20hz_to_20khz_or_nyquist() {
        assert_eq!(freq_to_x_frac(DISPLAY_MIN_HZ, 48_000.0), 0.0);
        assert!((freq_to_x_frac(DISPLAY_MAX_HZ, 48_000.0) - 1.0).abs() < 1e-6);
        assert!((freq_to_x_frac(16_000.0, 32_000.0) - 1.0).abs() < 1e-6);
        assert!(freq_to_x_frac(12_000.0, 48_000.0) < 0.95);
        let low_decade = freq_to_x_frac(1000.0, 48_000.0) - freq_to_x_frac(100.0, 48_000.0);
        let high_decade = freq_to_x_frac(10_000.0, 48_000.0) - freq_to_x_frac(1000.0, 48_000.0);
        assert!(
            (low_decade - high_decade).abs() < 1e-5,
            "decades must be equally wide"
        );
    }

    #[test]
    fn test_display_sample_rate_falls_back_before_publication() {
        assert_eq!(display_sample_rate(0.0), REFERENCE_SAMPLE_RATE);
        assert_eq!(display_sample_rate(96_000.0), 96_000.0);
    }

    #[test]
    fn test_tone_peaks_at_the_column_of_its_frequency() {
        const WIDTH: usize = 900;
        for &sr in &SAMPLE_RATES {
            for &tone_bin in &[3_usize, 43, 128, 700] {
                let tone_hz = tone_bin as f32 * sr / FFT_SIZE as f32;
                if !(DISPLAY_MIN_HZ..=display_top_hz(sr)).contains(&tone_hz) {
                    continue;
                }
                let mut bins = vec![0.0_f32; SPECTRUM_BINS];
                bins[tone_bin] = 1.0;
                let column_hz = |c: usize| x_frac_to_freq(c as f32 / WIDTH as f32, sr);
                let magnitude =
                    |c: usize| column_magnitude(&bins, column_hz(c), column_hz(c + 1), sr);
                let peak_column = (0..WIDTH)
                    .max_by(|&a, &b| magnitude(a).total_cmp(&magnitude(b)))
                    .unwrap();
                let expected = freq_to_x_frac(tone_hz, sr) * WIDTH as f32;
                assert!(
                    (peak_column as f32 - expected).abs() <= 1.5,
                    "{tone_hz} Hz at {sr} Hz: peak drawn at column {peak_column}, marker at {expected}"
                );
            }
        }
    }

    #[test]
    fn test_column_magnitude_interpolates_between_bins() {
        let sr = 48_000.0;
        let hz = |bin: f32| bin * sr / FFT_SIZE as f32;
        let mut bins = vec![0.0_f32; SPECTRUM_BINS];
        bins[2] = 1.0;
        let between = column_magnitude(&bins, hz(2.24), hz(2.26), sr);
        assert!((between - 0.75).abs() < 1e-3, "got {between}");
        assert_eq!(column_magnitude(&[], 100.0, 200.0, sr), 0.0);
    }

    #[test]
    fn test_bell_band_edges_follow_freq_and_q() {
        for &(f, q) in &[
            (200.0_f32, 1.0_f32),
            (800.0, 0.3),
            (3000.0, 8.0),
            (8000.0, 2.5),
        ] {
            let (lo, hi) = bell_band_edges_hz(f, q);
            assert!(lo < f && f < hi, "{f} Hz must sit inside {lo}..{hi}");
            assert!(
                ((lo * hi).sqrt() / f - 1.0).abs() < 1e-4,
                "{lo}..{hi} not centred on {f}"
            );
            assert!(
                ((hi - lo) - f / q).abs() < f * 1e-4,
                "{lo}..{hi} must be f/Q wide"
            );
        }
        let (wide_lo, wide_hi) = bell_band_edges_hz(1000.0, 0.5);
        let (narrow_lo, narrow_hi) = bell_band_edges_hz(1000.0, 4.0);
        assert!(wide_lo < narrow_lo && wide_hi > narrow_hi);
    }
}
