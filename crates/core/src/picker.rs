//! Picker: spatial index over tessellated mark bounds.
//!
//! At v0.1 the picker is a skeleton type — implementation (quadtree over
//! mark bounding boxes) lands in a follow-up. The public surface is fixed so
//! bindings can wire events without waiting for the implementation.

use crate::mark::PickHit;

/// CPU-side spatial picker.
///
/// Ownership lives in [`crate::Chart`]; bindings never construct one directly.
#[derive(Debug, Default)]
pub struct Picker {
    /// Internal quadtree (not yet implemented).
    _private: (),
}

impl Picker {
    /// Construct an empty picker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the hit, if any, for a screen-local point.
    ///
    /// Placeholder — always returns `None` until the quadtree lands.
    #[must_use]
    pub fn pick(&self, _point: (f32, f32)) -> Option<PickHit> {
        None
    }
}
