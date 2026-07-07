//! Violin plot: kernel-density mirror per group, drawn as a filled path.
//!
//! ## Statistical method
//!
//! Each group's density is a Gaussian kernel-density estimate evaluated at
//! [`ViolinOptions::resolution`] points spanning `[min − 1.5h, max + 1.5h]`,
//! so the tails close smoothly just past the data. The bandwidth `h` defaults
//! to Silverman's rule of thumb
//!
//! ```text
//! h = 0.9 · min(σ, IQR / 1.349) · n^(−1/5)
//! ```
//!
//! where `σ` is the (population) standard deviation and `IQR` the interquartile
//! range. Taking `min(σ, IQR/1.349)` makes the estimate robust to heavy tails
//! and outliers. It can be overridden with [`ViolinOptions::bandwidth`].
//!
//! Degenerate groups are handled without panicking: a single sample, or an
//! all-identical group (σ = 0 and IQR = 0), falls back to a unit bandwidth so
//! the violin renders a small finite blob rather than an infinitely thin
//! spike. Non-finite samples are rejected at build time with
//! [`ViolinError::NonFiniteValue`]. No external stats dependency — the KDE is
//! computed inline.
//!
//! ## Example
//!
//! ```
//! use berthacharts_dist::violin::{ViolinGroup, ViolinSpec};
//! use berthacharts_dist::core::{ChartSize, Workspace};
//!
//! let spec = ViolinSpec::new(vec![
//!     ViolinGroup::new("A", (1..=40).map(|i| i as f32).collect()),
//! ]);
//! let chart = spec
//!     .try_build_chart(Workspace::new(), ChartSize::new(480, 320))
//!     .expect("valid violin");
//! assert_eq!(chart.scene().layers.len(), 1);
//! ```

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LinearScale, Mark, MarkId, PathCommand, PathPrim, PickCtx,
    PickHit, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx,
    TooltipField, TooltipGuide, Workspace,
};

const GROUP_DATASET: DatasetId = DatasetId::new(0);
const VIOLIN_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One group of samples for a violin plot.
#[derive(Debug, Clone, PartialEq)]
pub struct ViolinGroup {
    /// Display label (x-axis category).
    pub label: String,
    /// Sample values.
    pub values: Vec<f32>,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
}

impl ViolinGroup {
    /// Build a violin group with a default colour.
    #[must_use]
    pub fn new(label: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
            color: [0.45, 0.55, 0.85, 0.7],
        }
    }

    /// Override colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// Violin plot configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViolinOptions {
    /// Plot padding (pixels).
    pub padding: f32,
    /// Number of density samples per violin (more = smoother, slower).
    pub resolution: usize,
    /// Width of each violin as fraction of slot (0..1).
    pub width_ratio: f32,
    /// Bandwidth override. `None` = Silverman's rule.
    pub bandwidth: Option<f32>,
}

impl Default for ViolinOptions {
    fn default() -> Self {
        Self {
            padding: 30.0,
            resolution: 64,
            width_ratio: 0.7,
            bandwidth: None,
        }
    }
}

/// Violin chart spec.
#[derive(Debug, Clone)]
pub struct ViolinSpec {
    groups: Vec<ViolinGroup>,
    options: ViolinOptions,
}

impl ViolinSpec {
    /// Build a violin spec.
    #[must_use]
    pub fn new(groups: Vec<ViolinGroup>) -> Self {
        Self {
            groups,
            options: ViolinOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: ViolinOptions) -> Self {
        self.options = options;
        self
    }

    /// Validate the groups without building a chart.
    ///
    /// Rejects an empty spec, empty groups, empty labels, and any non-finite
    /// sample value.
    pub fn validate(&self) -> Result<(), ViolinError> {
        if self.groups.is_empty() {
            return Err(ViolinError::Empty);
        }
        for (i, g) in self.groups.iter().enumerate() {
            if g.label.trim().is_empty() {
                return Err(ViolinError::EmptyLabel(i));
            }
            if g.values.is_empty() {
                return Err(ViolinError::EmptyGroup(i));
            }
            for &v in &g.values {
                if !v.is_finite() {
                    return Err(ViolinError::NonFiniteValue {
                        label: g.label.clone(),
                        value: v,
                    });
                }
            }
        }
        Ok(())
    }

    /// Compute the reusable violin layout without building a chart.
    pub fn layout(&self, size: ChartSize) -> Result<ViolinLayout, ViolinError> {
        self.validate()?;
        Ok(compute_layout(
            &self.groups,
            &self.options,
            size.full_viewport().plot_area,
        ))
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, ViolinError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }
}

/// Errors during violin build.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ViolinError {
    /// No groups supplied.
    Empty,
    /// Group has no samples.
    EmptyGroup(usize),
    /// Group at the given index has an empty or whitespace-only label.
    EmptyLabel(usize),
    /// A sample value was non-finite (NaN or infinity).
    NonFiniteValue {
        /// Label of the offending group.
        label: String,
        /// The offending value.
        value: f32,
    },
}

impl fmt::Display for ViolinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "violin plot has no groups"),
            Self::EmptyGroup(i) => write!(f, "group at index {i} has no samples"),
            Self::EmptyLabel(i) => write!(f, "group at index {i} has an empty label"),
            Self::NonFiniteValue { label, value } => {
                write!(f, "violin value for `{label}` is not finite: {value}")
            }
        }
    }
}

impl std::error::Error for ViolinError {}

/// Per-group density samples + screen positions.
#[derive(Debug, Clone, PartialEq)]
pub struct ViolinShape {
    /// Label.
    pub label: String,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// Centre x in screen pixels.
    pub center_x: f32,
    /// Polygon vertices in screen pixels (outline of the violin).
    pub polygon: Vec<[f32; 2]>,
    /// Number of source samples.
    pub count: usize,
    /// Minimum source sample value.
    pub min: f32,
    /// Median source sample value (type-7 linear interpolation).
    pub median: f32,
    /// Maximum source sample value.
    pub max: f32,
}

/// Computed violin layout.
#[derive(Debug, Clone, PartialEq)]
pub struct ViolinLayout {
    /// One per input group.
    pub shapes: Vec<ViolinShape>,
}

impl ChartSpec for ViolinSpec {
    type Error = ViolinError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;
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

        let mark: Arc<dyn Mark> = Arc::new(ViolinMark::new(VIOLIN_MARK, layout.clone()));
        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                VIOLIN_MARK,
                GROUP_DATASET,
                vec![
                    TooltipField::new("N", "count").as_integer(),
                    TooltipField::new("Median", "median").as_number(2),
                    TooltipField::new("Min", "min").as_number(2),
                    TooltipField::new("Max", "max").as_number(2),
                ],
            )
            .with_title_column("label"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(group_labels(&layout))
                .with_collision_padding(3.0)
                .with_max_visible(layout.shapes.len()),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout)).with_name("violin groups"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

/// Silverman's rule-of-thumb bandwidth: `h = 0.9 · min(σ, IQR/1.349) · n^(−1/5)`.
///
/// `sorted` must be ascending. Falls back to a unit bandwidth for a single
/// sample or a fully degenerate (zero-spread) group so the KDE stays finite.
fn silverman_bandwidth(sorted: &[f32]) -> f32 {
    let n = sorted.len();
    if n < 2 {
        return 1.0;
    }
    let nf = n as f32;
    let mean = sorted.iter().sum::<f32>() / nf;
    let var = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / nf;
    let stddev = var.sqrt();
    let iqr = quantile_sorted(sorted, 0.75) - quantile_sorted(sorted, 0.25);
    let spread = if iqr > 0.0 {
        stddev.min(iqr / 1.349)
    } else {
        stddev
    };
    let bw = 0.9 * spread * nf.powf(-0.2);
    if bw.is_finite() && bw > 0.0 {
        bw
    } else {
        1.0
    }
}

/// Type-7 (linear-interpolation) quantile of an ascending slice.
fn quantile_sorted(sorted: &[f32], q: f32) -> f32 {
    match sorted.len() {
        0 => f32::NAN,
        1 => sorted[0],
        len => {
            let pos = q * (len as f32 - 1.0);
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(len - 1);
            let frac = pos - lo as f32;
            sorted[lo] * (1.0 - frac) + sorted[hi] * frac
        }
    }
}

fn gaussian_kde(values: &[f32], at: f32, bandwidth: f32) -> f32 {
    let n = values.len() as f32;
    let inv = 1.0 / (n * bandwidth * (2.0_f32 * std::f32::consts::PI).sqrt());
    let mut s = 0.0_f32;
    for &v in values {
        let z = (at - v) / bandwidth;
        s += (-0.5 * z * z).exp();
    }
    s * inv
}

fn compute_layout(groups: &[ViolinGroup], options: &ViolinOptions, plot: Rect) -> ViolinLayout {
    let n = groups.len();
    let pad = options.padding;
    let inner = Rect::new(
        plot.x + pad,
        plot.y + pad,
        (plot.w - 2.0 * pad).max(1.0),
        (plot.h - 2.0 * pad).max(1.0),
    );

    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for g in groups {
        for &v in &g.values {
            if v.is_finite() {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
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
    let half_width = slot * options.width_ratio * 0.5;

    let shapes: Vec<ViolinShape> = groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let center_x = inner.x + (i as f32 + 0.5) * slot;
            // Values were validated finite before layout; sort once so the
            // bandwidth, median, and range all read from the same buffer.
            let mut sorted = g.values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let g_min = *sorted.first().unwrap_or(&0.0);
            let g_max = *sorted.last().unwrap_or(&0.0);
            let median = quantile_sorted(&sorted, 0.5);
            let bw = options
                .bandwidth
                .unwrap_or_else(|| silverman_bandwidth(&sorted));
            // Compute densities at resolution samples spanning the group's range.
            let lo = g_min - bw * 1.5;
            let hi = g_max + bw * 1.5;
            let res = options.resolution.max(8);
            let mut densities = Vec::with_capacity(res);
            let mut max_d = 0.0_f32;
            for s in 0..res {
                let t = s as f32 / (res - 1) as f32;
                let v = lo + (hi - lo) * t;
                let d = gaussian_kde(&sorted, v, bw);
                densities.push((v, d));
                if d > max_d {
                    max_d = d;
                }
            }
            // Normalise to half_width.
            let scale = if max_d > 0.0 { half_width / max_d } else { 0.0 };
            let mut polygon: Vec<[f32; 2]> = Vec::with_capacity(res * 2 + 2);
            // Right side bottom-up
            for &(v, d) in &densities {
                polygon.push([center_x + d * scale, map_y(v)]);
            }
            // Left side top-down (reverse)
            for &(v, d) in densities.iter().rev() {
                polygon.push([center_x - d * scale, map_y(v)]);
            }
            ViolinShape {
                label: g.label.clone(),
                color: g.color,
                center_x,
                polygon,
                count: g.values.len(),
                min: g_min,
                median,
                max: g_max,
            }
        })
        .collect();

    ViolinLayout { shapes }
}

fn group_dataset(layout: &ViolinLayout) -> Dataset {
    let mut label: Vec<Arc<str>> = Vec::new();
    let mut cx: Vec<f32> = Vec::new();
    let mut count: Vec<i64> = Vec::new();
    let mut min: Vec<f32> = Vec::new();
    let mut median: Vec<f32> = Vec::new();
    let mut max: Vec<f32> = Vec::new();
    for s in &layout.shapes {
        label.push(Arc::from(s.label.as_str()));
        cx.push(s.center_x);
        count.push(s.count as i64);
        min.push(s.min);
        median.push(s.median);
        max.push(s.max);
    }
    Dataset::new(
        GROUP_DATASET,
        1,
        vec![
            ("label".to_string(), Column::Utf8(ColumnData::new(label))),
            ("center_x".to_string(), Column::F32(ColumnData::new(cx))),
            ("count".to_string(), Column::I64(ColumnData::new(count))),
            ("min".to_string(), Column::F32(ColumnData::new(min))),
            ("median".to_string(), Column::F32(ColumnData::new(median))),
            ("max".to_string(), Column::F32(ColumnData::new(max))),
        ],
    )
}

fn group_labels(layout: &ViolinLayout) -> Vec<LabelItem> {
    layout
        .shapes
        .iter()
        .map(|shape| {
            let y = violin_bottom_y(shape) + 12.0;
            LabelItem::new(shape.center_x, y, shape.label.clone())
                .with_anchor(LabelAnchor::Bottom)
                .with_kind(LabelKind::Column)
                .with_priority(LabelPriority::Important)
        })
        .collect()
}

fn snap_targets(layout: &ViolinLayout) -> Vec<SnapTarget> {
    layout
        .shapes
        .iter()
        .map(|shape| {
            SnapTarget::new(shape.center_x, violin_mid_y(shape), SnapKind::Center)
                .with_radius(8.0)
                .with_label(format!("{} distribution", shape.label))
                .with_priority(2)
        })
        .collect()
}

fn violin_mid_y(shape: &ViolinShape) -> f32 {
    let (top, bottom) = violin_y_extent(shape);
    (top + bottom) * 0.5
}

fn violin_bottom_y(shape: &ViolinShape) -> f32 {
    violin_y_extent(shape).1
}

fn violin_y_extent(shape: &ViolinShape) -> (f32, f32) {
    shape.polygon.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(top, bottom), point| (top.min(point[1]), bottom.max(point[1])),
    )
}

#[derive(Debug, Clone)]
struct ViolinMark {
    id: MarkId,
    layout: ViolinLayout,
}

impl ViolinMark {
    fn new(id: MarkId, layout: ViolinLayout) -> Self {
        Self { id, layout }
    }
}

impl Mark for ViolinMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.shapes.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let paths: Vec<PathPrim> = self
            .layout
            .shapes
            .iter()
            .filter(|s| s.polygon.len() >= 3)
            .map(|s| {
                let mut cmds = Vec::with_capacity(s.polygon.len() + 1);
                let first = s.polygon[0];
                cmds.push(PathCommand::MoveTo {
                    x: first[0],
                    y: first[1],
                });
                for p in &s.polygon[1..] {
                    cmds.push(PathCommand::LineTo { x: p[0], y: p[1] });
                }
                cmds.push(PathCommand::Close);
                PathPrim {
                    commands: cmds,
                    fill: s.color,
                    stroke: [1.0, 1.0, 1.0, 0.7],
                    stroke_width: 0.6,
                }
            })
            .collect();
        if paths.is_empty() {
            Geometry::Empty
        } else {
            Geometry::Paths(paths)
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        for (row, s) in self.layout.shapes.iter().enumerate() {
            if point_in_polygon(&s.polygon, [px, py]) {
                return Some(PickHit {
                    mark: self.id,
                    row: Some(row),
                    distance: 0.0,
                    payload: None,
                });
            }
        }
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for s in &self.layout.shapes {
            for p in &s.polygon {
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

fn point_in_polygon(poly: &[[f32; 2]], p: [f32; 2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (poly[i][0], poly[i][1]);
        let (xj, yj) = (poly[j][0], poly[j][1]);
        if ((yi > p[1]) != (yj > p[1]))
            && (p[0] < (xj - xi) * (p[1] - yi) / ((yj - yi) + f32::EPSILON) + xi)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, Guide, SnapKind};

    #[test]
    fn empty_spec_rejected() {
        let r = ViolinSpec::new(vec![]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(ViolinError::Empty)));
    }

    #[test]
    fn empty_group_rejected() {
        let r = ViolinSpec::new(vec![ViolinGroup::new("a", vec![])]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(ViolinError::EmptyGroup(0))));
    }

    #[test]
    fn polygon_is_symmetric_around_center() {
        let g = vec![ViolinGroup::new("a", (1..=50).map(|i| i as f32).collect())];
        let layout = compute_layout(
            &g,
            &ViolinOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let s = &layout.shapes[0];
        let n = s.polygon.len();
        // The polygon is built right-side-up then left-side-down; should be
        // 2 × resolution vertices.
        assert!(n >= 16);
        // First and last should mirror around center_x: distance from first to
        // center should equal distance from center to last (opposite sides).
        let first = s.polygon[0];
        let last = s.polygon[n - 1];
        let right_offset = first[0] - s.center_x;
        let left_offset = s.center_x - last[0];
        assert!(
            (right_offset - left_offset).abs() < 1e-2,
            "right_offset {right_offset} != left_offset {left_offset}"
        );
    }

    #[test]
    fn silverman_falls_back_on_zero_spread() {
        assert_eq!(silverman_bandwidth(&[5.0; 10]), 1.0);
        assert_eq!(silverman_bandwidth(&[7.0]), 1.0);
    }

    #[test]
    fn silverman_is_positive_and_robust() {
        let mut spread: Vec<f32> = (1..=50).map(|i| i as f32).collect();
        spread.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let bw = silverman_bandwidth(&spread);
        assert!(bw.is_finite() && bw > 0.0, "bw = {bw}");
        // A single extreme outlier must not blow up the (robust) bandwidth: the
        // IQR/1.349 term caps growth versus a pure-σ rule.
        let mut with_outlier = spread.clone();
        with_outlier.push(10_000.0);
        with_outlier.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let bw_out = silverman_bandwidth(&with_outlier);
        assert!(bw_out < bw * 3.0, "bw_out {bw_out} vs bw {bw}");
    }

    #[test]
    fn non_finite_value_rejected() {
        let err = ViolinSpec::new(vec![ViolinGroup::new("a", vec![1.0, f32::INFINITY])])
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .unwrap_err();
        assert!(matches!(err, ViolinError::NonFiniteValue { .. }));
    }

    #[test]
    fn identical_values_yield_finite_polygon() {
        let layout = compute_layout(
            &[ViolinGroup::new("a", vec![5.0, 5.0, 5.0, 5.0])],
            &ViolinOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let shape = &layout.shapes[0];
        assert!(shape
            .polygon
            .iter()
            .all(|p| p[0].is_finite() && p[1].is_finite()));
        assert_eq!(shape.median, 5.0);
    }

    #[test]
    fn median_exposed_in_dataset() {
        let workspace = berthacharts_core::Workspace::new();
        ViolinSpec::new(vec![ViolinGroup::new("a", vec![1.0, 2.0, 3.0, 4.0, 5.0])])
            .try_build_chart(workspace.clone(), ChartSize::new(400, 300))
            .expect("chart");
        let dataset = workspace.dataset(GROUP_DATASET).expect("group dataset");
        let median = match dataset.column("median").expect("median").as_ref() {
            Column::F32(values) => values,
            other => panic!("expected f32 median column, got {}", other.dtype()),
        };
        assert!((median.values[0] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        for size in [
            ChartSize::new(0, 0),
            ChartSize::new(1, 1),
            ChartSize::new(0, 300),
            ChartSize::new(400, 0),
        ] {
            let _ = ViolinSpec::new(vec![ViolinGroup::new("a", vec![1.0, 2.0, 3.0])])
                .try_build_chart(berthacharts_core::Workspace::new(), size);
        }
    }

    #[test]
    fn build_chart_succeeds() {
        let groups = vec![
            ViolinGroup::new("A", (1..=30).map(|i| i as f32).collect()),
            ViolinGroup::new("B", (5..=35).map(|i| i as f32 * 0.9).collect()),
        ];
        let chart = ViolinSpec::new(groups)
            .build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(600, 400),
            )
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn build_chart_exposes_group_tooltips_labels_and_snap_targets() {
        let workspace = berthacharts_core::Workspace::new();
        let chart = ViolinSpec::new(vec![
            ViolinGroup::new("A", vec![1.0, 2.0, 3.0]),
            ViolinGroup::new("B", vec![2.0, 4.0, 6.0]),
        ])
        .build_chart(workspace.clone(), ChartSize::new(420, 300))
        .expect("chart");

        let tooltip = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Tooltip(tooltip) if tooltip.mark == VIOLIN_MARK => Some(tooltip),
                _ => None,
            })
            .expect("violin tooltip guide");
        assert_eq!(tooltip.title_column.as_deref(), Some("label"));
        assert!(tooltip.fields.iter().any(|field| field.column == "count"));
        assert!(tooltip.fields.iter().any(|field| field.column == "min"));
        assert!(tooltip.fields.iter().any(|field| field.column == "max"));

        let dataset = workspace.dataset(GROUP_DATASET).expect("group dataset");
        let count = match dataset.column("count").expect("count").as_ref() {
            Column::I64(values) => values,
            other => panic!("expected i64 count column, got {}", other.dtype()),
        };
        assert_eq!(count.values, vec![3, 3]);

        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        assert_eq!(labels.items.len(), 2);

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|target| target.kind == SnapKind::Center));
    }
}
