//! Heatmap demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::heatmap::{HeatmapCell, HeatmapSpec};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 420;
const H: u32 = 320;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_headers = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(demo_heatmap_spec());
    let summary = spec.summary();
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo heatmap spec should be valid")
    });

    view! {
        <section id="grid" class="example">
            <div class="example-head">
                <div>
                    <h2>"Cohort Heatmap"</h2>
                    <p>
                        "A cohort matrix with baseline deltas, signal glyphs, and per-cell tooltips."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>{summary.cells.to_string()}</strong>" cells"</span>
                    <span><strong>{summary.strong.to_string()}</strong>" strong"</span>
                    <span><strong>{summary.watch.to_string()}</strong>" watch"</span>
                </div>
            </div>
            <DisplayControls label="Heatmap display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Headers" state=show_headers />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class=move || heatmap_stage_class(
                show_data_labels.get(),
                show_headers.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
                <div class="heatmap-labels heatmap-cols">
                    <span>"Conversion"</span><span>"Activation"</span><span>"Retention"</span><span>"Revenue"</span>
                </div>
                <div class="heatmap-labels heatmap-rows">
                    <span>"SMB"</span><span>"Midmarket"</span><span>"Enterprise"</span>
                </div>
            </div>
        </section>
    }
}

fn demo_heatmap_spec() -> HeatmapSpec {
    let segments = ["SMB", "Midmarket", "Enterprise"];
    let metrics = ["Conversion", "Activation", "Retention", "Revenue"];
    let scores = [
        [0.82, 0.71, 0.57, 0.67],
        [0.76, 0.64, 0.51, 0.56],
        [0.67, 0.60, 0.47, 0.50],
    ];
    let baselines = [0.70, 0.66, 0.58, 0.60];
    let mut cells = Vec::with_capacity(segments.len() * metrics.len());

    for (row, segment) in segments.iter().enumerate() {
        for (column, metric) in metrics.iter().enumerate() {
            cells.push(
                HeatmapCell::new(*segment, *metric, scores[row][column])
                    .with_baseline(baselines[column]),
            );
        }
    }

    HeatmapSpec::new(cells)
        .with_rows(segments.to_vec())
        .with_columns(metrics.to_vec())
}

fn heatmap_stage_class(show_data_labels: bool, show_headers: bool, show_legend: bool) -> String {
    let mut class = String::from("chart-stage heatmap-stage");
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_headers {
        class.push_str(" hide-headers");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}
