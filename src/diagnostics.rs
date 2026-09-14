//! Opt-in runtime probe, compiled only with `--features diagnostics` (`just deploy-diag`).
//!
//! The audio thread records per-slot peak levels, the IO layout, the slot order and DynEQ
//! analyzer counters; the DynEQ spectrum view appends them to `%TEMP%\bcs_diag.log` every 2 s
//! while it is open. Every writer is lock-free and allocation-free. Peaks use plain
//! load/compare/store: the audio thread is the only writer and a lost update between the
//! GUI's swap and the next block only drops one block's peak.

use std::sync::atomic::{AtomicU32, Ordering};

/// Stage slots: 0 = host input, 1..=7 = after each rack slot, 8 = plugin output,
/// 9 = sidechain input.
pub const STAGES: usize = 10;
/// Stage peak value meaning "did not run since the last read".
pub const STAGE_NOT_RUN: f32 = -1.0;

pub struct Probe {
    blocks: AtomicU32,
    frames_written: AtomicU32,
    peak_since_read: AtomicU32,
    non_finite_blocks: AtomicU32,
    stage_peaks: [AtomicU32; STAGES],
    layout: AtomicU32,
    order: AtomicU32,
}

impl Probe {
    pub fn new() -> Self {
        Self {
            blocks: AtomicU32::new(0),
            frames_written: AtomicU32::new(0),
            peak_since_read: AtomicU32::new(0.0_f32.to_bits()),
            non_finite_blocks: AtomicU32::new(0),
            stage_peaks: std::array::from_fn(|_| AtomicU32::new(STAGE_NOT_RUN.to_bits())),
            layout: AtomicU32::new(0),
            order: AtomicU32::new(0),
        }
    }

    /// **Audio thread.** Count one published analyzer FFT frame.
    pub fn note_frame_written(&self) {
        self.frames_written.fetch_add(1, Ordering::Relaxed);
    }

    /// **Audio thread.** Count a block seen by the DynEQ module and fold its peak (from
    /// [`peak`]) into the running max; a non-finite peak is counted separately.
    pub fn note_block(&self, peak: f32) {
        self.blocks.fetch_add(1, Ordering::Relaxed);
        if peak.is_finite() {
            fold_max(&self.peak_since_read, peak);
        } else {
            self.non_finite_blocks.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// **GUI thread.** `(blocks, frames_written, peak_since_last_call, non_finite_blocks)`;
    /// resets the peak.
    pub fn take_counters(&self) -> (u32, u32, f32, u32) {
        (
            self.blocks.load(Ordering::Relaxed),
            self.frames_written.load(Ordering::Relaxed),
            f32::from_bits(
                self.peak_since_read
                    .swap(0.0_f32.to_bits(), Ordering::Relaxed),
            ),
            self.non_finite_blocks.load(Ordering::Relaxed),
        )
    }

    /// **Audio thread.** Fold a stage's peak into its running max.
    pub fn note_stage_peak(&self, stage: usize, peak: f32) {
        if let Some(slot) = self.stage_peaks.get(stage) {
            fold_max(slot, peak);
        }
    }

    /// **Audio thread.** Packed IO layout (`bypass<<31 | main_ch<<24 | aux_buses<<20 |
    /// aux_ch<<16 | samples`) and slot order (3 bits per slot).
    pub fn note_layout(&self, layout: u32, order: u32) {
        self.layout.store(layout, Ordering::Relaxed);
        self.order.store(order, Ordering::Relaxed);
    }

    /// **GUI thread.** `(stage_peaks, layout, order)`; resets the stage peaks to
    /// [`STAGE_NOT_RUN`].
    pub fn take_stages(&self) -> ([f32; STAGES], u32, u32) {
        (
            std::array::from_fn(|i| {
                f32::from_bits(self.stage_peaks[i].swap(STAGE_NOT_RUN.to_bits(), Ordering::Relaxed))
            }),
            self.layout.load(Ordering::Relaxed),
            self.order.load(Ordering::Relaxed),
        )
    }
}

fn fold_max(slot: &AtomicU32, value: f32) {
    if value > f32::from_bits(slot.load(Ordering::Relaxed)) {
        slot.store(value.to_bits(), Ordering::Relaxed);
    }
}

/// Absolute peak over all channels. Any non-finite sample returns infinity, because `max`
/// would otherwise skip NaN and report silence.
pub fn peak(channels: &[&mut [f32]]) -> f32 {
    channels
        .iter()
        .flat_map(|ch| ch.iter())
        .fold(0.0_f32, |m, &s| {
            if s.is_finite() {
                m.max(s.abs())
            } else {
                f32::INFINITY
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_peaks_reset_to_not_run_after_take() {
        let probe = Probe::new();
        probe.note_stage_peak(3, 0.5);
        probe.note_stage_peak(3, 0.25);
        let (stages, _, _) = probe.take_stages();
        assert_eq!(stages[3], 0.5);
        assert_eq!(stages[0], STAGE_NOT_RUN);
        assert_eq!(probe.take_stages().0[3], STAGE_NOT_RUN);
    }

    #[test]
    fn non_finite_block_is_counted_not_folded() {
        let probe = Probe::new();
        let mut l = vec![0.1_f32, f32::NAN];
        let mut r = vec![0.2_f32, 0.0];
        let channels: [&mut [f32]; 2] = [&mut l, &mut r];
        probe.note_block(peak(&channels));
        probe.note_block(0.3);
        let (blocks, _, block_peak, non_finite) = probe.take_counters();
        assert_eq!((blocks, non_finite), (2, 1));
        assert_eq!(block_peak, 0.3);
    }
}
