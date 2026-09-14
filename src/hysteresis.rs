//! Single-cell play-operator hysteresis (#16, ADR-0012).

use crate::shaping::one_pole_coeff;

/// Play-operator half-width at `amount = 1.0`, as a fraction of the recent input peak.
///
/// Relative rather than absolute: a fixed half-width acts as a dead band that silences
/// quiet signals and crossover-distorts everything else (ADR-0012 amendment).
const MAX_RELATIVE_WIDTH: f32 = 0.06;
/// Release of the peak follower that scales the loop width.
const LEVEL_RELEASE_S: f32 = 0.1;
/// Ceiling on the tracked level; also maps a NaN input to a finite level that decays.
const MAX_TRACKED_LEVEL: f32 = 16.0;
/// Tracked levels below this snap to zero so silence cannot leave subnormal state.
const LEVEL_FLOOR: f32 = 1.0e-20;

pub struct HysteresisCell {
    y_prev: f32,
    level: f32,
    level_release: f32,
}

impl HysteresisCell {
    /// `sample_rate` is the rate the cell is clocked at — the oversampled rate when it
    /// runs inside an oversampling loop.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            y_prev: 0.0,
            level: 0.0,
            level_release: one_pole_coeff(LEVEL_RELEASE_S, sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.y_prev = 0.0;
        self.level = 0.0;
    }

    /// Discrete play (backlash) operator: `y[n] = max(x[n]-r, min(x[n]+r, y[n-1]))`, with
    /// `r = MAX_RELATIVE_WIDTH * amount * peak(|x|)`, so the loop keeps the same shape at
    /// any level. `amount` is clamped to 0..=1; at `amount <= 0` this is an exact
    /// passthrough that keeps `y_prev` and the level tracking `x`.
    #[inline]
    pub fn process(&mut self, x: f32, amount: f32) -> f32 {
        let magnitude = x.abs().min(MAX_TRACKED_LEVEL);
        if magnitude > self.level {
            self.level = magnitude;
        } else {
            self.level += (magnitude - self.level) * self.level_release;
            if self.level < LEVEL_FLOOR {
                self.level = 0.0;
            }
        }

        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            self.y_prev = x;
            return x;
        }
        let r = MAX_RELATIVE_WIDTH * amount * self.level;
        let y = (x - r).max((x + r).min(self.y_prev));
        self.y_prev = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 192_000.0;

    fn sine(amp: f32, i: usize) -> f32 {
        ((std::f64::consts::TAU * 440.0 * i as f64 / SR as f64).sin() * amp as f64) as f32
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt()
    }

    #[test]
    fn zero_amount_is_exact_passthrough() {
        let mut h = HysteresisCell::new(SR);
        for &x in &[-1.0_f32, -0.3, 0.0, 0.4, 1.0] {
            assert_eq!(h.process(x, 0.0), x);
        }
    }

    #[test]
    fn reset_matches_a_fresh_cell() {
        let mut h = HysteresisCell::new(SR);
        h.process(2.0, 0.5);
        h.reset();
        let mut fresh = HysteresisCell::new(SR);
        for &x in &[0.05_f32, 0.3, -0.2, 0.01] {
            assert_eq!(h.process(x, 0.5), fresh.process(x, 0.5));
        }
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let mut h = HysteresisCell::new(SR);
        for i in 0..2048 {
            let x = (i as f32 * 0.037).sin() * 1.5;
            let y = h.process(x, 0.7);
            assert!(y.is_finite(), "non-finite at {i}: {y}");
            assert!(y.abs() < 2.0, "implausibly large at {i}: {y}");
        }
    }

    #[test]
    fn amount_outside_0_1_is_clamped() {
        let mut over = HysteresisCell::new(SR);
        let mut at_max = HysteresisCell::new(SR);
        for &x in &[-2.0_f32, -0.5, 0.0, 0.5, 2.0] {
            assert_eq!(over.process(x, 5.0), at_max.process(x, 1.0));
        }

        let mut negative = HysteresisCell::new(SR);
        for &x in &[-1.0_f32, -0.3, 0.0, 0.4, 1.0] {
            assert_eq!(negative.process(x, -3.0), x);
        }
    }

    /// The defining property of hysteresis: the same input value produces different output
    /// depending on approach direction, and the two rails differ by exactly `2r`.
    #[test]
    fn approach_direction_determines_which_rail_is_hit() {
        let amount = 0.6_f32;

        let mut descending = HysteresisCell::new(SR);
        descending.process(3.0, amount);
        let from_above = descending.process(1.0, amount);
        let r = MAX_RELATIVE_WIDTH * amount * descending.level;

        let mut ascending = HysteresisCell::new(SR);
        ascending.process(-3.0, amount);
        let from_below = ascending.process(1.0, amount);

        assert!(
            (from_above - (1.0 + r)).abs() < 1.0e-6,
            "got {from_above}, want {}",
            1.0 + r
        );
        assert!(
            (from_below - (1.0 - r)).abs() < 1.0e-6,
            "got {from_below}, want {}",
            1.0 - r
        );
    }

    /// Regression: the old absolute half-width (0.15 at amount 1) swallowed any signal
    /// quieter than the dead band.
    #[test]
    fn quiet_signal_is_not_swallowed() {
        let mut h = HysteresisCell::new(SR);
        let amp = 0.01;
        let n = 19_200;
        let input: Vec<f32> = (0..n).map(|i| sine(amp, i)).collect();
        let output: Vec<f32> = input.iter().map(|&x| h.process(x, 1.0)).collect();
        let ratio = rms(&output[n / 2..]) / rms(&input[n / 2..]);
        assert!(
            ratio > 0.9,
            "-40 dBFS sine lost level through the cell: ratio {ratio}"
        );
    }

    #[test]
    fn loop_shape_is_level_independent() {
        let residual_ratio = |amp: f32| {
            let mut h = HysteresisCell::new(SR);
            let n = 19_200;
            let input: Vec<f32> = (0..n).map(|i| sine(amp, i)).collect();
            let residual: Vec<f32> = input.iter().map(|&x| h.process(x, 1.0) - x).collect();
            rms(&residual[n / 2..]) / rms(&input[n / 2..])
        };
        let quiet_db = 20.0 * residual_ratio(0.01).log10();
        let loud_db = 20.0 * residual_ratio(0.5).log10();
        assert!(
            (quiet_db - loud_db).abs() < 1.0,
            "loop shape changed with level: quiet {quiet_db} dB, loud {loud_db} dB"
        );
    }

    #[test]
    fn non_finite_input_does_not_latch() {
        let mut h = HysteresisCell::new(SR);
        h.process(f32::NAN, 1.0);
        let n = SR as usize;
        let output: Vec<f32> = (0..n).map(|i| h.process(sine(0.5, i), 1.0)).collect();
        assert!(output.iter().all(|y| y.is_finite()));
        assert!(
            rms(&output[n / 2..]) > 0.3,
            "cell stayed stuck after a NaN input"
        );
    }
}
