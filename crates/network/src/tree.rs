//! Tree / dendrogram layout.
//!
//! Hierarchical layout for parent-child trees. Simplified Reingold-Tilford:
//! depth determines one axis, sibling order + subtree width determines the
//! other. Supports top-down / bottom-up / left-to-right / right-to-left
//! orientations.
//!
//! Structure is taken from [`TreeNode::parent`]; explicit [`TreeEdge`]s are
//! optional and only carry per-edge styling. The layout is defensive about
//! degenerate input:
//!
//! - A single node renders centered; a chain renders as a straight spine.
//! - Very deep and very wide trees are laid out iteratively, so pathological
//!   depth cannot overflow the call stack.
//! - Non-finite radii, colors, and padding are sanitized before they reach the
//!   renderer or the hit-tester.
//! - Multiple roots are rejected as [`TreeError::MultipleRoots`] unless
//!   [`TreeOptions::allow_forest`] opts into forest layout, and cycles /
//!   orphaned components are rejected as [`TreeError::Cycle`].
//!
//! ```
//! use berthacharts_network::tree::{TreeNode, TreeOptions, TreeOrientation, TreeSpec};
//! use berthacharts_network::core::ChartSize;
//!
//! let spec = TreeSpec::new(
//!     vec![
//!         TreeNode::root("root", "Root"),
//!         TreeNode::child("a", "A", "root"),
//!         TreeNode::child("b", "B", "root"),
//!     ],
//!     vec![],
//! )
//! .with_options(TreeOptions::default().with_orientation(TreeOrientation::LeftRight));
//!
//! let layout = spec.layout(ChartSize::new(400, 300)).expect("valid tree");
//! assert_eq!(layout.nodes.len(), 3);
//! assert_eq!(layout.edges.len(), 2);
//! ```

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
    /// Margin from the plot edges (pixels). Non-finite or negative values are
    /// treated as `0.0` during layout.
    pub padding: f32,
    /// Render node labels.
    pub show_labels: bool,
    /// Per-label cap. `None` lets the overlay attempt every label.
    pub max_visible_labels: Option<usize>,
    /// Width of the line connecting parent to child (pixels).
    pub edge_width: f32,
    /// Lay out a forest (more than one root) instead of rejecting it.
    ///
    /// When `false` (the default) a second root is a
    /// [`TreeError::MultipleRoots`]. When `true` each root's subtree is packed
    /// into its own contiguous band of leaf slots, so roots render side by side
    /// without overlap.
    pub allow_forest: bool,
}

impl Default for TreeOptions {
    fn default() -> Self {
        Self {
            orientation: TreeOrientation::TopDown,
            padding: 30.0,
            show_labels: true,
            max_visible_labels: None,
            edge_width: 1.2,
            allow_forest: false,
        }
    }
}

impl TreeOptions {
    /// Set the tree orientation.
    #[must_use]
    pub const fn with_orientation(mut self, orientation: TreeOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the edge-inset padding in pixels.
    #[must_use]
    pub const fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Toggle node labels on or off.
    #[must_use]
    pub const fn with_show_labels(mut self, show_labels: bool) -> Self {
        self.show_labels = show_labels;
        self
    }

    /// Cap the number of node labels the overlay may place.
    #[must_use]
    pub const fn with_max_visible_labels(mut self, max_visible_labels: usize) -> Self {
        self.max_visible_labels = Some(max_visible_labels);
        self
    }

    /// Set the parent→child connector width in pixels.
    #[must_use]
    pub const fn with_edge_width(mut self, edge_width: f32) -> Self {
        self.edge_width = edge_width;
        self
    }

    /// Lay out multiple roots as a forest instead of returning
    /// [`TreeError::MultipleRoots`].
    #[must_use]
    pub const fn with_allow_forest(mut self, allow_forest: bool) -> Self {
        self.allow_forest = allow_forest;
        self
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

    /// Compute the reusable node/edge layout for `size` without building a
    /// full [`Chart`]. Handy for tests, hit-testing harnesses, or callers that
    /// drive their own renderer.
    ///
    /// # Errors
    /// Returns a [`TreeError`] when the input is not a valid tree — or forest,
    /// when [`TreeOptions::allow_forest`] is set.
    pub fn layout(&self, size: ChartSize) -> Result<TreeLayout, TreeError> {
        compute_layout(
            &self.nodes,
            &self.edges,
            &self.options,
            size.full_viewport().plot_area,
        )
    }

    /// Compile this spec into a chart without importing the [`ChartSpec`]
    /// trait at the call site.
    ///
    /// # Errors
    /// Propagates any [`TreeError`] raised while computing the layout.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, TreeError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
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
        workspace.upsert_dataset(node_dataset(&layout, &self.nodes));
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
                    TooltipField::new("Children", "children").as_integer(),
                    TooltipField::new("Radius", "radius").as_number(1),
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
        // Zero nodes, or every node has a parent (a pure cycle).
        return Err(TreeError::NoRoot);
    }
    if roots.len() > 1 && !options.allow_forest {
        let names: Vec<String> = roots.iter().map(|&i| nodes[i].id.clone()).collect();
        return Err(TreeError::MultipleRoots(names));
    }

    // Depth + reachability via an iterative DFS seeded from every root. Using an
    // explicit heap stack (not recursion) keeps pathologically deep trees from
    // overflowing the call stack. Since every non-root node has exactly one
    // parent, a node reached twice signals a cycle.
    let mut depth = vec![u32::MAX; n];
    let mut order = Vec::with_capacity(n);
    {
        // Seed reversed so the first root pops (and lays out) first.
        let mut stack: Vec<(usize, u32)> = roots.iter().rev().map(|&r| (r, 0)).collect();
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
        // A node no root can reach sits on a parent cycle or in a disconnected
        // component. Report the first such node so the error is actionable.
        let orphan = nodes
            .iter()
            .enumerate()
            .find(|(i, _)| depth[*i] == u32::MAX)
            .map(|(_, node)| node.id.clone())
            .unwrap_or_else(|| "(unreachable)".to_string());
        return Err(TreeError::Cycle(orphan));
    }

    // "Subtree-width" layout: leaves take equal, contiguous slots in left-to-
    // right DFS order; each internal node centers over its first and last child.
    // Evaluated as an iterative post-order (explicit stack of `(node, next
    // child cursor)` frames) so depth cannot overflow the call stack. Roots are
    // visited in order, so each root's subtree owns a contiguous slot band and
    // forests never overlap.
    let mut leaf_index = 0_usize;
    let mut x_slot = vec![0.0_f32; n];
    {
        let mut stack: Vec<(usize, usize)> = Vec::new();
        for &root in &roots {
            stack.push((root, 0));
            while let Some((node, child_cursor)) = stack.last().copied() {
                if child_cursor < children[node].len() {
                    let child = children[node][child_cursor];
                    if let Some(top) = stack.last_mut() {
                        top.1 += 1;
                    }
                    stack.push((child, 0));
                } else {
                    let kids = &children[node];
                    if kids.is_empty() {
                        x_slot[node] = leaf_index as f32;
                        leaf_index += 1;
                    } else {
                        let lo = x_slot[kids[0]];
                        let hi = x_slot[*kids.last().unwrap()];
                        x_slot[node] = (lo + hi) * 0.5;
                    }
                    stack.pop();
                }
            }
        }
    }

    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let total_slots = leaf_index.max(1) as f32;
    // A non-finite or negative padding would poison every node coordinate.
    let pad = if options.padding.is_finite() {
        options.padding.max(0.0)
    } else {
        0.0
    };
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
            radius: sanitize_radius(node.radius),
            color: sanitize_color(node.color),
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
            let color = sanitize_color(
                edge_lookup
                    .get(&(parent_id.clone(), node.id.clone()))
                    .copied()
                    .unwrap_or([0.55, 0.55, 0.60, 0.7]),
            );
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
            // Root labels anchor the whole diagram; never let the overlay drop
            // one to place a leaf label instead.
            let priority = if n.depth == 0 {
                LabelPriority::Required
            } else {
                LabelPriority::Important
            };
            LabelItem::new(x, y, n.label.clone())
                .with_anchor(anchor)
                .with_kind(LabelKind::Node)
                .with_priority(priority)
        })
        .collect()
}

fn node_dataset(layout: &TreeLayout, input_nodes: &[TreeNode]) -> Dataset {
    // Fan-out per node, keyed by parent id, so tooltips can surface how many
    // direct children each node has (0 == leaf).
    let mut child_counts: AHashMap<&str, i64> = AHashMap::new();
    for node in input_nodes {
        if let Some(parent) = &node.parent {
            *child_counts.entry(parent.as_str()).or_insert(0) += 1;
        }
    }

    let mut id_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut label_col: Vec<Arc<str>> = Vec::with_capacity(layout.nodes.len());
    let mut x_col = Vec::with_capacity(layout.nodes.len());
    let mut y_col = Vec::with_capacity(layout.nodes.len());
    let mut depth_col = Vec::with_capacity(layout.nodes.len());
    let mut radius_col = Vec::with_capacity(layout.nodes.len());
    let mut children_col = Vec::with_capacity(layout.nodes.len());
    for n in &layout.nodes {
        id_col.push(Arc::from(n.id.as_str()));
        label_col.push(Arc::from(n.label.as_str()));
        x_col.push(n.x);
        y_col.push(n.y);
        depth_col.push(n.depth as i64);
        radius_col.push(n.radius);
        children_col.push(child_counts.get(n.id.as_str()).copied().unwrap_or(0));
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
            (
                "children".to_string(),
                Column::I64(ColumnData::new(children_col)),
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
        // A non-finite width would poison both the stroke and the fingerprint.
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            1.0
        };
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
        let tol = edge_pick_tolerance(self.width);
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        for (row, e) in self.edges.iter().enumerate() {
            let d = segment_distance(e.parent, e.child, [px, py]);
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

/// Clamp a user-supplied radius to a finite, non-negative pixel value.
///
/// A non-finite radius would poison bounds computation and the snap-radius
/// clamp; collapsing it to `0.0` keeps the node pickable (within slack) while
/// staying safe.
fn sanitize_radius(radius: f32) -> f32 {
    if radius.is_finite() {
        radius.max(0.0)
    } else {
        0.0
    }
}

/// Clamp each channel of a premultiplied RGBA color into `0.0..=1.0`.
///
/// Non-finite channels collapse to `0.0` so a stray `NaN`/`inf` cannot poison
/// the renderer's blend math or a downstream fingerprint.
fn sanitize_color(color: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (slot, channel) in out.iter_mut().zip(color) {
        *slot = if channel.is_finite() {
            channel.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    out
}

/// Hit-test tolerance for an edge of the given stroke width. Half the stroke
/// straddles the centerline, plus a few pixels of Fitts's-law slack so hairline
/// connectors stay clickable.
fn edge_pick_tolerance(width: f32) -> f32 {
    let half = if width.is_finite() {
        (width * 0.5).max(0.0)
    } else {
        0.0
    };
    half + 3.0
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

    #[test]
    fn handles_single_node() {
        let nodes = vec![TreeNode::root("solo", "Solo")];
        let layout = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        )
        .expect("single-node layout");
        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(layout.nodes[0].depth, 0);
        assert!(layout.edges.is_empty());
        assert!(layout.nodes[0].x.is_finite() && layout.nodes[0].y.is_finite());

        let chart = TreeSpec::new(nodes, vec![])
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 400),
            )
            .expect("single-node chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn empty_input_is_rejected() {
        let result = compute_layout(
            &[],
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::NoRoot)));
    }

    #[test]
    fn deep_chain_lays_out_iteratively() {
        // A chain this deep overflows the call stack under recursive slot
        // assignment on a default 2 MiB test-thread stack; the iterative
        // post-order walk handles it in O(n) heap space.
        const DEPTH: usize = 40_000;
        let mut nodes = Vec::with_capacity(DEPTH);
        nodes.push(TreeNode::root("n0", "n0"));
        for i in 1..DEPTH {
            nodes.push(TreeNode::child(
                format!("n{i}"),
                format!("n{i}"),
                format!("n{}", i - 1),
            ));
        }
        let layout = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        )
        .expect("deep chain layout");
        assert_eq!(layout.nodes.len(), DEPTH);
        assert_eq!(layout.nodes[DEPTH - 1].depth as usize, DEPTH - 1);
        // A single-leaf chain collapses onto one vertical slot.
        let x0 = layout.nodes[0].x;
        assert!((layout.nodes[DEPTH - 1].x - x0).abs() < 0.001);
    }

    #[test]
    fn forest_layout_is_opt_in() {
        let nodes = vec![
            TreeNode::root("a", "A"),
            TreeNode::child("a1", "A1", "a"),
            TreeNode::root("b", "B"),
            TreeNode::child("b1", "B1", "b"),
        ];

        // Default: a second root is an error.
        let rejected = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(rejected, Err(TreeError::MultipleRoots(_))));

        // Opt in: both roots lay out side by side without overlap.
        let options = TreeOptions::default().with_allow_forest(true);
        let layout = compute_layout(&nodes, &[], &options, Rect::new(0.0, 0.0, 400.0, 400.0))
            .expect("forest layout");
        assert_eq!(layout.nodes.len(), 4);
        assert_eq!(layout.nodes[0].depth, 0); // root a
        assert_eq!(layout.nodes[2].depth, 0); // root b
        assert_eq!(layout.edges.len(), 2);
        // The first root's slot band sits left of the second root's band.
        assert!(layout.nodes[0].x < layout.nodes[2].x);
    }

    #[test]
    fn rejects_disconnected_cycle_component() {
        // `r` is the only root; `a`/`b` form a parent cycle unreachable from it.
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("a", "A", "b"),
            TreeNode::child("b", "B", "a"),
        ];
        let result = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        assert!(matches!(result, Err(TreeError::Cycle(_))));
    }

    #[test]
    fn sanitizes_non_finite_radius_and_color() {
        let nodes = vec![
            TreeNode::root("r", "Root").with_radius(f32::NAN),
            TreeNode::child("c", "C", "r")
                .with_radius(f32::INFINITY)
                .with_color([f32::NAN, 0.5, -3.0, 2.0]),
        ];
        let layout = compute_layout(
            &nodes,
            &[],
            &TreeOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        )
        .expect("layout");

        // Non-finite radii collapse to a finite, non-negative value.
        assert_eq!(layout.nodes[0].radius, 0.0);
        assert_eq!(layout.nodes[1].radius, 0.0);
        // Color channels clamp into range; NaN → 0, out-of-range → clamped.
        assert_eq!(layout.nodes[1].color, [0.0, 0.5, 0.0, 1.0]);
        for node in &layout.nodes {
            assert!(node.radius.is_finite());
            assert!(node.color.iter().all(|c| (0.0..=1.0).contains(c)));
        }
        for edge in &layout.edges {
            assert!(edge.color.iter().all(|c| c.is_finite()));
        }
    }

    #[test]
    fn non_finite_padding_keeps_coords_finite() {
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("a", "A", "r"),
            TreeNode::child("b", "B", "r"),
        ];
        let options = TreeOptions::default().with_padding(f32::NAN);
        let layout = compute_layout(&nodes, &[], &options, Rect::new(0.0, 0.0, 400.0, 400.0))
            .expect("layout");
        for node in &layout.nodes {
            assert!(node.x.is_finite(), "x must be finite, got {}", node.x);
            assert!(node.y.is_finite(), "y must be finite, got {}", node.y);
        }
    }

    #[test]
    fn edge_pick_tolerance_scales_with_stroke_width() {
        let nodes = vec![TreeNode::root("r", "Root"), TreeNode::child("c", "C", "r")];
        let size = ChartSize::new(400, 300);

        let thick = TreeSpec::new(nodes.clone(), vec![])
            .with_options(TreeOptions::default().with_edge_width(10.0));
        let layout = thick.layout(size).expect("layout");
        // Single child => root and child share x; the connector is vertical.
        let x = layout.nodes[0].x;
        let mid_y = (layout.nodes[0].y + layout.nodes[1].y) * 0.5;
        let probe = (x + 6.0, mid_y); // 6 px off the centerline, far from both nodes

        let thick_chart = thick
            .try_build_chart(berthacharts_core::Workspace::new(), size)
            .expect("chart");
        let hit = thick_chart
            .pick(probe)
            .expect("a 10 px connector should be hit 6 px off-center");
        assert_eq!(hit.mark, EDGE_MARK);

        // The identical probe misses a hairline connector (tol ~3.6 px < 6 px).
        let thin =
            TreeSpec::new(nodes, vec![]).with_options(TreeOptions::default().with_edge_width(1.2));
        let thin_chart = thin
            .try_build_chart(berthacharts_core::Workspace::new(), size)
            .expect("chart");
        assert!(
            thin_chart.pick(probe).is_none(),
            "a hairline connector should miss 6 px off-center"
        );
    }

    #[test]
    fn node_dataset_reports_child_counts() {
        let nodes = vec![
            TreeNode::root("r", "Root"),
            TreeNode::child("a", "A", "r"),
            TreeNode::child("b", "B", "r"),
            TreeNode::child("a1", "A1", "a"),
        ];
        let workspace = berthacharts_core::Workspace::new();
        TreeSpec::new(nodes, vec![])
            .try_build_chart(workspace.clone(), ChartSize::new(400, 400))
            .expect("chart");

        let dataset = workspace.dataset(NODE_DATASET).expect("node dataset");
        let column = dataset.column("children").expect("children column");
        let Column::I64(counts) = column.as_ref() else {
            panic!("children column should be i64");
        };
        // Node order is preserved: root has 2 children, `a` has 1, the rest leaf.
        assert_eq!(counts.values, vec![2, 1, 0, 0]);
    }
}
