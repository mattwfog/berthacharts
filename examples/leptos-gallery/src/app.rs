//! App shell + routing.
//!
//! The gallery catalog owns section order and anchors. The shell renders that
//! registry and provides runtime context to each example.

use leptos::prelude::*;

use crate::gallery;

/// Top-level app.
#[component]
pub fn App() -> impl IntoView {
    let runtime = gallery::detect_runtime();
    let meta_label = runtime.meta_label();
    provide_context(runtime);

    let nav_items = gallery::EXAMPLES
        .iter()
        .map(|example| {
            view! {
                <a href=example.href()>{example.nav_label}</a>
            }
        })
        .collect::<Vec<_>>();

    let examples = gallery::EXAMPLES
        .iter()
        .copied()
        .map(gallery::render_example)
        .collect::<Vec<_>>();

    view! {
        <header>
            <h1>"Bertha Charts"</h1>
            <span class="meta">{meta_label}</span>
        </header>

        <nav>
            {nav_items}
        </nav>

        <main>
            {examples}
        </main>
    }
}
