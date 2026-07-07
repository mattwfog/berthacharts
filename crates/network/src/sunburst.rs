//! Sunburst chart spec and annular sector mark.

use std::f32::consts::TAU;
use std::fmt;
use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use berthacharts_core::{
    CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset, DatasetId,
    Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind, LabelPriority,
    LabelTooltip, LabelTooltipRow, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem, LinePrim,
    LinearScale, Mark, MarkId, PickCtx, PickHit, Rect, Scale, ScaleId, Scene, SnapKind, SnapTarget,
    SnapTargetSet, TessellateCtx, TooltipField, TooltipGuide, TrianglePrim, Workspace,
};

const SECTOR_DATASET: DatasetId = DatasetId::new(0);
const SECTOR_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);
const MIN_SWEEP: f32 = 0.0001;

/// A hierarchical node supplied by users.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstNode {
    /// Stable node id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Parent node id. `None` marks the root.
    pub parent: Option<String>,
    /// Node value. Leaf values drive angular size; parent values are used when
    /// larger than their child sum.
    pub value: f32,
    /// Display/legend group.
    pub group: String,
    /// Premultiplied sector color.
    pub color: [f32; 4],
    /// Optional sort key within siblings. Lower renders earlier clockwise.
    pub order: Option<i32>,
}

impl SunburstNode {
    /// Build a sunburst node.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        parent: Option<impl Into<String>>,
        value: f32,
        group: impl Into<String>,
        color: [f32; 4],
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            parent: parent.map(Into::into),
            value,
            group: group.into(),
            color,
            order: None,
        }
    }

    /// Set sibling-local sort order.
    #[must_use]
    pub const fn with_order(mut self, order: i32) -> Self {
        self.order = Some(order);
        self
    }
}

/// Minimal path input for users who do not want to declare parent ids.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstPath {
    /// Ordered hierarchy segments from root to leaf.
    pub segments: Vec<String>,
    /// Leaf magnitude.
    pub value: f32,
    /// Display/legend group for the leaf branch.
    pub group: String,
    /// Optional premultiplied sector color. Defaults from the group palette.
    pub color: Option<[f32; 4]>,
}

impl SunburstPath {
    /// Build a minimal sunburst path.
    #[must_use]
    pub fn new(segments: Vec<impl Into<String>>, value: f32, group: impl Into<String>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
            value,
            group: group.into(),
            color: None,
        }
    }

    /// Set a premultiplied color for this path.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = Some(color);
        self
    }
}

/// Legend item for a sunburst group.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstLegendItem {
    /// Display label.
    pub label: String,
    /// Swatch color.
    pub color: [f32; 4],
}

impl SunburstLegendItem {
    /// Build a legend item.
    #[must_use]
    pub fn new(label: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            label: label.into(),
            color,
        }
    }
}

/// Layout and guide options for a sunburst chart.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstOptions {
    /// Outer padding in CSS pixels.
    pub padding: f32,
    /// Radius of the central root disk.
    pub inner_radius: f32,
    /// Gap between rings in CSS pixels.
    pub ring_gap: f32,
    /// Angular gap between sibling sectors in radians.
    pub angular_gap: f32,
    /// Separator stroke color.
    pub separator_color: [f32; 4],
    /// Separator stroke width in CSS pixels.
    pub separator_width: f32,
    /// Sort siblings by descending computed value when no explicit order is set.
    pub sort_siblings_by_value: bool,
    /// Start angle in radians. `-TAU / 4` starts at 12 o'clock.
    pub start_angle: f32,
    /// Total angular span in radians.
    pub sweep_angle: f32,
    /// Minimum angular span for direct labels.
    pub min_label_sweep: f32,
    /// Minimum sector area proxy for direct labels.
    pub min_label_area: f32,
    /// Maximum visible overlay labels.
    pub max_visible_labels: usize,
}

impl Default for SunburstOptions {
    fn default() -> Self {
        Self {
            padding: 34.0,
            inner_radius: 54.0,
            ring_gap: 2.0,
            angular_gap: 0.006,
            separator_color: rgba(1.0, 1.0, 1.0, 0.72),
            separator_width: 1.0,
            sort_siblings_by_value: true,
            start_angle: -TAU * 0.25,
            sweep_angle: TAU,
            min_label_sweep: 0.18,
            min_label_area: 820.0,
            max_visible_labels: 24,
        }
    }
}

impl SunburstOptions {
    /// Set the outer padding in CSS pixels.
    #[must_use]
    pub const fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set the radius of the central root disk.
    #[must_use]
    pub const fn with_inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the gap between rings in CSS pixels.
    #[must_use]
    pub const fn with_ring_gap(mut self, ring_gap: f32) -> Self {
        self.ring_gap = ring_gap;
        self
    }

    /// Set the angular gap between sibling sectors in radians.
    #[must_use]
    pub const fn with_angular_gap(mut self, angular_gap: f32) -> Self {
        self.angular_gap = angular_gap;
        self
    }

    /// Set the start angle in radians.
    #[must_use]
    pub const fn with_start_angle(mut self, start_angle: f32) -> Self {
        self.start_angle = start_angle;
        self
    }

    /// Set the total angular span in radians.
    #[must_use]
    pub const fn with_sweep_angle(mut self, sweep_angle: f32) -> Self {
        self.sweep_angle = sweep_angle;
        self
    }

    /// Set direct-label visibility thresholds.
    #[must_use]
    pub const fn with_label_thresholds(
        mut self,
        min_label_sweep: f32,
        min_label_area: f32,
    ) -> Self {
        self.min_label_sweep = min_label_sweep;
        self.min_label_area = min_label_area;
        self
    }

    /// Set the maximum number of visible overlay labels.
    #[must_use]
    pub const fn with_max_visible_labels(mut self, max_visible_labels: usize) -> Self {
        self.max_visible_labels = max_visible_labels;
        self
    }

    /// Set separator stroke styling.
    #[must_use]
    pub const fn with_separator(mut self, color: [f32; 4], width: f32) -> Self {
        self.separator_color = color;
        self.separator_width = width;
        self
    }

    /// Control whether unordered siblings sort by descending computed value.
    #[must_use]
    pub const fn with_sort_siblings_by_value(mut self, sort_siblings_by_value: bool) -> Self {
        self.sort_siblings_by_value = sort_siblings_by_value;
        self
    }
}

/// Reusable sunburst chart specification.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstSpec {
    /// Nodes in author order.
    pub nodes: Vec<SunburstNode>,
    /// Optional explicit legend items. When empty, items are inferred from
    /// node groups in first-seen order.
    pub legend: Vec<SunburstLegendItem>,
    /// Layout and guide options.
    pub options: SunburstOptions,
}

impl SunburstSpec {
    /// Build a sunburst spec from hierarchical nodes.
    #[must_use]
    pub fn new(nodes: Vec<SunburstNode>) -> Self {
        Self {
            nodes,
            legend: Vec::new(),
            options: SunburstOptions::default(),
        }
    }

    /// Build a sunburst spec from root-to-leaf paths.
    #[must_use]
    pub fn from_paths(paths: Vec<SunburstPath>) -> Self {
        let mut nodes = Vec::<SunburstNode>::new();
        let mut seen = AHashSet::<String>::new();
        let mut group_colors = AHashMap::<String, [f32; 4]>::new();

        for path in paths {
            if path.segments.is_empty() {
                continue;
            }
            let next_color_index = group_colors.len();
            let color = *group_colors.entry(path.group.clone()).or_insert_with(|| {
                path.color
                    .unwrap_or_else(|| group_palette(next_color_index))
            });
            let mut parent: Option<String> = None;
            let mut id = String::new();
            for (depth, segment) in path.segments.iter().enumerate() {
                if depth > 0 {
                    id.push('/');
                }
                id.push_str(&slug(segment));
                if seen.insert(id.clone()) {
                    nodes.push(SunburstNode::new(
                        id.clone(),
                        segment.clone(),
                        parent.clone(),
                        0.0,
                        if depth == 0 { "root" } else { &path.group },
                        if depth == 0 {
                            rgba(0.16, 0.20, 0.28, 1.0)
                        } else {
                            color
                        },
                    ));
                }
                parent = Some(id.clone());
            }
            if let Some(last) = nodes
                .iter_mut()
                .find(|node| Some(&node.id) == parent.as_ref())
            {
                last.value += path.value;
            }
        }

        Self::new(nodes)
    }

    /// Set explicit legend items.
    #[must_use]
    pub fn with_legend(mut self, legend: Vec<SunburstLegendItem>) -> Self {
        self.legend = legend;
        self
    }

    /// Set layout and guide options.
    #[must_use]
    pub const fn with_options(mut self, options: SunburstOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute reusable layout without building a chart.
    pub fn layout(&self, size: ChartSize) -> Result<SunburstLayout, SunburstError> {
        validate_nodes(&self.nodes)?;
        validate_options(&self.options)?;

        let root = root_index(&self.nodes)?;
        let mut children = child_index(&self.nodes)?;
        validate_acyclic(root, &children, self.nodes.len())?;

        let mut totals = vec![0.0; self.nodes.len()];
        let root_total = compute_total(root, &children, &self.nodes, &mut totals);
        if root_total <= 0.0 {
            return Err(SunburstError::NonPositiveTotal);
        }
        sort_children(
            &mut children,
            &self.nodes,
            &totals,
            self.options.sort_siblings_by_value,
        );

        let mut depths = vec![0usize; self.nodes.len()];
        assign_depths(root, 0, &children, &mut depths);
        let max_depth = depths.iter().copied().max().unwrap_or(0);
        let center = (size.width as f32 * 0.5, size.height as f32 * 0.5);
        let outer_radius = (size.width.min(size.height) as f32 * 0.5 - self.options.padding)
            .max(self.options.inner_radius + 12.0);
        let ring_step = if max_depth == 0 {
            0.0
        } else {
            ((outer_radius - self.options.inner_radius) / max_depth as f32).max(8.0)
        };

        let mut sectors = Vec::with_capacity(self.nodes.len());
        let mut paths = vec![String::new(); self.nodes.len()];
        layout_node(
            LayoutFrame {
                index: root,
                parent: None,
                branch: None,
                sibling_index: 0,
                sibling_count: 1,
                start_angle: self.options.start_angle,
                sweep: self.options.sweep_angle,
            },
            &LayoutInputs {
                nodes: &self.nodes,
                children: &children,
                totals: &totals,
                depths: &depths,
                center,
                max_depth,
                root_total,
                ring_step,
                options: &self.options,
            },
            &mut paths,
            &mut sectors,
        );

        Ok(SunburstLayout {
            sectors,
            root_total,
            max_depth,
            center,
            outer_radius,
            size,
        })
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, SunburstError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    /// Compute headline statistics for this hierarchy.
    pub fn summary(&self) -> Result<SunburstSummary, SunburstError> {
        validate_nodes(&self.nodes)?;
        let root = root_index(&self.nodes)?;
        let mut children = child_index(&self.nodes)?;
        validate_acyclic(root, &children, self.nodes.len())?;

        let mut totals = vec![0.0; self.nodes.len()];
        let root_total = compute_total(root, &children, &self.nodes, &mut totals);
        if root_total <= 0.0 {
            return Err(SunburstError::NonPositiveTotal);
        }
        sort_children(
            &mut children,
            &self.nodes,
            &totals,
            self.options.sort_siblings_by_value,
        );

        let mut depths = vec![0usize; self.nodes.len()];
        assign_depths(root, 0, &children, &mut depths);
        let leaves = (0..self.nodes.len())
            .filter(|index| children[*index].is_empty())
            .collect::<Vec<_>>();
        let mut ranked_leaves = leaves.clone();
        ranked_leaves.sort_by(|left, right| {
            totals[*right]
                .partial_cmp(&totals[*left])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.cmp(right))
        });
        let largest_leaf = ranked_leaves.first().map(|index| &self.nodes[*index]);
        let largest_leaf_value = ranked_leaves.first().map_or(0.0, |index| totals[*index]);
        let top_three_total = ranked_leaves
            .iter()
            .take(3)
            .map(|index| totals[*index])
            .sum::<f32>();

        Ok(SunburstSummary {
            total: root_total,
            nodes: self.nodes.len(),
            leaves: leaves.len(),
            max_depth: depths.iter().copied().max().unwrap_or(0),
            branches: branch_summaries(root, &children, &self.nodes, &totals, root_total),
            largest_leaf_label: largest_leaf.map_or_else(String::new, |node| node.label.clone()),
            largest_leaf_path: ranked_leaves
                .first()
                .map_or_else(String::new, |index| node_path(*index, &self.nodes)),
            largest_leaf_value,
            top_three_leaf_share: top_three_total / root_total * 100.0,
        })
    }

    fn labels(&self, layout: &SunburstLayout) -> Vec<LabelItem> {
        let mut labels = Vec::new();
        for sector in layout
            .sectors
            .iter()
            .filter(|sector| sector.has_visible_area())
        {
            if sector.depth == 0 {
                labels.push(
                    LabelItem::new(layout.center.0, layout.center.1, &sector.label)
                        .with_detail(format!("{:.0}", sector.value))
                        .with_kind(LabelKind::Node)
                        .with_priority(LabelPriority::Required)
                        .with_anchor(LabelAnchor::Center)
                        .with_reposition(false)
                        .with_tooltip(sector_label_tooltip(sector)),
                );
                continue;
            }

            if sector.depth == 1 {
                let (x, y) = sector.label_point(0.58);
                labels.push(
                    LabelItem::new(x, y, sector.label.to_ascii_lowercase())
                        .with_detail(format!("{:.0}%", sector.share_total))
                        .with_kind(LabelKind::Column)
                        .with_priority(LabelPriority::Required)
                        .with_anchor(LabelAnchor::Center)
                        .with_reposition(true)
                        .with_tooltip(sector_label_tooltip(sector)),
                );
                continue;
            }

            if should_direct_label_sector(sector, &self.options) {
                let (x, y) = sector.label_point(0.54);
                labels.push(
                    LabelItem::new(x, y, &sector.label)
                        .with_detail(format!("{:.0} / {:.0}%", sector.value, sector.share_total))
                        .with_kind(if sector.children == 0 {
                            LabelKind::Data
                        } else {
                            LabelKind::Node
                        })
                        .with_priority(if sector.children == 0 {
                            LabelPriority::Important
                        } else {
                            LabelPriority::Optional
                        })
                        .with_anchor(LabelAnchor::Center)
                        .with_tooltip(sector_label_tooltip(sector)),
                );
            }
        }
        labels
    }

    fn snap_targets(&self, layout: &SunburstLayout) -> Vec<SnapTarget> {
        layout
            .sectors
            .iter()
            .filter(|sector| sector.has_visible_area())
            .map(|sector| {
                let (x, y) = if sector.depth == 0 {
                    layout.center
                } else {
                    sector.label_point(0.55)
                };
                SnapTarget::new(
                    x,
                    y,
                    if sector.children == 0 {
                        SnapKind::Point
                    } else {
                        SnapKind::Node
                    },
                )
                .with_radius(
                    sector
                        .radial_thickness()
                        .mul_add(0.12, 5.0)
                        .clamp(5.0, 14.0),
                )
                .with_label(format!("{} sector", sector.label))
                .with_priority(if sector.depth <= 1 { 3 } else { 1 })
            })
            .collect()
    }

    fn legend_items(&self) -> Vec<LegendItem> {
        if !self.legend.is_empty() {
            return self
                .legend
                .iter()
                .map(|item| LegendItem::new(&item.label, item.color))
                .collect();
        }

        let mut seen = AHashSet::new();
        self.nodes
            .iter()
            .filter(|node| node.parent.is_some())
            .filter_map(|node| {
                if seen.insert(node.group.clone()) {
                    Some(LegendItem::new(&node.group, node.color))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl ChartSpec for SunburstSpec {
    type Error = SunburstError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        let layout = self.layout(size)?;

        let x_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (0.0, size.width as f64),
            (0.0, size.width as f32),
        ));
        let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (0.0, size.height as f64),
            (0.0, size.height as f32),
        ));
        workspace.upsert_scale(X_SCALE, x_scale);
        workspace.upsert_scale(Y_SCALE, y_scale);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(sector_dataset(&layout.sectors));

        let labels = self.labels(&layout);
        let mut scene = Scene::new(size.full_viewport());
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![Arc::new(SunburstSectorMark::new(
                SECTOR_MARK,
                layout.sectors.clone(),
                self.options.separator_color,
                self.options.separator_width,
            )) as Arc<dyn Mark>],
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });

        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                SECTOR_MARK,
                SECTOR_DATASET,
                vec![
                    TooltipField::new("Path", "path").as_label(),
                    TooltipField::new("Branch", "branch").as_label(),
                    TooltipField::new("Value", "value").as_integer(),
                    TooltipField::new("Share of total", "share_total").as_percent(1),
                    TooltipField::new("Branch share", "share_branch").as_percent(1),
                    TooltipField::new("Parent share", "share_parent").as_percent(1),
                    TooltipField::new("Depth", "depth").as_integer(),
                    TooltipField::new("Children", "children").as_integer(),
                    TooltipField::new("Leaves", "leaves").as_integer(),
                    TooltipField::new("Sibling rank", "sibling_rank").as_integer(),
                    TooltipField::new("Sibling count", "sibling_count").as_integer(),
                    TooltipField::new("Group", "group").as_label(),
                ],
            )
            .with_title_column("label"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(3.0)
                .with_max_visible(self.options.max_visible_labels),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(self.legend_items())
                .with_title("Sector group")
                .with_anchor(LegendAnchor::Bottom),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(self.snap_targets(&layout)).with_name("sunburst sectors"),
        ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

/// Computed sunburst layout.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstLayout {
    /// Laid-out sectors in render order.
    pub sectors: Vec<SunburstSector>,
    /// Root total.
    pub root_total: f32,
    /// Deepest hierarchy depth.
    pub max_depth: usize,
    /// Chart center.
    pub center: (f32, f32),
    /// Outer radius.
    pub outer_radius: f32,
    /// Size used for layout.
    pub size: ChartSize,
}

/// Headline statistics for a sunburst hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstSummary {
    /// Root total.
    pub total: f32,
    /// Number of nodes, including the root.
    pub nodes: usize,
    /// Number of leaf sectors.
    pub leaves: usize,
    /// Deepest hierarchy depth.
    pub max_depth: usize,
    /// First-ring branch summaries in layout order.
    pub branches: Vec<SunburstBranchSummary>,
    /// Label of the largest leaf.
    pub largest_leaf_label: String,
    /// Full path of the largest leaf.
    pub largest_leaf_path: String,
    /// Value of the largest leaf.
    pub largest_leaf_value: f32,
    /// Percent of total held by the three largest leaves.
    pub top_three_leaf_share: f32,
}

/// Summary statistics for a first-ring branch.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstBranchSummary {
    /// Branch label.
    pub label: String,
    /// Branch value.
    pub value: f32,
    /// Percent share of root total.
    pub share_total: f32,
    /// Number of leaf descendants.
    pub leaves: usize,
    /// Label of the largest leaf within this branch.
    pub largest_leaf_label: String,
    /// Value of the largest leaf within this branch.
    pub largest_leaf_value: f32,
    /// Percent share of branch held by the largest leaf.
    pub largest_leaf_share: f32,
}

/// Laid-out sunburst sector.
#[derive(Debug, Clone, PartialEq)]
pub struct SunburstSector {
    /// Source input index.
    pub source_index: usize,
    /// Stable id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Full root-to-node path.
    pub path: String,
    /// First-ring branch label.
    pub branch: String,
    /// Display/legend group.
    pub group: String,
    /// Hierarchy depth, with root at 0.
    pub depth: usize,
    /// Parent source index.
    pub parent: Option<usize>,
    /// Computed value.
    pub value: f32,
    /// Percent share of root total, in percentage points.
    pub share_total: f32,
    /// Percent share of parent total, in percentage points.
    pub share_parent: f32,
    /// Percent share of first-ring branch, in percentage points.
    pub share_branch: f32,
    /// Number of direct children.
    pub children: usize,
    /// Number of leaf descendants, including self for leaves.
    pub leaves: usize,
    /// Zero-based sibling position after layout sorting.
    pub sibling_index: usize,
    /// Number of siblings including this sector.
    pub sibling_count: usize,
    /// Center x.
    pub cx: f32,
    /// Center y.
    pub cy: f32,
    /// Inner radius.
    pub inner_radius: f32,
    /// Outer radius.
    pub outer_radius: f32,
    /// Start angle in radians.
    pub start_angle: f32,
    /// End angle in radians.
    pub end_angle: f32,
    /// Premultiplied sector color.
    pub color: [f32; 4],
}

impl SunburstSector {
    /// Angular span in radians.
    #[must_use]
    pub fn sweep(&self) -> f32 {
        self.end_angle - self.start_angle
    }

    /// Ring thickness in pixels.
    #[must_use]
    pub fn radial_thickness(&self) -> f32 {
        self.outer_radius - self.inner_radius
    }

    /// Label point inside the sector at a radial fraction.
    #[must_use]
    pub fn label_point(&self, radius_fraction: f32) -> (f32, f32) {
        let angle = (self.start_angle + self.end_angle) * 0.5;
        let radius =
            self.inner_radius + self.radial_thickness().max(0.0) * radius_fraction.clamp(0.0, 1.0);
        (
            self.cx + radius * angle.cos(),
            self.cy + radius * angle.sin(),
        )
    }

    fn area_proxy(&self) -> f32 {
        self.sweep().abs() * self.radial_thickness().max(0.0) * self.outer_radius.max(1.0)
    }

    fn has_visible_area(&self) -> bool {
        self.value > 0.0
            && self.sweep().abs() >= MIN_SWEEP
            && self.radial_thickness() > 0.0
            && self.start_angle.is_finite()
            && self.end_angle.is_finite()
    }
}

/// Error building a sunburst chart.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SunburstError {
    /// No nodes were supplied.
    EmptyNodes,
    /// A node id was empty.
    EmptyNodeId,
    /// A node id appeared more than once.
    DuplicateNode {
        /// Duplicate id.
        id: String,
    },
    /// More or fewer than one root was supplied.
    InvalidRootCount {
        /// Number of roots found.
        count: usize,
    },
    /// A node references an unknown parent.
    MissingParent {
        /// Missing parent id.
        id: String,
    },
    /// A node has a non-finite or negative value.
    InvalidValue {
        /// Node id.
        id: String,
        /// Bad value.
        value: f32,
    },
    /// A layout option is non-finite or outside the supported range.
    InvalidOption {
        /// Option field name.
        name: String,
        /// Bad value.
        value: f32,
    },
    /// The hierarchy contains a cycle or disconnected node.
    InvalidHierarchy,
    /// The computed root total was not positive.
    NonPositiveTotal,
}

impl fmt::Display for SunburstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNodes => write!(f, "sunburst requires at least one node"),
            Self::EmptyNodeId => write!(f, "sunburst node id cannot be empty"),
            Self::DuplicateNode { id } => write!(f, "duplicate sunburst node id `{id}`"),
            Self::InvalidRootCount { count } => {
                write!(f, "sunburst requires exactly one root, found {count}")
            }
            Self::MissingParent { id } => write!(f, "missing sunburst parent `{id}`"),
            Self::InvalidValue { id, value } => {
                write!(f, "sunburst node `{id}` has invalid value {value}")
            }
            Self::InvalidOption { name, value } => {
                write!(f, "sunburst option `{name}` has invalid value {value}")
            }
            Self::InvalidHierarchy => write!(f, "sunburst hierarchy is cyclic or disconnected"),
            Self::NonPositiveTotal => write!(f, "sunburst total must be positive"),
        }
    }
}

impl std::error::Error for SunburstError {}

#[derive(Debug, Clone)]
struct SunburstSectorMark {
    id: MarkId,
    sectors: Vec<SunburstSector>,
    separator_color: [f32; 4],
    separator_width: f32,
}

impl SunburstSectorMark {
    fn new(
        id: MarkId,
        sectors: Vec<SunburstSector>,
        separator_color: [f32; 4],
        separator_width: f32,
    ) -> Self {
        Self {
            id,
            sectors,
            separator_color,
            separator_width,
        }
    }
}

impl Mark for SunburstSectorMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        mix_u64(&mut h, self.id.get());
        mix_u64(&mut h, self.sectors.len() as u64);
        for value in self.separator_color {
            mix_u64(&mut h, value.to_bits() as u64);
        }
        mix_u64(&mut h, self.separator_width.to_bits() as u64);
        for sector in &self.sectors {
            mix_u64(&mut h, sector.source_index as u64);
            mix_u64(&mut h, sector.inner_radius.to_bits() as u64);
            mix_u64(&mut h, sector.outer_radius.to_bits() as u64);
            mix_u64(&mut h, sector.start_angle.to_bits() as u64);
            mix_u64(&mut h, sector.end_angle.to_bits() as u64);
            for value in sector.color {
                mix_u64(&mut h, value.to_bits() as u64);
            }
        }
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut triangles = Vec::new();
        let mut separators = Vec::new();
        for sector in &self.sectors {
            push_sector(sector, &mut triangles);
            push_sector_separator(
                sector,
                self.separator_color,
                self.separator_width,
                &mut separators,
            );
        }
        Geometry::Mixed(vec![
            Geometry::Triangles(triangles),
            Geometry::Lines(separators),
        ])
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        for (row, sector) in self.sectors.iter().enumerate().rev() {
            if sector_contains(sector, point) {
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
        Rect::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct LayoutInputs<'a> {
    nodes: &'a [SunburstNode],
    children: &'a [Vec<usize>],
    totals: &'a [f32],
    depths: &'a [usize],
    center: (f32, f32),
    max_depth: usize,
    root_total: f32,
    ring_step: f32,
    options: &'a SunburstOptions,
}

#[derive(Debug, Clone, Copy)]
struct LayoutFrame {
    index: usize,
    parent: Option<usize>,
    branch: Option<usize>,
    sibling_index: usize,
    sibling_count: usize,
    start_angle: f32,
    sweep: f32,
}

fn layout_node(
    frame: LayoutFrame,
    input: &LayoutInputs<'_>,
    paths: &mut [String],
    sectors: &mut Vec<SunburstSector>,
) {
    let LayoutFrame {
        index,
        parent,
        branch,
        sibling_index,
        sibling_count,
        start_angle,
        sweep,
    } = frame;
    let node = &input.nodes[index];
    let depth = input.depths[index];
    let (inner_radius, outer_radius) =
        radii(depth, input.max_depth, input.ring_step, input.options);
    let (display_start, display_sweep) =
        display_angle_span(depth, start_angle, sweep, input.options);
    let parent_total = parent.map_or(input.root_total, |parent| input.totals[parent]);
    let branch = parent.and(branch);
    let branch_total = branch.map_or(input.root_total, |branch| input.totals[branch]);
    let branch_label = branch.map_or_else(
        || node.label.clone(),
        |branch| input.nodes[branch].label.clone(),
    );
    let path = parent.map_or_else(
        || node.label.clone(),
        |parent| format!("{} / {}", paths[parent], node.label),
    );
    paths[index] = path.clone();

    let end_angle = if input.totals[index] > 0.0 && display_sweep > 0.0 {
        display_start + display_sweep.max(MIN_SWEEP)
    } else {
        display_start
    };

    sectors.push(SunburstSector {
        source_index: index,
        id: node.id.clone(),
        label: node.label.clone(),
        path,
        branch: branch_label,
        group: node.group.clone(),
        depth,
        parent,
        value: input.totals[index],
        share_total: input.totals[index] / input.root_total * 100.0,
        share_parent: if parent_total > 0.0 {
            input.totals[index] / parent_total * 100.0
        } else {
            100.0
        },
        share_branch: if branch_total > 0.0 {
            input.totals[index] / branch_total * 100.0
        } else {
            100.0
        },
        children: input.children[index].len(),
        leaves: leaf_count(index, input.children),
        sibling_index,
        sibling_count,
        cx: input.center.0,
        cy: input.center.1,
        inner_radius,
        outer_radius,
        start_angle: display_start,
        end_angle,
        color: node.color,
    });

    let mut cursor = start_angle;
    let child_count = input.children[index].len();
    for (child_index, child) in input.children[index].iter().enumerate() {
        let child_sweep = if input.totals[index] > 0.0 {
            sweep * input.totals[*child] / input.totals[index]
        } else {
            0.0
        };
        layout_node(
            LayoutFrame {
                index: *child,
                parent: Some(index),
                branch: if depth == 0 { Some(*child) } else { branch },
                sibling_index: child_index,
                sibling_count: child_count,
                start_angle: cursor,
                sweep: child_sweep,
            },
            input,
            paths,
            sectors,
        );
        cursor += child_sweep;
    }
}

fn display_angle_span(
    depth: usize,
    start_angle: f32,
    sweep: f32,
    options: &SunburstOptions,
) -> (f32, f32) {
    if depth == 0 || sweep <= options.angular_gap * 1.5 {
        return (start_angle, sweep);
    }
    let gap = options.angular_gap.min(sweep * 0.22);
    (start_angle + gap * 0.5, (sweep - gap).max(MIN_SWEEP))
}

fn radii(depth: usize, max_depth: usize, ring_step: f32, options: &SunburstOptions) -> (f32, f32) {
    if depth == 0 {
        return (0.0, options.inner_radius);
    }
    if max_depth == 0 {
        return (0.0, options.inner_radius);
    }
    let inner = options.inner_radius + (depth - 1) as f32 * ring_step + options.ring_gap * 0.5;
    let outer = options.inner_radius + depth as f32 * ring_step - options.ring_gap * 0.5;
    (inner.max(0.0), outer.max(inner + 1.0))
}

fn should_direct_label_sector(sector: &SunburstSector, options: &SunburstOptions) -> bool {
    sector.sweep().abs() >= options.min_label_sweep && sector.area_proxy() >= options.min_label_area
}

fn push_sector(sector: &SunburstSector, out: &mut Vec<TrianglePrim>) {
    let sweep = sector.sweep().abs();
    if sweep < MIN_SWEEP {
        return;
    }
    let segments = ((sweep / TAU) * 112.0).ceil().clamp(3.0, 96.0) as usize;
    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        let a0 = sector.start_angle + sector.sweep() * t0;
        let a1 = sector.start_angle + sector.sweep() * t1;
        let o0 = polar(sector, sector.outer_radius, a0);
        let o1 = polar(sector, sector.outer_radius, a1);
        if sector.inner_radius <= 0.5 {
            out.push(TrianglePrim {
                a: [sector.cx, sector.cy],
                b: o0,
                c: o1,
                fill: sector.color,
            });
        } else {
            let i0 = polar(sector, sector.inner_radius, a0);
            let i1 = polar(sector, sector.inner_radius, a1);
            out.push(TrianglePrim {
                a: i0,
                b: o0,
                c: o1,
                fill: sector.color,
            });
            out.push(TrianglePrim {
                a: i0,
                b: o1,
                c: i1,
                fill: sector.color,
            });
        }
    }
}

fn push_sector_separator(
    sector: &SunburstSector,
    stroke: [f32; 4],
    width: f32,
    out: &mut Vec<LinePrim>,
) {
    if width <= 0.0 || stroke[3] <= 0.0 {
        return;
    }
    let sweep = sector.sweep().abs();
    if sweep < MIN_SWEEP {
        return;
    }
    let segments = ((sweep / TAU) * 96.0).ceil().clamp(4.0, 72.0) as usize;
    out.push(LinePrim {
        points: (0..=segments)
            .map(|i| {
                let t = i as f32 / segments as f32;
                let angle = sector.start_angle + sector.sweep() * t;
                polar(sector, sector.outer_radius, angle)
            })
            .collect(),
        stroke,
        width,
        dash: None,
        join: 1,
        cap: 1,
    });
    if sector.inner_radius > 0.5 {
        for angle in [sector.start_angle, sector.end_angle] {
            out.push(LinePrim {
                points: vec![
                    polar(sector, sector.inner_radius, angle),
                    polar(sector, sector.outer_radius, angle),
                ],
                stroke,
                width,
                dash: None,
                join: 1,
                cap: 1,
            });
        }
    }
}

fn polar(sector: &SunburstSector, radius: f32, angle: f32) -> [f32; 2] {
    [
        sector.cx + radius * angle.cos(),
        sector.cy + radius * angle.sin(),
    ]
}

fn sector_contains(sector: &SunburstSector, point: (f32, f32)) -> bool {
    if !sector.has_visible_area() || !point.0.is_finite() || !point.1.is_finite() {
        return false;
    }
    let dx = point.0 - sector.cx;
    let dy = point.1 - sector.cy;
    let radius = dx.hypot(dy);
    if radius < sector.inner_radius || radius > sector.outer_radius {
        return false;
    }

    let mut angle = dy.atan2(dx);
    while angle < sector.start_angle {
        angle += TAU;
    }
    while angle > sector.start_angle + TAU {
        angle -= TAU;
    }
    angle <= sector.end_angle + 0.002
}

fn validate_options(options: &SunburstOptions) -> Result<(), SunburstError> {
    validate_nonnegative_option("padding", options.padding)?;
    validate_nonnegative_option("inner_radius", options.inner_radius)?;
    validate_nonnegative_option("ring_gap", options.ring_gap)?;
    validate_nonnegative_option("angular_gap", options.angular_gap)?;
    validate_nonnegative_option("separator_width", options.separator_width)?;
    validate_nonnegative_option("min_label_sweep", options.min_label_sweep)?;
    validate_nonnegative_option("min_label_area", options.min_label_area)?;
    validate_finite_option("start_angle", options.start_angle)?;
    if !options.sweep_angle.is_finite() || options.sweep_angle <= 0.0 || options.sweep_angle > TAU {
        return Err(SunburstError::InvalidOption {
            name: "sweep_angle".into(),
            value: options.sweep_angle,
        });
    }
    for (index, value) in options.separator_color.iter().copied().enumerate() {
        validate_finite_option(format!("separator_color[{index}]"), value)?;
    }
    Ok(())
}

fn validate_nonnegative_option(name: impl Into<String>, value: f32) -> Result<(), SunburstError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(SunburstError::InvalidOption {
            name: name.into(),
            value,
        })
    }
}

fn validate_finite_option(name: impl Into<String>, value: f32) -> Result<(), SunburstError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SunburstError::InvalidOption {
            name: name.into(),
            value,
        })
    }
}

fn validate_nodes(nodes: &[SunburstNode]) -> Result<(), SunburstError> {
    if nodes.is_empty() {
        return Err(SunburstError::EmptyNodes);
    }
    let mut seen = AHashSet::new();
    for node in nodes {
        if node.id.trim().is_empty() {
            return Err(SunburstError::EmptyNodeId);
        }
        if !seen.insert(node.id.clone()) {
            return Err(SunburstError::DuplicateNode {
                id: node.id.clone(),
            });
        }
        if !node.value.is_finite() || node.value < 0.0 {
            return Err(SunburstError::InvalidValue {
                id: node.id.clone(),
                value: node.value,
            });
        }
    }
    Ok(())
}

fn root_index(nodes: &[SunburstNode]) -> Result<usize, SunburstError> {
    let roots: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.parent.is_none().then_some(index))
        .collect();
    if roots.len() == 1 {
        Ok(roots[0])
    } else {
        Err(SunburstError::InvalidRootCount { count: roots.len() })
    }
}

fn child_index(nodes: &[SunburstNode]) -> Result<Vec<Vec<usize>>, SunburstError> {
    let lookup: AHashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    let mut children = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let Some(parent) = &node.parent else {
            continue;
        };
        let Some(parent_index) = lookup.get(parent.as_str()).copied() else {
            return Err(SunburstError::MissingParent { id: parent.clone() });
        };
        children[parent_index].push(index);
    }
    for child_set in &mut children {
        child_set.sort_by_key(|index| nodes[*index].order.unwrap_or(i32::MAX));
    }
    Ok(children)
}

fn sort_children(
    children: &mut [Vec<usize>],
    nodes: &[SunburstNode],
    totals: &[f32],
    sort_siblings_by_value: bool,
) {
    for child_set in children {
        child_set.sort_by(|left, right| {
            let left_order = nodes[*left].order.unwrap_or(i32::MAX);
            let right_order = nodes[*right].order.unwrap_or(i32::MAX);
            left_order.cmp(&right_order).then_with(|| {
                if sort_siblings_by_value {
                    totals[*right]
                        .partial_cmp(&totals[*left])
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| left.cmp(right))
                } else {
                    left.cmp(right)
                }
            })
        });
    }
}

fn validate_acyclic(
    root: usize,
    children: &[Vec<usize>],
    node_count: usize,
) -> Result<(), SunburstError> {
    let mut visited = AHashSet::new();
    let mut stack = AHashSet::new();
    if !visit(root, children, &mut visited, &mut stack) || visited.len() != node_count {
        return Err(SunburstError::InvalidHierarchy);
    }
    Ok(())
}

fn visit(
    index: usize,
    children: &[Vec<usize>],
    visited: &mut AHashSet<usize>,
    stack: &mut AHashSet<usize>,
) -> bool {
    if stack.contains(&index) {
        return false;
    }
    if visited.contains(&index) {
        return true;
    }
    stack.insert(index);
    for child in &children[index] {
        if !visit(*child, children, visited, stack) {
            return false;
        }
    }
    stack.remove(&index);
    visited.insert(index);
    true
}

fn compute_total(
    index: usize,
    children: &[Vec<usize>],
    nodes: &[SunburstNode],
    totals: &mut [f32],
) -> f32 {
    let child_total = children[index]
        .iter()
        .map(|child| compute_total(*child, children, nodes, totals))
        .sum::<f32>();
    let total = nodes[index].value.max(child_total);
    totals[index] = total;
    total
}

fn assign_depths(index: usize, depth: usize, children: &[Vec<usize>], depths: &mut [usize]) {
    depths[index] = depth;
    for child in &children[index] {
        assign_depths(*child, depth + 1, children, depths);
    }
}

fn node_path(index: usize, nodes: &[SunburstNode]) -> String {
    let lookup: AHashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    let mut labels = Vec::new();
    let mut cursor = Some(index);
    while let Some(index) = cursor {
        let node = &nodes[index];
        labels.push(node.label.clone());
        cursor = node
            .parent
            .as_deref()
            .and_then(|parent| lookup.get(parent).copied());
    }
    labels.reverse();
    labels.join(" / ")
}

fn leaf_count(index: usize, children: &[Vec<usize>]) -> usize {
    if children[index].is_empty() {
        return 1;
    }
    children[index]
        .iter()
        .map(|child| leaf_count(*child, children))
        .sum()
}

fn branch_summaries(
    root: usize,
    children: &[Vec<usize>],
    nodes: &[SunburstNode],
    totals: &[f32],
    root_total: f32,
) -> Vec<SunburstBranchSummary> {
    children[root]
        .iter()
        .map(|branch| {
            let leaves = collect_leaves(*branch, children);
            let largest_leaf = leaves
                .iter()
                .max_by(|left, right| {
                    totals[**left]
                        .partial_cmp(&totals[**right])
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| right.cmp(left))
                })
                .copied()
                .unwrap_or(*branch);
            let branch_total = totals[*branch];
            SunburstBranchSummary {
                label: nodes[*branch].label.clone(),
                value: branch_total,
                share_total: branch_total / root_total * 100.0,
                leaves: leaves.len(),
                largest_leaf_label: nodes[largest_leaf].label.clone(),
                largest_leaf_value: totals[largest_leaf],
                largest_leaf_share: if branch_total > 0.0 {
                    totals[largest_leaf] / branch_total * 100.0
                } else {
                    100.0
                },
            }
        })
        .collect()
}

fn collect_leaves(index: usize, children: &[Vec<usize>]) -> Vec<usize> {
    if children[index].is_empty() {
        return vec![index];
    }
    children[index]
        .iter()
        .flat_map(|child| collect_leaves(*child, children))
        .collect()
}

fn sector_dataset(sectors: &[SunburstSector]) -> Dataset {
    Dataset::new(
        SECTOR_DATASET,
        1,
        vec![
            (
                "label".into(),
                Column::Utf8(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| Arc::<str>::from(sector.label.clone()))
                        .collect(),
                )),
            ),
            (
                "path".into(),
                Column::Utf8(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| Arc::<str>::from(sector.path.clone()))
                        .collect(),
                )),
            ),
            (
                "branch".into(),
                Column::Utf8(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| Arc::<str>::from(sector.branch.clone()))
                        .collect(),
                )),
            ),
            (
                "value".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.value).collect(),
                )),
            ),
            (
                "share_total".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.share_total).collect(),
                )),
            ),
            (
                "share_parent".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.share_parent).collect(),
                )),
            ),
            (
                "share_branch".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.share_branch).collect(),
                )),
            ),
            (
                "depth".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.depth as f32).collect(),
                )),
            ),
            (
                "children".into(),
                Column::F32(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| sector.children as f32)
                        .collect(),
                )),
            ),
            (
                "leaves".into(),
                Column::F32(ColumnData::new(
                    sectors.iter().map(|sector| sector.leaves as f32).collect(),
                )),
            ),
            (
                "sibling_rank".into(),
                Column::F32(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| (sector.sibling_index + 1) as f32)
                        .collect(),
                )),
            ),
            (
                "sibling_count".into(),
                Column::F32(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| sector.sibling_count as f32)
                        .collect(),
                )),
            ),
            (
                "group".into(),
                Column::Utf8(ColumnData::new(
                    sectors
                        .iter()
                        .map(|sector| Arc::<str>::from(sector.group.clone()))
                        .collect(),
                )),
            ),
        ],
    )
}

fn sector_label_tooltip(sector: &SunburstSector) -> LabelTooltip {
    label_tooltip(
        &sector.label,
        vec![
            ("Path", sector.path.clone()),
            ("Branch", sector.branch.clone()),
            ("Value", format!("{:.0}", sector.value)),
            ("Share of total", format!("{:.1}%", sector.share_total)),
            ("Branch share", format!("{:.1}%", sector.share_branch)),
            ("Parent share", format!("{:.1}%", sector.share_parent)),
            ("Depth", format!("{}", sector.depth)),
            ("Leaves", format!("{}", sector.leaves)),
            (
                "Sibling rank",
                format!("{} of {}", sector.sibling_index + 1, sector.sibling_count),
            ),
        ],
    )
}

fn label_tooltip(title: impl Into<String>, rows: Vec<(&'static str, String)>) -> LabelTooltip {
    LabelTooltip::new(
        title,
        rows.into_iter()
            .map(|(label, value)| LabelTooltipRow::new(label, value))
            .collect(),
    )
}

fn mix_u64(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x0100_0000_01b3);
}

fn slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn group_palette(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 4]; 8] = [
        [0.08, 0.43, 0.72, 0.86],
        [0.13, 0.58, 0.52, 0.84],
        [0.67, 0.39, 0.16, 0.82],
        [0.61, 0.36, 0.70, 0.82],
        [0.76, 0.28, 0.25, 0.80],
        [0.35, 0.50, 0.22, 0.82],
        [0.18, 0.30, 0.48, 0.82],
        [0.72, 0.52, 0.18, 0.80],
    ];
    let [r, g, b, a] = COLORS[index % COLORS.len()];
    rgba(r, g, b, a)
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 0.001;

    fn assert_near(left: f32, right: f32) {
        assert!(
            (left - right).abs() <= EPS,
            "expected {left:.3} to be within {EPS:.3} of {right:.3}"
        );
    }

    fn demo_spec() -> SunburstSpec {
        SunburstSpec::new(vec![
            SunburstNode::new(
                "all",
                "All revenue",
                None::<String>,
                0.0,
                "root",
                rgba(0.16, 0.20, 0.28, 1.0),
            ),
            SunburstNode::new(
                "new",
                "New",
                Some("all"),
                0.0,
                "acquisition",
                rgba(0.08, 0.43, 0.72, 0.86),
            ),
            SunburstNode::new(
                "expansion",
                "Expansion",
                Some("all"),
                0.0,
                "growth",
                rgba(0.13, 0.58, 0.52, 0.84),
            ),
            SunburstNode::new(
                "new/direct",
                "Direct",
                Some("new"),
                42.0,
                "acquisition",
                rgba(0.08, 0.43, 0.72, 0.86),
            ),
            SunburstNode::new(
                "new/partner",
                "Partner",
                Some("new"),
                28.0,
                "acquisition",
                rgba(0.07, 0.48, 0.64, 0.82),
            ),
            SunburstNode::new(
                "expansion/seat",
                "Seats",
                Some("expansion"),
                36.0,
                "growth",
                rgba(0.13, 0.58, 0.52, 0.84),
            ),
            SunburstNode::new(
                "expansion/usage",
                "Usage",
                Some("expansion"),
                18.0,
                "growth",
                rgba(0.20, 0.62, 0.42, 0.80),
            ),
        ])
    }

    #[test]
    fn sunburst_rejects_missing_parent() {
        let spec = SunburstSpec::new(vec![
            SunburstNode::new(
                "root",
                "Root",
                None::<String>,
                0.0,
                "root",
                rgba(0.0, 0.0, 0.0, 1.0),
            ),
            SunburstNode::new(
                "child",
                "Child",
                Some("missing"),
                1.0,
                "x",
                rgba(0.0, 0.0, 0.0, 1.0),
            ),
        ]);

        assert_eq!(
            spec.layout(ChartSize::new(320, 240)).unwrap_err(),
            SunburstError::MissingParent {
                id: "missing".into()
            }
        );
    }

    #[test]
    fn sunburst_layout_computes_parent_totals_and_depths() {
        let layout = demo_spec().layout(ChartSize::new(520, 420)).unwrap();

        assert_eq!(layout.sectors.len(), 7);
        assert_eq!(layout.max_depth, 2);
        assert!((layout.root_total - 124.0).abs() < 0.01);
        let new = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "new")
            .unwrap();
        assert!((new.value - 70.0).abs() < 0.01);
        assert!((new.share_total - 56.4516).abs() < 0.05);
        assert_eq!(new.branch, "New");
        assert!((new.share_branch - 100.0).abs() < 0.01);

        let direct = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "new/direct")
            .unwrap();
        assert_eq!(direct.branch, "New");
        assert!((direct.share_branch - 60.0).abs() < 0.01);
    }

    #[test]
    fn sunburst_builds_chart_with_guides_and_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(520, 420))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.scene().guides.is_empty());
        assert_eq!(chart.snap_targets().len(), 7);
    }

    #[test]
    fn sunburst_layout_applies_angular_gaps_to_non_root_sectors() {
        let layout = demo_spec().layout(ChartSize::new(520, 420)).unwrap();
        let root = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "all")
            .unwrap();
        let child = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "new")
            .unwrap();

        assert!(child.start_angle > root.start_angle);
        assert!(child.end_angle < root.end_angle);
    }

    #[test]
    fn sunburst_sorts_unordered_siblings_by_computed_value() {
        let layout = demo_spec().layout(ChartSize::new(520, 420)).unwrap();
        let root_children: Vec<_> = layout
            .sectors
            .iter()
            .filter(|sector| sector.parent == Some(0))
            .map(|sector| sector.id.as_str())
            .collect();

        assert_eq!(root_children, vec!["new", "expansion"]);
    }

    #[test]
    fn sunburst_layout_allocates_fractional_values_against_parent_total() {
        let spec = SunburstSpec::new(vec![
            SunburstNode::new(
                "root",
                "Root",
                None::<String>,
                0.0,
                "root",
                rgba(0.16, 0.20, 0.28, 1.0),
            ),
            SunburstNode::new(
                "small",
                "Small",
                Some("root"),
                0.2,
                "mix",
                rgba(0.08, 0.43, 0.72, 0.86),
            ),
            SunburstNode::new(
                "large",
                "Large",
                Some("root"),
                0.3,
                "mix",
                rgba(0.13, 0.58, 0.52, 0.84),
            ),
        ])
        .with_options(SunburstOptions::default().with_angular_gap(0.0));

        let layout = spec.layout(ChartSize::new(420, 320)).unwrap();
        let root = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "root")
            .unwrap();
        let children = layout
            .sectors
            .iter()
            .filter(|sector| sector.parent == Some(0))
            .collect::<Vec<_>>();

        assert_near(layout.root_total, 0.5);
        assert_near(
            children.iter().map(|sector| sector.sweep()).sum::<f32>(),
            root.sweep(),
        );
        assert_near(children[0].sweep(), root.sweep() * 0.6);
        assert_eq!(children[0].id, "large");
    }

    #[test]
    fn sunburst_zero_value_sectors_are_not_visible_pickable_or_snappable() {
        let spec = SunburstSpec::new(vec![
            SunburstNode::new(
                "root",
                "Root",
                None::<String>,
                0.0,
                "root",
                rgba(0.16, 0.20, 0.28, 1.0),
            ),
            SunburstNode::new(
                "visible",
                "Visible",
                Some("root"),
                1.0,
                "mix",
                rgba(0.08, 0.43, 0.72, 0.86),
            ),
            SunburstNode::new(
                "zero",
                "Zero",
                Some("root"),
                0.0,
                "mix",
                rgba(0.13, 0.58, 0.52, 0.84),
            ),
        ]);

        let layout = spec.layout(ChartSize::new(420, 320)).unwrap();
        let zero = layout
            .sectors
            .iter()
            .find(|sector| sector.id == "zero")
            .unwrap();

        assert_near(zero.sweep(), 0.0);
        assert!(!sector_contains(zero, zero.label_point(0.5)));

        let chart = spec
            .try_build_chart(Workspace::new(), ChartSize::new(420, 320))
            .unwrap();
        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(&labels.items),
                _ => None,
            })
            .unwrap();
        assert!(!labels.iter().any(|label| label.text == "Zero"));

        let snap_labels = chart
            .snap_targets()
            .into_iter()
            .filter_map(|target| target.label)
            .collect::<Vec<_>>();
        assert!(!snap_labels.iter().any(|label| label == "Zero sector"));
    }

    #[test]
    fn sunburst_rejects_nonfinite_layout_options() {
        let spec = demo_spec().with_options(SunburstOptions::default().with_padding(f32::NAN));

        assert!(matches!(
            spec.layout(ChartSize::new(520, 420)).unwrap_err(),
            SunburstError::InvalidOption { name, .. } if name == "padding"
        ));
    }

    #[test]
    fn sunburst_options_builders_preserve_chained_settings() {
        let options = SunburstOptions::default()
            .with_padding(12.0)
            .with_inner_radius(24.0)
            .with_ring_gap(1.5)
            .with_angular_gap(0.0)
            .with_start_angle(0.25)
            .with_sweep_angle(TAU * 0.5)
            .with_label_thresholds(0.12, 480.0)
            .with_max_visible_labels(9)
            .with_separator(rgba(0.2, 0.3, 0.4, 0.5), 0.75)
            .with_sort_siblings_by_value(false);

        assert_near(options.padding, 12.0);
        assert_near(options.inner_radius, 24.0);
        assert_near(options.ring_gap, 1.5);
        assert_near(options.angular_gap, 0.0);
        assert_near(options.start_angle, 0.25);
        assert_near(options.sweep_angle, TAU * 0.5);
        assert_near(options.min_label_sweep, 0.12);
        assert_near(options.min_label_area, 480.0);
        assert_eq!(options.max_visible_labels, 9);
        assert_eq!(options.separator_color, rgba(0.2, 0.3, 0.4, 0.5));
        assert_near(options.separator_width, 0.75);
        assert!(!options.sort_siblings_by_value);
    }

    #[test]
    fn sunburst_summary_reports_concentration() {
        let summary = demo_spec().summary().unwrap();

        assert_eq!(summary.nodes, 7);
        assert_eq!(summary.leaves, 4);
        assert_eq!(summary.max_depth, 2);
        assert_eq!(summary.branches.len(), 2);
        assert_eq!(summary.branches[0].label, "New");
        assert_eq!(summary.branches[0].leaves, 2);
        assert_eq!(summary.branches[0].largest_leaf_label, "Direct");
        assert!((summary.branches[0].largest_leaf_share - 60.0).abs() < 0.01);
        assert_eq!(summary.largest_leaf_label, "Direct");
        assert_eq!(summary.largest_leaf_path, "All revenue / New / Direct");
        assert!((summary.largest_leaf_value - 42.0).abs() < 0.01);
        assert!((summary.top_three_leaf_share - 85.48).abs() < 0.05);
    }

    #[test]
    fn sunburst_from_paths_infers_hierarchy() {
        let spec = SunburstSpec::from_paths(vec![
            SunburstPath::new(vec!["Revenue", "New", "Direct"], 42.0, "acquisition"),
            SunburstPath::new(vec!["Revenue", "New", "Partner"], 28.0, "acquisition"),
        ]);
        let layout = spec.layout(ChartSize::new(420, 320)).unwrap();

        assert_eq!(spec.nodes.len(), 4);
        assert!((layout.root_total - 70.0).abs() < 0.01);
        assert_eq!(layout.max_depth, 2);
    }
}
