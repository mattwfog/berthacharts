//! # `berthacharts-dist`
//!
//! Distribution marks: violin, ridgeline, beeswarm, boxplot, ECDF, Q-Q, rug,
//! strip. Each is a `Mark` built on top of stats transforms.
//!
//! v0.1 ships the crate slot. Boxplot + ECDF land in v0.1.1 as canonical
//! examples.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
