//! Geospatial charts, maps, projections, and GeoJSON helpers.

#![forbid(unsafe_code)]

mod geojson;
mod map;
mod projection;

pub use berthacharts_core as core;
pub use geojson::{
    features_from_geojson_str, features_from_geojson_str_with_options, GeoJsonError,
    GeoJsonReadOptions,
};
pub use map::{
    GeoFeature, GeoGeometry, GeoMapError, GeoMapOptions, GeoMapSpec, GeoMapSummary, GeoPosition,
};
pub use projection::{GeoBounds, GeoProjection};
