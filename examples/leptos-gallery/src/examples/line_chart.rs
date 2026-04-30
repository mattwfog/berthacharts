//! Line chart demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use leptos::prelude::*;

use crate::chart_canvas::{chart_builder, ChartCanvas};
use crate::chart_chrome::{stage_class, DisplayControls, DisplayToggleButton};
use crate::data::{self, LineChartDataset};
use crate::gallery::runtime_context;

const W: u32 = 680;
const H: u32 = 360;

#[component]
pub fn View() -> impl IntoView {
    let runtime = runtime_context();
    let dataset = data::experiment_lines(runtime.data_profile);
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(line_spec(dataset));
    let summary = spec.summary();
    let peak_label = format!("{:.0}", summary.peak);
    let latest_label = format!("{:.0}", summary.latest_total);
    let build = chart_builder(spec.clone(), W, H, "demo line spec");

    view! {
        <section id="line-chart" class="example">
            <div class="example-head">
                <div>
                    <h2>{dataset.title}</h2>
                    <p>{dataset.description}</p>
                </div>
                <div class="stat-strip">
                    <span><strong>{summary.series.to_string()}</strong>" series"</span>
                    <span><strong>{summary.points.to_string()}</strong>" points"</span>
                    <span><strong>{peak_label}</strong>" peak"</span>
                    <span><strong>{latest_label}</strong>" latest total"</span>
                </div>
            </div>
            <DisplayControls label="Line chart display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Axes" state=show_axes />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class=move || stage_class("chart-stage line-stage", &[
                ("hide-data-labels", show_data_labels.get()),
                ("hide-axes", show_axes.get()),
                ("hide-legend", show_legend.get()),
            ])>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
        </section>
    }
}

fn line_spec(dataset: LineChartDataset) -> LineChartSpec {
    let options = LineChartOptions {
        x_axis_label: dataset.x_axis_label.to_string(),
        y_axis_label: dataset.y_axis_label.to_string(),
        y_domain: Some(dataset.y_domain),
        ..LineChartOptions::default()
    };

    LineChartSpec::new(
        dataset
            .points
            .iter()
            .map(|point| LineDatum::new(point.series, point.x, point.value).with_label(point.label))
            .collect(),
    )
    .with_options(options)
}
