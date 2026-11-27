//! Pennoa's ink model.
//!
//! A [`Stroke`] is a sequence of raw pen [`Sample`]s plus tool, colour and width.
//! Raw samples are kept forever (smoothing happens at render time), so the
//! on-disk encoding in [`codec`] must be compact and lossless up to its
//! documented quantisation.

pub mod codec;

use kurbo::Rect;

/// One pen sample in page coordinates.
///
/// * `x`, `y`: millimetres from the page origin (top-left). Infinite canvases use
///   the same unit; tiles keep their own origin, so values stay small.
/// * `pressure`: normalised 0.0..=1.0 (devices without pressure report 1.0).
/// * `tilt_x`, `tilt_y`: degrees, -90.0..=90.0 (0.0 when unknown).
/// * `t_us`: microseconds since the start of the stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub t_us: u64,
}

impl Sample {
    pub fn new(x: f32, y: f32, pressure: f32, t_us: u64) -> Self {
        Self {
            x,
            y,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            t_us,
        }
    }
}

/// Drawing tool that produced a stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tool {
    Pen = 0,
    Pencil = 1,
    Highlighter = 2,
    Marker = 3,
}

impl Tool {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Tool::Pen,
            1 => Tool::Pencil,
            2 => Tool::Highlighter,
            3 => Tool::Marker,
            _ => return None,
        })
    }
}

/// A finished stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Globally unique id (ULID bits).
    pub id: u128,
    pub tool: Tool,
    /// sRGB colour, `0xRRGGBBAA`.
    pub color: u32,
    /// Nominal width in millimetres at pressure 1.0.
    pub width: f32,
    /// Wall-clock start of the stroke, microseconds since the Unix epoch.
    /// Lets recordings (lecture audio) map a stroke to a moment in time.
    pub started_at_us: u64,
    pub samples: Vec<Sample>,
}

impl Stroke {
    /// Axis-aligned bounds of the sample centres, grown by half the width.
    pub fn bounds(&self) -> Option<Rect> {
        let first = self.samples.first()?;
        let mut r = Rect::new(
            first.x as f64,
            first.y as f64,
            first.x as f64,
            first.y as f64,
        );
        for s in &self.samples[1..] {
            r = r.union_pt((s.x as f64, s.y as f64));
        }
        Some(r.inflate(self.width as f64 / 2.0, self.width as f64 / 2.0))
    }

    /// Duration from first to last sample.
    pub fn duration_us(&self) -> u64 {
        self.samples.last().map_or(0, |s| s.t_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_include_half_width() {
        let s = Stroke {
            id: 1,
            tool: Tool::Pen,
            color: 0x1D2B5CFF,
            width: 2.0,
            started_at_us: 0,
            samples: vec![
                Sample::new(10.0, 10.0, 1.0, 0),
                Sample::new(20.0, 15.0, 1.0, 8_000),
            ],
        };
        let b = s.bounds().unwrap();
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (9.0, 9.0, 21.0, 16.0));
        assert_eq!(s.duration_us(), 8_000);
    }
}
