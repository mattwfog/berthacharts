//! # Bertha Charts — Core Kernel
//!
//! Foundational primitives for the Bertha Charts library. This crate defines
//! the public trait contract ([`Scale`], [`Coord`], [`Mark`], [`Transform`])
//! and the [`Chart`] / [`Workspace`] state machine. It contains no rendering,
//! no framework bindings, and no built-in chart types.
//!
//! ## Invariants
//!
//! - No `std::thread`, `std::time::Instant`, or `println!` — the kernel stays
//!   server-side and embedded-friendly.
//! - Public traits are the semver stability contract. Everything else is
//!   best-effort until the API is pinned at 1.0.
//! - Text, axes, legends, and tooltips are rendered by the binding layer's
//!   DOM overlay, never by this crate.
//!
//! ## Layering
//!
//! ```text
//! Dataset ──► Transform DAG ──► Scene { Layers { Marks } } ──► Geometry
//!                                       │
//!                                       └─ Coord system projects to screen
//!                                          Scales provide domain → range
//! ```
//!
//! The renderer (see `berthacharts-renderer-wgpu`) consumes [`Geometry`] via
//! [`Mark::tessellate`] and executes GPU draw calls. Core never touches GPU
//! state directly.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod channel;
pub mod chart;
pub mod coord;
pub mod coords;
pub mod dag;
pub mod dataset;
pub mod error;
pub mod event;
pub mod geometry;
pub mod guide;
pub mod ids;
pub mod interaction;
pub mod mark;
pub mod marks;
pub mod picker;
pub mod scale;
pub mod scales;
pub mod scene;
pub mod selection;
pub mod spec;
pub mod transform;
pub mod workspace;

// Re-export the public surface so users can `use berthacharts_core::Chart;`.
pub use channel::{Channel, ColorChannel, NumberChannel};
pub use chart::Chart;
pub use coord::{Coord, CoordId, Projected, Unprojected};
pub use coords::CartesianCoord;
pub use dag::{Dag, Fingerprint, NodeId};
pub use dataset::{Column, ColumnData, Dataset, DatasetId, DatasetRegistry};
pub use error::{ChartError, TransformError};
pub use event::{ChartEvent, Subscription};
pub use geometry::{Geometry, LinePrim, PathCommand, PathPrim, PointPrim, RectPrim, TrianglePrim};
pub use guide::{
    AxisGuide, AxisOrient, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind, LabelPriority,
    LabelTooltip, LabelTooltipRow, LegendAnchor, LegendGuide, LegendItem, TooltipField,
    TooltipGuide, TooltipValueFormat,
};
pub use ids::{LayerId, MarkId, ScaleId, SelectionId, TransformId};
pub use interaction::{Interaction, SnapKind, SnapTarget, SnapTargetSet};
pub use mark::{Mark, PickCtx, PickHit, TessellateCtx};
pub use marks::RectMark;
pub use picker::Picker;
pub use scale::{Scale, ScaleRegistry, ScaleUniforms, Tick};
pub use scales::{BandScale, LinearScale};
pub use scene::{BlendMode, Layer, Rect, Scene, Viewport};
pub use selection::{Selection, SelectionChannel};
pub use spec::{ChartSize, ChartSpec};
pub use transform::{Transform, TransformInputs, TransformOutput};
pub use workspace::Workspace;
