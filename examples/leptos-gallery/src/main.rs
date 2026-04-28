//! Entry point — mounts the Leptos app.

use leptos::mount::mount_to_body;

mod annotation_layer;
mod app;
mod chart_canvas;
mod chart_chrome;
mod dom_events;
mod examples;
mod guide_overlay;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Info);
    log::info!("berthacharts gallery booting");
    mount_to_body(app::App);
}
