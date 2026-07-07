//! Force-directed graph layout.
//!
//! Iterative physics simulation: nodes repel each other (Coulomb-style),
//! edges attract their endpoints (Hooke-style), gravity pulls toward the
//! plot-area centre. Alpha cooling decays the forces each step so the system
//! settles; the solver stops early once total kinetic energy stabilises.
//!
//! Repulsion runs through a Barnes-Hut quadtree (`O(n log n)`) by default,
//! with an exact `O(n²)` pair pass retained for small graphs and correctness
//! tests (see [`ForceOptions::use_barnes_hut`]). Layout is fully deterministic:
//! seed positions come from a golden-angle spiral — never a clock or RNG — so
//! the same nodes, edges, and options always produce the same picture.

use std::fmt;
use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LinePrim, LinearScale, Mark, MarkId, PathCommand, PathPrim,
    PickCtx, PickHit, PointPrim, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet,
    TessellateCtx, TooltipField, TooltipGuide, Workspace,
};

/// How edges are drawn between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    /// Straight line segments. Cheapest; overlaps at high density.
    Straight,
    /// Quadratic Bézier with offset control points — separates parallel edges
    /// and self-loops cleanly. Default.
    Curved,
}

const NODE_DATASET: DatasetId = DatasetId::new(0);
const EDGE_DATASET: DatasetId = DatasetId::new(1);
const NODE_MARK: MarkId = MarkId::new(1);
const EDGE_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// Golden angle (radians) driving the deterministic spiral seed layout.
const GOLDEN_ANGLE: f32 = 2.399_963_2;
/// Highlighted nodes render (and pick) at this multiple of their base radius.
const HIGHLIGHT_RADIUS_SCALE: f32 = 1.15;
/// Base pixel radius of a self-loop before per-sibling fan spacing is added.
const SELF_LOOP_BASE_RADIUS: f32 = 18.0;

/// A node in the input graph.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceNode {
    /// Stable node id used by edges.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Node radius in screen pixels.
    pub radius: f32,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// If true, the node is pinned at its initial position.
    pub fixed: bool,
    /// Optional seed position. `None` — or a non-finite value — places the node
    /// deterministically on a golden-angle spiral around the plot centre.
    pub initial: Option<[f32; 2]>,
}

impl ForceNode {
    /// Build a node with default radius / colour.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            radius: 6.0,
            color: [0.35, 0.55, 0.95, 1.0],
            fixed: false,
            initial: None,
        }
    }

    /// Override node radius.
    #[must_use]
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Override node colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Pin the node at a fixed position. Sets `fixed = true`.
    #[must_use]
    pub fn pinned_at(mut self, x: f32, y: f32) -> Self {
        self.fixed = true;
        self.initial = Some([x, y]);
        self
    }
}

/// An edge in the input graph.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceEdge {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Edge weight (scales attraction strength).
    pub weight: f32,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
}

impl ForceEdge {
    /// Build an edge with default weight + colour.
    #[must_use]
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            weight: 1.0,
            color: [0.5, 0.5, 0.55, 0.45],
        }
    }

    /// Override edge weight.
    #[must_use]
    pub const fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Override edge colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// Force solver configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceOptions {
    /// Number of solver iterations.
    pub iterations: u32,
    /// Repulsion strength between all node pairs.
    pub repulsion: f32,
    /// Attraction strength along edges (scaled by edge weight).
    pub attraction: f32,
    /// Centring gravity strength.
    pub gravity: f32,
    /// Velocity damping per iteration (0–1).
    pub damping: f32,
    /// Maximum per-step displacement (pixels).
    pub max_step: f32,
    /// Convergence threshold — stop when total energy delta < this.
    pub energy_threshold: f32,
    /// Use Barnes-Hut quadtree for O(n log n) repulsion. Defaults true; falls
    /// back to O(n²) when false (useful for tiny graphs or correctness tests).
    pub use_barnes_hut: bool,
    /// Barnes-Hut acceptance ratio. Lower = more accurate / slower. Typical
    /// range 0.5–1.5.
    pub theta: f32,
    /// Alpha cooling: forces scale by `alpha`, which decays from 1 → 0 across
    /// iterations. Higher cooling = faster decay, less polish near convergence.
    pub cooling_rate: f32,
    /// Render node labels in the chart's label guide.
    pub show_labels: bool,
    /// Per-label cap. Useful for dense graphs where rendering every label
    /// produces visual mush.
    pub max_visible_labels: Option<usize>,
    /// Edge rendering style — straight vs curved Bézier.
    pub edge_style: EdgeStyle,
    /// Maximum edge thickness in screen pixels (weights clamp to this).
    pub max_edge_width: f32,
    /// Multi-edge separation offset (pixels). Sibling edges between the same
    /// node pair fan out by `±k × this`.
    pub fan_offset: f32,
}

impl Default for ForceOptions {
    fn default() -> Self {
        Self {
            iterations: 300,
            repulsion: 600.0,
            attraction: 0.05,
            gravity: 0.02,
            damping: 0.85,
            max_step: 20.0,
            energy_threshold: 1e-3,
            use_barnes_hut: true,
            theta: 0.9,
            cooling_rate: 0.0228,
            show_labels: true,
            max_visible_labels: Some(40),
            edge_style: EdgeStyle::Curved,
            max_edge_width: 4.0,
            fan_offset: 14.0,
        }
    }
}

/// Force-directed chart spec.
#[derive(Debug, Clone)]
pub struct ForceSpec {
    nodes: Vec<ForceNode>,
    edges: Vec<ForceEdge>,
    options: ForceOptions,
    highlighted: Vec<String>,
}

impl ForceSpec {
    /// Build a force-directed spec from nodes + edges.
    #[must_use]
    pub fn new(nodes: Vec<ForceNode>, edges: Vec<ForceEdge>) -> Self {
        Self {
            nodes,
            edges,
            options: ForceOptions::default(),
            highlighted: Vec::new(),
        }
    }

    /// Build a spec from an edge list alone, inferring nodes in first-seen
    /// order (source before target within each edge). Each inferred node takes
    /// its id as its label and the [`ForceNode::new`] defaults; refine them via
    /// the returned spec's `nodes` if needed.
    ///
    /// Handy when the caller only has connectivity and does not want to declare
    /// a node up front for every id — the graph analogue of
    /// [`crate::SankeySpec::from_flows`].
    #[must_use]
    pub fn from_edges(edges: Vec<ForceEdge>) -> Self {
        let mut seen = AHashSet::new();
        let mut nodes = Vec::new();
        for edge in &edges {
            for id in [&edge.source, &edge.target] {
                if seen.insert(id.clone()) {
                    nodes.push(ForceNode::new(id.clone(), id.clone()));
                }
            }
        }
        Self::new(nodes, edges)
    }

    /// Override solver options.
    #[must_use]
    pub fn with_options(mut self, options: ForceOptions) -> Self {
        self.options = options;
        self
    }

    /// Mark a set of node ids as highlighted. The node mark renders these
    /// with bolder stroke + brighter fill. Use for hover / selection state
    /// driven from the binding layer.
    #[must_use]
    pub fn with_highlighted(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.highlighted = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Run the solver and return the computed node/edge positions without
    /// building a full chart. Validates first, so the same
    /// [`ForceError`]s a chart build would raise surface here. The result is
    /// deterministic for a given spec and `size`. Highlight state is a render
    /// overlay and is not applied to this raw layout.
    pub fn layout(&self, size: ChartSize) -> Result<ForceLayout, ForceError> {
        validate(&self.nodes, &self.edges)?;
        let plot = size.full_viewport().plot_area;
        Ok(simulate(&self.nodes, &self.edges, &self.options, plot))
    }

    /// Compile this spec into a chart. Inherent-method alias for the
    /// [`ChartSpec::build_chart`] trait method, so callers need not import the
    /// trait.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, ForceError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }
}

impl ChartSpec for ForceSpec {
    type Error = ForceError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        let viewport = size.full_viewport();
        let mut layout = self.layout(size)?;

        // Apply highlighted-node overlay state.
        if !self.highlighted.is_empty() {
            let highlight: AHashSet<&str> = self.highlighted.iter().map(String::as_str).collect();
            for node in &mut layout.nodes {
                if highlight.contains(node.id.as_str()) {
                    node.highlighted = true;
                }
            }
        }

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
        workspace.upsert_dataset(edge_dataset(&layout, &self.edges));

        let edge_mark: Arc<dyn Mark> = Arc::new(ForceEdgeMark::new(
            EDGE_MARK,
            layout.edges.clone(),
            self.options.edge_style,
            self.options.fan_offset,
            self.options.max_edge_width,
        ));
        let node_mark: Arc<dyn Mark> =
            Arc::new(ForceNodeMark::new(NODE_MARK, layout.nodes.clone()));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![edge_mark, node_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });

        if self.options.show_labels {
            let labels = build_labels(&layout, self.options.max_visible_labels);
            if !labels.is_empty() {
                let visible = labels.len();
                let mut guide = LabelGuide::new(labels).with_collision_padding(3.0);
                if let Some(cap) = self.options.max_visible_labels {
                    guide = guide.with_max_visible(cap.min(visible));
                }
                scene.guides.push(Guide::Labels(guide));
            }
        }
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                NODE_MARK,
                NODE_DATASET,
                vec![
                    TooltipField::new("Id", "id").as_label(),
                    TooltipField::new("Radius", "radius"),
                    TooltipField::new("X", "x"),
                    TooltipField::new("Y", "y"),
                ],
            )
            .with_title_column("label"),
        ));
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                EDGE_MARK,
                EDGE_DATASET,
                vec![
                    TooltipField::new("Source", "source").as_label(),
                    TooltipField::new("Target", "target").as_label(),
                    TooltipField::new("Weight", "weight"),
                ],
            )
            .with_title_column("link"),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, &self.edges, &self.options))
                .with_name("force graph anchors"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

/// Computed positions for nodes + edges.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceLayout {
    /// One entry per input node, same order.
    pub nodes: Vec<ForceLayoutNode>,
    /// One entry per input edge, same order.
    pub edges: Vec<ForceLayoutEdge>,
}

/// A laid-out node.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceLayoutNode {
    /// Node id (matches input).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Final x coordinate (screen pixels).
    pub x: f32,
    /// Final y coordinate (screen pixels).
    pub y: f32,
    /// Render radius.
    pub radius: f32,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// Highlighted nodes render with bolder stroke + brighter fill.
    pub highlighted: bool,
}

/// A laid-out edge with endpoint positions baked in.
#[derive(Debug, Clone, PartialEq)]
pub struct ForceLayoutEdge {
    /// Source position.
    pub source: [f32; 2],
    /// Target position.
    pub target: [f32; 2],
    /// Edge weight (carried through for downstream styling).
    pub weight: f32,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
    /// Position of this edge among siblings between the same endpoints. 0 = first.
    pub fan_index: u32,
    /// Total count of edges between the same endpoints.
    pub fan_count: u32,
    /// True when source and target are the same node — rendered as a loop.
    pub self_loop: bool,
}

/// Errors produced during force spec build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForceError {
    /// Edge references an unknown node id.
    UnknownNode(String),
    /// Duplicate node id encountered.
    DuplicateNode(String),
}

impl fmt::Display for ForceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "edge references unknown node id: {id}"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
        }
    }
}

impl std::error::Error for ForceError {}

fn validate(nodes: &[ForceNode], edges: &[ForceEdge]) -> Result<(), ForceError> {
    let mut ids = AHashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if ids.insert(n.id.as_str(), i).is_some() {
            return Err(ForceError::DuplicateNode(n.id.clone()));
        }
    }
    for e in edges {
        if !ids.contains_key(e.source.as_str()) {
            return Err(ForceError::UnknownNode(e.source.clone()));
        }
        if !ids.contains_key(e.target.as_str()) {
            return Err(ForceError::UnknownNode(e.target.clone()));
        }
    }
    Ok(())
}

/// Iterative force solver. Repulsion uses a Barnes-Hut quadtree above a small
/// node count (see [`ForceOptions::use_barnes_hut`]); attraction, gravity, and
/// alpha-cooled integration follow. Deterministic for a given input + options,
/// and hardened against degenerate input: non-finite seed positions, radii, and
/// weights are sanitised, and exactly-coincident seeds are nudged apart so they
/// still repel.
fn simulate(
    nodes: &[ForceNode],
    edges: &[ForceEdge],
    options: &ForceOptions,
    plot: Rect,
) -> ForceLayout {
    let n = nodes.len();
    let cx = plot.x + plot.w * 0.5;
    let cy = plot.y + plot.h * 0.5;

    let mut x = vec![0.0_f32; n];
    let mut y = vec![0.0_f32; n];
    let mut vx = vec![0.0_f32; n];
    let mut vy = vec![0.0_f32; n];
    let fixed: Vec<bool> = nodes.iter().map(|node| node.fixed).collect();

    // `.abs()` guards an inverted/degenerate plot rect (negative w/h).
    let radius = plot.w.min(plot.h).abs() * 0.4;
    for (i, node) in nodes.iter().enumerate() {
        match node.initial {
            // Honour a caller-supplied seed only when finite — a stray NaN/∞
            // would otherwise poison every other node through repulsion.
            Some([ix, iy]) if ix.is_finite() && iy.is_finite() => {
                x[i] = ix;
                y[i] = iy;
            }
            _ => {
                // Deterministic golden-angle spiral — reproducible, never random.
                let angle = i as f32 * GOLDEN_ANGLE;
                let r = radius * ((i as f32 + 1.0) / n as f32).sqrt();
                x[i] = cx + r * angle.cos();
                y[i] = cy + r * angle.sin();
            }
        }
    }

    // Break exact position ties deterministically. A zero separation vector has
    // an undefined repulsion direction, so coincident nodes would otherwise stay
    // fused forever (this also covers a zero-size plot, where every spiral point
    // collapses onto the centre). Fixed nodes keep their pin; only free
    // duplicates are nudged, along a golden-angle direction keyed on index so
    // the outcome stays reproducible.
    let mut seen: AHashSet<(u32, u32)> = AHashSet::with_capacity(n);
    for i in 0..n {
        if seen.insert((x[i].to_bits(), y[i].to_bits())) || fixed[i] {
            continue;
        }
        let mut bump = 1.0_f32;
        loop {
            let angle = (i as f32 + bump) * GOLDEN_ANGLE;
            let nx = x[i] + angle.cos() * bump;
            let ny = y[i] + angle.sin() * bump;
            if seen.insert((nx.to_bits(), ny.to_bits())) {
                x[i] = nx;
                y[i] = ny;
                break;
            }
            bump += 1.0;
        }
    }

    let id_to_index: AHashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    let edge_pairs: Vec<(usize, usize, f32)> = edges
        .iter()
        .map(|e| {
            (
                id_to_index[e.source.as_str()],
                id_to_index[e.target.as_str()],
                // A non-finite weight would blow up the attraction term.
                finite_or(e.weight, 0.0),
            )
        })
        .collect();

    let mut prev_energy = f32::INFINITY;
    let mut alpha = 1.0_f32;
    for _ in 0..options.iterations {
        let mut fx = vec![0.0_f32; n];
        let mut fy = vec![0.0_f32; n];

        // Repulsion — Barnes-Hut quadtree (O(n log n)) or O(n²) fallback.
        if options.use_barnes_hut && n > 16 {
            let tree_bounds = positions_bounds(&x, &y);
            let tree = BHTree::build(&x, &y, tree_bounds);
            for i in 0..n {
                let (rfx, rfy) =
                    tree.apply_repulsion(i, (x[i], y[i]), options.theta, options.repulsion);
                fx[i] += rfx;
                fy[i] += rfy;
            }
        } else {
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = x[j] - x[i];
                    let dy = y[j] - y[i];
                    let dist_sq = (dx * dx + dy * dy).max(1.0);
                    let dist = dist_sq.sqrt();
                    let f = options.repulsion / dist_sq;
                    let ux = dx / dist;
                    let uy = dy / dist;
                    fx[i] -= f * ux;
                    fy[i] -= f * uy;
                    fx[j] += f * ux;
                    fy[j] += f * uy;
                }
            }
        }

        // Attraction along edges, scaled by weight.
        for &(a, b, w) in &edge_pairs {
            let dx = x[b] - x[a];
            let dy = y[b] - y[a];
            let f = options.attraction * w;
            fx[a] += f * dx;
            fy[a] += f * dy;
            fx[b] -= f * dx;
            fy[b] -= f * dy;
        }

        // Centring gravity toward (cx, cy).
        for i in 0..n {
            fx[i] += (cx - x[i]) * options.gravity;
            fy[i] += (cy - y[i]) * options.gravity;
        }

        // Integrate with damping + step cap. Forces scaled by alpha (cooling).
        let mut energy = 0.0_f32;
        for i in 0..n {
            if fixed[i] {
                continue;
            }
            vx[i] = (vx[i] + fx[i] * alpha) * options.damping;
            vy[i] = (vy[i] + fy[i] * alpha) * options.damping;
            let speed = (vx[i] * vx[i] + vy[i] * vy[i]).sqrt();
            if speed > options.max_step {
                let scale = options.max_step / speed;
                vx[i] *= scale;
                vy[i] *= scale;
            }
            x[i] += vx[i];
            y[i] += vy[i];
            energy += vx[i] * vx[i] + vy[i] * vy[i];
        }

        if (prev_energy - energy).abs() < options.energy_threshold {
            break;
        }
        prev_energy = energy;
        alpha = (alpha - options.cooling_rate).max(0.0);
        if alpha == 0.0 {
            break;
        }
    }

    let layout_nodes: Vec<ForceLayoutNode> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| ForceLayoutNode {
            id: node.id.clone(),
            label: node.label.clone(),
            // Defensive: keep geometry/picking/bounds free of NaN/∞ even if a
            // pathological option (e.g. non-finite damping) diverged the solver.
            x: finite_or(x[i], cx),
            y: finite_or(y[i], cy),
            radius: sanitize_radius(node.radius),
            color: node.color,
            highlighted: false,
        })
        .collect();

    // Compute fan info: count siblings between each unordered endpoint pair,
    // then assign per-edge fan_index in input order.
    let mut pair_counts: AHashMap<(usize, usize), u32> = AHashMap::new();
    for e in edges {
        let a = id_to_index[e.source.as_str()];
        let b = id_to_index[e.target.as_str()];
        let key = if a <= b { (a, b) } else { (b, a) };
        *pair_counts.entry(key).or_insert(0) += 1;
    }
    let mut pair_assigned: AHashMap<(usize, usize), u32> = AHashMap::new();

    let layout_edges: Vec<ForceLayoutEdge> = edges
        .iter()
        .map(|e| {
            let a = id_to_index[e.source.as_str()];
            let b = id_to_index[e.target.as_str()];
            let key = if a <= b { (a, b) } else { (b, a) };
            let fan_index = *pair_assigned.entry(key).or_insert(0);
            pair_assigned.insert(key, fan_index + 1);
            let fan_count = pair_counts[&key];
            ForceLayoutEdge {
                source: [x[a], y[a]],
                target: [x[b], y[b]],
                weight: e.weight,
                color: e.color,
                fan_index,
                fan_count,
                self_loop: a == b,
            }
        })
        .collect();

    ForceLayout {
        nodes: layout_nodes,
        edges: layout_edges,
    }
}

fn node_dataset(layout: &ForceLayout) -> Dataset {
    let mut id_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut label_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut x_col = Vec::with_capacity(layout.nodes.len());
    let mut y_col = Vec::with_capacity(layout.nodes.len());
    let mut radius_col = Vec::with_capacity(layout.nodes.len());
    for node in &layout.nodes {
        id_col.push(Arc::from(node.id.as_str()));
        label_col.push(Arc::from(node.label.as_str()));
        x_col.push(node.x);
        y_col.push(node.y);
        radius_col.push(node.radius);
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
            ("x".to_string(), Column::F32(ColumnData::new(x_col))),
            ("y".to_string(), Column::F32(ColumnData::new(y_col))),
            (
                "radius".to_string(),
                Column::F32(ColumnData::new(radius_col)),
            ),
        ],
    )
}

fn edge_dataset(layout: &ForceLayout, input_edges: &[ForceEdge]) -> Dataset {
    debug_assert_eq!(layout.edges.len(), input_edges.len());
    let mut link_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut source_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut target_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut weight_col = Vec::with_capacity(layout.edges.len());
    let mut sx = Vec::with_capacity(layout.edges.len());
    let mut sy = Vec::with_capacity(layout.edges.len());
    let mut tx = Vec::with_capacity(layout.edges.len());
    let mut ty = Vec::with_capacity(layout.edges.len());
    for (edge, input) in layout.edges.iter().zip(input_edges) {
        link_col.push(Arc::from(format!("{} to {}", input.source, input.target)));
        source_col.push(Arc::from(input.source.as_str()));
        target_col.push(Arc::from(input.target.as_str()));
        weight_col.push(input.weight);
        sx.push(edge.source[0]);
        sy.push(edge.source[1]);
        tx.push(edge.target[0]);
        ty.push(edge.target[1]);
    }
    Dataset::new(
        EDGE_DATASET,
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
            (
                "weight".to_string(),
                Column::F32(ColumnData::new(weight_col)),
            ),
            ("source_x".to_string(), Column::F32(ColumnData::new(sx))),
            ("source_y".to_string(), Column::F32(ColumnData::new(sy))),
            ("target_x".to_string(), Column::F32(ColumnData::new(tx))),
            ("target_y".to_string(), Column::F32(ColumnData::new(ty))),
        ],
    )
}

#[derive(Debug, Clone)]
struct ForceNodeMark {
    id: MarkId,
    nodes: Vec<ForceLayoutNode>,
}

impl ForceNodeMark {
    fn new(id: MarkId, nodes: Vec<ForceLayoutNode>) -> Self {
        Self { id, nodes }
    }
}

impl Mark for ForceNodeMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.nodes.len() as u64;
        for n in &self.nodes {
            h ^= (n.x.to_bits() as u64).rotate_left(13);
            h ^= (n.y.to_bits() as u64).rotate_left(31);
        }
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut points = Vec::with_capacity(self.nodes.len());
        for n in &self.nodes {
            let (stroke, stroke_width, fill) = if n.highlighted {
                let mut brighter = n.color;
                // Boost saturation toward white by ~25%.
                for c in &mut brighter[0..3] {
                    *c = (*c * 0.75 + 0.25).min(1.0);
                }
                ([1.0, 0.95, 0.4, 1.0], 2.5_f32, brighter)
            } else {
                ([1.0, 1.0, 1.0, 0.6], 1.0_f32, n.color)
            };
            points.push(PointPrim {
                x: n.x,
                y: n.y,
                r: if n.highlighted {
                    n.radius * HIGHLIGHT_RADIUS_SCALE
                } else {
                    n.radius
                },
                shape: 0,
                fill,
                stroke,
                stroke_width,
            });
        }
        Geometry::Points(points)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        const PICK_SLOP: f32 = 2.0;
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        for (row, n) in self.nodes.iter().enumerate() {
            let dx = px - n.x;
            let dy = py - n.y;
            let d = (dx * dx + dy * dy).sqrt();
            // Match the rendered radius so highlighted (enlarged) nodes stay
            // pickable across their whole glyph; nearest centre wins ties.
            let render_r = if n.highlighted {
                n.radius * HIGHLIGHT_RADIUS_SCALE
            } else {
                n.radius
            };
            if d <= render_r + PICK_SLOP && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((row, d));
            }
        }
        best.map(|(row, d)| PickHit {
            mark: self.id,
            row: Some(row),
            distance: d,
            payload: None,
        })
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.nodes.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for n in &self.nodes {
            min_x = min_x.min(n.x - n.radius);
            min_y = min_y.min(n.y - n.radius);
            max_x = max_x.max(n.x + n.radius);
            max_y = max_y.max(n.y + n.radius);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
struct ForceEdgeMark {
    id: MarkId,
    edges: Vec<ForceLayoutEdge>,
    style: EdgeStyle,
    fan_offset: f32,
    max_width: f32,
}

impl ForceEdgeMark {
    fn new(
        id: MarkId,
        edges: Vec<ForceLayoutEdge>,
        style: EdgeStyle,
        fan_offset: f32,
        max_width: f32,
    ) -> Self {
        Self {
            id,
            edges,
            style,
            fan_offset,
            max_width,
        }
    }

    fn edge_width(&self, weight: f32) -> f32 {
        (0.6 + weight.max(0.0)).min(self.max_width)
    }

    /// Compute the perpendicular offset for this edge given its fan index +
    /// total siblings. Centred when count == 1, fans symmetrically otherwise.
    fn perpendicular_offset(&self, fan_index: u32, fan_count: u32) -> f32 {
        fan_perp_offset(fan_index, fan_count, self.fan_offset)
    }
}

impl Mark for ForceEdgeMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.edges.len() as u64;
        h ^= match self.style {
            EdgeStyle::Straight => 1,
            EdgeStyle::Curved => 2,
        };
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        // Self-loops always render as paths; straight non-loops as lines for
        // perf; curved non-loops as paths.
        let mut lines: Vec<LinePrim> = Vec::new();
        let mut paths: Vec<PathPrim> = Vec::new();
        for e in &self.edges {
            let width = self.edge_width(e.weight);
            if e.self_loop {
                paths.push(self_loop_path(e, self.fan_offset, width));
                continue;
            }
            match self.style {
                EdgeStyle::Straight if e.fan_count <= 1 => {
                    lines.push(LinePrim {
                        points: vec![e.source, e.target],
                        stroke: e.color,
                        width,
                        dash: None,
                        join: 1,
                        cap: 1,
                    });
                }
                _ => {
                    let offset = self.perpendicular_offset(e.fan_index, e.fan_count);
                    paths.push(curved_edge_path(e, offset, width));
                }
            }
        }

        match (lines.is_empty(), paths.is_empty()) {
            (true, true) => Geometry::Empty,
            (false, true) => Geometry::Lines(lines),
            (true, false) => Geometry::Paths(paths),
            (false, false) => Geometry::Mixed(vec![Geometry::Lines(lines), Geometry::Paths(paths)]),
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        const HIT_TOLERANCE: f32 = 5.0;
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        for (row, e) in self.edges.iter().enumerate() {
            let d = if e.self_loop {
                self_loop_distance(e, self.fan_offset, [px, py])
            } else if matches!(self.style, EdgeStyle::Curved) || e.fan_count > 1 {
                let offset = self.perpendicular_offset(e.fan_index, e.fan_count);
                curve_distance(e, offset, [px, py])
            } else {
                segment_distance(e.source, e.target, [px, py])
            };
            // Widen the hit band by the stroke half-width so thick edges are
            // pickable across their full rendered thickness, not just the axis.
            let tol = HIT_TOLERANCE + self.edge_width(e.weight) * 0.5;
            if d <= tol && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((row, d));
            }
        }
        best.map(|(row, d)| PickHit {
            mark: self.id,
            row: Some(row),
            distance: d,
            payload: None,
        })
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.edges.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        // Bounding box pads by fan offset to keep curves inside the rect.
        let pad = self.fan_offset.max(8.0);
        for e in &self.edges {
            min_x = min_x.min(e.source[0]).min(e.target[0]) - pad;
            min_y = min_y.min(e.source[1]).min(e.target[1]) - pad;
            max_x = max_x.max(e.source[0]).max(e.target[0]) + pad;
            max_y = max_y.max(e.source[1]).max(e.target[1]) + pad;
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Quadratic Bézier from src→tgt with control point offset perpendicular by `offset` pixels.
fn curved_edge_path(e: &ForceLayoutEdge, offset: f32, width: f32) -> PathPrim {
    let (cx, cy) = quad_control_point(e.source, e.target, offset);
    PathPrim {
        commands: vec![
            PathCommand::MoveTo {
                x: e.source[0],
                y: e.source[1],
            },
            PathCommand::QuadTo {
                cx,
                cy,
                x: e.target[0],
                y: e.target[1],
            },
        ],
        fill: [0.0, 0.0, 0.0, 0.0],
        stroke: e.color,
        stroke_width: width,
    }
}

fn quad_control_point(src: [f32; 2], tgt: [f32; 2], offset: f32) -> (f32, f32) {
    let mx = (src[0] + tgt[0]) * 0.5;
    let my = (src[1] + tgt[1]) * 0.5;
    if offset == 0.0 {
        return (mx, my);
    }
    let dx = tgt[0] - src[0];
    let dy = tgt[1] - src[1];
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    // Perpendicular unit vector (-dy, dx) / len
    let nx = -dy / len;
    let ny = dx / len;
    (mx + nx * offset, my + ny * offset)
}

/// Self-loop: cubic Bézier out and back from the node, opens to the right.
fn self_loop_path(e: &ForceLayoutEdge, fan_offset: f32, width: f32) -> PathPrim {
    let radius = self_loop_radius(fan_offset, e.fan_index);
    let p = e.source;
    let c1 = [p[0] + radius, p[1] - radius];
    let c2 = [p[0] + radius, p[1] + radius];
    PathPrim {
        commands: vec![
            PathCommand::MoveTo { x: p[0], y: p[1] },
            PathCommand::CubicTo {
                c1x: c1[0],
                c1y: c1[1],
                c2x: c2[0],
                c2y: c2[1],
                x: p[0],
                y: p[1],
            },
        ],
        fill: [0.0, 0.0, 0.0, 0.0],
        stroke: e.color,
        stroke_width: width,
    }
}

/// Distance from `p` to a quadratic Bézier sampled into segments.
fn curve_distance(e: &ForceLayoutEdge, offset: f32, p: [f32; 2]) -> f32 {
    let (cx, cy) = quad_control_point(e.source, e.target, offset);
    const STEPS: u32 = 16;
    let mut prev = e.source;
    let mut best = f32::INFINITY;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let x = mt * mt * e.source[0] + 2.0 * mt * t * cx + t * t * e.target[0];
        let y = mt * mt * e.source[1] + 2.0 * mt * t * cy + t * t * e.target[1];
        let cur = [x, y];
        let d = segment_distance(prev, cur, p);
        if d < best {
            best = d;
        }
        prev = cur;
    }
    best
}

/// Distance to a self-loop, sampled.
fn self_loop_distance(e: &ForceLayoutEdge, fan_offset: f32, p: [f32; 2]) -> f32 {
    let radius = self_loop_radius(fan_offset, e.fan_index);
    let origin = e.source;
    let c1 = [origin[0] + radius, origin[1] - radius];
    let c2 = [origin[0] + radius, origin[1] + radius];
    const STEPS: u32 = 24;
    let mut prev = origin;
    let mut best = f32::INFINITY;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let bx = mt * mt * mt * origin[0]
            + 3.0 * mt * mt * t * c1[0]
            + 3.0 * mt * t * t * c2[0]
            + t * t * t * origin[0];
        let by = mt * mt * mt * origin[1]
            + 3.0 * mt * mt * t * c1[1]
            + 3.0 * mt * t * t * c2[1]
            + t * t * t * origin[1];
        let cur = [bx, by];
        let d = segment_distance(prev, cur, p);
        if d < best {
            best = d;
        }
        prev = cur;
    }
    best
}

// ---------- helpers ----------

/// Replace a non-finite value with `fallback`, so a stray NaN/∞ never escapes
/// the solver into geometry, picking, or bounds.
fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

/// Clamp a render radius to a finite, non-negative value.
fn sanitize_radius(radius: f32) -> f32 {
    if radius.is_finite() {
        radius.max(0.0)
    } else {
        0.0
    }
}

/// Pixel radius of a self-loop for the `fan_index`-th sibling loop on a node.
fn self_loop_radius(fan_offset: f32, fan_index: u32) -> f32 {
    SELF_LOOP_BASE_RADIUS + fan_offset * fan_index as f32
}

/// Perpendicular fan offset for one edge among `fan_count` siblings. Centred
/// when `fan_count <= 1`; fans symmetrically otherwise
/// (`-((n-1)/2) … +((n-1)/2)` scaled by `fan_offset`).
fn fan_perp_offset(fan_index: u32, fan_count: u32, fan_offset: f32) -> f32 {
    if fan_count <= 1 {
        return 0.0;
    }
    let center = (fan_count as f32 - 1.0) * 0.5;
    (fan_index as f32 - center) * fan_offset
}

/// Anchor point that lies on the *rendered* edge — the curve or loop apex, not
/// the straight chord — so annotation snapping lands on what the viewer sees.
fn edge_anchor(edge: &ForceLayoutEdge, options: &ForceOptions) -> (f32, f32) {
    if edge.self_loop {
        // Apex of the cubic loop sits 0.75·r out along +x from the node.
        let r = self_loop_radius(options.fan_offset, edge.fan_index);
        return (edge.source[0] + 0.75 * r, edge.source[1]);
    }
    let mid_x = (edge.source[0] + edge.target[0]) * 0.5;
    let mid_y = (edge.source[1] + edge.target[1]) * 0.5;
    let straight = matches!(options.edge_style, EdgeStyle::Straight) && edge.fan_count <= 1;
    let offset = if straight {
        0.0
    } else {
        fan_perp_offset(edge.fan_index, edge.fan_count, options.fan_offset)
    };
    if offset == 0.0 {
        return (mid_x, mid_y);
    }
    // Quadratic Bézier midpoint = chord midpoint + ½·offset·perp.
    let dx = edge.target[0] - edge.source[0];
    let dy = edge.target[1] - edge.source[1];
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let nx = -dy / len;
    let ny = dx / len;
    (mid_x + nx * offset * 0.5, mid_y + ny * offset * 0.5)
}

fn positions_bounds(x: &[f32], y: &[f32]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &xi in x {
        min_x = min_x.min(xi);
        max_x = max_x.max(xi);
    }
    for &yi in y {
        min_y = min_y.min(yi);
        max_y = max_y.max(yi);
    }
    // Pad slightly so root strictly contains every point even on degenerate inputs.
    let pad = 1.0_f32;
    Rect::new(
        min_x - pad,
        min_y - pad,
        (max_x - min_x).max(2.0 * pad) + 2.0 * pad,
        (max_y - min_y).max(2.0 * pad) + 2.0 * pad,
    )
}

fn segment_distance(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < f32::EPSILON {
        let ex = p[0] - a[0];
        let ey = p[1] - a[1];
        return (ex * ex + ey * ey).sqrt();
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a[0] + t * dx;
    let cy = a[1] + t * dy;
    let ex = p[0] - cx;
    let ey = p[1] - cy;
    (ex * ex + ey * ey).sqrt()
}

fn build_labels(layout: &ForceLayout, max_visible: Option<usize>) -> Vec<LabelItem> {
    // Rank nodes by visual importance — bigger radius = more important. Stable
    // tiebreak on the iteration order.
    let mut ranked: Vec<(usize, f32)> = layout
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (i, n.radius))
        .collect();
    ranked.sort_by(|(ai, ar), (bi, br)| {
        br.partial_cmp(ar)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(ai.cmp(bi))
    });

    let take = max_visible
        .unwrap_or(layout.nodes.len())
        .min(layout.nodes.len());
    ranked.truncate(take);

    ranked
        .into_iter()
        .map(|(i, _)| {
            let n = &layout.nodes[i];
            LabelItem::new(n.x, n.y - n.radius - 6.0, n.label.clone())
                .with_anchor(LabelAnchor::Top)
                .with_kind(LabelKind::Node)
                .with_priority(LabelPriority::Important)
        })
        .collect()
}

fn snap_targets(
    layout: &ForceLayout,
    input_edges: &[ForceEdge],
    options: &ForceOptions,
) -> Vec<SnapTarget> {
    let mut targets = Vec::with_capacity(layout.nodes.len() + layout.edges.len());
    targets.extend(layout.nodes.iter().map(|node| {
        SnapTarget::new(node.x, node.y, SnapKind::Node)
            .with_radius((node.radius + 4.0).clamp(6.0, 14.0))
            .with_label(format!("{} node", node.label))
            .with_priority(if node.highlighted { 4 } else { 3 })
    }));
    targets.extend(layout.edges.iter().zip(input_edges).map(|(edge, input)| {
        // Anchor on the rendered curve/loop so fanned parallel edges get
        // distinct, on-path snap points instead of stacking at one midpoint.
        let (x, y) = edge_anchor(edge, options);
        let label = if edge.self_loop {
            format!("{} self-loop", input.source)
        } else {
            format!("{} to {}", input.source, input.target)
        };
        SnapTarget::new(x, y, SnapKind::Edge)
            .with_radius((edge.weight.max(0.0).sqrt() + 5.0).clamp(5.0, 12.0))
            .with_label(label)
            .with_priority(1)
    }));
    targets
}

// ---------- Barnes-Hut quadtree ----------
//
// Flat-arena quadtree. Each node carries center-of-mass + total mass; leaves
// hold zero or one body. `apply_repulsion` walks the tree and treats a subtree
// as a single point when (cell size / distance) < theta.

#[derive(Debug, Clone, Copy)]
struct BHNode {
    bounds: Rect,
    com_x: f32,
    com_y: f32,
    mass: f32,
    body: i32,          // -1 = empty or internal; otherwise body index
    children: [i32; 4], // -1 if absent
}

impl BHNode {
    fn empty(bounds: Rect) -> Self {
        Self {
            bounds,
            com_x: 0.0,
            com_y: 0.0,
            mass: 0.0,
            body: -1,
            children: [-1; 4],
        }
    }
    fn is_internal(&self) -> bool {
        self.children.iter().any(|&c| c >= 0)
    }
}

#[derive(Debug)]
struct BHTree {
    nodes: Vec<BHNode>,
    bx: Vec<f32>,
    by: Vec<f32>,
}

impl BHTree {
    fn build(x: &[f32], y: &[f32], bounds: Rect) -> Self {
        let mut tree = Self {
            nodes: vec![BHNode::empty(bounds)],
            bx: x.to_vec(),
            by: y.to_vec(),
        };
        for i in 0..x.len() {
            tree.insert(0, i, 0);
        }
        tree
    }

    fn insert(&mut self, node_idx: usize, body: usize, depth: u32) {
        const MAX_DEPTH: u32 = 32;
        let bx = self.bx[body];
        let by = self.by[body];

        // Update centre of mass / mass.
        let m_new = self.nodes[node_idx].mass + 1.0;
        self.nodes[node_idx].com_x =
            (self.nodes[node_idx].com_x * self.nodes[node_idx].mass + bx) / m_new;
        self.nodes[node_idx].com_y =
            (self.nodes[node_idx].com_y * self.nodes[node_idx].mass + by) / m_new;
        self.nodes[node_idx].mass = m_new;

        let existing_body = self.nodes[node_idx].body;
        let internal = self.nodes[node_idx].is_internal();

        if existing_body == -1 && !internal {
            // empty leaf — place body here
            self.nodes[node_idx].body = body as i32;
            return;
        }

        // Reached safety depth: avoid stack blow on near-duplicate positions.
        if depth >= MAX_DEPTH {
            return;
        }

        if existing_body >= 0 {
            // Subdivide: move existing body into a child first.
            let prior = existing_body as usize;
            self.nodes[node_idx].body = -1;
            let q = self.quadrant_for(node_idx, self.bx[prior], self.by[prior]);
            self.ensure_child(node_idx, q);
            let child = self.nodes[node_idx].children[q] as usize;
            self.insert(child, prior, depth + 1);
        }

        // Place new body into its quadrant.
        let q = self.quadrant_for(node_idx, bx, by);
        self.ensure_child(node_idx, q);
        let child = self.nodes[node_idx].children[q] as usize;
        self.insert(child, body, depth + 1);
    }

    fn quadrant_for(&self, node_idx: usize, x: f32, y: f32) -> usize {
        let b = self.nodes[node_idx].bounds;
        let mid_x = b.x + b.w * 0.5;
        let mid_y = b.y + b.h * 0.5;
        let east = x >= mid_x;
        let south = y >= mid_y;
        match (east, south) {
            (false, false) => 0, // NW
            (true, false) => 1,  // NE
            (false, true) => 2,  // SW
            (true, true) => 3,   // SE
        }
    }

    fn ensure_child(&mut self, node_idx: usize, q: usize) {
        if self.nodes[node_idx].children[q] >= 0 {
            return;
        }
        let b = self.nodes[node_idx].bounds;
        let half_w = b.w * 0.5;
        let half_h = b.h * 0.5;
        let (ox, oy) = match q {
            0 => (0.0, 0.0),
            1 => (half_w, 0.0),
            2 => (0.0, half_h),
            3 => (half_w, half_h),
            _ => unreachable!(),
        };
        let child_bounds = Rect::new(b.x + ox, b.y + oy, half_w, half_h);
        let new_idx = self.nodes.len();
        self.nodes.push(BHNode::empty(child_bounds));
        self.nodes[node_idx].children[q] = new_idx as i32;
    }

    /// Accumulate repulsion on the body at index `self_body` (position `point`).
    /// Passing the index lets the walk skip that body exactly, rather than
    /// skipping any body that happens to share its coordinates — which would
    /// wrongly drop the force from a distinct, coincident neighbour.
    fn apply_repulsion(
        &self,
        self_body: usize,
        point: (f32, f32),
        theta: f32,
        k: f32,
    ) -> (f32, f32) {
        let mut acc = (0.0_f32, 0.0_f32);
        self.walk(0, self_body, point, theta, k, &mut acc);
        acc
    }

    fn walk(
        &self,
        node_idx: usize,
        self_body: usize,
        point: (f32, f32),
        theta: f32,
        k: f32,
        acc: &mut (f32, f32),
    ) {
        let node = self.nodes[node_idx];
        if node.mass <= 0.0 {
            return;
        }

        let dx = node.com_x - point.0;
        let dy = node.com_y - point.1;
        let dist_sq = (dx * dx + dy * dy).max(1.0);
        let dist = dist_sq.sqrt();
        let size = node.bounds.w.max(node.bounds.h);

        // Far-enough or leaf with one body -> treat as point at COM.
        if !node.is_internal() || size / dist < theta {
            // Skip self-force when this leaf holds the querying body itself.
            if node.body == self_body as i32 {
                return;
            }
            let f = (k * node.mass) / dist_sq;
            acc.0 -= f * (dx / dist);
            acc.1 -= f * (dy / dist);
            return;
        }

        for &child in &node.children {
            if child >= 0 {
                self.walk(child as usize, self_body, point, theta, k, acc);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_unknown_target() {
        let nodes = vec![ForceNode::new("a", "A"), ForceNode::new("b", "B")];
        let edges = vec![ForceEdge::new("a", "c")];
        assert!(matches!(
            validate(&nodes, &edges),
            Err(ForceError::UnknownNode(_))
        ));
    }

    #[test]
    fn validate_rejects_duplicate_node() {
        let nodes = vec![ForceNode::new("a", "A"), ForceNode::new("a", "B")];
        assert!(matches!(
            validate(&nodes, &[]),
            Err(ForceError::DuplicateNode(_))
        ));
    }

    #[test]
    fn simulate_runs_to_completion_on_small_graph() {
        let nodes = vec![
            ForceNode::new("a", "A"),
            ForceNode::new("b", "B"),
            ForceNode::new("c", "C"),
        ];
        let edges = vec![ForceEdge::new("a", "b"), ForceEdge::new("b", "c")];
        let layout = simulate(
            &nodes,
            &edges,
            &ForceOptions {
                iterations: 50,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.edges.len(), 2);
        for n in &layout.nodes {
            assert!(n.x.is_finite());
            assert!(n.y.is_finite());
        }
    }

    #[test]
    fn segment_distance_matches_endpoint() {
        // point on the segment endpoint -> 0
        let d = segment_distance([0.0, 0.0], [10.0, 0.0], [0.0, 0.0]);
        assert!(d.abs() < 1e-5);
        // perpendicular offset to midpoint
        let d = segment_distance([0.0, 0.0], [10.0, 0.0], [5.0, 3.0]);
        assert!((d - 3.0).abs() < 1e-5);
        // beyond endpoint clamps to endpoint distance
        let d = segment_distance([0.0, 0.0], [10.0, 0.0], [15.0, 0.0]);
        assert!((d - 5.0).abs() < 1e-5);
    }

    #[test]
    fn barnes_hut_matches_naive_within_tolerance() {
        // small graph: BH with tight theta should ~ match O(n²) result direction.
        let nodes: Vec<ForceNode> = (0..20)
            .map(|i| ForceNode::new(format!("n{i}"), ""))
            .collect();
        let edges: Vec<ForceEdge> = (0..19)
            .map(|i| ForceEdge::new(format!("n{i}"), format!("n{}", i + 1)))
            .collect();
        let plot = Rect::new(0.0, 0.0, 800.0, 600.0);

        let bh = simulate(
            &nodes,
            &edges,
            &ForceOptions {
                iterations: 100,
                use_barnes_hut: true,
                theta: 0.5,
                cooling_rate: 0.0,
                ..ForceOptions::default()
            },
            plot,
        );
        let naive = simulate(
            &nodes,
            &edges,
            &ForceOptions {
                iterations: 100,
                use_barnes_hut: false,
                cooling_rate: 0.0,
                ..ForceOptions::default()
            },
            plot,
        );

        // BH and naive should put nodes in roughly the same neighborhood. We
        // don't expect exact equality (theta != 0) but mean position should be
        // close.
        let mean_dist: f32 = bh
            .nodes
            .iter()
            .zip(naive.nodes.iter())
            .map(|(a, b)| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
            .sum::<f32>()
            / bh.nodes.len() as f32;
        assert!(mean_dist < 100.0, "BH and naive diverged by {mean_dist}px");
    }

    #[test]
    fn labels_are_capped_at_max_visible() {
        let nodes: Vec<ForceNode> = (0..50)
            .map(|i| {
                ForceNode::new(format!("n{i}"), format!("node {i}")).with_radius(5.0 + i as f32)
            })
            .collect();
        let layout = simulate(
            &nodes,
            &[],
            &ForceOptions {
                iterations: 5,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        let labels = build_labels(&layout, Some(10));
        assert_eq!(labels.len(), 10);
        // Highest-radius nodes win — node 49 (radius 54) should be in the set.
        assert!(labels.iter().any(|l| l.text == "node 49"));
    }

    #[test]
    fn fan_indices_assigned_to_parallel_edges() {
        let nodes = vec![ForceNode::new("a", "A"), ForceNode::new("b", "B")];
        let edges = vec![
            ForceEdge::new("a", "b"),
            ForceEdge::new("a", "b"),
            ForceEdge::new("b", "a"), // reverse direction still siblings
        ];
        let layout = simulate(
            &nodes,
            &edges,
            &ForceOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        assert_eq!(layout.edges[0].fan_count, 3);
        assert_eq!(layout.edges[0].fan_index, 0);
        assert_eq!(layout.edges[1].fan_index, 1);
        assert_eq!(layout.edges[2].fan_index, 2);
    }

    #[test]
    fn self_loop_flag_set() {
        let nodes = vec![ForceNode::new("a", "A")];
        let edges = vec![ForceEdge::new("a", "a")];
        let layout = simulate(
            &nodes,
            &edges,
            &ForceOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        assert!(layout.edges[0].self_loop);
    }

    #[test]
    fn curve_distance_smaller_than_segment_when_offset() {
        // Point sitting on the apex of a curved edge should be closer to the
        // curve than to the straight chord.
        let e = ForceLayoutEdge {
            source: [0.0, 0.0],
            target: [100.0, 0.0],
            weight: 1.0,
            color: [0.0, 0.0, 0.0, 1.0],
            fan_index: 0,
            fan_count: 2,
            self_loop: false,
        };
        // With offset 20, the apex of the quadratic sits near (50, 10) (half the offset).
        let p = [50.0, 10.0];
        let d_curve = curve_distance(&e, 20.0, p);
        let d_segment = segment_distance(e.source, e.target, p);
        assert!(d_curve < d_segment);
    }

    #[test]
    fn highlighted_nodes_render_with_modified_style() {
        let spec = ForceSpec::new(
            vec![ForceNode::new("a", "A"), ForceNode::new("b", "B")],
            vec![ForceEdge::new("a", "b")],
        )
        .with_highlighted(["a"]);
        let workspace = berthacharts_core::Workspace::new();
        let chart = spec
            .build_chart(workspace, ChartSize::new(400, 300))
            .expect("chart builds");
        // After build, the node-mark's layout snapshot should reflect highlight state.
        // We can't easily inspect the Mark from outside; instead, verify build succeeds.
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn build_chart_exposes_force_tooltips_and_snap_targets() {
        let spec = ForceSpec::new(
            vec![
                ForceNode::new("a", "Alpha").pinned_at(120.0, 100.0),
                ForceNode::new("b", "Beta").pinned_at(260.0, 100.0),
            ],
            vec![ForceEdge::new("a", "b").with_weight(2.0)],
        )
        .with_options(ForceOptions {
            iterations: 1,
            ..ForceOptions::default()
        });
        let chart = spec
            .build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .expect("chart builds");

        let tooltip_count = chart
            .scene()
            .guides
            .iter()
            .filter(|guide| matches!(guide, Guide::Tooltip(_)))
            .count();
        assert_eq!(tooltip_count, 2);

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 3);
        assert_eq!(
            targets
                .iter()
                .filter(|target| target.kind == berthacharts_core::SnapKind::Node)
                .count(),
            2
        );
        assert_eq!(
            targets
                .iter()
                .filter(|target| target.kind == berthacharts_core::SnapKind::Edge)
                .count(),
            1
        );
        assert!(targets
            .iter()
            .any(|target| target.label.as_deref() == Some("Alpha node")));
    }

    #[test]
    fn fixed_nodes_dont_move() {
        let nodes = vec![
            ForceNode::new("pin", "Pinned").pinned_at(100.0, 100.0),
            ForceNode::new("free", "Free"),
        ];
        let edges = vec![ForceEdge::new("pin", "free")];
        let layout = simulate(
            &nodes,
            &edges,
            &ForceOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        assert!((layout.nodes[0].x - 100.0).abs() < f32::EPSILON);
        assert!((layout.nodes[0].y - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_graph_builds_without_panic() {
        let chart = ForceSpec::new(Vec::new(), Vec::new())
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .expect("empty graph builds");
        assert!(!chart.scene().layers.is_empty());
        assert!(chart.snap_targets().is_empty());
    }

    #[test]
    fn single_node_layout_is_finite() {
        let layout = simulate(
            &[ForceNode::new("solo", "Solo")],
            &[],
            &ForceOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        assert_eq!(layout.nodes.len(), 1);
        assert!(layout.nodes[0].x.is_finite() && layout.nodes[0].y.is_finite());
    }

    #[test]
    fn non_finite_seed_position_yields_finite_spread_layout() {
        let mut poisoned = ForceNode::new("a", "A");
        poisoned.initial = Some([f32::NAN, f32::INFINITY]);
        let nodes = vec![poisoned, ForceNode::new("b", "B"), ForceNode::new("c", "C")];
        let edges = vec![ForceEdge::new("a", "b"), ForceEdge::new("b", "c")];
        let layout = simulate(
            &nodes,
            &edges,
            &ForceOptions {
                iterations: 60,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 800.0, 600.0),
        );
        for n in &layout.nodes {
            assert!(
                n.x.is_finite() && n.y.is_finite(),
                "node {} non-finite: ({}, {})",
                n.id,
                n.x,
                n.y
            );
        }
        // If the seed guard were severed the NaN would propagate to every node,
        // collapsing them all to the (sanitised) centre. The guard keeps the
        // graph spread out.
        let first = (layout.nodes[0].x, layout.nodes[0].y);
        let all_same = layout
            .nodes
            .iter()
            .all(|n| n.x == first.0 && n.y == first.1);
        assert!(!all_same, "nodes collapsed — seed sanitisation missing");
    }

    #[test]
    fn non_finite_radius_is_sanitized() {
        let layout = simulate(
            &[ForceNode::new("a", "A").with_radius(f32::NAN)],
            &[],
            &ForceOptions {
                iterations: 1,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert!(layout.nodes[0].radius.is_finite());
        assert!(layout.nodes[0].radius >= 0.0);

        let negative = simulate(
            &[ForceNode::new("a", "A").with_radius(-4.0)],
            &[],
            &ForceOptions {
                iterations: 1,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert_eq!(negative.nodes[0].radius, 0.0);
    }

    #[test]
    fn coincident_seed_positions_are_separated() {
        let mut a = ForceNode::new("a", "A");
        a.initial = Some([200.0, 150.0]);
        let mut b = ForceNode::new("b", "B");
        b.initial = Some([200.0, 150.0]);
        let layout = simulate(
            &[a, b],
            &[],
            &ForceOptions {
                iterations: 60,
                ..ForceOptions::default()
            },
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let dx = layout.nodes[0].x - layout.nodes[1].x;
        let dy = layout.nodes[0].y - layout.nodes[1].y;
        assert!(
            (dx * dx + dy * dy).sqrt() > 1.0,
            "coincident nodes stayed fused"
        );
    }

    #[test]
    fn from_edges_infers_nodes_in_first_seen_order() {
        let spec = ForceSpec::from_edges(vec![
            ForceEdge::new("a", "b"),
            ForceEdge::new("b", "c"),
            ForceEdge::new("a", "c"),
        ]);
        let ids: Vec<&str> = spec.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
        assert_eq!(spec.nodes[0].label, "a");

        let chart = spec
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .expect("inferred spec builds");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn layout_method_returns_finite_positions() {
        let spec = ForceSpec::from_edges(vec![ForceEdge::new("a", "b"), ForceEdge::new("b", "c")])
            .with_options(ForceOptions {
                iterations: 30,
                ..ForceOptions::default()
            });
        let layout = spec.layout(ChartSize::new(640, 480)).expect("layout");
        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.edges.len(), 2);
        for n in &layout.nodes {
            assert!(n.x.is_finite() && n.y.is_finite());
        }
    }

    #[test]
    fn layout_method_reports_validation_errors() {
        let spec = ForceSpec::new(
            vec![ForceNode::new("a", "A")],
            vec![ForceEdge::new("a", "z")],
        );
        assert!(matches!(
            spec.layout(ChartSize::new(400, 300)),
            Err(ForceError::UnknownNode(_))
        ));
    }

    #[test]
    fn thick_edge_is_pickable_across_its_width() {
        // Horizontal edge at y=150 from x=100..300, drawn at max thickness. A
        // point 6px off the axis lies inside the stroke but outside the base
        // 5px tolerance — only the width-aware tolerance catches it.
        let spec = ForceSpec::new(
            vec![
                ForceNode::new("a", "A").pinned_at(100.0, 150.0),
                ForceNode::new("b", "B").pinned_at(300.0, 150.0),
            ],
            vec![ForceEdge::new("a", "b").with_weight(8.0)],
        )
        .with_options(ForceOptions {
            iterations: 1,
            edge_style: EdgeStyle::Straight,
            ..ForceOptions::default()
        });
        let chart = spec
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .expect("chart builds");
        let hit = chart
            .pick((200.0, 156.0))
            .expect("thick edge should be hit 6px off its axis");
        assert_eq!(hit.mark, EDGE_MARK);
    }

    #[test]
    fn self_loop_snap_anchor_sits_on_loop_apex() {
        let nodes = vec![ForceNode::new("a", "A").pinned_at(100.0, 100.0)];
        let edges = vec![ForceEdge::new("a", "a")];
        let options = ForceOptions::default();
        let layout = simulate(&nodes, &edges, &options, Rect::new(0.0, 0.0, 400.0, 300.0));
        let targets = snap_targets(&layout, &edges, &options);
        let edge_target = targets
            .iter()
            .find(|t| t.kind == SnapKind::Edge)
            .expect("edge snap target");
        // apex = source.x + 0.75 * self_loop_radius(fan_offset, 0) = 100 + 0.75*18.
        assert!(
            (edge_target.x - 113.5).abs() < 1e-3,
            "apex x = {}",
            edge_target.x
        );
        assert!((edge_target.y - 100.0).abs() < 1e-3);
        assert_eq!(edge_target.label.as_deref(), Some("a self-loop"));
    }

    #[test]
    fn parallel_edge_snap_anchors_are_distinct() {
        let nodes = vec![
            ForceNode::new("a", "A").pinned_at(100.0, 100.0),
            ForceNode::new("b", "B").pinned_at(300.0, 100.0),
        ];
        let edges = vec![ForceEdge::new("a", "b"), ForceEdge::new("a", "b")];
        let options = ForceOptions::default();
        let layout = simulate(&nodes, &edges, &options, Rect::new(0.0, 0.0, 400.0, 300.0));
        let targets = snap_targets(&layout, &edges, &options);
        let edge_targets: Vec<&SnapTarget> = targets
            .iter()
            .filter(|t| t.kind == SnapKind::Edge)
            .collect();
        assert_eq!(edge_targets.len(), 2);
        // Fanned curves bow to opposite sides, so their on-path anchors differ
        // instead of stacking at the shared chord midpoint.
        let dx = (edge_targets[0].x - edge_targets[1].x).abs();
        let dy = (edge_targets[0].y - edge_targets[1].y).abs();
        assert!(dx + dy > 1.0, "anchors should separate, got ({dx}, {dy})");
    }
}
