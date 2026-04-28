//! Renderer-mount component and helpers.
//!
//! [`ChartCanvas`] is the simplest drop-in: a `<canvas>` of the requested
//! logical size that initializes the wgpu renderer once on mount and draws
//! the chart returned by `builder`. For compositions that need overlays
//! around the canvas, use [`mount_renderer`] inside your own component.

use std::sync::Arc;

use berthacharts_core::{Chart, Workspace};
#[cfg(target_arch = "wasm32")]
use berthacharts_renderer_wgpu::Renderer;
use leptos::prelude::*;

/// Builder closure: receives a fresh [`Workspace`] and returns the [`Chart`]
/// to render. Wrapped in `Arc<dyn Fn>` so it can be cloned across the mount
/// effect boundary.
pub type BuildChart = Arc<dyn Fn(Arc<Workspace>) -> Chart + Send + Sync + 'static>;

/// A `<canvas>` of the given logical size that draws the chart produced by
/// `builder` as soon as the element is mounted.
///
/// On non-wasm targets the canvas is rendered as an empty element — the
/// component compiles, but no renderer is initialized. This keeps the crate
/// usable in `cargo check --workspace` on native hosts.
#[component]
pub fn ChartCanvas(width: u32, height: u32, builder: BuildChart) -> impl IntoView {
    let canvas_ref: NodeRef<leptos::html::Canvas> = NodeRef::new();
    let device_pixel_ratio = browser_device_pixel_ratio();
    let physical_width = physical_px(width, device_pixel_ratio);
    let physical_height = physical_px(height, device_pixel_ratio);

    mount_renderer(
        canvas_ref,
        width,
        height,
        physical_width,
        physical_height,
        builder,
    );

    view! {
        <canvas
            node_ref=canvas_ref
            width=physical_width
            height=physical_height
            style=move || format!("width:{width}px;height:{height}px")
        />
    }
}

/// Wire a wgpu renderer to a canvas referenced by `canvas_ref`. Call this
/// from inside a richer component that lays out its own `<canvas>` plus
/// overlays.
///
/// The renderer is initialized once on mount and intentionally leaked so it
/// outlives the effect — a follow-up will replace this with a reactive
/// renderer cell driven by signal changes.
pub fn mount_renderer(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    logical_width: u32,
    logical_height: u32,
    physical_width: u32,
    physical_height: u32,
    builder: BuildChart,
) {
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            let builder = builder.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match Renderer::new_for_canvas_with_logical(
                    canvas,
                    physical_width,
                    physical_height,
                    logical_width as f32,
                    logical_height as f32,
                )
                .await
                {
                    Ok(mut renderer) => {
                        let workspace = Workspace::new();
                        let chart = builder(workspace);
                        if let Err(e) = renderer.render(&chart) {
                            log::error!("render failed: {e}");
                        }
                        std::mem::forget(renderer);
                    }
                    Err(e) => log::error!("renderer init failed: {e}"),
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (
                &canvas_ref,
                logical_width,
                logical_height,
                physical_width,
                physical_height,
                &builder,
            );
        }
    });
}

/// Browser device pixel ratio, clamped to a sensible range. Falls back to
/// 1.0 if `window` is unavailable. Returns 1.0 on non-wasm targets.
pub fn browser_device_pixel_ratio() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        let device_pixel_ratio = web_sys::window()
            .map(|window| window.device_pixel_ratio() as f32)
            .unwrap_or(1.0);
        device_pixel_ratio.max(2.0).clamp(1.0, 3.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        1.0
    }
}

/// Convert a logical pixel size to physical pixels using `device_pixel_ratio`.
pub fn physical_px(logical: u32, device_pixel_ratio: f32) -> u32 {
    ((logical as f32) * device_pixel_ratio).round().max(1.0) as u32
}
