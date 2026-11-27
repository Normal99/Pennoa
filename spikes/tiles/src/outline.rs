//! Variable-width stroke outline: the shape Pennoa fills for every committed stroke.

use pn_ink::Stroke;
use vello::kurbo::{BezPath, Circle, Point, Rect, Shape, Vec2};

/// Returns the filled outline (in page millimetres) and its bounds.
///
/// Width follows pressure (`0.35 + 0.65 * pressure` of the nominal width). Each
/// segment is a quad between offset points; round caps are added as circles.
/// Production code will use a proper stroke modeller (ink-stroke-modeler-rs)
/// and curve fitting; this is enough to load the renderer realistically.
pub fn stroke_outline(stroke: &Stroke) -> (BezPath, Rect) {
    let pts: Vec<(Point, f64)> = stroke
        .samples
        .iter()
        .map(|s| {
            (
                Point::new(s.x as f64, s.y as f64),
                stroke.width as f64 * (0.35 + 0.65 * s.pressure as f64) / 2.0,
            )
        })
        .collect();
    let mut path = BezPath::new();
    if pts.is_empty() {
        return (path, Rect::ZERO);
    }

    if pts.len() > 1 {
        let mut left = Vec::with_capacity(pts.len());
        let mut right = Vec::with_capacity(pts.len());
        for i in 0..pts.len() {
            let prev = pts[i.saturating_sub(1)].0;
            let next = pts[(i + 1).min(pts.len() - 1)].0;
            let dir = next - prev;
            let n = if dir.hypot() > 1e-9 {
                Vec2::new(-dir.y, dir.x) / dir.hypot()
            } else {
                Vec2::new(0.0, 1.0)
            };
            let (p, r) = pts[i];
            left.push(p + n * r);
            right.push(p - n * r);
        }
        path.move_to(left[0]);
        for p in &left[1..] {
            path.line_to(*p);
        }
        for p in right.iter().rev() {
            path.line_to(*p);
        }
        path.close_path();
    }
    for &(p, r) in [pts.first(), pts.last()].into_iter().flatten() {
        path.extend(Circle::new(p, r).path_elements(0.01));
    }
    let bounds = path.bounding_box();
    (path, bounds)
}
