//! Bar chart spec with axes, direct labels, tooltips, legend, snap targets,
//! and optional trend/target diagnostics.

use std::sync::Arc;

use berthacharts_core::{
    AxisGuide, AxisOrient, BandScale, CartesianCoord, Chart, ChartSize, ChartSpec, ColorChannel,
    Column, ColumnData, CoordId, Dataset, DatasetId, Geometry, Guide, LabelAnchor, LabelGuide,
    LabelItem, LabelKind, LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem,
    LinePrim, LinearScale, Mark, MarkId, NumberChannel, PointPrim, Rect, RectMark, Scale, ScaleId,
    Scene, SnapKind, SnapTarget, SnapTargetSet, TooltipField, TooltipGuide, TrianglePrim,
    Workspace,
};

use crate::mark::GeometryMark;

const DATASET: DatasetId = DatasetId::new(0);
const SCRATCH_DATASET: DatasetId = DatasetId::new(99);
const BAR_MARK: MarkId = MarkId::new(1);
const ANALYSIS_MARK: MarkId = MarkId::new(80);
const TARGET_MARK: MarkId = MarkId::new(90);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One categorical bar.
#[derive(Debug, Clone, PartialEq)]
pub struct BarDatum {
    /// Category label.
    pub label: String,
    /// Numeric value.
    pub value: f32,
}

impl BarDatum {
    /// Build one bar datum.
    #[must_use]
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// Layout and guide options for a bar chart.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChartOptions {
    /// Left plot padding in CSS pixels.
    pub padding_left: f32,
    /// Right plot padding in CSS pixels.
    pub padding_right: f32,
    /// Top plot padding in CSS pixels.
    pub padding_top: f32,
    /// Bottom plot padding in CSS pixels.
    pub padding_bottom: f32,
    /// Optional fixed y-axis upper bound.
    pub y_max: Option<f32>,
    /// Optional target threshold.
    pub target: Option<f32>,
    /// X-axis label.
    pub x_axis_label: String,
    /// Y-axis label.
    pub y_axis_label: String,
    /// Desired y-axis tick count.
    pub y_tick_count: usize,
    /// Minimum direct-label budget. Dense charts may still be capped by data
    /// count and collision handling.
    pub max_visible_labels: Option<usize>,
}

impl Default for BarChartOptions {
    fn default() -> Self {
        Self {
            padding_left: 60.0,
            padding_right: 60.0,
            padding_top: 60.0,
            padding_bottom: 60.0,
            y_max: None,
            target: None,
            x_axis_label: "Category".to_string(),
            y_axis_label: "Value".to_string(),
            y_tick_count: 4,
            max_visible_labels: None,
        }
    }
}

/// Reusable categorical bar chart specification.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChartSpec {
    /// Bars in display order.
    pub data: Vec<BarDatum>,
    /// Layout and guide options.
    pub options: BarChartOptions,
}

impl BarChartSpec {
    /// Build a bar chart spec from data.
    #[must_use]
    pub fn new(data: Vec<BarDatum>) -> Self {
        Self {
            data,
            options: BarChartOptions::default(),
        }
    }

    /// Set the target threshold.
    #[must_use]
    pub const fn with_target(mut self, target: f32) -> Self {
        self.options.target = Some(target);
        self
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: BarChartOptions) -> Self {
        self.options = options;
        self
    }

    /// Compute headline summary values from this spec.
    #[must_use]
    pub fn summary(&self) -> BarChartSummary {
        let values: Vec<f32> = self.data.iter().map(|datum| datum.value).collect();
        let (_, slope, sigma) = fit_trend(&values);
        let peak = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let above_target = self.options.target.map_or(0, |target| {
            values.iter().filter(|value| **value >= target).count()
        });
        BarChartSummary {
            peak: if peak.is_finite() { peak } else { 0.0 },
            above_target,
            slope,
            sigma,
        }
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, BarChartError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), BarChartError> {
        if self.data.is_empty() {
            return Err(BarChartError::EmptyData);
        }
        for datum in &self.data {
            if datum.label.trim().is_empty() {
                return Err(BarChartError::EmptyLabel);
            }
            if !datum.value.is_finite() {
                return Err(BarChartError::NonFiniteValue {
                    label: datum.label.clone(),
                    value: datum.value,
                });
            }
        }
        if let Some(target) = self.options.target {
            if !target.is_finite() {
                return Err(BarChartError::NonFiniteTarget { value: target });
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

    fn y_max(&self, trend: &[f32], sigma: f32) -> f32 {
        if let Some(y_max) = self.options.y_max {
            return y_max.max(1.0);
        }
        let mut max_value = self
            .data
            .iter()
            .map(|datum| datum.value)
            .fold(0.0, f32::max);
        if let Some(target) = self.options.target {
            max_value = max_value.max(target);
        }
        for value in trend {
            max_value = max_value.max(*value + sigma * 0.75);
        }
        nice_upper(max_value)
    }
}

/// Summary statistics for a bar chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarChartSummary {
    /// Highest bar value.
    pub peak: f32,
    /// Number of bars at or above the configured target.
    pub above_target: usize,
    /// Least-squares slope over display order.
    pub slope: f32,
    /// Residual standard deviation.
    pub sigma: f32,
}

/// Error building a bar chart.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BarChartError {
    /// No bars were supplied.
    EmptyData,
    /// A bar label was empty.
    EmptyLabel,
    /// A bar value was non-finite.
    NonFiniteValue {
        /// Bar label.
        label: String,
        /// Bad value.
        value: f32,
    },
    /// The target value was non-finite.
    NonFiniteTarget {
        /// Bad value.
        value: f32,
    },
}

impl std::fmt::Display for BarChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyData => write!(f, "bar chart requires at least one datum"),
            Self::EmptyLabel => write!(f, "bar chart labels cannot be empty"),
            Self::NonFiniteValue { label, value } => {
                write!(f, "bar chart value for `{label}` is not finite: {value}")
            }
            Self::NonFiniteTarget { value } => {
                write!(f, "bar chart target is not finite: {value}")
            }
        }
    }
}

impl std::error::Error for BarChartError {}

impl ChartSpec for BarChartSpec {
    type Error = BarChartError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let category_labels: Vec<String> =
            self.data.iter().map(|datum| datum.label.clone()).collect();
        let values: Vec<f32> = self.data.iter().map(|datum| datum.value).collect();
        let (trend, _slope, sigma) = fit_trend(&values);
        let target = self.options.target;
        let target_gaps: Vec<f32> = values
            .iter()
            .map(|value| target.map_or(0.0, |target| value - target))
            .collect();
        let residuals: Vec<f32> = values
            .iter()
            .zip(&trend)
            .map(|(value, expected)| value - expected)
            .collect();
        let y_max = self.y_max(&trend, sigma);
        let plot = self.plot_area(size);

        let x_band = BandScale::new(category_labels.clone(), (plot.x, plot.x + plot.w))
            .with_padding_inner(0.15)
            .with_padding_outer(0.05);
        let bandwidth = x_band.bandwidth();
        let centers: Vec<f32> = (0..category_labels.len())
            .map(|idx| x_band.project(idx as f64) + bandwidth * 0.5)
            .collect();
        let y_axis = LinearScale::new((0.0, f64::from(y_max)), (plot.y + plot.h, plot.y));
        let grid_values = y_axis.ticks(self.options.y_tick_count.max(2));
        let target_y = target.map(|target| y_axis.project(f64::from(target)));
        let colors: Vec<[f32; 3]> = values
            .iter()
            .map(|value| bar_color(*value, y_max))
            .collect();

        workspace.upsert_scale(X_SCALE, Arc::new(x_band) as Arc<dyn Scale>);
        workspace.upsert_scale(Y_SCALE, Arc::new(y_axis.clone()) as Arc<dyn Scale>);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(bar_dataset(
            &category_labels,
            &values,
            &target_gaps,
            &trend,
            &residuals,
            &colors,
        ));

        let mut marks: Vec<Arc<dyn Mark>> = grid_values
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

        if let Some(target_y) = target_y {
            marks.push(Arc::new(RectMark::new(
                TARGET_MARK,
                SCRATCH_DATASET,
                NumberChannel::Constant(plot.x),
                NumberChannel::Constant(target_y),
                NumberChannel::Constant(plot.x + plot.w),
                NumberChannel::Constant(target_y + 2.0),
                [0.98, 0.40, 0.32, 1.0],
            )));
        }

        let mut bars = RectMark::new(
            BAR_MARK,
            DATASET,
            NumberChannel::Column {
                dataset: DATASET,
                name: "cat".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "value".into(),
                scale: Y_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "cat".into(),
                scale: X_SCALE,
            }
            .offset(bandwidth),
            NumberChannel::Column {
                dataset: DATASET,
                name: "base".into(),
                scale: Y_SCALE,
            },
            [0.40, 0.63, 1.00, 1.00],
        );
        bars.fill = ColorChannel::RgbaColumns {
            dataset: DATASET,
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };
        marks.push(Arc::new(bars));
        // The trend line + model band + residual glyphs are meaningful only
        // against a configured target/threshold. On plain categorical bars
        // (ranked customers, machines, locations) they read as a spurious
        // downward trend with stray triangles, so only draw the analysis layer
        // when a target is actually set.
        if self.options.target.is_some() {
            marks.push(Arc::new(GeometryMark::new(
                ANALYSIS_MARK,
                analysis_geometry(
                    &centers, &values, &trend, &residuals, sigma, &y_axis, target,
                ),
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
            )));
        }

        let labels = data_labels(&centers, &values, &target_gaps, &y_axis, target);
        let label_count = self
            .options
            .max_visible_labels
            .unwrap_or(labels.len())
            .min(labels.len());
        let snap_targets: Vec<SnapTarget> = centers
            .iter()
            .zip(&values)
            .zip(&category_labels)
            .map(|((x, value), category)| {
                SnapTarget::new(*x, y_axis.project(f64::from(*value)), SnapKind::Point)
                    .with_radius(6.0)
                    .with_label(format!("{category} observed"))
                    .with_priority(2)
            })
            .collect();

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
                .with_tick_count(category_labels.len()),
        ));
        scene.guides.push(Guide::Axis(
            AxisGuide::new(Y_SCALE, AxisOrient::Left)
                .with_label(self.options.y_axis_label.clone())
                .with_tick_count(self.options.y_tick_count),
        ));
        let mut tooltip_fields = vec![
            TooltipField::new("Value", "value").as_integer(),
            TooltipField::new("Model", "model").as_number(1),
            TooltipField::new("Residual", "residual").as_signed_number(1),
        ];
        if target.is_some() {
            tooltip_fields.insert(
                1,
                TooltipField::new("Target gap", "target_gap").as_signed_number(0),
            );
        }
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(BAR_MARK, DATASET, tooltip_fields).with_title_column("label"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(2.0)
                .with_max_visible(label_count),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(legend_items(target.is_some()))
                .with_title("Signals")
                .with_anchor(LegendAnchor::Bottom),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(snap_targets).with_name("observed values"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn bar_dataset(
    labels: &[String],
    values: &[f32],
    target_gaps: &[f32],
    trend: &[f32],
    residuals: &[f32],
    colors: &[[f32; 3]],
) -> Dataset {
    let n = labels.len();
    Dataset::new(
        DATASET,
        1,
        vec![
            (
                "cat".into(),
                Column::U32(ColumnData::new((0..n as u32).collect())),
            ),
            (
                "value".into(),
                Column::F32(ColumnData::new(values.to_vec())),
            ),
            ("base".into(), Column::F32(ColumnData::new(vec![0.0; n]))),
            (
                "target_gap".into(),
                Column::F32(ColumnData::new(target_gaps.to_vec())),
            ),
            ("model".into(), Column::F32(ColumnData::new(trend.to_vec()))),
            (
                "residual".into(),
                Column::F32(ColumnData::new(residuals.to_vec())),
            ),
            (
                "label".into(),
                Column::Utf8(ColumnData::new(
                    labels
                        .iter()
                        .map(|label| Arc::<str>::from(label.clone()))
                        .collect(),
                )),
            ),
            (
                "r".into(),
                Column::F32(ColumnData::new(colors.iter().map(|c| c[0]).collect())),
            ),
            (
                "g".into(),
                Column::F32(ColumnData::new(colors.iter().map(|c| c[1]).collect())),
            ),
            (
                "b".into(),
                Column::F32(ColumnData::new(colors.iter().map(|c| c[2]).collect())),
            ),
        ],
    )
}

fn data_labels(
    centers: &[f32],
    values: &[f32],
    target_gaps: &[f32],
    y_axis: &LinearScale,
    target: Option<f32>,
) -> Vec<LabelItem> {
    centers
        .iter()
        .zip(values)
        .zip(target_gaps)
        .map(|((x, value), target_gap)| {
            let mut label = LabelItem::new(
                *x,
                y_axis.project(f64::from(*value)) - 2.0,
                format!("{value:.0}"),
            )
            .with_kind(LabelKind::Data)
            .with_priority(if target.is_some_and(|target| *value >= target) {
                LabelPriority::Required
            } else {
                LabelPriority::Important
            })
            .with_anchor(LabelAnchor::Top);
            if target.is_some() {
                label = label.with_detail(format_signed(*target_gap));
            }
            label
        })
        .collect()
}

fn legend_items(has_target: bool) -> Vec<LegendItem> {
    let mut items = vec![
        LegendItem::new("observed", [0.38, 0.66, 0.87, 1.0]),
        LegendItem::new("trend", [0.17, 0.47, 0.82, 1.0]),
        LegendItem::new("model band", [0.18, 0.56, 0.92, 0.22]),
    ];
    if has_target {
        items.insert(1, LegendItem::new("target", [0.94, 0.42, 0.35, 1.0]));
        items.push(LegendItem::new("above target", [0.08, 0.64, 0.46, 1.0]));
    }
    items
}

fn fit_trend(values: &[f32]) -> (Vec<f32>, f32, f32) {
    if values.is_empty() {
        return (Vec::new(), 0.0, 0.0);
    }
    if values.len() == 1 {
        return (vec![values[0]], 0.0, 0.0);
    }

    let n = values.len() as f32;
    let x_mean = (n - 1.0) * 0.5;
    let y_mean = values.iter().sum::<f32>() / n;
    let sxx = (0..values.len())
        .map(|idx| {
            let dx = idx as f32 - x_mean;
            dx * dx
        })
        .sum::<f32>();
    let sxy = values
        .iter()
        .enumerate()
        .map(|(idx, value)| (idx as f32 - x_mean) * (value - y_mean))
        .sum::<f32>();
    let slope = if sxx.abs() < f32::EPSILON {
        0.0
    } else {
        sxy / sxx
    };
    let intercept = y_mean - slope * x_mean;
    let trend: Vec<f32> = (0..values.len())
        .map(|idx| intercept + slope * idx as f32)
        .collect();
    let residual_sum = values
        .iter()
        .zip(&trend)
        .map(|(value, expected)| {
            let residual = value - expected;
            residual * residual
        })
        .sum::<f32>();
    let sigma = (residual_sum / (n - 2.0).max(1.0)).sqrt();
    (trend, slope, sigma)
}

fn analysis_geometry(
    centers: &[f32],
    values: &[f32],
    trend: &[f32],
    residuals: &[f32],
    sigma: f32,
    y_axis: &LinearScale,
    target: Option<f32>,
) -> Geometry {
    let band = sigma.max(0.1) * 0.55;
    let mut band_triangles = Vec::with_capacity(centers.len().saturating_sub(1) * 2);
    for idx in 0..centers.len().saturating_sub(1) {
        let x0 = centers[idx];
        let x1 = centers[idx + 1];
        let hi0 = y_axis.project(f64::from(trend[idx] + band));
        let lo0 = y_axis.project(f64::from(trend[idx] - band));
        let hi1 = y_axis.project(f64::from(trend[idx + 1] + band));
        let lo1 = y_axis.project(f64::from(trend[idx + 1] - band));
        band_triangles.push(TrianglePrim {
            a: [x0, hi0],
            b: [x0, lo0],
            c: [x1, hi1],
            fill: rgba(0.18, 0.56, 0.92, 0.16),
        });
        band_triangles.push(TrianglePrim {
            a: [x1, hi1],
            b: [x0, lo0],
            c: [x1, lo1],
            fill: rgba(0.18, 0.56, 0.92, 0.16),
        });
    }

    let trend_line = LinePrim {
        points: centers
            .iter()
            .zip(trend)
            .map(|(x, y)| [*x, y_axis.project(f64::from(*y))])
            .collect(),
        stroke: rgba(0.08, 0.35, 0.72, 0.92),
        width: 2.5,
        dash: None,
        join: 1,
        cap: 1,
    };

    let points = centers
        .iter()
        .zip(values)
        .zip(residuals)
        .map(|((x, value), residual)| {
            let above_target = target.is_some_and(|target| *value >= target);
            let strong_miss = *residual < -sigma * 0.75;
            let (fill, stroke, shape, r) = if above_target {
                (
                    rgba(0.08, 0.64, 0.46, 0.95),
                    rgba(0.02, 0.28, 0.20, 0.90),
                    3,
                    5.2,
                )
            } else if strong_miss {
                (
                    rgba(0.86, 0.31, 0.28, 0.90),
                    rgba(0.42, 0.10, 0.09, 0.85),
                    2,
                    5.0,
                )
            } else {
                (
                    rgba(0.28, 0.51, 0.84, 0.82),
                    rgba(0.10, 0.22, 0.46, 0.70),
                    0,
                    4.4,
                )
            };
            PointPrim {
                x: *x,
                y: y_axis.project(f64::from(*value)),
                r,
                shape,
                fill,
                stroke,
                stroke_width: 1.3,
            }
        })
        .collect();

    Geometry::Mixed(vec![
        Geometry::Triangles(band_triangles),
        Geometry::Lines(vec![trend_line]),
        Geometry::Points(points),
    ])
}

fn bar_color(value: f32, y_max: f32) -> [f32; 3] {
    let t = (value / y_max.max(1.0)).clamp(0.0, 1.0);
    // Blue value-ramp in LINEAR RGB (the framebuffer is sRGB): short bars are
    // ~blue-400 (#60a5fa), tall bars deepen to ~blue-600 (#2563eb) — taller =
    // more saturated/darker, a mid-blue that keeps contrast on both a white
    // (light-mode) and dark page. Replaces the previous ramp whose TALLEST
    // bars came out palest cyan (weak on white).
    [0.117 - 0.098 * t, 0.376 - 0.251 * t, 0.956 - 0.125 * t]
}

fn nice_upper(value: f32) -> f32 {
    if value <= 0.0 {
        return 1.0;
    }
    let padded = value * 1.15;
    let step = if padded <= 10.0 {
        2.0
    } else if padded <= 50.0 {
        5.0
    } else if padded <= 100.0 {
        10.0
    } else {
        25.0
    };
    (padded / step).ceil() * step
}

fn format_signed(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.0}")
    } else {
        format!("{value:.0}")
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_spec() -> BarChartSpec {
        BarChartSpec::new(vec![
            BarDatum::new("Jan", 12.0),
            BarDatum::new("Feb", 19.0),
            BarDatum::new("Mar", 7.0),
            BarDatum::new("Apr", 22.0),
            BarDatum::new("May", 16.0),
            BarDatum::new("Jun", 25.0),
        ])
        .with_target(21.0)
    }

    #[test]
    fn bar_spec_builds_chart_with_guides_and_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(680, 390))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.scene().guides.is_empty());
        assert_eq!(chart.snap_targets().len(), 6);
    }

    #[test]
    fn bar_spec_scales_to_small_sizes() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(220, 160))
            .unwrap();

        assert_eq!(chart.scene().viewport.width, 220);
        assert!(chart.scene().viewport.plot_area.w >= 1.0);
    }

    #[test]
    fn bar_spec_rejects_non_finite_values() {
        let err = BarChartSpec::new(vec![BarDatum::new("bad", f32::NAN)])
            .try_build_chart(Workspace::new(), ChartSize::new(320, 200))
            .unwrap_err();

        assert!(matches!(err, BarChartError::NonFiniteValue { .. }));
    }
}
