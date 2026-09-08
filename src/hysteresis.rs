//! Single-cell play-operator hysteresis (#16, ADR-0012).

/// Play-operator half-width at `amount = 1.0`, in linear amplitude units.
const MAX_THRESHOLD: f32 = 0.15;

pub struct HysteresisCell {
    y_prev: f32,
}

impl HysteresisCell {
    pub fn new() -> Self {
        Self { y_prev: 0.0 }
    }

    pub fn reset(&mut self) {
        self.y_prev = 0.0;
    }

    /// Discrete play (backlash) operator: `y[n] = max(x[n]-r, min(x[n]+r, y[n-1]))`.
    /// `amount` (0..=1) sets the loop half-width `r`; at `amount <= 0` this is
    /// an exact passthrough that keeps `y_prev` tracking `x`.
    #[inline]
    pub fn process(&mut self, x: f32, amount: f32) -> f32 {
        let r = MAX_THRESHOLD * amount.clamp(0.0, 1.0);
        if r < 1.0e-6 {
            self.y_prev = x;
            return x;
        }
        let y = (x - r).max((x + r).min(self.y_prev));
        self.y_prev = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_amount_is_exact_passthrough() {
        let mut h = HysteresisCell::new();
        for &x in &[-1.0_f32, -0.3, 0.0, 0.4, 1.0] {
            assert_eq!(h.process(x, 0.0), x);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut h = HysteresisCell::new();
        // Pin the state to the upper rail (~1.925) then reset it away.
        h.process(2.0, 0.5);
        h.reset();
        // Post-reset, y_prev = 0.0 — a small input lands inside that band
        // and holds at 0.0, not at wherever the pre-reset rail was.
        let out = h.process(0.05, 0.5);
        assert!(
            (out - 0.0).abs() < 1.0e-6,
            "got {out}, reset did not clear state"
        );
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let mut h = HysteresisCell::new();
        for i in 0..2048 {
            let x = (i as f32 * 0.037).sin() * 1.5;
            let y = h.process(x, 0.7);
            assert!(y.is_finite(), "non-finite at {i}: {y}");
            assert!(y.abs() < 2.0, "implausibly large at {i}: {y}");
        }
    }

    #[test]
    fn amount_outside_0_1_is_clamped() {
        // amount > 1.0 must not widen the loop past MAX_THRESHOLD, and a
        // negative amount must behave like 0.0 (exact passthrough), not
        // invert the loop or panic.
        let mut over = HysteresisCell::new();
        let mut at_max = HysteresisCell::new();
        for &x in &[-2.0_f32, -0.5, 0.0, 0.5, 2.0] {
            assert_eq!(over.process(x, 5.0), at_max.process(x, 1.0));
        }

        let mut negative = HysteresisCell::new();
        for &x in &[-1.0_f32, -0.3, 0.0, 0.4, 1.0] {
            assert_eq!(negative.process(x, -3.0), x);
        }
    }

    /// The defining property of hysteresis: the same input value `x = 1.0`
    /// produces different output depending on approach direction — `x + r`
    /// when arrived at by descending from above, `x - r` when arrived at by
    /// ascending from below. A memoryless nonlinearity would give the same
    /// value either way; the play operator's two rails differ by exactly
    /// `2r`.
    #[test]
    fn approach_direction_determines_which_rail_is_hit() {
        let amount = 0.6_f32;
        let r = MAX_THRESHOLD * amount;

        let mut descending = HysteresisCell::new();
        descending.process(3.0, amount); // pin to the upper rail
        let from_above = descending.process(1.0, amount);
        assert!(
            (from_above - (1.0 + r)).abs() < 1.0e-6,
            "got {from_above}, want {}",
            1.0 + r
        );

        let mut ascending = HysteresisCell::new();
        ascending.process(-3.0, amount); // pin to the lower rail
        let from_below = ascending.process(1.0, amount);
        assert!(
            (from_below - (1.0 - r)).abs() < 1.0e-6,
            "got {from_below}, want {}",
            1.0 - r
        );

        assert!(
            (from_above - from_below - 2.0 * r).abs() < 1.0e-6,
            "rails should differ by exactly 2r={}: from_above={from_above}, from_below={from_below}",
            2.0 * r
        );
    }
}
