//! Coordinate systems: project layer-local (x,y) domain values to screen.
//!
//! `Coord` is a trait so custom projections (polar, parallel, geo, network,
//! arc, radial, ...) are first-class. Marks stay coord-agnostic — they hand
//! logical values to the coord and receive screen-space points.

use std::fmt::Debug;

pub use crate::ids::ScaleId;

/// Newtype for an ID of a coord system in a [`crate::Workspace`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CoordId(pub u32);

impl CoordId {
    /// Construct from raw.
    #[must_use]
    pub const fn new(v: u32) -> Self {
        Self(v)
    }

    /// Raw representation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A point in the logical (unprojected) space a coord operates on.
#[derive(Debug, Clone, Copy)]
pub struct Unprojected {
    /// First axis (Cartesian x, polar theta, parallel axis-0, geo lon, ...).
    pub u: f64,
    /// Second axis (Cartesian y, polar r, parallel axis-1, geo lat, ...).
    pub v: f64,
}

/// A point in screen-local coordinates (plot-area pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projected {
    /// Horizontal screen position.
    pub x: f32,
    /// Vertical screen position.
    pub y: f32,
}

/// A coordinate system: bidirectional mapping between a logical 2D space and
/// screen-local pixels within a layer's plot area.
pub trait Coord: Debug + Send + Sync + 'static {
    /// Project a logical point to screen-local pixels.
    fn project(&self, p: Unprojected) -> Projected;

    /// Inverse projection for hit testing.
    /// Returns `None` if no well-defined inverse exists at that screen point.
    fn unproject(&self, p: Projected) -> Option<Unprojected>;

    /// Cache key; MUST change when projection behavior would change.
    fn fingerprint(&self) -> u64;

    /// Scales the coord depends on (used by the DAG to invalidate transforms
    /// whose outputs feed marks in this coord when a scale changes).
    fn scale_deps(&self) -> &[ScaleId];

    /// Type-erased downcast support.
    fn as_any(&self) -> &dyn std::any::Any;
}
