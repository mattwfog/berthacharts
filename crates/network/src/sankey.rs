//! Sankey chart spec and ribbon mark.

use std::fmt;
use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use berthacharts_core::{
    CartesianCoord, Chart, ChartSize, ChartSpec, ColorChannel, Column, ColumnData, CoordId,
    Dataset, DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem,
    LabelKind, LabelPriority, LabelTooltip, LabelTooltipRow, Layer, LayerId, LegendAnchor,
    LegendGuide, LegendItem, LinePrim, LinearScale, Mark, MarkId, NumberChannel, PickCtx, PickHit,
    Rect, RectMark, Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx,
    TooltipField, TooltipGuide, TrianglePrim, Workspace,
};

const NODE_DATASET: DatasetId = DatasetId::new(0);
const LINK_DATASET: DatasetId = DatasetId::new(1);
const RIBBON_MARK: MarkId = MarkId::new(1);
const NODE_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);
const RIBBON_STEPS: usize = 32;

/// A Sankey node supplied by users.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    /// Stable node id used by links.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Column/stage index.
    pub stage: usize,
    /// Optional sort key within a stage. Lower renders earlier/topper.
    pub order: Option<i32>,
    /// RGB node color.
    pub color: [f32; 3],
}

impl SankeyNode {
    /// Build a Sankey node.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        stage: usize,
        color: [f32; 3],
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            stage,
            order: None,
            color,
        }
    }

    /// Set stage-local sort order.
    #[must_use]
    pub const fn with_order(mut self, order: i32) -> Self {
        self.order = Some(order);
        self
    }
}

/// A Sankey link supplied by users.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLink {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Link magnitude.
    pub value: f32,
    /// Display/legend class.
    pub class: String,
    /// Premultiplied ribbon color.
    pub color: [f32; 4],
}

impl SankeyLink {
    /// Build a Sankey link.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        value: f32,
        class: impl Into<String>,
        color: [f32; 4],
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value,
            class: class.into(),
            color,
        }
    }
}

/// Minimal Sankey flow input for users who do not want to declare a full
/// node/link schema up front.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyFlow {
    /// Source node id or label.
    pub source: String,
    /// Target node id or label.
    pub target: String,
    /// Flow magnitude.
    pub value: f32,
    /// Optional display/legend class. Defaults to `"flow"`.
    pub class: Option<String>,
    /// Optional premultiplied ribbon color. Defaults from the class palette.
    pub color: Option<[f32; 4]>,
}

impl SankeyFlow {
    /// Build a minimal Sankey flow.
    #[must_use]
    pub fn new(source: impl Into<String>, target: impl Into<String>, value: f32) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value,
            class: None,
            color: None,
        }
    }

    /// Set a legend/display class for this flow.
    #[must_use]
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Set a premultiplied ribbon color for this flow.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = Some(color);
        self
    }
}

/// A stage/column label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SankeyStage {
    /// Stage index.
    pub index: usize,
    /// Display label.
    pub label: String,
    /// Optional secondary detail.
    pub detail: Option<String>,
}

impl SankeyStage {
    /// Build a stage label.
    #[must_use]
    pub fn new(index: usize, label: impl Into<String>) -> Self {
        Self {
            index,
            label: label.into(),
            detail: None,
        }
    }

    /// Set secondary detail.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Legend item for a link class.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLegendItem {
    /// Display label.
    pub label: String,
    /// Swatch color.
    pub color: [f32; 4],
}

impl SankeyLegendItem {
    /// Build a legend item.
    #[must_use]
    pub fn new(label: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            label: label.into(),
            color,
        }
    }
}

/// Layout and guide options for a Sankey chart.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyOptions {
    /// Node width in CSS pixels.
    pub node_width: f32,
    /// Minimum vertical gap between stacked nodes.
    pub node_gap: f32,
    /// Left inset.
    pub padding_left: f32,
    /// Right inset.
    pub padding_right: f32,
    /// Top inset.
    pub padding_top: f32,
    /// Bottom inset.
    pub padding_bottom: f32,
    /// Pixels overlapped into node edges to hide antialias seams.
    pub ribbon_overlap: f32,
    /// Preferred value-to-pixel scale. The layout shrinks it if needed.
    pub preferred_flow_scale: f32,
    /// Minimum ribbon height in pixels for direct flow labels.
    pub min_flow_label_px: f32,
    /// Anchor position along ribbon for direct labels.
    pub flow_label_t: f32,
}

impl Default for SankeyOptions {
    fn default() -> Self {
        Self {
            node_width: 18.0,
            node_gap: 32.0,
            padding_left: 42.0,
            padding_right: 102.0,
            padding_top: 78.0,
            padding_bottom: 70.0,
            ribbon_overlap: 0.8,
            preferred_flow_scale: 1.0,
            min_flow_label_px: 36.0,
            flow_label_t: 0.52,
        }
    }
}

/// Reusable Sankey chart specification.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeySpec {
    /// Nodes in author order.
    pub nodes: Vec<SankeyNode>,
    /// Links in author order.
    pub links: Vec<SankeyLink>,
    /// Stage labels.
    pub stages: Vec<SankeyStage>,
    /// Optional explicit legend items. When empty, items are inferred from
    /// link classes in first-seen order.
    pub legend: Vec<SankeyLegendItem>,
    /// Layout and guide options.
    pub options: SankeyOptions,
}

impl SankeySpec {
    /// Build a Sankey spec from nodes and links.
    #[must_use]
    pub fn new(nodes: Vec<SankeyNode>, links: Vec<SankeyLink>) -> Self {
        Self {
            nodes,
            links,
            stages: Vec::new(),
            legend: Vec::new(),
            options: SankeyOptions::default(),
        }
    }

    /// Build a Sankey spec from minimal source/target/value flows.
    ///
    /// Nodes are inferred in first-seen order. Stages are assigned by longest
    /// observed upstream path, so a link can intentionally skip intermediate
    /// stages when the target also appears later in the graph.
    #[must_use]
    pub fn from_flows(flows: Vec<SankeyFlow>) -> Self {
        let node_ids = infer_node_order(&flows);
        let stages = infer_node_stages(&node_ids, &flows);
        let mut class_colors: AHashMap<String, [f32; 4]> = AHashMap::new();
        let mut next_class_color = 0usize;

        let nodes = node_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                SankeyNode::new(
                    id.clone(),
                    humanize_id(id),
                    stages.get(id.as_str()).copied().unwrap_or_default(),
                    node_palette(index),
                )
            })
            .collect();

        let links = flows
            .into_iter()
            .map(|flow| {
                let class = flow.class.unwrap_or_else(|| "flow".to_string());
                let color = flow.color.unwrap_or_else(|| {
                    let entry = class_colors.entry(class.clone()).or_insert_with(|| {
                        let color = link_palette(next_class_color);
                        next_class_color += 1;
                        color
                    });
                    *entry
                });
                SankeyLink::new(flow.source, flow.target, flow.value, class, color)
            })
            .collect();

        Self::new(nodes, links)
    }

    /// Set stage labels.
    #[must_use]
    pub fn with_stages(mut self, stages: Vec<SankeyStage>) -> Self {
        self.stages = stages;
        self
    }

    /// Set explicit legend items.
    #[must_use]
    pub fn with_legend(mut self, legend: Vec<SankeyLegendItem>) -> Self {
        self.legend = legend;
        self
    }

    /// Set layout/guide options.
    #[must_use]
    pub const fn with_options(mut self, options: SankeyOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute reusable layout without building a chart.
    pub fn layout(&self, size: ChartSize) -> Result<SankeyLayout, SankeyError> {
        self.validate()?;
        let totals = self.node_totals();
        let stage_count = self.stage_count();
        let stage_slots = self.stage_slots(stage_count);
        let flow_scale = self.flow_scale(size, &stage_slots, &totals);
        let layout_span = self.layout_span(&stage_slots, &totals, flow_scale);
        let stage_step = if stage_count <= 1 {
            0.0
        } else {
            ((size.width as f32)
                - self.options.padding_left
                - self.options.padding_right
                - self.options.node_width)
                / (stage_count - 1) as f32
        };

        let mut layout_nodes = Vec::with_capacity(self.nodes.len());
        for stage in 0..stage_count {
            let slot = &stage_slots[stage];
            let used = self.stage_used_height(slot, &totals, flow_scale);
            let y0 = self.options.padding_top + (layout_span - used).max(0.0) * 0.5;
            let x = self.options.padding_left + stage as f32 * stage_step;
            let mut y = y0;
            for &node_index in slot {
                let input = &self.nodes[node_index];
                let total = totals[node_index];
                let h = total * flow_scale;
                layout_nodes.push(SankeyLayoutNode {
                    source_index: node_index,
                    id: input.id.clone(),
                    label: input.label.clone(),
                    stage,
                    x,
                    y,
                    total,
                    flow_scale,
                    color: input.color,
                });
                y += h + self.options.node_gap;
            }
        }
        layout_nodes.sort_by_key(|node| node.source_index);

        let mut ribbons = self.build_ribbons(&layout_nodes, flow_scale)?;
        if let Err(message) = validate_stack_integrity(&layout_nodes, &self.links, &ribbons) {
            debug_assert!(false, "{message}");
        }

        Ok(SankeyLayout {
            nodes: layout_nodes,
            ribbons: {
                ribbons.shrink_to_fit();
                ribbons
            },
            stages: self.resolved_stages(stage_count, &totals),
            flow_scale,
            size,
        })
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, SankeyError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), SankeyError> {
        if self.nodes.is_empty() {
            return Err(SankeyError::EmptyNodes);
        }
        if self.links.is_empty() {
            return Err(SankeyError::EmptyLinks);
        }

        let mut ids = AHashSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                return Err(SankeyError::EmptyNodeId);
            }
            if !ids.insert(node.id.as_str()) {
                return Err(SankeyError::DuplicateNode {
                    id: node.id.clone(),
                });
            }
        }

        for link in &self.links {
            if link.value <= 0.0 || !link.value.is_finite() {
                return Err(SankeyError::NonPositiveLink {
                    source: link.source.clone(),
                    target: link.target.clone(),
                    value: link.value,
                });
            }
            if !ids.contains(link.source.as_str()) {
                return Err(SankeyError::MissingNode {
                    id: link.source.clone(),
                });
            }
            if !ids.contains(link.target.as_str()) {
                return Err(SankeyError::MissingNode {
                    id: link.target.clone(),
                });
            }
        }

        Ok(())
    }

    fn stage_count(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.stage)
            .max()
            .unwrap_or_default()
            + 1
    }

    fn stage_slots(&self, stage_count: usize) -> Vec<Vec<usize>> {
        let mut slots = vec![Vec::new(); stage_count];
        for (index, node) in self.nodes.iter().enumerate() {
            slots[node.stage].push(index);
        }
        for slot in &mut slots {
            slot.sort_by_key(|index| {
                let node = &self.nodes[*index];
                (node.order.unwrap_or(*index as i32), *index)
            });
        }
        slots
    }

    fn node_totals(&self) -> Vec<f32> {
        let index = self.node_lookup();
        let mut incoming = vec![0.0; self.nodes.len()];
        let mut outgoing = vec![0.0; self.nodes.len()];

        for link in &self.links {
            if let Some(&source) = index.get(link.source.as_str()) {
                outgoing[source] += link.value;
            }
            if let Some(&target) = index.get(link.target.as_str()) {
                incoming[target] += link.value;
            }
        }

        incoming
            .into_iter()
            .zip(outgoing)
            .map(|(incoming, outgoing)| incoming.max(outgoing))
            .collect()
    }

    fn flow_scale(&self, size: ChartSize, stage_slots: &[Vec<usize>], totals: &[f32]) -> f32 {
        let available =
            (size.height as f32 - self.options.padding_top - self.options.padding_bottom).max(1.0);
        let mut required_scale = self.options.preferred_flow_scale.max(0.01);

        for slot in stage_slots {
            let total: f32 = slot.iter().map(|index| totals[*index]).sum();
            if total <= f32::EPSILON {
                continue;
            }
            let gaps = self.options.node_gap * slot.len().saturating_sub(1) as f32;
            let scale = ((available - gaps).max(1.0) / total).max(0.01);
            required_scale = required_scale.min(scale);
        }

        required_scale
    }

    fn layout_span(&self, stage_slots: &[Vec<usize>], totals: &[f32], flow_scale: f32) -> f32 {
        stage_slots
            .iter()
            .map(|slot| self.stage_used_height(slot, totals, flow_scale))
            .fold(0.0, f32::max)
    }

    fn stage_used_height(&self, slot: &[usize], totals: &[f32], flow_scale: f32) -> f32 {
        let total: f32 = slot.iter().map(|index| totals[*index] * flow_scale).sum();
        total + self.options.node_gap * slot.len().saturating_sub(1) as f32
    }

    fn build_ribbons(
        &self,
        nodes: &[SankeyLayoutNode],
        flow_scale: f32,
    ) -> Result<Vec<SankeyRibbon>, SankeyError> {
        let index = layout_node_lookup(nodes);
        let mut source_offsets = vec![0.0; nodes.len()];
        let mut target_offsets = vec![0.0; nodes.len()];
        let mut ribbons = Vec::with_capacity(self.links.len());

        for link in &self.links {
            let source =
                *index
                    .get(link.source.as_str())
                    .ok_or_else(|| SankeyError::MissingNode {
                        id: link.source.clone(),
                    })?;
            let target =
                *index
                    .get(link.target.as_str())
                    .ok_or_else(|| SankeyError::MissingNode {
                        id: link.target.clone(),
                    })?;
            let source_node = &nodes[source];
            let target_node = &nodes[target];
            let height = link.value * flow_scale;
            let source_y0 = source_node.y + source_offsets[source];
            let source_y1 = source_y0 + height;
            let target_y0 = target_node.y + target_offsets[target];
            let target_y1 = target_y0 + height;
            source_offsets[source] += height;
            target_offsets[target] += height;
            ribbons.push(SankeyRibbon {
                source: link.source.clone(),
                target: link.target.clone(),
                source_stage: source_node.stage,
                target_stage: target_node.stage,
                value: link.value,
                class: link.class.clone(),
                source_x: source_node.x2(self.options.node_width) - self.options.ribbon_overlap,
                source_y0,
                source_y1,
                target_x: target_node.x + self.options.ribbon_overlap,
                target_y0,
                target_y1,
                color: link.color,
            });
        }

        Ok(ribbons)
    }

    fn resolved_stages(&self, stage_count: usize, totals: &[f32]) -> Vec<SankeyLayoutStage> {
        let stage_lookup: AHashMap<usize, &SankeyStage> = self
            .stages
            .iter()
            .map(|stage| (stage.index, stage))
            .collect();
        let stage_slots = self.stage_slots(stage_count);

        (0..stage_count)
            .map(|stage| {
                let stage_total: f32 = stage_slots[stage].iter().map(|index| totals[*index]).sum();
                let x = if stage_count <= 1 {
                    self.options.padding_left
                } else {
                    let step = ((self.options_width_hint(stage_count) - self.options.node_width)
                        / (stage_count - 1) as f32)
                        .max(0.0);
                    self.options.padding_left + stage as f32 * step
                };
                let fallback = format!("stage {}", stage + 1);
                let input = stage_lookup.get(&stage).copied();
                SankeyLayoutStage {
                    index: stage,
                    x: x + self.options.node_width * 0.5,
                    label: input.map(|stage| stage.label.clone()).unwrap_or(fallback),
                    detail: input
                        .and_then(|stage| stage.detail.clone())
                        .unwrap_or_else(|| format!("{stage_total:.0} total")),
                }
            })
            .collect()
    }

    fn options_width_hint(&self, stage_count: usize) -> f32 {
        // Used only for fallback stage-label x values before the final chart
        // size is known inside `resolved_stages`. Stage x is corrected by
        // `with_stage_positions` in `layout`.
        (self.options.padding_left + self.options.padding_right + self.options.node_width)
            .max(stage_count as f32)
    }

    fn node_lookup(&self) -> AHashMap<&str, usize> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect()
    }

    fn legend_items(&self) -> Vec<LegendItem> {
        if !self.legend.is_empty() {
            return self
                .legend
                .iter()
                .map(|item| LegendItem::new(item.label.clone(), item.color))
                .collect();
        }

        let mut seen = AHashSet::new();
        let mut items = Vec::new();
        for link in &self.links {
            if seen.insert(link.class.as_str()) {
                items.push(LegendItem::new(link.class.clone(), link.color));
            }
        }
        items
    }
}

impl ChartSpec for SankeySpec {
    type Error = SankeyError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        let mut layout = self.layout(size)?;
        layout.with_stage_positions(
            self.options.padding_left,
            self.options.padding_right,
            self.options.node_width,
        );

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

        let start_total = layout
            .nodes
            .iter()
            .filter(|node| node.stage == 0)
            .map(|node| node.total)
            .sum::<f32>()
            .max(1.0);

        workspace.upsert_dataset(node_dataset(
            &layout.nodes,
            start_total,
            self.options.node_width,
        ));
        workspace.upsert_dataset(link_dataset(&layout.ribbons, &layout.nodes, start_total));

        let mut node_mark = RectMark::new(
            NODE_MARK,
            NODE_DATASET,
            NumberChannel::Column {
                dataset: NODE_DATASET,
                name: "x1".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: NODE_DATASET,
                name: "y1".into(),
                scale: Y_SCALE,
            },
            NumberChannel::Column {
                dataset: NODE_DATASET,
                name: "x2".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: NODE_DATASET,
                name: "y2".into(),
                scale: Y_SCALE,
            },
            [0.2, 0.5, 0.9, 1.0],
        );
        node_mark.fill = ColorChannel::RgbaColumns {
            dataset: NODE_DATASET,
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };
        node_mark.stroke = ColorChannel::Constant(rgba(1.0, 1.0, 1.0, 0.0));
        node_mark.stroke_width = 0.0;
        node_mark.radius = 0.0;

        let labels = self.labels(&layout, start_total);
        let visible_label_count = labels.len();
        let mut scene = Scene::new(size.full_viewport());
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![
                Arc::new(SankeyRibbonMark::new(RIBBON_MARK, layout.ribbons.clone()))
                    as Arc<dyn Mark>,
                Arc::new(node_mark) as Arc<dyn Mark>,
            ],
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });

        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                RIBBON_MARK,
                LINK_DATASET,
                vec![
                    TooltipField::new("Source", "source"),
                    TooltipField::new("Target", "target"),
                    TooltipField::new("Flow", "flow").as_integer(),
                    TooltipField::new("Source %", "conversion").as_percent(1),
                    TooltipField::new("Start %", "start_share").as_percent(1),
                    TooltipField::new("Stage span", "stage_span").as_integer(),
                    TooltipField::new("Class", "class").as_label(),
                ],
            )
            .with_title_column("link"),
        ));
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                NODE_MARK,
                NODE_DATASET,
                vec![
                    TooltipField::new("Throughput", "throughput").as_integer(),
                    TooltipField::new("Share of start", "share").as_percent(0),
                ],
            )
            .with_title_column("name"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(3.0)
                .with_max_visible(visible_label_count),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(self.legend_items())
                .with_title("Flow class")
                .with_anchor(LegendAnchor::Bottom),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(self.snap_targets(&layout)).with_name("flow anchors"),
        ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

impl SankeySpec {
    fn labels(&self, layout: &SankeyLayout, start_total: f32) -> Vec<LabelItem> {
        let mut labels = Vec::new();
        labels.extend(layout.stages.iter().map(|stage| {
            LabelItem::new(stage.x, 26.0, stage.label.to_ascii_lowercase())
                .with_detail(&stage.detail)
                .with_kind(LabelKind::Column)
                .with_priority(LabelPriority::Required)
                .with_anchor(LabelAnchor::Bottom)
                .with_reposition(false)
                .with_tooltip(stage_label_tooltip(stage))
        }));
        labels.extend(layout.nodes.iter().map(|node| {
            LabelItem::new(
                node.x + self.options.node_width * 0.5,
                node.y + 6.0,
                &node.label,
            )
            .with_detail(format!(
                "{:.0} / {:.0}%",
                node.total,
                node.total / start_total * 100.0
            ))
            .with_kind(LabelKind::Node)
            .with_priority(LabelPriority::Required)
            .with_anchor(LabelAnchor::Top)
            .with_reposition(false)
            .with_tooltip(label_tooltip(
                &node.label,
                vec![
                    ("Throughput", format!("{:.0}", node.total)),
                    (
                        "Share of start",
                        format!("{:.0}%", node.total / start_total * 100.0),
                    ),
                    ("Stage", format!("{}", node.stage + 1)),
                ],
            ))
        }));
        labels.extend(
            layout
                .ribbons
                .iter()
                .filter(|ribbon| self.should_direct_label_flow(ribbon))
                .map(|ribbon| {
                    let (x, y) = ribbon.midpoint(self.options.flow_label_t);
                    let source_total = layout.node_total(&ribbon.source).unwrap_or(1.0);
                    LabelItem::new(x, y, format!("{:.0}", ribbon.value))
                        .with_detail(format!("{:.0}%", ribbon.value / start_total * 100.0))
                        .with_kind(LabelKind::Data)
                        .with_priority(LabelPriority::Important)
                        .with_anchor(LabelAnchor::Center)
                        .with_tooltip(label_tooltip(
                            format!("{} to {}", ribbon.source, ribbon.target),
                            vec![
                                ("Flow", format!("{:.0}", ribbon.value)),
                                (
                                    "Source share",
                                    format!("{:.0}%", ribbon.value / source_total * 100.0),
                                ),
                                (
                                    "Start share",
                                    format!("{:.0}%", ribbon.value / start_total * 100.0),
                                ),
                                ("Stage span", format!("{}", ribbon.stage_span())),
                                ("Class", ribbon.class.clone()),
                            ],
                        ))
                }),
        );
        labels
    }

    fn should_direct_label_flow(&self, ribbon: &SankeyRibbon) -> bool {
        ribbon.height() >= self.options.min_flow_label_px
    }

    fn snap_targets(&self, layout: &SankeyLayout) -> Vec<SnapTarget> {
        let mut targets: Vec<SnapTarget> = layout
            .nodes
            .iter()
            .map(|node| {
                SnapTarget::new(
                    node.x + self.options.node_width * 0.5,
                    node.y + node.height() * 0.5,
                    SnapKind::Node,
                )
                .with_radius(8.0)
                .with_label(format!("{} node", node.label))
                .with_priority(3)
            })
            .collect();
        targets.extend(
            layout
                .ribbons
                .iter()
                .filter(|ribbon| self.should_direct_label_flow(ribbon))
                .map(|ribbon| {
                    let (x, y) = ribbon.midpoint(self.options.flow_label_t);
                    SnapTarget::new(x, y, SnapKind::Edge)
                        .with_radius(ribbon.height().sqrt().clamp(5.0, 12.0))
                        .with_label(format!("{} to {}", ribbon.source, ribbon.target))
                        .with_priority(1)
                }),
        );
        targets
    }
}

/// Computed Sankey layout.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLayout {
    /// Laid-out nodes.
    pub nodes: Vec<SankeyLayoutNode>,
    /// Laid-out ribbons.
    pub ribbons: Vec<SankeyRibbon>,
    /// Stage label positions.
    pub stages: Vec<SankeyLayoutStage>,
    /// Final value-to-pixel flow scale.
    pub flow_scale: f32,
    /// Size used for layout.
    pub size: ChartSize,
}

impl SankeyLayout {
    fn with_stage_positions(&mut self, padding_left: f32, padding_right: f32, node_width: f32) {
        let stage_count = self.stages.len();
        if stage_count == 0 {
            return;
        }
        let step = if stage_count <= 1 {
            0.0
        } else {
            ((self.size.width as f32) - padding_left - padding_right - node_width)
                / (stage_count - 1) as f32
        };
        for stage in &mut self.stages {
            stage.x = padding_left + stage.index as f32 * step + node_width * 0.5;
        }
    }

    fn node_total(&self, id: &str) -> Option<f32> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.total)
    }
}

/// Laid-out Sankey node.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLayoutNode {
    /// Source input index.
    pub source_index: usize,
    /// Stable id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Stage index.
    pub stage: usize,
    /// Top-left x.
    pub x: f32,
    /// Top-left y.
    pub y: f32,
    /// Node total.
    pub total: f32,
    /// Flow scale used by this node.
    pub flow_scale: f32,
    /// RGB color.
    pub color: [f32; 3],
}

impl SankeyLayoutNode {
    /// Right x for this node at a supplied node width.
    #[must_use]
    pub fn x2(&self, node_width: f32) -> f32 {
        self.x + node_width
    }

    /// Bottom y for this node.
    #[must_use]
    pub fn y2(&self) -> f32 {
        self.y + self.height()
    }

    /// Rendered node height in pixels.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.total * self.flow_scale
    }
}

/// Laid-out stage label.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLayoutStage {
    /// Stage index.
    pub index: usize,
    /// Label anchor x.
    pub x: f32,
    /// Display label.
    pub label: String,
    /// Detail text.
    pub detail: String,
}

/// Laid-out Sankey ribbon.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyRibbon {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Source stage index.
    pub source_stage: usize,
    /// Target stage index.
    pub target_stage: usize,
    /// Link value.
    pub value: f32,
    /// Link class.
    pub class: String,
    /// Source x at node edge.
    pub source_x: f32,
    /// Source top y.
    pub source_y0: f32,
    /// Source bottom y.
    pub source_y1: f32,
    /// Target x at node edge.
    pub target_x: f32,
    /// Target top y.
    pub target_y0: f32,
    /// Target bottom y.
    pub target_y1: f32,
    /// Ribbon color.
    pub color: [f32; 4],
}

impl SankeyRibbon {
    /// Ribbon height in pixels.
    #[must_use]
    pub fn height(&self) -> f32 {
        (self.source_y1 - self.source_y0).abs()
    }

    /// Number of stage columns crossed by this ribbon.
    #[must_use]
    pub fn stage_span(&self) -> usize {
        self.target_stage.abs_diff(self.source_stage).max(1)
    }

    /// Ribbon centerline midpoint at `t`.
    #[must_use]
    pub fn midpoint(&self, t: f32) -> (f32, f32) {
        let top = self.edge(true, t);
        let bottom = self.edge(false, t);
        ((top[0] + bottom[0]) * 0.5, (top[1] + bottom[1]) * 0.5)
    }

    fn edge(&self, top: bool, t: f32) -> [f32; 2] {
        let dx = self.target_x - self.source_x;
        let c1x = self.source_x + dx * 0.48;
        let c2x = self.target_x - dx * 0.48;
        let (source_y, target_y) = if top {
            (self.source_y0, self.target_y0)
        } else {
            (self.source_y1, self.target_y1)
        };
        cubic(
            [self.source_x, source_y],
            [c1x, source_y],
            [c2x, target_y],
            [self.target_x, target_y],
            t,
        )
    }
}

/// Error building a Sankey chart.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SankeyError {
    /// No nodes were supplied.
    EmptyNodes,
    /// No links were supplied.
    EmptyLinks,
    /// A node id was empty.
    EmptyNodeId,
    /// A node id appeared more than once.
    DuplicateNode {
        /// Duplicate id.
        id: String,
    },
    /// A link references an unknown node.
    MissingNode {
        /// Missing node id.
        id: String,
    },
    /// A link has a non-positive or non-finite value.
    NonPositiveLink {
        /// Source id.
        source: String,
        /// Target id.
        target: String,
        /// Bad value.
        value: f32,
    },
}

impl fmt::Display for SankeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNodes => write!(f, "sankey requires at least one node"),
            Self::EmptyLinks => write!(f, "sankey requires at least one link"),
            Self::EmptyNodeId => write!(f, "sankey node id cannot be empty"),
            Self::DuplicateNode { id } => write!(f, "duplicate sankey node id `{id}`"),
            Self::MissingNode { id } => write!(f, "missing sankey node `{id}`"),
            Self::NonPositiveLink {
                source,
                target,
                value,
            } => write!(
                f,
                "sankey link `{source}` to `{target}` has invalid value {value}"
            ),
        }
    }
}

impl std::error::Error for SankeyError {}

#[derive(Debug, Clone)]
struct SankeyRibbonMark {
    id: MarkId,
    ribbons: Vec<SankeyRibbon>,
}

impl SankeyRibbonMark {
    fn new(id: MarkId, ribbons: Vec<SankeyRibbon>) -> Self {
        Self { id, ribbons }
    }
}

impl Mark for SankeyRibbonMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.ribbons.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut triangles = Vec::new();
        let mut outlines = Vec::new();
        for ribbon in &self.ribbons {
            push_ribbon(ribbon, &mut triangles);
            push_ribbon_outlines(ribbon, &mut outlines);
        }
        Geometry::Mixed(vec![
            Geometry::Triangles(triangles),
            Geometry::Lines(outlines),
        ])
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        for (row, ribbon) in self.ribbons.iter().enumerate().rev() {
            if ribbon_contains(ribbon, point) {
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

fn push_ribbon(ribbon: &SankeyRibbon, out: &mut Vec<TrianglePrim>) {
    let mut top = Vec::with_capacity(RIBBON_STEPS + 1);
    let mut bottom = Vec::with_capacity(RIBBON_STEPS + 1);
    for i in 0..=RIBBON_STEPS {
        let t = i as f32 / RIBBON_STEPS as f32;
        top.push(ribbon.edge(true, t));
        bottom.push(ribbon.edge(false, t));
    }

    for i in 0..RIBBON_STEPS {
        out.push(TrianglePrim {
            a: top[i],
            b: bottom[i],
            c: top[i + 1],
            fill: ribbon.color,
        });
        out.push(TrianglePrim {
            a: top[i + 1],
            b: bottom[i],
            c: bottom[i + 1],
            fill: ribbon.color,
        });
    }
}

fn push_ribbon_outlines(ribbon: &SankeyRibbon, out: &mut Vec<LinePrim>) {
    for top in [true, false] {
        out.push(LinePrim {
            points: (0..=RIBBON_STEPS)
                .map(|i| ribbon.edge(top, i as f32 / RIBBON_STEPS as f32))
                .collect(),
            stroke: rgba(1.0, 1.0, 1.0, 0.45),
            width: 1.0,
            dash: None,
            join: 1,
            cap: 1,
        });
    }
}

fn ribbon_contains(ribbon: &SankeyRibbon, point: (f32, f32)) -> bool {
    let (x, y) = point;
    if x < ribbon.source_x.min(ribbon.target_x) || x > ribbon.source_x.max(ribbon.target_x) {
        return false;
    }
    let dx = ribbon.target_x - ribbon.source_x;
    if dx.abs() < f32::EPSILON {
        return false;
    }
    let t = ((x - ribbon.source_x) / dx).clamp(0.0, 1.0);
    let top = ribbon.edge(true, t);
    let bottom = ribbon.edge(false, t);
    let y0 = top[1].min(bottom[1]) - 2.0;
    let y1 = top[1].max(bottom[1]) + 2.0;
    y >= y0 && y <= y1
}

fn cubic(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let mt = 1.0 - t;
    let a = mt * mt * mt;
    let b = 3.0 * mt * mt * t;
    let c = 3.0 * mt * t * t;
    let d = t * t * t;
    [
        a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0],
        a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1],
    ]
}

fn node_dataset(nodes: &[SankeyLayoutNode], start_total: f32, node_width: f32) -> Dataset {
    Dataset::new(
        NODE_DATASET,
        1,
        vec![
            (
                "x1".into(),
                Column::F32(ColumnData::new(nodes.iter().map(|node| node.x).collect())),
            ),
            (
                "y1".into(),
                Column::F32(ColumnData::new(nodes.iter().map(|node| node.y).collect())),
            ),
            (
                "x2".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.x2(node_width)).collect(),
                )),
            ),
            (
                "y2".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.y2()).collect(),
                )),
            ),
            (
                "r".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.color[0]).collect(),
                )),
            ),
            (
                "g".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.color[1]).collect(),
                )),
            ),
            (
                "b".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.color[2]).collect(),
                )),
            ),
            (
                "name".into(),
                Column::Utf8(ColumnData::new(
                    nodes
                        .iter()
                        .map(|node| Arc::<str>::from(node.label.clone()))
                        .collect(),
                )),
            ),
            (
                "throughput".into(),
                Column::F32(ColumnData::new(
                    nodes.iter().map(|node| node.total).collect(),
                )),
            ),
            (
                "share".into(),
                Column::F32(ColumnData::new(
                    nodes
                        .iter()
                        .map(|node| node.total / start_total * 100.0)
                        .collect(),
                )),
            ),
        ],
    )
}

fn link_dataset(ribbons: &[SankeyRibbon], nodes: &[SankeyLayoutNode], start_total: f32) -> Dataset {
    Dataset::new(
        LINK_DATASET,
        1,
        vec![
            (
                "link".into(),
                Column::Utf8(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| {
                            Arc::<str>::from(format!("{} to {}", ribbon.source, ribbon.target))
                        })
                        .collect(),
                )),
            ),
            (
                "source".into(),
                Column::Utf8(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| Arc::<str>::from(display_node(nodes, &ribbon.source)))
                        .collect(),
                )),
            ),
            (
                "target".into(),
                Column::Utf8(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| Arc::<str>::from(display_node(nodes, &ribbon.target)))
                        .collect(),
                )),
            ),
            (
                "flow".into(),
                Column::F32(ColumnData::new(
                    ribbons.iter().map(|ribbon| ribbon.value).collect(),
                )),
            ),
            (
                "conversion".into(),
                Column::F32(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| {
                            let source_total = nodes
                                .iter()
                                .find(|node| node.id == ribbon.source)
                                .map(|node| node.total)
                                .unwrap_or(1.0);
                            ribbon.value / source_total * 100.0
                        })
                        .collect(),
                )),
            ),
            (
                "start_share".into(),
                Column::F32(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| ribbon.value / start_total * 100.0)
                        .collect(),
                )),
            ),
            (
                "stage_span".into(),
                Column::F32(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| ribbon.stage_span() as f32)
                        .collect(),
                )),
            ),
            (
                "class".into(),
                Column::Utf8(ColumnData::new(
                    ribbons
                        .iter()
                        .map(|ribbon| Arc::<str>::from(ribbon.class.clone()))
                        .collect(),
                )),
            ),
        ],
    )
}

fn display_node(nodes: &[SankeyLayoutNode], id: &str) -> String {
    nodes
        .iter()
        .find(|node| node.id == id)
        .map(|node| node.label.clone())
        .unwrap_or_else(|| id.to_string())
}

fn layout_node_lookup(nodes: &[SankeyLayoutNode]) -> AHashMap<&str, usize> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect()
}

fn infer_node_order(flows: &[SankeyFlow]) -> Vec<String> {
    let mut seen = AHashSet::new();
    let mut nodes = Vec::new();
    for flow in flows {
        for id in [&flow.source, &flow.target] {
            if seen.insert(id.clone()) {
                nodes.push(id.clone());
            }
        }
    }
    nodes
}

fn infer_node_stages(node_ids: &[String], flows: &[SankeyFlow]) -> AHashMap<String, usize> {
    let mut stages: AHashMap<String, usize> = node_ids.iter().map(|id| (id.clone(), 0)).collect();

    for _ in 0..node_ids.len() {
        let mut changed = false;
        for flow in flows {
            let source_stage = stages
                .get(flow.source.as_str())
                .copied()
                .unwrap_or_default();
            let target_stage = stages
                .get(flow.target.as_str())
                .copied()
                .unwrap_or_default();
            let next_stage = source_stage.saturating_add(1);
            if next_stage > target_stage {
                stages.insert(flow.target.clone(), next_stage);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    stages
}

fn humanize_id(id: &str) -> String {
    let label = id.replace(['_', '-'], " ");
    let mut output = String::with_capacity(label.len());
    let mut capitalize = true;
    for ch in label.chars() {
        if ch.is_whitespace() {
            capitalize = true;
            output.push(ch);
        } else if capitalize {
            output.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn node_palette(index: usize) -> [f32; 3] {
    const COLORS: [[f32; 3]; 8] = [
        [0.10, 0.42, 0.76],
        [0.18, 0.63, 0.68],
        [0.32, 0.68, 0.51],
        [0.49, 0.63, 0.42],
        [0.71, 0.52, 0.28],
        [0.62, 0.45, 0.76],
        [0.83, 0.42, 0.34],
        [0.26, 0.34, 0.49],
    ];
    COLORS[index % COLORS.len()]
}

fn link_palette(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 4]; 8] = [
        [0.07, 0.45, 0.78, 0.62],
        [0.05, 0.58, 0.64, 0.58],
        [0.12, 0.60, 0.43, 0.54],
        [0.38, 0.53, 0.28, 0.46],
        [0.74, 0.25, 0.20, 0.50],
        [0.50, 0.34, 0.72, 0.52],
        [0.74, 0.48, 0.18, 0.50],
        [0.16, 0.23, 0.34, 0.45],
    ];
    let [r, g, b, a] = COLORS[index % COLORS.len()];
    rgba(r, g, b, a)
}

fn stage_label_tooltip(stage: &SankeyLayoutStage) -> LabelTooltip {
    label_tooltip(
        &stage.label,
        vec![
            ("Detail", stage.detail.clone()),
            ("Stage", format!("{}", stage.index + 1)),
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

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

fn validate_stack_integrity(
    nodes: &[SankeyLayoutNode],
    links: &[SankeyLink],
    ribbons: &[SankeyRibbon],
) -> Result<(), String> {
    let mut incoming = vec![0.0; nodes.len()];
    let mut outgoing = vec![0.0; nodes.len()];
    let index = layout_node_lookup(nodes);

    for link in links {
        let source = index[link.source.as_str()];
        let target = index[link.target.as_str()];
        outgoing[source] += link.value;
        incoming[target] += link.value;
    }

    const EPS: f32 = 0.01;
    for (idx, node) in nodes.iter().enumerate() {
        for (label, total) in [("outgoing", outgoing[idx]), ("incoming", incoming[idx])] {
            if total > node.total + EPS {
                return Err(format!(
                    "node {} {} total {:.2} exceeds node total {:.2}",
                    node.id, label, total, node.total
                ));
            }
        }
    }

    for ribbon in ribbons {
        let expected_height = ribbon.value * nodes[0].flow_scale;
        if (ribbon.source_y1 - ribbon.source_y0 - expected_height).abs() > EPS {
            return Err(format!("source stack gap on {}", ribbon.source));
        }
        if (ribbon.target_y1 - ribbon.target_y0 - expected_height).abs() > EPS {
            return Err(format!("target stack gap on {}", ribbon.target));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 0.01;

    fn assert_near(left: f32, right: f32) {
        assert!(
            (left - right).abs() <= EPS,
            "expected {left:.2} to be within {EPS:.2} of {right:.2}"
        );
    }

    fn demo_spec() -> SankeySpec {
        SankeySpec::new(
            vec![
                SankeyNode::new("website", "Website", 0, [0.10, 0.42, 0.76]),
                SankeyNode::new("organic", "Organic", 1, [0.16, 0.55, 0.84]),
                SankeyNode::new("paid", "Paid", 1, [0.18, 0.63, 0.68]),
                SankeyNode::new("referral", "Referral", 1, [0.32, 0.68, 0.51]),
                SankeyNode::new("trial", "Trial", 2, [0.17, 0.64, 0.74]),
                SankeyNode::new("sales", "Sales", 2, [0.49, 0.63, 0.42]),
                SankeyNode::new("won", "Won", 3, [0.10, 0.64, 0.48]),
                SankeyNode::new("lost", "Lost", 3, [0.83, 0.42, 0.34]),
            ],
            vec![
                SankeyLink::new(
                    "website",
                    "organic",
                    70.0,
                    "source mix",
                    rgba(0.07, 0.45, 0.78, 0.62),
                ),
                SankeyLink::new(
                    "website",
                    "paid",
                    54.0,
                    "source mix",
                    rgba(0.05, 0.58, 0.64, 0.58),
                ),
                SankeyLink::new(
                    "website",
                    "referral",
                    32.0,
                    "source mix",
                    rgba(0.12, 0.60, 0.43, 0.54),
                ),
                SankeyLink::new(
                    "organic",
                    "trial",
                    48.0,
                    "activation",
                    rgba(0.05, 0.48, 0.74, 0.58),
                ),
                SankeyLink::new(
                    "organic",
                    "sales",
                    22.0,
                    "sales assist",
                    rgba(0.38, 0.53, 0.28, 0.46),
                ),
                SankeyLink::new(
                    "paid",
                    "trial",
                    36.0,
                    "activation",
                    rgba(0.05, 0.58, 0.64, 0.56),
                ),
                SankeyLink::new(
                    "paid",
                    "sales",
                    18.0,
                    "sales assist",
                    rgba(0.38, 0.53, 0.28, 0.46),
                ),
                SankeyLink::new(
                    "referral",
                    "sales",
                    32.0,
                    "partner lift",
                    rgba(0.12, 0.60, 0.43, 0.52),
                ),
                SankeyLink::new(
                    "trial",
                    "won",
                    58.0,
                    "converted",
                    rgba(0.04, 0.56, 0.38, 0.64),
                ),
                SankeyLink::new(
                    "trial",
                    "lost",
                    26.0,
                    "drop-off",
                    rgba(0.74, 0.25, 0.20, 0.50),
                ),
                SankeyLink::new(
                    "sales",
                    "won",
                    34.0,
                    "converted",
                    rgba(0.04, 0.56, 0.38, 0.58),
                ),
                SankeyLink::new(
                    "sales",
                    "lost",
                    38.0,
                    "drop-off",
                    rgba(0.74, 0.25, 0.20, 0.50),
                ),
            ],
        )
    }

    #[test]
    fn sankey_spec_rejects_missing_nodes() {
        let spec = SankeySpec::new(
            vec![SankeyNode::new("a", "A", 0, [0.0, 0.0, 0.0])],
            vec![SankeyLink::new(
                "a",
                "b",
                1.0,
                "x",
                rgba(0.0, 0.0, 0.0, 1.0),
            )],
        );

        assert_eq!(
            spec.layout(ChartSize::new(320, 240)).unwrap_err(),
            SankeyError::MissingNode { id: "b".into() }
        );
    }

    #[test]
    fn sankey_layout_scales_to_small_heights() {
        let layout = demo_spec().layout(ChartSize::new(620, 220)).unwrap();

        assert!(layout.flow_scale < 1.0);
        for node in &layout.nodes {
            assert!(node.y >= SankeyOptions::default().padding_top - EPS);
            assert!(node.y2() <= 220.0 - SankeyOptions::default().padding_bottom + EPS);
        }
    }

    #[test]
    fn sankey_layout_packs_each_node_without_stack_gaps() {
        let spec = demo_spec();
        let layout = spec.layout(ChartSize::new(620, 400)).unwrap();

        validate_stack_integrity(&layout.nodes, &spec.links, &layout.ribbons)
            .expect("valid sankey layout");

        for node in &layout.nodes {
            if let Some((y0, y1)) = stack_extent(&layout.ribbons, &node.id, true) {
                assert_near(y0, node.y);
                assert_near(y1, node.y2());
            }
            if let Some((y0, y1)) = stack_extent(&layout.ribbons, &node.id, false) {
                assert_near(y0, node.y);
                assert_near(y1, node.y2());
            }
        }
    }

    #[test]
    fn sankey_builds_chart_with_semantic_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(620, 400))
            .unwrap();

        assert!(!chart.scene().layers.is_empty());
        assert!(!chart.scene().guides.is_empty());
        assert!(!chart.snap_targets().is_empty());
    }

    #[test]
    fn sankey_from_flows_infers_nodes_classes_and_stages() {
        let spec = SankeySpec::from_flows(vec![
            SankeyFlow::new("website", "organic", 70.0).with_class("source mix"),
            SankeyFlow::new("organic", "trial", 48.0).with_class("activation"),
            SankeyFlow::new("trial", "lost", 12.0),
            SankeyFlow::new("organic", "lost", 22.0).with_class("drop-off"),
        ]);

        assert_eq!(spec.nodes.len(), 4);
        assert_eq!(spec.nodes[0].label, "Website");
        assert_eq!(spec.nodes[1].stage, 1);
        assert_eq!(spec.nodes[2].stage, 2);
        assert_eq!(spec.nodes[3].stage, 3);
        assert_eq!(spec.links[2].class, "flow");

        let layout = spec.layout(ChartSize::new(620, 400)).unwrap();
        let skip = layout
            .ribbons
            .iter()
            .find(|ribbon| ribbon.source == "organic" && ribbon.target == "lost")
            .unwrap();
        assert_eq!(skip.stage_span(), 2);
    }

    #[test]
    fn sankey_from_flows_accepts_partially_aggregated_nodes() {
        let spec = SankeySpec::from_flows(vec![
            SankeyFlow::new("in", "middle", 10.0),
            SankeyFlow::new("middle", "out", 3.0),
        ]);

        let layout = spec.layout(ChartSize::new(360, 240)).unwrap();
        validate_stack_integrity(&layout.nodes, &spec.links, &layout.ribbons)
            .expect("partial node totals should be valid");
    }

    #[test]
    fn sankey_from_flows_builds_chart_with_defaults() {
        let chart =
            SankeySpec::from_flows(vec![SankeyFlow::new("start_node", "end_node", 10.0)
                .with_color(rgba(0.2, 0.4, 0.7, 0.6))])
            .try_build_chart(Workspace::new(), ChartSize::new(320, 220))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.snap_targets().is_empty());
    }

    fn stack_extent(ribbons: &[SankeyRibbon], node: &str, source_side: bool) -> Option<(f32, f32)> {
        let mut ys = Vec::new();
        for ribbon in ribbons {
            if source_side && ribbon.source == node {
                ys.push(ribbon.source_y0);
                ys.push(ribbon.source_y1);
            }
            if !source_side && ribbon.target == node {
                ys.push(ribbon.target_y0);
                ys.push(ribbon.target_y1);
            }
        }
        if ys.is_empty() {
            None
        } else {
            Some((
                ys.iter().copied().fold(f32::INFINITY, f32::min),
                ys.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            ))
        }
    }
}
