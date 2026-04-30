//! Sunburst chart demo built through the public `berthacharts-network` spec.

use std::sync::Arc;

use berthacharts_core::{ChartSize, ChartSpec};
use berthacharts_network::sunburst::{
    SunburstLegendItem, SunburstNode, SunburstOptions, SunburstSpec,
};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 680;
const H: u32 = 470;

#[component]
pub fn View() -> impl IntoView {
    let show_segments = RwSignal::new(true);
    let show_data_labels = RwSignal::new(true);
    let show_legend = RwSignal::new(true);
    let show_diagnostics = RwSignal::new(false);
    let demo_spec = demo_sunburst_spec();
    let summary = demo_spec
        .summary()
        .expect("demo sunburst summary should be valid");
    let total_label = format!("{:.0}", summary.total);
    let structure_label = format!(
        "{} rings / {} leaves",
        summary.max_depth + 1,
        summary.leaves
    );
    let largest_label = format!(
        "{}: {:.0}",
        short_path(&summary.largest_leaf_path),
        summary.largest_leaf_value
    );
    let concentration_label = format!("{:.0}% top 3", summary.top_three_leaf_share);
    let branch_mix_label = summary
        .branches
        .iter()
        .map(|branch| format!("{} {:.0}", branch.label, branch.value))
        .collect::<Vec<_>>()
        .join(" / ");
    let branch_leader_label = summary
        .branches
        .iter()
        .map(|branch| {
            format!(
                "{}: {} {:.0}%",
                branch.label, branch.largest_leaf_label, branch.largest_leaf_share
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let spec = Arc::new(demo_spec);
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo sunburst spec should be valid")
    });

    view! {
        <section id="sunburst" class="example">
            <div class="example-head">
                <div>
                    <h2>"Revenue Sunburst"</h2>
                    <p>
                        "A four-ring revenue hierarchy with branch share, parent mix, leaf counts, labels, tooltips, and snap targets."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>{total_label}</strong>" total"</span>
                    <span><strong>"76"</strong>" acquisition"</span>
                    <span><strong>"69"</strong>" growth"</span>
                    <span><strong>"42"</strong>" renewal"</span>
                    <span><strong>{concentration_label}</strong>" concentration"</span>
                </div>
            </div>
            <DisplayControls label="Sunburst display options">
                <DisplayToggleButton label="Segment labels" state=show_segments />
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Legend" state=show_legend />
                <DisplayToggleButton label="Diagnostics" state=show_diagnostics />
            </DisplayControls>
            <div class=move || sunburst_stage_class(
                show_segments.get(),
                show_data_labels.get(),
                show_legend.get(),
                show_diagnostics.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
                <div class="sunburst-diagnostics">
                    <div><span>"Largest path"</span><strong>{largest_label}</strong><em>"Enterprise direct contributes 24% of acquisition and 10% of total."</em></div>
                    <div><span>"Branch mix"</span><strong>{branch_mix_label}</strong><em>"Branch values are computed from descendant leaves, then sorted by configured order/value."</em></div>
                    <div><span>"Branch leaders"</span><strong>{branch_leader_label}</strong><em>"Each branch exposes its largest leaf and share-of-branch for concentration checks."</em></div>
                    <div><span>"Structure"</span><strong>{structure_label}</strong><em>"Every sector exposes path, depth, leaves, sibling rank, parent share, and total share on hover."</em></div>
                </div>
            </div>
        </section>
    }
}

fn demo_sunburst_spec() -> SunburstSpec {
    SunburstSpec::new(vec![
        SunburstNode::new(
            "revenue",
            "Revenue",
            None::<String>,
            0.0,
            "root",
            rgba(0.16, 0.20, 0.28, 1.0),
        ),
        SunburstNode::new(
            "new",
            "Acquisition",
            Some("revenue"),
            0.0,
            "acquisition",
            rgba(0.08, 0.43, 0.72, 0.86),
        )
        .with_order(0),
        SunburstNode::new(
            "expansion",
            "Expansion",
            Some("revenue"),
            0.0,
            "growth",
            rgba(0.13, 0.58, 0.52, 0.84),
        )
        .with_order(1),
        SunburstNode::new(
            "retention",
            "Renewal",
            Some("revenue"),
            0.0,
            "renewal",
            rgba(0.67, 0.39, 0.16, 0.82),
        )
        .with_order(2),
        SunburstNode::new(
            "new/direct",
            "Direct",
            Some("new"),
            0.0,
            "acquisition",
            rgba(0.08, 0.43, 0.72, 0.86),
        )
        .with_order(0),
        SunburstNode::new(
            "new/partner",
            "Partner",
            Some("new"),
            0.0,
            "acquisition",
            rgba(0.07, 0.48, 0.64, 0.82),
        )
        .with_order(1),
        SunburstNode::new(
            "new/product",
            "Product-led",
            Some("new"),
            0.0,
            "acquisition",
            rgba(0.16, 0.55, 0.78, 0.80),
        )
        .with_order(2),
        SunburstNode::new(
            "expansion/seats",
            "Seats",
            Some("expansion"),
            0.0,
            "growth",
            rgba(0.13, 0.58, 0.52, 0.84),
        )
        .with_order(0),
        SunburstNode::new(
            "expansion/usage",
            "Usage",
            Some("expansion"),
            0.0,
            "growth",
            rgba(0.20, 0.62, 0.42, 0.80),
        )
        .with_order(1),
        SunburstNode::new(
            "expansion/services",
            "Services",
            Some("expansion"),
            0.0,
            "growth",
            rgba(0.35, 0.59, 0.32, 0.78),
        )
        .with_order(2),
        SunburstNode::new(
            "retention/renewal",
            "Contract",
            Some("retention"),
            0.0,
            "renewal",
            rgba(0.67, 0.39, 0.16, 0.82),
        )
        .with_order(0),
        SunburstNode::new(
            "retention/save",
            "Save",
            Some("retention"),
            0.0,
            "renewal",
            rgba(0.73, 0.48, 0.20, 0.78),
        ),
        SunburstNode::new(
            "new/direct/enterprise",
            "Enterprise",
            Some("new/direct"),
            18.0,
            "acquisition",
            rgba(0.06, 0.36, 0.68, 0.88),
        ),
        SunburstNode::new(
            "new/direct/midmarket",
            "Mid-market",
            Some("new/direct"),
            14.0,
            "acquisition",
            rgba(0.08, 0.44, 0.74, 0.84),
        ),
        SunburstNode::new(
            "new/direct/smb",
            "SMB",
            Some("new/direct"),
            10.0,
            "acquisition",
            rgba(0.14, 0.52, 0.78, 0.80),
        ),
        SunburstNode::new(
            "new/partner/agency",
            "Agency",
            Some("new/partner"),
            12.0,
            "acquisition",
            rgba(0.05, 0.44, 0.58, 0.84),
        ),
        SunburstNode::new(
            "new/partner/integration",
            "Integration",
            Some("new/partner"),
            9.0,
            "acquisition",
            rgba(0.08, 0.52, 0.62, 0.80),
        ),
        SunburstNode::new(
            "new/product/selfserve",
            "Self-serve",
            Some("new/product"),
            8.0,
            "acquisition",
            rgba(0.15, 0.50, 0.76, 0.80),
        ),
        SunburstNode::new(
            "new/product/community",
            "Community",
            Some("new/product"),
            5.0,
            "acquisition",
            rgba(0.20, 0.58, 0.80, 0.76),
        ),
        SunburstNode::new(
            "expansion/seats/enterprise",
            "Enterprise",
            Some("expansion/seats"),
            17.0,
            "growth",
            rgba(0.10, 0.50, 0.44, 0.84),
        ),
        SunburstNode::new(
            "expansion/seats/midmarket",
            "Mid-market",
            Some("expansion/seats"),
            9.0,
            "growth",
            rgba(0.14, 0.58, 0.50, 0.80),
        ),
        SunburstNode::new(
            "expansion/seats/smb",
            "SMB",
            Some("expansion/seats"),
            5.0,
            "growth",
            rgba(0.20, 0.64, 0.56, 0.76),
        ),
        SunburstNode::new(
            "expansion/usage/compute",
            "Compute",
            Some("expansion/usage"),
            14.0,
            "growth",
            rgba(0.18, 0.58, 0.35, 0.82),
        ),
        SunburstNode::new(
            "expansion/usage/api",
            "API",
            Some("expansion/usage"),
            10.0,
            "growth",
            rgba(0.24, 0.64, 0.40, 0.78),
        ),
        SunburstNode::new(
            "expansion/services/onboarding",
            "Onboarding",
            Some("expansion/services"),
            8.0,
            "growth",
            rgba(0.32, 0.56, 0.30, 0.78),
        ),
        SunburstNode::new(
            "expansion/services/advisory",
            "Advisory",
            Some("expansion/services"),
            6.0,
            "growth",
            rgba(0.40, 0.62, 0.34, 0.74),
        ),
        SunburstNode::new(
            "retention/renewal/annual",
            "Annual",
            Some("retention/renewal"),
            20.0,
            "renewal",
            rgba(0.66, 0.36, 0.14, 0.82),
        ),
        SunburstNode::new(
            "retention/renewal/multiyear",
            "Multi-year",
            Some("retention/renewal"),
            9.0,
            "renewal",
            rgba(0.72, 0.43, 0.18, 0.78),
        ),
        SunburstNode::new(
            "retention/save/concession",
            "Concession",
            Some("retention/save"),
            7.0,
            "renewal",
            rgba(0.72, 0.47, 0.19, 0.78),
        ),
        SunburstNode::new(
            "retention/save/success",
            "Success",
            Some("retention/save"),
            6.0,
            "renewal",
            rgba(0.78, 0.53, 0.23, 0.74),
        ),
    ])
    .with_legend(vec![
        SunburstLegendItem::new("acquisition", [0.08 * 0.86, 0.43 * 0.86, 0.72 * 0.86, 0.86]),
        SunburstLegendItem::new("growth", [0.13 * 0.84, 0.58 * 0.84, 0.52 * 0.84, 0.84]),
        SunburstLegendItem::new("renewal", [0.67 * 0.82, 0.39 * 0.82, 0.16 * 0.82, 0.82]),
    ])
    .with_options(SunburstOptions {
        padding: 28.0,
        inner_radius: 58.0,
        ring_gap: 3.0,
        angular_gap: 0.008,
        max_visible_labels: 36,
        ..SunburstOptions::default()
    })
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

fn short_path(path: &str) -> String {
    path.split(" / ").skip(2).collect::<Vec<_>>().join(" / ")
}

fn sunburst_stage_class(
    show_segments: bool,
    show_data_labels: bool,
    show_legend: bool,
    show_diagnostics: bool,
) -> String {
    let mut class = String::from("chart-stage sunburst-stage");
    if !show_segments {
        class.push_str(" hide-column-labels hide-node-labels");
    }
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    if !show_diagnostics {
        class.push_str(" hide-sunburst-diagnostics");
    }
    class
}
