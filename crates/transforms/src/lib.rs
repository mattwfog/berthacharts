//! # `berthacharts-transforms`
//!
//! Fundamental transforms that every analytical chart relies on:
//! filter, bin, aggregate, stack, sort, group, window, normalize.
//!
//! v0.0.1 ships seed implementations for the four transforms that unblock the
//! most downstream work — [`FilterRange`], [`Bin`], [`Aggregate`], and
//! [`Stack`]. Each implements [`berthacharts_core::Transform`] and is keyed by
//! a stable 64-bit fingerprint so the DAG can memoize results.
//!
//! Future transforms (sort, window, normalize, group, join) will land in
//! v0.1.x as the trait surface stabilizes.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use berthacharts_core as core;

mod aggregate;
mod bin;
mod filter;
mod fingerprint;
mod stack;

pub use aggregate::{AggOp, Aggregate};
pub use bin::Bin;
pub use filter::FilterRange;
pub use stack::Stack;
