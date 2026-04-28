//! App shell + routing.
//!
//! The gallery is a flat list today — no routing state to manage beyond
//! scroll anchors. When the example set grows, swap in `leptos_router`.

use leptos::prelude::*;

use crate::examples;

/// Top-level app.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <header>
            <h1>"Bertha Charts"</h1>
            <span class="meta">"v0.0.1 · Leptos · WebGL2"</span>
        </header>

        <nav>
            <a href="#hello-rect">"Layers"</a>
            <a href="#bar-chart">"Revenue Bars"</a>
            <a href="#line-chart">"Lines"</a>
            <a href="#scatter-plot">"Scatter"</a>
            <a href="#grid">"Heatmap"</a>
            <a href="#sankey">"Sankey"</a>
        </nav>

        <main>
            <examples::hello_rect::View />
            <examples::bar_chart::View />
            <examples::line_chart::View />
            <examples::scatter_plot::View />
            <examples::grid::View />
            <examples::sankey::View />
        </main>
    }
}
