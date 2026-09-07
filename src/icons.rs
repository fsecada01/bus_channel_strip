// src/icons.rs
// Small vector icon set rendered through Skia paths, replacing the previous
// Unicode-glyph icons (roadmap v2.0 §4.4 — see docs/V2_ROADMAP.md).

use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconKind {
    Close,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    /// Counterclockwise restore/reset arrow — replaces the previous `↺` glyph.
    Restore,
}

/// Fixed-color vector icon drawn directly via Skia paths. Follows
/// `SpectrumCanvas`'s lock-free-atomic draw() pattern in `editor.rs`, minus
/// the reactive data source — icons here are static per call site, and a
/// state-driven icon (e.g. the DynEQ expand chevron) is swapped by rebuilding
/// the `Icon` inside a `Binding`, not by mutating this view in place.
pub struct Icon {
    kind: IconKind,
    color: vg::Color,
}

impl Icon {
    pub fn new(cx: &mut Context, kind: IconKind, color: vg::Color) -> Handle<'_, Self> {
        Self { kind, color }.build(cx, |_cx| {})
    }
}

impl View for Icon {
    fn element(&self) -> Option<&'static str> {
        Some("icon")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h < 1.0 {
            return;
        }

        let mut paint = vg::Paint::default();
        paint.set_color(self.color);
        paint.set_style(vg::PaintStyle::Stroke);
        paint.set_stroke_width((bounds.w.min(bounds.h) * 0.14).max(1.0));
        paint.set_anti_alias(true);

        // Icons are drawn inside a centered square inset from the widget
        // bounds so the stroke never clips against the container edge.
        let size = bounds.w.min(bounds.h);
        let half = size * 0.5 - size * 0.22;
        let center_x = bounds.x + bounds.w * 0.5;
        let center_y = bounds.y + bounds.h * 0.5;

        let mut path = vg::PathBuilder::new();
        match self.kind {
            IconKind::Close => {
                path.move_to((center_x - half, center_y - half));
                path.line_to((center_x + half, center_y + half));
                path.move_to((center_x + half, center_y - half));
                path.line_to((center_x - half, center_y + half));
            }
            IconKind::ChevronLeft => {
                path.move_to((center_x + half * 0.6, center_y - half));
                path.line_to((center_x - half * 0.6, center_y));
                path.line_to((center_x + half * 0.6, center_y + half));
            }
            IconKind::ChevronRight => {
                path.move_to((center_x - half * 0.6, center_y - half));
                path.line_to((center_x + half * 0.6, center_y));
                path.line_to((center_x - half * 0.6, center_y + half));
            }
            IconKind::ChevronDown => {
                path.move_to((center_x - half, center_y - half * 0.6));
                path.line_to((center_x, center_y + half * 0.6));
                path.line_to((center_x + half, center_y - half * 0.6));
            }
            IconKind::Restore => {
                // ~285° arc (a 75° gap near the 3-o'clock position) approximated
                // as a polyline, plus a small arrowhead at the trailing end.
                const START_DEG: f32 = 20.0;
                const END_DEG: f32 = 305.0;
                const SEGMENTS: u32 = 20;
                let radius = half * 0.85;
                for i in 0..=SEGMENTS {
                    let t = i as f32 / SEGMENTS as f32;
                    let rad = (START_DEG + (END_DEG - START_DEG) * t).to_radians();
                    let point = (center_x + radius * rad.cos(), center_y + radius * rad.sin());
                    if i == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }

                let end_rad = END_DEG.to_radians();
                let tip = (
                    center_x + radius * end_rad.cos(),
                    center_y + radius * end_rad.sin(),
                );
                // Unit tangent at the arc's end, direction of travel.
                let (tx, ty) = (-end_rad.sin(), end_rad.cos());
                let arrow_len = half * 0.5;
                for wing_angle in [2.4_f32, -2.4_f32] {
                    let (s, c) = wing_angle.sin_cos();
                    let wing_dx = tx * c - ty * s;
                    let wing_dy = tx * s + ty * c;
                    path.move_to(tip);
                    path.line_to((tip.0 + wing_dx * arrow_len, tip.1 + wing_dy * arrow_len));
                }
            }
        }
        canvas.draw_path(&path.detach(), &paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_kinds_are_distinct() {
        let kinds = [
            IconKind::Close,
            IconKind::ChevronLeft,
            IconKind::ChevronRight,
            IconKind::ChevronDown,
        ];
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }
}
