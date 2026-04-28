//! Reusable canvas component that bridges Leptos → wgpu renderer.
//!
//! Takes a builder closure `(Workspace) -> Chart` and renders it once on mount.
//! Browser-visible overlays are delegated to small modules so this component
//! stays focused on renderer lifecycle and top-level interaction routing.

use std::sync::Arc;

use berthacharts_core::{Chart, Workspace};
#[cfg(target_arch = "wasm32")]
use berthacharts_renderer_wgpu::Renderer;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::annotation_layer::{
    chart_snap_targets, AnnotationLayer, AnnotationState, AnnotationToolbar,
};
use crate::dom_events::{event_offset_in_current_target, event_target_has_class};
use crate::guide_overlay::{render_guides_html, render_tooltip_html};

/// Builder type: takes the workspace and returns a fully-populated [`Chart`].
pub type BuildChart = Arc<dyn Fn(Arc<Workspace>) -> Chart + Send + Sync + 'static>;

/// A `<canvas>` of given pixel size that renders the chart produced by `builder`
/// as soon as the element is mounted.
#[component]
pub fn ChartCanvas(
    width: u32,
    height: u32,
    /// Closure that produces the chart to render. Receives a fresh workspace.
    builder: BuildChart,
) -> impl IntoView {
    let canvas_ref: NodeRef<leptos::html::Canvas> = NodeRef::new();
    let device_pixel_ratio = browser_device_pixel_ratio();
    let physical_width = physical_px(width, device_pixel_ratio);
    let physical_height = physical_px(height, device_pixel_ratio);
    let overlay_chart = Arc::new(builder(Workspace::new()));
    let rendered_guides = render_guides_html(&overlay_chart, width, height);
    let guide_overlay = rendered_guides.overlay;
    let guide_flow_top = rendered_guides.flow_top;
    let guide_flow_bottom = rendered_guides.flow_bottom;
    let tooltip_html = RwSignal::new(String::new());
    let tooltip_style = RwSignal::new(String::from("display:none"));
    let pick_chart = overlay_chart.clone();
    let snap_targets = Arc::new(chart_snap_targets(&overlay_chart));
    let has_snap_targets = !snap_targets.is_empty();
    let annotations = AnnotationState::new(has_snap_targets);
    let draw_enabled = annotations.draw_enabled();
    let snap_targets_layer = snap_targets.clone();

    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            // `.get()` on a typed NodeRef already yields `web_sys::HtmlCanvasElement`
            // in Leptos 0.7 when the element type is `html::Canvas`.
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            let builder = builder.clone();
            spawn_local(async move {
                match Renderer::new_for_canvas_with_logical(
                    canvas,
                    physical_width,
                    physical_height,
                    width as f32,
                    height as f32,
                )
                .await
                {
                    Ok(mut renderer) => {
                        let workspace = Workspace::new();
                        let chart = builder(workspace);
                        if let Err(e) = renderer.render(&chart) {
                            log::error!("render failed: {e}");
                        }
                        // Keep the renderer alive for the lifetime of the page.
                        // A follow-up component will store it in a reactive cell
                        // so signal changes can drive redraws.
                        std::mem::forget(renderer);
                    }
                    Err(e) => log::error!("renderer init failed: {e}"),
                }
            });
        }
    });

    Effect::new(move |_| {
        if draw_enabled.get() {
            tooltip_style.set(String::from("display:none"));
        }
    });

    view! {
        <div class="canvas-wrap">
            <div class="guide-flow guide-flow-top" inner_html=guide_flow_top></div>
            <AnnotationToolbar state=annotations has_snap_targets=has_snap_targets />
            <div
                class="canvas-stack"
                style=move || format!("width:{width}px;height:{height}px")
                on:mousemove=move |ev| {
                    if draw_enabled.get_untracked() {
                        tooltip_style.set(String::from("display:none"));
                        return;
                    }
                    if event_target_has_class(&ev, "guide-label-has-tooltip") {
                        tooltip_style.set(String::from("display:none"));
                        return;
                    }
                    let (x, y) = event_offset_in_current_target(&ev);
                    if let Some(hit) = pick_chart.pick((x, y)) {
                        if let Some(html) = render_tooltip_html(&pick_chart, &hit) {
                            let left = ((x as i32) + 14).min(width as i32 - 176).max(8);
                            let top = ((y as i32) + 14).min(height as i32 - 128).max(8);
                            tooltip_html.set(html);
                            tooltip_style.set(format!(
                                "display:block;left:{}px;top:{}px",
                                left,
                                top,
                            ));
                            return;
                        }
                    }
                    tooltip_style.set(String::from("display:none"));
                }
                on:mouseleave=move |_| {
                    tooltip_style.set(String::from("display:none"));
                }
            >
                <canvas
                    node_ref=canvas_ref
                    width=physical_width
                    height=physical_height
                    style=move || format!("width:{width}px;height:{height}px")
                />
                <div class="guide-layer" inner_html=guide_overlay></div>
                <AnnotationLayer
                    width=width
                    height=height
                    state=annotations
                    snap_targets=snap_targets_layer
                />
                <div class="chart-tooltip" style=move || tooltip_style.get() inner_html=move || tooltip_html.get()></div>
            </div>
            <div class="guide-flow guide-flow-bottom" inner_html=guide_flow_bottom></div>
        </div>
    }
}

fn browser_device_pixel_ratio() -> f32 {
    let device_pixel_ratio = web_sys::window()
        .map(|window| window.device_pixel_ratio() as f32)
        .unwrap_or(1.0);

    device_pixel_ratio.max(2.0).clamp(1.0, 3.0)
}

fn physical_px(logical: u32, device_pixel_ratio: f32) -> u32 {
    ((logical as f32) * device_pixel_ratio).round().max(1.0) as u32
}
