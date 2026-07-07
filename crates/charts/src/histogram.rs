//! Histogram chart spec with automatic binning, axes, direct labels, tooltips,
//! legend, and snap targets.

use std::sync::Arc;

use berthacharts_core::{
    AxisGuide, AxisOrient, CartesianCoord, Chart, ChartSize, ChartSpec, ColorChannel, Column,
    ColumnData, CoordId, Dataset, DatasetId, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem, LinearScale, Mark,
    MarkId, NumberChannel, Rect, RectMark, Scale, ScaleId, Scene, SnapKind, SnapTarget,
    SnapTargetSet, TooltipField, TooltipGuide, Workspace,
};

const DATASET: DatasetId = DatasetId::new(0);
const SCRATCH_DATASET: DatasetId = DatasetId::new(99);
const BAR_MARK: MarkId = MarkId::new(1);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);
const MAX_AUTO_BINS: usize = 200;

/// One resolved histogram bin.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBin {
    /// Inclusive lower bin edge, except for display precision.
    pub lower: f32,
    /// Exclusive upper bin edge; the final bin also includes the sample max.
    pub upper: f32,
    /// Number of samples in this bin.
    pub count: usize,
    /// Count divided by `(total_samples * bin_width)`.
    pub density: f32,
    /// Share of the total sample count in percentage points.
    pub percent: f32,
}

/// Layout and binning options for a histogram.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramOptions {
    /// Left plot padding in CSS pixels.
    pub padding_left: f32,
    /// Right plot padding in CSS pixels.
    pub padding_right: f32,
    /// Top plot padding in CSS pixels.
    pub padding_top: f32,
    /// Bottom plot padding in CSS pixels.
    pub padding_bottom: f32,
    /// Optional fixed bin count. `None` uses Freedman-Diaconis
    /// (`2 * IQR / n^(1/3)`) and falls back to Sturges
    /// (`ceil(log2(n) + 1)`) when the IQR or range is zero. The resolved count
    /// is clamped to `1..=200`.
    pub bin_count: Option<usize>,
    /// Plot density instead of raw counts.
    pub normalize: bool,
    /// X-axis label.
    pub x_axis_label: String,
    /// Desired y-axis tick count.
    pub y_tick_count: usize,
    /// Maximum direct-label budget. When absent, the tallest bins get a small
    /// default budget and shorter bins are downgraded by priority.
    pub max_visible_labels: Option<usize>,
}

impl Default for HistogramOptions {
    fn default() -> Self {
        Self {
            padding_left: 60.0,
            padding_right: 60.0,
            padding_top: 60.0,
            padding_bottom: 60.0,
            bin_count: None,
            normalize: false,
            x_axis_label: "Sample".to_string(),
            y_tick_count: 4,
            max_visible_labels: None,
        }
    }
}

/// Reusable histogram chart specification.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramSpec {
    /// Raw sample values.
    pub samples: Vec<f32>,
    /// Layout and binning options.
    pub options: HistogramOptions,
}

impl HistogramSpec {
    /// Build a histogram spec from raw samples.
    #[must_use]
    pub fn new(samples: Vec<f32>) -> Self {
        Self {
            samples,
            options: HistogramOptions::default(),
        }
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: HistogramOptions) -> Self {
        self.options = options;
        self
    }

    /// Set a fixed bin count.
    #[must_use]
    pub const fn with_bin_count(mut self, bin_count: usize) -> Self {
        self.options.bin_count = Some(bin_count);
        self
    }

    /// Render density instead of raw counts.
    #[must_use]
    pub const fn normalized(mut self) -> Self {
        self.options.normalize = true;
        self
    }

    /// Return the bins resolved from the current options.
    pub fn bins(&self) -> Result<Vec<HistogramBin>, HistogramError> {
        self.validate()?;
        Ok(bin_samples(&self.samples, self.options.bin_count))
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, HistogramError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), HistogramError> {
        if self.samples.is_empty() {
            return Err(HistogramError::EmptySamples);
        }
        for (index, value) in self.samples.iter().enumerate() {
            if !value.is_finite() {
                return Err(HistogramError::NonFiniteSample {
                    index,
                    value: *value,
                });
            }
        }
        Ok(())
    }

    fn plot_area(&self, size: ChartSize) -> Rect {
        let width = size.width as f32;
        let height = size.height as f32;
        // Upper bounds floored at 0: a collapsing container (0x0 during page
        // teardown) must degrade to the 1px Rect below, never invert the
        // clamp bounds: f32::clamp panics when min > max, and a panic here
        // unwinds through the wasm &mut self frame leaving the handle's
        // borrow flag permanently set (destroy then throws).
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

/// Error building a histogram.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum HistogramError {
    /// No samples were supplied.
    EmptySamples,
    /// A sample value was non-finite.
    NonFiniteSample {
        /// Sample index in the input vector.
        index: usize,
        /// Bad value.
        value: f32,
    },
}

impl std::fmt::Display for HistogramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySamples => write!(f, "histogram requires at least one sample"),
            Self::NonFiniteSample { index, value } => {
                write!(
                    f,
                    "histogram sample at index {index} is not finite: {value}"
                )
            }
        }
    }
}

impl std::error::Error for HistogramError {}

impl ChartSpec for HistogramSpec {
    type Error = HistogramError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let bins = bin_samples(&self.samples, self.options.bin_count);
        let plot = self.plot_area(size);
        let domain = x_domain(&bins);
        let values: Vec<f32> = bins
            .iter()
            .map(|bin| {
                if self.options.normalize {
                    bin.density
                } else {
                    bin.count as f32
                }
            })
            .collect();
        let y_max = nice_upper(values.iter().copied().fold(0.0, f32::max));

        let x_axis = LinearScale::new(domain, (plot.x, plot.x + plot.w)).clamped();
        let y_axis = LinearScale::new((0.0, f64::from(y_max)), (plot.y + plot.h, plot.y)).clamped();
        let grid_values = y_axis.ticks(self.options.y_tick_count.max(2));
        let colors: Vec<[f32; 3]> = values
            .iter()
            .map(|value| histogram_color(*value, y_max))
            .collect();

        workspace.upsert_scale(X_SCALE, Arc::new(x_axis.clone()) as Arc<dyn Scale>);
        workspace.upsert_scale(Y_SCALE, Arc::new(y_axis.clone()) as Arc<dyn Scale>);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(histogram_dataset(&bins, &values, &colors));

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

        let mut bars = RectMark::new(
            BAR_MARK,
            DATASET,
            NumberChannel::Column {
                dataset: DATASET,
                name: "lower".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "value".into(),
                scale: Y_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "upper".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "base".into(),
                scale: Y_SCALE,
            },
            [0.11, 0.55, 0.74, 1.00],
        );
        bars.fill = ColorChannel::RgbaColumns {
            dataset: DATASET,
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };
        marks.push(Arc::new(bars));

        let centers = bin_centers(&bins, &x_axis);
        let labels = data_labels(&bins, &centers, &values, &y_axis, self.options.normalize);
        let label_count = self
            .options
            .max_visible_labels
            .unwrap_or_else(|| labels.len().min(8))
            .min(labels.len());
        let snap_targets: Vec<SnapTarget> = bins
            .iter()
            .zip(&centers)
            .zip(&values)
            .map(|((bin, x), value)| {
                SnapTarget::new(*x, y_axis.project(f64::from(*value)), SnapKind::Point)
                    .with_radius(6.0)
                    .with_label(format!("{} bin top", format_range(bin.lower, bin.upper)))
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
                .with_tick_count(bins.len().clamp(2, 10)),
        ));
        scene.guides.push(Guide::Axis(
            AxisGuide::new(Y_SCALE, AxisOrient::Left)
                .with_label(if self.options.normalize {
                    "Density"
                } else {
                    "Count"
                })
                .with_tick_count(self.options.y_tick_count),
        ));
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(
                BAR_MARK,
                DATASET,
                vec![
                    TooltipField::new("Range", "range").as_label(),
                    TooltipField::new("Count", "count").as_integer(),
                    TooltipField::new("Percent", "percent").as_percent(1),
                ],
            )
            .with_title_column("range"),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(2.0)
                .with_max_visible(label_count),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(vec![LegendItem::new(
                if self.options.normalize {
                    "density"
                } else {
                    "count"
                },
                [0.10, 0.48, 0.74, 1.0],
            )])
            .with_title("Distribution")
            .with_anchor(LegendAnchor::Bottom),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(snap_targets).with_name("histogram bin tops"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn histogram_dataset(bins: &[HistogramBin], values: &[f32], colors: &[[f32; 3]]) -> Dataset {
    Dataset::new(
        DATASET,
        1,
        vec![
            (
                "lower".into(),
                Column::F32(ColumnData::new(bins.iter().map(|bin| bin.lower).collect())),
            ),
            (
                "upper".into(),
                Column::F32(ColumnData::new(bins.iter().map(|bin| bin.upper).collect())),
            ),
            (
                "value".into(),
                Column::F32(ColumnData::new(values.to_vec())),
            ),
            (
                "base".into(),
                Column::F32(ColumnData::new(vec![0.0; bins.len()])),
            ),
            (
                "count".into(),
                Column::U32(ColumnData::new(
                    bins.iter()
                        .map(|bin| bin.count.min(u32::MAX as usize) as u32)
                        .collect(),
                )),
            ),
            (
                "percent".into(),
                Column::F32(ColumnData::new(
                    bins.iter().map(|bin| bin.percent).collect(),
                )),
            ),
            (
                "range".into(),
                Column::Utf8(ColumnData::new(
                    bins.iter()
                        .map(|bin| Arc::<str>::from(format_range(bin.lower, bin.upper)))
                        .collect(),
                )),
            ),
            (
                "r".into(),
                Column::F32(ColumnData::new(
                    colors.iter().map(|color| color[0]).collect(),
                )),
            ),
            (
                "g".into(),
                Column::F32(ColumnData::new(
                    colors.iter().map(|color| color[1]).collect(),
                )),
            ),
            (
                "b".into(),
                Column::F32(ColumnData::new(
                    colors.iter().map(|color| color[2]).collect(),
                )),
            ),
        ],
    )
}

fn data_labels(
    bins: &[HistogramBin],
    centers: &[f32],
    values: &[f32],
    y_axis: &LinearScale,
    normalize: bool,
) -> Vec<LabelItem> {
    let max_value = values.iter().copied().fold(0.0, f32::max).max(1.0);
    bins.iter()
        .zip(centers)
        .zip(values)
        .filter(|((bin, _), _)| bin.count > 0)
        .map(|((bin, x), value)| {
            let ratio = *value / max_value;
            let priority = if ratio >= 0.85 {
                LabelPriority::Required
            } else if ratio >= 0.60 {
                LabelPriority::Important
            } else {
                LabelPriority::Optional
            };
            LabelItem::new(
                *x,
                y_axis.project(f64::from(*value)) - 2.0,
                if normalize {
                    format!("{value:.2}")
                } else {
                    bin.count.to_string()
                },
            )
            .with_kind(LabelKind::Data)
            .with_priority(priority)
            .with_anchor(LabelAnchor::Top)
            .with_detail(format!("{:.1}%", bin.percent))
        })
        .collect()
}

fn bin_samples(samples: &[f32], requested_bin_count: Option<usize>) -> Vec<HistogramBin> {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f32::total_cmp);

    let min = f64::from(sorted[0]);
    let max = f64::from(sorted[sorted.len() - 1]);
    let bin_count = if (max - min).abs() < f64::EPSILON {
        1
    } else {
        requested_bin_count
            .map(clamp_bin_count)
            .unwrap_or_else(|| automatic_bin_count(&sorted))
    };
    let (domain_min, domain_max) = if bin_count == 1 && (max - min).abs() < f64::EPSILON {
        identical_domain(min)
    } else {
        (min, max)
    };
    let width = ((domain_max - domain_min) / bin_count as f64).max(f64::MIN_POSITIVE);
    let mut counts = vec![0usize; bin_count];
    for sample in samples {
        let raw = ((f64::from(*sample) - domain_min) / width).floor();
        let index = if raw.is_finite() {
            (raw as isize).clamp(0, bin_count as isize - 1) as usize
        } else {
            0
        };
        counts[index] += 1;
    }

    let total = samples.len() as f64;
    counts
        .into_iter()
        .enumerate()
        .map(|(index, count)| {
            let lower = domain_min + width * index as f64;
            let upper = if index + 1 == bin_count {
                domain_max
            } else {
                domain_min + width * (index + 1) as f64
            };
            let density = count as f64 / (total * width);
            HistogramBin {
                lower: finite_f32(lower),
                upper: finite_f32(upper),
                count,
                density: finite_f32(density.max(0.0)),
                percent: finite_f32(count as f64 * 100.0 / total),
            }
        })
        .collect()
}

fn automatic_bin_count(sorted: &[f32]) -> usize {
    if sorted.len() <= 1 {
        return 1;
    }
    let min = f64::from(sorted[0]);
    let max = f64::from(sorted[sorted.len() - 1]);
    let range = max - min;
    if range <= f64::EPSILON {
        return 1;
    }

    let q1 = quantile(sorted, 0.25);
    let q3 = quantile(sorted, 0.75);
    let iqr = q3 - q1;
    if iqr > f64::EPSILON && iqr.is_finite() {
        let width = 2.0 * iqr / (sorted.len() as f64).cbrt();
        if width > f64::EPSILON && width.is_finite() {
            return clamp_bin_count_f64((range / width).ceil());
        }
    }

    clamp_bin_count_f64((sorted.len() as f64).log2().ceil() + 1.0)
}

fn quantile(sorted: &[f32], p: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * p;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        return f64::from(sorted[lower_index]);
    }
    let fraction = position - lower_index as f64;
    let lower = f64::from(sorted[lower_index]);
    let upper = f64::from(sorted[upper_index]);
    lower + (upper - lower) * fraction
}

fn clamp_bin_count(value: usize) -> usize {
    value.clamp(1, MAX_AUTO_BINS)
}

fn clamp_bin_count_f64(value: f64) -> usize {
    if !value.is_finite() {
        return MAX_AUTO_BINS;
    }
    value.clamp(1.0, MAX_AUTO_BINS as f64) as usize
}

fn identical_domain(value: f64) -> (f64, f64) {
    let span = (value.abs() * 0.05).max(1.0);
    let min_limit = f64::from(f32::MIN);
    let max_limit = f64::from(f32::MAX);
    let lower = (value - span).max(min_limit);
    let upper = (value + span).min(max_limit);
    if upper > lower {
        (lower, upper)
    } else if value >= 0.0 {
        ((value - span).max(min_limit), value)
    } else {
        (value, (value + span).min(max_limit))
    }
}

fn x_domain(bins: &[HistogramBin]) -> (f64, f64) {
    let first = f64::from(bins.first().map_or(0.0, |bin| bin.lower));
    let last = f64::from(bins.last().map_or(1.0, |bin| bin.upper));
    if last > first {
        (first, last)
    } else {
        let (lower, upper) = identical_domain(first);
        (lower, upper)
    }
}

fn bin_centers(bins: &[HistogramBin], x_axis: &LinearScale) -> Vec<f32> {
    bins.iter()
        .map(|bin| {
            let center = (f64::from(bin.lower) + f64::from(bin.upper)) * 0.5;
            x_axis.project(center)
        })
        .collect()
}

fn nice_upper(value: f32) -> f32 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let padded = f64::from(value) * 1.12;
    let exponent = padded.log10().floor();
    let base = 10_f64.powf(exponent);
    let fraction = padded / base;
    let nice_fraction = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    finite_f32(nice_fraction * base).max(1.0)
}

fn histogram_color(value: f32, y_max: f32) -> [f32; 3] {
    let t = (value / y_max.max(1.0)).clamp(0.0, 1.0);
    [0.08 + 0.04 * t, 0.50 - 0.16 * t, 0.70 + 0.18 * t]
}

fn format_range(lower: f32, upper: f32) -> String {
    format!("{} to {}", format_edge(lower), format_edge(upper))
}

fn format_edge(value: f32) -> String {
    let abs = value.abs();
    if abs >= 1000.0 || (abs > 0.0 && abs < 0.01) {
        format!("{value:.2e}")
    } else if abs >= 100.0 {
        format!("{value:.0}")
    } else if abs >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn finite_f32(value: f64) -> f32 {
    if value > f64::from(f32::MAX) {
        f32::MAX
    } else if value < f64::from(f32::MIN) {
        f32::MIN
    } else {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use berthacharts_core::{
        ChartSize, Column, DatasetId, Guide, LabelPriority, TooltipValueFormat, Workspace,
    };

    use super::*;

    const DATASET: DatasetId = DatasetId::new(0);

    fn sample_values() -> Vec<f32> {
        vec![0.0, 0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 4.0]
    }

    fn f32_column(dataset: &berthacharts_core::Dataset, name: &str) -> Vec<f32> {
        match dataset.column(name).expect(name).as_ref() {
            Column::F32(data) => data.values.clone(),
            other => panic!("expected f32 column `{name}`, got {}", other.dtype()),
        }
    }

    fn u32_column(dataset: &berthacharts_core::Dataset, name: &str) -> Vec<u32> {
        match dataset.column(name).expect(name).as_ref() {
            Column::U32(data) => data.values.clone(),
            other => panic!("expected u32 column `{name}`, got {}", other.dtype()),
        }
    }

    #[test]
    fn histogram_spec_builds_chart_with_guides_labels_and_snap_targets() {
        let workspace = Workspace::new();
        let chart = HistogramSpec::new(sample_values())
            .with_options(HistogramOptions {
                bin_count: Some(4),
                ..HistogramOptions::default()
            })
            .try_build_chart(workspace.clone(), ChartSize::new(680, 390))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(chart.scene().guides.iter().any(
            |guide| matches!(guide, Guide::Axis(axis) if axis.label.as_deref() == Some("Sample"))
        ));
        assert!(chart.scene().guides.iter().any(
            |guide| matches!(guide, Guide::Axis(axis) if axis.label.as_deref() == Some("Count"))
        ));
        assert!(chart
            .scene()
            .guides
            .iter()
            .any(|guide| matches!(guide, Guide::Tooltip(_))));
        assert!(chart
            .scene()
            .guides
            .iter()
            .any(|guide| matches!(guide, Guide::Legend(_))));

        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        assert!(labels
            .items
            .iter()
            .any(|label| label.priority == LabelPriority::Required));
        assert!(labels
            .items
            .iter()
            .any(|label| label.priority == LabelPriority::Optional));
        assert_eq!(chart.snap_targets().len(), 4);

        let dataset = workspace.dataset(DATASET).expect("histogram dataset");
        assert_eq!(dataset.len(), 4);
        assert_eq!(u32_column(&dataset, "count").iter().sum::<u32>(), 12);
    }

    #[test]
    fn automatic_bin_count_uses_freedman_diaconis_before_sturges_fallback() {
        let workspace = Workspace::new();
        HistogramSpec::new((1..=100).map(|value| value as f32).collect())
            .try_build_chart(workspace.clone(), ChartSize::new(600, 360))
            .unwrap();

        let dataset = workspace.dataset(DATASET).expect("histogram dataset");
        assert_eq!(dataset.len(), 5);
    }

    #[test]
    fn all_identical_samples_use_one_non_zero_width_bin() {
        let workspace = Workspace::new();
        HistogramSpec::new(vec![7.0, 7.0, 7.0, 7.0])
            .try_build_chart(workspace.clone(), ChartSize::new(500, 320))
            .unwrap();

        let dataset = workspace.dataset(DATASET).expect("histogram dataset");
        assert_eq!(dataset.len(), 1);
        assert_eq!(u32_column(&dataset, "count"), vec![4]);

        let lower = f32_column(&dataset, "lower");
        let upper = f32_column(&dataset, "upper");
        assert!(lower[0].is_finite());
        assert!(upper[0].is_finite());
        assert!(upper[0] > lower[0]);
    }

    #[test]
    fn rejects_non_finite_samples() {
        let err = HistogramSpec::new(vec![1.0, f32::NAN, 2.0])
            .try_build_chart(Workspace::new(), ChartSize::new(320, 200))
            .unwrap_err();

        assert!(matches!(
            err,
            HistogramError::NonFiniteSample {
                index: 1,
                value
            } if value.is_nan()
        ));
    }

    #[test]
    fn rejects_empty_samples() {
        let err = HistogramSpec::new(Vec::new())
            .try_build_chart(Workspace::new(), ChartSize::new(320, 200))
            .unwrap_err();

        assert_eq!(err, HistogramError::EmptySamples);
    }

    #[test]
    fn normalize_mode_uses_density_values_and_density_axis() {
        let workspace = Workspace::new();
        let chart = HistogramSpec::new(vec![0.0, 0.5, 1.0, 1.5])
            .with_options(HistogramOptions {
                bin_count: Some(2),
                normalize: true,
                ..HistogramOptions::default()
            })
            .try_build_chart(workspace.clone(), ChartSize::new(500, 320))
            .unwrap();

        assert!(chart.scene().guides.iter().any(
            |guide| matches!(guide, Guide::Axis(axis) if axis.label.as_deref() == Some("Density"))
        ));

        let dataset = workspace.dataset(DATASET).expect("histogram dataset");
        let values = f32_column(&dataset, "value");
        let lower = f32_column(&dataset, "lower");
        let upper = f32_column(&dataset, "upper");
        let area = values
            .iter()
            .zip(lower.iter().zip(&upper))
            .map(|(density, (lower, upper))| density * (upper - lower))
            .sum::<f32>();
        assert!((area - 1.0).abs() < 1e-5);
    }

    #[test]
    fn tooltip_fields_include_range_count_and_percent() {
        let chart = HistogramSpec::new(sample_values())
            .with_options(HistogramOptions {
                bin_count: Some(4),
                ..HistogramOptions::default()
            })
            .try_build_chart(Workspace::new(), ChartSize::new(680, 390))
            .unwrap();

        let tooltip = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Tooltip(tooltip) => Some(tooltip),
                _ => None,
            })
            .expect("tooltip guide");

        assert_eq!(tooltip.title_column.as_deref(), Some("range"));
        assert_eq!(tooltip.fields.len(), 3);
        assert_eq!(tooltip.fields[0].label, "Range");
        assert_eq!(tooltip.fields[0].column, "range");
        assert_eq!(tooltip.fields[0].format, TooltipValueFormat::Label);
        assert_eq!(tooltip.fields[1].label, "Count");
        assert_eq!(tooltip.fields[1].column, "count");
        assert_eq!(tooltip.fields[1].format, TooltipValueFormat::Integer);
        assert_eq!(tooltip.fields[2].label, "Percent");
        assert_eq!(tooltip.fields[2].column, "percent");
        assert_eq!(
            tooltip.fields[2].format,
            TooltipValueFormat::Percent { decimals: 1 }
        );
    }
}
