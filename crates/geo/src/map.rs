//! Geo map spec built on the core scene/mark contracts.

use std::collections::BTreeMap;
use std::sync::Arc;

use berthacharts_core::{
    CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset, DatasetId,
    Geometry, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind, LabelPriority, Layer, LayerId,
    LegendAnchor, LegendGuide, LegendItem, LinePrim, Mark, MarkId, PickCtx, PickHit, PointPrim,
    Rect, RectPrim, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx,
    TooltipField, TooltipGuide, TrianglePrim, Workspace,
};

use crate::projection::{GeoBounds, GeoProjection};

const DATASET: DatasetId = DatasetId::new(0);
const GEO_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One longitude/latitude coordinate in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPosition {
    /// Longitude in degrees.
    pub lon: f32,
    /// Latitude in degrees.
    pub lat: f32,
}

impl GeoPosition {
    /// Build a longitude/latitude coordinate.
    #[must_use]
    pub const fn new(lon: f32, lat: f32) -> Self {
        Self { lon, lat }
    }
}

/// Geometry accepted by geospatial specs.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeoGeometry {
    /// Single point.
    Point(GeoPosition),
    /// Multiple points.
    MultiPoint(Vec<GeoPosition>),
    /// Single line string.
    LineString(Vec<GeoPosition>),
    /// Multiple line strings.
    MultiLineString(Vec<Vec<GeoPosition>>),
    /// Polygon rings. Ring 0 is the exterior; later rings are holes.
    Polygon(Vec<Vec<GeoPosition>>),
    /// Multiple polygons.
    MultiPolygon(Vec<Vec<Vec<GeoPosition>>>),
    /// Geometry collection.
    GeometryCollection(Vec<GeoGeometry>),
}

impl GeoGeometry {
    /// Human-readable geometry kind.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Point(_) | Self::MultiPoint(_) => "point",
            Self::LineString(_) | Self::MultiLineString(_) => "line",
            Self::Polygon(_) | Self::MultiPolygon(_) => "polygon",
            Self::GeometryCollection(_) => "collection",
        }
    }

    /// Visit every coordinate in this geometry.
    pub fn visit_positions(&self, visitor: &mut impl FnMut(GeoPosition)) {
        match self {
            Self::Point(position) => visitor(*position),
            Self::MultiPoint(points) | Self::LineString(points) => {
                for position in points {
                    visitor(*position);
                }
            }
            Self::MultiLineString(lines) | Self::Polygon(lines) => {
                for line in lines {
                    for position in line {
                        visitor(*position);
                    }
                }
            }
            Self::MultiPolygon(polygons) => {
                for polygon in polygons {
                    for ring in polygon {
                        for position in ring {
                            visitor(*position);
                        }
                    }
                }
            }
            Self::GeometryCollection(geometries) => {
                for geometry in geometries {
                    geometry.visit_positions(visitor);
                }
            }
        }
    }
}

/// One feature on a map.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoFeature {
    /// Display label.
    pub label: String,
    /// Feature geometry.
    pub geometry: GeoGeometry,
    /// Optional choropleth value.
    pub value: Option<f32>,
    /// Optional grouping/category name.
    pub group: String,
    /// Source properties, typically from GeoJSON.
    pub properties: BTreeMap<String, String>,
}

impl GeoFeature {
    /// Build a feature.
    #[must_use]
    pub fn new(label: impl Into<String>, geometry: GeoGeometry) -> Self {
        Self {
            label: label.into(),
            group: geometry.kind().to_string(),
            geometry,
            value: None,
            properties: BTreeMap::new(),
        }
    }

    /// Set a choropleth value.
    #[must_use]
    pub const fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Set a group/category.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }
}

/// Layout and styling options for a geo map.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoMapOptions {
    /// Left plot padding.
    pub padding_left: f32,
    /// Right plot padding.
    pub padding_right: f32,
    /// Top plot padding.
    pub padding_top: f32,
    /// Bottom plot padding.
    pub padding_bottom: f32,
    /// Projection for lon/lat data.
    pub projection: GeoProjection,
    /// Optional fixed geographic bounds.
    pub bounds: Option<GeoBounds>,
    /// Optional fixed choropleth domain.
    pub value_domain: Option<(f32, f32)>,
    /// Legend title for choropleth values.
    pub legend_title: String,
    /// Whether to emit a choropleth legend when values are present.
    pub show_legend: bool,
    /// Number of legend swatches for continuous choropleth values.
    pub legend_steps: usize,
    /// Preferred legend placement.
    pub legend_anchor: LegendAnchor,
    /// Lowest-value choropleth color.
    pub low_color: [f32; 4],
    /// Highest-value choropleth color.
    pub high_color: [f32; 4],
    /// Plot background behind the projected map.
    pub background_fill: [f32; 4],
    /// Fill used to mask interior polygon rings.
    pub hole_fill: [f32; 4],
    /// Whether to draw projected latitude/longitude reference lines.
    pub show_graticule: bool,
    /// Longitude spacing for reference lines.
    pub graticule_lon_step: f32,
    /// Latitude spacing for reference lines.
    pub graticule_lat_step: f32,
    /// Graticule stroke.
    pub graticule_color: [f32; 4],
    /// Graticule stroke width.
    pub graticule_width: f32,
    /// Screen-space simplification tolerance in pixels. `0` disables simplification.
    pub simplify_tolerance: f32,
    /// Fill for features without values.
    pub default_fill: [f32; 4],
    /// Categorical fallback colors for features without values.
    pub categorical_palette: Vec<[f32; 4]>,
    /// Boundary stroke.
    pub stroke: [f32; 4],
    /// Boundary width.
    pub stroke_width: f32,
    /// Radius for point features.
    pub point_radius: f32,
    /// Optional value-scaled point radius range.
    pub point_radius_range: Option<(f32, f32)>,
    /// Optional value-scaled line width range.
    pub line_width_range: Option<(f32, f32)>,
    /// Maximum labels emitted to the DOM overlay.
    pub max_labels: usize,
}

impl Default for GeoMapOptions {
    fn default() -> Self {
        Self {
            padding_left: 18.0,
            padding_right: 18.0,
            padding_top: 18.0,
            padding_bottom: 26.0,
            projection: GeoProjection::Equirectangular,
            bounds: None,
            value_domain: None,
            legend_title: "Value".to_string(),
            show_legend: true,
            legend_steps: 5,
            legend_anchor: LegendAnchor::Bottom,
            low_color: rgba(0.83, 0.91, 0.91, 0.92),
            high_color: rgba(0.04, 0.42, 0.50, 0.95),
            background_fill: rgba(0.96, 0.98, 0.99, 1.0),
            hole_fill: rgba(0.96, 0.98, 0.99, 1.0),
            show_graticule: true,
            graticule_lon_step: 0.10,
            graticule_lat_step: 0.10,
            graticule_color: rgba(0.42, 0.55, 0.66, 0.18),
            graticule_width: 0.75,
            simplify_tolerance: 0.25,
            default_fill: rgba(0.78, 0.82, 0.86, 0.72),
            categorical_palette: vec![
                rgba(0.35, 0.50, 0.70, 0.78),
                rgba(0.18, 0.56, 0.48, 0.78),
                rgba(0.76, 0.45, 0.24, 0.76),
                rgba(0.57, 0.43, 0.72, 0.76),
                rgba(0.78, 0.34, 0.34, 0.74),
                rgba(0.26, 0.36, 0.46, 0.74),
            ],
            stroke: rgba(1.0, 1.0, 1.0, 0.92),
            stroke_width: 1.15,
            point_radius: 5.5,
            point_radius_range: Some((4.5, 10.5)),
            line_width_range: Some((2.0, 5.5)),
            max_labels: 12,
        }
    }
}

/// Reusable geospatial map specification.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoMapSpec {
    /// Features in author order.
    pub features: Vec<GeoFeature>,
    /// Layout and style options.
    pub options: GeoMapOptions,
}

impl GeoMapSpec {
    /// Build a geo map spec.
    #[must_use]
    pub fn new(features: Vec<GeoFeature>) -> Self {
        Self {
            features,
            options: GeoMapOptions::default(),
        }
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: GeoMapOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute headline summary values.
    #[must_use]
    pub fn summary(&self) -> GeoMapSummary {
        let mut points = 0;
        let mut lines = 0;
        let mut polygons = 0;
        let mut groups = Vec::new();
        let mut value_min = f32::INFINITY;
        let mut value_max = f32::NEG_INFINITY;
        let mut total_area_km2 = 0.0;
        let mut total_length_km = 0.0;
        for feature in &self.features {
            count_geometry(&feature.geometry, &mut points, &mut lines, &mut polygons);
            total_area_km2 += geometry_area_km2(&feature.geometry);
            total_length_km += geometry_length_km(&feature.geometry);
            if !groups.contains(&feature.group) {
                groups.push(feature.group.clone());
            }
            if let Some(value) = feature.value {
                value_min = value_min.min(value);
                value_max = value_max.max(value);
            }
        }
        GeoMapSummary {
            features: self.features.len(),
            points,
            lines,
            polygons,
            groups: groups.len(),
            value_min: value_min.is_finite().then_some(value_min),
            value_max: value_max.is_finite().then_some(value_max),
            total_area_km2,
            total_length_km,
            bounds: GeoBounds::from_geometries(
                self.features.iter().map(|feature| &feature.geometry),
            ),
        }
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, GeoMapError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), GeoMapError> {
        if self.features.is_empty() {
            return Err(GeoMapError::EmptyData);
        }
        for feature in &self.features {
            if feature.label.trim().is_empty() {
                return Err(GeoMapError::EmptyLabel);
            }
            if !geometry_is_finite(&feature.geometry)
                || feature.value.is_some_and(|value| !value.is_finite())
            {
                return Err(GeoMapError::InvalidFeature {
                    label: feature.label.clone(),
                });
            }
        }
        Ok(())
    }

    fn plot_area(&self, size: ChartSize) -> Rect {
        let width = size.width as f32;
        let height = size.height as f32;
        let left = self.options.padding_left.clamp(0.0, width - 1.0);
        let top = self.options.padding_top.clamp(0.0, height - 1.0);
        let right = self.options.padding_right.clamp(0.0, width - left - 1.0);
        let bottom = self.options.padding_bottom.clamp(0.0, height - top - 1.0);
        Rect::new(
            left,
            top,
            (width - left - right).max(1.0),
            (height - top - bottom).max(1.0),
        )
    }
}

/// Summary values for a map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoMapSummary {
    /// Number of features.
    pub features: usize,
    /// Number of point geometries.
    pub points: usize,
    /// Number of line geometries.
    pub lines: usize,
    /// Number of polygon geometries.
    pub polygons: usize,
    /// Number of feature groups.
    pub groups: usize,
    /// Minimum choropleth value, if any values were supplied.
    pub value_min: Option<f32>,
    /// Maximum choropleth value, if any values were supplied.
    pub value_max: Option<f32>,
    /// Approximate total polygon area in square kilometers.
    pub total_area_km2: f32,
    /// Approximate total line/ring length in kilometers.
    pub total_length_km: f32,
    /// Geographic bounds, if any coordinates were present.
    pub bounds: Option<GeoBounds>,
}

/// Error building a geo map.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeoMapError {
    /// No features were supplied.
    EmptyData,
    /// A feature label was empty.
    EmptyLabel,
    /// A feature had non-finite coordinates or value.
    InvalidFeature {
        /// Feature label.
        label: String,
    },
    /// No coordinates could be projected.
    EmptyGeometry,
}

impl std::fmt::Display for GeoMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyData => write!(f, "geo map requires at least one feature"),
            Self::EmptyLabel => write!(f, "geo feature labels cannot be empty"),
            Self::InvalidFeature { label } => write!(f, "geo feature `{label}` is invalid"),
            Self::EmptyGeometry => write!(f, "geo map has no coordinates to project"),
        }
    }
}

impl std::error::Error for GeoMapError {}

impl ChartSpec for GeoMapSpec {
    type Error = GeoMapError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let plot = self.plot_area(size);
        let projected = ProjectedMap::build(self, plot)?;
        let legend = projected.legend.clone();

        workspace.upsert_scale(
            X_SCALE,
            Arc::new(berthacharts_core::LinearScale::new((0.0, 1.0), (0.0, 1.0))) as Arc<dyn Scale>,
        );
        workspace.upsert_scale(
            Y_SCALE,
            Arc::new(berthacharts_core::LinearScale::new((0.0, 1.0), (0.0, 1.0))) as Arc<dyn Scale>,
        );
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(geo_dataset(
            &self.features,
            &self.options,
            projected.value_domain,
            &group_order(&self.features),
        ));

        let mut scene = Scene::new(size.viewport_with_plot_area(plot));
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![Arc::new(GeoMapMark::new(
                GEO_MARK,
                projected.geometry,
                projected.hits,
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
            ))],
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                GEO_MARK,
                DATASET,
                vec![
                    TooltipField::new("Group", "group").as_label(),
                    TooltipField::new("Geometry", "kind").as_label(),
                    TooltipField::new("Value", "value").as_number(1),
                    TooltipField::new("Symbol", "symbol_radius").as_number(1),
                    TooltipField::new("Line", "line_width").as_number(1),
                    TooltipField::new("Area km2", "area_km2").as_number(1),
                    TooltipField::new("Length km", "length_km").as_number(1),
                    TooltipField::new("Longitude", "longitude").as_number(3),
                    TooltipField::new("Latitude", "latitude").as_number(3),
                ],
            )
            .with_title_column("label"),
        ));
        if let Some(legend) = legend {
            scene.guides.push(Guide::Legend(legend));
        }
        let labels: Vec<LabelItem> = projected
            .labels
            .into_iter()
            .take(self.options.max_labels)
            .collect();
        let label_count = labels.len();
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(4.0)
                .with_max_visible(label_count),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(projected.snap_targets).with_name("geo features"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

#[derive(Debug, Clone)]
struct GeoMapMark {
    id: MarkId,
    geometry: Geometry,
    hits: Vec<FeatureHit>,
    bounds: Rect,
}

impl GeoMapMark {
    fn new(id: MarkId, geometry: Geometry, hits: Vec<FeatureHit>, bounds: Rect) -> Self {
        Self {
            id,
            geometry,
            hits,
            bounds,
        }
    }
}

impl Mark for GeoMapMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.hits.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        self.geometry.clone()
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        self.hits
            .iter()
            .rev()
            .filter_map(|hit| {
                hit.pick(point)
                    .map(|distance| (hit.feature_index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(feature_index, distance)| PickHit {
                mark: self.id,
                row: Some(feature_index),
                distance,
                payload: None,
            })
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.bounds
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
struct ProjectedMap {
    geometry: Geometry,
    hits: Vec<FeatureHit>,
    labels: Vec<LabelItem>,
    snap_targets: Vec<SnapTarget>,
    legend: Option<LegendGuide>,
    value_domain: (f32, f32),
}

impl ProjectedMap {
    fn build(spec: &GeoMapSpec, plot: Rect) -> Result<Self, GeoMapError> {
        let raw_bounds = RawBounds::build(spec)?;
        let value_domain = value_domain(spec);
        let groups = group_order(&spec.features);
        let transformer = ScreenTransform::new(raw_bounds, plot);

        let mut fills = Vec::new();
        let mut hole_masks = Vec::new();
        let mut graticule = Vec::new();
        let mut boundaries = Vec::new();
        let mut points = Vec::new();
        let mut hits = Vec::new();
        let mut labels = Vec::new();
        let mut snap_targets = Vec::new();
        if spec.options.show_graticule {
            graticule.extend(graticule_lines(spec, &transformer));
        }

        for (feature_index, feature) in spec.features.iter().enumerate() {
            let style = feature_style(feature, spec, value_domain, &groups);
            let mut centroids = Vec::new();
            project_geometry(
                &feature.geometry,
                feature_index,
                style,
                spec.options.hole_fill,
                spec.options.simplify_tolerance,
                spec.options.projection,
                &transformer,
                &mut fills,
                &mut hole_masks,
                &mut boundaries,
                &mut points,
                &mut hits,
                &mut centroids,
            );
            if let Some([x, y]) = average_points(&centroids) {
                labels.push(
                    LabelItem::new(x, y, feature.label.clone())
                        .with_detail(feature.value.map_or_else(
                            || feature.group.clone(),
                            |value| format!("{} · {value:.1}", feature.group),
                        ))
                        .with_kind(LabelKind::Data)
                        .with_priority(label_priority(feature.value, value_domain))
                        .with_anchor(LabelAnchor::Top),
                );
                snap_targets.push(
                    SnapTarget::new(x, y, SnapKind::Point)
                        .with_radius(8.0)
                        .with_label(feature.label.clone())
                        .with_priority(1),
                );
            }
        }

        Ok(Self {
            geometry: Geometry::Mixed(vec![
                Geometry::Rects(vec![RectPrim {
                    x: plot.x,
                    y: plot.y,
                    w: plot.w,
                    h: plot.h,
                    fill: spec.options.background_fill,
                    stroke: rgba(0.42, 0.55, 0.66, 0.16),
                    stroke_width: 1.0,
                    radius: 0.0,
                }]),
                Geometry::Lines(graticule),
                Geometry::Triangles(fills),
                Geometry::Triangles(hole_masks),
                Geometry::Lines(boundaries),
                Geometry::Points(points),
            ]),
            hits,
            labels,
            snap_targets,
            legend: choropleth_legend(spec, value_domain),
            value_domain,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct GeoVisualStyle {
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
    point_radius: f32,
    point_shape: u32,
    line_width: f32,
}

#[derive(Debug, Clone, Copy)]
struct RawBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl RawBounds {
    fn build(spec: &GeoMapSpec) -> Result<Self, GeoMapError> {
        let mut bounds = RawBounds {
            min_x: f32::INFINITY,
            min_y: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            max_y: f32::NEG_INFINITY,
        };
        if let Some(geo_bounds) = spec.options.bounds {
            for position in [
                GeoPosition::new(geo_bounds.min_lon, geo_bounds.min_lat),
                GeoPosition::new(geo_bounds.max_lon, geo_bounds.max_lat),
            ] {
                bounds.push(spec.options.projection.project(position));
            }
        } else {
            for feature in &spec.features {
                feature.geometry.visit_positions(&mut |position| {
                    bounds.push(spec.options.projection.project(position))
                });
            }
        }
        bounds
            .is_valid()
            .then_some(bounds)
            .ok_or(GeoMapError::EmptyGeometry)
    }

    fn push(&mut self, point: [f32; 2]) {
        self.min_x = self.min_x.min(point[0]);
        self.max_x = self.max_x.max(point[0]);
        self.min_y = self.min_y.min(point[1]);
        self.max_y = self.max_y.max(point[1]);
    }

    fn is_valid(self) -> bool {
        self.min_x.is_finite()
            && self.max_x.is_finite()
            && self.min_y.is_finite()
            && self.max_y.is_finite()
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenTransform {
    bounds: RawBounds,
    scale: f32,
    x_offset: f32,
    y_offset: f32,
}

impl ScreenTransform {
    fn new(bounds: RawBounds, plot: Rect) -> Self {
        let raw_w = (bounds.max_x - bounds.min_x).abs().max(1.0e-6);
        let raw_h = (bounds.max_y - bounds.min_y).abs().max(1.0e-6);
        let scale = (plot.w / raw_w).min(plot.h / raw_h);
        let x_offset = plot.x + (plot.w - raw_w * scale) * 0.5;
        let y_offset = plot.y + (plot.h - raw_h * scale) * 0.5;
        Self {
            bounds,
            scale,
            x_offset,
            y_offset,
        }
    }

    fn screen(self, point: [f32; 2]) -> [f32; 2] {
        [
            self.x_offset + (point[0] - self.bounds.min_x) * self.scale,
            self.y_offset + (point[1] - self.bounds.min_y) * self.scale,
        ]
    }
}

#[derive(Debug, Clone)]
struct FeatureHit {
    feature_index: usize,
    shape: HitShape,
}

impl FeatureHit {
    fn pick(&self, point: (f32, f32)) -> Option<f32> {
        match &self.shape {
            HitShape::Point { center, radius } => {
                let distance = distance(point, (center[0], center[1]));
                (distance <= radius + 4.0).then_some(distance)
            }
            HitShape::Line { points, width } => {
                distance_to_polyline(point, points).filter(|distance| *distance <= width + 4.0)
            }
            HitShape::Polygon { rings } => rings.first().and_then(|outer| {
                let inside_outer = point_in_ring(point, outer);
                let inside_hole = rings.iter().skip(1).any(|ring| point_in_ring(point, ring));
                (inside_outer && !inside_hole).then_some(0.0)
            }),
        }
    }
}

#[derive(Debug, Clone)]
enum HitShape {
    Point { center: [f32; 2], radius: f32 },
    Line { points: Vec<[f32; 2]>, width: f32 },
    Polygon { rings: Vec<Vec<[f32; 2]>> },
}

#[allow(clippy::too_many_arguments)]
fn project_geometry(
    geometry: &GeoGeometry,
    feature_index: usize,
    style: GeoVisualStyle,
    hole_fill: [f32; 4],
    simplify_tolerance: f32,
    projection: GeoProjection,
    transform: &ScreenTransform,
    fills: &mut Vec<TrianglePrim>,
    hole_masks: &mut Vec<TrianglePrim>,
    boundaries: &mut Vec<LinePrim>,
    points: &mut Vec<PointPrim>,
    hits: &mut Vec<FeatureHit>,
    centroids: &mut Vec<[f32; 2]>,
) {
    match geometry {
        GeoGeometry::Point(position) => {
            let point = transform.screen(projection.project(*position));
            points.push(PointPrim {
                x: point[0],
                y: point[1],
                r: style.point_radius,
                shape: style.point_shape,
                fill: style.fill,
                stroke: style.stroke,
                stroke_width: style.stroke_width,
            });
            hits.push(FeatureHit {
                feature_index,
                shape: HitShape::Point {
                    center: point,
                    radius: style.point_radius,
                },
            });
            centroids.push(point);
        }
        GeoGeometry::MultiPoint(positions) => {
            for position in positions {
                project_geometry(
                    &GeoGeometry::Point(*position),
                    feature_index,
                    style,
                    hole_fill,
                    simplify_tolerance,
                    projection,
                    transform,
                    fills,
                    hole_masks,
                    boundaries,
                    points,
                    hits,
                    centroids,
                );
            }
        }
        GeoGeometry::LineString(positions) => {
            let screen_points = project_line(positions, projection, transform, simplify_tolerance);
            if screen_points.len() >= 2 {
                boundaries.push(LinePrim {
                    points: screen_points.clone(),
                    stroke: style.fill,
                    width: style.line_width,
                    dash: None,
                    join: 1,
                    cap: 1,
                });
                hits.push(FeatureHit {
                    feature_index,
                    shape: HitShape::Line {
                        points: screen_points.clone(),
                        width: style.line_width,
                    },
                });
                centroids.extend(screen_points);
            }
        }
        GeoGeometry::MultiLineString(line_strings) => {
            for positions in line_strings {
                project_geometry(
                    &GeoGeometry::LineString(positions.clone()),
                    feature_index,
                    style,
                    hole_fill,
                    simplify_tolerance,
                    projection,
                    transform,
                    fills,
                    hole_masks,
                    boundaries,
                    points,
                    hits,
                    centroids,
                );
            }
        }
        GeoGeometry::Polygon(rings) => {
            let screen_rings = project_rings(rings, projection, transform, simplify_tolerance);
            if let Some(outer) = screen_rings.first() {
                fills.extend(triangulate_ring(outer, style.fill));
                for hole in screen_rings.iter().skip(1) {
                    hole_masks.extend(triangulate_ring(hole, hole_fill));
                }
                for ring in &screen_rings {
                    if ring.len() >= 2 {
                        boundaries.push(LinePrim {
                            points: closed_ring(ring),
                            stroke: style.stroke,
                            width: style.stroke_width,
                            dash: None,
                            join: 1,
                            cap: 1,
                        });
                    }
                }
                hits.push(FeatureHit {
                    feature_index,
                    shape: HitShape::Polygon {
                        rings: screen_rings.clone(),
                    },
                });
                if let Some(center) = polygon_centroid(outer) {
                    centroids.push(center);
                }
            }
        }
        GeoGeometry::MultiPolygon(polygons) => {
            for rings in polygons {
                project_geometry(
                    &GeoGeometry::Polygon(rings.clone()),
                    feature_index,
                    style,
                    hole_fill,
                    simplify_tolerance,
                    projection,
                    transform,
                    fills,
                    hole_masks,
                    boundaries,
                    points,
                    hits,
                    centroids,
                );
            }
        }
        GeoGeometry::GeometryCollection(geometries) => {
            for geometry in geometries {
                project_geometry(
                    geometry,
                    feature_index,
                    style,
                    hole_fill,
                    simplify_tolerance,
                    projection,
                    transform,
                    fills,
                    hole_masks,
                    boundaries,
                    points,
                    hits,
                    centroids,
                );
            }
        }
    }
}

fn project_line(
    positions: &[GeoPosition],
    projection: GeoProjection,
    transform: &ScreenTransform,
    simplify_tolerance: f32,
) -> Vec<[f32; 2]> {
    let points: Vec<[f32; 2]> = positions
        .iter()
        .map(|position| transform.screen(projection.project(*position)))
        .collect();
    simplify_polyline(&points, simplify_tolerance)
}

fn graticule_lines(spec: &GeoMapSpec, transform: &ScreenTransform) -> Vec<LinePrim> {
    let Some(bounds) = spec.options.bounds.or_else(|| {
        GeoBounds::from_geometries(spec.features.iter().map(|feature| &feature.geometry))
    }) else {
        return Vec::new();
    };
    let lon_step = spec.options.graticule_lon_step.abs().max(0.01);
    let lat_step = spec.options.graticule_lat_step.abs().max(0.01);
    let mut lines = Vec::new();
    let mut lon = (bounds.min_lon / lon_step).floor() * lon_step;
    while lon <= bounds.max_lon + lon_step * 0.5 {
        let points = sample_graticule_line(
            |t| GeoPosition::new(lon, bounds.min_lat + bounds.lat_span() * t),
            spec.options.projection,
            transform,
        );
        lines.push(LinePrim {
            points,
            stroke: spec.options.graticule_color,
            width: spec.options.graticule_width,
            dash: Some(vec![3.0, 5.0]),
            join: 1,
            cap: 1,
        });
        lon += lon_step;
    }
    let mut lat = (bounds.min_lat / lat_step).floor() * lat_step;
    while lat <= bounds.max_lat + lat_step * 0.5 {
        let points = sample_graticule_line(
            |t| GeoPosition::new(bounds.min_lon + bounds.lon_span() * t, lat),
            spec.options.projection,
            transform,
        );
        lines.push(LinePrim {
            points,
            stroke: spec.options.graticule_color,
            width: spec.options.graticule_width,
            dash: Some(vec![3.0, 5.0]),
            join: 1,
            cap: 1,
        });
        lat += lat_step;
    }
    lines
}

fn sample_graticule_line(
    mut position: impl FnMut(f32) -> GeoPosition,
    projection: GeoProjection,
    transform: &ScreenTransform,
) -> Vec<[f32; 2]> {
    (0..=24)
        .map(|index| {
            let t = index as f32 / 24.0;
            transform.screen(projection.project(position(t)))
        })
        .collect()
}

fn project_rings(
    rings: &[Vec<GeoPosition>],
    projection: GeoProjection,
    transform: &ScreenTransform,
    simplify_tolerance: f32,
) -> Vec<Vec<[f32; 2]>> {
    rings
        .iter()
        .map(|ring| {
            let projected: Vec<[f32; 2]> = ring
                .iter()
                .map(|position| transform.screen(projection.project(*position)))
                .collect();
            simplify_ring(&projected, simplify_tolerance)
        })
        .filter(|ring| ring.len() >= 3)
        .collect()
}

fn closed_ring(ring: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut points = normalized_ring(ring);
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    points
}

fn normalized_ring(ring: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut points = ring.to_vec();
    if points.len() >= 2 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn simplify_ring(ring: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    let points = normalized_ring(ring);
    let simplified = simplify_polyline(&points, tolerance);
    if simplified.len() >= 3 {
        simplified
    } else {
        points
    }
}

fn simplify_polyline(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if tolerance <= 0.0 || points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    simplify_range(
        points,
        0,
        points.len() - 1,
        tolerance * tolerance,
        &mut keep,
    );
    points
        .iter()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(*point))
        .collect()
}

fn simplify_range(
    points: &[[f32; 2]],
    start: usize,
    end: usize,
    tolerance2: f32,
    keep: &mut [bool],
) {
    if end <= start + 1 {
        return;
    }
    let mut max_distance2 = 0.0;
    let mut max_index = start;
    for index in start + 1..end {
        let distance2 = distance_to_segment_squared(points[index], points[start], points[end]);
        if distance2 > max_distance2 {
            max_distance2 = distance2;
            max_index = index;
        }
    }
    if max_distance2 > tolerance2 {
        keep[max_index] = true;
        simplify_range(points, start, max_index, tolerance2, keep);
        simplify_range(points, max_index, end, tolerance2, keep);
    }
}

fn triangulate_ring(ring: &[[f32; 2]], fill: [f32; 4]) -> Vec<TrianglePrim> {
    let points = normalized_ring(ring);
    if points.len() < 3 {
        return Vec::new();
    }
    let clockwise = signed_area(&points) < 0.0;
    let mut indices: Vec<usize> = (0..points.len()).collect();
    let mut triangles = Vec::new();
    let mut guard = 0;

    while indices.len() > 3 && guard < points.len() * points.len() {
        guard += 1;
        let mut clipped = false;
        for i in 0..indices.len() {
            let prev = indices[(i + indices.len() - 1) % indices.len()];
            let curr = indices[i];
            let next = indices[(i + 1) % indices.len()];
            if !is_convex(points[prev], points[curr], points[next], clockwise) {
                continue;
            }
            let contains_point = indices.iter().copied().any(|candidate| {
                candidate != prev
                    && candidate != curr
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[prev],
                        points[curr],
                        points[next],
                    )
            });
            if contains_point {
                continue;
            }
            triangles.push(TrianglePrim {
                a: points[prev],
                b: points[curr],
                c: points[next],
                fill,
            });
            indices.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if indices.len() == 3 {
        triangles.push(TrianglePrim {
            a: points[indices[0]],
            b: points[indices[1]],
            c: points[indices[2]],
            fill,
        });
    }
    triangles
}

fn is_convex(a: [f32; 2], b: [f32; 2], c: [f32; 2], clockwise: bool) -> bool {
    let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if clockwise {
        cross < 0.0
    } else {
        cross > 0.0
    }
}

fn point_in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

fn sign(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
}

fn signed_area(points: &[[f32; 2]]) -> f32 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let next = (i + 1) % points.len();
        area += points[i][0] * points[next][1] - points[next][0] * points[i][1];
    }
    area * 0.5
}

fn polygon_centroid(points: &[[f32; 2]]) -> Option<[f32; 2]> {
    let points = normalized_ring(points);
    if points.is_empty() {
        return None;
    }
    let area = signed_area(&points);
    if area.abs() < f32::EPSILON {
        return average_points(&points);
    }
    let mut x = 0.0;
    let mut y = 0.0;
    for i in 0..points.len() {
        let next = (i + 1) % points.len();
        let cross = points[i][0] * points[next][1] - points[next][0] * points[i][1];
        x += (points[i][0] + points[next][0]) * cross;
        y += (points[i][1] + points[next][1]) * cross;
    }
    Some([x / (6.0 * area), y / (6.0 * area)])
}

fn average_points(points: &[[f32; 2]]) -> Option<[f32; 2]> {
    if points.is_empty() {
        return None;
    }
    let (x, y) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), point| (x + point[0], y + point[1]));
    Some([x / points.len() as f32, y / points.len() as f32])
}

fn point_in_ring(point: (f32, f32), ring: &[[f32; 2]]) -> bool {
    let ring = normalized_ring(ring);
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let yi = ring[i][1];
        let yj = ring[j][1];
        let intersects = (yi > point.1) != (yj > point.1)
            && point.0 < (ring[j][0] - ring[i][0]) * (point.1 - yi) / (yj - yi) + ring[i][0];
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn distance_to_polyline(point: (f32, f32), line: &[[f32; 2]]) -> Option<f32> {
    line.windows(2)
        .map(|segment| distance_to_segment(point, segment[0], segment[1]))
        .min_by(f32::total_cmp)
}

fn distance_to_segment(point: (f32, f32), a: [f32; 2], b: [f32; 2]) -> f32 {
    distance_to_segment_squared([point.0, point.1], a, b).sqrt()
}

fn distance_to_segment_squared(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let px = point[0];
    let py = point[1];
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        let dx = px - a[0];
        let dy = py - a[1];
        return dx * dx + dy * dy;
    }
    let t = (((px - a[0]) * dx + (py - a[1]) * dy) / len2).clamp(0.0, 1.0);
    let sx = a[0] + t * dx;
    let sy = a[1] + t * dy;
    let dx = px - sx;
    let dy = py - sy;
    dx * dx + dy * dy
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

fn label_priority(value: Option<f32>, domain: (f32, f32)) -> LabelPriority {
    let Some(value) = value else {
        return LabelPriority::Optional;
    };
    if normalized(value, domain) >= 0.72 {
        LabelPriority::Required
    } else {
        LabelPriority::Important
    }
}

fn choropleth_legend(spec: &GeoMapSpec, domain: (f32, f32)) -> Option<LegendGuide> {
    if !spec.options.show_legend || !spec.features.iter().any(|feature| feature.value.is_some()) {
        return None;
    }
    let steps = spec.options.legend_steps.clamp(2, 9);
    let mut items = Vec::with_capacity(steps);
    for index in 0..steps {
        let t = if steps == 1 {
            0.0
        } else {
            index as f32 / (steps - 1) as f32
        };
        let value = domain.0 + (domain.1 - domain.0) * t;
        items.push(LegendItem::new(
            format_legend_value(value),
            interpolate_color(spec.options.low_color, spec.options.high_color, t),
        ));
    }
    Some(
        LegendGuide::new(items)
            .with_title(spec.options.legend_title.clone())
            .with_anchor(spec.options.legend_anchor),
    )
}

fn format_legend_value(value: f32) -> String {
    if value.abs() >= 100.0 || value.fract().abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn value_domain(spec: &GeoMapSpec) -> (f32, f32) {
    if let Some((lo, hi)) = spec.options.value_domain {
        if lo.is_finite() && hi.is_finite() {
            return if lo <= hi { (lo, hi) } else { (hi, lo) };
        }
    }

    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for value in spec.features.iter().filter_map(|feature| feature.value) {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    if lo.is_finite() && hi.is_finite() {
        (lo, hi)
    } else {
        (0.0, 1.0)
    }
}

fn group_order(features: &[GeoFeature]) -> Vec<String> {
    let mut groups = Vec::new();
    for feature in features {
        if !groups.contains(&feature.group) {
            groups.push(feature.group.clone());
        }
    }
    groups
}

fn feature_style(
    feature: &GeoFeature,
    spec: &GeoMapSpec,
    domain: (f32, f32),
    groups: &[String],
) -> GeoVisualStyle {
    let t = feature.value.map(|value| normalized(value, domain));
    let group_index = index_of(groups, &feature.group).unwrap_or(0);
    let fill = t
        .map(|value| interpolate_color(spec.options.low_color, spec.options.high_color, value))
        .unwrap_or_else(|| categorical_color(&spec.options, group_index));
    GeoVisualStyle {
        fill,
        stroke: spec.options.stroke,
        stroke_width: spec.options.stroke_width,
        point_radius: scaled_or_default(
            t,
            spec.options.point_radius_range,
            spec.options.point_radius,
        ),
        point_shape: group_index as u32 % 4,
        line_width: scaled_or_default(
            t,
            spec.options.line_width_range,
            spec.options.stroke_width.max(2.0),
        ),
    }
}

fn scaled_or_default(t: Option<f32>, range: Option<(f32, f32)>, default: f32) -> f32 {
    let Some((lo, hi)) = range else {
        return default;
    };
    let Some(t) = t else {
        return default;
    };
    lo + (hi - lo) * t
}

fn categorical_color(options: &GeoMapOptions, index: usize) -> [f32; 4] {
    options
        .categorical_palette
        .get(index % options.categorical_palette.len().max(1))
        .copied()
        .unwrap_or(options.default_fill)
}

fn index_of(items: &[String], value: &str) -> Option<usize> {
    items.iter().position(|item| item == value)
}

fn geometry_center(geometry: &GeoGeometry) -> Option<GeoPosition> {
    let mut lon = 0.0;
    let mut lat = 0.0;
    let mut count = 0usize;
    geometry.visit_positions(&mut |position| {
        lon += position.lon;
        lat += position.lat;
        count += 1;
    });
    (count > 0).then_some(GeoPosition::new(lon / count as f32, lat / count as f32))
}

fn geometry_length_km(geometry: &GeoGeometry) -> f32 {
    match geometry {
        GeoGeometry::Point(_) | GeoGeometry::MultiPoint(_) => 0.0,
        GeoGeometry::LineString(line) => line_length_km(line, false),
        GeoGeometry::MultiLineString(lines) => {
            lines.iter().map(|line| line_length_km(line, false)).sum()
        }
        GeoGeometry::Polygon(rings) => rings.iter().map(|ring| line_length_km(ring, true)).sum(),
        GeoGeometry::MultiPolygon(polygons) => polygons
            .iter()
            .flat_map(|polygon| polygon.iter())
            .map(|ring| line_length_km(ring, true))
            .sum(),
        GeoGeometry::GeometryCollection(geometries) => {
            geometries.iter().map(geometry_length_km).sum()
        }
    }
}

fn geometry_area_km2(geometry: &GeoGeometry) -> f32 {
    match geometry {
        GeoGeometry::Polygon(rings) => polygon_area_km2(rings),
        GeoGeometry::MultiPolygon(polygons) => polygons
            .iter()
            .map(|polygon| polygon_area_km2(polygon))
            .sum(),
        GeoGeometry::GeometryCollection(geometries) => {
            geometries.iter().map(geometry_area_km2).sum()
        }
        _ => 0.0,
    }
}

fn polygon_area_km2(rings: &[Vec<GeoPosition>]) -> f32 {
    let Some(outer) = rings.first() else {
        return 0.0;
    };
    let reference_lat = average_latitude(outer).unwrap_or(0.0);
    let outer_area = ring_area_km2(outer, reference_lat).abs();
    let holes = rings
        .iter()
        .skip(1)
        .map(|ring| ring_area_km2(ring, reference_lat).abs())
        .sum::<f32>();
    (outer_area - holes).max(0.0)
}

fn ring_area_km2(ring: &[GeoPosition], reference_lat: f32) -> f32 {
    let points: Vec<[f32; 2]> = ring
        .iter()
        .map(|position| lon_lat_to_local_km(*position, reference_lat))
        .collect();
    signed_area(&points).abs()
}

fn line_length_km(line: &[GeoPosition], closed: bool) -> f32 {
    if line.len() < 2 {
        return 0.0;
    }
    let mut total = line
        .windows(2)
        .map(|segment| haversine_km(segment[0], segment[1]))
        .sum::<f32>();
    if closed && line.first() != line.last() {
        total += haversine_km(*line.last().expect("line length checked"), line[0]);
    }
    total
}

fn average_latitude(line: &[GeoPosition]) -> Option<f32> {
    (!line.is_empty())
        .then(|| line.iter().map(|position| position.lat).sum::<f32>() / line.len() as f32)
}

fn lon_lat_to_local_km(position: GeoPosition, reference_lat: f32) -> [f32; 2] {
    const EARTH_RADIUS_KM: f32 = 6_371.009;
    [
        position.lon.to_radians() * EARTH_RADIUS_KM * reference_lat.to_radians().cos(),
        position.lat.to_radians() * EARTH_RADIUS_KM,
    ]
}

fn haversine_km(a: GeoPosition, b: GeoPosition) -> f32 {
    const EARTH_RADIUS_KM: f32 = 6_371.009;
    let d_lat = (b.lat - a.lat).to_radians();
    let d_lon = (b.lon - a.lon).to_radians();
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let h = (d_lat * 0.5).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon * 0.5).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().asin()
}

fn geo_dataset(
    features: &[GeoFeature],
    options: &GeoMapOptions,
    domain: (f32, f32),
    groups: &[String],
) -> Dataset {
    Dataset::new(
        DATASET,
        1,
        vec![
            (
                "label".into(),
                Column::Utf8(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| Arc::<str>::from(feature.label.clone()))
                        .collect(),
                )),
            ),
            (
                "group".into(),
                Column::Utf8(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| Arc::<str>::from(feature.group.clone()))
                        .collect(),
                )),
            ),
            (
                "kind".into(),
                Column::Utf8(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| Arc::<str>::from(feature.geometry.kind()))
                        .collect(),
                )),
            ),
            (
                "value".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| feature.value.unwrap_or(f32::NAN))
                        .collect(),
                )),
            ),
            (
                "longitude".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| {
                            geometry_center(&feature.geometry).map_or(f32::NAN, |center| center.lon)
                        })
                        .collect(),
                )),
            ),
            (
                "latitude".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| {
                            geometry_center(&feature.geometry).map_or(f32::NAN, |center| center.lat)
                        })
                        .collect(),
                )),
            ),
            (
                "symbol_radius".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| {
                            let t = feature.value.map(|value| normalized(value, domain));
                            scaled_or_default(t, options.point_radius_range, options.point_radius)
                        })
                        .collect(),
                )),
            ),
            (
                "line_width".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| {
                            let t = feature.value.map(|value| normalized(value, domain));
                            scaled_or_default(
                                t,
                                options.line_width_range,
                                options.stroke_width.max(2.0),
                            )
                        })
                        .collect(),
                )),
            ),
            (
                "group_index".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| index_of(groups, &feature.group).unwrap_or(0) as f32)
                        .collect(),
                )),
            ),
            (
                "area_km2".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| geometry_area_km2(&feature.geometry))
                        .collect(),
                )),
            ),
            (
                "length_km".into(),
                Column::F32(ColumnData::new(
                    features
                        .iter()
                        .map(|feature| geometry_length_km(&feature.geometry))
                        .collect(),
                )),
            ),
        ],
    )
}

fn count_geometry(
    geometry: &GeoGeometry,
    points: &mut usize,
    lines: &mut usize,
    polygons: &mut usize,
) {
    match geometry {
        GeoGeometry::Point(_) => *points += 1,
        GeoGeometry::MultiPoint(items) => *points += items.len(),
        GeoGeometry::LineString(_) => *lines += 1,
        GeoGeometry::MultiLineString(items) => *lines += items.len(),
        GeoGeometry::Polygon(_) => *polygons += 1,
        GeoGeometry::MultiPolygon(items) => *polygons += items.len(),
        GeoGeometry::GeometryCollection(items) => {
            for item in items {
                count_geometry(item, points, lines, polygons);
            }
        }
    }
}

fn geometry_is_finite(geometry: &GeoGeometry) -> bool {
    let mut ok = true;
    geometry.visit_positions(&mut |position| {
        ok &= position.lon.is_finite() && position.lat.is_finite();
    });
    ok
}

fn normalized(value: f32, (lo, hi): (f32, f32)) -> f32 {
    if (hi - lo).abs() < f32::EPSILON {
        0.5
    } else {
        ((value - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

fn interpolate_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_geometry() {
        let spec = GeoMapSpec::new(vec![
            GeoFeature::new("a", GeoGeometry::Point(GeoPosition::new(-73.9, 40.7))),
            GeoFeature::new(
                "b",
                GeoGeometry::Polygon(vec![vec![
                    GeoPosition::new(0.0, 0.0),
                    GeoPosition::new(1.0, 0.0),
                    GeoPosition::new(1.0, 1.0),
                    GeoPosition::new(0.0, 1.0),
                    GeoPosition::new(0.0, 0.0),
                ]]),
            ),
        ]);
        let summary = spec.summary();
        assert_eq!(summary.features, 2);
        assert_eq!(summary.points, 1);
        assert_eq!(summary.polygons, 1);
        assert!(summary.total_area_km2 > 12_000.0);
        assert!(summary.total_length_km > 400.0);
        assert!(summary.bounds.is_some());
    }

    #[test]
    fn triangulates_square() {
        let triangles = triangulate_ring(
            &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]],
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(triangles.len(), 2);
    }

    #[test]
    fn simplifies_collinear_polyline() {
        let points = [[0.0, 0.0], [1.0, 0.02], [2.0, -0.01], [3.0, 0.0]];
        let simplified = simplify_polyline(&points, 0.1);
        assert_eq!(simplified, vec![[0.0, 0.0], [3.0, 0.0]]);
    }

    #[test]
    fn value_style_scales_symbols_and_routes() {
        let spec = GeoMapSpec::new(vec![
            GeoFeature::new("low", GeoGeometry::Point(GeoPosition::new(0.0, 0.0))).with_value(0.0),
            GeoFeature::new("high", GeoGeometry::Point(GeoPosition::new(1.0, 1.0)))
                .with_value(10.0),
        ]);
        let groups = group_order(&spec.features);
        let low = feature_style(&spec.features[0], &spec, (0.0, 10.0), &groups);
        let high = feature_style(&spec.features[1], &spec, (0.0, 10.0), &groups);

        assert!(high.point_radius > low.point_radius);
        assert!(high.line_width > low.line_width);
    }

    #[test]
    fn geometry_metrics_cover_lines_polygons_and_holes() {
        let line =
            GeoGeometry::LineString(vec![GeoPosition::new(0.0, 0.0), GeoPosition::new(1.0, 0.0)]);
        assert!((geometry_length_km(&line) - 111.2).abs() < 0.8);

        let solid = GeoGeometry::Polygon(vec![vec![
            GeoPosition::new(0.0, 0.0),
            GeoPosition::new(1.0, 0.0),
            GeoPosition::new(1.0, 1.0),
            GeoPosition::new(0.0, 1.0),
            GeoPosition::new(0.0, 0.0),
        ]]);
        let with_hole = GeoGeometry::Polygon(vec![
            vec![
                GeoPosition::new(0.0, 0.0),
                GeoPosition::new(1.0, 0.0),
                GeoPosition::new(1.0, 1.0),
                GeoPosition::new(0.0, 1.0),
                GeoPosition::new(0.0, 0.0),
            ],
            vec![
                GeoPosition::new(0.25, 0.25),
                GeoPosition::new(0.75, 0.25),
                GeoPosition::new(0.75, 0.75),
                GeoPosition::new(0.25, 0.75),
                GeoPosition::new(0.25, 0.25),
            ],
        ]);

        assert!(geometry_area_km2(&solid) > geometry_area_km2(&with_hole));
        assert!(geometry_area_km2(&with_hole) > 8_000.0);
    }

    #[test]
    fn map_builds_chart() {
        let spec = GeoMapSpec::new(vec![GeoFeature::new(
            "area",
            GeoGeometry::Polygon(vec![vec![
                GeoPosition::new(-1.0, -1.0),
                GeoPosition::new(1.0, -1.0),
                GeoPosition::new(1.0, 1.0),
                GeoPosition::new(-1.0, 1.0),
                GeoPosition::new(-1.0, -1.0),
            ]]),
        )
        .with_value(4.0)]);
        let chart = spec
            .build_chart(Workspace::new(), ChartSize::new(320, 240))
            .expect("valid map");
        assert_eq!(chart.scene().layers.len(), 1);
        assert_eq!(chart.scene().guides.len(), 3);
    }
}
