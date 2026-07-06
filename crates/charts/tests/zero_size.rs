//! Degenerate-size regression: building any chart at 0x0 (or 1x1) must not
//! panic — it is what a collapsing container feeds the kernel during page
//! teardown, and a panic there poisons the WASM handle (wasm-bindgen borrow
//! flag never clears when the trap unwinds through the &mut self frame).

use berthacharts_charts::{
    BarChartSpec, BarDatum, HeatmapCell, HeatmapSpec, LineChartSpec, LineDatum, ScatterDatum,
    ScatterPlotSpec,
};
use berthacharts_core::{ChartSize, ChartSpec, Workspace};

fn sizes() -> Vec<ChartSize> {
    vec![
        ChartSize::new(0, 0),
        ChartSize::new(1, 1),
        ChartSize::new(0, 360),
        ChartSize::new(640, 0),
        ChartSize::new(2, 2),
    ]
}

#[test]
fn bar_degenerate_sizes_do_not_panic() {
    for size in sizes() {
        let ws = Workspace::new();
        let spec = BarChartSpec::new(vec![BarDatum::new("a", 1.0), BarDatum::new("b", 2.0)]);
        let _ = spec.build_chart(ws, size);
    }
}

#[test]
fn line_degenerate_sizes_do_not_panic() {
    for size in sizes() {
        let ws = Workspace::new();
        let spec = LineChartSpec::new(vec![
            LineDatum::new("s", 0.0, 1.0),
            LineDatum::new("s", 1.0, 2.0),
        ]);
        let _ = spec.build_chart(ws, size);
    }
}

#[test]
fn scatter_degenerate_sizes_do_not_panic() {
    for size in sizes() {
        let ws = Workspace::new();
        let spec = ScatterPlotSpec::new(vec![
            ScatterDatum::new("a", 0.0, 1.0),
            ScatterDatum::new("b", 1.0, 2.0),
        ]);
        let _ = spec.build_chart(ws, size);
    }
}

#[test]
fn heatmap_degenerate_sizes_do_not_panic() {
    for size in sizes() {
        let ws = Workspace::new();
        let spec = HeatmapSpec::new(vec![
            HeatmapCell::new("r1", "c1", 0.5),
            HeatmapCell::new("r1", "c2", 0.7),
        ]);
        let _ = spec.build_chart(ws, size);
    }
}
