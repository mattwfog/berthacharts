//! Tree / dendogram layout.
//!
//! Hierarchical layout for parent-child trees. Simplified Reingold-Tilford:
//! depth determines axis, sibling order + subtree width determines the other.
//! Supports top-down / bottom-up / left-to-right / right-to-left orientations.

use std::fmt;
use std::sync::Arc;

use ahash::AHashMap;
use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LinePrim, LinearScale, Mark, MarkId, PickCtx, PickHit,
    PointPrim, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx,
    TooltipField, TooltipGuide, Workspace,
};

const NODE_DATASET: DatasetId = DatasetId::new(0);
const EDGE_DATASET: DatasetId = DatasetId::new(1);
const NODE_MARK: MarkId = MarkId::new(1);
const EDGE_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// A node in the input tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    /// Stable id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Parent id. `None` denotes the root. Exactly one root allowed.
    pub parent: Option<String>,
    /// Node radius in screen pixels.
    pub radius: f32,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
}

impl TreeNode {
    /// Build a root node with default styling.
    #[must_use]
    pub fn root(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            parent: None,
            radius: 7.0,
            color: [0.55, 0.45, 0.85, 1.0],
        }
    }

    /// Build a child node.
    #[must_use]
    pub fn child(
        id: impl Into<String>,
        label: impl Into<String>,
        parent: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            parent: Some(parent.into()),
            radius: 6.0,
            color: [0.45, 0.55, 0.85, 1.0],
        }
    }

    /// Override radius.
    #[must_use]
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Override colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// An explicit parent→child edge. Optional — most trees express edges via
/// `TreeNode.parent`. Use this when you need per-edge styling.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeEdge {
    /// Parent id.
    pub parent: String,
    /// Child id.
    pub child: String,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
}

impl TreeEdge {
    /// Build a tree edge.
    #[must_use]
    pub fn new(parent: impl Into<String>, child: impl Into<String>) -> Self {
        Self {
            parent: parent.into(),
            child: child.into(),
            color: [0.55, 0.55, 0.60, 0.7],
        }
    }
}

/// Tree orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeOrientation {
    /// Root at top, leaves at bottom. Default.
    TopDown,
    /// Root at bottom, leaves at top.
    BottomUp,
    /// Root at left, leaves to the right.
    LeftRight,
    /// Root at right, leaves to the left.
    RightLeft,
}

/// Tree layout configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeOptions {
    /// Tree orientation.
    pub orientation: TreeOrientation,
    /// Margin from the plot edges (pixels).
    pub padding: f32,
    /// Render node labels.
    pub show_labels: bool,
    /// Per-label cap.
    pub max_visible_labels: Option<usize>,
    /// Width of the line connecting parent to child.
    pub edge_width: f32,
}

impl Default for TreeOptions {
    fn default() -> Self {
        Self {
            orientation: TreeOrientation::TopDown,
            padding: 30.0,
            show_labels: true,
            max_visible_labels: None,
            edge_width: 1.2,
        }
    }
}

/// Tree chart spec.
#[derive(Debug, Clone)]
pub struct TreeSpec {
    nodes: Vec<TreeNode>,
    edges: Vec<TreeEdge>,
    options: TreeOptions,
}

impl TreeSpec {
    /// Build a tree spec. Edges are optional — if absent, parent→child links
    /// rendered from `TreeNode.parent` in default styling.
    #[must_use]
    pub fn new(nodes: Vec<TreeNode>, edges: Vec<TreeEdge>) -> Self {
        Self {
            nodes,
            edges,
            options: TreeOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: TreeOptions) -> Self {
        self.options = options;
        self
    }
}

/// Computed positions for nodes + edges.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeLayout {
    /// One per input node, same order.
    pub nodes: Vec<TreeLayoutNode>,
    /// One per parent→child relationship.
    pub edges: Vec<TreeLayoutEdge>,
}

/// A laid-out node.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeLayoutNode {
    /// Matches input id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Final x in screen pixels.
    pub x: f32,
    /// Final y in screen pixels.
    pub y: f32,
    /// Render radius.
    pub radius: f32,
    /// Premultiplied RGBA fill.
    pub color: [f32; 4],
    /// Depth from root (0 = root).
    pub depth: u32,
}

/// A laid-out edge.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeLayoutEdge {
    /// Parent endpoint.
    pub parent: [f32; 2],
    /// Child endpoint.
    pub child: [f32; 2],
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
}

/// Errors during tree build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// No root node (every node has a parent).
    NoRoot,
    /// More than one node has `parent: None`.
    MultipleRoots(Vec<String>),
    /// Node references a parent that doesn't exist.
    UnknownParent(String),
    /// Duplicate node id.
    DuplicateNode(String),
    /// Cycle detected — input is not a tree.
    Cycle(String),
    /// Edge references unknown node.
    UnknownEdgeNode(String),
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRoot => write!(f, "tree has no root (every node has a parent)"),
            Self::MultipleRoots(ids) => write!(f, "multiple roots: {}", ids.join(", ")),
            Self::UnknownParent(p) => write!(f, "node references unknown parent: {p}"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
            Self::Cycle(id) => write!(f, "cycle detected near node: {id}"),
            Self::UnknownEdgeNode(id) => write!(f, "edge references unknown node: {id}"),
        }
    }
}

impl std::error::Error for TreeError {}

impl ChartSpec for TreeSpec {
    type Error = TreeError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        let layout = compute_layout(
            &self.nodes,
            &self.edges,
            &self.options,
            size.full_viewport().plot_area,
        )?;
        let viewport = size.full_viewport();

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
        workspace.upsert_dataset(edge_dataset(&layout, &self.nodes));

        let edge_mark: Arc<dyn Mark> = Arc::new(TreeEdgeMark::new(
            EDGE_MARK,
            layout.edges.clone(),
            self.options.edge_width,
        ));
        let node_mark: Arc<dyn Mark> = Arc::new(TreeNodeMark::new(NODE_MARK, layout.nodes.clone()));

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
            let labels = build_labels(
                &layout,
                self.options.max_visible_labels,
                self.options.orientation,
            );
            if !labels.is_empty() {
                let mut guide = LabelGuide::new(labels).with_collision_padding(3.0);
                if let Some(cap) = self.options.max_visible_labels {
                    guide = guide.with_max_visible(cap);
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
                    TooltipField::new("Depth", "depth").as_integer(),
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
                    TooltipField::new("Parent", "parent").as_label(),
                    TooltipField::new("Child", "child").as_label(),
                ],
            )
            .with_title_column("link"),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, &self.nodes)).with_name("tree anchors"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn compute_layout(
    nodes: &[TreeNode],
    edges: &[TreeEdge],
    options: &TreeOptions,
    plot: Rect,
) -> Result<TreeLayout, TreeError> {
    let n = nodes.len();
    let mut id_to_index = AHashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if id_to_index.insert(node.id.clone(), i).is_some() {
            return Err(TreeError::DuplicateNode(node.id.clone()));
        }
    }
    for e in edges {
        if !id_to_index.contains_key(&e.parent) {
            return Err(TreeError::UnknownEdgeNode(e.parent.clone()));
        }
        if !id_to_index.contains_key(&e.child) {
            return Err(TreeError::UnknownEdgeNode(e.child.clone()));
        }
    }

    let mut roots: Vec<usize> = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, node) in nodes.iter().enumerate() {
        match &node.parent {
            None => roots.push(i),
            Some(p) => {
                let pi = id_to_index
                    .get(p)
                    .copied()
                    .ok_or_else(|| TreeError::UnknownParent(p.clone()))?;
                children[pi].push(i);
            }
        }
    }
    if roots.is_empty() {
        return Err(TreeError::NoRoot);
    }
    if roots.len() > 1 {
        let names: Vec<String> = roots.iter().map(|&i| nodes[i].id.clone()).collect();
        return Err(TreeError::MultipleRoots(names));
    }
    let root = roots[0];

    // Cycle check via DFS.
    let mut depth = vec![u32::MAX; n];
    let mut order = Vec::with_capacity(n);
    {
        let mut stack: Vec<(usize, u32)> = vec![(root, 0)];
        while let Some((i, d)) = stack.pop() {
            if depth[i] != u32::MAX {
                return Err(TreeError::Cycle(nodes[i].id.clone()));
            }
            depth[i] = d;
            order.push(i);
            for &c in children[i].iter().rev() {
                stack.push((c, d + 1));
            }
        }
    }
    if order.len() != n {
        // Some node not reached from root — disconnected
        return Err(TreeError::Cycle("(disconnected nodes)".to_string()));
    }

    // Simple "subtree-width" layout: leaf nodes get equal slots; internal nodes
    // anchor at the midpoint of their children's slots.
    let mut leaf_index = 0_usize;
    let mut x_slot = vec![0.0_f32; n];

    // Post-order traversal: iterative
    fn assign_slots(
        node: usize,
        children: &[Vec<usize>],
        x_slot: &mut [f32],
        leaf_index: &mut usize,
    ) {
        let kids = &children[node];
        if kids.is_empty() {
            x_slot[node] = *leaf_index as f32;
            *leaf_index += 1;
            return;
        }
        for &k in kids {
            assign_slots(k, children, x_slot, leaf_index);
        }
        let lo = x_slot[kids[0]];
        let hi = x_slot[*kids.last().unwrap()];
        x_slot[node] = (lo + hi) * 0.5;
    }
    assign_slots(root, &children, &mut x_slot, &mut leaf_index);

    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let total_slots = leaf_index.max(1) as f32;
    let pad = options.padding;
    let plot_w = (plot.w - 2.0 * pad).max(1.0);
    let plot_h = (plot.h - 2.0 * pad).max(1.0);

    let mut layout_nodes = Vec::with_capacity(n);
    for (i, node) in nodes.iter().enumerate() {
        let slot_fraction = if total_slots > 1.0 {
            x_slot[i] / (total_slots - 1.0)
        } else {
            0.5
        };
        let depth_fraction = if max_depth > 0 {
            depth[i] as f32 / max_depth as f32
        } else {
            0.5
        };
        let (x, y) = match options.orientation {
            TreeOrientation::TopDown => (
                plot.x + pad + slot_fraction * plot_w,
                plot.y + pad + depth_fraction * plot_h,
            ),
            TreeOrientation::BottomUp => (
                plot.x + pad + slot_fraction * plot_w,
                plot.y + pad + (1.0 - depth_fraction) * plot_h,
            ),
            TreeOrientation::LeftRight => (
                plot.x + pad + depth_fraction * plot_w,
                plot.y + pad + slot_fraction * plot_h,
            ),
            TreeOrientation::RightLeft => (
                plot.x + pad + (1.0 - depth_fraction) * plot_w,
                plot.y + pad + slot_fraction * plot_h,
            ),
        };
        layout_nodes.push(TreeLayoutNode {
            id: node.id.clone(),
            label: node.label.clone(),
            x,
            y,
            radius: node.radius,
            color: node.color,
            depth: depth[i],
        });
    }

    // Generate edges: prefer explicit TreeEdge entries, else infer from parent links.
    let mut edge_lookup: AHashMap<(String, String), [f32; 4]> = AHashMap::new();
    for e in edges {
        edge_lookup.insert((e.parent.clone(), e.child.clone()), e.color);
    }
    let mut layout_edges = Vec::new();
    for (ci, node) in nodes.iter().enumerate() {
        if let Some(parent_id) = &node.parent {
            let pi = id_to_index[parent_id];
            let color = edge_lookup
                .get(&(parent_id.clone(), node.id.clone()))
                .copied()
                .unwrap_or([0.55, 0.55, 0.60, 0.7]);
            layout_edges.push(TreeLayoutEdge {
                parent: [layout_nodes[pi].x, layout_nodes[pi].y],
                child: [layout_nodes[ci].x, layout_nodes[ci].y],
                color,
            });
        }
    }

    Ok(TreeLayout {
        nodes: layout_nodes,
        edges: layout_edges,
    })
}

fn build_labels(
    layout: &TreeLayout,
    max_visible: Option<usize>,
    orientation: TreeOrientation,
) -> Vec<LabelItem> {
    let mut indices: Vec<usize> = (0..layout.nodes.len()).collect();
    indices.sort_by_key(|&i| layout.nodes[i].depth);
    let take = max_visible
        .unwrap_or(layout.nodes.len())
        .min(layout.nodes.len());
    indices.truncate(take);
    indices
        .into_iter()
        .map(|i| {
            let n = &layout.nodes[i];
            let (x, y, anchor) = match orientation {
                TreeOrientation::TopDown => (n.x, n.y - n.radius - 6.0, LabelAnchor::Top),
                TreeOrientation::BottomUp => (n.x, n.y + n.radius + 6.0, LabelAnchor::Bottom),
                TreeOrientation::LeftRight => (n.x + n.radius + 8.0, n.y, LabelAnchor::Right),
                TreeOrientation::RightLeft => (n.x - n.radius - 8.0, n.y, LabelAnchor::Left),
            };
            LabelItem::new(x, y, n.label.clone())
                .with_anchor(anchor)
                .with_kind(LabelKind::Node)
                .with_priority(LabelPriority::Important)
        })
        .collect()
}

fn node_dataset(layout: &TreeLayout) -> Dataset {
    let mut id_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut label_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut x_col = Vec::with_capacity(layout.nodes.len());
    let mut y_col = Vec::with_capacity(layout.nodes.len());
    let mut depth_col = Vec::with_capacity(layout.nodes.len());
    let mut radius_col = Vec::with_capacity(layout.nodes.len());
    for n in &layout.nodes {
        id_col.push(Arc::from(n.id.as_str()));
        label_col.push(Arc::from(n.label.as_str()));
        x_col.push(n.x);
        y_col.push(n.y);
        depth_col.push(n.depth as i64);
        radius_col.push(n.radius);
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
            ("depth".to_string(), Column::I64(ColumnData::new(depth_col))),
            (
                "radius".to_string(),
                Column::F32(ColumnData::new(radius_col)),
            ),
        ],
    )
}

fn edge_dataset(layout: &TreeLayout, input_nodes: &[TreeNode]) -> Dataset {
    let mut link_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut parent_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut child_col: Vec<Arc<str>> = Vec::with_capacity(layout.edges.len());
    let mut px = Vec::with_capacity(layout.edges.len());
    let mut py = Vec::with_capacity(layout.edges.len());
    let mut cx = Vec::with_capacity(layout.edges.len());
    let mut cy = Vec::with_capacity(layout.edges.len());
    let metadata = tree_edge_metadata(input_nodes);
    debug_assert_eq!(layout.edges.len(), metadata.len());
    for (e, (parent, child)) in layout.edges.iter().zip(metadata.iter()) {
        link_col.push(Arc::from(format!("{parent} to {child}")));
        parent_col.push(Arc::from(parent.as_str()));
        child_col.push(Arc::from(child.as_str()));
        px.push(e.parent[0]);
        py.push(e.parent[1]);
        cx.push(e.child[0]);
        cy.push(e.child[1]);
    }
    Dataset::new(
        EDGE_DATASET,
        1,
        vec![
            ("link".to_string(), Column::Utf8(ColumnData::new(link_col))),
            (
                "parent".to_string(),
                Column::Utf8(ColumnData::new(parent_col)),
            ),
            (
                "child".to_string(),
                Column::Utf8(ColumnData::new(child_col)),
            ),
            ("parent_x".to_string(), Column::F32(ColumnData::new(px))),
            ("parent_y".to_string(), Column::F32(ColumnData::new(py))),
            ("child_x".to_string(), Column::F32(ColumnData::new(cx))),
            ("child_y".to_string(), Column::F32(ColumnData::new(cy))),
        ],
    )
}

fn snap_targets(layout: &TreeLayout, input_nodes: &[TreeNode]) -> Vec<SnapTarget> {
    let mut targets = Vec::with_capacity(layout.nodes.len() + layout.edges.len());
    targets.extend(layout.nodes.iter().map(|node| {
        SnapTarget::new(node.x, node.y, SnapKind::Node)
            .with_radius((node.radius + 4.0).clamp(6.0, 14.0))
            .with_label(format!("{} node", node.label))
            .with_priority(if node.depth == 0 { 4 } else { 3 })
    }));
    let metadata = tree_edge_metadata(input_nodes);
    targets.extend(
        layout
            .edges
            .iter()
            .zip(metadata.iter())
            .map(|(edge, (parent, child))| {
                SnapTarget::new(
                    (edge.parent[0] + edge.child[0]) * 0.5,
                    (edge.parent[1] + edge.child[1]) * 0.5,
                    SnapKind::Edge,
                )
                .with_radius(6.0)
                .with_label(format!("{parent} to {child}"))
                .with_priority(1)
            }),
    );
    targets
}

fn tree_edge_metadata(input_nodes: &[TreeNode]) -> Vec<(String, String)> {
    input_nodes
        .iter()
        .filter_map(|node| {
            node.parent
                .as_ref()
                .map(|parent| (parent.clone(), node.id.clone()))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct TreeNodeMark {
    id: MarkId,
    nodes: Vec<TreeLayoutNode>,
}

impl TreeNodeMark {
    fn new(id: MarkId, nodes: Vec<TreeLayoutNode>) -> Self {
        Self { id, nodes }
    }
}

impl Mark for TreeNodeMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.nodes.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut points = Vec::with_capacity(self.nodes.len());
        for n in &self.nodes {
            points.push(PointPrim {
                x: n.x,
                y: n.y,
                r: n.radius,
                shape: 0,
                fill: n.color,
                stroke: [1.0, 1.0, 1.0, 0.6],
                stroke_width: 1.0,
            });
        }
        Geometry::Points(points)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        for (row, n) in self.nodes.iter().enumerate() {
            let dx = px - n.x;
            let dy = py - n.y;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= n.radius + 2.0 && best.is_none_or(|(_, bd)| d < bd) {
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
struct TreeEdgeMark {
    id: MarkId,
    edges: Vec<TreeLayoutEdge>,
    width: f32,
}

impl TreeEdgeMark {
    fn new(id: MarkId, edges: Vec<TreeLayoutEdge>, width: f32) -> Self {
        Self { id, edges, width }
    }
}

impl Mark for TreeEdgeMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.edges.len() as u64;
        h ^= self.width.to_bits() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut lines = Vec::with_capacity(self.edges.len());
        for e in &self.edges {
            lines.push(LinePrim {
                points: vec![e.parent, e.child],
                stroke: e.color,
                width: self.width,
                dash: None,
                join: 1,
                cap: 1,
            });
        }
        Geometry::Lines(lines)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        const TOL: f32 = 4.0;
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        for (row, e) in self.edges.iter().enumerate() {
            let d = segment_distance(e.parent, e.child, [px, py]);
            if d <= TOL && best.is_none_or(|(_, bd)| d < bd) {
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
        for e in &self.edges {
            min_x = min_x.min(e.parent[0]).min(e.child[0]);
            min_y = min_y.min(e.parent[1]).min(e.child[1]);
            max_x = max_x.max(e.parent[0]).max(e.child[0]);
            max_y = max_y.max(e.parent[1]).max(e.child[1]);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_no_root() {
        let nodes = vec![
            TreeNode::child("a", "A", "b"),
            TreeNode::child("b", "B", "a"),
        ];
        let result = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::NoRoot)));
    }

    #[test]
    fn rejects_multiple_roots() {
        let nodes = vec![TreeNode::root("a", "A"), TreeNode::root("b", "B")];
        let result = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::MultipleRoots(_))));
    }

    #[test]
    fn rejects_unknown_parent() {
        let nodes = vec![TreeNode::root("a", "A"), TreeNode::child("b", "B", "ghost")];
        let result = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::UnknownParent(_))));
    }

    #[test]
    fn rejects_duplicate_id() {
        let nodes = vec![TreeNode::root("a", "A"), TreeNode::child("a", "A2", "a")];
        let result = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::DuplicateNode(_))));
    }

    #[test]
    fn compute_layout_places_root_at_depth_zero() {
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("c1", "C1", "r"),
            TreeNode::child("c2", "C2", "r"),
        ];
        let layout = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        )
        .expect("layout");
        assert_eq!(layout.nodes[0].depth, 0);
        assert_eq!(layout.nodes[1].depth, 1);
        assert_eq!(layout.nodes[2].depth, 1);
        // Root's x should sit between its children's x.
        let rx = layout.nodes[0].x;
        let c1x = layout.nodes[1].x;
        let c2x = layout.nodes[2].x;
        assert!(rx >= c1x.min(c2x) && rx <= c1x.max(c2x));
        // Three nodes -> three positions, root + two edges -> two edges.
        assert_eq!(layout.edges.len(), 2);
    }

    #[test]
    fn orientation_left_right_places_root_on_left() {
        let nodes = vec![TreeNode::root("r", "Root"), TreeNode::child("c", "C", "r")];
        let layout = compute_layout(
            &nodes,
            &[],
            &TreeOptions {
                orientation: TreeOrientation::LeftRight,
                ..TreeOptions::default()
            },
            Rect::new(0.0, 0.0, 400.0, 400.0),
        )
        .expect("layout");
        assert!(layout.nodes[0].x < layout.nodes[1].x);
    }

    #[test]
    fn left_right_labels_are_offset_beside_nodes() {
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("c", "Child", "r"),
        ];
        let options = TreeOptions {
            orientation: TreeOrientation::LeftRight,
            ..TreeOptions::default()
        };
        let layout = compute_layout(
            &nodes,
            &[],
            &options,
            ChartSize::new(400, 300).full_viewport().plot_area,
        )
        .expect("layout");
        let chart = TreeSpec::new(nodes, vec![])
            .with_options(options)
            .build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .expect("chart");
        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        let root_label = labels
            .items
            .iter()
            .find(|label| label.text == "Root")
            .expect("root label");

        assert!(root_label.x > layout.nodes[0].x + layout.nodes[0].radius);
        assert_eq!(root_label.y, layout.nodes[0].y);
        assert_eq!(root_label.anchor, berthacharts_core::LabelAnchor::Right);
    }

    #[test]
    fn build_chart_succeeds() {
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("a", "A", "r"),
            TreeNode::child("b", "B", "r"),
        ];
        let workspace = berthacharts_core::Workspace::new();
        let chart = TreeSpec::new(nodes, vec![])
            .build_chart(workspace, ChartSize::new(400, 400))
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }
}
