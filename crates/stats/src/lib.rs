//! # `berthacharts-stats`
//!
//! Statistical transforms: linear / LOESS / polynomial regression, KDE, CDF,
//! quantile, bootstrap CI, correlation, PCA.
//!
//! v0.1 ships the crate slot. Seed implementations (OLS, KDE, bootstrap CI)
//! land in v0.1.1 to validate the transform trait contract.

#![forbid(unsafe_code)]

pub use berthacharts_core as core;
