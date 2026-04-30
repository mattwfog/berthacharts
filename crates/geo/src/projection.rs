//! Geographic projections used by map specs.

use std::f32::consts::PI;

use crate::map::{GeoGeometry, GeoPosition};

/// Geographic longitude/latitude bounds in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    /// Minimum longitude.
    pub min_lon: f32,
    /// Minimum latitude.
    pub min_lat: f32,
    /// Maximum longitude.
    pub max_lon: f32,
    /// Maximum latitude.
    pub max_lat: f32,
}

impl GeoBounds {
    /// Build bounds from all coordinates in a geometry list.
    #[must_use]
    pub fn from_geometries<'a>(geometries: impl Iterator<Item = &'a GeoGeometry>) -> Option<Self> {
        let mut bounds = BoundsAccumulator::default();
        for geometry in geometries {
            geometry.visit_positions(&mut |position| bounds.push(position));
        }
        bounds.finish()
    }

    /// Width in longitude degrees.
    #[must_use]
    pub fn lon_span(self) -> f32 {
        self.max_lon - self.min_lon
    }

    /// Height in latitude degrees.
    #[must_use]
    pub fn lat_span(self) -> f32 {
        self.max_lat - self.min_lat
    }
}

#[derive(Default)]
struct BoundsAccumulator {
    min_lon: f32,
    min_lat: f32,
    max_lon: f32,
    max_lat: f32,
    initialized: bool,
}

impl BoundsAccumulator {
    fn push(&mut self, position: GeoPosition) {
        if !self.initialized {
            self.min_lon = position.lon;
            self.max_lon = position.lon;
            self.min_lat = position.lat;
            self.max_lat = position.lat;
            self.initialized = true;
            return;
        }
        self.min_lon = self.min_lon.min(position.lon);
        self.max_lon = self.max_lon.max(position.lon);
        self.min_lat = self.min_lat.min(position.lat);
        self.max_lat = self.max_lat.max(position.lat);
    }

    fn finish(self) -> Option<GeoBounds> {
        self.initialized.then_some(GeoBounds {
            min_lon: self.min_lon,
            min_lat: self.min_lat,
            max_lon: self.max_lon,
            max_lat: self.max_lat,
        })
    }
}

/// Map projection for longitude/latitude input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoProjection {
    /// Plate carrée / equirectangular projection.
    Equirectangular,
    /// Spherical Web Mercator. Latitudes are clamped to the valid Web Mercator range.
    WebMercator,
}

impl GeoProjection {
    /// Project lon/lat into an unscaled planar coordinate pair.
    #[must_use]
    pub fn project(self, position: GeoPosition) -> [f32; 2] {
        match self {
            Self::Equirectangular => [position.lon, -position.lat],
            Self::WebMercator => {
                let lat = position.lat.clamp(-85.051_13, 85.051_13);
                let y = ((PI / 4.0) + (lat.to_radians() / 2.0)).tan().ln();
                [position.lon.to_radians(), -y]
            }
        }
    }
}
