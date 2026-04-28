//! Gallery canvas wrapper.
//!
//! Composes [`berthacharts_leptos::mount_renderer`] with the gallery's
//! overlay UI: guides, tooltip, and the annotation editor. Keeps the
//! reusable renderer-mount logic in the binding crate while the rich
//! interaction layer stays application-side.

use std::sync::Arc;

use berthacharts_core::Workspace;
use berthacharts_leptos::{browser_device_pixel_ratio, mount_renderer, physical_px};
use leptos::prelude::*;

use crate::annotation_layer::{
    chart_snap_targets, AnnotationLayer, AnnotationState, AnnotationToolbar,
};
use crate::dom_events::{event_offset_in_current_target, event_target_has_class};
use crate::guide_overlay::{render_guides_html, render_tooltip_html};

// Re-export so the existing `use crate::chart_canvas::BuildChart` paths in
// the gallery example modules keep working.
pub use berthacharts_leptos::BuildChart;

/// A `<canvas>` of given pixel size that renders the chart produced by `builder`
/// as soon as the element is mounted, plus the gallery's guide / tooltip /
/// annotation overlays.
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

    mount_renderer(
        canvas_ref,
        width,
        height,
        physical_width,
        physical_height,
        builder,
    );

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
