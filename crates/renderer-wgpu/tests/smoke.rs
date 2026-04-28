//! End-to-end smoke test.
//!
//! Builds the minimum possible chart (one `RectMark` with constant channels,
//! one layer, identity `CartesianCoord`) and renders it offscreen. Verifies
//! that the center pixel falls inside the rect (red) and that the corner
//! pixel shows the clear color (blue).
//!
//! Exercises every seam established by the v0.1 kernel:
//!
//! - `Workspace` ownership of scales + coords
//! - `Scene` / `Layer` composition
//! - `Mark::tessellate` → `Geometry::Rects`
//! - wgpu pipeline creation, instance upload, draw-indexed
//! - texture → buffer readback with row-alignment stripping

use std::sync::Arc;

use berthacharts_core::{
    BandScale, CartesianCoord, Chart, CoordId, Dataset, DatasetId, Geometry, Layer, LinePrim,
    LinearScale, Mark, MarkId, NumberChannel, PickCtx, PickHit, PointPrim, Rect, RectMark, Scale,
    ScaleId, Scene, TessellateCtx, Viewport, Workspace,
};
use berthacharts_renderer_wgpu::{ClearColor, RenderError, Renderer};

const W: u32 = 64;
const H: u32 = 64;

fn pixel(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * W + x) * 4) as usize;
    [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
}

fn try_new_renderer() -> Option<Renderer> {
    match Renderer::new_offscreen(W, H) {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter | RenderError::DeviceRequest(_)) => {
            eprintln!("skipping smoke test: no GPU adapter available");
            None
        }
        Err(e) => panic!("unexpected renderer init failure: {e}"),
    }
}

#[derive(Debug)]
struct PrimitiveMark;

impl Mark for PrimitiveMark {
    fn id(&self) -> MarkId {
        MarkId::new(77)
    }

    fn fingerprint(&self) -> u64 {
        77
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        Geometry::Mixed(vec![
            Geometry::Lines(vec![LinePrim {
                points: vec![[8.0, 20.0], [56.0, 20.0]],
                stroke: [0.0, 0.82, 0.18, 1.0],
                width: 5.0,
                dash: None,
                join: 1,
                cap: 1,
            }]),
            Geometry::Points(vec![PointPrim {
                x: 32.0,
                y: 42.0,
                r: 7.0,
                shape: 0,
                fill: [1.0, 0.0, 0.0, 1.0],
                stroke: [0.0, 0.0, 0.0, 0.0],
                stroke_width: 0.0,
            }]),
        ])
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        Rect::new(0.0, 0.0, W as f32, H as f32)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn renders_a_red_rect_on_blue_background() {
    let Some(mut renderer) = try_new_renderer() else {
        return;
    };
    renderer.clear_color = ClearColor([0.0, 0.0, 1.0, 1.0]);

    // Workspace with Cartesian coord that's identity over a 64×64 plot area.
    let ws = Workspace::new();
    let x_scale: Arc<dyn Scale> = Arc::new(LinearScale::new((0.0, W as f64), (0.0, W as f32)));
    let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new((0.0, H as f64), (0.0, H as f32)));
    ws.upsert_scale(ScaleId::new(1), x_scale);
    ws.upsert_scale(ScaleId::new(2), y_scale);
    ws.upsert_coord(
        CoordId::new(0),
        Arc::new(CartesianCoord::new(ScaleId::new(1), ScaleId::new(2))),
    );

    // One rect 32×32 centered on the 64×64 target.
    let rect = RectMark::new(
        MarkId::new(1),
        DatasetId::new(0), // absent — all channels are constants, mark emits 1 rect
        NumberChannel::Constant(16.0),
        NumberChannel::Constant(16.0),
        NumberChannel::Constant(48.0),
        NumberChannel::Constant(48.0),
        [1.0, 0.0, 0.0, 1.0],
    );

    let layer = Layer {
        id: berthacharts_core::LayerId::new(0),
        coord: CoordId::new(0),
        marks: vec![Arc::new(rect)],
        blend: berthacharts_core::BlendMode::Normal,
        opacity: 1.0,
        z: 0,
        clip: None,
    };

    let mut scene = Scene::new(Viewport::full(W, H, 1.0));
    scene.layers.push(layer);

    let mut chart = Chart::new(ws.clone(), scene.viewport);
    chart.set_scene(scene);

    renderer.render(&chart).expect("render should succeed");
    let pixels = renderer.read_pixels().expect("readback should succeed");
    assert_eq!(pixels.len(), (W * H * 4) as usize, "pixel buffer size");

    // Center pixel (32, 32) is inside the red rect (16..48, 16..48).
    let center = pixel(&pixels, 32, 32);
    assert!(
        center[0] > 200 && center[1] < 40 && center[2] < 40,
        "center pixel should be red, got {center:?}",
    );

    // Corner pixel (5, 5) is outside the rect — clear-color blue.
    let corner = pixel(&pixels, 5, 5);
    assert!(
        corner[0] < 40 && corner[1] < 40 && corner[2] > 200,
        "corner pixel should be blue, got {corner:?}",
    );
}

#[test]
fn band_scale_positions_three_bars() {
    let Some(mut renderer) = try_new_renderer() else {
        return;
    };
    renderer.clear_color = ClearColor([1.0, 1.0, 1.0, 1.0]);

    let ws = Workspace::new();

    // Band scale over 3 categories across the 64-pixel width.
    let band = BandScale::new(["A", "B", "C"], (0.0, W as f32))
        .with_padding_inner(0.0)
        .with_padding_outer(0.0);
    // Sanity: each band is ~21.33 px wide.
    assert!((band.bandwidth() - (W as f32) / 3.0).abs() < 0.5);
    let bandwidth = band.bandwidth();

    let band_arc: Arc<dyn Scale> = Arc::new(band);
    ws.upsert_scale(ScaleId::new(1), band_arc);
    let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new((0.0, 1.0), (0.0, H as f32)));
    ws.upsert_scale(ScaleId::new(2), y_scale);
    ws.upsert_coord(
        CoordId::new(0),
        Arc::new(CartesianCoord::new(ScaleId::new(1), ScaleId::new(2))),
    );

    // Dataset: one row per category, y values constant.
    let data = Dataset::new(
        DatasetId::new(0),
        1,
        vec![(
            "cat".into(),
            berthacharts_core::Column::U32(berthacharts_core::ColumnData::new(vec![0, 1, 2])),
        )],
    );
    ws.upsert_dataset(data);

    // Use band scale to position x (left edge of band) and an offset channel
    // for x2 (right edge = left edge + bandwidth).
    let rect = RectMark::new(
        MarkId::new(1),
        DatasetId::new(0),
        NumberChannel::Column {
            dataset: DatasetId::new(0),
            name: "cat".into(),
            scale: ScaleId::new(1),
        },
        NumberChannel::Constant(20.0),
        NumberChannel::Column {
            dataset: DatasetId::new(0),
            name: "cat".into(),
            scale: ScaleId::new(1),
        }
        .offset(bandwidth),
        NumberChannel::Constant(60.0),
        [0.2, 0.6, 0.9, 1.0],
    );

    let layer = Layer {
        id: berthacharts_core::LayerId::new(0),
        coord: CoordId::new(0),
        marks: vec![Arc::new(rect)],
        blend: berthacharts_core::BlendMode::Normal,
        opacity: 1.0,
        z: 0,
        clip: None,
    };

    let mut scene = Scene::new(Viewport::full(W, H, 1.0));
    scene.layers.push(layer);
    let mut chart = Chart::new(ws, scene.viewport);
    chart.set_scene(scene);

    renderer.render(&chart).expect("render should succeed");
    let pixels = renderer.read_pixels().expect("readback should succeed");

    // The bars cover roughly x=[0..21.3], x=[21.3..42.6], and x=[42.6..64]
    // at y=[20..60]. Expected bar color after sRGB encoding is ~(124, 203, 243).
    //
    // Assertion distinguishes bar from white clear (255, 255, 255, 255):
    // bar has R noticeably below 255, blue channel near 243.
    let in_first = pixel(&pixels, 10, 40);
    assert!(
        in_first[0] < 180 && in_first[2] > 220,
        "expected bar fill inside first bar, got {in_first:?}",
    );

    let in_second = pixel(&pixels, 32, 40);
    assert!(
        in_second[0] < 180 && in_second[2] > 220,
        "expected bar fill inside second bar, got {in_second:?}",
    );

    let in_third = pixel(&pixels, 54, 40);
    assert!(
        in_third[0] < 180 && in_third[2] > 220,
        "expected bar fill inside third bar, got {in_third:?}",
    );

    // Outside the bars (y above top edge) the clear color shows through.
    let above_bars = pixel(&pixels, 10, 2);
    assert!(
        above_bars[0] > 240 && above_bars[1] > 240 && above_bars[2] > 240,
        "expected white clear color above bars, got {above_bars:?}",
    );
}

#[test]
fn renders_points_and_lines() {
    let Some(mut renderer) = try_new_renderer() else {
        return;
    };
    renderer.clear_color = ClearColor([1.0, 1.0, 1.0, 1.0]);

    let ws = Workspace::new();
    ws.upsert_coord(
        CoordId::new(0),
        Arc::new(CartesianCoord::new(ScaleId::new(1), ScaleId::new(2))),
    );

    let layer = Layer {
        id: berthacharts_core::LayerId::new(0),
        coord: CoordId::new(0),
        marks: vec![Arc::new(PrimitiveMark)],
        blend: berthacharts_core::BlendMode::Normal,
        opacity: 1.0,
        z: 0,
        clip: None,
    };

    let mut scene = Scene::new(Viewport::full(W, H, 1.0));
    scene.layers.push(layer);
    let mut chart = Chart::new(ws, scene.viewport);
    chart.set_scene(scene);

    renderer.render(&chart).expect("render should succeed");
    let pixels = renderer.read_pixels().expect("readback should succeed");

    let line_pixel = pixel(&pixels, 32, 20);
    assert!(
        line_pixel[1] > 190 && line_pixel[0] < 60 && line_pixel[2] < 140,
        "expected green line pixel, got {line_pixel:?}",
    );

    let point_pixel = pixel(&pixels, 32, 42);
    assert!(
        point_pixel[0] > 200 && point_pixel[1] < 50 && point_pixel[2] < 50,
        "expected red point pixel, got {point_pixel:?}",
    );

    let clear_pixel = pixel(&pixels, 4, 4);
    assert!(
        clear_pixel[0] > 240 && clear_pixel[1] > 240 && clear_pixel[2] > 240,
        "expected white clear color, got {clear_pixel:?}",
    );
}
