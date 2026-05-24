//! Box-and-whisker plot. One box per group; whiskers extend to the
//! farthest non-outlier point inside `whisker_factor × IQR`. Outliers render
//! as individual points.

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Layer, LayerId, LinePrim, LinearScale, Mark, MarkId, PickCtx, PickHit,
    PointPrim, Rect, RectPrim, Scale, ScaleId, Scene, TessellateCtx, Workspace,
};

const GROUP_DATASET: DatasetId = DatasetId::new(0);
const OUTLIER_DATASET: DatasetId = DatasetId::new(1);
const BOX_MARK: MarkId = MarkId::new(1);
const OUTLIER_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One group of samples for the box plot.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPlotGroup {
    /// Group label (x-axis category).
    pub label: String,
    /// Sample values.
    pub values: Vec<f32>,
    /// Premultiplied RGBA fill for the box.
    pub color: [f32; 4],
}

impl BoxPlotGroup {
    /// Build a group with a default colour.
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

/// Box plot configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxPlotOptions {
    /// Whisker extension factor (1.5 = Tukey's classic IQR fence).
    pub whisker_factor: f32,
    /// Box width as fraction of slot (0..1).
    pub box_width_ratio: f32,
    /// Plot padding (pixels).
    pub padding: f32,
    /// Whisker line width.
    pub line_width: f32,
    /// Outlier point radius.
    pub outlier_radius: f32,
}

impl Default for BoxPlotOptions {
    fn default() -> Self {
        Self {
            whisker_factor: 1.5,
            box_width_ratio: 0.6,
            padding: 30.0,
            line_width: 1.2,
            outlier_radius: 2.5,
        }
    }
}

/// Computed quartiles + whisker bounds for one group.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPlotStats {
    /// First quartile (25th percentile).
    pub q1: f32,
    /// Median (50th).
    pub median: f32,
    /// Third quartile (75th).
    pub q3: f32,
    /// Lower whisker tip (smallest non-outlier).
    pub lower_whisker: f32,
    /// Upper whisker tip (largest non-outlier).
    pub upper_whisker: f32,
    /// Points outside the whisker bounds.
    pub outliers: Vec<f32>,
    /// Total sample count.
    pub count: usize,
}

/// Box plot chart spec.
#[derive(Debug, Clone)]
pub struct BoxPlotSpec {
    groups: Vec<BoxPlotGroup>,
    options: BoxPlotOptions,
}

impl BoxPlotSpec {
    /// Build a box-plot spec from groups.
    #[must_use]
    pub fn new(groups: Vec<BoxPlotGroup>) -> Self {
        Self {
            groups,
            options: BoxPlotOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: BoxPlotOptions) -> Self {
        self.options = options;
        self
    }
}

/// Errors during box-plot build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoxPlotError {
    /// No groups supplied.
    Empty,
    /// Group at the given index has zero samples.
    EmptyGroup(usize),
}

impl fmt::Display for BoxPlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "box plot has no groups"),
            Self::EmptyGroup(i) => write!(f, "group at index {i} has no samples"),
        }
    }
}

impl std::error::Error for BoxPlotError {}

/// Per-group layout output.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPlotLayoutGroup {
    /// Original label.
    pub label: String,
    /// Original colour.
    pub color: [f32; 4],
    /// Statistics summary.
    pub stats: BoxPlotStats,
    /// Centre x in screen pixels.
    pub center_x: f32,
    /// Pixel y for each statistic (mapped).
    pub y_q1: f32,
    /// Pixel y for the median.
    pub y_median: f32,
    /// Pixel y for q3.
    pub y_q3: f32,
    /// Pixel y for the lower whisker.
    pub y_lower: f32,
    /// Pixel y for the upper whisker.
    pub y_upper: f32,
    /// Pixel ys for each outlier.
    pub y_outliers: Vec<f32>,
}

/// Whole-chart layout output.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxPlotLayout {
    /// Per-group computed positions.
    pub groups: Vec<BoxPlotLayoutGroup>,
    /// Bar width in pixels.
    pub box_width: f32,
}

impl ChartSpec for BoxPlotSpec {
    type Error = BoxPlotError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        if self.groups.is_empty() {
            return Err(BoxPlotError::Empty);
        }
        for (i, g) in self.groups.iter().enumerate() {
            if g.values.is_empty() {
                return Err(BoxPlotError::EmptyGroup(i));
            }
        }
        let viewport = size.full_viewport();
        let layout = compute_layout(&self.groups, &self.options, viewport.plot_area);

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
        workspace.upsert_dataset(group_dataset(&layout));
        workspace.upsert_dataset(outlier_dataset(&layout));

        let box_mark: Arc<dyn Mark> = Arc::new(BoxMark::new(
            BOX_MARK,
            layout.clone(),
            self.options,
        ));
        let outlier_mark: Arc<dyn Mark> = Arc::new(OutlierMark::new(
            OUTLIER_MARK,
            layout.clone(),
            self.options.outlier_radius,
        ));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![box_mark, outlier_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

/// Compute Tukey-style quartiles + whiskers for a single sample.
#[must_use]
pub fn compute_stats(values: &[f32], whisker_factor: f32) -> BoxPlotStats {
    let mut v: Vec<f32> = values.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    let q1 = percentile(&v, 0.25);
    let median = percentile(&v, 0.50);
    let q3 = percentile(&v, 0.75);
    let iqr = q3 - q1;
    let fence_low = q1 - whisker_factor * iqr;
    let fence_high = q3 + whisker_factor * iqr;
    let mut lower_whisker = q1;
    let mut upper_whisker = q3;
    let mut outliers = Vec::new();
    for &x in &v {
        if x < fence_low || x > fence_high {
            outliers.push(x);
        } else {
            if x < lower_whisker {
                lower_whisker = x;
            }
            if x > upper_whisker {
                upper_whisker = x;
            }
        }
    }
    BoxPlotStats {
        q1,
        median,
        q3,
        lower_whisker,
        upper_whisker,
        outliers,
        count: n,
    }
}

fn percentile(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return f32::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q * (sorted.len() as f32 - 1.0);
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - lo as f32;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

fn compute_layout(
    groups: &[BoxPlotGroup],
    options: &BoxPlotOptions,
    plot: Rect,
) -> BoxPlotLayout {
    let n = groups.len();
    let pad = options.padding;
    let inner = Rect::new(
        plot.x + pad,
        plot.y + pad,
        (plot.w - 2.0 * pad).max(1.0),
        (plot.h - 2.0 * pad).max(1.0),
    );

    let stats: Vec<BoxPlotStats> = groups
        .iter()
        .map(|g| compute_stats(&g.values, options.whisker_factor))
        .collect();

    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for s in &stats {
        y_min = y_min.min(s.lower_whisker);
        y_max = y_max.max(s.upper_whisker);
        for &o in &s.outliers {
            y_min = y_min.min(o);
            y_max = y_max.max(o);
        }
    }
    if y_max <= y_min {
        y_max = y_min + 1.0;
    }
    let span = y_max - y_min;
    y_min -= span * 0.05;
    y_max += span * 0.05;

    let map_y = |v: f32| inner.y + inner.h - (v - y_min) / (y_max - y_min) * inner.h;

    let slot = inner.w / n as f32;
    let box_width = (slot * options.box_width_ratio).max(2.0);

    let layout_groups: Vec<BoxPlotLayoutGroup> = groups
        .iter()
        .zip(stats.into_iter())
        .enumerate()
        .map(|(i, (g, s))| {
            let center_x = inner.x + (i as f32 + 0.5) * slot;
            BoxPlotLayoutGroup {
                label: g.label.clone(),
                color: g.color,
                y_q1: map_y(s.q1),
                y_median: map_y(s.median),
                y_q3: map_y(s.q3),
                y_lower: map_y(s.lower_whisker),
                y_upper: map_y(s.upper_whisker),
                y_outliers: s.outliers.iter().map(|&v| map_y(v)).collect(),
                stats: s,
                center_x,
            }
        })
        .collect();

    BoxPlotLayout {
        groups: layout_groups,
        box_width,
    }
}

fn group_dataset(layout: &BoxPlotLayout) -> Dataset {
    let mut label: Vec<Arc<str>> = Vec::new();
    let mut median: Vec<f32> = Vec::new();
    let mut q1: Vec<f32> = Vec::new();
    let mut q3: Vec<f32> = Vec::new();
    let mut count: Vec<i64> = Vec::new();
    for g in &layout.groups {
        label.push(Arc::from(g.label.as_str()));
        median.push(g.stats.median);
        q1.push(g.stats.q1);
        q3.push(g.stats.q3);
        count.push(g.stats.count as i64);
    }
    Dataset::new(
        GROUP_DATASET,
        1,
        vec![
            ("label".to_string(), Column::Utf8(ColumnData::new(label))),
            ("median".to_string(), Column::F32(ColumnData::new(median))),
            ("q1".to_string(), Column::F32(ColumnData::new(q1))),
            ("q3".to_string(), Column::F32(ColumnData::new(q3))),
            ("count".to_string(), Column::I64(ColumnData::new(count))),
        ],
    )
}

fn outlier_dataset(layout: &BoxPlotLayout) -> Dataset {
    let mut g_idx: Vec<i64> = Vec::new();
    let mut x: Vec<f32> = Vec::new();
    let mut y: Vec<f32> = Vec::new();
    for (gi, g) in layout.groups.iter().enumerate() {
        for &v in &g.y_outliers {
            g_idx.push(gi as i64);
            x.push(g.center_x);
            y.push(v);
        }
    }
    Dataset::new(
        OUTLIER_DATASET,
        1,
        vec![
            ("group".to_string(), Column::I64(ColumnData::new(g_idx))),
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

#[derive(Debug, Clone)]
struct BoxMark {
    id: MarkId,
    layout: BoxPlotLayout,
    options: BoxPlotOptions,
}

impl BoxMark {
    fn new(id: MarkId, layout: BoxPlotLayout, options: BoxPlotOptions) -> Self {
        Self {
            id,
            layout,
            options,
        }
    }
}

impl Mark for BoxMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.groups.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut rects: Vec<RectPrim> = Vec::with_capacity(self.layout.groups.len());
        let mut lines: Vec<LinePrim> = Vec::with_capacity(self.layout.groups.len() * 4);
        let half = self.layout.box_width * 0.5;
        for g in &self.layout.groups {
            // IQR box from q3 (top) to q1 (bottom)
            let body_top = g.y_q3.min(g.y_q1);
            let body_h = (g.y_q1 - g.y_q3).abs().max(1.0);
            rects.push(RectPrim {
                x: g.center_x - half,
                y: body_top,
                w: self.layout.box_width,
                h: body_h,
                fill: g.color,
                stroke: [1.0, 1.0, 1.0, 0.7],
                stroke_width: 1.0,
                radius: 0.0,
            });
            // median line across the box
            lines.push(LinePrim {
                points: vec![
                    [g.center_x - half, g.y_median],
                    [g.center_x + half, g.y_median],
                ],
                stroke: [1.0, 1.0, 1.0, 0.95],
                width: self.options.line_width + 0.4,
                dash: None,
                join: 1,
                cap: 1,
            });
            // upper whisker stem + cap
            lines.push(LinePrim {
                points: vec![[g.center_x, g.y_q3], [g.center_x, g.y_upper]],
                stroke: g.color,
                width: self.options.line_width,
                dash: None,
                join: 1,
                cap: 1,
            });
            lines.push(LinePrim {
                points: vec![
                    [g.center_x - half * 0.6, g.y_upper],
                    [g.center_x + half * 0.6, g.y_upper],
                ],
                stroke: g.color,
                width: self.options.line_width,
                dash: None,
                join: 1,
                cap: 1,
            });
            // lower whisker stem + cap
            lines.push(LinePrim {
                points: vec![[g.center_x, g.y_q1], [g.center_x, g.y_lower]],
                stroke: g.color,
                width: self.options.line_width,
                dash: None,
                join: 1,
                cap: 1,
            });
            lines.push(LinePrim {
                points: vec![
                    [g.center_x - half * 0.6, g.y_lower],
                    [g.center_x + half * 0.6, g.y_lower],
                ],
                stroke: g.color,
                width: self.options.line_width,
                dash: None,
                join: 1,
                cap: 1,
            });
        }
        Geometry::Mixed(vec![Geometry::Rects(rects), Geometry::Lines(lines)])
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        let half = self.layout.box_width * 0.5;
        for (row, g) in self.layout.groups.iter().enumerate() {
            if (px - g.center_x).abs() <= half {
                let top = g.y_upper.min(g.y_lower);
                let bot = g.y_upper.max(g.y_lower);
                if py >= top && py <= bot {
                    return Some(PickHit {
                        mark: self.id,
                        row: Some(row),
                        distance: 0.0,
                        payload: None,
                    });
                }
            }
        }
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.layout.groups.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let half = self.layout.box_width * 0.5;
        for g in &self.layout.groups {
            min_x = min_x.min(g.center_x - half);
            max_x = max_x.max(g.center_x + half);
            min_y = min_y.min(g.y_upper).min(g.y_lower);
            max_y = max_y.max(g.y_upper).max(g.y_lower);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
struct OutlierMark {
    id: MarkId,
    layout: BoxPlotLayout,
    radius: f32,
}

impl OutlierMark {
    fn new(id: MarkId, layout: BoxPlotLayout, radius: f32) -> Self {
        Self {
            id,
            layout,
            radius,
        }
    }
}

impl Mark for OutlierMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        let outlier_count: usize = self.layout.groups.iter().map(|g| g.y_outliers.len()).sum();
        h ^= outlier_count as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut points = Vec::new();
        for g in &self.layout.groups {
            for &y in &g.y_outliers {
                points.push(PointPrim {
                    x: g.center_x,
                    y,
                    r: self.radius,
                    shape: 0,
                    fill: g.color,
                    stroke: [1.0, 1.0, 1.0, 0.7],
                    stroke_width: 0.6,
                });
            }
        }
        Geometry::Points(points)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.layout.groups.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for g in &self.layout.groups {
            for &y in &g.y_outliers {
                min_x = min_x.min(g.center_x);
                max_x = max_x.max(g.center_x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        if min_x.is_infinite() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let pad = self.radius * 2.0;
        Rect::new(min_x - pad, min_y - pad, max_x - min_x + 2.0 * pad, max_y - min_y + 2.0 * pad)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_spec_rejected() {
        let result = BoxPlotSpec::new(vec![])
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(400, 300));
        assert!(matches!(result, Err(BoxPlotError::Empty)));
    }

    #[test]
    fn empty_group_rejected() {
        let result = BoxPlotSpec::new(vec![BoxPlotGroup::new("a", vec![])])
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(400, 300));
        assert!(matches!(result, Err(BoxPlotError::EmptyGroup(0))));
    }

    #[test]
    fn percentile_matches_known() {
        let v: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&v, 0.0) - 1.0).abs() < 1e-5);
        assert!((percentile(&v, 0.50) - 3.0).abs() < 1e-5);
        assert!((percentile(&v, 1.0) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn stats_separate_outliers() {
        // 1..10 has median ~5.5, IQR 5. 100 should be an outlier.
        let v: Vec<f32> = (1..=10).map(|i| i as f32).chain(std::iter::once(100.0)).collect();
        let s = compute_stats(&v, 1.5);
        assert!(s.outliers.contains(&100.0));
        assert!(s.upper_whisker < 100.0);
    }

    #[test]
    fn build_chart_succeeds() {
        let groups = vec![
            BoxPlotGroup::new("A", (1..=20).map(|i| i as f32).collect()),
            BoxPlotGroup::new("B", (5..=25).map(|i| i as f32 * 1.2).collect()),
        ];
        let chart = BoxPlotSpec::new(groups)
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(600, 400))
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }
}
