// src/spectral.rs — Lock-free spectrum analysis pipeline.
//
// SpectrumData is written by the audio thread and read by the GUI thread
// with no locks. Safety is achieved by:
//   - Storing f32 magnitude values as their raw u32 bits in AtomicU32.
//   - Using Relaxed ordering for individual bin reads/writes (torn reads
//     of a single f32 produce a valid f32 — worst case a slightly stale value).
//   - Using Release/Acquire ordering on `dirty` to establish happens-before
//     between the audio thread write and the GUI thread read.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Number of frequency bins published to the GUI.
/// With FFT_SIZE = 2048 this covers 0 … fs/4 Hz (all useful audio range
/// at 44.1 kHz through 192 kHz sample rates).
pub const SPECTRUM_BINS: usize = 512;

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
}

impl SpectrumData {
    pub fn new() -> Self {
        Self {
            bins: (0..SPECTRUM_BINS).map(|_| AtomicU32::new(0)).collect(),
            dirty: AtomicBool::new(false),
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

// ── AnalysisResult ────────────────────────────────────────────────────────────
//
// Lock-free result of the one-shot sidechain masking analysis.
// Written exclusively by the audio thread; read exclusively by the GUI thread.
// Protocol: audio thread writes all fields with Relaxed ordering, then stores
// `ready = true` with Release ordering. GUI reads `ready` with Acquire ordering
// before reading the other fields, establishing the happens-before relationship.

/// Lock-free analysis results for the sidechain masking feature.
pub struct AnalysisResult {
    /// Audio thread sets this after writing results; GUI reads then clears it.
    pub ready: AtomicBool,
    /// Index of the suggested DynEQ band to target (0 = LOW … 3 = HIGH).
    pub target_band: AtomicU32,
    /// Suggested center frequency in Hz, stored as raw f32 bits.
    pub target_freq: AtomicU32,
    /// Suggested threshold in dB, stored as raw f32 bits.
    pub target_threshold_db: AtomicU32,
    /// Per-bin overlap product (main_mag × sidechain_mag) for the GUI overlay.
    /// Values are raw magnitudes — normalise for display.
    pub overlap_bins: Vec<AtomicU32>,
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            target_band: AtomicU32::new(0),
            target_freq: AtomicU32::new((1000.0_f32).to_bits()),
            target_threshold_db: AtomicU32::new((-18.0_f32).to_bits()),
            overlap_bins: (0..SPECTRUM_BINS).map(|_| AtomicU32::new(0)).collect(),
        }
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

// ── GainReductionData ─────────────────────────────────────────────────────────
//
// Lock-free per-band gain reduction written by the audio thread and read by
// the GUI thread for the spectrum overlay. Relaxed ordering is sufficient —
// the GUI only uses these values for display; a stale read is acceptable.

/// Lock-free per-band gain reduction (dB) shared with the GUI thread.
pub struct GainReductionData {
    /// Gain reduction amount in dB for each of the 4 DynEQ bands, as raw f32
    /// bits. 0.0 = no reduction; positive values = attenuation amount.
    pub bands: [AtomicU32; 4],
}

impl GainReductionData {
    pub fn new() -> Self {
        Self {
            bands: [
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ],
        }
    }
}

impl Default for GainReductionData {
    fn default() -> Self {
        Self::new()
    }
}
