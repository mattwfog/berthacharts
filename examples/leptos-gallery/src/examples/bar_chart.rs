//! Bar chart demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::bar::{BarChartOptions, BarChartSpec, BarDatum};
use leptos::prelude::*;

use crate::chart_canvas::{chart_builder, ChartCanvas};
use crate::chart_chrome::{stage_class, DisplayControls, DisplayToggleButton};
use crate::data::{self, BarChartDataset};
use crate::gallery::runtime_context;

const W: u32 = 680;
const H: u32 = 390;

#[component]
pub fn View() -> impl IntoView {
    let runtime = runtime_context();
    let dataset = data::revenue_bars(runtime.data_profile);
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_annotations = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(bar_spec(dataset));
    let summary = spec.summary();
    let slope_label = format_slope(summary.slope);
    let sigma_label = format!("{:.1}", summary.sigma);
    let peak_label = format!("{:.0}", summary.peak);
    let build = chart_builder(spec.clone(), W, H, "demo bar spec");

    view! {
        <section id="bar-chart" class="example">
            <div class="example-head">
                <div>
                    <h2>{dataset.title}</h2>
                    <p>{dataset.description}</p>
                </div>
                <div class="stat-strip">
                    <span><strong>{peak_label}</strong>" peak"</span>
                    <span><strong>{summary.above_target.to_string()}</strong>" above target"</span>
                    <span><strong>{slope_label}</strong>" slope"</span>
                    <span><strong>{sigma_label}</strong>" sigma"</span>
                </div>
            </div>
            <DisplayControls label="Bar chart display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Axes" state=show_axes />
                <DisplayToggleButton label="Annotations" state=show_annotations />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class=move || stage_class("chart-stage bar-stage", &[
                ("hide-data-labels", show_data_labels.get()),
                ("hide-axes", show_axes.get()),
                ("hide-annotations", show_annotations.get()),
                ("hide-legend", show_legend.get()),
            ])>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
        </section>
    }
}

fn bar_spec(dataset: BarChartDataset) -> BarChartSpec {
    let mut options = BarChartOptions {
        target: Some(dataset.target),
        x_axis_label: dataset.x_axis_label.to_string(),
        y_axis_label: dataset.y_axis_label.to_string(),
        ..BarChartOptions::default()
    };
    options.y_max = Some(dataset.y_max);

    BarChartSpec::new(
        dataset
            .values
            .iter()
            .map(|datum| BarDatum::new(datum.label, datum.value))
            .collect(),
    )
    .with_options(options)
}

fn format_slope(slope: f32) -> String {
    if slope >= 0.0 {
        format!("+{slope:.1}/mo")
    } else {
        format!("{slope:.1}/mo")
    }
}
