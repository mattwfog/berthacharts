//! # `berthacharts-bindings-react`
//!
//! WASM bridge exposing Bertha Charts to JavaScript / React. Publishes as
//! `@berthacharts/react` via wasm-pack.
//!
//! JS API:
//!   const chart = await BerthaChart.create(canvas, 640, 360);
//!   chart.bar(jsonString);      // BarSpec JSON
//!   chart.line(jsonString);     // LineSpec JSON
//!   chart.scatter(jsonString);  // ScatterSpec JSON
//!   chart.heatmap(jsonString);  // HeatmapSpec JSON
//!   chart.guides();             // → JSON of axes, labels, legend
//!   chart.resize(800, 480);
//!   chart.destroy();

#![forbid(unsafe_code)]
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::sync::Arc;

use berthacharts_charts::{
    BarChartOptions, BarChartSpec, BarDatum, HeatmapCell, HeatmapOptions, HeatmapSpec,
    LineChartOptions, LineChartSpec, LineDatum, ScatterDatum, ScatterPlotOptions, ScatterPlotSpec,
};
use berthacharts_core::Workspace;
#[cfg(target_arch = "wasm32")]
use berthacharts_core::{ChartSize, ChartSpec};
use berthacharts_renderer_wgpu::Renderer;
use serde::{Deserialize, Serialize};

mod flows;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn dpr() -> f32 {
    web_sys::window()
        .map(|w| w.device_pixel_ratio() as f32)
        .unwrap_or(1.0)
        .clamp(1.0, 3.0)
}

#[cfg(target_arch = "wasm32")]
fn physical(logical: u32, dpr: f32) -> u32 {
    ((logical as f32) * dpr).round().max(1.0) as u32
}

/// Opaque chart handle exposed to JS.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct BerthaChart {
    renderer: Renderer,
    workspace: Arc<Workspace>,
    logical_w: u32,
    logical_h: u32,
    last_guides: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl BerthaChart {
    /// Initialize a chart renderer on a canvas element.
    #[wasm_bindgen]
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<BerthaChart, JsValue> {
        let dpr = dpr();
        let pw = physical(width, dpr);
        let ph = physical(height, dpr);
        canvas.set_width(pw);
        canvas.set_height(ph);

        let mut renderer =
            Renderer::new_for_canvas_with_logical(canvas, pw, ph, width as f32, height as f32)
                .await
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        // Clear to fully transparent so the canvas composites over the host
        // page's background instead of baking in a fixed fill. This is what
        // makes the chart adapt to light AND dark themes: the page background
        // shows through, and the DOM overlay (axes / value labels) is themed by
        // the host's own CSS. Requires a transparent surface alpha mode, which
        // the renderer selects when the platform offers one.
        renderer.clear_color = berthacharts_renderer_wgpu::ClearColor([0.0, 0.0, 0.0, 0.0]);

        Ok(BerthaChart {
            renderer,
            workspace: Workspace::new(),
            logical_w: width,
            logical_h: height,
            last_guides: None,
        })
    }

    fn render_chart(&mut self, chart: &berthacharts_core::Chart) -> Result<(), JsValue> {
        self.renderer
            .render(chart)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.last_guides = Some(extract_guides(chart));
        Ok(())
    }

    /// Render a bar chart from JSON.
    #[wasm_bindgen]
    pub fn bar(&mut self, json: &str) -> Result<(), JsValue> {
        let input: BarInput =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let spec = input.into_spec();
        let size = ChartSize::new(self.logical_w, self.logical_h);
        let chart = spec
            .build_chart(self.workspace.clone(), size)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.render_chart(&chart)
    }

    /// Render a line chart from JSON.
    #[wasm_bindgen]
    pub fn line(&mut self, json: &str) -> Result<(), JsValue> {
        let input: LineInput =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let spec = input.into_spec();
        let size = ChartSize::new(self.logical_w, self.logical_h);
        let chart = spec
            .build_chart(self.workspace.clone(), size)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.render_chart(&chart)
    }

    /// Render a scatter plot from JSON.
    #[wasm_bindgen]
    pub fn scatter(&mut self, json: &str) -> Result<(), JsValue> {
        let input: ScatterInput =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let spec = input.into_spec();
        let size = ChartSize::new(self.logical_w, self.logical_h);
        let chart = spec
            .build_chart(self.workspace.clone(), size)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.render_chart(&chart)
    }

    /// Render a heatmap from JSON.
    #[wasm_bindgen]
    pub fn heatmap(&mut self, json: &str) -> Result<(), JsValue> {
        let input: HeatmapInput =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let spec = input.into_spec();
        let size = ChartSize::new(self.logical_w, self.logical_h);
        let chart = spec
            .build_chart(self.workspace.clone(), size)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.render_chart(&chart)
    }

    /// Return guide overlay data (axes, labels, legend) as JSON.
    /// Call after bar/line/scatter/heatmap to get the DOM overlay data.
    #[wasm_bindgen]
    pub fn guides(&self) -> Option<String> {
        self.last_guides.clone()
    }

    /// Resize the renderer.
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        let dpr = dpr();
        self.logical_w = width;
        self.logical_h = height;
        self.renderer.resize_with_logical(
            physical(width, dpr),
            physical(height, dpr),
            width as f32,
            height as f32,
        );
    }

    /// Release renderer resources.
    #[wasm_bindgen]
    pub fn destroy(self) {
        drop(self);
    }
}

// ---------------------------------------------------------------------------
// Guide extraction — serialize scene guides into a JS-friendly JSON shape.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GuidesOutput {
    axes: Vec<AxisOutput>,
    labels: Vec<LabelOutput>,
    legend: Option<LegendOutput>,
    plot_area: PlotAreaOutput,
}

#[derive(Serialize)]
struct PlotAreaOutput {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Serialize)]
struct AxisOutput {
    orient: &'static str,
    label: Option<String>,
    ticks: Vec<TickOutput>,
}

#[derive(Serialize)]
struct TickOutput {
    position: f32,
    label: String,
}

#[derive(Serialize)]
struct LabelOutput {
    x: f32,
    y: f32,
    text: String,
    detail: Option<String>,
    anchor: &'static str,
}

#[derive(Serialize)]
struct LegendOutput {
    title: Option<String>,
    anchor: &'static str,
    items: Vec<LegendItemOutput>,
}

#[derive(Serialize)]
struct LegendItemOutput {
    label: String,
    color: String,
}

fn extract_guides(chart: &berthacharts_core::Chart) -> String {
    let scene = chart.scene();
    let workspace = chart.workspace();
    let scales = workspace.scales();
    let plot = scene.viewport.plot_area;

    let mut axes = Vec::new();
    let mut labels = Vec::new();
    let mut legend = None;

    for guide in &scene.guides {
        match guide {
            berthacharts_core::Guide::Axis(ag) => {
                let orient = match ag.orient {
                    berthacharts_core::AxisOrient::Top => "top",
                    berthacharts_core::AxisOrient::Right => "right",
                    berthacharts_core::AxisOrient::Bottom => "bottom",
                    berthacharts_core::AxisOrient::Left => "left",
                    _ => "bottom",
                };
                let ticks = if let Some(scale) = scales.get(ag.scale) {
                    scale
                        .ticks(ag.tick_count)
                        .into_iter()
                        .map(|t| TickOutput {
                            position: t.position,
                            label: t.label,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                axes.push(AxisOutput {
                    orient,
                    label: ag.label.clone(),
                    ticks,
                });
            }
            berthacharts_core::Guide::Labels(lg) => {
                for item in &lg.items {
                    let anchor = match item.anchor {
                        berthacharts_core::LabelAnchor::Center => "center",
                        berthacharts_core::LabelAnchor::Top => "top",
                        berthacharts_core::LabelAnchor::Bottom => "bottom",
                        berthacharts_core::LabelAnchor::Left => "left",
                        berthacharts_core::LabelAnchor::Right => "right",
                        _ => "center",
                    };
                    labels.push(LabelOutput {
                        x: item.x,
                        y: item.y,
                        text: item.text.clone(),
                        detail: item.detail.clone(),
                        anchor,
                    });
                }
            }
            berthacharts_core::Guide::Legend(lg) => {
                let anchor = match lg.anchor {
                    berthacharts_core::LegendAnchor::Top => "top",
                    berthacharts_core::LegendAnchor::Bottom => "bottom",
                    berthacharts_core::LegendAnchor::TopLeft => "top-left",
                    _ => "bottom",
                };
                legend = Some(LegendOutput {
                    title: lg.title.clone(),
                    anchor,
                    items: lg
                        .items
                        .iter()
                        .map(|i| LegendItemOutput {
                            label: i.label.clone(),
                            color: rgba_to_css(i.color),
                        })
                        .collect(),
                });
            }
            _ => {}
        }
    }

    let output = GuidesOutput {
        axes,
        labels,
        legend,
        plot_area: PlotAreaOutput {
            x: plot.x,
            y: plot.y,
            w: plot.w,
            h: plot.h,
        },
    };
    serde_json::to_string(&output).unwrap_or_default()
}

fn rgba_to_css(c: [f32; 4]) -> String {
    let a = c[3].max(0.001);
    let r = ((c[0] / a).clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = ((c[1] / a).clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = ((c[2] / a).clamp(0.0, 1.0) * 255.0).round() as u8;
    if (a - 1.0).abs() < 0.01 {
        format!("rgb({r},{g},{b})")
    } else {
        format!("rgba({r},{g},{b},{:.2})", a)
    }
}

// ---------------------------------------------------------------------------
// JSON input shapes — serde types at the WASM boundary.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct BarInput {
    data: Vec<BarDatumInput>,
    #[serde(default)]
    target: Option<f32>,
    #[serde(default = "default_axis_label")]
    x_label: String,
    #[serde(default = "default_axis_label")]
    y_label: String,
    #[serde(default)]
    y_max: Option<f32>,
    #[serde(default = "default_tick_count")]
    y_ticks: usize,
}

#[derive(Deserialize)]
struct BarDatumInput {
    label: String,
    value: f32,
}

impl BarInput {
    fn into_spec(self) -> BarChartSpec {
        let data = self
            .data
            .into_iter()
            .map(|d| BarDatum::new(d.label, d.value))
            .collect();
        let mut opts = BarChartOptions::default();
        opts.x_axis_label = self.x_label;
        opts.y_axis_label = self.y_label;
        opts.y_max = self.y_max;
        opts.target = self.target;
        opts.y_tick_count = self.y_ticks;
        BarChartSpec::new(data).with_options(opts)
    }
}

#[derive(Deserialize)]
struct LineInput {
    data: Vec<LineDatumInput>,
    #[serde(default = "default_axis_label")]
    x_label: String,
    #[serde(default = "default_axis_label")]
    y_label: String,
    #[serde(default = "default_tick_count")]
    x_ticks: usize,
    #[serde(default = "default_tick_count")]
    y_ticks: usize,
    #[serde(default = "default_line_width")]
    line_width: f32,
    #[serde(default = "default_true")]
    show_points: bool,
}

#[derive(Deserialize)]
struct LineDatumInput {
    series: String,
    x: f32,
    y: f32,
    #[serde(default)]
    label: Option<String>,
}

impl LineInput {
    fn into_spec(self) -> LineChartSpec {
        let data = self
            .data
            .into_iter()
            .map(|d| {
                let mut datum = LineDatum::new(d.series, d.x, d.y);
                if let Some(label) = d.label {
                    datum = datum.with_label(label);
                }
                datum
            })
            .collect();
        let mut opts = LineChartOptions::default();
        opts.x_axis_label = self.x_label;
        opts.y_axis_label = self.y_label;
        opts.x_tick_count = self.x_ticks;
        opts.y_tick_count = self.y_ticks;
        opts.line_width = self.line_width;
        opts.show_points = self.show_points;
        LineChartSpec::new(data).with_options(opts)
    }
}

#[derive(Deserialize)]
struct ScatterInput {
    data: Vec<ScatterDatumInput>,
    #[serde(default = "default_axis_label")]
    x_label: String,
    #[serde(default = "default_axis_label")]
    y_label: String,
    #[serde(default = "default_tick_count")]
    x_ticks: usize,
    #[serde(default = "default_tick_count")]
    y_ticks: usize,
}

#[derive(Deserialize)]
struct ScatterDatumInput {
    label: String,
    x: f32,
    y: f32,
    #[serde(default = "default_group")]
    group: String,
    #[serde(default)]
    radius: Option<f32>,
}

impl ScatterInput {
    fn into_spec(self) -> ScatterPlotSpec {
        let data = self
            .data
            .into_iter()
            .map(|d| {
                let mut datum = ScatterDatum::new(d.label, d.x, d.y);
                datum.group = d.group;
                datum.radius = d.radius;
                datum
            })
            .collect();
        let mut opts = ScatterPlotOptions::default();
        opts.x_axis_label = self.x_label;
        opts.y_axis_label = self.y_label;
        opts.x_tick_count = self.x_ticks;
        opts.y_tick_count = self.y_ticks;
        ScatterPlotSpec::new(data).with_options(opts)
    }
}

#[derive(Deserialize)]
struct HeatmapInput {
    cells: Vec<HeatmapCellInput>,
    #[serde(default = "default_signal_threshold")]
    signal_threshold: f32,
    #[serde(default)]
    legend_title: Option<String>,
    #[serde(default)]
    max_visible_labels: Option<usize>,
}

#[derive(Deserialize)]
struct HeatmapCellInput {
    row: String,
    column: String,
    value: f32,
    #[serde(default)]
    baseline: Option<f32>,
    #[serde(default)]
    label_detail: Option<String>,
}

impl HeatmapInput {
    fn into_spec(self) -> HeatmapSpec {
        let cells = self
            .cells
            .into_iter()
            .map(|c| {
                let mut cell = HeatmapCell::new(c.row, c.column, c.value);
                cell.baseline = c.baseline;
                cell.label_detail = c.label_detail;
                cell
            })
            .collect();
        let mut opts = HeatmapOptions::default();
        opts.signal_threshold = self.signal_threshold;
        if let Some(title) = self.legend_title {
            opts.legend_title = title;
        }
        opts.max_visible_labels = self.max_visible_labels;
        HeatmapSpec::new(cells).with_options(opts)
    }
}

fn default_axis_label() -> String {
    String::new()
}

fn default_tick_count() -> usize {
    5
}

fn default_line_width() -> f32 {
    2.4
}

fn default_true() -> bool {
    true
}

fn default_group() -> String {
    "points".to_string()
}

fn default_signal_threshold() -> f32 {
    0.05
}
