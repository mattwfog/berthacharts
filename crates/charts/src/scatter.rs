//! Scatter plot spec with grouped points, trend line, outlier labels,
//! point-level tooltips, legend, and snap targets.

use std::sync::Arc;

use berthacharts_core::{
    AxisGuide, AxisOrient, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData,
    CoordId, Dataset, DatasetId, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem, LinePrim, LinearScale,
    Mark, MarkId, NumberChannel, PointPrim, Rect, RectMark, Scale, ScaleId, Scene, SnapKind,
    SnapTarget, SnapTargetSet, TooltipField, TooltipGuide, Workspace,
};

use crate::mark::{LineCollectionMark, PointCollectionMark};

const DATASET: DatasetId = DatasetId::new(0);
const SCRATCH_DATASET: DatasetId = DatasetId::new(99);
const POINT_MARK: MarkId = MarkId::new(1);
const TREND_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One point in a scatter plot.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterDatum {
    /// Display label.
    pub label: String,
    /// X value.
    pub x: f32,
    /// Y value.
    pub y: f32,
    /// Optional group.
    pub group: String,
    /// Optional point radius override.
    pub radius: Option<f32>,
}

impl ScatterDatum {
    /// Build one scatter datum.
    #[must_use]
    pub fn new(label: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            label: label.into(),
            x,
            y,
            group: "points".to_string(),
            radius: None,
        }
    }

    /// Set the point group.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Set a point radius override.
    #[must_use]
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

/// Layout and guide options for a scatter plot.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterPlotOptions {
    /// Left plot padding.
    pub padding_left: f32,
    /// Right plot padding.
    pub padding_right: f32,
    /// Top plot padding.
    pub padding_top: f32,
    /// Bottom plot padding.
    pub padding_bottom: f32,
    /// Optional fixed x domain.
    pub x_domain: Option<(f32, f32)>,
    /// Optional fixed y domain.
    pub y_domain: Option<(f32, f32)>,
    /// X-axis label.
    pub x_axis_label: String,
    /// Y-axis label.
    pub y_axis_label: String,
    /// X tick-count hint.
    pub x_tick_count: usize,
    /// Y tick-count hint.
    pub y_tick_count: usize,
    /// Default point radius.
    pub point_radius: f32,
    /// Whether to draw a least-squares trend line.
    pub show_trend: bool,
    /// Maximum outlier labels to show.
    pub max_visible_labels: usize,
}

impl Default for ScatterPlotOptions {
    fn default() -> Self {
        Self {
            padding_left: 60.0,
            padding_right: 42.0,
            padding_top: 44.0,
            padding_bottom: 58.0,
            x_domain: None,
            y_domain: None,
            x_axis_label: "X".to_string(),
            y_axis_label: "Y".to_string(),
            x_tick_count: 5,
            y_tick_count: 5,
            point_radius: 5.2,
            show_trend: true,
            max_visible_labels: 6,
        }
    }
}

/// Reusable grouped scatter plot specification.
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterPlotSpec {
    /// Points in author order.
    pub data: Vec<ScatterDatum>,
    /// Layout and guide options.
    pub options: ScatterPlotOptions,
}

impl ScatterPlotSpec {
    /// Build a scatter plot spec.
    #[must_use]
    pub fn new(data: Vec<ScatterDatum>) -> Self {
        Self {
            data,
            options: ScatterPlotOptions::default(),
        }
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: ScatterPlotOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute headline summary values.
    #[must_use]
    pub fn summary(&self) -> ScatterPlotSummary {
        let (slope, intercept) = fit_trend(&self.data);
        ScatterPlotSummary {
            points: self.data.len(),
            groups: group_order(&self.data).len(),
            correlation: correlation(&self.data),
            slope,
            intercept,
        }
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, ScatterPlotError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), ScatterPlotError> {
        if self.data.is_empty() {
            return Err(ScatterPlotError::EmptyData);
        }
        for datum in &self.data {
            if datum.label.trim().is_empty() {
                return Err(ScatterPlotError::EmptyLabel);
            }
            if datum.group.trim().is_empty() {
                return Err(ScatterPlotError::EmptyGroup);
            }
            if !datum.x.is_finite()
                || !datum.y.is_finite()
                || datum
                    .radius
                    .is_some_and(|radius| !radius.is_finite() || radius <= 0.0)
            {
                return Err(ScatterPlotError::InvalidPoint {
                    label: datum.label.clone(),
                    x: datum.x,
                    y: datum.y,
                });
            }
        }
        Ok(())
    }

    fn plot_area(&self, size: ChartSize) -> Rect {
        let width = size.width as f32;
        let height = size.height as f32;
        // Upper bounds floored at 0 — see bar.rs plot_area: inverted clamp
        // bounds panic, and a panic poisons the wasm handle.
        let left = self.options.padding_left.clamp(0.0, (width - 1.0).max(0.0));
        let top = self.options.padding_top.clamp(0.0, (height - 1.0).max(0.0));
        let right = self
            .options
            .padding_right
            .clamp(0.0, (width - left - 1.0).max(0.0));
        let bottom = self
            .options
            .padding_bottom
            .clamp(0.0, (height - top - 1.0).max(0.0));
        Rect::new(
            left,
            top,
            (width - left - right).max(1.0),
            (height - top - bottom).max(1.0),
        )
    }
}

/// Summary statistics for a scatter plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterPlotSummary {
    /// Number of points.
    pub points: usize,
    /// Number of groups.
    pub groups: usize,
    /// Pearson correlation coefficient.
    pub correlation: f32,
    /// Least-squares trend slope.
    pub slope: f32,
    /// Least-squares trend intercept.
    pub intercept: f32,
}

/// Error building a scatter plot.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ScatterPlotError {
    /// No points were supplied.
    EmptyData,
    /// A point label was empty.
    EmptyLabel,
    /// A group label was empty.
    EmptyGroup,
    /// A point coordinate or radius was invalid.
    InvalidPoint {
        /// Point label.
        label: String,
        /// Bad x.
        x: f32,
        /// Bad y.
        y: f32,
    },
}

impl std::fmt::Display for ScatterPlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyData => write!(f, "scatter plot requires at least one point"),
            Self::EmptyLabel => write!(f, "scatter point labels cannot be empty"),
            Self::EmptyGroup => write!(f, "scatter point groups cannot be empty"),
            Self::InvalidPoint { label, x, y } => {
                write!(f, "scatter point `{label}` is invalid: ({x}, {y})")
            }
        }
    }
}

impl std::error::Error for ScatterPlotError {}

impl ChartSpec for ScatterPlotSpec {
    type Error = ScatterPlotError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let plot = self.plot_area(size);
        let groups = group_order(&self.data);
        let x_domain = self
            .options
            .x_domain
            .unwrap_or_else(|| nice_domain(domain_of(self.data.iter().map(|datum| datum.x))));
        let y_domain = self
            .options
            .y_domain
            .unwrap_or_else(|| nice_domain(domain_of(self.data.iter().map(|datum| datum.y))));
        let x_scale = LinearScale::new(
            (f64::from(x_domain.0), f64::from(x_domain.1)),
            (plot.x, plot.x + plot.w),
        );
        let y_scale = LinearScale::new(
            (f64::from(y_domain.0), f64::from(y_domain.1)),
            (plot.y + plot.h, plot.y),
        );
        let x_ticks = x_scale.ticks(self.options.x_tick_count.max(2));
        let y_ticks = y_scale.ticks(self.options.y_tick_count.max(2));

        workspace.upsert_scale(X_SCALE, Arc::new(x_scale.clone()) as Arc<dyn Scale>);
        workspace.upsert_scale(Y_SCALE, Arc::new(y_scale.clone()) as Arc<dyn Scale>);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(scatter_dataset(&self.data));

        let mut marks: Vec<Arc<dyn Mark>> = y_ticks
            .into_iter()
            .enumerate()
            .map(|(index, tick)| {
                Arc::new(RectMark::new(
                    MarkId::new(100 + index as u64),
                    SCRATCH_DATASET,
                    NumberChannel::Constant(plot.x),
                    NumberChannel::Constant(tick.position),
                    NumberChannel::Constant(plot.x + plot.w),
                    NumberChannel::Constant(tick.position + 1.0),
                    [0.90, 0.92, 0.96, 1.0],
                )) as Arc<dyn Mark>
            })
            .collect();
        marks.extend(x_ticks.into_iter().enumerate().map(|(index, tick)| {
            Arc::new(RectMark::new(
                MarkId::new(140 + index as u64),
                SCRATCH_DATASET,
                NumberChannel::Constant(tick.position),
                NumberChannel::Constant(plot.y),
                NumberChannel::Constant(tick.position + 1.0),
                NumberChannel::Constant(plot.y + plot.h),
                [0.94, 0.96, 0.98, 1.0],
            )) as Arc<dyn Mark>
        }));

        if self.options.show_trend && self.data.len() >= 2 {
            marks.push(Arc::new(LineCollectionMark::new(
                TREND_MARK,
                vec![trend_line(&self.data, &x_scale, &y_scale, x_domain)],
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
            )));
        }

        marks.push(Arc::new(PointCollectionMark::new(
            POINT_MARK,
            point_primitives(
                &self.data,
                &groups,
                &x_scale,
                &y_scale,
                self.options.point_radius,
            ),
            Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
        )));

        let mut scene = Scene::new(size.viewport_with_plot_area(plot));
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks,
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Axis(
            AxisGuide::new(X_SCALE, AxisOrient::Bottom)
                .with_label(self.options.x_axis_label.clone())
                .with_tick_count(self.options.x_tick_count),
        ));
        scene.guides.push(Guide::Axis(
            AxisGuide::new(Y_SCALE, AxisOrient::Left)
                .with_label(self.options.y_axis_label.clone())
                .with_tick_count(self.options.y_tick_count),
        ));
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                POINT_MARK,
                DATASET,
                vec![
                    TooltipField::new("Group", "group").as_label(),
                    TooltipField::new("X", "x").as_number(1),
                    TooltipField::new("Y", "y").as_number(1),
                    TooltipField::new("Residual", "residual").as_signed_number(1),
                ],
            )
            .with_title_column("label"),
        ));
        let labels = outlier_labels(
            &self.data,
            &x_scale,
            &y_scale,
            self.options.max_visible_labels,
        );
        let label_count = labels.len().min(self.options.max_visible_labels);
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(4.0)
                .with_max_visible(label_count),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(
                groups
                    .iter()
                    .enumerate()
                    .map(|(index, name)| LegendItem::new(name.clone(), palette(index)))
                    .collect(),
            )
            .with_title("Group")
            .with_anchor(LegendAnchor::Bottom),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(snap_targets(&self.data, &x_scale, &y_scale))
                    .with_name("scatter points"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn scatter_dataset(data: &[ScatterDatum]) -> Dataset {
    let (slope, intercept) = fit_trend(data);
    Dataset::new(
        DATASET,
        1,
        vec![
            (
                "x".into(),
                Column::F32(ColumnData::new(data.iter().map(|datum| datum.x).collect())),
            ),
            (
                "y".into(),
                Column::F32(ColumnData::new(data.iter().map(|datum| datum.y).collect())),
            ),
            (
                "residual".into(),
                Column::F32(ColumnData::new(
                    data.iter()
                        .map(|datum| datum.y - (slope * datum.x + intercept))
                        .collect(),
                )),
            ),
            (
                "label".into(),
                Column::Utf8(ColumnData::new(
                    data.iter()
                        .map(|datum| Arc::<str>::from(datum.label.clone()))
                        .collect(),
                )),
            ),
            (
                "group".into(),
                Column::Utf8(ColumnData::new(
                    data.iter()
                        .map(|datum| Arc::<str>::from(datum.group.clone()))
                        .collect(),
                )),
            ),
        ],
    )
}

fn point_primitives(
    data: &[ScatterDatum],
    groups: &[String],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
    default_radius: f32,
) -> Vec<PointPrim> {
    data.iter()
        .map(|datum| {
            let color = palette(index_of(groups, &datum.group).unwrap_or(0));
            PointPrim {
                x: x_scale.project(f64::from(datum.x)),
                y: y_scale.project(f64::from(datum.y)),
                r: datum.radius.unwrap_or(default_radius),
                shape: index_of(groups, &datum.group).unwrap_or(0) as u32 % 4,
                fill: color,
                stroke: rgba(1.0, 1.0, 1.0, 0.92),
                stroke_width: 1.4,
            }
        })
        .collect()
}

fn trend_line(
    data: &[ScatterDatum],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
    x_domain: (f32, f32),
) -> LinePrim {
    let (slope, intercept) = fit_trend(data);
    LinePrim {
        points: [x_domain.0, x_domain.1]
            .into_iter()
            .map(|x| {
                [
                    x_scale.project(f64::from(x)),
                    y_scale.project(f64::from(slope * x + intercept)),
                ]
            })
            .collect(),
        stroke: rgba(0.14, 0.22, 0.34, 0.55),
        width: 2.0,
        dash: Some(vec![6.0, 5.0]),
        join: 1,
        cap: 1,
    }
}

fn outlier_labels(
    data: &[ScatterDatum],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
    max_visible: usize,
) -> Vec<LabelItem> {
    let (slope, intercept) = fit_trend(data);
    let mut ranked: Vec<&ScatterDatum> = data.iter().collect();
    ranked.sort_by(|a, b| {
        let ar = (a.y - (slope * a.x + intercept)).abs();
        let br = (b.y - (slope * b.x + intercept)).abs();
        br.total_cmp(&ar)
    });
    ranked
        .into_iter()
        .take(max_visible)
        .map(|datum| {
            let residual = datum.y - (slope * datum.x + intercept);
            LabelItem::new(
                x_scale.project(f64::from(datum.x)),
                y_scale.project(f64::from(datum.y)) - 10.0,
                datum.label.clone(),
            )
            .with_detail(format_signed(residual))
            .with_kind(LabelKind::Data)
            .with_priority(LabelPriority::Important)
            .with_anchor(LabelAnchor::Top)
        })
        .collect()
}

fn snap_targets(
    data: &[ScatterDatum],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
) -> Vec<SnapTarget> {
    data.iter()
        .map(|datum| {
            SnapTarget::new(
                x_scale.project(f64::from(datum.x)),
                y_scale.project(f64::from(datum.y)),
                SnapKind::Point,
            )
            .with_radius(datum.radius.unwrap_or(5.2) + 2.0)
            .with_label(datum.label.clone())
            .with_priority(2)
        })
        .collect()
}

fn group_order(data: &[ScatterDatum]) -> Vec<String> {
    let mut groups = Vec::new();
    for datum in data {
        if !groups.contains(&datum.group) {
            groups.push(datum.group.clone());
        }
    }
    groups
}

fn index_of(items: &[String], value: &str) -> Option<usize> {
    items.iter().position(|item| item == value)
}

fn domain_of(values: impl Iterator<Item = f32>) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for value in values {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    (lo, hi)
}

fn nice_domain((lo, hi): (f32, f32)) -> (f32, f32) {
    if !lo.is_finite() || !hi.is_finite() {
        return (0.0, 1.0);
    }
    if (hi - lo).abs() < f32::EPSILON {
        return (lo - 1.0, hi + 1.0);
    }
    let pad = (hi - lo) * 0.08;
    (lo - pad, hi + pad)
}

fn fit_trend(data: &[ScatterDatum]) -> (f32, f32) {
    if data.len() < 2 {
        return (0.0, data.first().map_or(0.0, |datum| datum.y));
    }
    let n = data.len() as f32;
    let x_mean = data.iter().map(|datum| datum.x).sum::<f32>() / n;
    let y_mean = data.iter().map(|datum| datum.y).sum::<f32>() / n;
    let sxx = data
        .iter()
        .map(|datum| {
            let dx = datum.x - x_mean;
            dx * dx
        })
        .sum::<f32>();
    let sxy = data
        .iter()
        .map(|datum| (datum.x - x_mean) * (datum.y - y_mean))
        .sum::<f32>();
    let slope = if sxx.abs() < f32::EPSILON {
        0.0
    } else {
        sxy / sxx
    };
    (slope, y_mean - slope * x_mean)
}

fn correlation(data: &[ScatterDatum]) -> f32 {
    if data.len() < 2 {
        return 0.0;
    }
    let n = data.len() as f32;
    let x_mean = data.iter().map(|datum| datum.x).sum::<f32>() / n;
    let y_mean = data.iter().map(|datum| datum.y).sum::<f32>() / n;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for datum in data {
        let dx = datum.x - x_mean;
        let dy = datum.y - y_mean;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if sxx <= f32::EPSILON || syy <= f32::EPSILON {
        0.0
    } else {
        sxy / (sxx * syy).sqrt()
    }
}

fn palette(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 4]; 6] = [
        [0.06, 0.42, 0.78, 0.86],
        [0.05, 0.58, 0.48, 0.86],
        [0.82, 0.33, 0.27, 0.84],
        [0.55, 0.42, 0.74, 0.84],
        [0.78, 0.52, 0.19, 0.84],
        [0.20, 0.30, 0.44, 0.82],
    ];
    let [r, g, b, a] = COLORS[index % COLORS.len()];
    rgba(r, g, b, a)
}

fn format_signed(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.1}")
    } else {
        format!("{value:.1}")
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_spec() -> ScatterPlotSpec {
        ScatterPlotSpec::new(vec![
            ScatterDatum::new("Alpha", 2.1, 4.0).with_group("baseline"),
            ScatterDatum::new("Beta", 2.8, 5.2).with_group("baseline"),
            ScatterDatum::new("Gamma", 3.4, 6.1).with_group("baseline"),
            ScatterDatum::new("Delta", 4.0, 8.8).with_group("expansion"),
            ScatterDatum::new("Epsilon", 4.7, 9.4).with_group("expansion"),
            ScatterDatum::new("Zeta", 5.5, 11.8).with_group("expansion"),
        ])
    }

    #[test]
    fn scatter_spec_builds_chart_with_guides_and_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(620, 360))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.scene().guides.is_empty());
        assert_eq!(chart.snap_targets().len(), 6);
    }

    #[test]
    fn scatter_summary_computes_groups_and_correlation() {
        let summary = demo_spec().summary();

        assert_eq!(summary.points, 6);
        assert_eq!(summary.groups, 2);
        assert!(summary.correlation > 0.95);
    }

    #[test]
    fn scatter_rejects_invalid_points() {
        let err = ScatterPlotSpec::new(vec![ScatterDatum::new("bad", 1.0, f32::NAN)])
            .try_build_chart(Workspace::new(), ChartSize::new(320, 200))
            .unwrap_err();

        assert!(matches!(err, ScatterPlotError::InvalidPoint { .. }));
    }
}
