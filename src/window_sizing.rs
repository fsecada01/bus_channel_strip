//! Window sizing math for the resizable GUI (issue #20).
//!
//! Real continuous host-driven drag-resize is not available: `nice-plug`
//! only gained a `ResizeHint`/`EditorHandle::set_size` mechanism for that in
//! `nice-plug-core` 0.2+, but `vizia_plug`'s editor implementation still
//! targets `nice-plug-core ^0.1.5` and has not been updated to it (verified
//! against both projects' current sources). So resizing here stays
//! discrete: a small set of zoom levels, each mapped to a real host-window
//! size via `GuiContext::request_resize()`, rather than a CSS-only visual
//! scale as before.
//!
//! Per the issue's sign-off ("maintain aspect ratio... with letterboxing"),
//! every produced size preserves the base window's aspect ratio exactly.
//! The achievable range is the intersection of both axes' bounds rather
//! than the literal corners, since scaling [`BASE_WINDOW_WIDTH`] /
//! [`BASE_WINDOW_HEIGHT`] uniformly to hit 1100 width would undershoot the
//! 750 minimum height (and symmetrically at the top end) — this keeps every
//! level strictly inside the stated 1100x750..=2400x1400 envelope without
//! needing to composite actual letterbox bars.

/// The GUI's original fixed size, and the size used at [`DEFAULT_ZOOM_LEVEL`].
pub const BASE_WINDOW_WIDTH: u32 = 1300;
pub const BASE_WINDOW_HEIGHT: u32 = 860;

/// Acceptance-criteria bounds from issue #20.
pub const MIN_WINDOW_WIDTH: u32 = 1100;
pub const MIN_WINDOW_HEIGHT: u32 = 750;
pub const MAX_WINDOW_WIDTH: u32 = 2400;
pub const MAX_WINDOW_HEIGHT: u32 = 1400;

/// Discrete zoom levels (percent of the base window size) the GUI's zoom
/// buttons cycle through.
pub const ZOOM_LEVELS: [u8; 5] = [75, 100, 125, 150, 200];

pub const DEFAULT_ZOOM_LEVEL: u8 = 100;

/// The smallest uniform scale factor that keeps *both* axes at or above
/// their minimum bound.
fn min_scale() -> f64 {
    (MIN_WINDOW_WIDTH as f64 / BASE_WINDOW_WIDTH as f64)
        .max(MIN_WINDOW_HEIGHT as f64 / BASE_WINDOW_HEIGHT as f64)
}

/// The largest uniform scale factor that keeps *both* axes at or below
/// their maximum bound.
fn max_scale() -> f64 {
    (MAX_WINDOW_WIDTH as f64 / BASE_WINDOW_WIDTH as f64)
        .min(MAX_WINDOW_HEIGHT as f64 / BASE_WINDOW_HEIGHT as f64)
}

/// Snap a requested zoom level to a supported [`ZOOM_LEVELS`] entry,
/// falling back to [`DEFAULT_ZOOM_LEVEL`] for anything unrecognized (e.g. a
/// stray value from a foreign/corrupted session).
pub(crate) fn clamp_zoom_level(level: u8) -> u8 {
    if ZOOM_LEVELS.contains(&level) {
        level
    } else {
        DEFAULT_ZOOM_LEVEL
    }
}

/// The uniform scale factor for a zoom level, clamped so the resulting
/// window never violates either axis's bound.
pub(crate) fn scale_factor_for_zoom(level: u8) -> f64 {
    let requested = clamp_zoom_level(level) as f64 / 100.0;
    requested.clamp(min_scale(), max_scale())
}

/// The real host-window size (logical pixels) for a zoom level, preserving
/// the base window's aspect ratio.
pub(crate) fn window_size_for_zoom(level: u8) -> (u32, u32) {
    let scale = scale_factor_for_zoom(level);
    (
        (BASE_WINDOW_WIDTH as f64 * scale).round() as u32,
        (BASE_WINDOW_HEIGHT as f64 * scale).round() as u32,
    )
}

/// Reverse-map a persisted `ViziaState` scale factor back to the nearest
/// [`ZOOM_LEVELS`] entry, used to restore the zoom buttons' selected state
/// when the editor is (re)created (e.g. after a session reload).
pub(crate) fn zoom_level_for_scale_factor(scale_factor: f64) -> u8 {
    ZOOM_LEVELS
        .iter()
        .copied()
        .min_by(|&a, &b| {
            let da = (scale_factor_for_zoom(a) - scale_factor).abs();
            let db = (scale_factor_for_zoom(b) - scale_factor).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(DEFAULT_ZOOM_LEVEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_zoom_level_accepts_known_levels() {
        for &level in &ZOOM_LEVELS {
            assert_eq!(clamp_zoom_level(level), level);
        }
    }

    #[test]
    fn clamp_zoom_level_falls_back_to_default_for_unknown() {
        assert_eq!(clamp_zoom_level(42), DEFAULT_ZOOM_LEVEL);
        assert_eq!(clamp_zoom_level(0), DEFAULT_ZOOM_LEVEL);
        assert_eq!(clamp_zoom_level(255), DEFAULT_ZOOM_LEVEL);
    }

    #[test]
    fn window_size_for_zoom_stays_within_issue_20_bounds() {
        for &level in &ZOOM_LEVELS {
            let (w, h) = window_size_for_zoom(level);
            assert!(
                (MIN_WINDOW_WIDTH..=MAX_WINDOW_WIDTH).contains(&w),
                "width {w} out of bounds for zoom {level}"
            );
            assert!(
                (MIN_WINDOW_HEIGHT..=MAX_WINDOW_HEIGHT).contains(&h),
                "height {h} out of bounds for zoom {level}"
            );
        }
    }

    #[test]
    fn window_size_for_zoom_preserves_base_aspect_ratio() {
        let base_ratio = BASE_WINDOW_WIDTH as f64 / BASE_WINDOW_HEIGHT as f64;
        for &level in &ZOOM_LEVELS {
            let (w, h) = window_size_for_zoom(level);
            let ratio = w as f64 / h as f64;
            assert!(
                (ratio - base_ratio).abs() < 0.01,
                "zoom {level} broke aspect ratio: {ratio} vs {base_ratio}"
            );
        }
    }

    #[test]
    fn default_zoom_matches_base_window_size() {
        assert_eq!(
            window_size_for_zoom(DEFAULT_ZOOM_LEVEL),
            (BASE_WINDOW_WIDTH, BASE_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn extreme_zoom_levels_clamp_into_the_achievable_range() {
        // Raw 75% would put height at 645 (below the 750 minimum) and raw
        // 200% would put height at 1720 (above the 1400 maximum) if scaled
        // without clamping — both must be pulled back onto the opposite
        // axis's bound instead.
        let (_, h75) = window_size_for_zoom(75);
        assert!(h75 >= MIN_WINDOW_HEIGHT);
        let (w200, h200) = window_size_for_zoom(200);
        assert!(w200 <= MAX_WINDOW_WIDTH);
        assert!(h200 <= MAX_WINDOW_HEIGHT);
    }

    #[test]
    fn zoom_level_for_scale_factor_round_trips_exact_matches() {
        for &level in &ZOOM_LEVELS {
            let scale = scale_factor_for_zoom(level);
            assert_eq!(zoom_level_for_scale_factor(scale), level);
        }
    }

    #[test]
    fn zoom_level_for_scale_factor_picks_nearest_for_foreign_values() {
        // A value from a foreign/corrupted session between the 100% and
        // 125% scale factors should snap to whichever is closer.
        let near_100 = scale_factor_for_zoom(100) + 0.01;
        assert_eq!(zoom_level_for_scale_factor(near_100), 100);
    }

    #[test]
    fn zoom_levels_are_monotonically_increasing_in_size() {
        let sizes: Vec<(u32, u32)> = ZOOM_LEVELS
            .iter()
            .map(|&z| window_size_for_zoom(z))
            .collect();
        for pair in sizes.windows(2) {
            assert!(
                pair[1].0 > pair[0].0,
                "widths must strictly increase: {pair:?}"
            );
            assert!(
                pair[1].1 > pair[0].1,
                "heights must strictly increase: {pair:?}"
            );
        }
    }
}
