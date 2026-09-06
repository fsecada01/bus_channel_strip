use nice_plug::prelude::*;

#[derive(Params)]
pub struct SheenParams {
    // ── Sheen Module Parameters ──────────────────────────────────────────
    // Pinned master-end "polish coat". Always default-ON; the brass plate in
    // the chassis header opens the back view that exposes these sliders.
    // Factory values are research-grounded (see ADR-0006).
    #[cfg(feature = "sheen")]
    #[id = "sheen_bypass"]
    pub sheen_bypass: BoolParam,

    #[cfg(feature = "sheen")]
    #[id = "sheen_body_db"]
    pub sheen_body_db: FloatParam,
    #[cfg(feature = "sheen")]
    #[id = "sheen_body_bypass"]
    pub sheen_body_bypass: BoolParam,

    #[cfg(feature = "sheen")]
    #[id = "sheen_presence_db"]
    pub sheen_presence_db: FloatParam,
    #[cfg(feature = "sheen")]
    #[id = "sheen_presence_bypass"]
    pub sheen_presence_bypass: BoolParam,

    #[cfg(feature = "sheen")]
    #[id = "sheen_air_db"]
    pub sheen_air_db: FloatParam,
    #[cfg(feature = "sheen")]
    #[id = "sheen_air_bypass"]
    pub sheen_air_bypass: BoolParam,

    #[cfg(feature = "sheen")]
    #[id = "sheen_warmth"]
    pub sheen_warmth: FloatParam,
    #[cfg(feature = "sheen")]
    #[id = "sheen_warmth_bypass"]
    pub sheen_warmth_bypass: BoolParam,
    /// #16: opt-in "tape" hysteresis sub-mode for WARMTH.
    #[cfg(feature = "sheen")]
    #[id = "sheen_warmth_tape_mode"]
    pub sheen_warmth_tape_mode: BoolParam,

    #[cfg(feature = "sheen")]
    #[id = "sheen_width"]
    pub sheen_width: FloatParam,
    #[cfg(feature = "sheen")]
    #[id = "sheen_width_bypass"]
    pub sheen_width_bypass: BoolParam,
}

impl Default for SheenParams {
    fn default() -> Self {
        Self {
            // Default ON (sheen_bypass = false). Per-stage values follow
            // the polish-plugin consensus synthesis (see ADR-0006).
            #[cfg(feature = "sheen")]
            sheen_bypass: BoolParam::new("Sheen Bypass", false),

            #[cfg(feature = "sheen")]
            sheen_body_db: FloatParam::new(
                "Sheen Body",
                1.0,
                FloatRange::Linear {
                    min: -2.0,
                    max: 3.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit(" dB")
            .with_step_size(0.1),
            #[cfg(feature = "sheen")]
            sheen_body_bypass: BoolParam::new("Sheen Body Bypass", false),

            #[cfg(feature = "sheen")]
            sheen_presence_db: FloatParam::new(
                "Sheen Presence",
                0.0,
                FloatRange::Linear {
                    min: -3.0,
                    max: 3.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit(" dB")
            .with_step_size(0.1),
            #[cfg(feature = "sheen")]
            sheen_presence_bypass: BoolParam::new("Sheen Presence Bypass", false),

            #[cfg(feature = "sheen")]
            sheen_air_db: FloatParam::new(
                "Sheen Air",
                1.8,
                FloatRange::Linear { min: 0.0, max: 4.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit(" dB")
            .with_step_size(0.1),
            #[cfg(feature = "sheen")]
            sheen_air_bypass: BoolParam::new("Sheen Air Bypass", false),

            #[cfg(feature = "sheen")]
            sheen_warmth: FloatParam::new(
                "Sheen Warmth",
                0.20,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit("")
            .with_step_size(0.01),
            #[cfg(feature = "sheen")]
            sheen_warmth_bypass: BoolParam::new("Sheen Warmth Bypass", false),
            // #16: opt-in "tape" sub-mode — off by default, no migration note needed.
            #[cfg(feature = "sheen")]
            sheen_warmth_tape_mode: BoolParam::new("Sheen Warmth Tape Mode", false),

            #[cfg(feature = "sheen")]
            sheen_width: FloatParam::new(
                "Sheen Width",
                0.50,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0))
            .with_unit("")
            .with_step_size(0.01),
            #[cfg(feature = "sheen")]
            sheen_width_bypass: BoolParam::new("Sheen Width Bypass", false),
        }
    }
}
