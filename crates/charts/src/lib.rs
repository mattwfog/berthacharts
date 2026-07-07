//! Reusable Cartesian chart specifications.
//!
//! This crate sits above `berthacharts-core`: specs own data normalization,
//! layout, guides, direct labels, and interaction metadata, then compile to
//! regular core charts.

#![forbid(unsafe_code)]

pub mod area;
pub mod bar;
pub mod heatmap;
pub mod histogram;
pub mod line;
mod mark;
pub mod scatter;
pub mod sparkline;

pub use area::{
    AreaBand, AreaChartError, AreaChartLayout, AreaChartOptions, AreaChartSpec, AreaDatum,
    StackMode,
};
pub use bar::{BarChartError, BarChartOptions, BarChartSpec, BarChartSummary, BarDatum};
pub use berthacharts_core as core;
pub use heatmap::{HeatmapCell, HeatmapError, HeatmapOptions, HeatmapSpec, HeatmapSummary};
pub use histogram::{HistogramBin, HistogramError, HistogramOptions, HistogramSpec};
pub use line::{LineChartError, LineChartOptions, LineChartSpec, LineChartSummary, LineDatum};
pub use scatter::{
    ScatterDatum, ScatterPlotError, ScatterPlotOptions, ScatterPlotSpec, ScatterPlotSummary,
};
pub use sparkline::{
    DotMode, SparklineDatum, SparklineError, SparklineLayout, SparklineOptions, SparklineSpec,
};
