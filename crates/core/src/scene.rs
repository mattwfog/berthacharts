//! Scene graph: the declarative description of what a chart draws.
//!
//! A [`Scene`] is a flat list of [`Layer`]s. Each layer owns its own
//! [`crate::Coord`] system and a list of marks. Layers composite top-down
//! with a blend mode — mimicking Photoshop / SVG layer semantics.

use std::sync::Arc;

use crate::coord::CoordId;
use crate::guide::Guide;
use crate::ids::{LayerId, MarkId};
use crate::interaction::Interaction;
use crate::mark::Mark;

/// A rectangle in screen-local pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Top-left x.
    pub x: f32,
    /// Top-left y.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Build from top-left + size.
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Zero-sized rect at origin.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    /// Returns true if `p` lies inside the rect (inclusive).
    #[must_use]
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 <= self.x + self.w && p.1 >= self.y && p.1 <= self.y + self.h
    }
}

/// Viewport describes the drawable surface and its plot-area sub-region.
///
/// The plot area is the region available to marks after reserving margin for
/// the DOM overlay (axes, titles, legends). Layers clip to the plot area by
/// default; bindings may override.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// Canvas width in CSS pixels.
    pub width: u32,
    /// Canvas height in CSS pixels.
    pub height: u32,
    /// Device pixel ratio (`window.devicePixelRatio`).
    pub device_pixel_ratio: f32,
    /// Subregion where marks render.
    pub plot_area: Rect,
}

impl Viewport {
    /// Construct a viewport with plot_area covering the full canvas.
    #[must_use]
    pub fn full(width: u32, height: u32, dpr: f32) -> Self {
        Self {
            width,
            height,
            device_pixel_ratio: dpr,
            plot_area: Rect::new(0.0, 0.0, width as f32, height as f32),
        }
    }
}

/// Compositing mode between a layer and everything beneath it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendMode {
    /// Source-over (default).
    #[default]
    Normal,
    /// Additive blending — useful for density / heatmap accumulation.
    Additive,
    /// Multiply — useful for shadows / dark overlays.
    Multiply,
    /// Screen — useful for bright overlays.
    Screen,
}

/// A single layer in a scene.
///
/// Every layer has its own coord system. That is what lets a polar radar live
/// inside the same scene as a Cartesian reference grid.
pub struct Layer {
    /// Stable id (required for diffing).
    pub id: LayerId,
    /// Coord system identifier. The actual coord is held in the workspace.
    pub coord: CoordId,
    /// Marks in z-order (earlier = drawn first).
    pub marks: Vec<Arc<dyn Mark>>,
    /// Compositing mode against layers below.
    pub blend: BlendMode,
    /// Opacity multiplier (`0.0..=1.0`).
    pub opacity: f32,
    /// Z-order; higher draws later (on top).
    pub z: i32,
    /// Optional clip rectangle (plot-area-local). `None` ⇒ clip to plot area.
    pub clip: Option<Rect>,
}

impl std::fmt::Debug for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layer")
            .field("id", &self.id)
            .field("coord", &self.coord)
            .field("mark_count", &self.marks.len())
            .field("blend", &self.blend)
            .field("opacity", &self.opacity)
            .field("z", &self.z)
            .field("clip", &self.clip)
            .finish()
    }
}

/// A complete declarative scene description.
#[derive(Debug)]
pub struct Scene {
    /// Layers in document order (pre-sort). The renderer sorts by `z` on
    /// prepare.
    pub layers: Vec<Layer>,
    /// Overlay guides (axes, legends, future tooltip specs).
    pub guides: Vec<Guide>,
    /// Declarative interaction affordances consumed by bindings and overlays.
    pub interactions: Vec<Interaction>,
    /// Current viewport geometry.
    pub viewport: Viewport,
}

impl Scene {
    /// Build an empty scene at the given viewport.
    #[must_use]
    pub fn new(viewport: Viewport) -> Self {
        Self {
            layers: Vec::new(),
            guides: Vec::new(),
            interactions: Vec::new(),
            viewport,
        }
    }

    /// Find a layer by id.
    #[must_use]
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Find a mark across all layers.
    #[must_use]
    pub fn mark(&self, id: MarkId) -> Option<(&Layer, &Arc<dyn Mark>)> {
        for layer in &self.layers {
            if let Some(m) = layer.marks.iter().find(|m| m.id() == id) {
                return Some((layer, m));
            }
        }
        None
    }
}
