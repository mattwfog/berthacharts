//! # `berthacharts-network`
//!
//! Network / graph charts and layouts: Sankey and sunburst now,
//! force-directed/chord/tree layouts later. Specs compile data into core
//! scenes, guides, interactions, and renderer-neutral marks.

#![forbid(unsafe_code)]

pub mod chord;
pub mod force;
pub mod sankey;
pub mod sunburst;
pub mod tree;

pub use berthacharts_core as core;
pub use chord::{
    ChordArc, ChordError, ChordLayout, ChordLink, ChordNode, ChordOptions, ChordRibbon, ChordSpec,
};
pub use force::{
    EdgeStyle, ForceEdge, ForceError, ForceLayout, ForceLayoutEdge, ForceLayoutNode, ForceNode,
    ForceOptions, ForceSpec,
};
pub use tree::{
    TreeEdge, TreeError, TreeLayout, TreeLayoutEdge, TreeLayoutNode, TreeNode, TreeOptions,
    TreeOrientation, TreeSpec,
};
pub use sankey::{
    SankeyError, SankeyFlow, SankeyLayout, SankeyLayoutNode, SankeyLayoutStage, SankeyLegendItem,
    SankeyLink, SankeyNode, SankeyOptions, SankeyRibbon, SankeySpec, SankeyStage,
};
pub use sunburst::{
    SunburstBranchSummary, SunburstError, SunburstLayout, SunburstLegendItem, SunburstNode,
    SunburstOptions, SunburstPath, SunburstSector, SunburstSpec, SunburstSummary,
};
