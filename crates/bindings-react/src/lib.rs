//! # `berthacharts-bindings-react`
//!
//! Thin WASM bindings published to npm as `@berthacharts/react`. Exposes a
//! handle-based API over the wgpu renderer and the first-party chart specs.
//!
//! ## JS usage
//!
//! ```js,ignore
//! import init, { BerthaCanvas } from "@berthacharts/react";
//! await init();
//!
//! const canvas = document.getElementById("c");
//! const view = await BerthaCanvas.new(canvas, 600, 400, window.devicePixelRatio);
//! view.set_bar_chart(JSON.stringify([
//!     { label: "Q1", value: 42 },
//!     { label: "Q2", value: 57 },
//! ]));
//! ```
//!
//! The crate also re-exports the core kernel + renderer for users who want
//! to build their own JS-callable wrappers.

#![forbid(unsafe_code)]
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::sync::Arc;

use berthacharts_charts::{
    BarChartSpec, BarDatum, HeatmapCell, HeatmapSpec, LineChartSpec, LineDatum, ScatterDatum,
    ScatterPlotSpec,
};
use berthacharts_core::{Chart, ChartSize, ChartSpec, PickCtx, Workspace};
use berthacharts_network::{ForceEdge, ForceNode, ForceSpec};
use berthacharts_renderer_wgpu::Renderer;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

pub use berthacharts_core as core;
pub use berthacharts_renderer_wgpu as renderer;

/// JS-shaped bar datum.
#[derive(Debug, Deserialize)]
struct BarJs {
    label: String,
    value: f32,
}

/// JS-shaped line datum.
#[derive(Debug, Deserialize)]
struct LineJs {
    #[serde(default)]
    series: Option<String>,
    x: f32,
    y: f32,
}

/// JS-shaped scatter datum.
#[derive(Debug, Deserialize)]
struct ScatterJs {
    #[serde(default)]
    label: Option<String>,
    x: f32,
    y: f32,
    #[serde(default)]
    r: Option<f32>,
}

/// JS-shaped heatmap cell.
#[derive(Debug, Deserialize)]
struct HeatmapJs {
    row: String,
    col: String,
    value: f32,
}

/// JS-shaped force-graph node.
#[derive(Debug, Deserialize)]
struct ForceNodeJs {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    radius: Option<f32>,
    #[serde(default)]
    color: Option<[f32; 4]>,
}

/// JS-shaped force-graph edge.
#[derive(Debug, Deserialize)]
struct ForceEdgeJs {
    source: String,
    target: String,
    #[serde(default)]
    weight: Option<f32>,
}

/// JS-shaped force-graph input.
#[derive(Debug, Deserialize)]
struct ForceGraphJs {
    nodes: Vec<ForceNodeJs>,
    edges: Vec<ForceEdgeJs>,
}

/// JS-shaped pick result.
#[derive(Debug, Serialize)]
struct PickJs {
    mark: u32,
    row: Option<usize>,
    distance: f32,
}

/// Stateful canvas + renderer + workspace handle. One per `<canvas>`.
///
/// Only present on `wasm32-unknown-unknown` — the underlying wgpu surface
/// constructors require a browser canvas.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct BerthaCanvas {
    canvas: HtmlCanvasElement,
    logical_width: u32,
    logical_height: u32,
    physical_width: u32,
    physical_height: u32,
    renderer: Option<Renderer>,
    workspace: Arc<Workspace>,
    chart: Option<Chart>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl BerthaCanvas {
    /// Construct a handle. Call [`init`](Self::init) before rendering — it
    /// initializes the async wgpu surface.
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas: HtmlCanvasElement,
        logical_width: u32,
        logical_height: u32,
        device_pixel_ratio: f32,
    ) -> Self {
        let physical_width =
            ((logical_width as f32) * device_pixel_ratio).round().max(1.0) as u32;
        let physical_height =
            ((logical_height as f32) * device_pixel_ratio).round().max(1.0) as u32;
        Self {
            canvas,
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            renderer: None,
            workspace: Workspace::new(),
            chart: None,
        }
    }

    /// Async wgpu init. Resolves once the surface is ready.
    #[wasm_bindgen]
    pub async fn init(&mut self) -> Result<(), JsValue> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let renderer = Renderer::new_for_canvas_with_logical(
            self.canvas.clone(),
            self.physical_width,
            self.physical_height,
            self.logical_width as f32,
            self.logical_height as f32,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("renderer init: {e}")))?;
        self.renderer = Some(renderer);
        Ok(())
    }

    /// Logical width (CSS pixels).
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.logical_width
    }

    /// Logical height (CSS pixels).
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.logical_height
    }

    /// Build a bar chart from JSON. Replaces the current scene.
    /// JSON shape: `[ { "label": "Q1", "value": 42, "color": [r,g,b,a]? }, ... ]`
    #[wasm_bindgen]
    pub fn set_bar_chart(&mut self, data_json: &str) -> Result<(), JsValue> {
        let data: Vec<BarJs> = serde_json::from_str(data_json)
            .map_err(|e| JsValue::from_str(&format!("bar parse: {e}")))?;
        let datums: Vec<BarDatum> = data
            .into_iter()
            .map(|d| BarDatum::new(d.label, d.value))
            .collect();
        let spec = BarChartSpec::new(datums);
        let chart = spec
            .build_chart(self.workspace.clone(), self.chart_size())
            .map_err(|e| JsValue::from_str(&format!("bar build: {e}")))?;
        self.set_chart(chart)
    }

    /// Build a line chart from JSON.
    /// JSON shape: `[ { "x": 0, "y": 10 }, { "x": 1, "y": 12 }, ... ]`
    #[wasm_bindgen]
    pub fn set_line_chart(&mut self, data_json: &str) -> Result<(), JsValue> {
        let data: Vec<LineJs> = serde_json::from_str(data_json)
            .map_err(|e| JsValue::from_str(&format!("line parse: {e}")))?;
        let datums: Vec<LineDatum> = data
            .into_iter()
            .map(|d| LineDatum::new(d.series.unwrap_or_else(|| "data".to_string()), d.x, d.y))
            .collect();
        let spec = LineChartSpec::new(datums);
        let chart = spec
            .build_chart(self.workspace.clone(), self.chart_size())
            .map_err(|e| JsValue::from_str(&format!("line build: {e}")))?;
        self.set_chart(chart)
    }

    /// Build a scatter plot from JSON.
    /// JSON shape: `[ { "x": 1.0, "y": 2.0, "r": 4, "color": [r,g,b,a]? }, ... ]`
    #[wasm_bindgen]
    pub fn set_scatter_plot(&mut self, data_json: &str) -> Result<(), JsValue> {
        let data: Vec<ScatterJs> = serde_json::from_str(data_json)
            .map_err(|e| JsValue::from_str(&format!("scatter parse: {e}")))?;
        let datums: Vec<ScatterDatum> = data
            .into_iter()
            .map(|d| {
                let label = d.label.unwrap_or_default();
                let mut s = ScatterDatum::new(label, d.x, d.y);
                if let Some(r) = d.r {
                    s = s.with_radius(r);
                }
                s
            })
            .collect();
        let spec = ScatterPlotSpec::new(datums);
        let chart = spec
            .build_chart(self.workspace.clone(), self.chart_size())
            .map_err(|e| JsValue::from_str(&format!("scatter build: {e}")))?;
        self.set_chart(chart)
    }

    /// Build a heatmap from JSON.
    /// JSON shape: `[ { "row": "A", "col": "X", "value": 0.5 }, ... ]`
    #[wasm_bindgen]
    pub fn set_heatmap(&mut self, data_json: &str) -> Result<(), JsValue> {
        let data: Vec<HeatmapJs> = serde_json::from_str(data_json)
            .map_err(|e| JsValue::from_str(&format!("heatmap parse: {e}")))?;
        let cells: Vec<HeatmapCell> = data
            .into_iter()
            .map(|d| HeatmapCell::new(d.row, d.col, d.value))
            .collect();
        let spec = HeatmapSpec::new(cells);
        let chart = spec
            .build_chart(self.workspace.clone(), self.chart_size())
            .map_err(|e| JsValue::from_str(&format!("heatmap build: {e}")))?;
        self.set_chart(chart)
    }

    /// Build a force-directed graph from JSON.
    /// JSON shape: `{ "nodes": [{ "id": "a", "label": "...", "radius": 6, "color": [..]? }, ...],
    ///                "edges": [{ "source": "a", "target": "b", "weight": 1.0 }, ...] }`
    #[wasm_bindgen]
    pub fn set_force_graph(&mut self, data_json: &str) -> Result<(), JsValue> {
        let g: ForceGraphJs = serde_json::from_str(data_json)
            .map_err(|e| JsValue::from_str(&format!("force parse: {e}")))?;
        let nodes: Vec<ForceNode> = g
            .nodes
            .into_iter()
            .map(|n| {
                let label = n.label.unwrap_or_else(|| n.id.clone());
                let mut node = ForceNode::new(n.id, label);
                if let Some(r) = n.radius {
                    node = node.with_radius(r);
                }
                if let Some(c) = n.color {
                    node = node.with_color(c);
                }
                node
            })
            .collect();
        let edges: Vec<ForceEdge> = g
            .edges
            .into_iter()
            .map(|e| {
                let mut edge = ForceEdge::new(e.source, e.target);
                if let Some(w) = e.weight {
                    edge = edge.with_weight(w);
                }
                edge
            })
            .collect();
        let spec = ForceSpec::new(nodes, edges);
        let chart = spec
            .build_chart(self.workspace.clone(), self.chart_size())
            .map_err(|e| JsValue::from_str(&format!("force build: {e}")))?;
        self.set_chart(chart)
    }

    /// Render the current scene. No-op if no chart has been set or init wasn't called.
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| JsValue::from_str("renderer not initialized — call init() first"))?;
        let chart = self
            .chart
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no chart set — call set_* first"))?;
        renderer
            .render(chart)
            .map_err(|e| JsValue::from_str(&format!("render: {e}")))?;
        Ok(())
    }

    /// Hit-test a point in screen pixels. Returns `null` on miss; otherwise
    /// `{ mark, row, distance }`.
    #[wasm_bindgen]
    pub fn pick(&self, x: f32, y: f32) -> Result<JsValue, JsValue> {
        let chart = match &self.chart {
            Some(c) => c,
            None => return Ok(JsValue::NULL),
        };
        let scales = self.workspace.scales();
        let datasets = self.workspace.datasets();
        let selection = self.workspace.selection();
        for layer in chart.scene().layers.iter().rev() {
            let coord = match self.workspace.coord(layer.coord) {
                Some(c) => c,
                None => continue,
            };
            let ctx = PickCtx::new(
                coord.as_ref(),
                &scales,
                &datasets,
                &selection,
                chart.scene().viewport.plot_area,
                chart.scene().viewport.device_pixel_ratio,
            );
            for mark in layer.marks.iter().rev() {
                if let Some(hit) = mark.pick(&ctx, (x, y)) {
                    let out = PickJs {
                        mark: hit.mark.get() as u32,
                        row: hit.row,
                        distance: hit.distance,
                    };
                    return serde_wasm_bindgen::to_value(&out)
                        .map_err(|e| JsValue::from_str(&format!("pick serialize: {e}")));
                }
            }
        }
        Ok(JsValue::NULL)
    }

    fn chart_size(&self) -> ChartSize {
        ChartSize::new(self.logical_width, self.logical_height)
            .with_device_pixel_ratio(self.physical_width as f32 / self.logical_width as f32)
    }

    fn set_chart(&mut self, chart: Chart) -> Result<(), JsValue> {
        self.chart = Some(chart);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer
                .render(self.chart.as_ref().unwrap())
                .map_err(|e| JsValue::from_str(&format!("render: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // The wasm-bindgen API can't be unit-tested in plain `cargo test` (needs
    // a real canvas). Smoke tests for the JSON-decoding helpers below ensure
    // payload shapes parse without surprises.

    use super::*;

    #[test]
    fn bar_json_parses() {
        let v: Vec<BarJs> = serde_json::from_str(
            r#"[{"label":"Q1","value":42.0},{"label":"Q2","value":57.0}]"#,
        )
        .unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].label, "Q2");
    }

    #[test]
    fn force_json_parses() {
        let g: ForceGraphJs = serde_json::from_str(
            r#"{"nodes":[{"id":"a"},{"id":"b","radius":8.0}],"edges":[{"source":"a","target":"b","weight":2.0}]}"#,
        )
        .unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
    }

    #[test]
    fn scatter_json_parses_with_optional_fields() {
        let v: Vec<ScatterJs> = serde_json::from_str(
            r#"[{"x":1.0,"y":2.0},{"label":"big","x":3.0,"y":4.0,"r":5.0}]"#,
        )
        .unwrap();
        assert_eq!(v.len(), 2);
        assert!(v[0].r.is_none());
        assert_eq!(v[1].r, Some(5.0));
        assert_eq!(v[1].label.as_deref(), Some("big"));
    }
}
