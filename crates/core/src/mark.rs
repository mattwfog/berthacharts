//! Mark trait: the visual representation of data.
//!
//! A mark answers three questions for each frame:
//! 1. What primitive geometry should the renderer draw? ([`Mark::tessellate`])
//! 2. Which mark does a point in screen space hit? ([`Mark::pick`])
//! 3. Has anything about me changed since last frame? ([`Mark::fingerprint`])

use std::fmt::Debug;
use std::sync::Arc;

use crate::coord::Coord;
use crate::dataset::DatasetRegistry;
use crate::geometry::Geometry;
use crate::ids::MarkId;
use crate::scale::ScaleRegistry;
use crate::scene::Rect;
use crate::selection::Selection;

/// A visual mark — one element of a [`crate::Layer`].
pub trait Mark: Debug + Send + Sync + 'static {
    /// Stable identifier. MUST remain constant across frames for the same
    /// logical mark (same rule as a React key).
    fn id(&self) -> MarkId;

    /// 64-bit cache key. The renderer reuses last frame's tessellated output
    /// when the fingerprint is unchanged.
    fn fingerprint(&self) -> u64;

    /// Produce renderer-neutral geometry. Called during the prepare phase.
    fn tessellate(&self, ctx: &TessellateCtx<'_>) -> Geometry;

    /// Hit-test a screen-local point against this mark.
    ///
    /// Called in quadtree bucket order; implementations should be cheap. Return
    /// `None` when the point misses or the mark is un-pickable.
    fn pick(&self, ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit>;

    /// Axis-aligned screen-space bounding box. Used to build the quadtree and
    /// to cull marks that don't overlap the viewport.
    fn bounds(&self, ctx: &TessellateCtx<'_>) -> Rect;

    /// Type-erased downcast support.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Context available during tessellation.
#[non_exhaustive]
pub struct TessellateCtx<'a> {
    /// Coord system for the owning layer.
    pub coord: &'a dyn Coord,
    /// Registered scales on the workspace.
    pub scales: &'a ScaleRegistry,
    /// Registered datasets on the workspace (after DAG resolution).
    pub datasets: &'a DatasetRegistry,
    /// Layer plot-area in screen pixels.
    pub plot_area: Rect,
    /// Device pixel ratio (for hairline widths, AA feathers, etc.).
    pub device_pixel_ratio: f32,
}

impl<'a> TessellateCtx<'a> {
    /// Construct a tessellation context. The struct is `#[non_exhaustive]` so
    /// external crates (the renderer) must go through this constructor.
    #[must_use]
    pub fn new(
        coord: &'a dyn Coord,
        scales: &'a ScaleRegistry,
        datasets: &'a DatasetRegistry,
        plot_area: Rect,
        device_pixel_ratio: f32,
    ) -> Self {
        Self {
            coord,
            scales,
            datasets,
            plot_area,
            device_pixel_ratio,
        }
    }
}

/// Context available during picking.
#[non_exhaustive]
pub struct PickCtx<'a> {
    /// Coord system for the owning layer.
    pub coord: &'a dyn Coord,
    /// Registered scales.
    pub scales: &'a ScaleRegistry,
    /// Registered datasets.
    pub datasets: &'a DatasetRegistry,
    /// Current selection state on the workspace.
    pub selection: &'a Selection,
    /// Layer plot-area.
    pub plot_area: Rect,
    /// Device pixel ratio.
    pub device_pixel_ratio: f32,
}

impl<'a> PickCtx<'a> {
    /// Construct a pick context — the struct is `#[non_exhaustive]`.
    #[must_use]
    pub fn new(
        coord: &'a dyn Coord,
        scales: &'a ScaleRegistry,
        datasets: &'a DatasetRegistry,
        selection: &'a Selection,
        plot_area: Rect,
        device_pixel_ratio: f32,
    ) -> Self {
        Self {
            coord,
            scales,
            datasets,
            selection,
            plot_area,
            device_pixel_ratio,
        }
    }
}

/// A successful hit test.
#[derive(Debug, Clone)]
pub struct PickHit {
    /// Mark that was hit.
    pub mark: MarkId,
    /// Row index within the mark's source dataset, if applicable.
    pub row: Option<usize>,
    /// Distance from the cursor to the hit primitive (pixels). Lower wins when
    /// multiple marks overlap.
    pub distance: f32,
    /// Optional payload bag for binding-layer consumption.
    pub payload: Option<Arc<dyn std::any::Any + Send + Sync>>,
}
