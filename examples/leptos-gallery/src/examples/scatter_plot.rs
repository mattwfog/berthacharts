//! Scatter plot demo built through the public `berthacharts-charts` spec.

use std::sync::Arc;

use berthacharts_charts::scatter::{ScatterDatum, ScatterPlotOptions, ScatterPlotSpec};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 620;
const H: u32 = 360;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(demo_scatter_spec());
    let summary = spec.summary();
    let correlation_label = format!("{:.2}", summary.correlation);
    let slope_label = format!("{:+.1}", summary.slope);
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo scatter spec should be valid")
    });

    view! {
        <section id="scatter-plot" class="example">
            <div class="example-head">
                <div>
                    <h2>"Discovery Scatter"</h2>
                    <p>
                        "Grouped points compare evidence strength against measured impact, with residual labels and a fitted trend."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>{summary.points.to_string()}</strong>" points"</span>
                    <span><strong>{summary.groups.to_string()}</strong>" groups"</span>
                    <span><strong>{correlation_label}</strong>" corr"</span>
                    <span><strong>{slope_label}</strong>" slope"</span>
                </div>
            </div>
            <DisplayControls label="Scatter plot display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Axes" state=show_axes />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class=move || chart_stage_class(
                "chart-stage scatter-stage",
                show_data_labels.get(),
                show_axes.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
        </section>
    }
}

fn demo_scatter_spec() -> ScatterPlotSpec {
    let options = ScatterPlotOptions {
        x_axis_label: "Evidence strength".to_string(),
        y_axis_label: "Measured impact".to_string(),
        x_domain: Some((1.5, 7.2)),
        y_domain: Some((3.0, 16.0)),
        max_visible_labels: 5,
        ..ScatterPlotOptions::default()
    };

    ScatterPlotSpec::new(vec![
        ScatterDatum::new("Alpha", 2.1, 4.0).with_group("baseline"),
        ScatterDatum::new("Beta", 2.8, 5.2).with_group("baseline"),
        ScatterDatum::new("Gamma", 3.4, 6.1).with_group("baseline"),
        ScatterDatum::new("Delta", 4.0, 8.8).with_group("expansion"),
        ScatterDatum::new("Epsilon", 4.7, 9.4).with_group("expansion"),
        ScatterDatum::new("Zeta", 5.5, 11.8).with_group("expansion"),
        ScatterDatum::new("Eta", 6.1, 13.2).with_group("frontier").with_radius(6.4),
        ScatterDatum::new("Theta", 6.8, 12.4).with_group("frontier").with_radius(6.0),
        ScatterDatum::new("Iota", 5.0, 7.2).with_group("watch"),
        ScatterDatum::new("Kappa", 6.4, 9.0).with_group("watch"),
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
