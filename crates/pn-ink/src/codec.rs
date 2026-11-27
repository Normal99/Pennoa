//! Compact, versioned encoding of a stroke's samples (see `docs/FORMAT.md`).
//!
//! Layout (all integers are LEB128 varints, signed ones zig-zag encoded):
//!
//! ```text
//! u8      version (= 1)
//! u8      flags   (bit 0: tilt present)
//! varint  sample count
//! sample 0 absolute, then for every following sample the delta to the previous:
//!   x, y        signed, units of 0.01 mm
//!   pressure    signed, units of 1/1023
//!   tilt_x/y    signed, units of 0.5 degree      (only if flag bit 0)
//!   t           unsigned, microseconds (always increasing or equal)
//! ```
//!
//! Quantisation: position 0.01 mm, pressure 1/1023, tilt 0.5°. Typical pen input
//! (≈ 240 Hz) encodes to about 5–7 bytes per sample.

use crate::Sample;

pub const VERSION: u8 = 1;
const FLAG_TILT: u8 = 1;

const POS_SCALE: f32 = 100.0; // 0.01 mm
const PRESSURE_SCALE: f32 = 1023.0;
const TILT_SCALE: f32 = 2.0; // 0.5 degree

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEnd,
    UnsupportedVersion(u8),
    VarintOverflow,
    TooManySamples(u64),
    TrailingBytes(usize),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEnd => write!(f, "unexpected end of point data"),
            DecodeError::UnsupportedVersion(v) => {
                write!(f, "unsupported point encoding version {v}")
            }
            DecodeError::VarintOverflow => write!(f, "varint overflow"),
            DecodeError::TooManySamples(n) => write!(f, "implausible sample count {n}"),
            DecodeError::TrailingBytes(n) => write!(f, "{n} trailing bytes after point data"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Hard cap so a corrupt or hostile file can't make us allocate gigabytes.
pub const MAX_SAMPLES: u64 = 1 << 20;

#[derive(Clone, Copy, Default)]
struct Q {
    x: i64,
    y: i64,
    p: i64,
    tx: i64,
    ty: i64,
    t: u64,
}

fn quantize(s: &Sample) -> Q {
    Q {
        x: (s.x * POS_SCALE).round() as i64,
        y: (s.y * POS_SCALE).round() as i64,
        p: (s.pressure.clamp(0.0, 1.0) * PRESSURE_SCALE).round() as i64,
        tx: (s.tilt_x.clamp(-90.0, 90.0) * TILT_SCALE).round() as i64,
        ty: (s.tilt_y.clamp(-90.0, 90.0) * TILT_SCALE).round() as i64,
        t: s.t_us,
    }
}

fn dequantize(q: &Q) -> Sample {
    Sample {
        x: q.x as f32 / POS_SCALE,
        y: q.y as f32 / POS_SCALE,
        pressure: q.p as f32 / PRESSURE_SCALE,
        tilt_x: q.tx as f32 / TILT_SCALE,
        tilt_y: q.ty as f32 / TILT_SCALE,
        t_us: q.t,
    }
}

fn put_u(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_i(out: &mut Vec<u8>, v: i64) {
    put_u(out, ((v << 1) ^ (v >> 63)) as u64);
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, DecodeError> {
        let b = *self.buf.get(self.pos).ok_or(DecodeError::UnexpectedEnd)?;
        self.pos += 1;
        Ok(b)
    }

    fn u(&mut self) -> Result<u64, DecodeError> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(DecodeError::VarintOverflow)
    }

    fn i(&mut self) -> Result<i64, DecodeError> {
        let u = self.u()?;
        Ok(((u >> 1) as i64) ^ -((u & 1) as i64))
    }
}

/// Encode samples. Timestamps must be non-decreasing (pen input always is);
/// a decreasing timestamp is clamped to the previous one.
pub fn encode(samples: &[Sample]) -> Vec<u8> {
    let has_tilt = samples.iter().any(|s| s.tilt_x != 0.0 || s.tilt_y != 0.0);
    let mut out = Vec::with_capacity(3 + samples.len() * 7);
    out.push(VERSION);
    out.push(if has_tilt { FLAG_TILT } else { 0 });
    put_u(&mut out, samples.len() as u64);

    let mut prev = Q::default();
    for (i, s) in samples.iter().enumerate() {
        let mut q = quantize(s);
        if i > 0 && q.t < prev.t {
            q.t = prev.t;
        }
        put_i(&mut out, q.x - prev.x);
        put_i(&mut out, q.y - prev.y);
        put_i(&mut out, q.p - prev.p);
        if has_tilt {
            put_i(&mut out, q.tx - prev.tx);
            put_i(&mut out, q.ty - prev.ty);
        }
        put_u(&mut out, q.t - prev.t);
        prev = q;
    }
    out
}

/// Decode samples produced by [`encode`].
pub fn decode(buf: &[u8]) -> Result<Vec<Sample>, DecodeError> {
    let mut r = Reader { buf, pos: 0 };
    let version = r.byte()?;
    if version != VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let has_tilt = r.byte()? & FLAG_TILT != 0;
    let n = r.u()?;
    if n > MAX_SAMPLES {
        return Err(DecodeError::TooManySamples(n));
    }
    // Every sample needs at least 4 bytes, so a short buffer can't claim many.
    let min_bytes = if has_tilt { 6 } else { 4 };
    if n.saturating_mul(min_bytes) > (buf.len() - r.pos) as u64 {
        return Err(DecodeError::UnexpectedEnd);
    }

    let mut out = Vec::with_capacity(n as usize);
    let mut q = Q::default();
    for _ in 0..n {
        q.x = q.x.wrapping_add(r.i()?);
        q.y = q.y.wrapping_add(r.i()?);
        q.p = q.p.wrapping_add(r.i()?);
        if has_tilt {
            q.tx = q.tx.wrapping_add(r.i()?);
            q.ty = q.ty.wrapping_add(r.i()?);
        }
        q.t = q.t.wrapping_add(r.u()?);
        out.push(dequantize(&q));
    }
    if r.pos != buf.len() {
        return Err(DecodeError::TrailingBytes(buf.len() - r.pos));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn handwriting_like(n: usize) -> Vec<Sample> {
        // A wobbly stroke sampled at 240 Hz.
        (0..n)
            .map(|i| {
                let t = i as f32 / 240.0;
                Sample {
                    x: 20.0 + 30.0 * t + (t * 40.0).sin() * 1.5,
                    y: 40.0 + (t * 25.0).cos() * 2.0,
                    pressure: 0.5 + 0.3 * (t * 7.0).sin(),
                    tilt_x: 30.0 + 2.0 * t,
                    tilt_y: -10.0,
                    t_us: (i as u64) * 4_167,
                }
            })
            .collect()
    }

    #[test]
    fn empty_round_trip() {
        assert_eq!(decode(&encode(&[])).unwrap(), vec![]);
    }

    #[test]
    fn typical_size_per_sample() {
        let samples = handwriting_like(480);
        let bytes = encode(&samples);
        let per = bytes.len() as f32 / samples.len() as f32;
        eprintln!("point encoding: {per:.2} bytes/sample");
        assert!(per < 8.0, "{per:.2} bytes/sample");
    }

    #[test]
    fn rejects_hostile_input() {
        assert_eq!(decode(&[]), Err(DecodeError::UnexpectedEnd));
        assert_eq!(decode(&[9, 0, 0]), Err(DecodeError::UnsupportedVersion(9)));
        // claims a million samples but carries none
        let mut b = vec![VERSION, 0];
        put_u(&mut b, 1_000_000);
        assert_eq!(decode(&b), Err(DecodeError::UnexpectedEnd));
        // claims too many samples outright
        let mut b = vec![VERSION, 0];
        put_u(&mut b, MAX_SAMPLES + 1);
        assert_eq!(
            decode(&b),
            Err(DecodeError::TooManySamples(MAX_SAMPLES + 1))
        );
        // trailing garbage
        let mut b = encode(&handwriting_like(3));
        b.push(0);
        assert_eq!(decode(&b), Err(DecodeError::TrailingBytes(1)));
    }

    fn sample_strategy() -> impl Strategy<Value = Sample> {
        (
            -5_000.0f32..5_000.0,
            -5_000.0f32..5_000.0,
            0.0f32..=1.0,
            -90.0f32..=90.0,
            -90.0f32..=90.0,
            0u64..20_000,
        )
            .prop_map(|(x, y, pressure, tilt_x, tilt_y, dt)| Sample {
                x,
                y,
                pressure,
                tilt_x,
                tilt_y,
                t_us: dt,
            })
    }

    proptest! {
        #[test]
        fn round_trip_within_quantisation(raw in prop::collection::vec(sample_strategy(), 0..300)) {
            // make timestamps cumulative, as pen input is
            let mut t = 0;
            let samples: Vec<Sample> = raw.into_iter().map(|mut s| { t += s.t_us; s.t_us = t; s }).collect();
            let decoded = decode(&encode(&samples)).unwrap();
            prop_assert_eq!(decoded.len(), samples.len());
            for (a, b) in samples.iter().zip(&decoded) {
                prop_assert!((a.x - b.x).abs() <= 0.0051 + a.x.abs() * 1e-6);
                prop_assert!((a.y - b.y).abs() <= 0.0051 + a.y.abs() * 1e-6);
                prop_assert!((a.pressure - b.pressure).abs() <= 0.5 / 1023.0 + 1e-6);
                prop_assert!((a.tilt_x - b.tilt_x).abs() <= 0.2501);
                prop_assert!((a.tilt_y - b.tilt_y).abs() <= 0.2501);
                prop_assert_eq!(a.t_us, b.t_us);
            }
        }

        #[test]
        fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
            let _ = decode(&bytes);
        }
    }
}
