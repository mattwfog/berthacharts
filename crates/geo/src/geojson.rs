//! Minimal GeoJSON reader for map specs.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::map::{GeoFeature, GeoGeometry, GeoPosition};

/// Options used when converting GeoJSON features into Bertha geo features.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoJsonReadOptions {
    /// Property name used as the feature label.
    pub label_property: Option<String>,
    /// Property name used as the choropleth value.
    pub value_property: Option<String>,
    /// Property name used as the feature group.
    pub group_property: Option<String>,
}

impl Default for GeoJsonReadOptions {
    fn default() -> Self {
        Self {
            label_property: Some("name".to_string()),
            value_property: None,
            group_property: None,
        }
    }
}

/// GeoJSON parse or conversion error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeoJsonError {
    /// Input was not valid JSON.
    InvalidJson(String),
    /// A required GeoJSON member was missing.
    MissingMember(&'static str),
    /// A GeoJSON type was not supported by this reader.
    UnsupportedType(String),
    /// Coordinates were malformed.
    InvalidCoordinates,
}

impl std::fmt::Display for GeoJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid GeoJSON: {message}"),
            Self::MissingMember(member) => write!(f, "GeoJSON is missing `{member}`"),
            Self::UnsupportedType(kind) => write!(f, "unsupported GeoJSON type `{kind}`"),
            Self::InvalidCoordinates => write!(f, "GeoJSON coordinates are malformed"),
        }
    }
}

impl std::error::Error for GeoJsonError {}

/// Parse a GeoJSON string into map features using default property mapping.
///
/// Supports `FeatureCollection`, `Feature`, and bare geometry objects for
/// Point/MultiPoint, LineString/MultiLineString, Polygon/MultiPolygon, and
/// GeometryCollection.
pub fn features_from_geojson_str(input: &str) -> Result<Vec<GeoFeature>, GeoJsonError> {
    features_from_geojson_str_with_options(input, &GeoJsonReadOptions::default())
}

/// Parse a GeoJSON string into map features with explicit property mapping.
pub fn features_from_geojson_str_with_options(
    input: &str,
    options: &GeoJsonReadOptions,
) -> Result<Vec<GeoFeature>, GeoJsonError> {
    let value: Value =
        serde_json::from_str(input).map_err(|err| GeoJsonError::InvalidJson(err.to_string()))?;
    parse_root(&value, options)
}

fn parse_root(
    value: &Value,
    options: &GeoJsonReadOptions,
) -> Result<Vec<GeoFeature>, GeoJsonError> {
    match type_name(value)? {
        "FeatureCollection" => value
            .get("features")
            .and_then(Value::as_array)
            .ok_or(GeoJsonError::MissingMember("features"))?
            .iter()
            .map(|feature| parse_feature(feature, options))
            .collect(),
        "Feature" => Ok(vec![parse_feature(value, options)?]),
        _ => Ok(vec![GeoFeature::new("geometry", parse_geometry(value)?)]),
    }
}

fn parse_feature(value: &Value, options: &GeoJsonReadOptions) -> Result<GeoFeature, GeoJsonError> {
    if type_name(value)? != "Feature" {
        return Err(GeoJsonError::UnsupportedType(type_name(value)?.to_string()));
    }
    let geometry = value
        .get("geometry")
        .ok_or(GeoJsonError::MissingMember("geometry"))
        .and_then(parse_geometry)?;
    let properties = value
        .get("properties")
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), property_to_string(value)))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let label = options
        .label_property
        .as_deref()
        .and_then(|key| properties.get(key).cloned())
        .or_else(|| properties.get("name").cloned())
        .or_else(|| properties.get("NAME").cloned())
        .or_else(|| value.get("id").map(property_to_string))
        .unwrap_or_else(|| "feature".to_string());
    let value_number = options
        .value_property
        .as_deref()
        .and_then(|key| properties.get(key))
        .and_then(|raw| raw.parse::<f32>().ok());
    let group = options
        .group_property
        .as_deref()
        .and_then(|key| properties.get(key).cloned())
        .unwrap_or_else(|| geometry.kind().to_string());

    Ok(GeoFeature {
        label,
        geometry,
        value: value_number,
        group,
        properties,
    })
}

fn parse_geometry(value: &Value) -> Result<GeoGeometry, GeoJsonError> {
    match type_name(value)? {
        "Point" => Ok(GeoGeometry::Point(position(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "MultiPoint" => Ok(GeoGeometry::MultiPoint(positions(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "LineString" => Ok(GeoGeometry::LineString(positions(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "MultiLineString" => Ok(GeoGeometry::MultiLineString(line_strings(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "Polygon" => Ok(GeoGeometry::Polygon(polygon(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "MultiPolygon" => Ok(GeoGeometry::MultiPolygon(polygons(
            value
                .get("coordinates")
                .ok_or(GeoJsonError::MissingMember("coordinates"))?,
        )?)),
        "GeometryCollection" => {
            let geometries = value
                .get("geometries")
                .and_then(Value::as_array)
                .ok_or(GeoJsonError::MissingMember("geometries"))?;
            Ok(GeoGeometry::GeometryCollection(
                geometries
                    .iter()
                    .map(parse_geometry)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        kind => Err(GeoJsonError::UnsupportedType(kind.to_string())),
    }
}

fn type_name(value: &Value) -> Result<&str, GeoJsonError> {
    value
        .get("type")
        .and_then(Value::as_str)
        .ok_or(GeoJsonError::MissingMember("type"))
}

fn position(value: &Value) -> Result<GeoPosition, GeoJsonError> {
    let coords = value.as_array().ok_or(GeoJsonError::InvalidCoordinates)?;
    let lon = coords
        .first()
        .and_then(Value::as_f64)
        .ok_or(GeoJsonError::InvalidCoordinates)? as f32;
    let lat = coords
        .get(1)
        .and_then(Value::as_f64)
        .ok_or(GeoJsonError::InvalidCoordinates)? as f32;
    if !lon.is_finite() || !lat.is_finite() {
        return Err(GeoJsonError::InvalidCoordinates);
    }
    Ok(GeoPosition { lon, lat })
}

fn positions(value: &Value) -> Result<Vec<GeoPosition>, GeoJsonError> {
    value
        .as_array()
        .ok_or(GeoJsonError::InvalidCoordinates)?
        .iter()
        .map(position)
        .collect()
}

fn line_strings(value: &Value) -> Result<Vec<Vec<GeoPosition>>, GeoJsonError> {
    value
        .as_array()
        .ok_or(GeoJsonError::InvalidCoordinates)?
        .iter()
        .map(positions)
        .collect()
}

fn polygon(value: &Value) -> Result<Vec<Vec<GeoPosition>>, GeoJsonError> {
    line_strings(value)
}

fn polygons(value: &Value) -> Result<Vec<Vec<Vec<GeoPosition>>>, GeoJsonError> {
    value
        .as_array()
        .ok_or(GeoJsonError::InvalidCoordinates)?
        .iter()
        .map(polygon)
        .collect()
}

fn property_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_feature_collection_with_property_mapping() {
        let input = r#"
        {
          "type": "FeatureCollection",
          "features": [{
            "type": "Feature",
            "properties": { "name": "North", "score": 42.5, "region": "A" },
            "geometry": { "type": "Point", "coordinates": [-73.9, 40.7] }
          }]
        }
        "#;
        let features = features_from_geojson_str_with_options(
            input,
            &GeoJsonReadOptions {
                label_property: Some("name".to_string()),
                value_property: Some("score".to_string()),
                group_property: Some("region".to_string()),
            },
        )
        .expect("valid GeoJSON");

        assert_eq!(features.len(), 1);
        assert_eq!(features[0].label, "North");
        assert_eq!(features[0].value, Some(42.5));
        assert_eq!(features[0].group, "A");
    }
}
