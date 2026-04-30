//! Reusable gallery data fixtures.
//!
//! These are deliberately plain Rust structs. Chart specs can map them into
//! `berthacharts-*` types, while future loaders can replace these fixtures with
//! CSV, API, or generated data without rewriting each example component.

use crate::gallery::DataProfile;

const RETAIL_REVENUE_BARS: &[CategoryValue] = &[
    CategoryValue::new("Jan", 12.0),
    CategoryValue::new("Feb", 19.0),
    CategoryValue::new("Mar", 7.0),
    CategoryValue::new("Apr", 22.0),
    CategoryValue::new("May", 16.0),
    CategoryValue::new("Jun", 25.0),
];

const GROWTH_REVENUE_BARS: &[CategoryValue] = &[
    CategoryValue::new("Jan", 18.0),
    CategoryValue::new("Feb", 24.0),
    CategoryValue::new("Mar", 31.0),
    CategoryValue::new("Apr", 27.0),
    CategoryValue::new("May", 35.0),
    CategoryValue::new("Jun", 39.0),
];

const OPERATIONS_REVENUE_BARS: &[CategoryValue] = &[
    CategoryValue::new("Jan", 54.0),
    CategoryValue::new("Feb", 63.0),
    CategoryValue::new("Mar", 69.0),
    CategoryValue::new("Apr", 77.0),
    CategoryValue::new("May", 74.0),
    CategoryValue::new("Jun", 86.0),
];

const RETAIL_EXPERIMENT_LINES: &[SeriesPoint] = &[
    SeriesPoint::new("Control", 1.0, 21.0, "Control week 1"),
    SeriesPoint::new("Control", 2.0, 23.0, "Control week 2"),
    SeriesPoint::new("Control", 3.0, 25.0, "Control week 3"),
    SeriesPoint::new("Control", 4.0, 24.0, "Control week 4"),
    SeriesPoint::new("Control", 5.0, 27.0, "Control week 5"),
    SeriesPoint::new("Variant A", 1.0, 19.0, "Variant A week 1"),
    SeriesPoint::new("Variant A", 2.0, 25.0, "Variant A week 2"),
    SeriesPoint::new("Variant A", 3.0, 31.0, "Variant A week 3"),
    SeriesPoint::new("Variant A", 4.0, 38.0, "Variant A week 4"),
    SeriesPoint::new("Variant A", 5.0, 43.0, "Variant A week 5"),
    SeriesPoint::new("Variant B", 1.0, 20.0, "Variant B week 1"),
    SeriesPoint::new("Variant B", 2.0, 24.0, "Variant B week 2"),
    SeriesPoint::new("Variant B", 3.0, 29.0, "Variant B week 3"),
    SeriesPoint::new("Variant B", 4.0, 33.0, "Variant B week 4"),
    SeriesPoint::new("Variant B", 5.0, 37.0, "Variant B week 5"),
];

const GROWTH_EXPERIMENT_LINES: &[SeriesPoint] = &[
    SeriesPoint::new("Organic", 1.0, 32.0, "Organic week 1"),
    SeriesPoint::new("Organic", 2.0, 37.0, "Organic week 2"),
    SeriesPoint::new("Organic", 3.0, 42.0, "Organic week 3"),
    SeriesPoint::new("Organic", 4.0, 48.0, "Organic week 4"),
    SeriesPoint::new("Organic", 5.0, 57.0, "Organic week 5"),
    SeriesPoint::new("Paid", 1.0, 24.0, "Paid week 1"),
    SeriesPoint::new("Paid", 2.0, 33.0, "Paid week 2"),
    SeriesPoint::new("Paid", 3.0, 46.0, "Paid week 3"),
    SeriesPoint::new("Paid", 4.0, 58.0, "Paid week 4"),
    SeriesPoint::new("Paid", 5.0, 72.0, "Paid week 5"),
    SeriesPoint::new("Partner", 1.0, 18.0, "Partner week 1"),
    SeriesPoint::new("Partner", 2.0, 21.0, "Partner week 2"),
    SeriesPoint::new("Partner", 3.0, 28.0, "Partner week 3"),
    SeriesPoint::new("Partner", 4.0, 36.0, "Partner week 4"),
    SeriesPoint::new("Partner", 5.0, 44.0, "Partner week 5"),
];

const OPERATIONS_EXPERIMENT_LINES: &[SeriesPoint] = &[
    SeriesPoint::new("Standard", 1.0, 88.0, "Standard week 1"),
    SeriesPoint::new("Standard", 2.0, 82.0, "Standard week 2"),
    SeriesPoint::new("Standard", 3.0, 74.0, "Standard week 3"),
    SeriesPoint::new("Standard", 4.0, 67.0, "Standard week 4"),
    SeriesPoint::new("Standard", 5.0, 61.0, "Standard week 5"),
    SeriesPoint::new("Priority", 1.0, 47.0, "Priority week 1"),
    SeriesPoint::new("Priority", 2.0, 45.0, "Priority week 2"),
    SeriesPoint::new("Priority", 3.0, 39.0, "Priority week 3"),
    SeriesPoint::new("Priority", 4.0, 34.0, "Priority week 4"),
    SeriesPoint::new("Priority", 5.0, 29.0, "Priority week 5"),
    SeriesPoint::new("Escalated", 1.0, 22.0, "Escalated week 1"),
    SeriesPoint::new("Escalated", 2.0, 27.0, "Escalated week 2"),
    SeriesPoint::new("Escalated", 3.0, 31.0, "Escalated week 3"),
    SeriesPoint::new("Escalated", 4.0, 26.0, "Escalated week 4"),
    SeriesPoint::new("Escalated", 5.0, 19.0, "Escalated week 5"),
];

#[derive(Clone, Copy, Debug)]
pub struct CategoryValue {
    pub label: &'static str,
    pub value: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BarChartDataset {
    pub title: &'static str,
    pub description: &'static str,
    pub x_axis_label: &'static str,
    pub y_axis_label: &'static str,
    pub target: f32,
    pub y_max: f32,
    pub values: &'static [CategoryValue],
}

#[derive(Clone, Copy, Debug)]
pub struct SeriesPoint {
    pub series: &'static str,
    pub x: f32,
    pub value: f32,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct LineChartDataset {
    pub title: &'static str,
    pub description: &'static str,
    pub x_axis_label: &'static str,
    pub y_axis_label: &'static str,
    pub y_domain: (f32, f32),
    pub points: &'static [SeriesPoint],
}

pub fn revenue_bars(profile: DataProfile) -> BarChartDataset {
    match profile {
        DataProfile::Retail => BarChartDataset {
            title: "Revenue by Month",
            description: "Bars carry observed retail revenue; the overlay adds a fitted trend, residual band, target threshold, and outlier markers.",
            x_axis_label: "Month",
            y_axis_label: "Revenue",
            target: 21.0,
            y_max: 30.0,
            values: RETAIL_REVENUE_BARS,
        },
        DataProfile::Growth => BarChartDataset {
            title: "Pipeline by Month",
            description: "Monthly pipeline creation with a target line and residual band for spotting repeatable acquisition momentum.",
            x_axis_label: "Month",
            y_axis_label: "Pipeline",
            target: 28.0,
            y_max: 42.0,
            values: GROWTH_REVENUE_BARS,
        },
        DataProfile::Operations => BarChartDataset {
            title: "Resolved Tickets by Month",
            description: "Operational throughput with a target line and uncertainty band for capacity planning across support teams.",
            x_axis_label: "Month",
            y_axis_label: "Resolved Tickets",
            target: 72.0,
            y_max: 96.0,
            values: OPERATIONS_REVENUE_BARS,
        },
    }
}

pub fn experiment_lines(profile: DataProfile) -> LineChartDataset {
    match profile {
        DataProfile::Retail => LineChartDataset {
            title: "Experiment Lift Trend",
            description: "A multi-series line chart with endpoint labels, point tooltips, snap targets, and a shared analytical scale.",
            x_axis_label: "Week",
            y_axis_label: "Activation Index",
            y_domain: (0.0, 52.0),
            points: RETAIL_EXPERIMENT_LINES,
        },
        DataProfile::Growth => LineChartDataset {
            title: "Acquisition Cohort Trend",
            description: "Weekly activation by channel, sharing scale and guide behavior across a different growth dataset.",
            x_axis_label: "Week",
            y_axis_label: "Qualified Accounts",
            y_domain: (0.0, 88.0),
            points: GROWTH_EXPERIMENT_LINES,
        },
        DataProfile::Operations => LineChartDataset {
            title: "Queue Health Trend",
            description: "Weekly queue pressure by service tier, using the same chart component against an operations dataset.",
            x_axis_label: "Week",
            y_axis_label: "Open Cases",
            y_domain: (0.0, 120.0),
            points: OPERATIONS_EXPERIMENT_LINES,
        },
    }
}

impl CategoryValue {
    pub const fn new(label: &'static str, value: f32) -> Self {
        Self { label, value }
    }
}

impl SeriesPoint {
    pub const fn new(series: &'static str, x: f32, value: f32, label: &'static str) -> Self {
        Self {
            series,
            x,
            value,
            label,
        }
    }
}
