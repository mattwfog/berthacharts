//! Downstream-style smoke tests for the facade crate.

#[cfg(any(
    feature = "charts",
    feature = "network",
    feature = "geo",
    feature = "dist",
    feature = "finance"
))]
use berthacharts::prelude::*;

#[test]
fn root_exports_core_chart_size() {
    let viewport = berthacharts::ChartSize::new(320, 240).full_viewport();

    assert_eq!(viewport.width, 320);
    assert_eq!(viewport.height, 240);
}

#[cfg(feature = "charts")]
#[test]
fn prelude_builds_a_bar_chart() {
    let spec = BarChartSpec::new(vec![
        BarDatum::new("Q1", 24.0),
        BarDatum::new("Q2", 31.0),
        BarDatum::new("Q3", 37.0),
    ])
    .with_target(30.0);

    let summary = spec.summary();
    assert_eq!(summary.peak, 37.0);
    assert_eq!(summary.above_target, 2);

    let chart = spec
        .build(ChartSize::new(640, 360))
        .expect("bar chart should build");

    assert!(chart.is_dirty());
    assert!(!chart.scene().layers.is_empty());
    assert!(!chart.scene().guides.is_empty());
    assert!(!chart.snap_targets().is_empty());
}

#[cfg(feature = "charts")]
#[test]
fn namespaced_imports_build_a_scatter_plot() {
    let spec = berthacharts::charts::ScatterPlotSpec::new(vec![
        berthacharts::charts::ScatterDatum::new("A", 1.0, 2.0).with_group("north"),
        berthacharts::charts::ScatterDatum::new("B", 2.0, 4.0).with_group("north"),
        berthacharts::charts::ScatterDatum::new("C", 3.0, 3.0).with_group("south"),
    ]);

    let chart = spec
        .try_build_chart(
            berthacharts::core::Workspace::new(),
            berthacharts::core::ChartSize::new(480, 320),
        )
        .expect("scatter plot should build");

    assert_eq!(spec.summary().groups, 2);
    assert_eq!(chart.scene().layers.len(), 1);
}

#[cfg(feature = "charts")]
#[test]
fn root_exports_build_a_line_chart() {
    let chart = berthacharts::LineChartSpec::new(vec![
        berthacharts::LineDatum::new("actual", 1.0, 12.0),
        berthacharts::LineDatum::new("actual", 2.0, 18.0),
    ])
    .build(berthacharts::ChartSize::new(480, 280))
    .expect("line chart should build");

    assert!(!chart.scene().guides.is_empty());
}

#[cfg(feature = "charts")]
#[test]
fn prelude_builds_a_histogram() {
    let samples: Vec<f32> = (0..200).map(|i| (i % 17) as f32 * 0.5).collect();
    let chart = HistogramSpec::new(samples)
        .build(ChartSize::new(640, 360))
        .expect("histogram should build");

    assert!(!chart.scene().layers.is_empty());
    assert!(!chart.scene().guides.is_empty());
}

#[cfg(feature = "dist")]
#[test]
fn prelude_builds_a_boxplot() {
    let chart = BoxPlotSpec::new(vec![
        BoxPlotGroup::new("control", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
        BoxPlotGroup::new("variant", vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0]),
    ])
    .build(ChartSize::new(640, 360))
    .expect("boxplot should build");

    assert!(!chart.scene().layers.is_empty());
    assert!(!chart.scene().guides.is_empty());
}

#[cfg(feature = "finance")]
#[test]
fn prelude_builds_a_candlestick_chart() {
    let chart = CandlestickSpec::new(vec![
        Candle::new(1, 10.0, 12.0, 9.5, 11.0),
        Candle::new(2, 11.0, 13.0, 10.5, 12.5),
        Candle::new(3, 12.5, 12.8, 10.0, 10.4),
    ])
    .build(ChartSize::new(640, 360))
    .expect("candlestick chart should build");

    assert!(!chart.scene().layers.is_empty());
    assert!(!chart.scene().guides.is_empty());
}

#[cfg(feature = "network")]
#[test]
fn prelude_builds_a_sankey_chart() {
    let chart = SankeySpec::from_flows(vec![
        SankeyFlow::new("Lead", "Qualified", 100.0),
        SankeyFlow::new("Qualified", "Won", 40.0),
    ])
    .build(ChartSize::new(640, 320))
    .expect("sankey chart should build");

    assert!(!chart.snap_targets().is_empty());
}

#[cfg(feature = "geo")]
#[test]
fn prelude_builds_a_geo_map() {
    let chart = GeoMapSpec::new(vec![GeoFeature::new(
        "route",
        GeoGeometry::LineString(vec![
            GeoPosition::new(-122.42, 37.77),
            GeoPosition::new(-73.98, 40.75),
        ]),
    )])
    .build(ChartSize::new(640, 320))
    .expect("geo map should build");

    assert_eq!(chart.scene().layers.len(), 1);
}
