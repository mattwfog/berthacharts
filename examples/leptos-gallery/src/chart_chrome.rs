//! Shared chart chrome controls for the gallery.
//!
//! The chart specs own the data and guides. This module owns small Leptos
//! controls that switch guide visibility without duplicating button markup in
//! every example.

use leptos::prelude::*;

#[component]
pub fn DisplayControls(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <div class="display-controls" aria-label=label>
            {children()}
        </div>
    }
}

#[component]
pub fn DisplayToggleButton(#[prop(into)] label: String, state: RwSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || display_toggle_class(state.get())
            aria-pressed=move || state.get().to_string()
            on:click=move |_| state.update(|value| *value = !*value)
        >
            {label}
        </button>
    }
}

fn display_toggle_class(active: bool) -> &'static str {
    if active {
        "display-toggle is-active"
    } else {
        "display-toggle"
    }
}

pub fn stage_class(base: &str, hidden_when_false: &[(&str, bool)]) -> String {
    let mut class = String::from(base);
    for (hidden_class, visible) in hidden_when_false {
        if !*visible {
            class.push(' ');
            class.push_str(hidden_class);
        }
    }
    class
}
