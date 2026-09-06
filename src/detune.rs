//! Per-instance stereo micro-detuning (TMT-style, #17): every stereo module
//! that shares identical L/R filter coefficients gets a tiny, fixed,
//! per-channel frequency deviation instead, so two plugin instances on the
//! same bus — and the two channels within one instance — don't share the
//! machine-precise center image real analog channel strips never have.
//! See `docs/adr/0013-stereo-micro-detuning.md`.

/// ±0.3% — Brainworx's published bx_console figure. Hard-coded per the
/// issue #17 sign-off (2026-09-06): a "how much randomness" knob creates
/// choice paralysis without obvious benefit, so this is not exposed as an
/// automatable parameter.
pub(crate) const TMT_MAX_DEVIATION: f32 = 0.003;

/// No deviation — both channels at nominal frequency. Used by tests that
/// null a module's output against an exact analytic reference.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const IDENTITY_DETUNE: [f32; 2] = [1.0, 1.0];

/// Minimal splitmix64 PRNG. Not cryptographic — used only to pick each
/// module instance's fixed micro-detune amount, never anything signal- or
/// security-sensitive. `next_u64` is pure integer arithmetic (allocation-
/// free, syscall-free), so it's safe to call from the audio thread, unlike
/// an OS entropy source — that's what lets `reset()` re-seed in place.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DetuneRng(u64);

impl DetuneRng {
    /// Seed from a nondeterministic source. Call only off the audio thread
    /// (plugin/module construction) — this reads the system clock.
    pub(crate) fn seed_from_entropy() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        // Mix in a stack address so instances constructed within the same
        // clock tick (a host spinning up several plugins back-to-back)
        // still diverge.
        let addr = &nanos as *const u64 as u64;
        Self(nanos ^ addr.rotate_left(17))
    }

    /// Fixed, non-random seed — deterministic tests only.
    #[cfg(test)]
    pub(crate) fn from_seed(seed: u64) -> Self {
        Self(seed)
    }

    /// Advance and return the next 64-bit output (splitmix64 step).
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next value, uniform in `[-1.0, 1.0]`.
    fn next_bipolar_unit(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32; // top 24 bits → clean f32 mantissa
        (bits as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
    }
}

/// Draw a fresh pair of independent per-channel frequency multipliers, each
/// in `[1 - TMT_MAX_DEVIATION, 1 + TMT_MAX_DEVIATION]`. Multiply a filter's
/// nominal corner frequency by `detune[ch]` before building coefficients.
pub(crate) fn micro_detune_pair(rng: &mut DetuneRng) -> [f32; 2] {
    [
        1.0 + rng.next_bipolar_unit() * TMT_MAX_DEVIATION,
        1.0 + rng.next_bipolar_unit() * TMT_MAX_DEVIATION,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_detune_pair_stays_within_bounds_across_many_seeds() {
        for seed in 0..10_000u64 {
            let mut rng = DetuneRng::from_seed(seed);
            let pair = micro_detune_pair(&mut rng);
            for (ch, &m) in pair.iter().enumerate() {
                assert!(
                    (1.0 - TMT_MAX_DEVIATION..=1.0 + TMT_MAX_DEVIATION).contains(&m),
                    "seed {seed} ch {ch}: multiplier {m} outside ±{TMT_MAX_DEVIATION} band"
                );
            }
        }
    }

    #[test]
    fn micro_detune_pair_is_deterministic_for_a_fixed_seed() {
        let mut a = DetuneRng::from_seed(12345);
        let mut b = DetuneRng::from_seed(12345);
        assert_eq!(micro_detune_pair(&mut a), micro_detune_pair(&mut b));
    }

    /// The whole point of #17: L and R must not land on the same multiplier.
    /// A within-band collision is possible in principle but the probability
    /// under a 24-bit mantissa is astronomically small; scanning many seeds
    /// and requiring the large majority to differ rules out a systematic bug
    /// (e.g. both channels accidentally drawing from the same rng state)
    /// without the test itself being flaky.
    #[test]
    fn micro_detune_pair_channels_usually_differ() {
        let mismatches = (0..1000u64)
            .filter(|&seed| {
                let mut rng = DetuneRng::from_seed(seed);
                let [l, r] = micro_detune_pair(&mut rng);
                l != r
            })
            .count();
        assert!(
            mismatches > 990,
            "expected L/R multipliers to differ almost always, got {mismatches}/1000"
        );
    }

    /// "Re-seeds correctly on reset()" (#17 DoD): advancing the same `DetuneRng`
    /// twice (simulating construction, then a later `reset()`) must produce a
    /// different pair the second time, not silently repeat the first.
    #[test]
    fn advancing_rng_after_first_draw_yields_a_different_pair() {
        let mut rng = DetuneRng::from_seed(777);
        let first = micro_detune_pair(&mut rng);
        let second = micro_detune_pair(&mut rng);
        assert_ne!(
            first, second,
            "reset()-style re-seed produced an identical pair"
        );
    }

    #[test]
    fn seed_from_entropy_produces_a_usable_pair() {
        // Not asserting anything time-based (that would be flaky) — just
        // that the entropy path wires up to a valid, in-band pair.
        let mut rng = DetuneRng::seed_from_entropy();
        let pair = micro_detune_pair(&mut rng);
        for &m in &pair {
            assert!((1.0 - TMT_MAX_DEVIATION..=1.0 + TMT_MAX_DEVIATION).contains(&m));
        }
    }
}
