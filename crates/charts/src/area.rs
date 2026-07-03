//! Multi-series area chart: line with a filled region below the curve.
//!
//! Three modes supported via [`StackMode`]: overlapping (each series
//! independently filled with translucent colour), stacked (cumulative
//! y values per x), and normalized (100% stacked — each x sums to 1).

use std::fmt;
use std::sync::Arc;

use ahash::AHashMap;
use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem,
    LinePrim, LinearScale, Mark, MarkId, PathCommand, PathPrim, PickCtx, PickHit, Rect, Scale,
    ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx, TooltipField, TooltipGuide,
    Workspace,
};

const DATASET: DatasetId = DatasetId::new(0);
const AREA_MARK: MarkId = MarkId::new(1);
const LINE_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One observed point.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaDatum {
    /// Series id.
    pub series: String,
    /// X value.
    pub x: f32,
    /// Y value (>= 0 for stacked modes; negative values clipped to 0 in those).
    pub y: f32,
}

impl AreaDatum {
    /// Build a datum.
    #[must_use]
    pub fn new(series: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            series: series.into(),
            x,
            y,
        }
    }
}

/// Stack mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackMode {
    /// Each series drawn independently atop the others. Default. Translucent
    /// fills so overlap is visible.
    Overlap,
    /// Series stacked cumulatively per x. Fills opaque.
    Stacked,
    /// Stacked + normalized so the total at each x is 1.0. Useful for
    /// part-of-whole over time.
    Normalized,
}

/// Configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaChartOptions {
    /// Plot padding.
    pub padding: f32,
    /// Stack mode.
    pub stack: StackMode,
    /// Fill opacity in Overlap mode (0..1). Ignored in Stacked / Normalized.
    pub overlap_fill_opacity: f32,
    /// Series colour palette. Cycles if more series than colours.
    pub palette: Vec<[f32; 4]>,
    /// Whether to draw the line atop the fill.
    pub show_line: bool,
    /// Line width.
    pub line_width: f32,
    /// Optional fixed y domain (Overlap / Stacked only).
    pub y_domain: Option<(f32, f32)>,
}

impl Default for AreaChartOptions {
    fn default() -> Self {
        Self {
            padding: 30.0,
            stack: StackMode::Overlap,
            overlap_fill_opacity: 0.28,
            palette: default_palette(),
            show_line: true,
            line_width: 1.8,
            y_domain: None,
        }
    }
}

fn default_palette() -> Vec<[f32; 4]> {
    // Mild, color-blind-friendlier palette. Alpha set to 1.0; mode applies opacity.
    vec![
        [0.30, 0.45, 0.85, 1.0],
        [0.85, 0.45, 0.30, 1.0],
        [0.40, 0.70, 0.45, 1.0],
        [0.75, 0.55, 0.85, 1.0],
        [0.85, 0.65, 0.30, 1.0],
        [0.40, 0.70, 0.80, 1.0],
    ]
}

/// Area chart spec.
#[derive(Debug, Clone)]
pub struct AreaChartSpec {
    data: Vec<AreaDatum>,
    options: AreaChartOptions,
}

impl AreaChartSpec {
    /// Build a spec.
    #[must_use]
    pub fn new(data: Vec<AreaDatum>) -> Self {
        Self {
            data,
            options: AreaChartOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub fn with_options(mut self, options: AreaChartOptions) -> Self {
        self.options = options;
        self
    }
}

/// Errors during area build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaChartError {
    /// No data points.
    Empty,
    /// A series has fewer than 2 points.
    SeriesTooShort(String),
}

impl fmt::Display for AreaChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "area chart has no data"),
            Self::SeriesTooShort(s) => write!(f, "series `{s}` has fewer than 2 points"),
        }
    }
}

impl std::error::Error for AreaChartError {}

/// Per-series rendered area band.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaBand {
    /// Series id.
    pub series: String,
    /// Upper polyline (top edge of the band).
    pub upper: Vec<[f32; 2]>,
    /// Lower polyline (bottom edge of the band).
    pub lower: Vec<[f32; 2]>,
    /// Fill colour.
    pub color: [f32; 4],
}

/// Computed area layout.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaChartLayout {
    /// One per series (drawn bottom-up).
    pub bands: Vec<AreaBand>,
    /// Plot rect (post-padding).
    pub plot: Rect,
}

impl ChartSpec for AreaChartSpec {
    type Error = AreaChartError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        if self.data.is_empty() {
            return Err(AreaChartError::Empty);
        }
        let viewport = size.full_viewport();
        let layout = compute_layout(&self.data, &self.options, viewport.plot_area)?;

        let x_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (0.0, viewport.width as f64),
            (0.0, viewport.width as f32),
        ));
        let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (0.0, viewport.height as f64),
            (0.0, viewport.height as f32),
        ));
        workspace.upsert_scale(X_SCALE, x_scale);
        workspace.upsert_scale(Y_SCALE, y_scale);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(dataset(&layout));

        let area_mark: Arc<dyn Mark> =
            Arc::new(AreaMark::new(AREA_MARK, layout.clone(), self.options.stack));
        let mut marks: Vec<Arc<dyn Mark>> = vec![area_mark];
        if self.options.show_line {
            marks.push(Arc::new(AreaLineMark::new(
                LINE_MARK,
                layout.clone(),
                self.options.line_width,
            )));
        }

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks,
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                AREA_MARK,
                DATASET,
                vec![
                    TooltipField::new("Series", "series").as_label(),
                    TooltipField::new("X", "x").as_number(1),
                    TooltipField::new("Y", "y").as_number(1),
                ],
            )
            .with_title_column("series"),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(legend_items(&layout))
                .with_title("Series")
                .with_anchor(LegendAnchor::Bottom),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout)).with_name("area band tops"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn compute_layout(
    data: &[AreaDatum],
    options: &AreaChartOptions,
    plot: Rect,
) -> Result<AreaChartLayout, AreaChartError> {
    // Group by series, preserving first-seen order.
    let mut series_order: Vec<String> = Vec::new();
    let mut by_series: AHashMap<String, Vec<(f32, f32)>> = AHashMap::new();
    for d in data {
        if !by_series.contains_key(&d.series) {
            series_order.push(d.series.clone());
        }
        by_series
            .entry(d.series.clone())
            .or_default()
            .push((d.x, d.y.max(0.0)));
    }
    for series in &series_order {
        let v = &by_series[series];
        if v.len() < 2 {
            return Err(AreaChartError::SeriesTooShort(series.clone()));
        }
    }
    // Sort each series by x for monotone polyline.
    for v in by_series.values_mut() {
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    // Collect global x universe (sorted unique).
    let mut xs: Vec<f32> = data.iter().map(|d| d.x).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.dedup_by(|a, b| (*a - *b).abs() < f32::EPSILON);
    let x_min = *xs.first().unwrap();
    let x_max = *xs.last().unwrap();

    let pad = options.padding;
    let inner = Rect::new(
        plot.x + pad,
        plot.y + pad,
        (plot.w - 2.0 * pad).max(1.0),
        (plot.h - 2.0 * pad).max(1.0),
    );

    // Resolve per-series per-x value via linear interpolation when missing.
    let interp = |series_pts: &[(f32, f32)], x: f32| -> f32 {
        if x <= series_pts[0].0 {
            return series_pts[0].1;
        }
        if x >= series_pts[series_pts.len() - 1].0 {
            return series_pts[series_pts.len() - 1].1;
        }
        for w in series_pts.windows(2) {
            let (x0, y0) = w[0];
            let (x1, y1) = w[1];
            if x >= x0 && x <= x1 {
                if (x1 - x0).abs() < f32::EPSILON {
                    return y0;
                }
                let t = (x - x0) / (x1 - x0);
                return y0 + (y1 - y0) * t;
            }
        }
        series_pts[series_pts.len() - 1].1
    };

    let n_series = series_order.len();
    let n_x = xs.len();
    let mut grid = vec![vec![0.0_f32; n_x]; n_series];
    for (si, series) in series_order.iter().enumerate() {
        let pts = &by_series[series];
        for (xi, &x) in xs.iter().enumerate() {
            grid[si][xi] = interp(pts, x);
        }
    }

    // Compute lower/upper per (series, x_index) per mode.
    let mut lower = vec![vec![0.0_f32; n_x]; n_series];
    let mut upper = vec![vec![0.0_f32; n_x]; n_series];
    match options.stack {
        StackMode::Overlap => {
            for si in 0..n_series {
                for xi in 0..n_x {
                    lower[si][xi] = 0.0;
                    upper[si][xi] = grid[si][xi];
                }
            }
        }
        StackMode::Stacked => {
            let mut running = vec![0.0_f32; n_x];
            for si in 0..n_series {
                for xi in 0..n_x {
                    lower[si][xi] = running[xi];
                    upper[si][xi] = running[xi] + grid[si][xi];
                    running[xi] = upper[si][xi];
                }
            }
        }
        StackMode::Normalized => {
            // Per-x totals
            let mut totals = vec![0.0_f32; n_x];
            for xi in 0..n_x {
                for si in 0..n_series {
                    totals[xi] += grid[si][xi];
                }
            }
            let mut running = vec![0.0_f32; n_x];
            for si in 0..n_series {
                for xi in 0..n_x {
                    let t = if totals[xi] > 0.0 {
                        grid[si][xi] / totals[xi]
                    } else {
                        0.0
                    };
                    lower[si][xi] = running[xi];
                    upper[si][xi] = running[xi] + t;
                    running[xi] = upper[si][xi];
                }
            }
        }
    }

    // Compute y range
    let (y_min, y_max) = match options.stack {
        StackMode::Normalized => (0.0_f32, 1.0_f32),
        _ => match options.y_domain {
            Some((lo, hi)) => (lo, hi),
            None => {
                let mut hi = 0.0_f32;
                for s in &upper {
                    for &v in s {
                        if v > hi {
                            hi = v;
                        }
                    }
                }
                (0.0, hi.max(1.0))
            }
        },
    };
    let y_pad = (y_max - y_min) * 0.05;
    let y_min_final = y_min - y_pad;
    let y_max_final = y_max + y_pad;

    let map_x = |x: f32| {
        if x_max > x_min {
            inner.x + (x - x_min) / (x_max - x_min) * inner.w
        } else {
            inner.x + inner.w * 0.5
        }
    };
    let map_y =
        |v: f32| inner.y + inner.h - (v - y_min_final) / (y_max_final - y_min_final) * inner.h;

    let bands: Vec<AreaBand> = series_order
        .iter()
        .enumerate()
        .map(|(si, series)| {
            let color = pick_color(
                &options.palette,
                si,
                options.stack,
                options.overlap_fill_opacity,
            );
            let upper_pts: Vec<[f32; 2]> = xs
                .iter()
                .enumerate()
                .map(|(xi, &x)| [map_x(x), map_y(upper[si][xi])])
                .collect();
            let lower_pts: Vec<[f32; 2]> = xs
                .iter()
                .enumerate()
                .map(|(xi, &x)| [map_x(x), map_y(lower[si][xi])])
                .collect();
            AreaBand {
                series: series.clone(),
                upper: upper_pts,
                lower: lower_pts,
                color,
            }
        })
        .collect();

    Ok(AreaChartLayout { bands, plot: inner })
}

fn pick_color(palette: &[[f32; 4]], i: usize, stack: StackMode, overlap_opacity: f32) -> [f32; 4] {
    let base = if palette.is_empty() {
        [0.45, 0.55, 0.85, 1.0]
    } else {
        palette[i % palette.len()]
    };
    let mut out = base;
    if matches!(stack, StackMode::Overlap) {
        out[3] = overlap_opacity.clamp(0.0, 1.0);
    }
    out
}

fn dataset(layout: &AreaChartLayout) -> Dataset {
    let mut series: Vec<Arc<str>> = Vec::new();
    let mut x: Vec<f32> = Vec::new();
    let mut y: Vec<f32> = Vec::new();
    for band in &layout.bands {
        for p in &band.upper {
            series.push(Arc::from(band.series.as_str()));
            x.push(p[0]);
            y.push(p[1]);
        }
    }
    Dataset::new(
        DATASET,
        1,
        vec![
            ("series".to_string(), Column::Utf8(ColumnData::new(series))),
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

fn legend_items(layout: &AreaChartLayout) -> Vec<LegendItem> {
    layout
        .bands
        .iter()
        .map(|band| {
            let mut color = band.color;
            color[3] = 1.0;
            LegendItem::new(band.series.clone(), color)
        })
        .collect()
}

fn snap_targets(layout: &AreaChartLayout) -> Vec<SnapTarget> {
    layout
        .bands
        .iter()
        .flat_map(|band| {
            band.upper.iter().enumerate().map(|(index, point)| {
                SnapTarget::new(point[0], point[1], SnapKind::Point)
                    .with_radius(6.0)
                    .with_label(format!("{} top {}", band.series, index + 1))
                    .with_priority(1)
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct AreaMark {
    id: MarkId,
    layout: AreaChartLayout,
    stack: StackMode,
}

impl AreaMark {
    fn new(id: MarkId, layout: AreaChartLayout, stack: StackMode) -> Self {
        Self { id, layout, stack }
    }
}

impl Mark for AreaMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.bands.len() as u64;
        h ^= match self.stack {
            StackMode::Overlap => 1,
            StackMode::Stacked => 2,
            StackMode::Normalized => 3,
        };
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut paths = Vec::with_capacity(self.layout.bands.len());
        for band in &self.layout.bands {
            if band.upper.len() < 2 {
                continue;
            }
            let mut commands = Vec::with_capacity(band.upper.len() * 2 + 2);
            let first = band.upper[0];
            commands.push(PathCommand::MoveTo {
                x: first[0],
                y: first[1],
            });
            for p in &band.upper[1..] {
                commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
            }
            for p in band.lower.iter().rev() {
                commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
            }
            commands.push(PathCommand::Close);
            paths.push(PathPrim {
                commands,
                fill: band.color,
                stroke: [0.0, 0.0, 0.0, 0.0],
                stroke_width: 0.0,
            });
        }
        if paths.is_empty() {
            Geometry::Empty
        } else {
            Geometry::Paths(paths)
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        // Walk bands top-down (later-drawn series on top).
        for (band_index, band) in self.layout.bands.iter().enumerate().rev() {
            if band_contains(band, [px, py]) {
                let row_offset = self.layout.bands[..band_index]
                    .iter()
                    .map(|band| band.upper.len())
                    .sum::<usize>();
                let point_index = nearest_upper_point_index(band, [px, py]);
                return Some(PickHit {
                    mark: self.id,
                    row: Some(row_offset + point_index),
                    distance: 0.0,
                    payload: None,
                });
            }
        }
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.layout.plot
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn nearest_upper_point_index(band: &AreaBand, point: [f32; 2]) -> usize {
    band.upper
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a[0] - point[0]).mul_add(a[0] - point[0], (a[1] - point[1]).powi(2));
            let db = (b[0] - point[0]).mul_add(b[0] - point[0], (b[1] - point[1]).powi(2));
            da.total_cmp(&db)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn band_contains(band: &AreaBand, p: [f32; 2]) -> bool {
    if band.upper.len() < 2 {
        return false;
    }
    // x must lie in band's x range
    let x_min = band.upper[0][0];
    let x_max = band.upper.last().unwrap()[0];
    if p[0] < x_min.min(x_max) || p[0] > x_min.max(x_max) {
        return false;
    }
    // find the segment in upper/lower bracketing p[0]
    let interp = |poly: &[[f32; 2]], x: f32| -> f32 {
        for w in poly.windows(2) {
            let (a, b) = (w[0], w[1]);
            let lo = a[0].min(b[0]);
            let hi = a[0].max(b[0]);
            if x >= lo && x <= hi {
                if (b[0] - a[0]).abs() < f32::EPSILON {
                    return a[1];
                }
                let t = (x - a[0]) / (b[0] - a[0]);
                return a[1] + (b[1] - a[1]) * t;
            }
        }
        poly.last().unwrap()[1]
    };
    let y_up = interp(&band.upper, p[0]);
    let y_lo = interp(&band.lower, p[0]);
    let y_min = y_up.min(y_lo);
    let y_max = y_up.max(y_lo);
    p[1] >= y_min && p[1] <= y_max
}

#[derive(Debug, Clone)]
struct AreaLineMark {
    id: MarkId,
    layout: AreaChartLayout,
    width: f32,
}

impl AreaLineMark {
    fn new(id: MarkId, layout: AreaChartLayout, width: f32) -> Self {
        Self { id, layout, width }
    }
}

impl Mark for AreaLineMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.bands.len() as u64;
        h ^= self.width.to_bits() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let lines: Vec<LinePrim> = self
            .layout
            .bands
            .iter()
            .filter(|b| b.upper.len() >= 2)
            .map(|b| {
                let mut stroke = b.color;
                stroke[3] = 1.0; // full alpha for the topline regardless of fill mode
                LinePrim {
                    points: b.upper.clone(),
                    stroke,
                    width: self.width,
                    dash: None,
                    join: 1,
                    cap: 1,
                }
            })
            .collect();
        if lines.is_empty() {
            Geometry::Empty
        } else {
            Geometry::Lines(lines)
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.layout.plot
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, Guide, PickCtx, TooltipValueFormat};

    fn sample_two_series() -> Vec<AreaDatum> {
        let mut v = Vec::new();
        for i in 0..10 {
            v.push(AreaDatum::new("a", i as f32, 5.0 + (i as f32).sin()));
            v.push(AreaDatum::new("b", i as f32, 3.0 + (i as f32 * 0.5).cos()));
        }
        v
    }

    #[test]
    fn empty_spec_rejected() {
        let r = AreaChartSpec::new(vec![]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(AreaChartError::Empty)));
    }

    #[test]
    fn short_series_rejected() {
        let r = AreaChartSpec::new(vec![AreaDatum::new("a", 0.0, 5.0)]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(AreaChartError::SeriesTooShort(_))));
    }

    #[test]
    fn builds_overlap_default() {
        let chart = AreaChartSpec::new(sample_two_series())
            .build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(600, 400),
            )
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn emits_semantic_guides_and_snap_targets_for_all_stack_modes() {
        for stack in [
            StackMode::Overlap,
            StackMode::Stacked,
            StackMode::Normalized,
        ] {
            let chart = AreaChartSpec::new(sample_two_series())
                .with_options(AreaChartOptions {
                    stack,
                    ..AreaChartOptions::default()
                })
                .build_chart(
                    berthacharts_core::Workspace::new(),
                    ChartSize::new(600, 400),
                )
                .expect("chart");

            assert_eq!(chart.snap_targets().len(), 20, "{stack:?}");

            let legend = chart
                .scene()
                .guides
                .iter()
                .find_map(|guide| match guide {
                    Guide::Legend(legend) => Some(legend),
                    _ => None,
                })
                .expect("legend guide");
            assert_eq!(legend.items.len(), 2, "{stack:?}");
            assert_eq!(legend.items[0].label, "a");
            assert_eq!(legend.items[1].label, "b");

            let tooltip = chart
                .scene()
                .guides
                .iter()
                .find_map(|guide| match guide {
                    Guide::Tooltip(tooltip) => Some(tooltip),
                    _ => None,
                })
                .expect("tooltip guide");
            assert_eq!(tooltip.title_column.as_deref(), Some("series"));
            assert_eq!(tooltip.fields.len(), 3);
            assert_eq!(tooltip.fields[0].label, "Series");
            assert_eq!(tooltip.fields[0].column, "series");
            assert_eq!(tooltip.fields[0].format, TooltipValueFormat::Label);
            assert_eq!(tooltip.fields[1].label, "X");
            assert_eq!(tooltip.fields[1].column, "x");
            assert_eq!(
                tooltip.fields[1].format,
                TooltipValueFormat::Number { decimals: 1 }
            );
            assert_eq!(tooltip.fields[2].label, "Y");
            assert_eq!(tooltip.fields[2].column, "y");
            assert_eq!(
                tooltip.fields[2].format,
                TooltipValueFormat::Number { decimals: 1 }
            );
        }
    }

    #[test]
    fn area_pick_rows_align_with_point_tooltip_dataset() {
        let workspace = berthacharts_core::Workspace::new();
        let chart = AreaChartSpec::new(sample_two_series())
            .build_chart(workspace.clone(), ChartSize::new(600, 400))
            .expect("chart");

        let mark = chart.scene().layers[0].marks[0]
            .as_any()
            .downcast_ref::<AreaMark>()
            .expect("area mark");
        let point_index = 3;
        let band_index = 1;
        let point = mark.layout.bands[band_index].upper[point_index];

        let coord = workspace.coord(COORD).expect("coord");
        let scales = workspace.scales();
        let datasets = workspace.datasets();
        let selection = workspace.selection();
        let ctx = PickCtx::new(
            coord.as_ref(),
            &scales,
            &datasets,
            &selection,
            chart.scene().viewport.plot_area,
            1.0,
        );

        let hit = mark.pick(&ctx, (point[0], point[1])).expect("hit");
        let row = hit.row.expect("row");
        assert_eq!(
            row,
            band_index * mark.layout.bands[band_index].upper.len() + point_index
        );

        let dataset = workspace.dataset(DATASET).expect("dataset");
        let series = match dataset.column("series").expect("series").as_ref() {
            Column::Utf8(data) => data,
            other => panic!("expected utf8 series column, got {}", other.dtype()),
        };
        assert_eq!(series.values[row].as_ref(), "b");
    }

    #[test]
    fn stacked_upper_above_lower() {
        let layout = compute_layout(
            &sample_two_series(),
            &AreaChartOptions {
                stack: StackMode::Stacked,
                ..AreaChartOptions::default()
            },
            Rect::new(0.0, 0.0, 600.0, 400.0),
        )
        .expect("layout");
        // Second band's lower should equal first band's upper at every x
        // (cumulative stacking) in data units. After mapping to screen pixels
        // (y inverted) this still holds component-wise.
        let b0 = &layout.bands[0];
        let b1 = &layout.bands[1];
        assert_eq!(b0.upper.len(), b1.lower.len());
        for (u, l) in b0.upper.iter().zip(b1.lower.iter()) {
            assert!((u[1] - l[1]).abs() < 1e-3);
        }
    }

    #[test]
    fn normalized_sums_to_one() {
        let layout = compute_layout(
            &sample_two_series(),
            &AreaChartOptions {
                stack: StackMode::Normalized,
                ..AreaChartOptions::default()
            },
            Rect::new(0.0, 0.0, 600.0, 400.0),
        )
        .expect("layout");
        // Top band's upper should map to y=1.0 in data space (which is the
        // top of the plot in screen space — y == inner.y after padding).
        let b_top = layout.bands.last().unwrap();
        // At each x, the top of the top band should be near the plot top (small y in screen).
        let plot_top = layout.plot.y;
        for p in &b_top.upper {
            // Allow 5% padding the layout adds.
            assert!(p[1] - plot_top < layout.plot.h * 0.1);
        }
    }
}
