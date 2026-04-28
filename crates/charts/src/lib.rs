//! Reusable Cartesian chart specifications.
//!
//! This crate sits above `berthacharts-core`: specs own data normalization,
//! layout, guides, direct labels, and interaction metadata, then compile to
//! regular core charts.

#![forbid(unsafe_code)]

pub mod bar;
pub mod heatmap;
pub mod line;
mod mark;
pub mod scatter;

pub use berthacharts_core as core;
