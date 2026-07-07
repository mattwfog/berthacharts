//! Chord chart: nodes placed on a circle's perimeter, weighted ribbons drawn
//! between them through the disk. Useful for showing pairwise relationships
//! (matrix-like) without drowning in lines.
//!
//! Build a spec directly from [`ChordNode`]s and [`ChordLink`]s, or from a
//! square weighted adjacency matrix via [`ChordSpec::from_matrix`]. Degenerate
//! inputs (empty, single group, zero-sum rows, dense group counts) lay out
//! without panicking; non-finite or negative link values are rejected up front
//! by [`ChordSpec`]'s validation rather than poisoning the geometry.

use std::f32::consts::TAU;
use std::fmt;
use std::sync::Arc;

use ahash::AHashMap;
use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LinearScale, Mark, MarkId, PathCommand, PathPrim, PickCtx,
    PickHit, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx,
    TooltipField, TooltipGuide, Workspace,
};

const NODE_DATASET: DatasetId = DatasetId::new(0);
const LINK_DATASET: DatasetId = DatasetId::new(1);
const ARC_MARK: MarkId = MarkId::new(1);
const RIBBON_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// A node placed on the chord perimeter.
#[derive(Debug, Clone, PartialEq)]
pub struct ChordNode {
    /// Stable id used by links.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Premultiplied RGBA fill for the arc.
    pub color: [f32; 4],
}

impl ChordNode {
    /// Build a node with default colour.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
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

/// A weighted ribbon between two nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct ChordLink {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Magnitude — drives ribbon width.
    pub value: f32,
    /// Optional premultiplied RGBA. `None` inherits from source-node colour.
    pub color: Option<[f32; 4]>,
}

impl ChordLink {
    /// Build a link with inherited colour.
    #[must_use]
    pub fn new(source: impl Into<String>, target: impl Into<String>, value: f32) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value,
            color: None,
        }
    }

    /// Override colour explicitly.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = Some(color);
        self
    }
}

/// Chord chart configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChordOptions {
    /// Outer ring inset from the plot box edges (pixels).
    pub padding: f32,
    /// Arc-band thickness (pixels). Arc renders between inner & outer radii.
    pub arc_thickness: f32,
    /// Angular gap between adjacent node arcs (radians). Typical 0.02–0.06.
    pub arc_gap: f32,
    /// Ribbon opacity multiplier (overrides link.color alpha).
    pub ribbon_opacity: f32,
    /// Render node labels.
    pub show_labels: bool,
    /// Label distance from the outer arc edge (pixels).
    pub label_offset: f32,
}

impl Default for ChordOptions {
    fn default() -> Self {
        Self {
            padding: 60.0,
            arc_thickness: 16.0,
            arc_gap: 0.035,
            ribbon_opacity: 0.55,
            show_labels: true,
            label_offset: 10.0,
        }
    }
}

impl ChordOptions {
    /// Set the outer-ring inset from the plot box edges (pixels).
    #[must_use]
    pub const fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the arc-band thickness (pixels).
    #[must_use]
    pub const fn with_arc_thickness(mut self, arc_thickness: f32) -> Self {
        self.arc_thickness = arc_thickness;
        self
    }

    /// Set the angular gap between adjacent node arcs (radians).
    #[must_use]
    pub const fn with_arc_gap(mut self, arc_gap: f32) -> Self {
        self.arc_gap = arc_gap;
        self
    }

    /// Set the ribbon opacity multiplier.
    #[must_use]
    pub const fn with_ribbon_opacity(mut self, ribbon_opacity: f32) -> Self {
        self.ribbon_opacity = ribbon_opacity;
        self
    }

    /// Toggle node-label rendering.
    #[must_use]
    pub const fn with_labels(mut self, show_labels: bool) -> Self {
        self.show_labels = show_labels;
        self
    }

    /// Set the label distance from the outer arc edge (pixels).
    #[must_use]
    pub const fn with_label_offset(mut self, label_offset: f32) -> Self {
        self.label_offset = label_offset;
        self
    }
}

/// Chord chart spec.
#[derive(Debug, Clone)]
pub struct ChordSpec {
    nodes: Vec<ChordNode>,
    links: Vec<ChordLink>,
    options: ChordOptions,
}

impl ChordSpec {
    /// Build a chord spec from nodes + links.
    #[must_use]
    pub fn new(nodes: Vec<ChordNode>, links: Vec<ChordLink>) -> Self {
        Self {
            nodes,
            links,
            options: ChordOptions::default(),
        }
    }

    /// Build a chord spec from a square weighted adjacency matrix.
    ///
    /// `labels` names each group in row/column order; `matrix[i][j]` is the
    /// weight flowing from group `i` to group `j`. One [`ChordLink`] is emitted
    /// per finite, strictly positive cell — including the diagonal, which
    /// becomes a self-link. Zero, negative, and non-finite cells are skipped, so
    /// a sparse matrix stays sparse. A symmetric matrix therefore yields a
    /// reciprocal ribbon for each direction; pre-sum the two triangles if you
    /// want a single undirected ribbon per pair.
    ///
    /// Ragged input never panics: rows shorter than `labels` are treated as
    /// zero-padded and surplus columns are ignored. Node ids are the row index
    /// as a string, so repeated labels do not collide, and each node is colored
    /// from a built-in categorical palette.
    #[must_use]
    pub fn from_matrix(labels: Vec<String>, matrix: &[Vec<f32>]) -> Self {
        let n = labels.len();
        let nodes = labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                ChordNode::new(i.to_string(), label.clone()).with_color(node_palette(i))
            })
            .collect();

        let mut links = Vec::new();
        for i in 0..n {
            let row = matrix.get(i);
            for j in 0..n {
                let value = row.and_then(|r| r.get(j)).copied().unwrap_or(0.0);
                if value.is_finite() && value > 0.0 {
                    links.push(ChordLink::new(i.to_string(), j.to_string(), value));
                }
            }
        }

        Self::new(nodes, links)
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: ChordOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute the reusable layout without building a chart.
    ///
    /// Runs the same validation as [`ChartSpec::build_chart`], so it surfaces
    /// [`ChordError`] for degenerate or invalid input before any scene is built.
    pub fn layout(&self, size: ChartSize) -> Result<ChordLayout, ChordError> {
        validate(&self.nodes, &self.links)?;
        let plot = size.full_viewport().plot_area;
        compute_layout(&self.nodes, &self.links, &self.options, plot)
    }

    /// Compile this spec into a chart without importing [`ChartSpec`] at the
    /// call site.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, ChordError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }
}

/// Computed chord positions.
#[derive(Debug, Clone, PartialEq)]
pub struct ChordLayout {
    /// One per node, in input order.
    pub arcs: Vec<ChordArc>,
    /// One per link, in input order.
    pub ribbons: Vec<ChordRibbon>,
    /// Plot centre.
    pub center: [f32; 2],
    /// Inner radius (where ribbons start).
    pub inner_radius: f32,
    /// Outer radius (where arc band ends).
    pub outer_radius: f32,
}

/// A node's arc on the perimeter.
#[derive(Debug, Clone, PartialEq)]
pub struct ChordArc {
    /// Matches input node id.
    pub node_id: String,
    /// Display label.
    pub label: String,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// Start angle in radians. `0` points east (+x); the angle increases
    /// clockwise on screen because the pixel y-axis points down.
    pub start_angle: f32,
    /// End angle in radians. Always `>= start_angle`; a zero-length span marks
    /// a node with no incident links.
    pub end_angle: f32,
    /// Sum of link values touching this node (both endpoints) — drives arc
    /// length.
    pub total_value: f32,
}

/// A ribbon connecting two node arcs.
#[derive(Debug, Clone, PartialEq)]
pub struct ChordRibbon {
    /// Angular start on the source arc.
    pub source_start: f32,
    /// Angular end on the source arc.
    pub source_end: f32,
    /// Angular start on the target arc.
    pub target_start: f32,
    /// Angular end on the target arc.
    pub target_end: f32,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// Link value.
    pub value: f32,
}

/// Errors during chord build.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ChordError {
    /// Link references an unknown node id.
    UnknownNode(String),
    /// Duplicate node id.
    DuplicateNode(String),
    /// A link value is negative or non-finite (NaN/∞). Zero is allowed — it is
    /// simply an absent relationship — but a bad magnitude would poison the arc
    /// geometry, so it is rejected up front.
    InvalidValue {
        /// Source id of the offending link.
        source: String,
        /// Target id of the offending link.
        target: String,
        /// The offending value.
        value: f32,
    },
    /// No nodes, or all link values zero — nothing to lay out.
    Empty,
}

impl fmt::Display for ChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "link references unknown node id: {id}"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
            Self::InvalidValue {
                source,
                target,
                value,
            } => write!(f, "link `{source}` to `{target}` has invalid value {value}"),
            Self::Empty => write!(f, "chord has no nodes or no nonzero links"),
        }
    }
}

impl std::error::Error for ChordError {}

impl ChartSpec for ChordSpec {
    type Error = ChordError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        validate(&self.nodes, &self.links)?;
        let viewport = size.full_viewport();
        let plot = viewport.plot_area;
        let layout = compute_layout(&self.nodes, &self.links, &self.options, plot)?;

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
        workspace.upsert_dataset(node_dataset(&layout));
        workspace.upsert_dataset(link_dataset(&layout, &self.links));

        let ribbon_mark: Arc<dyn Mark> = Arc::new(ChordRibbonMark::new(
            RIBBON_MARK,
            layout.clone(),
            self.options.ribbon_opacity,
        ));
        let arc_mark: Arc<dyn Mark> = Arc::new(ChordArcMark::new(ARC_MARK, layout.clone()));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![ribbon_mark, arc_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });

        if self.options.show_labels {
            let labels = build_labels(&layout, self.options.label_offset);
            if !labels.is_empty() {
                scene.guides.push(Guide::Labels(
                    LabelGuide::new(labels).with_collision_padding(3.0),
                ));
            }
        }
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                ARC_MARK,
                NODE_DATASET,
                vec![
                    TooltipField::new("Id", "id").as_label(),
                    TooltipField::new("Total", "total"),
                    TooltipField::new("Share", "share").as_percent(1),
                ],
            )
            .with_title_column("label"),
        ));
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                RIBBON_MARK,
                LINK_DATASET,
                vec![
                    TooltipField::new("Source", "source").as_label(),
                    TooltipField::new("Target", "target").as_label(),
                    TooltipField::new("Value", "value"),
                    TooltipField::new("Share", "share").as_percent(1),
                    TooltipField::new("Source %", "source_share").as_percent(1),
                ],
            )
            .with_title_column("link"),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, &self.links)).with_name("chord anchors"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn validate(nodes: &[ChordNode], links: &[ChordLink]) -> Result<(), ChordError> {
    if nodes.is_empty() {
        return Err(ChordError::Empty);
    }
    let mut seen = AHashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if seen.insert(n.id.as_str(), i).is_some() {
            return Err(ChordError::DuplicateNode(n.id.clone()));
        }
    }
    for l in links {
        if !seen.contains_key(l.source.as_str()) {
            return Err(ChordError::UnknownNode(l.source.clone()));
        }
        if !seen.contains_key(l.target.as_str()) {
            return Err(ChordError::UnknownNode(l.target.clone()));
        }
        if !l.value.is_finite() || l.value < 0.0 {
            return Err(ChordError::InvalidValue {
                source: l.source.clone(),
                target: l.target.clone(),
                value: l.value,
            });
        }
    }
    Ok(())
}

fn compute_layout(
    nodes: &[ChordNode],
    links: &[ChordLink],
    options: &ChordOptions,
    plot: Rect,
) -> Result<ChordLayout, ChordError> {
    let n = nodes.len();
    let id_to_index: AHashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    // Sum of link values touching each node (both endpoints).
    let mut totals = vec![0.0_f32; n];
    for l in links {
        let s = id_to_index[l.source.as_str()];
        let t = id_to_index[l.target.as_str()];
        totals[s] += l.value;
        if s != t {
            totals[t] += l.value;
        }
    }
    let grand_total: f32 = totals.iter().sum();
    if grand_total <= 0.0 {
        return Err(ChordError::Empty);
    }

    let cx = plot.x + plot.w * 0.5;
    let cy = plot.y + plot.h * 0.5;
    let outer_radius = (plot.w.min(plot.h) * 0.5 - options.padding).max(40.0);
    // Keep a strictly positive band even for absurd thickness so the arc never
    // inverts (inner > outer) or collapses to the centre.
    let thickness = options.arc_thickness.max(1.0).min(outer_radius - 1.0);
    let inner_radius = outer_radius - thickness;

    // Allocate arcs. Cap the combined inter-arc gap at half the circle so dense
    // group counts (large `n` * `arc_gap`) can never drive `usable_angle`
    // negative and invert every arc.
    let n_f = n as f32;
    let total_gap = (options.arc_gap.max(0.0) * n_f).min(TAU * 0.5);
    let per_gap = if n > 0 { total_gap / n_f } else { 0.0 };
    let usable_angle = (TAU - total_gap).max(0.0);
    let mut arcs = Vec::with_capacity(n);
    let mut cursor = 0.0_f32;
    for (i, node) in nodes.iter().enumerate() {
        let span = if grand_total > 0.0 {
            usable_angle * (totals[i] / grand_total)
        } else {
            0.0
        };
        arcs.push(ChordArc {
            node_id: node.id.clone(),
            label: node.label.clone(),
            color: node.color,
            start_angle: cursor,
            end_angle: cursor + span,
            total_value: totals[i],
        });
        cursor += span + per_gap;
    }

    // Within each arc, sub-spans for each link end. Order: by link input order
    // for deterministic placement.
    let mut consumed = vec![0.0_f32; n]; // angular share already allocated within each arc
    let mut ribbons = Vec::with_capacity(links.len());
    for l in links {
        let s = id_to_index[l.source.as_str()];
        let t = id_to_index[l.target.as_str()];
        let arc_s_len = arcs[s].end_angle - arcs[s].start_angle;
        let arc_t_len = arcs[t].end_angle - arcs[t].start_angle;
        let s_share = if totals[s] > 0.0 {
            arc_s_len * (l.value / totals[s])
        } else {
            0.0
        };
        let t_share = if totals[t] > 0.0 {
            arc_t_len * (l.value / totals[t])
        } else {
            0.0
        };
        let s_start = arcs[s].start_angle + consumed[s];
        let s_end = s_start + s_share;
        let (t_start, t_end) = if s == t {
            consumed[s] += s_share;
            (s_start, s_end)
        } else {
            consumed[s] += s_share;
            let t_start = arcs[t].start_angle + consumed[t];
            let t_end = t_start + t_share;
            consumed[t] += t_share;
            (t_start, t_end)
        };

        let color = l.color.unwrap_or(arcs[s].color);
        ribbons.push(ChordRibbon {
            source_start: s_start,
            source_end: s_end,
            target_start: t_start,
            target_end: t_end,
            color,
            value: l.value,
        });
    }

    Ok(ChordLayout {
        arcs,
        ribbons,
        center: [cx, cy],
        inner_radius,
        outer_radius,
    })
}

fn point_at(center: [f32; 2], radius: f32, angle: f32) -> [f32; 2] {
    [
        center[0] + radius * angle.cos(),
        center[1] + radius * angle.sin(),
    ]
}

/// Evaluate a quadratic Bézier at `t` with endpoints `p0`/`p1` and control
/// point `ctrl` — used to trace a ribbon centerline for hit-testing.
fn quad_point(p0: [f32; 2], ctrl: [f32; 2], p1: [f32; 2], t: f32) -> [f32; 2] {
    let mt = 1.0 - t;
    let a = mt * mt;
    let b = 2.0 * mt * t;
    let c = t * t;
    [
        a * p0[0] + b * ctrl[0] + c * p1[0],
        a * p0[1] + b * ctrl[1] + c * p1[1],
    ]
}

/// Categorical node palette for [`ChordSpec::from_matrix`]. Colors are opaque,
/// so premultiplied and straight RGBA coincide.
fn node_palette(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 4]; 8] = [
        [0.20, 0.48, 0.83, 1.0],
        [0.24, 0.67, 0.72, 1.0],
        [0.36, 0.70, 0.52, 1.0],
        [0.53, 0.66, 0.40, 1.0],
        [0.80, 0.58, 0.30, 1.0],
        [0.66, 0.48, 0.78, 1.0],
        [0.85, 0.45, 0.38, 1.0],
        [0.30, 0.38, 0.52, 1.0],
    ];
    COLORS[index % COLORS.len()]
}

/// Find the ribbon whose centerline is nearest `point`, returning its row and
/// the distance in pixels.
///
/// Each ribbon's centerline is the quadratic curve from the source-arc midpoint,
/// through the disk centre, to the target-arc midpoint. Sampling that curve —
/// rather than testing a single midpoint — makes the entire ribbon pickable and
/// lets overlapping ribbons disambiguate by true proximity. A hit requires the
/// point to sit inside the inner disk and within the ribbon's half-width (its
/// angular thickness at the inner ring, floored so hairline ribbons stay
/// reachable).
fn nearest_ribbon(layout: &ChordLayout, point: (f32, f32)) -> Option<(usize, f32)> {
    let (px, py) = point;
    let center = layout.center;
    let dx = px - center[0];
    let dy = py - center[1];
    if (dx * dx + dy * dy).sqrt() > layout.inner_radius {
        return None;
    }
    let inner = layout.inner_radius;
    const STEPS: usize = 16;
    let mut best: Option<(usize, f32)> = None;
    for (row, r) in layout.ribbons.iter().enumerate() {
        let mid_s = (r.source_start + r.source_end) * 0.5;
        let mid_t = (r.target_start + r.target_end) * 0.5;
        let p_s = point_at(center, inner, mid_s);
        let p_t = point_at(center, inner, mid_t);

        let mut min_d = f32::INFINITY;
        for i in 0..=STEPS {
            let t = i as f32 / STEPS as f32;
            let p = quad_point(p_s, center, p_t, t);
            let d = ((p[0] - px).powi(2) + (p[1] - py).powi(2)).sqrt();
            if d < min_d {
                min_d = d;
            }
        }

        // Ribbon width at the inner ring = arc length of its widest end.
        let width = inner
            * (r.source_end - r.source_start)
                .abs()
                .max((r.target_end - r.target_start).abs());
        let threshold = (width * 0.5).max(6.0);
        if min_d <= threshold && best.is_none_or(|(_, bd)| min_d < bd) {
            best = Some((row, min_d));
        }
    }
    best
}

fn arc_to_path(
    center: [f32; 2],
    inner: f32,
    outer: f32,
    start: f32,
    end: f32,
    color: [f32; 4],
) -> PathPrim {
    let steps = ((end - start).abs() / 0.05).ceil().max(2.0) as usize;
    let mut commands = Vec::with_capacity(steps * 2 + 2);
    let outer_start = point_at(center, outer, start);
    commands.push(PathCommand::MoveTo {
        x: outer_start[0],
        y: outer_start[1],
    });
    // outer arc
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = start + (end - start) * t;
        let p = point_at(center, outer, a);
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    // line down to inner end
    let inner_end = point_at(center, inner, end);
    commands.push(PathCommand::LineTo {
        x: inner_end[0],
        y: inner_end[1],
    });
    // inner arc reverse
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = end + (start - end) * t;
        let p = point_at(center, inner, a);
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    commands.push(PathCommand::Close);
    PathPrim {
        commands,
        fill: color,
        stroke: [1.0, 1.0, 1.0, 0.7],
        stroke_width: 0.6,
    }
}

fn ribbon_to_path(center: [f32; 2], inner: f32, ribbon: &ChordRibbon, opacity: f32) -> PathPrim {
    // Bezier band from source arc to target arc through the centre.
    // Path: M src_start -> arc to src_end -> Q center -> target_end -> arc back
    // -> Q center -> src_start. The arc-sample loops recompute src_end/tgt_start
    // implicitly, so only the two anchors we `MoveTo`/`QuadTo` are bound here.
    let src_start = point_at(center, inner, ribbon.source_start);
    let tgt_end = point_at(center, inner, ribbon.target_end);

    let mut color = ribbon.color;
    color[3] *= opacity;

    let mut commands = Vec::new();
    commands.push(PathCommand::MoveTo {
        x: src_start[0],
        y: src_start[1],
    });
    // sample arc src_start -> src_end along the inner ring
    let arc_steps = 6_usize;
    for i in 1..=arc_steps {
        let t = i as f32 / arc_steps as f32;
        let a = ribbon.source_start + (ribbon.source_end - ribbon.source_start) * t;
        let p = point_at(center, inner, a);
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    // quadratic to target_end through centre
    commands.push(PathCommand::QuadTo {
        cx: center[0],
        cy: center[1],
        x: tgt_end[0],
        y: tgt_end[1],
    });
    // sample arc target_end -> target_start
    for i in 1..=arc_steps {
        let t = i as f32 / arc_steps as f32;
        let a = ribbon.target_end + (ribbon.target_start - ribbon.target_end) * t;
        let p = point_at(center, inner, a);
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    // quadratic back to src_start
    commands.push(PathCommand::QuadTo {
        cx: center[0],
        cy: center[1],
        x: src_start[0],
        y: src_start[1],
    });
    commands.push(PathCommand::Close);

    PathPrim {
        commands,
        fill: color,
        stroke: [0.0, 0.0, 0.0, 0.0],
        stroke_width: 0.0,
    }
}

fn build_labels(layout: &ChordLayout, offset: f32) -> Vec<LabelItem> {
    let radius = layout.outer_radius + offset;
    // Only arcs with a visible span are labelled. Rank the rest by size so the
    // overlay's collision resolver keeps the analytically dominant groups when
    // labels cannot all fit at dense group counts: above-average arcs stay
    // `Important`, thinner ones degrade to `Optional`.
    let visible: Vec<&ChordArc> = layout
        .arcs
        .iter()
        .filter(|a| a.end_angle > a.start_angle + 0.001)
        .collect();
    if visible.is_empty() {
        return Vec::new();
    }
    let avg = visible.iter().map(|a| a.total_value).sum::<f32>() / visible.len() as f32;
    visible
        .into_iter()
        .map(|a| {
            let mid = (a.start_angle + a.end_angle) * 0.5;
            let p = point_at(layout.center, radius, mid);
            let anchor = if p[0] >= layout.center[0] {
                LabelAnchor::Right
            } else {
                LabelAnchor::Left
            };
            let priority = if a.total_value >= avg {
                LabelPriority::Important
            } else {
                LabelPriority::Optional
            };
            LabelItem::new(p[0], p[1], a.label.clone())
                .with_anchor(anchor)
                .with_kind(LabelKind::Node)
                .with_priority(priority)
        })
        .collect()
}

fn node_dataset(layout: &ChordLayout) -> Dataset {
    let grand_total: f32 = layout.arcs.iter().map(|a| a.total_value).sum();
    let mut id_col: Vec<Arc<str>> = Vec::with_capacity(layout.arcs.len());
    let mut label_col: Vec<Arc<str>> = Vec::with_capacity(layout.arcs.len());
    let mut total: Vec<f32> = Vec::with_capacity(layout.arcs.len());
    let mut share: Vec<f32> = Vec::with_capacity(layout.arcs.len());
    for a in &layout.arcs {
        id_col.push(Arc::from(a.node_id.as_str()));
        label_col.push(Arc::from(a.label.as_str()));
        total.push(a.total_value);
        share.push(if grand_total > 0.0 {
            a.total_value / grand_total * 100.0
        } else {
            0.0
        });
    }
    Dataset::new(
        NODE_DATASET,
        1,
        vec![
            ("id".to_string(), Column::Utf8(ColumnData::new(id_col))),
            (
                "label".to_string(),
                Column::Utf8(ColumnData::new(label_col)),
            ),
            ("total".to_string(), Column::F32(ColumnData::new(total))),
            ("share".to_string(), Column::F32(ColumnData::new(share))),
        ],
    )
}

fn link_dataset(layout: &ChordLayout, input_links: &[ChordLink]) -> Dataset {
    debug_assert_eq!(layout.ribbons.len(), input_links.len());
    let total_value: f32 = layout.ribbons.iter().map(|r| r.value).sum();
    let source_totals: AHashMap<&str, f32> = layout
        .arcs
        .iter()
        .map(|a| (a.node_id.as_str(), a.total_value))
        .collect();
    let mut link_col: Vec<Arc<str>> = Vec::with_capacity(layout.ribbons.len());
    let mut source_col: Vec<Arc<str>> = Vec::with_capacity(layout.ribbons.len());
    let mut target_col: Vec<Arc<str>> = Vec::with_capacity(layout.ribbons.len());
    let mut values: Vec<f32> = Vec::with_capacity(layout.ribbons.len());
    let mut share: Vec<f32> = Vec::with_capacity(layout.ribbons.len());
    let mut source_share: Vec<f32> = Vec::with_capacity(layout.ribbons.len());
    for (r, input) in layout.ribbons.iter().zip(input_links) {
        link_col.push(Arc::from(format!("{} to {}", input.source, input.target)));
        source_col.push(Arc::from(input.source.as_str()));
        target_col.push(Arc::from(input.target.as_str()));
        values.push(r.value);
        share.push(if total_value > 0.0 {
            r.value / total_value * 100.0
        } else {
            0.0
        });
        let src_total = source_totals
            .get(input.source.as_str())
            .copied()
            .unwrap_or(0.0);
        source_share.push(if src_total > 0.0 {
            r.value / src_total * 100.0
        } else {
            0.0
        });
    }
    Dataset::new(
        LINK_DATASET,
        1,
        vec![
            ("link".to_string(), Column::Utf8(ColumnData::new(link_col))),
            (
                "source".to_string(),
                Column::Utf8(ColumnData::new(source_col)),
            ),
            (
                "target".to_string(),
                Column::Utf8(ColumnData::new(target_col)),
            ),
            ("value".to_string(), Column::F32(ColumnData::new(values))),
            ("share".to_string(), Column::F32(ColumnData::new(share))),
            (
                "source_share".to_string(),
                Column::F32(ColumnData::new(source_share)),
            ),
        ],
    )
}

fn snap_targets(layout: &ChordLayout, input_links: &[ChordLink]) -> Vec<SnapTarget> {
    let mut targets = Vec::with_capacity(layout.arcs.len() + layout.ribbons.len());
    let arc_radius = (layout.inner_radius + layout.outer_radius) * 0.5;
    targets.extend(layout.arcs.iter().map(|arc| {
        let mid = (arc.start_angle + arc.end_angle) * 0.5;
        let p = point_at(layout.center, arc_radius, mid);
        SnapTarget::new(p[0], p[1], SnapKind::Node)
            .with_radius(
                (layout.outer_radius - layout.inner_radius)
                    .mul_add(0.2, 5.0)
                    .clamp(5.0, 12.0),
            )
            .with_label(format!("{} arc", arc.label))
            .with_priority(3)
    }));
    targets.extend(
        layout
            .ribbons
            .iter()
            .zip(input_links)
            .map(|(ribbon, input)| {
                let source_mid = (ribbon.source_start + ribbon.source_end) * 0.5;
                let target_mid = (ribbon.target_start + ribbon.target_end) * 0.5;
                let source = point_at(layout.center, layout.inner_radius, source_mid);
                let target = point_at(layout.center, layout.inner_radius, target_mid);
                SnapTarget::new(
                    (source[0] + target[0]) * 0.5,
                    (source[1] + target[1]) * 0.5,
                    SnapKind::Edge,
                )
                .with_radius(
                    ribbon
                        .value
                        .max(0.0)
                        .sqrt()
                        .mul_add(0.5, 5.0)
                        .clamp(5.0, 12.0),
                )
                .with_label(format!("{} to {}", input.source, input.target))
                .with_priority(1)
            }),
    );
    targets
}

#[derive(Debug, Clone)]
struct ChordArcMark {
    id: MarkId,
    layout: ChordLayout,
}

impl ChordArcMark {
    fn new(id: MarkId, layout: ChordLayout) -> Self {
        Self { id, layout }
    }
}

impl Mark for ChordArcMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.arcs.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut paths = Vec::with_capacity(self.layout.arcs.len());
        for a in &self.layout.arcs {
            if a.end_angle - a.start_angle < 0.0005 {
                continue;
            }
            paths.push(arc_to_path(
                self.layout.center,
                self.layout.inner_radius,
                self.layout.outer_radius,
                a.start_angle,
                a.end_angle,
                a.color,
            ));
        }
        Geometry::Paths(paths)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        let dx = px - self.layout.center[0];
        let dy = py - self.layout.center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < self.layout.inner_radius || dist > self.layout.outer_radius {
            return None;
        }
        let angle = normalize_angle(dy.atan2(dx));
        for (row, a) in self.layout.arcs.iter().enumerate() {
            let start = normalize_angle(a.start_angle);
            let end = normalize_angle(a.end_angle);
            if angle_in_arc(angle, start, end) {
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
        let c = self.layout.center;
        let r = self.layout.outer_radius;
        Rect::new(c[0] - r, c[1] - r, 2.0 * r, 2.0 * r)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
struct ChordRibbonMark {
    id: MarkId,
    layout: ChordLayout,
    opacity: f32,
}

impl ChordRibbonMark {
    fn new(id: MarkId, layout: ChordLayout, opacity: f32) -> Self {
        Self {
            id,
            layout,
            opacity,
        }
    }
}

impl Mark for ChordRibbonMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.layout.ribbons.len() as u64;
        h ^= self.opacity.to_bits() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut paths = Vec::with_capacity(self.layout.ribbons.len());
        for r in &self.layout.ribbons {
            paths.push(ribbon_to_path(
                self.layout.center,
                self.layout.inner_radius,
                r,
                self.opacity,
            ));
        }
        Geometry::Paths(paths)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        nearest_ribbon(&self.layout, point).map(|(row, distance)| PickHit {
            mark: self.id,
            row: Some(row),
            distance,
            payload: None,
        })
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        let c = self.layout.center;
        let r = self.layout.inner_radius;
        Rect::new(c[0] - r, c[1] - r, 2.0 * r, 2.0 * r)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn normalize_angle(a: f32) -> f32 {
    let mut a = a % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a
}

fn angle_in_arc(angle: f32, start: f32, end: f32) -> bool {
    if start <= end {
        angle >= start && angle <= end
    } else {
        angle >= start || angle <= end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_unknown_node() {
        let nodes = vec![ChordNode::new("a", "A")];
        let links = vec![ChordLink::new("a", "b", 1.0)];
        assert!(matches!(
            validate(&nodes, &links),
            Err(ChordError::UnknownNode(_))
        ));
    }

    #[test]
    fn empty_nodes_rejected() {
        assert!(matches!(validate(&[], &[]), Err(ChordError::Empty)));
    }

    #[test]
    fn compute_layout_allocates_arcs_proportional_to_value() {
        let nodes = vec![
            ChordNode::new("a", "A"),
            ChordNode::new("b", "B"),
            ChordNode::new("c", "C"),
        ];
        let links = vec![
            ChordLink::new("a", "b", 1.0),
            ChordLink::new("a", "c", 1.0),
            ChordLink::new("b", "c", 1.0),
        ];
        let layout = compute_layout(
            &nodes,
            &links,
            &ChordOptions::default(),
            Rect::new(0.0, 0.0, 600.0, 600.0),
        )
        .expect("layout");
        // a has 2 links, b has 2 links, c has 2 links -> equal spans
        let span_a = layout.arcs[0].end_angle - layout.arcs[0].start_angle;
        let span_b = layout.arcs[1].end_angle - layout.arcs[1].start_angle;
        let span_c = layout.arcs[2].end_angle - layout.arcs[2].start_angle;
        assert!((span_a - span_b).abs() < 1e-4);
        assert!((span_b - span_c).abs() < 1e-4);
        assert_eq!(layout.ribbons.len(), 3);
    }

    #[test]
    fn empty_links_rejected() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let result = compute_layout(
            &nodes,
            &[],
            &ChordOptions::default(),
            Rect::new(0.0, 0.0, 600.0, 600.0),
        );
        assert!(matches!(result, Err(ChordError::Empty)));
    }

    #[test]
    fn build_chart_succeeds() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let links = vec![ChordLink::new("a", "b", 1.0)];
        let workspace = berthacharts_core::Workspace::new();
        let chart = ChordSpec::new(nodes, links)
            .build_chart(workspace, ChartSize::new(600, 600))
            .expect("chart builds");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn angle_in_arc_wraps_around_zero() {
        // Arc from 5.5 -> 0.5 (crosses 0)
        assert!(angle_in_arc(6.0, 5.5, 0.5));
        assert!(angle_in_arc(0.3, 5.5, 0.5));
        assert!(!angle_in_arc(3.0, 5.5, 0.5));
    }

    fn square(w: f32) -> Rect {
        Rect::new(0.0, 0.0, w, w)
    }

    #[test]
    fn rejects_non_finite_link_value() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let links = vec![ChordLink::new("a", "b", bad)];
            assert!(
                matches!(
                    validate(&nodes, &links),
                    Err(ChordError::InvalidValue { .. })
                ),
                "value {bad} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_negative_link_value() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let links = vec![ChordLink::new("a", "b", -1.0)];
        assert!(matches!(
            validate(&nodes, &links),
            Err(ChordError::InvalidValue { value, .. }) if value == -1.0
        ));
    }

    #[test]
    fn zero_value_links_are_allowed_but_all_zero_lays_out_nothing() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        // A zero link alongside a positive one validates and lays out.
        let mixed = vec![ChordLink::new("a", "b", 0.0), ChordLink::new("a", "b", 2.0)];
        assert!(validate(&nodes, &mixed).is_ok());
        let layout = compute_layout(&nodes, &mixed, &ChordOptions::default(), square(600.0))
            .expect("mixed layout");
        assert_eq!(layout.ribbons.len(), 2);
        // All-zero links validate (zero is an absent relationship) but there is
        // nothing to lay out.
        let zero_only = vec![ChordLink::new("a", "b", 0.0)];
        assert!(validate(&nodes, &zero_only).is_ok());
        assert!(matches!(
            compute_layout(&nodes, &zero_only, &ChordOptions::default(), square(600.0)),
            Err(ChordError::Empty)
        ));
    }

    #[test]
    fn dense_group_count_keeps_arcs_finite_and_within_circle() {
        let n = 300usize;
        let nodes: Vec<ChordNode> = (0..n)
            .map(|i| ChordNode::new(i.to_string(), i.to_string()))
            .collect();
        // Ring graph: every node gains a positive total from two incident links.
        let links: Vec<ChordLink> = (0..n)
            .map(|i| ChordLink::new(i.to_string(), ((i + 1) % n).to_string(), 1.0))
            .collect();
        let layout = compute_layout(&nodes, &links, &ChordOptions::default(), square(600.0))
            .expect("dense layout");
        for a in &layout.arcs {
            assert!(a.start_angle.is_finite() && a.end_angle.is_finite());
            assert!(
                a.end_angle >= a.start_angle,
                "arc must not invert at high node counts"
            );
        }
        let last_end = layout.arcs.last().expect("arcs").end_angle;
        assert!(
            last_end <= std::f32::consts::TAU + 1e-3,
            "arcs overflow the circle: {last_end}"
        );
    }

    #[test]
    fn from_matrix_emits_link_per_positive_cell_including_self_links() {
        let labels = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let matrix = vec![
            vec![0.0, 2.0, 0.0],
            vec![1.0, 0.0, 3.0],
            vec![0.0, 0.0, 5.0], // diagonal -> self-link on C
        ];
        let spec = ChordSpec::from_matrix(labels, &matrix);
        assert_eq!(spec.nodes.len(), 3);
        assert_eq!(spec.nodes[0].id, "0");
        assert_eq!(spec.nodes[0].label, "A");
        // Positive cells: (0,1), (1,0), (1,2), (2,2) -> 4 links.
        assert_eq!(spec.links.len(), 4);
        assert!(spec
            .links
            .iter()
            .any(|l| l.source == "2" && l.target == "2" && l.value == 5.0));
    }

    #[test]
    fn from_matrix_skips_invalid_cells_and_tolerates_ragged_rows() {
        let labels = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        // Ragged + negative + NaN cells must never panic or produce links.
        let matrix = vec![vec![0.0, -1.0], vec![f32::NAN, 4.0]];
        let spec = ChordSpec::from_matrix(labels, &matrix);
        assert_eq!(spec.nodes.len(), 3);
        // Only (1,1) = 4.0 survives.
        assert_eq!(spec.links.len(), 1);
        assert_eq!(spec.links[0].value, 4.0);
        let chart = spec
            .try_build_chart(Workspace::new(), ChartSize::new(400, 400))
            .expect("ragged matrix still builds");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn label_priority_tracks_arc_value() {
        let nodes = vec![
            ChordNode::new("hub", "Hub"),
            ChordNode::new("a", "A"),
            ChordNode::new("b", "B"),
            ChordNode::new("c", "C"),
        ];
        let links = vec![
            ChordLink::new("hub", "a", 100.0),
            ChordLink::new("hub", "b", 100.0),
            ChordLink::new("hub", "c", 100.0),
        ];
        let layout = compute_layout(&nodes, &links, &ChordOptions::default(), square(600.0))
            .expect("layout");
        let labels = build_labels(&layout, 10.0);
        // hub total 300 vs spokes 100 each; average 150 -> hub Important, spokes
        // Optional so the overlay drops spoke labels first at dense counts.
        let hub = labels.iter().find(|l| l.text == "Hub").expect("hub label");
        assert_eq!(hub.priority, LabelPriority::Important);
        let spoke = labels.iter().find(|l| l.text == "A").expect("spoke label");
        assert_eq!(spoke.priority, LabelPriority::Optional);
    }

    #[test]
    fn nearest_ribbon_picks_partway_along_centerline() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let links = vec![ChordLink::new("a", "b", 10.0)];
        let layout = compute_layout(&nodes, &links, &ChordOptions::default(), square(600.0))
            .expect("layout");
        let r = &layout.ribbons[0];
        let mid_s = (r.source_start + r.source_end) * 0.5;
        let mid_t = (r.target_start + r.target_end) * 0.5;
        let p_s = point_at(layout.center, layout.inner_radius, mid_s);
        let p_t = point_at(layout.center, layout.inner_radius, mid_t);
        // A point near the source end of the centerline is far from the ribbon
        // midpoint, yet must still pick the ribbon. Using an exact sample `t`
        // makes the centerline distance zero.
        let probe = quad_point(p_s, layout.center, p_t, 2.0 / 16.0);
        // It is well away from the disk centre (the old midpoint-only test zone).
        let from_center =
            ((probe[0] - layout.center[0]).powi(2) + (probe[1] - layout.center[1]).powi(2)).sqrt();
        assert!(from_center > 50.0, "probe should be far from centre");
        let (row, distance) =
            nearest_ribbon(&layout, (probe[0], probe[1])).expect("centerline point hits ribbon");
        assert_eq!(row, 0);
        assert!(distance < 1e-2, "distance on the centerline should be ~0");
    }

    #[test]
    fn nearest_ribbon_misses_outside_inner_disk() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let links = vec![ChordLink::new("a", "b", 10.0)];
        let layout = compute_layout(&nodes, &links, &ChordOptions::default(), square(600.0))
            .expect("layout");
        let outside = (
            layout.center[0] + layout.outer_radius + 50.0,
            layout.center[1],
        );
        assert!(nearest_ribbon(&layout, outside).is_none());
    }

    #[test]
    fn options_builders_set_fields() {
        let opts = ChordOptions::default()
            .with_padding(12.0)
            .with_arc_thickness(9.0)
            .with_arc_gap(0.1)
            .with_ribbon_opacity(0.3)
            .with_labels(false)
            .with_label_offset(4.0);
        assert_eq!(opts.padding, 12.0);
        assert_eq!(opts.arc_thickness, 9.0);
        assert_eq!(opts.arc_gap, 0.1);
        assert_eq!(opts.ribbon_opacity, 0.3);
        assert!(!opts.show_labels);
        assert_eq!(opts.label_offset, 4.0);
    }

    #[test]
    fn build_chart_exposes_share_columns() {
        let nodes = vec![
            ChordNode::new("a", "A"),
            ChordNode::new("b", "B"),
            ChordNode::new("c", "C"),
        ];
        let links = vec![
            ChordLink::new("a", "b", 1.0),
            ChordLink::new("b", "c", 1.0),
            ChordLink::new("a", "c", 2.0),
        ];
        let workspace = Workspace::new();
        ChordSpec::new(nodes, links)
            .try_build_chart(workspace.clone(), ChartSize::new(600, 600))
            .expect("chart");

        let node_ds = workspace.dataset(NODE_DATASET).expect("node dataset");
        let Column::F32(share) = node_ds.column("share").expect("node share").as_ref() else {
            panic!("node share should be f32");
        };
        let sum: f32 = share.values.iter().sum();
        assert!(
            (sum - 100.0).abs() < 1e-3,
            "node shares sum to 100, got {sum}"
        );

        let link_ds = workspace.dataset(LINK_DATASET).expect("link dataset");
        assert!(link_ds.column("share").is_some());
        assert!(link_ds.column("source_share").is_some());
    }

    #[test]
    fn layout_helper_surfaces_validation_errors() {
        let nodes = vec![ChordNode::new("a", "A"), ChordNode::new("b", "B")];
        let bad = ChordSpec::new(nodes.clone(), vec![ChordLink::new("a", "b", f32::NAN)]);
        assert!(matches!(
            bad.layout(ChartSize::new(600, 600)),
            Err(ChordError::InvalidValue { .. })
        ));
        let good = ChordSpec::new(nodes, vec![ChordLink::new("a", "b", 3.0)]);
        assert_eq!(
            good.layout(ChartSize::new(600, 600))
                .expect("layout")
                .arcs
                .len(),
            2
        );
    }
}
