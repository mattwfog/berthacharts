//! SVG annotation controls layered above rendered charts.
//!
//! This module deliberately owns drawing state, snap-target collection, and
//! annotation SVG rendering. `ChartCanvas` only decides whether chart-level
//! hover behavior should run while drawing is active.

use std::sync::Arc;

use berthacharts_core::{Chart, Geometry, SnapKind, SnapTarget, TessellateCtx};
use leptos::prelude::*;

use crate::dom_events::{event_offset_in_current_target, event_target_value_as_f32};

type SketchPoint = (f32, f32);

const SKETCH_COLORS: [SketchColor; 5] = [
    SketchColor::new("Ink", "#16202f"),
    SketchColor::new("Blue", "#215cae"),
    SketchColor::new("Teal", "#139b80"),
    SketchColor::new("Red", "#c84a3f"),
    SketchColor::new("Gold", "#a66b00"),
];

#[derive(Debug, Clone, Copy)]
struct SketchColor {
    name: &'static str,
    value: &'static str,
}

impl SketchColor {
    const fn new(name: &'static str, value: &'static str) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, Clone)]
struct SketchPath {
    points: Vec<SketchPoint>,
    color: String,
    width: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotationState {
    draw_enabled: RwSignal<bool>,
    snap_enabled: RwSignal<bool>,
    is_sketching: RwSignal<bool>,
    paths: RwSignal<Vec<SketchPath>>,
    active_path: RwSignal<Vec<SketchPoint>>,
    color: RwSignal<String>,
    width: RwSignal<f32>,
}

impl AnnotationState {
    pub fn new(has_snap_targets: bool) -> Self {
        Self {
            draw_enabled: RwSignal::new(false),
            snap_enabled: RwSignal::new(has_snap_targets),
            is_sketching: RwSignal::new(false),
            paths: RwSignal::new(Vec::new()),
            active_path: RwSignal::new(Vec::new()),
            color: RwSignal::new(String::from(SKETCH_COLORS[0].value)),
            width: RwSignal::new(2.5),
        }
    }

    pub fn draw_enabled(self) -> RwSignal<bool> {
        self.draw_enabled
    }

    fn toggle_draw(self) {
        self.draw_enabled.update(|enabled| *enabled = !*enabled);
        self.clear_active();
    }

    fn clear_all(self) {
        self.paths.set(Vec::new());
        self.clear_active();
    }

    fn clear_active(self) {
        self.active_path.set(Vec::new());
        self.is_sketching.set(false);
    }

    fn start_path(self, point: SketchPoint) {
        self.is_sketching.set(true);
        self.active_path.set(vec![point]);
    }

    fn append_point(self, point: SketchPoint) {
        self.active_path.update(|points| {
            if should_add_sketch_point(points.last().copied(), point) {
                points.push(point);
            }
        });
    }

    fn finish_path(self) {
        let active = self.active_path.get_untracked();
        if active.len() > 1 {
            let color = self.color.get_untracked();
            let width = normalized_sketch_width(self.width.get_untracked());
            self.paths.update(|paths| {
                paths.push(SketchPath {
                    points: active,
                    color,
                    width,
                })
            });
        }
        self.clear_active();
    }
}

#[component]
pub fn AnnotationToolbar(state: AnnotationState, has_snap_targets: bool) -> impl IntoView {
    view! {
        <div class="sketch-toolbar" aria-label="Drawing tools">
            <div
                class=move || sketch_popover_class(state.draw_enabled.get())
                aria-hidden=move || (!state.draw_enabled.get()).to_string()
            >
                <div class="sketch-tool-group" aria-label="Stroke color">
                    {SKETCH_COLORS.into_iter().map(|color| {
                        view! {
                            <button
                                type="button"
                                class=move || sketch_swatch_class(state.color.get() == color.value)
                                style=sketch_color_style(color.value)
                                aria-label=format!("{} stroke", color.name)
                                on:click=move |_| {
                                    state.color.set(String::from(color.value));
                                }
                            ></button>
                        }
                    }).collect_view()}
                </div>
                <div class="sketch-tool-group sketch-width-group">
                    <button
                        type="button"
                        class="sketch-stepper"
                        aria-label="Decrease stroke width"
                        on:click=move |_| {
                            state.width.update(|width| {
                                *width = normalized_sketch_width(*width - 0.5);
                            });
                        }
                    >
                        "-"
                    </button>
                    <input
                        class="sketch-width"
                        type="range"
                        min="1"
                        max="8"
                        step="0.5"
                        prop:value=move || format!("{:.1}", state.width.get())
                        aria-label="Stroke width"
                        on:input=move |ev| {
                            state.width.set(event_target_value_as_f32(&ev, state.width.get_untracked()));
                        }
                        on:change=move |ev| {
                            state.width.set(event_target_value_as_f32(&ev, state.width.get_untracked()));
                        }
                    />
                    <span class="sketch-width-readout">{move || format!("{:.1}", state.width.get())}</span>
                    <button
                        type="button"
                        class="sketch-stepper"
                        aria-label="Increase stroke width"
                        on:click=move |_| {
                            state.width.update(|width| {
                                *width = normalized_sketch_width(*width + 0.5);
                            });
                        }
                    >
                        "+"
                    </button>
                </div>
                <button
                    type="button"
                    class=move || sketch_button_class(state.snap_enabled.get() && has_snap_targets)
                    aria-pressed=move || (state.snap_enabled.get() && has_snap_targets).to_string()
                    disabled=move || !has_snap_targets
                    on:click=move |_| {
                        if has_snap_targets {
                            state.snap_enabled.update(|enabled| *enabled = !*enabled);
                        }
                    }
                >
                    "Snap"
                </button>
                <button
                    type="button"
                    class="sketch-button"
                    disabled=move || state.paths.get().is_empty() && state.active_path.get().is_empty()
                    on:click=move |_| state.clear_all()
                >
                    "Clear"
                </button>
            </div>
            <button
                type="button"
                class=move || sketch_button_class(state.draw_enabled.get())
                aria-pressed=move || state.draw_enabled.get().to_string()
                aria-expanded=move || state.draw_enabled.get().to_string()
                on:click=move |_| state.toggle_draw()
            >
                "Draw"
            </button>
        </div>
    }
}

#[component]
pub fn AnnotationLayer(
    width: u32,
    height: u32,
    state: AnnotationState,
    snap_targets: Arc<Vec<SnapTarget>>,
) -> impl IntoView {
    let snap_targets_down = snap_targets.clone();
    let snap_targets_move = snap_targets.clone();
    let snap_targets_overlay = snap_targets.clone();

    view! {
        <svg
            class=move || sketch_layer_class(state.draw_enabled.get())
            viewBox=move || format!("0 0 {width} {height}")
            on:mousedown=move |ev| {
                if !state.draw_enabled.get_untracked() {
                    return;
                }
                ev.prevent_default();
                state.start_path(sketch_event_point(
                    &ev,
                    width,
                    height,
                    state.snap_enabled.get_untracked(),
                    snap_targets_down.as_slice(),
                ));
            }
            on:mousemove=move |ev| {
                if !state.draw_enabled.get_untracked() || !state.is_sketching.get_untracked() {
                    return;
                }
                ev.prevent_default();
                let point = sketch_event_point(
                    &ev,
                    width,
                    height,
                    state.snap_enabled.get_untracked(),
                    snap_targets_move.as_slice(),
                );
                state.append_point(point);
            }
            on:mouseup=move |ev| {
                if !state.draw_enabled.get_untracked() || !state.is_sketching.get_untracked() {
                    return;
                }
                ev.prevent_default();
                state.finish_path();
            }
            on:mouseleave=move |ev| {
                if !state.draw_enabled.get_untracked() || !state.is_sketching.get_untracked() {
                    return;
                }
                ev.prevent_default();
                state.finish_path();
            }
        >
            {move || render_snap_targets(
                state.draw_enabled.get() && state.snap_enabled.get(),
                snap_targets_overlay.as_slice(),
            )}
            {move || render_sketch_paths(
                state.paths.get(),
                state.active_path.get(),
                state.color.get(),
                state.width.get(),
            )}
        </svg>
    }
}

pub fn chart_snap_targets(chart: &Chart) -> Vec<SnapTarget> {
    let authored_targets = chart.snap_targets();
    if !authored_targets.is_empty() {
        return authored_targets;
    }

    circle_geometry_snap_targets(chart)
}

fn circle_geometry_snap_targets(chart: &Chart) -> Vec<SnapTarget> {
    let scene = chart.scene();
    let scales = chart.workspace().scales();
    let datasets = chart.workspace().datasets();
    let mut targets = Vec::new();

    for layer in &scene.layers {
        let Some(coord) = chart.workspace().coord(layer.coord) else {
            continue;
        };
        let ctx = TessellateCtx::new(
            coord.as_ref(),
            &scales,
            &datasets,
            scene.viewport.plot_area,
            scene.viewport.device_pixel_ratio,
        );
        for mark in &layer.marks {
            collect_circle_snap_targets(&mark.tessellate(&ctx), &mut targets);
        }
    }

    targets
}

fn collect_circle_snap_targets(geometry: &Geometry, targets: &mut Vec<SnapTarget>) {
    match geometry {
        Geometry::Points(points) => {
            targets.extend(points.iter().filter(|point| point.shape == 0).map(|point| {
                SnapTarget::new(point.x, point.y, SnapKind::Point).with_radius(point.r)
            }));
        }
        Geometry::Mixed(parts) => {
            for part in parts {
                collect_circle_snap_targets(part, targets);
            }
        }
        _ => {}
    }
}

fn sketch_swatch_class(active: bool) -> &'static str {
    if active {
        "sketch-color-swatch is-active"
    } else {
        "sketch-color-swatch"
    }
}

fn sketch_color_style(color: &str) -> String {
    format!("--sketch-color:{color}")
}

fn sketch_popover_class(open: bool) -> &'static str {
    if open {
        "sketch-popover is-open"
    } else {
        "sketch-popover"
    }
}

fn sketch_button_class(draw_enabled: bool) -> &'static str {
    if draw_enabled {
        "sketch-button is-active"
    } else {
        "sketch-button"
    }
}

fn sketch_layer_class(draw_enabled: bool) -> &'static str {
    if draw_enabled {
        "sketch-layer is-drawing"
    } else {
        "sketch-layer"
    }
}

fn sketch_event_point(
    ev: &web_sys::MouseEvent,
    width: u32,
    height: u32,
    snap_enabled: bool,
    snap_targets: &[SnapTarget],
) -> SketchPoint {
    let point = clamped_event_point(ev, width, height);
    snap_to_circle(point, snap_enabled, snap_targets)
}

fn clamped_event_point(ev: &web_sys::MouseEvent, width: u32, height: u32) -> SketchPoint {
    let (x, y) = event_offset_in_current_target(ev);
    (x.clamp(0.0, width as f32), y.clamp(0.0, height as f32))
}

fn snap_to_circle(
    point: SketchPoint,
    snap_enabled: bool,
    snap_targets: &[SnapTarget],
) -> SketchPoint {
    if !snap_enabled {
        return point;
    }

    snap_targets
        .iter()
        .filter_map(|target| {
            let dx = point.0 - target.x;
            let dy = point.1 - target.y;
            let distance2 = dx * dx + dy * dy;
            let snap_radius = target.radius + 12.0;
            if distance2 <= snap_radius * snap_radius {
                Some((distance2, target))
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(point, |(_, target)| (target.x, target.y))
}

fn should_add_sketch_point(previous: Option<SketchPoint>, point: SketchPoint) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    let dx = point.0 - previous.0;
    let dy = point.1 - previous.1;
    dx * dx + dy * dy >= 4.0
}

fn render_snap_targets(show: bool, targets: &[SnapTarget]) -> impl IntoView {
    let targets = if show { targets.to_vec() } else { Vec::new() };

    targets
        .into_iter()
        .map(|target| {
            let cx = format!("{:.1}", target.x);
            let cy = format!("{:.1}", target.y);
            let radius = format!("{:.1}", target.radius + 5.0);
            view! { <circle class="sketch-snap-target" cx=cx cy=cy r=radius /> }
        })
        .collect_view()
}

fn render_sketch_paths(
    paths: Vec<SketchPath>,
    active: Vec<SketchPoint>,
    active_color: String,
    active_width: f32,
) -> impl IntoView {
    let mut all_paths = paths;
    if active.len() > 1 {
        all_paths.push(SketchPath {
            points: active,
            color: active_color,
            width: normalized_sketch_width(active_width),
        });
    }

    all_paths
        .into_iter()
        .filter(|path| path.points.len() > 1)
        .map(|path| {
            let points = sketch_points_attr(&path.points);
            let style = sketch_path_style(&path);
            view! { <polyline class="sketch-stroke" points=points style=style /> }
        })
        .collect_view()
}

fn normalized_sketch_width(width: f32) -> f32 {
    width.clamp(1.0, 8.0)
}

fn sketch_path_style(path: &SketchPath) -> String {
    format!(
        "stroke:{};stroke-width:{:.1}",
        path.color,
        normalized_sketch_width(path.width),
    )
}

fn sketch_points_attr(points: &[SketchPoint]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}
