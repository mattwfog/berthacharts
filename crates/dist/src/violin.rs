//! Violin plot: kernel-density mirror per group, drawn as a filled path.
//!
//! Uses a simple Gaussian kernel with a Silverman's-rule bandwidth heuristic.
//! No external stats dependency — KDE is computed inline.

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Layer, LayerId, LinearScale, Mark, MarkId, PathCommand, PathPrim, PickCtx,
    PickHit, Rect, Scale, ScaleId, Scene, TessellateCtx, Workspace,
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
}

/// Errors during violin build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolinError {
    /// No groups supplied.
    Empty,
    /// Group has no samples.
    EmptyGroup(usize),
}

impl fmt::Display for ViolinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "violin plot has no groups"),
            Self::EmptyGroup(i) => write!(f, "group at index {i} has no samples"),
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
        if self.groups.is_empty() {
            return Err(ViolinError::Empty);
        }
        for (i, g) in self.groups.iter().enumerate() {
            if g.values.is_empty() {
                return Err(ViolinError::EmptyGroup(i));
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

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn silverman_bandwidth(values: &[f32]) -> f32 {
    let n = values.len() as f32;
    if n < 2.0 {
        return 1.0;
    }
    let mean = values.iter().sum::<f32>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let stddev = var.sqrt();
    let bw = 1.06 * stddev * n.powf(-0.2);
    bw.max(1e-3)
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

fn compute_layout(
    groups: &[ViolinGroup],
    options: &ViolinOptions,
    plot: Rect,
) -> ViolinLayout {
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
            let bw = options.bandwidth.unwrap_or_else(|| silverman_bandwidth(&g.values));
            // Compute densities at resolution samples spanning the group's range.
            let g_min = g.values.iter().copied().fold(f32::INFINITY, f32::min);
            let g_max = g.values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let lo = g_min - bw * 1.5;
            let hi = g_max + bw * 1.5;
            let res = options.resolution.max(8);
            let mut densities = Vec::with_capacity(res);
            let mut max_d = 0.0_f32;
            for s in 0..res {
                let t = s as f32 / (res - 1) as f32;
                let v = lo + (hi - lo) * t;
                let d = gaussian_kde(&g.values, v, bw);
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
            }
        })
        .collect();

    ViolinLayout { shapes }
}

fn group_dataset(layout: &ViolinLayout) -> Dataset {
    let mut label: Vec<Arc<str>> = Vec::new();
    let mut cx: Vec<f32> = Vec::new();
    for s in &layout.shapes {
        label.push(Arc::from(s.label.as_str()));
        cx.push(s.center_x);
    }
    Dataset::new(
        GROUP_DATASET,
        1,
        vec![
            ("label".to_string(), Column::Utf8(ColumnData::new(label))),
            ("center_x".to_string(), Column::F32(ColumnData::new(cx))),
        ],
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

    #[test]
    fn empty_spec_rejected() {
        let r = ViolinSpec::new(vec![])
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(400, 300));
        assert!(matches!(r, Err(ViolinError::Empty)));
    }

    #[test]
    fn empty_group_rejected() {
        let r = ViolinSpec::new(vec![ViolinGroup::new("a", vec![])])
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(400, 300));
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
    fn build_chart_succeeds() {
        let groups = vec![
            ViolinGroup::new("A", (1..=30).map(|i| i as f32).collect()),
            ViolinGroup::new("B", (5..=35).map(|i| i as f32 * 0.9).collect()),
        ];
        let chart = ViolinSpec::new(groups)
            .build_chart(berthacharts_core::Workspace::new(), ChartSize::new(600, 400))
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }
}
