//! # `berthacharts-anno`
//!
//! Annotation primitives: reference lines, reference bands, arrows.
//! Annotations are first-class because users edit them. An
//! [`AnnotationLayer`] is a single Mark consumers attach to their scene to
//! overlay multiple annotations at once.

#![forbid(unsafe_code)]

pub mod annotation;

pub use annotation::{
    AnnotationLayer, Arrow, AxisRef, BandAxis, ConfidenceRibbon, ReferenceBand, ReferenceLine,
    StatBracket, TextCallout,
};
pub use berthacharts_core as core;
