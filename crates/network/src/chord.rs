//! Chord chart: nodes placed on a circle's perimeter, weighted ribbons drawn
//! between them through the disk. Useful for showing pairwise relationships
//! (matrix-like) without drowning in lines.

use std::f32::consts::TAU;
use std::fmt;
use std::sync::Arc;

use ahash::AHashMap;
use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, LabelGuide, LabelItem, LabelPriority, Layer, LayerId, LinearScale,
    Mark, MarkId, PathCommand, PathPrim, PickCtx, PickHit, Rect, Scale, ScaleId, Scene,
    TessellateCtx, Workspace,
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

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: ChordOptions) -> Self {
        self.options = options;
        self
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
    /// Start angle (radians, 0 = east, sweep CCW).
    pub start_angle: f32,
    /// End angle (radians).
    pub end_angle: f32,
    /// Sum of link values touching this node — drives arc length.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChordError {
    /// Link references an unknown node id.
    UnknownNode(String),
    /// Duplicate node id.
    DuplicateNode(String),
    /// No nodes, or all link values zero — nothing to lay out.
    Empty,
}

impl fmt::Display for ChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "link references unknown node id: {id}"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
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
        workspace.upsert_dataset(link_dataset(&layout));

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
    let inner_radius = (outer_radius - options.arc_thickness).max(20.0);

    // Allocate arcs.
    let total_gap = options.arc_gap * n as f32;
    let usable_angle = TAU - total_gap;
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
        cursor += span + options.arc_gap;
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
        consumed[s] += s_share;

        let t_start = arcs[t].start_angle + consumed[t];
        let t_end = t_start + t_share;
        if s != t {
            consumed[t] += t_share;
        } else {
            // self-loop already consumed via s_share; bump again for symmetry
            consumed[t] += t_share;
        }

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
    // Path: M src_start -> arc to src_end -> Q center -> target_end -> arc back -> Q center -> src_start
    let src_start = point_at(center, inner, ribbon.source_start);
    let src_end = point_at(center, inner, ribbon.source_end);
    let tgt_start = point_at(center, inner, ribbon.target_start);
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
    // suppress unused warnings on tgt_start and src_end
    let _ = (tgt_start, src_end);

    PathPrim {
        commands,
        fill: color,
        stroke: [0.0, 0.0, 0.0, 0.0],
        stroke_width: 0.0,
    }
}

fn build_labels(layout: &ChordLayout, offset: f32) -> Vec<LabelItem> {
    let radius = layout.outer_radius + offset;
    layout
        .arcs
        .iter()
        .filter(|a| a.end_angle > a.start_angle + 0.001)
        .map(|a| {
            let mid = (a.start_angle + a.end_angle) * 0.5;
            let p = point_at(layout.center, radius, mid);
            LabelItem::new(p[0], p[1], a.label.clone()).with_priority(LabelPriority::Important)
        })
        .collect()
}

fn node_dataset(layout: &ChordLayout) -> Dataset {
    let mut id_col: Vec<Arc<str>> = Vec::with_capacity(layout.arcs.len());
    let mut total: Vec<f32> = Vec::with_capacity(layout.arcs.len());
    for a in &layout.arcs {
        id_col.push(Arc::from(a.node_id.as_str()));
        total.push(a.total_value);
    }
    Dataset::new(
        NODE_DATASET,
        1,
        vec![
            ("id".to_string(), Column::Utf8(ColumnData::new(id_col))),
            ("total".to_string(), Column::F32(ColumnData::new(total))),
        ],
    )
}

fn link_dataset(layout: &ChordLayout) -> Dataset {
    let mut values: Vec<f32> = Vec::with_capacity(layout.ribbons.len());
    for r in &layout.ribbons {
        values.push(r.value);
    }
    Dataset::new(
        LINK_DATASET,
        1,
        vec![("value".to_string(), Column::F32(ColumnData::new(values)))],
    )
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
        // Approximate: hit when point is within the inner disk AND near a ribbon
        // sample. For v1 we just test inner disk and return the closest ribbon
        // by midpoint distance.
        let (px, py) = point;
        let dx = px - self.layout.center[0];
        let dy = py - self.layout.center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > self.layout.inner_radius {
            return None;
        }
        let mut best: Option<(usize, f32)> = None;
        for (row, r) in self.layout.ribbons.iter().enumerate() {
            let mid_s = (r.source_start + r.source_end) * 0.5;
            let mid_t = (r.target_start + r.target_end) * 0.5;
            let p_s = point_at(self.layout.center, self.layout.inner_radius, mid_s);
            let p_t = point_at(self.layout.center, self.layout.inner_radius, mid_t);
            let mx = (p_s[0] + p_t[0]) * 0.5;
            let my = (p_s[1] + p_t[1]) * 0.5;
            let d = ((mx - px).powi(2) + (my - py).powi(2)).sqrt();
            if d < 12.0 && best.map_or(true, |(_, bd)| d < bd) {
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
}
