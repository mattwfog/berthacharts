//! "Hello world" — one red rectangle on a blue background.
//!
//! Mirrors the `renders_a_red_rect_on_blue_background` smoke test but in
//! the browser. Verifies: WebGL2 init → canvas surface → pipeline → draw.

use std::sync::Arc;

use berthacharts_core::{
    CartesianCoord, Chart, CoordId, DatasetId, Guide, LabelAnchor, LabelGuide, LabelItem,
    LabelKind, LabelPriority, Layer, LayerId, LinearScale, MarkId, NumberChannel, RectMark, Scale,
    ScaleId, Scene, SnapKind, SnapTarget, SnapTargetSet, Viewport,
};
use berthacharts_renderer_wgpu::ClearColor;
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 320;
const H: u32 = 220;

/// Wrapping Leptos view.
#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);

    let build: BuildChart = Arc::new(|ws| {
        let x_scale: Arc<dyn Scale> = Arc::new(LinearScale::new((0.0, W as f64), (0.0, W as f32)));
        let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new((0.0, H as f64), (0.0, H as f32)));
        ws.upsert_scale(ScaleId::new(1), x_scale);
        ws.upsert_scale(ScaleId::new(2), y_scale);
        ws.upsert_coord(
            CoordId::new(0),
            Arc::new(CartesianCoord::new(ScaleId::new(1), ScaleId::new(2))),
        );

        let reference = RectMark::new(
            MarkId::new(1),
            DatasetId::new(0),
            NumberChannel::Constant(54.0),
            NumberChannel::Constant(42.0),
            NumberChannel::Constant(268.0),
            NumberChannel::Constant(178.0),
            [0.92, 0.95, 1.00, 1.0],
        );

        let rect = RectMark::new(
            MarkId::new(2),
            DatasetId::new(0),
            NumberChannel::Constant(84.0),
            NumberChannel::Constant(62.0),
            NumberChannel::Constant(230.0),
            NumberChannel::Constant(158.0),
            [0.22, 0.47, 0.88, 1.0],
        );

        let accent = RectMark::new(
            MarkId::new(3),
            DatasetId::new(0),
            NumberChannel::Constant(176.0),
            NumberChannel::Constant(90.0),
            NumberChannel::Constant(256.0),
            NumberChannel::Constant(138.0),
            [0.10, 0.70, 0.58, 1.0],
        );

        let layer = Layer {
            id: LayerId::new(0),
            coord: CoordId::new(0),
            marks: vec![Arc::new(reference), Arc::new(rect), Arc::new(accent)],
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        };

        let mut scene = Scene::new(Viewport::full(W, H, 1.0));
        scene.layers.push(layer);
        scene.guides.push(Guide::Labels(LabelGuide::new(vec![
            compact_label(160.0, 42.0, "reference", "214 x 136", LabelAnchor::Top),
            compact_label(157.0, 110.0, "measure", "146 x 96", LabelAnchor::Center),
            compact_label(216.0, 138.0, "annotation", "80 x 48", LabelAnchor::Bottom),
        ])));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(vec![
                    SnapTarget::new(161.0, 110.0, SnapKind::Center).with_label("reference center"),
                    SnapTarget::new(157.0, 110.0, SnapKind::Center).with_label("measure center"),
                    SnapTarget::new(216.0, 138.0, SnapKind::Center).with_label("annotation center"),
                ])
                .with_name("rectangle centers"),
            ));

        let mut chart = Chart::new(ws, scene.viewport);
        chart.set_scene(scene);
        chart
    });

    // Clear color applied to the canvas. ChartCanvas doesn't expose this
    // today — we set it from the builder by feeding it through the chart's
    // render pass. For now we accept the default white background; follow-up
    // adds per-canvas ClearColor plumbing.
    let _ = ClearColor::default();

    view! {
        <section id="hello-rect" class="example">
            <div class="example-head">
                <div>
                    <h2>"Layer Composition"</h2>
                    <p>
                        "Constant channels, z-ordered marks, and a Cartesian projection rendered through WebGL2."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>"3"</strong>" marks"</span>
                    <span><strong>"GPU"</strong>" instanced"</span>
                </div>
            </div>
            <DisplayControls label="Layer composition display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
            </DisplayControls>
            <div class=move || compact_stage_class(show_data_labels.get())>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
        </section>
    }
}

fn compact_label(x: f32, y: f32, text: &str, detail: &str, anchor: LabelAnchor) -> LabelItem {
    LabelItem::new(x, y, text)
        .with_detail(detail)
        .with_kind(LabelKind::Data)
        .with_priority(LabelPriority::Required)
        .with_anchor(anchor)
}

fn compact_stage_class(show_data_labels: bool) -> String {
    let mut class = String::from("chart-stage compact-stage");
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    class
}
