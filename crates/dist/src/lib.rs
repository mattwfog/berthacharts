//! # `berthacharts-dist`
//!
//! Distribution marks: boxplot, ECDF, violin, beeswarm. Each is a
//! [`berthacharts_core::ChartSpec`] built on the same Mark trait surface as
//! the rest of Bertha.

#![forbid(unsafe_code)]

pub mod beeswarm;
pub mod boxplot;
pub mod ecdf;
pub mod violin;

pub use berthacharts_core as core;
pub use beeswarm::{
    BeeswarmError, BeeswarmGroup, BeeswarmGroupLayout, BeeswarmLayout, BeeswarmOptions,
    BeeswarmSpec, SwarmDot,
};
pub use boxplot::{
    compute_stats, BoxPlotError, BoxPlotGroup, BoxPlotLayout, BoxPlotLayoutGroup, BoxPlotOptions,
    BoxPlotSpec, BoxPlotStats,
};
pub use ecdf::{EcdfError, EcdfLayout, EcdfOptions, EcdfSeries, EcdfSeriesLayout, EcdfSpec};
pub use violin::{
    ViolinError, ViolinGroup, ViolinLayout, ViolinOptions, ViolinShape, ViolinSpec,
};
