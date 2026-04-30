//! Downstream-style smoke tests for the facade crate.

use berthacharts::prelude::*;

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
