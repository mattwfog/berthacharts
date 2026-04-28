//! Error types used across the kernel.

use thiserror::Error;

/// Errors produced by the public [`crate::Chart`] surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ChartError {
    /// A referenced scale, dataset, coord system, or mark does not exist.
    #[error("unknown reference: {kind} id={id}")]
    UnknownRef {
        /// Kind of reference that could not be resolved ("scale", "dataset", ...).
        kind: &'static str,
        /// The raw numeric ID that failed to resolve.
        id: u64,
    },

    /// A transform failed while running inside the DAG.
    #[error(transparent)]
    Transform(#[from] TransformError),

    /// An invariant of the scene graph was violated (e.g. duplicate mark id).
    #[error("scene invariant violated: {0}")]
    Scene(&'static str),
}

/// Errors produced by [`crate::Transform::run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransformError {
    /// Transform received the wrong number of inputs.
    #[error("expected {expected} inputs, got {actual}")]
    Arity {
        /// Number of inputs declared by [`crate::Transform::input_arity`].
        expected: usize,
        /// Number of inputs actually supplied.
        actual: usize,
    },

    /// A required column was missing from an input dataset.
    #[error("missing column `{0}`")]
    MissingColumn(String),

    /// A column existed but had an incompatible dtype.
    #[error("column `{column}` has dtype `{got}`, expected `{expected}`")]
    DtypeMismatch {
        /// Column name.
        column: String,
        /// Actual dtype tag.
        got: &'static str,
        /// Required dtype tag.
        expected: &'static str,
    },

    /// Transform-specific validation failure (e.g. non-finite input, empty dataset).
    #[error("transform `{name}` failed: {message}")]
    Other {
        /// Transform name for logs.
        name: &'static str,
        /// Human-readable message.
        message: String,
    },
}
