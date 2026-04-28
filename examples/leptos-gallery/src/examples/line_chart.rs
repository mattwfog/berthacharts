//! Line chart demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 680;
const H: u32 = 360;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(demo_line_spec());
    let summary = spec.summary();
    let peak_label = format!("{:.0}", summary.peak);
    let latest_label = format!("{:.0}", summary.latest_total);
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo line spec should be valid")
    });

    view! {
        <section id="line-chart" class="example">
            <div class="example-head">
                <div>
                    <h2>"Experiment Lift Trend"</h2>
                    <p>
                        "A multi-series line chart with endpoint labels, point tooltips, snap targets, and a shared analytical scale."
                    </p>
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
            <div class=move || chart_stage_class(
                "chart-stage line-stage",
                show_data_labels.get(),
                show_axes.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
        </section>
    }
}

fn demo_line_spec() -> LineChartSpec {
    let options = LineChartOptions {
        x_axis_label: "Week".to_string(),
        y_axis_label: "Activation Index".to_string(),
        y_domain: Some((0.0, 52.0)),
        ..LineChartOptions::default()
    };

    LineChartSpec::new(vec![
        LineDatum::new("Control", 1.0, 21.0).with_label("Control week 1"),
        LineDatum::new("Control", 2.0, 23.0).with_label("Control week 2"),
        LineDatum::new("Control", 3.0, 25.0).with_label("Control week 3"),
        LineDatum::new("Control", 4.0, 24.0).with_label("Control week 4"),
        LineDatum::new("Control", 5.0, 27.0).with_label("Control week 5"),
        LineDatum::new("Variant A", 1.0, 19.0).with_label("Variant A week 1"),
        LineDatum::new("Variant A", 2.0, 25.0).with_label("Variant A week 2"),
        LineDatum::new("Variant A", 3.0, 31.0).with_label("Variant A week 3"),
        LineDatum::new("Variant A", 4.0, 38.0).with_label("Variant A week 4"),
        LineDatum::new("Variant A", 5.0, 43.0).with_label("Variant A week 5"),
        LineDatum::new("Variant B", 1.0, 20.0).with_label("Variant B week 1"),
        LineDatum::new("Variant B", 2.0, 24.0).with_label("Variant B week 2"),
        LineDatum::new("Variant B", 3.0, 29.0).with_label("Variant B week 3"),
        LineDatum::new("Variant B", 4.0, 33.0).with_label("Variant B week 4"),
        LineDatum::new("Variant B", 5.0, 37.0).with_label("Variant B week 5"),
    ])
    .with_options(options)
}

fn chart_stage_class(
    base: &'static str,
    show_data_labels: bool,
    show_axes: bool,
    show_legend: bool,
) -> String {
    let mut class = String::from(base);
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_axes {
        class.push_str(" hide-axes");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}
