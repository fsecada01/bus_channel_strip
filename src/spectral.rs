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
        assert!(
            !ar.ready.load(Ordering::Relaxed),
            "AnalysisResult should not be ready by default"
        );
    }

    #[test]
    fn test_analysis_result_default_freq_is_1khz() {
        let ar = AnalysisResult::new();
        let freq = f32::from_bits(ar.target_freq.load(Ordering::Relaxed));
        assert!(
            (freq - 1000.0).abs() < 0.1,
            "Default target_freq should be 1000 Hz, got {freq}"
        );
    }

    #[test]
    fn test_analysis_result_default_threshold_is_minus_18db() {
        let ar = AnalysisResult::new();
        let thresh = f32::from_bits(ar.target_threshold_db.load(Ordering::Relaxed));
        assert!(
            (thresh - (-18.0)).abs() < 0.1,
            "Default threshold should be -18 dB, got {thresh}"
        );
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

    // ── Constants ─────────────────────────────────────────────────────────────

    #[test]
    fn test_spectrum_constants_sane() {
        assert_eq!(SPECTRUM_BINS, 512);
        assert_eq!(FFT_SIZE, 2048);
        // FFT_SIZE >= 2 × SPECTRUM_BINS ensures proper positive-frequency coverage
        assert!(FFT_SIZE >= SPECTRUM_BINS * 2);
    }
}
