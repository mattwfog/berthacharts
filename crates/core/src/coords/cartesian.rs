//! Cartesian coord system — the identity coord.
//!
//! Scales are configured with ranges in plot-area pixels, so the coord system
//! is deliberately a pass-through. Non-Cartesian coords (polar, geo, network)
//! are where the projection machinery earns its keep.

use crate::coord::{Coord, Projected, Unprojected};
use crate::ids::ScaleId;

/// Identity mapping with references to the x / y scales the layer uses.
#[derive(Debug, Clone)]
pub struct CartesianCoord {
    /// Scale driving the horizontal axis.
    pub x: ScaleId,
    /// Scale driving the vertical axis.
    pub y: ScaleId,
    /// Cached scale-dependency list returned by [`Coord::scale_deps`].
    deps: [ScaleId; 2],
}

impl CartesianCoord {
    /// Build a Cartesian coord from two scale ids.
    #[must_use]
    pub const fn new(x: ScaleId, y: ScaleId) -> Self {
        Self { x, y, deps: [x, y] }
    }
}

impl Coord for CartesianCoord {
    fn project(&self, p: Unprojected) -> Projected {
        Projected {
            x: p.u as f32,
            y: p.v as f32,
        }
    }

    fn unproject(&self, p: Projected) -> Option<Unprojected> {
        Some(Unprojected {
            u: f64::from(p.x),
            v: f64::from(p.y),
        })
    }

    fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        h ^= u64::from(self.x.get());
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= u64::from(self.y.get());
        h = h.wrapping_mul(0x0100_0000_01b3);
        h
    }

    fn scale_deps(&self) -> &[ScaleId] {
        &self.deps
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
