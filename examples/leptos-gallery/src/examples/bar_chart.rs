//! Bar chart demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::bar::{BarChartOptions, BarChartSpec, BarDatum};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 680;
const H: u32 = 390;
const TARGET_VALUE: f32 = 21.0;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_annotations = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(demo_bar_spec());
    let summary = spec.summary();
    let slope_label = format_slope(summary.slope);
    let sigma_label = format!("{:.1}", summary.sigma);
    let peak_label = format!("{:.0}", summary.peak);
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo bar spec should be valid")
    });

    view! {
        <section id="bar-chart" class="example">
            <div class="example-head">
                <div>
                    <h2>"Revenue by Month"</h2>
                    <p>
                        "Bars carry the observed values; the overlay adds a fitted trend, residual band, "
                        "target threshold, and outlier markers."
                    </p>
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
            <div class=move || bar_stage_class(
                show_data_labels.get(),
                show_axes.get(),
                show_annotations.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
                <div class="bar-annotations">
                    <span class="chart-annotation target-annotation">"target 21"</span>
                    <span class="chart-annotation trend-annotation">"linear fit + uncertainty"</span>
                </div>
            </div>
        </section>
    }
}

fn demo_bar_spec() -> BarChartSpec {
    let mut options = BarChartOptions {
        target: Some(TARGET_VALUE),
        x_axis_label: "Month".to_string(),
        y_axis_label: "Revenue".to_string(),
        ..BarChartOptions::default()
    };
    options.y_max = Some(30.0);

    BarChartSpec::new(vec![
        BarDatum::new("Jan", 12.0),
        BarDatum::new("Feb", 19.0),
        BarDatum::new("Mar", 7.0),
        BarDatum::new("Apr", 22.0),
        BarDatum::new("May", 16.0),
        BarDatum::new("Jun", 25.0),
    ])
    .with_options(options)
}

fn format_slope(slope: f32) -> String {
    if slope >= 0.0 {
        format!("+{slope:.1}/mo")
    } else {
        format!("{slope:.1}/mo")
    }
}

fn bar_stage_class(
    show_data_labels: bool,
    show_axes: bool,
    show_annotations: bool,
    show_legend: bool,
) -> String {
    let mut class = String::from("chart-stage bar-stage");
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_axes {
        class.push_str(" hide-axes");
    }
    if !show_annotations {
        class.push_str(" hide-annotations");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}
