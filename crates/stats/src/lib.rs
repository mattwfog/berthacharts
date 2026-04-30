//! # `berthacharts-stats`
//!
//! Statistical transforms and models: linear / LOESS / polynomial regression,
//! KDE, CDF, quantile, bootstrap CI, correlation, PCA, and 3D Gaussian models.
//!
//! v0.1 ships a dependency-light 3D Gaussian model first, giving downstream
//! charts a concrete statistical surface for density volumes and confidence
//! ellipsoids.

#![forbid(unsafe_code)]

mod gaussian3;

pub use berthacharts_core as core;
pub use gaussian3::{
    confidence_radius_3d, Bounds3, CovarianceEstimator, Gaussian3, Gaussian3Component,
    Gaussian3DensityVoxel, Gaussian3Ellipsoid, Gaussian3Error, Gaussian3Feature,
    Gaussian3FeatureGame, Gaussian3FitOptions, Gaussian3Mesh, Gaussian3Mixture,
    Gaussian3MixtureCandidate, Gaussian3MixtureFitOptions, Gaussian3MixtureSelection,
    Gaussian3Shapley, Gaussian3ShapleyInteraction, Gaussian3ShapleyScore, Gaussian3SigmaPoint,
    Gaussian3SigmaPointSet, Gaussian3Summary, Mat3, SymmetricEigen3, Vec3,
};
