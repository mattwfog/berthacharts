//! Sparkline: a single tiny line chart with no axes, no legend, no labels.
//!
//! Designed for inline use — dashboards, table cells, headlines. Optional
//! decorations: baseline (zero line), min/max dot markers, area fill below
//! the curve, or first/last point dots for endpoint emphasis.

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, LabelTooltip, LabelTooltipRow, Layer, LayerId, LinePrim, LinearScale, Mark,
    MarkId, PathCommand, PathPrim, PickCtx, PickHit, PointPrim, Rect, Scale, ScaleId, Scene,
    SnapKind, SnapTarget, SnapTargetSet, TessellateCtx, Workspace,
};

const DATASET: DatasetId = DatasetId::new(0);
const LINE_MARK: MarkId = MarkId::new(1);
const DOTS_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One observed point in a sparkline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparklineDatum {
    /// X value (monotonic, typically a time index).
    pub x: f32,
    /// Y value.
    pub y: f32,
}

impl SparklineDatum {
    /// Build a datum.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Which decorative marks to draw alongside the main line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotMode {
    /// No additional dots.
    None,
    /// Highlight min + max.
    MinMax,
    /// Highlight first + last point.
    FirstLast,
    /// Highlight min, max, first, last.
    All,
}

/// Sparkline configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparklineOptions {
    /// Inner plot padding (pixels).
    pub padding: f32,
    /// Line stroke colour.
    pub stroke: [f32; 4],
    /// Line width.
    pub line_width: f32,
    /// Optional area fill colour below the curve.
    pub fill: Option<[f32; 4]>,
    /// Decorative dot mode.
    pub dots: DotMode,
    /// Dot fill colour (used for all decorative dots).
    pub dot_color: [f32; 4],
    /// Dot radius.
    pub dot_radius: f32,
    /// Draw a horizontal baseline at y = 0 if 0 is within the data's y range.
    pub baseline: bool,
    /// Baseline stroke colour.
    pub baseline_color: [f32; 4],
    /// Optional fixed y domain. `None` = data extent.
    pub y_domain: Option<(f32, f32)>,
}

impl Default for SparklineOptions {
    fn default() -> Self {
        Self {
            padding: 4.0,
            stroke: [0.30, 0.45, 0.85, 1.0],
            line_width: 1.5,
            fill: None,
            dots: DotMode::None,
            dot_color: [0.85, 0.30, 0.30, 1.0],
            dot_radius: 2.5,
            baseline: false,
            baseline_color: [0.6, 0.6, 0.65, 0.5],
            y_domain: None,
        }
    }
}

/// Sparkline chart spec.
#[derive(Debug, Clone)]
pub struct SparklineSpec {
    data: Vec<SparklineDatum>,
    options: SparklineOptions,
}

impl SparklineSpec {
    /// Build a sparkline spec.
    #[must_use]
    pub fn new(data: Vec<SparklineDatum>) -> Self {
        Self {
            data,
            options: SparklineOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: SparklineOptions) -> Self {
        self.options = options;
        self
    }
}

/// Errors during sparkline build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SparklineError {
    /// Spec has fewer than 2 points.
    InsufficientData,
}

impl fmt::Display for SparklineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData => write!(f, "sparkline needs at least 2 points"),
        }
    }
}

impl std::error::Error for SparklineError {}

/// Computed layout.
#[derive(Debug, Clone, PartialEq)]
pub struct SparklineLayout {
    /// Polyline points in screen pixels.
    pub points: Vec<[f32; 2]>,
    /// Baseline y in screen pixels, if drawn.
    pub baseline_y: Option<f32>,
    /// Min point index + screen position.
    pub min: ([f32; 2], usize),
    /// Max point index + screen position.
    pub max: ([f32; 2], usize),
    /// First and last point screen positions.
    pub first: [f32; 2],
    /// Last point screen position.
    pub last: [f32; 2],
    /// Plot rect (pixels, post-padding).
    pub plot: Rect,
}

impl ChartSpec for SparklineSpec {
    type Error = SparklineError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        if self.data.len() < 2 {
            return Err(SparklineError::InsufficientData);
        }
        let viewport = size.full_viewport();
        let layout = compute_layout(&self.data, &self.options, viewport.plot_area);

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

        let line_mark: Arc<dyn Mark> = Arc::new(SparklineLineMark::new(
            LINE_MARK,
            layout.clone(),
            self.options,
        ));
        let dots_mark: Arc<dyn Mark> = Arc::new(SparklineDotsMark::new(
            DOTS_MARK,
            layout.clone(),
            self.options.dots,
            self.options.dot_color,
            self.options.dot_radius,
        ));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![line_mark, dots_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, &self.data)).with_name("sparkline anchors"),
        ));
        if compact_labels_allowed(&layout) {
            let labels = compact_labels(&layout, &self.data);
            if !labels.is_empty() {
                let max_visible = labels.len();
                scene.guides.push(Guide::Labels(
                    LabelGuide::new(labels)
                        .with_collision_padding(2.0)
                        .with_max_visible(max_visible),
                ));
            }
        }

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn compute_layout(
    data: &[SparklineDatum],
    options: &SparklineOptions,
    plot: Rect,
) -> SparklineLayout {
    let pad = options.padding;
    let inner = Rect::new(
        plot.x + pad,
        plot.y + pad,
        (plot.w - 2.0 * pad).max(1.0),
        (plot.h - 2.0 * pad).max(1.0),
    );

    let x_min = data.iter().map(|d| d.x).fold(f32::INFINITY, f32::min);
    let x_max = data.iter().map(|d| d.x).fold(f32::NEG_INFINITY, f32::max);
    let (y_min_raw, y_max_raw) = match options.y_domain {
        Some((lo, hi)) => (lo, hi),
        None => (
            data.iter().map(|d| d.y).fold(f32::INFINITY, f32::min),
            data.iter().map(|d| d.y).fold(f32::NEG_INFINITY, f32::max),
        ),
    };
    let mut y_min = y_min_raw;
    let mut y_max = y_max_raw;
    if y_max <= y_min {
        y_max = y_min + 1.0;
    }
    let y_span = y_max - y_min;
    y_min -= y_span * 0.05;
    y_max += y_span * 0.05;

    let map_x = |v: f32| {
        if x_max > x_min {
            inner.x + (v - x_min) / (x_max - x_min) * inner.w
        } else {
            inner.x + inner.w * 0.5
        }
    };
    let map_y = |v: f32| inner.y + inner.h - (v - y_min) / (y_max - y_min) * inner.h;

    let points: Vec<[f32; 2]> = data.iter().map(|d| [map_x(d.x), map_y(d.y)]).collect();

    let (min_idx, max_idx) = {
        let mut lo = 0;
        let mut hi = 0;
        for (i, d) in data.iter().enumerate() {
            if d.y < data[lo].y {
                lo = i;
            }
            if d.y > data[hi].y {
                hi = i;
            }
        }
        (lo, hi)
    };

    let baseline_y = if options.baseline && y_min < 0.0 && y_max > 0.0 {
        Some(map_y(0.0))
    } else {
        None
    };

    SparklineLayout {
        first: *points.first().unwrap(),
        last: *points.last().unwrap(),
        min: (points[min_idx], min_idx),
        max: (points[max_idx], max_idx),
        baseline_y,
        points,
        plot: inner,
    }
}

fn dataset(layout: &SparklineLayout) -> Dataset {
    let mut x = Vec::with_capacity(layout.points.len());
    let mut y = Vec::with_capacity(layout.points.len());
    for p in &layout.points {
        x.push(p[0]);
        y.push(p[1]);
    }
    Dataset::new(
        DATASET,
        1,
        vec![
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

#[derive(Debug, Clone, Copy)]
struct SparklineAnchor {
    role: &'static str,
    index: usize,
    point: [f32; 2],
    label_anchor: LabelAnchor,
    label_priority: LabelPriority,
    snap_priority: i16,
}

fn semantic_anchors(layout: &SparklineLayout) -> Vec<SparklineAnchor> {
    let mut anchors = Vec::new();
    let mut seen = Vec::new();
    let mut push = |anchor: SparklineAnchor| {
        if !seen.contains(&anchor.index) {
            seen.push(anchor.index);
            anchors.push(anchor);
        }
    };

    push(SparklineAnchor {
        role: "start",
        index: 0,
        point: layout.first,
        label_anchor: LabelAnchor::Right,
        label_priority: LabelPriority::Required,
        snap_priority: 2,
    });
    push(SparklineAnchor {
        role: "min",
        index: layout.min.1,
        point: layout.min.0,
        label_anchor: LabelAnchor::Top,
        label_priority: LabelPriority::Important,
        snap_priority: 3,
    });
    push(SparklineAnchor {
        role: "max",
        index: layout.max.1,
        point: layout.max.0,
        label_anchor: LabelAnchor::Bottom,
        label_priority: LabelPriority::Important,
        snap_priority: 3,
    });
    push(SparklineAnchor {
        role: "end",
        index: layout.points.len() - 1,
        point: layout.last,
        label_anchor: LabelAnchor::Left,
        label_priority: LabelPriority::Required,
        snap_priority: 2,
    });

    anchors
}

fn snap_targets(layout: &SparklineLayout, data: &[SparklineDatum]) -> Vec<SnapTarget> {
    semantic_anchors(layout)
        .into_iter()
        .map(|anchor| {
            let datum = data[anchor.index];
            SnapTarget::new(anchor.point[0], anchor.point[1], SnapKind::Point)
                .with_radius(7.0)
                .with_label(format!("{} {:.1}", anchor.role, datum.y))
                .with_priority(anchor.snap_priority)
        })
        .collect()
}

fn compact_labels_allowed(layout: &SparklineLayout) -> bool {
    if layout.points.len() < 2 {
        return false;
    }
    let spacing = layout.plot.w / (layout.points.len() - 1) as f32;
    layout.points.len() <= 12 && spacing >= 12.0 && layout.plot.h >= 24.0
}

fn compact_labels(layout: &SparklineLayout, data: &[SparklineDatum]) -> Vec<LabelItem> {
    semantic_anchors(layout)
        .into_iter()
        .map(|anchor| {
            let datum = data[anchor.index];
            LabelItem::new(anchor.point[0], anchor.point[1], anchor.role)
                .with_detail(format!("{:.1}", datum.y))
                .with_anchor(anchor.label_anchor)
                .with_kind(LabelKind::Data)
                .with_priority(anchor.label_priority)
                .with_tooltip(LabelTooltip::new(
                    anchor.role,
                    vec![
                        LabelTooltipRow::new("X", format!("{:.1}", datum.x)),
                        LabelTooltipRow::new("Y", format!("{:.1}", datum.y)),
                    ],
                ))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct SparklineLineMark {
    id: MarkId,
    layout: SparklineLayout,
    options: SparklineOptions,
}

impl SparklineLineMark {
    fn new(id: MarkId, layout: SparklineLayout, options: SparklineOptions) -> Self {
        Self {
            id,
            layout,
            options,
        }
    }
}

impl Mark for SparklineLineMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.points.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut parts: Vec<Geometry> = Vec::new();

        // Area fill (optional): polygon from first.x along the line back to last.x at the bottom.
        if let Some(fill) = self.options.fill {
            if self.layout.points.len() >= 2 {
                let bottom_y = self.layout.plot.y + self.layout.plot.h;
                let mut commands = Vec::with_capacity(self.layout.points.len() + 3);
                let first = self.layout.points[0];
                commands.push(PathCommand::MoveTo {
                    x: first[0],
                    y: bottom_y,
                });
                for p in &self.layout.points {
                    commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
                }
                let last = *self.layout.points.last().unwrap();
                commands.push(PathCommand::LineTo {
                    x: last[0],
                    y: bottom_y,
                });
                commands.push(PathCommand::Close);
                parts.push(Geometry::Paths(vec![PathPrim {
                    commands,
                    fill,
                    stroke: [0.0, 0.0, 0.0, 0.0],
                    stroke_width: 0.0,
                }]));
            }
        }

        // Baseline (optional)
        if let Some(by) = self.layout.baseline_y {
            parts.push(Geometry::Lines(vec![LinePrim {
                points: vec![
                    [self.layout.plot.x, by],
                    [self.layout.plot.x + self.layout.plot.w, by],
                ],
                stroke: self.options.baseline_color,
                width: 1.0,
                dash: Some(vec![3.0, 3.0]),
                join: 1,
                cap: 1,
            }]));
        }

        // Main line
        parts.push(Geometry::Lines(vec![LinePrim {
            points: self.layout.points.clone(),
            stroke: self.options.stroke,
            width: self.options.line_width,
            dash: None,
            join: 1,
            cap: 1,
        }]));

        match parts.len() {
            0 => Geometry::Empty,
            1 => parts.into_iter().next().unwrap(),
            _ => Geometry::Mixed(parts),
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

#[derive(Debug, Clone)]
struct SparklineDotsMark {
    id: MarkId,
    layout: SparklineLayout,
    mode: DotMode,
    color: [f32; 4],
    radius: f32,
}

impl SparklineDotsMark {
    fn new(
        id: MarkId,
        layout: SparklineLayout,
        mode: DotMode,
        color: [f32; 4],
        radius: f32,
    ) -> Self {
        Self {
            id,
            layout,
            mode,
            color,
            radius,
        }
    }
}

impl Mark for SparklineDotsMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= match self.mode {
            DotMode::None => 0,
            DotMode::MinMax => 1,
            DotMode::FirstLast => 2,
            DotMode::All => 3,
        };
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        if matches!(self.mode, DotMode::None) {
            return Geometry::Empty;
        }
        let mut points = Vec::new();
        let mut push = |p: [f32; 2]| {
            points.push(PointPrim {
                x: p[0],
                y: p[1],
                r: self.radius,
                shape: 0,
                fill: self.color,
                stroke: [1.0, 1.0, 1.0, 0.7],
                stroke_width: 0.6,
            });
        };
        match self.mode {
            DotMode::MinMax => {
                push(self.layout.min.0);
                push(self.layout.max.0);
            }
            DotMode::FirstLast => {
                push(self.layout.first);
                push(self.layout.last);
            }
            DotMode::All => {
                push(self.layout.min.0);
                push(self.layout.max.0);
                push(self.layout.first);
                push(self.layout.last);
            }
            DotMode::None => {}
        }
        Geometry::Points(points)
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
    use berthacharts_core::Guide;

    #[test]
    fn requires_two_points() {
        let r = SparklineSpec::new(vec![SparklineDatum::new(0.0, 1.0)])
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(120, 30));
        assert!(matches!(r, Err(SparklineError::InsufficientData)));
    }

    #[test]
    fn builds_with_defaults() {
        let data: Vec<SparklineDatum> = (0..10)
            .map(|i| SparklineDatum::new(i as f32, (i as f32).sin()))
            .collect();
        let chart = SparklineSpec::new(data)
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(120, 30))
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn emits_endpoint_and_extrema_snap_targets_with_compact_labels_when_sparse() {
        let data = vec![
            SparklineDatum::new(0.0, 5.0),
            SparklineDatum::new(1.0, 10.0),
            SparklineDatum::new(2.0, 1.0),
            SparklineDatum::new(3.0, 8.0),
        ];

        let chart = SparklineSpec::new(data)
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(160, 48))
            .expect("chart");

        assert_eq!(chart.snap_targets().len(), 4);

        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        assert_eq!(labels.items.len(), 4);
        assert_eq!(labels.max_visible, Some(4));
        assert_eq!(labels.items[0].text, "start");
        assert_eq!(labels.items[1].text, "min");
        assert_eq!(labels.items[2].text, "max");
        assert_eq!(labels.items[3].text, "end");
    }

    #[test]
    fn omits_compact_labels_when_data_is_too_dense() {
        let data: Vec<SparklineDatum> = (0..24)
            .map(|i| SparklineDatum::new(i as f32, (i as f32 * 0.35).sin()))
            .collect();

        let chart = SparklineSpec::new(data)
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(120, 30))
            .expect("chart");

        assert_eq!(chart.snap_targets().len(), 4);
        assert!(chart
            .scene()
            .guides
            .iter()
            .all(|guide| !matches!(guide, Guide::Labels(_))));
    }

    #[test]
    fn layout_identifies_extrema() {
        let data = vec![
            SparklineDatum::new(0.0, 5.0),
            SparklineDatum::new(1.0, 10.0),
            SparklineDatum::new(2.0, 1.0),
            SparklineDatum::new(3.0, 8.0),
        ];
        let layout = compute_layout(
            &data,
            &SparklineOptions::default(),
            Rect::new(0.0, 0.0, 120.0, 30.0),
        );
        assert_eq!(layout.min.1, 2); // index of y=1
        assert_eq!(layout.max.1, 1); // index of y=10
    }

    #[test]
    fn baseline_emerges_when_zero_in_range() {
        let data = vec![
            SparklineDatum::new(0.0, -5.0),
            SparklineDatum::new(1.0, 5.0),
        ];
        let layout = compute_layout(
            &data,
            &SparklineOptions {
                baseline: true,
                ..SparklineOptions::default()
            },
            Rect::new(0.0, 0.0, 120.0, 30.0),
        );
        assert!(layout.baseline_y.is_some());
    }

    #[test]
    fn baseline_skipped_when_zero_outside_range() {
        let data = vec![
            SparklineDatum::new(0.0, 5.0),
            SparklineDatum::new(1.0, 10.0),
        ];
        let layout = compute_layout(
            &data,
            &SparklineOptions {
                baseline: true,
                ..SparklineOptions::default()
            },
            Rect::new(0.0, 0.0, 120.0, 30.0),
        );
        assert!(layout.baseline_y.is_none());
    }
}
