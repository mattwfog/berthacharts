//! Multi-series line chart spec with axes, grid, legend, end labels,
//! point-level tooltips, and snap targets.

use std::sync::Arc;

use berthacharts_core::{
    AxisGuide, AxisOrient, CartesianCoord, Chart, ChartSize, ChartSpec, Column, ColumnData,
    CoordId, Dataset, DatasetId, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, LabelTooltip, LabelTooltipRow, Layer, LayerId, LegendAnchor, LegendGuide,
    LegendItem, LinePrim, LinearScale, Mark, MarkId, NumberChannel, Rect, RectMark, Scale, ScaleId,
    Scene, SnapKind, SnapTarget, SnapTargetSet, TooltipField, TooltipGuide, Workspace,
};

use crate::mark::{LineCollectionMark, PointCollectionMark};

const DATASET: DatasetId = DatasetId::new(0);
const SCRATCH_DATASET: DatasetId = DatasetId::new(99);
const LINE_MARK: MarkId = MarkId::new(1);
const POINT_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One observed point in a line chart.
#[derive(Debug, Clone, PartialEq)]
pub struct LineDatum {
    /// Series name.
    pub series: String,
    /// X value.
    pub x: f32,
    /// Y value.
    pub y: f32,
    /// Optional display label for this point.
    pub label: Option<String>,
}

impl LineDatum {
    /// Build one line datum.
    #[must_use]
    pub fn new(series: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            series: series.into(),
            x,
            y,
            label: None,
        }
    }

    /// Set a display label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Layout and guide options for a line chart.
#[derive(Debug, Clone, PartialEq)]
pub struct LineChartOptions {
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
    /// Stroke width.
    pub line_width: f32,
    /// Point radius.
    pub point_radius: f32,
    /// Whether point markers are emitted.
    pub show_points: bool,
}

impl Default for LineChartOptions {
    fn default() -> Self {
        Self {
            padding_left: 60.0,
            padding_right: 78.0,
            padding_top: 44.0,
            padding_bottom: 58.0,
            x_domain: None,
            y_domain: None,
            x_axis_label: "X".to_string(),
            y_axis_label: "Y".to_string(),
            x_tick_count: 5,
            y_tick_count: 5,
            line_width: 2.4,
            point_radius: 4.6,
            show_points: true,
        }
    }
}

/// Reusable multi-series line chart specification.
#[derive(Debug, Clone, PartialEq)]
pub struct LineChartSpec {
    /// Points in author order.
    pub data: Vec<LineDatum>,
    /// Layout and guide options.
    pub options: LineChartOptions,
}

impl LineChartSpec {
    /// Build a line chart spec.
    #[must_use]
    pub fn new(data: Vec<LineDatum>) -> Self {
        Self {
            data,
            options: LineChartOptions::default(),
        }
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: LineChartOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute headline summary values.
    #[must_use]
    pub fn summary(&self) -> LineChartSummary {
        let series = series_order(&self.data);
        let peak = self.data.iter().map(|datum| datum.y).fold(0.0, f32::max);
        let latest = series
            .iter()
            .filter_map(|name| latest_for_series(&self.data, name))
            .map(|datum| datum.y)
            .sum();
        LineChartSummary {
            series: series.len(),
            points: self.data.len(),
            peak,
            latest_total: latest,
        }
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, LineChartError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), LineChartError> {
        if self.data.is_empty() {
            return Err(LineChartError::EmptyData);
        }
        for datum in &self.data {
            if datum.series.trim().is_empty() {
                return Err(LineChartError::EmptySeries);
            }
            if !datum.x.is_finite() || !datum.y.is_finite() {
                return Err(LineChartError::NonFinitePoint {
                    series: datum.series.clone(),
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
        let left = self.options.padding_left.clamp(0.0, width - 1.0);
        let top = self.options.padding_top.clamp(0.0, height - 1.0);
        let right = self.options.padding_right.clamp(0.0, width - left - 1.0);
        let bottom = self.options.padding_bottom.clamp(0.0, height - top - 1.0);
        Rect::new(
            left,
            top,
            (width - left - right).max(1.0),
            (height - top - bottom).max(1.0),
        )
    }
}

/// Summary statistics for a line chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineChartSummary {
    /// Number of series.
    pub series: usize,
    /// Number of points.
    pub points: usize,
    /// Highest y value.
    pub peak: f32,
    /// Sum of the latest point in each series.
    pub latest_total: f32,
}

/// Error building a line chart.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LineChartError {
    /// No points were supplied.
    EmptyData,
    /// A series name was empty.
    EmptySeries,
    /// A point had a non-finite coordinate.
    NonFinitePoint {
        /// Series name.
        series: String,
        /// Bad x.
        x: f32,
        /// Bad y.
        y: f32,
    },
}

impl std::fmt::Display for LineChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyData => write!(f, "line chart requires at least one point"),
            Self::EmptySeries => write!(f, "line chart series cannot be empty"),
            Self::NonFinitePoint { series, x, y } => {
                write!(
                    f,
                    "line chart point for `{series}` is not finite: ({x}, {y})"
                )
            }
        }
    }
}

impl std::error::Error for LineChartError {}

impl ChartSpec for LineChartSpec {
    type Error = LineChartError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let plot = self.plot_area(size);
        let series = series_order(&self.data);
        let x_domain = self
            .options
            .x_domain
            .unwrap_or_else(|| nice_domain(domain_of(self.data.iter().map(|datum| datum.x))));
        let y_domain = self.options.y_domain.unwrap_or_else(|| {
            nice_domain_with_zero(domain_of(self.data.iter().map(|datum| datum.y)))
        });
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
        workspace.upsert_dataset(line_dataset(&self.data));

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
                    [0.20, 0.24, 0.32, 1.0],
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
                [0.18, 0.22, 0.30, 1.0],
            )) as Arc<dyn Mark>
        }));

        let line_primitives = line_primitives(
            &self.data,
            &series,
            &x_scale,
            &y_scale,
            self.options.line_width,
        );
        marks.push(Arc::new(LineCollectionMark::new(
            LINE_MARK,
            line_primitives,
            Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
        )));

        if self.options.show_points {
            marks.push(Arc::new(PointCollectionMark::new(
                POINT_MARK,
                point_primitives(
                    &self.data,
                    &series,
                    &x_scale,
                    &y_scale,
                    self.options.point_radius,
                ),
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
            )));
        }

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
                    TooltipField::new("Series", "series").as_label(),
                    TooltipField::new("X", "x").as_number(1),
                    TooltipField::new("Y", "y").as_number(1),
                ],
            )
            .with_title_column("label"),
        ));
        let labels = endpoint_labels(&self.data, &series, &x_scale, &y_scale);
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(4.0)
                .with_max_visible(series.len()),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(
                series
                    .iter()
                    .enumerate()
                    .map(|(index, name)| LegendItem::new(name.clone(), palette(index)))
                    .collect(),
            )
            .with_title("Series")
            .with_anchor(LegendAnchor::Bottom),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(snap_targets(&self.data, &x_scale, &y_scale))
                    .with_name("line points"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn line_dataset(data: &[LineDatum]) -> Dataset {
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
                "series".into(),
                Column::Utf8(ColumnData::new(
                    data.iter()
                        .map(|datum| Arc::<str>::from(datum.series.clone()))
                        .collect(),
                )),
            ),
            (
                "label".into(),
                Column::Utf8(ColumnData::new(
                    data.iter()
                        .map(|datum| {
                            Arc::<str>::from(
                                datum
                                    .label
                                    .clone()
                                    .unwrap_or_else(|| format!("{} {}", datum.series, datum.x)),
                            )
                        })
                        .collect(),
                )),
            ),
        ],
    )
}

fn line_primitives(
    data: &[LineDatum],
    series: &[String],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
    width: f32,
) -> Vec<LinePrim> {
    series
        .iter()
        .enumerate()
        .filter_map(|(series_index, name)| {
            let mut points: Vec<&LineDatum> =
                data.iter().filter(|datum| &datum.series == name).collect();
            points.sort_by(|a, b| a.x.total_cmp(&b.x));
            (points.len() >= 2).then(|| LinePrim {
                points: points
                    .into_iter()
                    .map(|datum| {
                        [
                            x_scale.project(f64::from(datum.x)),
                            y_scale.project(f64::from(datum.y)),
                        ]
                    })
                    .collect(),
                stroke: palette(series_index),
                width,
                dash: None,
                join: 1,
                cap: 1,
            })
        })
        .collect()
}

fn point_primitives(
    data: &[LineDatum],
    series: &[String],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
    radius: f32,
) -> Vec<berthacharts_core::PointPrim> {
    data.iter()
        .map(|datum| {
            let color = palette(index_of(series, &datum.series).unwrap_or(0));
            berthacharts_core::PointPrim {
                x: x_scale.project(f64::from(datum.x)),
                y: y_scale.project(f64::from(datum.y)),
                r: radius,
                shape: 0,
                fill: rgba(
                    color[0] / color[3],
                    color[1] / color[3],
                    color[2] / color[3],
                    0.88,
                ),
                stroke: rgba(1.0, 1.0, 1.0, 0.92),
                stroke_width: 1.4,
            }
        })
        .collect()
}

fn endpoint_labels(
    data: &[LineDatum],
    series: &[String],
    x_scale: &LinearScale,
    y_scale: &LinearScale,
) -> Vec<LabelItem> {
    series
        .iter()
        .filter_map(|name| latest_for_series(data, name))
        .map(|datum| {
            LabelItem::new(
                x_scale.project(f64::from(datum.x)) + 8.0,
                y_scale.project(f64::from(datum.y)),
                datum.series.clone(),
            )
            .with_detail(format!("{:.1}", datum.y))
            .with_kind(LabelKind::Flow)
            .with_priority(LabelPriority::Required)
            .with_anchor(LabelAnchor::Right)
            .with_tooltip(LabelTooltip::new(
                datum.series.clone(),
                vec![
                    LabelTooltipRow::new("Latest x", format!("{:.1}", datum.x)),
                    LabelTooltipRow::new("Latest y", format!("{:.1}", datum.y)),
                ],
            ))
        })
        .collect()
}

fn snap_targets(
    data: &[LineDatum],
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
            .with_radius(6.0)
            .with_label(
                datum
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("{} {:.1}", datum.series, datum.x)),
            )
            .with_priority(2)
        })
        .collect()
}

fn series_order(data: &[LineDatum]) -> Vec<String> {
    let mut series = Vec::new();
    for datum in data {
        if !series.contains(&datum.series) {
            series.push(datum.series.clone());
        }
    }
    series
}

fn latest_for_series<'a>(data: &'a [LineDatum], series: &str) -> Option<&'a LineDatum> {
    data.iter()
        .filter(|datum| datum.series == series)
        .max_by(|a, b| a.x.total_cmp(&b.x))
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
    let pad = (hi - lo) * 0.06;
    (lo - pad, hi + pad)
}

fn nice_domain_with_zero((lo, hi): (f32, f32)) -> (f32, f32) {
    let (lo, hi) = nice_domain((lo.min(0.0), hi));
    (lo, hi)
}

fn palette(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 4]; 6] = [
        [0.06, 0.42, 0.78, 0.92],
        [0.05, 0.58, 0.48, 0.90],
        [0.82, 0.33, 0.27, 0.88],
        [0.55, 0.42, 0.74, 0.88],
        [0.78, 0.52, 0.19, 0.88],
        [0.20, 0.30, 0.44, 0.86],
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

    fn demo_spec() -> LineChartSpec {
        LineChartSpec::new(vec![
            LineDatum::new("Control", 1.0, 21.0),
            LineDatum::new("Control", 2.0, 23.0),
            LineDatum::new("Control", 3.0, 25.0),
            LineDatum::new("Control", 4.0, 24.0),
            LineDatum::new("Variant", 1.0, 19.0),
            LineDatum::new("Variant", 2.0, 25.0),
            LineDatum::new("Variant", 3.0, 31.0),
            LineDatum::new("Variant", 4.0, 38.0),
        ])
    }

    #[test]
    fn line_spec_builds_chart_with_guides_and_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(680, 360))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.scene().guides.is_empty());
        assert_eq!(chart.snap_targets().len(), 8);
    }

    #[test]
    fn line_summary_counts_series_and_latest_total() {
        let summary = demo_spec().summary();

        assert_eq!(summary.series, 2);
        assert_eq!(summary.points, 8);
        assert_eq!(summary.latest_total, 62.0);
    }

    #[test]
    fn line_spec_rejects_non_finite_points() {
        let err = LineChartSpec::new(vec![LineDatum::new("x", f32::NAN, 1.0)])
            .try_build_chart(Workspace::new(), ChartSize::new(320, 200))
            .unwrap_err();

        assert!(matches!(err, LineChartError::NonFinitePoint { .. }));
    }
}
