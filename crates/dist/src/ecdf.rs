//! Empirical Cumulative Distribution Function (ECDF).
//!
//! For each input series, draws a step function: x = sample value (sorted),
//! y = cumulative fraction (0..1). Multiple series overlay for comparison.

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem, LinePrim, LinearScale,
    Mark, MarkId, PickCtx, PickHit, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget,
    SnapTargetSet, TessellateCtx, Workspace,
};

const SERIES_DATASET: DatasetId = DatasetId::new(0);
const LINE_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One ECDF series.
#[derive(Debug, Clone, PartialEq)]
pub struct EcdfSeries {
    /// Display label.
    pub label: String,
    /// Sample values.
    pub values: Vec<f32>,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
}

impl EcdfSeries {
    /// Build a series with default colour.
    #[must_use]
    pub fn new(label: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
            color: [0.45, 0.55, 0.85, 1.0],
        }
    }

    /// Override colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// ECDF chart configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcdfOptions {
    /// Plot padding (pixels).
    pub padding: f32,
    /// Line width.
    pub line_width: f32,
}

impl Default for EcdfOptions {
    fn default() -> Self {
        Self {
            padding: 30.0,
            line_width: 1.5,
        }
    }
}

/// ECDF chart spec.
#[derive(Debug, Clone)]
pub struct EcdfSpec {
    series: Vec<EcdfSeries>,
    options: EcdfOptions,
}

impl EcdfSpec {
    /// Build an ECDF spec from one or more series.
    #[must_use]
    pub fn new(series: Vec<EcdfSeries>) -> Self {
        Self {
            series,
            options: EcdfOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: EcdfOptions) -> Self {
        self.options = options;
        self
    }
}

/// Errors during ECDF build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcdfError {
    /// No series supplied.
    Empty,
    /// Series at index has no samples.
    EmptySeries(usize),
}

impl fmt::Display for EcdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "ECDF spec has no series"),
            Self::EmptySeries(i) => write!(f, "series at index {i} has no samples"),
        }
    }
}

impl std::error::Error for EcdfError {}

/// Computed ECDF step polylines.
#[derive(Debug, Clone, PartialEq)]
pub struct EcdfLayout {
    /// One per series, same order.
    pub series: Vec<EcdfSeriesLayout>,
}

/// Step polyline for one ECDF series.
#[derive(Debug, Clone, PartialEq)]
pub struct EcdfSeriesLayout {
    /// Display label.
    pub label: String,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
    /// Step polyline points in screen pixels.
    pub points: Vec<[f32; 2]>,
}

impl ChartSpec for EcdfSpec {
    type Error = EcdfError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        if self.series.is_empty() {
            return Err(EcdfError::Empty);
        }
        for (i, s) in self.series.iter().enumerate() {
            if s.values.is_empty() {
                return Err(EcdfError::EmptySeries(i));
            }
        }

        let viewport = size.full_viewport();
        let layout = compute_layout(&self.series, &self.options, viewport.plot_area);

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
        workspace.upsert_dataset(series_dataset(&layout));

        let line_mark: Arc<dyn Mark> = Arc::new(EcdfLineMark::new(
            LINE_MARK,
            layout.clone(),
            self.options.line_width,
        ));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![line_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Legend(
            LegendGuide::new(legend_items(&layout))
                .with_title("Series")
                .with_anchor(LegendAnchor::Bottom),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(endpoint_labels(&layout))
                .with_collision_padding(4.0)
                .with_max_visible(layout.series.len()),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout)).with_name("ecdf endpoints"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn legend_items(layout: &EcdfLayout) -> Vec<LegendItem> {
    layout
        .series
        .iter()
        .map(|series| LegendItem::new(series.label.clone(), series.color))
        .collect()
}

fn endpoint_labels(layout: &EcdfLayout) -> Vec<LabelItem> {
    layout
        .series
        .iter()
        .filter_map(|series| {
            series.points.last().map(|point| {
                LabelItem::new(point[0], point[1], series.label.clone())
                    .with_anchor(LabelAnchor::Right)
                    .with_kind(LabelKind::Data)
                    .with_priority(LabelPriority::Important)
            })
        })
        .collect()
}

fn snap_targets(layout: &EcdfLayout) -> Vec<SnapTarget> {
    let mut targets = Vec::with_capacity(layout.series.len() * 2);
    for series in &layout.series {
        if let Some(point) = series.points.first() {
            targets.push(
                SnapTarget::new(point[0], point[1], SnapKind::Point)
                    .with_radius(6.0)
                    .with_label(format!("{} start", series.label))
                    .with_priority(2),
            );
        }
        if let Some(point) = series.points.last() {
            targets.push(
                SnapTarget::new(point[0], point[1], SnapKind::Point)
                    .with_radius(6.0)
                    .with_label(format!("{} end", series.label))
                    .with_priority(3),
            );
        }
    }
    targets
}

fn compute_layout(series: &[EcdfSeries], options: &EcdfOptions, plot: Rect) -> EcdfLayout {
    let pad = options.padding;
    let inner = Rect::new(
        plot.x + pad,
        plot.y + pad,
        (plot.w - 2.0 * pad).max(1.0),
        (plot.h - 2.0 * pad).max(1.0),
    );

    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut sorted_series: Vec<(String, [f32; 4], Vec<f32>)> = Vec::with_capacity(series.len());
    for s in series {
        let mut v: Vec<f32> = s.values.iter().copied().filter(|x| x.is_finite()).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if let (Some(&lo), Some(&hi)) = (v.first(), v.last()) {
            x_min = x_min.min(lo);
            x_max = x_max.max(hi);
        }
        sorted_series.push((s.label.clone(), s.color, v));
    }

    if x_max <= x_min {
        x_max = x_min + 1.0;
    }
    let span = x_max - x_min;
    x_min -= span * 0.02;
    x_max += span * 0.02;
    let map_x = |v: f32| inner.x + (v - x_min) / (x_max - x_min) * inner.w;
    let map_y = |frac: f32| inner.y + inner.h - frac * inner.h;

    let series_layout: Vec<EcdfSeriesLayout> = sorted_series
        .into_iter()
        .map(|(label, color, sorted)| {
            let n = sorted.len() as f32;
            let mut points = Vec::with_capacity(sorted.len() * 2 + 2);
            // start at y=0 at the leftmost x
            if let Some(&first) = sorted.first() {
                points.push([map_x(first), map_y(0.0)]);
            }
            for (i, &v) in sorted.iter().enumerate() {
                let frac = (i as f32 + 1.0) / n;
                let x_px = map_x(v);
                // horizontal step from previous y to this x
                if let Some(&last) = points.last() {
                    points.push([x_px, last[1]]);
                }
                // vertical step up to new y
                points.push([x_px, map_y(frac)]);
            }
            EcdfSeriesLayout {
                label,
                color,
                points,
            }
        })
        .collect();

    EcdfLayout {
        series: series_layout,
    }
}

fn series_dataset(layout: &EcdfLayout) -> Dataset {
    let mut series_idx: Vec<i64> = Vec::new();
    let mut x: Vec<f32> = Vec::new();
    let mut y: Vec<f32> = Vec::new();
    for (si, s) in layout.series.iter().enumerate() {
        for p in &s.points {
            series_idx.push(si as i64);
            x.push(p[0]);
            y.push(p[1]);
        }
    }
    Dataset::new(
        SERIES_DATASET,
        1,
        vec![
            (
                "series".to_string(),
                Column::I64(ColumnData::new(series_idx)),
            ),
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

#[derive(Debug, Clone)]
struct EcdfLineMark {
    id: MarkId,
    layout: EcdfLayout,
    width: f32,
}

impl EcdfLineMark {
    fn new(id: MarkId, layout: EcdfLayout, width: f32) -> Self {
        Self { id, layout, width }
    }
}

impl Mark for EcdfLineMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.series.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let lines: Vec<LinePrim> = self
            .layout
            .series
            .iter()
            .filter(|s| s.points.len() >= 2)
            .map(|s| LinePrim {
                points: s.points.clone(),
                stroke: s.color,
                width: self.width,
                dash: None,
                join: 1,
                cap: 1,
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
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for s in &self.layout.series {
            for p in &s.points {
                min_x = min_x.min(p[0]);
                min_y = min_y.min(p[1]);
                max_x = max_x.max(p[0]);
                max_y = max_y.max(p[1]);
            }
        }
        if min_x.is_infinite() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Guide, SnapKind};

    #[test]
    fn empty_spec_rejected() {
        let result = EcdfSpec::new(vec![]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(result, Err(EcdfError::Empty)));
    }

    #[test]
    fn empty_series_rejected() {
        let result = EcdfSpec::new(vec![EcdfSeries::new("a", vec![])]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(result, Err(EcdfError::EmptySeries(0))));
    }

    #[test]
    fn step_polyline_has_2n_plus_one_points() {
        // n samples → start point + 2n step points = 2n+1
        let layout = compute_layout(
            &[EcdfSeries::new("a", vec![1.0, 2.0, 3.0])],
            &EcdfOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert_eq!(layout.series[0].points.len(), 7);
    }

    #[test]
    fn step_terminates_at_y_zero() {
        // The first point should sit at the bottom of the plot (frac=0 maps to inner.y + inner.h).
        let layout = compute_layout(
            &[EcdfSeries::new("a", vec![5.0])],
            &EcdfOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let first = layout.series[0].points.first().unwrap();
        // Inner y = 30, inner h = 240 → bottom y = 270.
        assert!((first[1] - 270.0).abs() < 1e-3);
    }

    #[test]
    fn build_chart_succeeds() {
        let chart = EcdfSpec::new(vec![
            EcdfSeries::new("A", (1..=20).map(|i| i as f32).collect()),
            EcdfSeries::new("B", (5..=25).map(|i| i as f32 * 1.1).collect()),
        ])
        .build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(500, 400),
        )
        .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn build_chart_exposes_series_legend_labels_and_snap_targets() {
        let chart = EcdfSpec::new(vec![
            EcdfSeries::new("A", vec![1.0, 2.0, 3.0]),
            EcdfSeries::new("B", vec![2.0, 4.0, 6.0]),
        ])
        .build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(420, 300),
        )
        .expect("chart");

        let legend = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Legend(legend) => Some(legend),
                _ => None,
            })
            .expect("legend guide");
        assert_eq!(legend.items.len(), 2);
        assert_eq!(legend.items[0].label, "A");

        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        assert!(labels.items.iter().any(|item| item.text == "A"));
        assert!(labels.items.iter().any(|item| item.text == "B"));

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 4);
        assert!(targets.iter().all(|target| target.kind == SnapKind::Point));
        assert!(targets
            .iter()
            .any(|target| target.label.as_deref() == Some("A start")));
        assert!(targets
            .iter()
            .any(|target| target.label.as_deref() == Some("B end")));
    }
}
