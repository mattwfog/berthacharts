//! Gallery canvas wrapper.
//!
//! Composes [`berthacharts_leptos::mount_renderer`] with the gallery's
//! overlay UI: guides, tooltip, and the annotation editor. Keeps the
//! reusable renderer-mount logic in the binding crate while the rich
//! interaction layer stays application-side.

use std::sync::Arc;

use berthacharts_core::{ChartSize, ChartSpec, Workspace};
use berthacharts_leptos::{browser_device_pixel_ratio, physical_px};
#[cfg(target_arch = "wasm32")]
use berthacharts_renderer_wgpu::Renderer;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};

use crate::annotation_layer::{
    chart_snap_targets, AnnotationLayer, AnnotationState, AnnotationToolbar,
};
use crate::dom_events::{
    event_current_target_size, event_offset_in_current_target, event_target_has_class,
};
use crate::guide_overlay::{render_guides_html, render_tooltip_html};

// Re-export so the existing `use crate::chart_canvas::BuildChart` paths in
// the gallery example modules keep working.
pub use berthacharts_leptos::BuildChart;

pub fn chart_builder<S>(spec: Arc<S>, width: u32, height: u32, label: &'static str) -> BuildChart
where
    S: ChartSpec + Send + Sync + 'static,
    S::Error: std::fmt::Debug,
{
    Arc::new(move |ws| {
        spec.build_chart(ws, ChartSize::new(width, height))
            .unwrap_or_else(|error| panic!("{label} should be valid: {error:?}"))
    })
}

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
    let viewport_ref: NodeRef<leptos::html::Div> = NodeRef::new();
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

    mount_renderer_when_visible(
        canvas_ref,
        width,
        height,
        physical_width,
        physical_height,
        builder,
    );
    maintain_chart_scale(viewport_ref, width, height);

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
                node_ref=viewport_ref
                class="canvas-viewport"
                style=move || format!(
                    "width:100%;max-width:{width}px;aspect-ratio:{width}/{height};--chart-scale:1"
                )
            >
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
                        let (css_x, css_y) = event_offset_in_current_target(&ev);
                        let (css_width, css_height) = event_current_target_size(&ev);
                        let scale_x = if css_width > 0.0 {
                            width as f32 / css_width
                        } else {
                            1.0
                        };
                        let scale_y = if css_height > 0.0 {
                            height as f32 / css_height
                        } else {
                            1.0
                        };
                        let logical_x = css_x * scale_x;
                        let logical_y = css_y * scale_y;
                        if let Some(hit) = pick_chart.pick((logical_x, logical_y)) {
                            if let Some(html) = render_tooltip_html(&pick_chart, &hit) {
                                let left = ((logical_x as i32) + 14).min(width as i32 - 176).max(8);
                                let top = ((logical_y as i32) + 14).min(height as i32 - 128).max(8);
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
            </div>
            <div class="guide-flow guide-flow-bottom" inner_html=guide_flow_bottom></div>
        </div>
    }
}

fn maintain_chart_scale(
    viewport_ref: NodeRef<leptos::html::Div>,
    logical_width: u32,
    logical_height: u32,
) {
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(viewport) = viewport_ref.get() else {
                return;
            };

            let set_scale = {
                let viewport = viewport.clone();
                move || {
                    let client_width = viewport.client_width().max(1) as f32;
                    let scale = (client_width / logical_width as f32).min(1.0);
                    let style = format!(
                        "width:100%;max-width:{}px;aspect-ratio:{}/{};--chart-scale:{}",
                        logical_width, logical_width, logical_height, scale
                    );
                    if let Err(error) = viewport.set_attribute("style", &style) {
                        log::warn!("failed to update chart scale: {error:?}");
                    }
                }
            };
            set_scale();

            let set_scale_for_observer = set_scale.clone();
            let callback =
                Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(move |_, _| {
                    set_scale_for_observer();
                });
            let Ok(observer) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref())
            else {
                callback.forget();
                return;
            };
            observer.observe(viewport.as_ref());
            std::mem::forget(observer);
            callback.forget();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&viewport_ref, logical_width, logical_height);
        }
    });
}

fn mount_renderer_when_visible(
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
            use std::cell::{Cell, RefCell};
            use std::rc::Rc;

            let Some(canvas) = canvas_ref.get() else {
                return;
            };

            let renderer = Rc::new(RefCell::new(None::<Renderer>));
            let pending = Rc::new(Cell::new(false));
            let visible = Rc::new(Cell::new(false));
            let canvas_for_callback = canvas.clone();
            let builder_for_callback = builder.clone();
            let renderer_for_callback = renderer.clone();
            let pending_for_callback = pending.clone();
            let visible_for_callback = visible.clone();

            let callback = Closure::<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>::new(
                move |entries: js_sys::Array, _observer: web_sys::IntersectionObserver| {
                    let is_visible = entries.iter().any(|entry: wasm_bindgen::JsValue| {
                        let entry = entry.unchecked_into::<web_sys::IntersectionObserverEntry>();
                        entry.is_intersecting()
                    });

                    visible_for_callback.set(is_visible);

                    if !is_visible {
                        *renderer_for_callback.borrow_mut() = None;
                        return;
                    }

                    if renderer_for_callback.borrow().is_some() || pending_for_callback.get() {
                        return;
                    }

                    pending_for_callback.set(true);
                    let canvas = canvas_for_callback.clone();
                    let builder = builder_for_callback.clone();
                    let renderer_state = renderer_for_callback.clone();
                    let pending = pending_for_callback.clone();
                    let visible = visible_for_callback.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        let result = Renderer::new_for_canvas_with_logical(
                            canvas,
                            physical_width,
                            physical_height,
                            logical_width as f32,
                            logical_height as f32,
                        )
                        .await;

                        pending.set(false);

                        match result {
                            Ok(mut active_renderer) => {
                                let workspace = Workspace::new();
                                let chart = builder(workspace);
                                if let Err(e) = active_renderer.render(&chart) {
                                    log::error!("render failed: {e}");
                                }
                                if visible.get() {
                                    *renderer_state.borrow_mut() = Some(active_renderer);
                                }
                            }
                            Err(e) => log::error!("renderer init failed: {e}"),
                        }
                    });
                },
            );

            let options = web_sys::IntersectionObserverInit::new();
            options.set_root_margin("600px 0px");
            let Ok(observer) = web_sys::IntersectionObserver::new_with_options(
                callback.as_ref().unchecked_ref(),
                &options,
            ) else {
                log::warn!("IntersectionObserver unavailable; rendering chart immediately");
                pending.set(true);
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
                callback.forget();
                return;
            };

            observer.observe(canvas.as_ref());
            std::mem::forget(observer);
            callback.forget();
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
