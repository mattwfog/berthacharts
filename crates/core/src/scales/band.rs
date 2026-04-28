//! Band scale: partitions a range into N equally-spaced bands for categorical axes.
//!
//! Follows d3-scale's band-scale semantics:
//! - `step` = range / (N - padding_inner + 2 × padding_outer)
//! - `bandwidth` = step × (1 - padding_inner)
//! - Band `i` starts at `range.0 + padding_outer × step + i × step`

use std::sync::Arc;

use crate::scale::{Scale, ScaleUniforms, Tick};

use super::util::{hash_f32, hash_u32};

/// A band scale. Values are supplied as integer indices into [`BandScale::domain`].
#[derive(Debug, Clone)]
pub struct BandScale {
    /// Ordered category labels.
    pub domain: Vec<Arc<str>>,
    /// Range `(lo, hi)` — typically plot-area pixel extents on the categorical axis.
    pub range: (f32, f32),
    /// Inner padding as a fraction of `step` in `[0, 1)`. Gap between bands.
    pub padding_inner: f32,
    /// Outer padding as a fraction of `step` in `[0, 1)`. Margin at range edges.
    pub padding_outer: f32,
}

impl BandScale {
    /// Build a band scale.
    #[must_use]
    pub fn new(domain: impl IntoIterator<Item = impl Into<Arc<str>>>, range: (f32, f32)) -> Self {
        Self {
            domain: domain.into_iter().map(Into::into).collect(),
            range,
            padding_inner: 0.1,
            padding_outer: 0.05,
        }
    }

    /// Set inner padding in-place.
    #[must_use]
    pub const fn with_padding_inner(mut self, p: f32) -> Self {
        self.padding_inner = p;
        self
    }

    /// Set outer padding in-place.
    #[must_use]
    pub const fn with_padding_outer(mut self, p: f32) -> Self {
        self.padding_outer = p;
        self
    }

    /// Full step size between adjacent band starts.
    #[must_use]
    pub fn step(&self) -> f32 {
        let n = self.domain.len() as f32;
        if n <= 0.0 {
            return 0.0;
        }
        let total = (self.range.1 - self.range.0).abs();
        total / (n - self.padding_inner + 2.0 * self.padding_outer).max(1.0)
    }

    /// Drawable width of a single band.
    #[must_use]
    pub fn bandwidth(&self) -> f32 {
        self.step() * (1.0 - self.padding_inner)
    }

    /// Directly project an index to the band's starting position.
    #[must_use]
    pub fn project_index(&self, index: usize) -> f32 {
        if self.domain.is_empty() {
            return f32::NAN;
        }
        let sign = (self.range.1 - self.range.0).signum();
        let step = self.step();
        let start = self.range.0 + sign * step * self.padding_outer;
        start + sign * step * index as f32
    }
}

impl Scale for BandScale {
    fn project(&self, value: f64) -> f32 {
        let idx = value as usize;
        if idx >= self.domain.len() {
            return f32::NAN;
        }
        self.project_index(idx)
    }

    fn unproject(&self, position: f32) -> Option<f64> {
        if self.domain.is_empty() {
            return None;
        }
        let step = self.step();
        if step.abs() < f32::EPSILON {
            return None;
        }
        let sign = (self.range.1 - self.range.0).signum();
        let start = self.range.0 + sign * step * self.padding_outer;
        let idx = (((position - start) / (sign * step)).round() as isize)
            .clamp(0, self.domain.len() as isize - 1);
        Some(idx as f64)
    }

    fn ticks(&self, _count: usize) -> Vec<Tick> {
        self.domain
            .iter()
            .enumerate()
            .map(|(i, label)| Tick {
                value: i as f64,
                position: self.project_index(i) + self.bandwidth() * 0.5,
                label: label.to_string(),
            })
            .collect()
    }

    fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        hash_u32(&mut h, 3); // kind discriminator
        for label in &self.domain {
            for byte in label.as_bytes() {
                h ^= u64::from(*byte);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            // separator so [ab, c] ≠ [a, bc]
            h ^= 0xff;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        hash_f32(&mut h, self.range.0);
        hash_f32(&mut h, self.range.1);
        hash_f32(&mut h, self.padding_inner);
        hash_f32(&mut h, self.padding_outer);
        h
    }

    fn gpu_uniforms(&self) -> ScaleUniforms {
        ScaleUniforms {
            kind: 3,
            flags: 0,
            domain_lo: 0.0,
            domain_hi: self.domain.len() as f32,
            range_lo: self.range.0,
            range_hi: self.range.1,
            domain_lo_hi: 0.0,
            domain_hi_hi: 0.0,
            aux0: self.step(),
            aux1: self.padding_inner,
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
    fn three_bands_partition_range() {
        let s = BandScale::new(["A", "B", "C"], (0.0, 300.0))
            .with_padding_inner(0.0)
            .with_padding_outer(0.0);
        assert!((s.step() - 100.0).abs() < 1e-4);
        assert!((s.bandwidth() - 100.0).abs() < 1e-4);
        assert!((s.project_index(0) - 0.0).abs() < 1e-4);
        assert!((s.project_index(1) - 100.0).abs() < 1e-4);
        assert!((s.project_index(2) - 200.0).abs() < 1e-4);
    }

    #[test]
    fn inner_padding_shrinks_bandwidth() {
        let s = BandScale::new(["A", "B"], (0.0, 100.0))
            .with_padding_inner(0.5)
            .with_padding_outer(0.0);
        // step = 100 / (2 - 0.5) = 66.67; bandwidth = step * 0.5 = 33.33
        assert!((s.bandwidth() - 33.333_3).abs() < 1e-3);
    }
}
