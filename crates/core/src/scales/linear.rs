//! Linear scale: `f(x) = range.0 + t * (range.1 - range.0)` where `t = (x - d0) / (d1 - d0)`.

use crate::scale::{Scale, ScaleUniforms, Tick};

use super::util::{format_tick, hash_f32, hash_f64, hash_u32, nice_step};

/// A linear domain-to-range projection with optional clamping.
#[derive(Debug, Clone)]
pub struct LinearScale {
    /// Domain `(lo, hi)` — high-precision so time-like values stay accurate.
    pub domain: (f64, f64),
    /// Range `(lo, hi)` in the units a coord system consumes (typically pixels).
    pub range: (f32, f32),
    /// When true, inputs outside the domain are clamped to it.
    pub clamp: bool,
}

impl LinearScale {
    /// Build a new linear scale.
    #[must_use]
    pub const fn new(domain: (f64, f64), range: (f32, f32)) -> Self {
        Self {
            domain,
            range,
            clamp: false,
        }
    }

    /// Enable clamping.
    #[must_use]
    pub const fn clamped(mut self) -> Self {
        self.clamp = true;
        self
    }
}

impl Scale for LinearScale {
    fn project(&self, value: f64) -> f32 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let t = if (d1 - d0).abs() < f64::EPSILON {
            0.0
        } else {
            (value - d0) / (d1 - d0)
        };
        let t = if self.clamp { t.clamp(0.0, 1.0) } else { t };
        r0 + (t as f32) * (r1 - r0)
    }

    fn unproject(&self, position: f32) -> Option<f64> {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        if (r1 - r0).abs() < f32::EPSILON {
            return None;
        }
        let t = f64::from((position - r0) / (r1 - r0));
        Some(d0 + t * (d1 - d0))
    }

    fn ticks(&self, count: usize) -> Vec<Tick> {
        let (d0, d1) = self.domain;
        if (d1 - d0).abs() < f64::EPSILON || count == 0 {
            return Vec::new();
        }
        let (lo, hi) = if d0 <= d1 { (d0, d1) } else { (d1, d0) };
        let step = nice_step(hi - lo, count);
        let start = (lo / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut v = start;
        // `1e-9` guard tolerates accumulated f64 drift at the upper bound.
        while v <= hi + step * 1e-9 {
            ticks.push(Tick {
                value: v,
                position: self.project(v),
                label: format_tick(v, step),
            });
            v += step;
        }
        ticks
    }

    fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        hash_u32(&mut h, 1); // kind discriminator
        hash_f64(&mut h, self.domain.0);
        hash_f64(&mut h, self.domain.1);
        hash_f32(&mut h, self.range.0);
        hash_f32(&mut h, self.range.1);
        hash_u32(&mut h, u32::from(self.clamp));
        h
    }

    fn gpu_uniforms(&self) -> ScaleUniforms {
        ScaleUniforms {
            kind: 0,
            flags: if self.clamp { 1 } else { 0 },
            domain_lo: self.domain.0 as f32,
            domain_hi: self.domain.1 as f32,
            range_lo: self.range.0,
            range_hi: self.range.1,
            domain_lo_hi: (self.domain.0 - (self.domain.0 as f32) as f64) as f32,
            domain_hi_hi: (self.domain.1 - (self.domain.1 as f32) as f64) as f32,
            aux0: 0.0,
            aux1: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_midpoint() {
        let s = LinearScale::new((0.0, 10.0), (0.0, 100.0));
        assert!((s.project(5.0) - 50.0).abs() < 1e-5);
    }

    #[test]
    fn clamps_when_enabled() {
        let s = LinearScale::new((0.0, 1.0), (0.0, 100.0)).clamped();
        assert!((s.project(2.0) - 100.0).abs() < 1e-5);
        assert!((s.project(-1.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn unprojects_roundtrip() {
        let s = LinearScale::new((5.0, 25.0), (10.0, 410.0));
        let p = s.project(17.5);
        let v = s.unproject(p).unwrap();
        assert!((v - 17.5).abs() < 1e-5);
    }

    #[test]
    fn fingerprint_changes_with_domain() {
        let a = LinearScale::new((0.0, 1.0), (0.0, 100.0));
        let b = LinearScale::new((0.0, 2.0), (0.0, 100.0));
        assert_ne!(a.fingerprint(), b.fingerprint());
    }
}
