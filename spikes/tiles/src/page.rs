//! Synthetic handwriting page: ruled A4, words made of loopy strokes sampled at 240 Hz.

use pn_ink::{Sample, Stroke, Tool};
use vello::peniko::Color;

pub const PAGE_W_MM: f64 = 210.0;
pub const PAGE_H_MM: f64 = 297.0;

/// Deterministic pseudo-random numbers (xorshift) so runs are comparable.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub fn synthetic_page(lines: usize) -> Vec<Stroke> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut strokes = Vec::new();
    let line_h = 9.0f32; // ~9 mm ruled lines
    let mut id = 0u128;
    for line in 0..lines {
        let baseline = 20.0 + line as f32 * line_h;
        let mut x = 15.0f32;
        while x < 190.0 {
            // a word: 2–5 strokes, each a run of cursive-like loops
            let parts = 2 + (rng.next() * 4.0) as usize;
            for _ in 0..parts {
                let loops = 1.0 + rng.next() * 3.0;
                let len = 4.0 + rng.next() * 8.0;
                let n = (60.0 + rng.next() * 120.0) as usize;
                let phase = rng.next() * 6.28;
                let height = 2.5 + rng.next() * 2.0;
                let samples = (0..n)
                    .map(|i| {
                        let u = i as f32 / (n - 1) as f32;
                        let a = u * loops * std::f32::consts::TAU + phase;
                        Sample {
                            x: x + u * len + a.cos() * 0.9,
                            y: baseline - height * (0.5 + 0.5 * a.sin()),
                            pressure: (0.45 + 0.4 * (u * 3.1).sin()).clamp(0.05, 1.0),
                            tilt_x: 35.0,
                            tilt_y: -8.0,
                            t_us: i as u64 * 4_167,
                        }
                    })
                    .collect();
                id += 1;
                strokes.push(Stroke {
                    id,
                    tool: Tool::Pen,
                    color: 0x1D2B5CFF,
                    width: 0.5,
                    started_at_us: 0,
                    samples,
                });
                x += len * 0.8;
            }
            x += 4.0 + rng.next() * 3.0; // word gap
        }
    }
    strokes
}

pub fn color(rgba: u32) -> Color {
    let [r, g, b, a] = rgba.to_be_bytes();
    Color::from_rgba8(r, g, b, a)
}
