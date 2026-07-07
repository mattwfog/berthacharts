//! Empirical Cumulative Distribution Function (ECDF).
//!
//! For each input series, draws a right-continuous step function: `x` = sample
//! value (sorted ascending), `y` = cumulative fraction `F(x) = #{xᵢ ≤ x} / n`
//! in `[0, 1]`. Multiple series overlay for comparison.
//!
//! ## Ties
//!
//! Repeated values are collapsed into a single jump: at each distinct value the
//! curve rises by `(count of ties) / n` in one vertical step, so `m` identical
//! samples produce one riser of height `m/n` rather than `m` coincident risers.
//! A series of `d` distinct values therefore has `2·d + 1` vertices.
//!
//! Non-finite samples are rejected at build time with
//! [`EcdfError::NonFiniteValue`].
//!
//! ## Example
//!
//! ```
//! use berthacharts_dist::ecdf::{EcdfSeries, EcdfSpec};
//! use berthacharts_dist::core::{ChartSize, Workspace};
//!
//! let spec = EcdfSpec::new(vec![
//!     EcdfSeries::new("A", vec![1.0, 2.0, 2.0, 3.0, 5.0]),
//!     EcdfSeries::new("B", vec![2.0, 4.0, 6.0]),
//! ]);
//! let chart = spec
//!     .try_build_chart(Workspace::new(), ChartSize::new(480, 320))
//!     .expect("valid ECDF");
//! assert_eq!(chart.scene().layers.len(), 1);
//! ```

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

    /// Validate the series without building a chart.
    ///
    /// Rejects an empty spec, empty series, empty labels, and any non-finite
    /// sample value.
    pub fn validate(&self) -> Result<(), EcdfError> {
        if self.series.is_empty() {
            return Err(EcdfError::Empty);
        }
        for (i, s) in self.series.iter().enumerate() {
            if s.label.trim().is_empty() {
                return Err(EcdfError::EmptyLabel(i));
            }
            if s.values.is_empty() {
                return Err(EcdfError::EmptySeries(i));
            }
            for &v in &s.values {
                if !v.is_finite() {
                    return Err(EcdfError::NonFiniteValue {
                        label: s.label.clone(),
                        value: v,
                    });
                }
            }
        }
        Ok(())
    }

    /// Compute the reusable ECDF layout without building a chart.
    pub fn layout(&self, size: ChartSize) -> Result<EcdfLayout, EcdfError> {
        self.validate()?;
        Ok(compute_layout(
            &self.series,
            &self.options,
            size.full_viewport().plot_area,
        ))
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, EcdfError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }
}

/// Errors during ECDF build.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EcdfError {
    /// No series supplied.
    Empty,
    /// Series at index has no samples.
    EmptySeries(usize),
    /// Series at the given index has an empty or whitespace-only label.
    EmptyLabel(usize),
    /// A sample value was non-finite (NaN or infinity).
    NonFiniteValue {
        /// Label of the offending series.
        label: String,
        /// The offending value.
        value: f32,
    },
}

impl fmt::Display for EcdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "ECDF spec has no series"),
            Self::EmptySeries(i) => write!(f, "series at index {i} has no samples"),
            Self::EmptyLabel(i) => write!(f, "series at index {i} has an empty label"),
            Self::NonFiniteValue { label, value } => {
                write!(f, "ECDF value for `{label}` is not finite: {value}")
            }
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
        self.validate()?;
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
            // Walk distinct values, collapsing ties into one jump of height
            // (#ties / n) so F(x) = #{xᵢ ≤ x} / n is exact and the polyline has
            // no zero-length risers.
            let mut i = 0;
            while i < sorted.len() {
                let v = sorted[i];
                let mut j = i + 1;
                while j < sorted.len() && sorted[j] == v {
                    j += 1;
                }
                let frac = j as f32 / n;
                let x_px = map_x(v);
                // horizontal step from previous cumulative height to this x
                if let Some(&last) = points.last() {
                    points.push([x_px, last[1]]);
                }
                // vertical step up to the new cumulative height
                points.push([x_px, map_y(frac)]);
                i = j;
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
    fn step_polyline_has_2d_plus_one_points() {
        // d distinct values → start point + 2d step points = 2d+1
        let layout = compute_layout(
            &[EcdfSeries::new("a", vec![1.0, 2.0, 3.0])],
            &EcdfOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert_eq!(layout.series[0].points.len(), 7);
    }

    #[test]
    fn ties_collapse_into_single_jump() {
        // [1,1,2,2,2] has 2 distinct values → 2·2 + 1 = 5 vertices. After the
        // first distinct value F jumps to 2/5, so the vertex sits at
        // map_y(0.4) = 30 + 240 − 0.4·240 = 174 on a 400×300 canvas.
        let layout = compute_layout(
            &[EcdfSeries::new("a", vec![1.0, 1.0, 2.0, 2.0, 2.0])],
            &EcdfOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let points = &layout.series[0].points;
        assert_eq!(points.len(), 5);
        assert!((points[2][1] - 174.0).abs() < 1e-3, "y = {}", points[2][1]);
        // The final cumulative height reaches F = 1 (top of the plot, y = 30).
        assert!((points[4][1] - 30.0).abs() < 1e-3, "y = {}", points[4][1]);
    }

    #[test]
    fn non_finite_value_rejected() {
        let err = EcdfSpec::new(vec![EcdfSeries::new("a", vec![1.0, f32::NAN])])
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .unwrap_err();
        assert!(matches!(err, EcdfError::NonFiniteValue { .. }));
    }

    #[test]
    fn empty_label_rejected() {
        let err = EcdfSpec::new(vec![EcdfSeries::new("  ", vec![1.0, 2.0])])
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .unwrap_err();
        assert!(matches!(err, EcdfError::EmptyLabel(0)));
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        for size in [
            ChartSize::new(0, 0),
            ChartSize::new(1, 1),
            ChartSize::new(0, 300),
            ChartSize::new(400, 0),
        ] {
            let _ = EcdfSpec::new(vec![EcdfSeries::new("a", vec![1.0, 2.0, 3.0])])
                .try_build_chart(berthacharts_core::Workspace::new(), size);
        }
    }

    #[test]
    fn layout_matches_series_count() {
        let layout = EcdfSpec::new(vec![
            EcdfSeries::new("a", vec![1.0, 2.0]),
            EcdfSeries::new("b", vec![3.0, 4.0]),
        ])
        .layout(ChartSize::new(400, 300))
        .expect("layout");
        assert_eq!(layout.series.len(), 2);
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
