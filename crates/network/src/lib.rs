//! # `berthacharts-network`
//!
//! Network / graph charts and layouts: Sankey now, force-directed/chord/tree
//! layouts later. Specs compile data into core scenes, guides, interactions,
//! and renderer-neutral marks.

#![forbid(unsafe_code)]

pub mod sankey;

pub use berthacharts_core as core;
