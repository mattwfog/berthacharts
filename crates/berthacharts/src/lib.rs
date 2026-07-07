//! User-facing entry point for Bertha Charts.
//!
//! This crate is a facade over the smaller workspace crates. Use it when you
//! want a stable import path for application code:
//!
//! ```
//! #[cfg(feature = "charts")]
//! {
//!     use berthacharts::prelude::*;
//!
//!     let spec = BarChartSpec::new(vec![
//!         BarDatum::new("Q1", 42.0),
//!         BarDatum::new("Q2", 57.0),
//!     ]);
//!     let chart = spec.build(ChartSize::new(640, 360));
//!     assert!(chart.is_ok());
//! }
//! ```
//!
//! Advanced users can still depend on the leaf crates directly. Every
//! feature-gated crate in the facade is published; enable the features you
//! need (`dist`, `finance`, `network`, `geo`, `stats`, `anno`,
//! `renderer-wgpu`, `leptos`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

/// Core chart kernel: scales, coordinates, datasets, scenes, guides, and
/// interaction primitives.
pub mod core {
    pub use berthacharts_core::*;
}

/// Reusable first-party chart specifications.
#[cfg(feature = "charts")]
pub mod charts {
    pub use berthacharts_charts::*;
}

/// Data transforms that implement the core transform trait.
#[cfg(feature = "transforms")]
pub mod transforms {
    pub use berthacharts_transforms::*;
}

/// Statistical models and transforms.
#[cfg(feature = "stats")]
pub mod stats {
    pub use berthacharts_stats::*;
}

/// Geospatial charts, maps, projections, and GeoJSON helpers.
#[cfg(feature = "geo")]
pub mod geo {
    pub use berthacharts_geo::*;
}

/// Network and hierarchy chart specifications.
#[cfg(feature = "network")]
pub mod network {
    pub use berthacharts_network::*;
}

/// Annotation primitives — reference lines, bands, arrows.
#[cfg(feature = "anno")]
pub mod anno {
    pub use berthacharts_anno::*;
}

/// Distribution marks — boxplot, violin, beeswarm, ECDF.
#[cfg(feature = "dist")]
pub mod dist {
    pub use berthacharts_dist::*;
}

/// Finance domain — candlestick / OHLC + indicators (MA, EMA, Bollinger, RSI).
#[cfg(feature = "finance")]
pub mod finance {
    pub use berthacharts_finance::*;
}

/// wgpu renderer backend.
#[cfg(feature = "renderer-wgpu")]
pub mod renderer_wgpu {
    pub use berthacharts_renderer_wgpu::*;
}

/// Leptos bindings.
#[cfg(feature = "leptos")]
pub mod leptos {
    pub use berthacharts_leptos::*;
}

/// Convenience methods for any [`ChartSpec`].
///
/// Import this trait, or use [`prelude`], to build a chart without manually
/// creating a [`Workspace`] first.
pub trait ChartSpecExt: ChartSpec {
    /// Build this spec in a fresh workspace.
    fn build(&self, size: ChartSize) -> Result<Chart, Self::Error> {
        self.build_in(Workspace::new(), size)
    }

    /// Build this spec in an existing workspace.
    fn build_in(&self, workspace: Arc<Workspace>, size: ChartSize) -> Result<Chart, Self::Error> {
        self.build_chart(workspace, size)
    }
}

impl<T> ChartSpecExt for T where T: ChartSpec + ?Sized {}

/// Common imports for application code.
pub mod prelude {
    pub use crate::core::{
        Chart, ChartError, ChartSize, ChartSpec, Dataset, DatasetId, Guide, Mark, Rect, Scene,
        Viewport, Workspace,
    };
    pub use crate::ChartSpecExt;

    #[cfg(feature = "charts")]
    pub use crate::charts::{
        AreaChartError, AreaChartOptions, AreaChartSpec, AreaDatum, BarChartError, BarChartOptions,
        BarChartSpec, BarChartSummary, BarDatum, DotMode, HeatmapCell, HeatmapError,
        HeatmapOptions, HeatmapSpec, HeatmapSummary, HistogramBin, HistogramError,
        HistogramOptions, HistogramSpec, LineChartError, LineChartOptions, LineChartSpec,
        LineChartSummary, LineDatum, ScatterDatum, ScatterPlotError, ScatterPlotOptions,
        ScatterPlotSpec, ScatterPlotSummary, SparklineDatum, SparklineError, SparklineOptions,
        SparklineSpec, StackMode,
    };

    #[cfg(feature = "transforms")]
    pub use crate::transforms::{AggOp, Aggregate, Bin, FilterRange, Stack};

    #[cfg(feature = "geo")]
    pub use crate::geo::{
        GeoFeature, GeoGeometry, GeoJsonError, GeoJsonReadOptions, GeoMapError, GeoMapOptions,
        GeoMapSpec, GeoMapSummary, GeoPosition, GeoProjection,
    };

    #[cfg(feature = "network")]
    pub use crate::network::{
        ChordError, ChordLink, ChordNode, ChordOptions, ChordSpec, EdgeStyle, ForceEdge,
        ForceError, ForceNode, ForceOptions, ForceSpec, SankeyError, SankeyFlow, SankeyLegendItem,
        SankeyLink, SankeyNode, SankeyOptions, SankeySpec, SankeyStage, SunburstError,
        SunburstLegendItem, SunburstNode, SunburstOptions, SunburstPath, SunburstSpec, TreeEdge,
        TreeError, TreeNode, TreeOptions, TreeOrientation, TreeSpec,
    };

    #[cfg(feature = "anno")]
    pub use crate::anno::{
        AnnotationLayer, Arrow, AxisRef, BandAxis, ReferenceBand, ReferenceLine,
    };

    #[cfg(feature = "dist")]
    pub use crate::dist::{
        BeeswarmError, BeeswarmGroup, BeeswarmOptions, BeeswarmSpec, BoxPlotError, BoxPlotGroup,
        BoxPlotOptions, BoxPlotSpec, BoxPlotStats, EcdfError, EcdfOptions, EcdfSeries, EcdfSpec,
        ViolinError, ViolinGroup, ViolinOptions, ViolinSpec,
    };

    #[cfg(feature = "finance")]
    pub use crate::finance::{
        atr, bollinger_bands, exponential_moving_average, ichimoku, macd, moving_average, obv, rsi,
        stochastic, vwap, williams_r, BollingerBands, Candle, CandleStyle, CandlestickError,
        CandlestickOptions, CandlestickSpec, Ichimoku, Macd, Overlay, Stochastic,
    };

    #[cfg(feature = "stats")]
    pub use crate::stats::{
        confidence_radius_3d, CovarianceEstimator, Gaussian3, Gaussian3Error, Gaussian3FitOptions,
        Gaussian3Mixture, Gaussian3MixtureFitOptions, Mat3, Vec3,
    };
}

#[cfg(feature = "charts")]
pub use berthacharts_charts::{
    AreaChartError, AreaChartOptions, AreaChartSpec, AreaDatum, BarChartError, BarChartOptions,
    BarChartSpec, BarChartSummary, BarDatum, DotMode, HeatmapCell, HeatmapError, HeatmapOptions,
    HeatmapSpec, HeatmapSummary, HistogramBin, HistogramError, HistogramOptions, HistogramSpec,
    LineChartError, LineChartOptions, LineChartSpec, LineChartSummary, LineDatum, ScatterDatum,
    ScatterPlotError, ScatterPlotOptions, ScatterPlotSpec, ScatterPlotSummary, SparklineDatum,
    SparklineError, SparklineOptions, SparklineSpec, StackMode,
};
pub use berthacharts_core::{
    Chart, ChartError, ChartSize, ChartSpec, Dataset, DatasetId, Guide, Mark, Rect, Scene,
    Viewport, Workspace,
};
#[cfg(feature = "geo")]
pub use berthacharts_geo::{
    features_from_geojson_str, features_from_geojson_str_with_options, GeoFeature, GeoGeometry,
    GeoJsonError, GeoJsonReadOptions, GeoMapError, GeoMapOptions, GeoMapSpec, GeoMapSummary,
    GeoPosition, GeoProjection,
};
#[cfg(feature = "network")]
pub use berthacharts_network::{
    SankeyError, SankeyFlow, SankeyLayout, SankeyLayoutNode, SankeyLayoutStage, SankeyLegendItem,
    SankeyLink, SankeyNode, SankeyOptions, SankeyRibbon, SankeySpec, SankeyStage,
    SunburstBranchSummary, SunburstError, SunburstLayout, SunburstLegendItem, SunburstNode,
    SunburstOptions, SunburstPath, SunburstSector, SunburstSpec, SunburstSummary,
};
#[cfg(feature = "stats")]
pub use berthacharts_stats::{
    confidence_radius_3d, Bounds3, CovarianceEstimator, Gaussian3, Gaussian3Component,
    Gaussian3DensityVoxel, Gaussian3Ellipsoid, Gaussian3Error, Gaussian3Feature,
    Gaussian3FeatureGame, Gaussian3FitOptions, Gaussian3Mesh, Gaussian3Mixture,
    Gaussian3MixtureCandidate, Gaussian3MixtureFitOptions, Gaussian3MixtureSelection,
    Gaussian3Shapley, Gaussian3ShapleyInteraction, Gaussian3ShapleyScore, Gaussian3SigmaPoint,
    Gaussian3SigmaPointSet, Gaussian3Summary, Mat3, SymmetricEigen3, Vec3,
};
#[cfg(feature = "transforms")]
pub use berthacharts_transforms::{AggOp, Aggregate, Bin, FilterRange, Stack};
