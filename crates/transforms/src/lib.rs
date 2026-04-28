//! # `berthacharts-transforms`
//!
//! Fundamental transforms that every analytical chart relies on:
//! filter, bin, aggregate, stack, sort, group, window, normalize.
//!
//! v0.1 ships the crate slot — implementations land in v0.1.1 once the core
//! trait contract is pinned.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
