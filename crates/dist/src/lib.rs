//! # `berthacharts-dist`
//!
//! Distribution marks: [`boxplot`], [`ecdf`], [`violin`], and [`beeswarm`].
//! Each is a [`berthacharts_core::ChartSpec`] built on the same Mark trait
//! surface as the rest of Bertha.
//!
//! ## Family conventions
//!
//! The four specs share one shape so they compose predictably:
//!
//! - **Types**: `XxxGroup`/`EcdfSeries` inputs, `XxxOptions`, `XxxSpec`,
//!   `XxxLayout`, and a `#[non_exhaustive]` `XxxError`.
//! - **Validation** (`XxxSpec::validate`): every spec rejects an empty input,
//!   an empty group/series, an empty label, and any non-finite sample value
//!   with a typed error before layout runs.
//! - **Building**: `try_build_chart(workspace, size)` compiles a chart and
//!   `layout(size)` returns the reusable geometry without one. Degenerate
//!   chart sizes (down to `0×0`) never panic.
//! - **Statistics**: boxplot quartiles use type-7 linear interpolation with
//!   Tukey 1.5·IQR whiskers; violin bandwidth defaults to Silverman's rule;
//!   ECDF collapses ties into exact `m/n` jumps. See each module for details.
//!
//! ## Example
//!
//! ```
//! use berthacharts_dist::boxplot::{BoxPlotGroup, BoxPlotSpec};
//! use berthacharts_dist::core::{ChartSize, Workspace};
//!
//! let spec = BoxPlotSpec::new(vec![
//!     BoxPlotGroup::new("control", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
//!     BoxPlotGroup::new("treatment", vec![2.0, 3.0, 5.0, 8.0, 13.0]),
//! ]);
//! let chart = spec
//!     .try_build_chart(Workspace::new(), ChartSize::new(640, 400))
//!     .expect("valid box plot");
//! assert_eq!(chart.scene().layers.len(), 1);
//! assert!(!chart.snap_targets().is_empty());
//! ```

#![forbid(unsafe_code)]

pub mod beeswarm;
pub mod boxplot;
pub mod ecdf;
pub mod violin;

pub use beeswarm::{
    BeeswarmError, BeeswarmGroup, BeeswarmGroupLayout, BeeswarmLayout, BeeswarmOptions,
    BeeswarmSpec, SwarmDot,
};
pub use berthacharts_core as core;
pub use boxplot::{
    compute_stats, BoxPlotError, BoxPlotGroup, BoxPlotLayout, BoxPlotLayoutGroup, BoxPlotOptions,
    BoxPlotSpec, BoxPlotStats,
};
pub use ecdf::{EcdfError, EcdfLayout, EcdfOptions, EcdfSeries, EcdfSeriesLayout, EcdfSpec};
pub use violin::{ViolinError, ViolinGroup, ViolinLayout, ViolinOptions, ViolinShape, ViolinSpec};
