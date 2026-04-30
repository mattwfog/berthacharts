//! # `berthacharts-network`
//!
//! Network / graph charts and layouts: Sankey and sunburst now,
//! force-directed/chord/tree layouts later. Specs compile data into core
//! scenes, guides, interactions, and renderer-neutral marks.

#![forbid(unsafe_code)]

pub mod sankey;
pub mod sunburst;

pub use berthacharts_core as core;
pub use sankey::{
    SankeyError, SankeyFlow, SankeyLayout, SankeyLayoutNode, SankeyLayoutStage, SankeyLegendItem,
    SankeyLink, SankeyNode, SankeyOptions, SankeyRibbon, SankeySpec, SankeyStage,
};
pub use sunburst::{
    SunburstBranchSummary, SunburstError, SunburstLayout, SunburstLegendItem, SunburstNode,
    SunburstOptions, SunburstPath, SunburstSector, SunburstSpec, SunburstSummary,
};
