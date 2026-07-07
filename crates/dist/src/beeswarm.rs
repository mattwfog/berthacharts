//! Beeswarm: every sample as a non-overlapping dot. Useful when a boxplot
//! summary hides multi-modal structure or you want to see individual outliers.
//!
//! ## Layout algorithm
//!
//! Dots are placed group by group. Within a group, samples are processed in
//! ascending value order (so their target screen `y` is monotonic) and each
//! dot is given the `x` closest to the group centre that clears every
//! already-placed neighbour by at least `2·radius + 1` px. Each nearby
//! neighbour forbids a horizontal interval around its `x`; the new dot takes
//! the nearest free position outside all of them. Because the scan is
//! value-sorted it stops as soon as a neighbour is more than one separation
//! away in `y`, and it is capped so a dense spike of near-identical values
//! stays bounded rather than running away. Positions are clamped to
//! `±(max_offset_ratio · slot)`, so the swarm never overflows its slot; when a
//! band saturates, the closest in-bounds position is used.
//!
//! Non-finite samples are rejected at build time with
//! [`BeeswarmError::NonFiniteValue`].
//!
//! ## Example
//!
//! ```
//! use berthacharts_dist::beeswarm::{BeeswarmGroup, BeeswarmSpec};
//! use berthacharts_dist::core::{ChartSize, Workspace};
//!
//! let spec = BeeswarmSpec::new(vec![
//!     BeeswarmGroup::new("A", (1..=60).map(|i| (i % 12) as f32).collect()),
//! ]);
//! let chart = spec
//!     .try_build_chart(Workspace::new(), ChartSize::new(480, 320))
//!     .expect("valid beeswarm");
//! assert_eq!(chart.scene().layers.len(), 1);
//! ```

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData, CoordId, Dataset,
    DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LinearScale, Mark, MarkId, PickCtx, PickHit, PointPrim, Rect,
    Scale, ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, TessellateCtx, TooltipField,
    TooltipGuide, Workspace,
};

const GROUP_DATASET: DatasetId = DatasetId::new(0);
const SWARM_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One group of samples for a beeswarm.
#[derive(Debug, Clone, PartialEq)]
pub struct BeeswarmGroup {
    /// Display label.
    pub label: String,
    /// Sample values.
    pub values: Vec<f32>,
    /// Premultiplied RGBA fill for each dot.
    pub color: [f32; 4],
}

impl BeeswarmGroup {
    /// Build a group with default colour.
    #[must_use]
    pub fn new(label: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
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

/// Beeswarm configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeeswarmOptions {
    /// Plot padding (pixels).
    pub padding: f32,
    /// Dot radius (pixels).
    pub dot_radius: f32,
    /// Maximum half-width fraction of slot (0..1). Dots fan out within this.
    pub max_offset_ratio: f32,
}

impl Default for BeeswarmOptions {
    fn default() -> Self {
        Self {
            padding: 30.0,
            dot_radius: 3.0,
            max_offset_ratio: 0.45,
        }
    }
}

/// Beeswarm chart spec.
#[derive(Debug, Clone)]
pub struct BeeswarmSpec {
    groups: Vec<BeeswarmGroup>,
    options: BeeswarmOptions,
}

impl BeeswarmSpec {
    /// Build a beeswarm spec.
    #[must_use]
    pub fn new(groups: Vec<BeeswarmGroup>) -> Self {
        Self {
            groups,
            options: BeeswarmOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: BeeswarmOptions) -> Self {
        self.options = options;
        self
    }

    /// Validate the groups without building a chart.
    ///
    /// Rejects an empty spec, empty groups, empty labels, and any non-finite
    /// sample value.
    pub fn validate(&self) -> Result<(), BeeswarmError> {
        if self.groups.is_empty() {
            return Err(BeeswarmError::Empty);
        }
        for (i, g) in self.groups.iter().enumerate() {
            if g.label.trim().is_empty() {
                return Err(BeeswarmError::EmptyLabel(i));
            }
            if g.values.is_empty() {
                return Err(BeeswarmError::EmptyGroup(i));
            }
            for &v in &g.values {
                if !v.is_finite() {
                    return Err(BeeswarmError::NonFiniteValue {
                        label: g.label.clone(),
                        value: v,
                    });
                }
            }
        }
        Ok(())
    }

    /// Compute the reusable beeswarm layout without building a chart.
    pub fn layout(&self, size: ChartSize) -> Result<BeeswarmLayout, BeeswarmError> {
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
    ) -> Result<Chart, BeeswarmError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }
}

/// Errors during beeswarm build.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BeeswarmError {
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

impl fmt::Display for BeeswarmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "beeswarm has no groups"),
            Self::EmptyGroup(i) => write!(f, "group at index {i} has no samples"),
            Self::EmptyLabel(i) => write!(f, "group at index {i} has an empty label"),
            Self::NonFiniteValue { label, value } => {
                write!(f, "beeswarm value for `{label}` is not finite: {value}")
            }
        }
    }
}

impl std::error::Error for BeeswarmError {}

/// One dot in the swarm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwarmDot {
    /// Screen x.
    pub x: f32,
    /// Screen y.
    pub y: f32,
    /// Dot colour.
    pub color: [f32; 4],
    /// Index of source value within its group.
    pub source_row: usize,
    /// Original sample value.
    pub value: f32,
}

/// Computed beeswarm layout.
#[derive(Debug, Clone, PartialEq)]
pub struct BeeswarmLayout {
    /// Per-group dot list.
    pub groups: Vec<BeeswarmGroupLayout>,
}

/// Per-group swarm layout.
#[derive(Debug, Clone, PartialEq)]
pub struct BeeswarmGroupLayout {
    /// Display label.
    pub label: String,
    /// Centre x.
    pub center_x: f32,
    /// Positioned dots.
    pub dots: Vec<SwarmDot>,
}

impl ChartSpec for BeeswarmSpec {
    type Error = BeeswarmError;

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

        let mark: Arc<dyn Mark> = Arc::new(BeeswarmMark::new(
            SWARM_MARK,
            layout.clone(),
            self.options.dot_radius,
        ));
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
                SWARM_MARK,
                GROUP_DATASET,
                vec![
                    TooltipField::new("Group", "group").as_label(),
                    TooltipField::new("Value", "value").as_number(2),
                    TooltipField::new("Sample", "sample").as_integer(),
                ],
            )
            .with_title_column("group"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(group_labels(&layout))
                .with_collision_padding(3.0)
                .with_max_visible(layout.groups.len()),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, self.options.dot_radius))
                .with_name("beeswarm samples"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn compute_layout(
    groups: &[BeeswarmGroup],
    options: &BeeswarmOptions,
    plot: Rect,
) -> BeeswarmLayout {
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
    let max_off = (slot * options.max_offset_ratio).max(0.0);
    let r = options.dot_radius.max(0.1);
    let min_sep = 2.0 * r + 1.0;

    let group_layouts: Vec<BeeswarmGroupLayout> = groups
        .iter()
        .enumerate()
        .map(|(gi, g)| {
            let center_x = inner.x + (gi as f32 + 0.5) * slot;
            // Sort by value so screen y is monotonic; that lets the collision
            // scan stop early and keeps the swarm symmetric.
            let mut indexed: Vec<(usize, f32)> = g.values.iter().copied().enumerate().collect();
            indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let mut placed: Vec<SwarmDot> = Vec::with_capacity(indexed.len());
            for (row, value) in indexed {
                let y = map_y(value);
                let x = place_dot(center_x, y, max_off, min_sep, &placed);
                placed.push(SwarmDot {
                    x,
                    y,
                    color: g.color,
                    source_row: row,
                    value,
                });
            }
            BeeswarmGroupLayout {
                label: g.label.clone(),
                center_x,
                dots: placed,
            }
        })
        .collect();

    BeeswarmLayout {
        groups: group_layouts,
    }
}

/// Upper bound on neighbours consulted when placing a dot. A dense spike of
/// near-identical values would otherwise scan the whole group; the band is
/// already saturated well before this many, so the cap keeps placement O(n)
/// without changing the visible result.
const MAX_NEIGHBOR_SCAN: usize = 128;

/// Place one dot: return the x nearest `center_x` (within `±max_off`) that
/// clears every nearby placed neighbour by `min_sep`. `placed` is ordered by
/// descending y (samples arrive value-sorted), so the scan stops once a
/// neighbour is at least `min_sep` away in y.
fn place_dot(center_x: f32, y: f32, max_off: f32, min_sep: f32, placed: &[SwarmDot]) -> f32 {
    let mut intervals: Vec<(f32, f32)> = Vec::new();
    for d in placed.iter().rev().take(MAX_NEIGHBOR_SCAN) {
        let dy = (y - d.y).abs();
        if dy >= min_sep {
            break;
        }
        // Horizontal half-width another dot forbids at this y offset.
        let half = (min_sep * min_sep - dy * dy).max(0.0).sqrt();
        intervals.push((d.x - half, d.x + half));
    }
    nearest_free_x(
        center_x,
        center_x - max_off,
        center_x + max_off,
        &mut intervals,
    )
}

/// Choose the x closest to `center`, clamped to `[min_x, max_x]`, avoiding
/// every forbidden interval. When the band is fully saturated the closest
/// in-bounds position is returned (accepting overlap) so the swarm stays
/// bounded.
fn nearest_free_x(center: f32, min_x: f32, max_x: f32, intervals: &mut [(f32, f32)]) -> f32 {
    if intervals.is_empty() {
        return center.clamp(min_x, max_x);
    }
    intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    // Candidate free positions: the centre plus each interval edge, all clamped
    // into the allowed band. Rank by (is-free, distance-to-centre).
    let mut best: Option<f32> = None;
    let mut best_key: Option<(bool, f32)> = None;
    let mut consider = |x: f32| {
        let x = x.clamp(min_x, max_x);
        let occupied = intervals.iter().any(|&(lo, hi)| x > lo && x < hi);
        let key = (occupied, (x - center).abs());
        if best_key.is_none_or(|bk| key < bk) {
            best_key = Some(key);
            best = Some(x);
        }
    };
    consider(center);
    for &(lo, hi) in intervals.iter() {
        consider(lo);
        consider(hi);
    }
    best.unwrap_or_else(|| center.clamp(min_x, max_x))
}

fn group_dataset(layout: &BeeswarmLayout) -> Dataset {
    let mut group: Vec<Arc<str>> = Vec::new();
    let mut sample: Vec<i64> = Vec::new();
    let mut value: Vec<f32> = Vec::new();
    let mut x: Vec<f32> = Vec::new();
    let mut y: Vec<f32> = Vec::new();
    for g in &layout.groups {
        for dot in &g.dots {
            group.push(Arc::from(g.label.as_str()));
            sample.push(dot.source_row as i64 + 1);
            value.push(dot.value);
            x.push(dot.x);
            y.push(dot.y);
        }
    }
    Dataset::new(
        GROUP_DATASET,
        1,
        vec![
            ("group".to_string(), Column::Utf8(ColumnData::new(group))),
            ("sample".to_string(), Column::I64(ColumnData::new(sample))),
            ("value".to_string(), Column::F32(ColumnData::new(value))),
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

fn group_labels(layout: &BeeswarmLayout) -> Vec<LabelItem> {
    layout
        .groups
        .iter()
        .filter_map(|group| {
            let y = group
                .dots
                .iter()
                .map(|dot| dot.y)
                .fold(f32::NEG_INFINITY, f32::max);
            y.is_finite().then(|| {
                LabelItem::new(group.center_x, y + 12.0, group.label.clone())
                    .with_anchor(LabelAnchor::Bottom)
                    .with_kind(LabelKind::Column)
                    .with_priority(LabelPriority::Important)
            })
        })
        .collect()
}

fn snap_targets(layout: &BeeswarmLayout, radius: f32) -> Vec<SnapTarget> {
    layout
        .groups
        .iter()
        .flat_map(|group| {
            group.dots.iter().map(move |dot| {
                SnapTarget::new(dot.x, dot.y, SnapKind::Point)
                    .with_radius((radius + 4.0).clamp(5.0, 12.0))
                    .with_label(format!("{} sample {}", group.label, dot.source_row + 1))
                    .with_priority(2)
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct BeeswarmMark {
    id: MarkId,
    layout: BeeswarmLayout,
    radius: f32,
}

impl BeeswarmMark {
    fn new(id: MarkId, layout: BeeswarmLayout, radius: f32) -> Self {
        Self { id, layout, radius }
    }
}

impl Mark for BeeswarmMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        let total: usize = self.layout.groups.iter().map(|g| g.dots.len()).sum();
        h ^= total as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut points = Vec::new();
        for g in &self.layout.groups {
            for d in &g.dots {
                points.push(PointPrim {
                    x: d.x,
                    y: d.y,
                    r: self.radius,
                    shape: 0,
                    fill: d.color,
                    stroke: [1.0, 1.0, 1.0, 0.5],
                    stroke_width: 0.5,
                });
            }
        }
        Geometry::Points(points)
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        let mut best: Option<(usize, f32)> = None;
        let mut flat_row = 0usize;
        for g in &self.layout.groups {
            for d in &g.dots {
                let dx = px - d.x;
                let dy = py - d.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= self.radius + 2.0 && best.is_none_or(|(_, bd)| dist < bd) {
                    best = Some((flat_row, dist));
                }
                flat_row += 1;
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
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for g in &self.layout.groups {
            for d in &g.dots {
                min_x = min_x.min(d.x - self.radius);
                min_y = min_y.min(d.y - self.radius);
                max_x = max_x.max(d.x + self.radius);
                max_y = max_y.max(d.y + self.radius);
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

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, Guide, SnapKind};

    #[test]
    fn empty_spec_rejected() {
        let r = BeeswarmSpec::new(vec![]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(BeeswarmError::Empty)));
    }

    #[test]
    fn empty_group_rejected() {
        let r = BeeswarmSpec::new(vec![BeeswarmGroup::new("a", vec![])]).build_chart(
            berthacharts_core::Workspace::new(),
            ChartSize::new(400, 300),
        );
        assert!(matches!(r, Err(BeeswarmError::EmptyGroup(0))));
    }

    #[test]
    fn every_sample_yields_dot() {
        let g = vec![BeeswarmGroup::new(
            "a",
            (1..=20).map(|i| i as f32).collect(),
        )];
        let layout = compute_layout(
            &g,
            &BeeswarmOptions::default(),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert_eq!(layout.groups[0].dots.len(), 20);
    }

    #[test]
    fn identical_values_pack_without_overlap() {
        let opts = BeeswarmOptions::default();
        let layout = compute_layout(
            &[BeeswarmGroup::new("a", vec![5.0; 20])],
            &opts,
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let dots = &layout.groups[0].dots;
        assert_eq!(dots.len(), 20);
        let min_sep = 2.0 * opts.dot_radius + 1.0;
        for i in 0..dots.len() {
            for j in (i + 1)..dots.len() {
                let dx = dots[i].x - dots[j].x;
                let dy = dots[i].y - dots[j].y;
                let dist = (dx * dx + dy * dy).sqrt();
                assert!(dist >= min_sep - 0.05, "dots {i},{j} overlap: dist {dist}");
            }
        }
        let center = layout.groups[0].center_x;
        let max_off = 340.0 * opts.max_offset_ratio;
        for d in dots {
            assert!(
                (d.x - center).abs() <= max_off + 0.05,
                "dot escaped slot: {}",
                d.x
            );
        }
    }

    #[test]
    fn dense_spike_stays_bounded_and_finite() {
        let opts = BeeswarmOptions::default();
        let layout = compute_layout(
            &[BeeswarmGroup::new("a", vec![5.0; 500])],
            &opts,
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        let dots = &layout.groups[0].dots;
        assert_eq!(dots.len(), 500);
        let center = layout.groups[0].center_x;
        let max_off = 340.0 * opts.max_offset_ratio;
        for d in dots {
            assert!(d.x.is_finite() && d.y.is_finite());
            assert!(
                (d.x - center).abs() <= max_off + 0.05,
                "dot escaped slot: {}",
                d.x
            );
        }
    }

    #[test]
    fn non_finite_value_rejected() {
        let err = BeeswarmSpec::new(vec![BeeswarmGroup::new("a", vec![1.0, f32::NAN])])
            .try_build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(400, 300),
            )
            .unwrap_err();
        assert!(matches!(err, BeeswarmError::NonFiniteValue { .. }));
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        for size in [
            ChartSize::new(0, 0),
            ChartSize::new(1, 1),
            ChartSize::new(0, 300),
            ChartSize::new(400, 0),
        ] {
            let _ = BeeswarmSpec::new(vec![BeeswarmGroup::new("a", vec![1.0, 2.0, 3.0])])
                .try_build_chart(berthacharts_core::Workspace::new(), size);
        }
    }

    #[test]
    fn layout_matches_dot_count() {
        let layout = BeeswarmSpec::new(vec![BeeswarmGroup::new("a", vec![1.0, 2.0, 3.0])])
            .layout(ChartSize::new(400, 300))
            .expect("layout");
        assert_eq!(layout.groups[0].dots.len(), 3);
    }

    #[test]
    fn build_chart_succeeds() {
        let groups = vec![
            BeeswarmGroup::new("A", (1..=30).map(|i| i as f32).collect()),
            BeeswarmGroup::new("B", (5..=35).map(|i| i as f32 * 1.1).collect()),
        ];
        let chart = BeeswarmSpec::new(groups)
            .build_chart(
                berthacharts_core::Workspace::new(),
                ChartSize::new(600, 400),
            )
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn build_chart_exposes_dot_tooltips_and_snap_targets() {
        let workspace = berthacharts_core::Workspace::new();
        let chart = BeeswarmSpec::new(vec![
            BeeswarmGroup::new("A", vec![1.0, 2.0]),
            BeeswarmGroup::new("B", vec![3.0]),
        ])
        .build_chart(workspace.clone(), ChartSize::new(360, 240))
        .expect("chart");

        let tooltip = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Tooltip(tooltip) if tooltip.mark == SWARM_MARK => Some(tooltip),
                _ => None,
            })
            .expect("dot tooltip guide");
        assert_eq!(tooltip.title_column.as_deref(), Some("group"));
        assert_eq!(tooltip.fields.len(), 3);
        assert_eq!(tooltip.fields[1].column, "value");

        let dataset = workspace.dataset(GROUP_DATASET).expect("swarm dataset");
        assert_eq!(dataset.len(), 3);
        let group = match dataset.column("group").expect("group").as_ref() {
            Column::Utf8(values) => values,
            other => panic!("expected utf8 group column, got {}", other.dtype()),
        };
        assert_eq!(group.values[2].as_ref(), "B");

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 3);
        assert!(targets.iter().all(|target| target.kind == SnapKind::Point));
        assert!(targets
            .iter()
            .any(|target| target.label.as_deref() == Some("B sample 1")));
    }
}
