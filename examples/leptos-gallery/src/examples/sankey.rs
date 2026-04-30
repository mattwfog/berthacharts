//! Sankey chart demo built through the public `berthacharts-network` spec.

use std::sync::Arc;

use berthacharts_core::{ChartSize, ChartSpec};
use berthacharts_network::sankey::{
    SankeyLegendItem, SankeyLink, SankeyNode, SankeySpec, SankeyStage,
};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 620;
const H: u32 = 400;

#[component]
pub fn View() -> impl IntoView {
    let show_columns = RwSignal::new(true);
    let show_nodes = RwSignal::new(true);
    let show_data_labels = RwSignal::new(false);
    let show_legend = RwSignal::new(true);
    let show_diagnostics = RwSignal::new(false);
    let spec = Arc::new(demo_sankey_spec());
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo sankey spec should be valid")
    });

    view! {
        <section id="sankey" class="example">
            <div class="example-head">
                <div>
                    <h2>"Sankey Flow"</h2>
                    <p>
                        "Flow ribbons are hit-testable GPU triangles with node and link diagnostics."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>"156"</strong>" starts"</span>
                    <span><strong>"92"</strong>" won"</span>
                    <span><strong>"64"</strong>" lost"</span>
                    <span><strong>"59%"</strong>" win rate"</span>
                </div>
            </div>
            <DisplayControls label="Sankey display options">
                <DisplayToggleButton label="Stage labels" state=show_columns />
                <DisplayToggleButton label="Node labels" state=show_nodes />
                <DisplayToggleButton label="Legend" state=show_legend />
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Diagnostics" state=show_diagnostics />
            </DisplayControls>
            <div class=move || sankey_stage_class(
                show_columns.get(),
                show_nodes.get(),
                show_data_labels.get(),
                show_legend.get(),
                show_diagnostics.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
                <div class="sankey-diagnostics">
                    <div><span>"Source mix"</span><strong>"70 organic / 54 paid / 32 referral"</strong><em>"Organic supplies the largest entry stream at 45% of starts."</em></div>
                    <div><span>"Qualification"</span><strong>"84 trial / 72 sales"</strong><em>"Trial carries 54% of starts; sales-assist carries 46%."</em></div>
                    <div><span>"Largest loss"</span><strong>"Sales to Lost: 38"</strong><em>"24% of starts exit after sales qualification."</em></div>
                    <div><span>"Best conversion"</span><strong>"Referral to Sales: 100%"</strong><em>"Partner-originated flow avoids the trial branch."</em></div>
                </div>
            </div>
        </section>
    }
}

fn demo_sankey_spec() -> SankeySpec {
    SankeySpec::new(
        vec![
            SankeyNode::new("website", "Website", 0, [0.10, 0.42, 0.76]),
            SankeyNode::new("organic", "Organic", 1, [0.16, 0.55, 0.84]),
            SankeyNode::new("paid", "Paid", 1, [0.18, 0.63, 0.68]),
            SankeyNode::new("referral", "Referral", 1, [0.32, 0.68, 0.51]),
            SankeyNode::new("trial", "Trial", 2, [0.17, 0.64, 0.74]),
            SankeyNode::new("sales", "Sales", 2, [0.49, 0.63, 0.42]),
            SankeyNode::new("won", "Won", 3, [0.10, 0.64, 0.48]),
            SankeyNode::new("lost", "Lost", 3, [0.83, 0.42, 0.34]),
        ],
        vec![
            SankeyLink::new(
                "website",
                "organic",
                70.0,
                "source mix",
                rgba(0.07, 0.45, 0.78, 0.62),
            ),
            SankeyLink::new(
                "website",
                "paid",
                54.0,
                "source mix",
                rgba(0.05, 0.58, 0.64, 0.58),
            ),
            SankeyLink::new(
                "website",
                "referral",
                32.0,
                "source mix",
                rgba(0.12, 0.60, 0.43, 0.54),
            ),
            SankeyLink::new(
                "organic",
                "trial",
                48.0,
                "activation",
                rgba(0.05, 0.48, 0.74, 0.58),
            ),
            SankeyLink::new(
                "organic",
                "sales",
                22.0,
                "sales assist",
                rgba(0.38, 0.53, 0.28, 0.46),
            ),
            SankeyLink::new(
                "paid",
                "trial",
                36.0,
                "activation",
                rgba(0.05, 0.58, 0.64, 0.56),
            ),
            SankeyLink::new(
                "paid",
                "sales",
                18.0,
                "sales assist",
                rgba(0.38, 0.53, 0.28, 0.46),
            ),
            SankeyLink::new(
                "referral",
                "sales",
                32.0,
                "partner lift",
                rgba(0.12, 0.60, 0.43, 0.52),
            ),
            SankeyLink::new(
                "trial",
                "won",
                58.0,
                "converted",
                rgba(0.04, 0.56, 0.38, 0.64),
            ),
            SankeyLink::new(
                "trial",
                "lost",
                26.0,
                "drop-off",
                rgba(0.74, 0.25, 0.20, 0.50),
            ),
            SankeyLink::new(
                "sales",
                "won",
                34.0,
                "converted",
                rgba(0.04, 0.56, 0.38, 0.58),
            ),
            SankeyLink::new(
                "sales",
                "lost",
                38.0,
                "drop-off",
                rgba(0.74, 0.25, 0.20, 0.50),
            ),
        ],
    )
    .with_stages(vec![
        SankeyStage::new(0, "acquire").with_detail("156 starts"),
        SankeyStage::new(1, "source").with_detail("45 / 35 / 21%"),
        SankeyStage::new(2, "qualify").with_detail("84 trial / 72 sales"),
        SankeyStage::new(3, "outcome").with_detail("92 won / 64 lost"),
    ])
    .with_legend(vec![
        SankeyLegendItem::new("converted", [0.04, 0.56, 0.38, 0.78]),
        SankeyLegendItem::new("activation", [0.07, 0.46, 0.74, 0.78]),
        SankeyLegendItem::new("drop-off", [0.74, 0.25, 0.20, 0.70]),
    ])
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

fn sankey_stage_class(
    show_columns: bool,
    show_nodes: bool,
    show_data_labels: bool,
    show_legend: bool,
    show_diagnostics: bool,
) -> String {
    let mut class = String::from("chart-stage sankey-stage");
    if !show_columns {
        class.push_str(" hide-column-labels");
    }
    if !show_nodes {
        class.push_str(" hide-node-labels");
    }
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    if !show_diagnostics {
        class.push_str(" hide-diagnostics");
    }
    class
}
