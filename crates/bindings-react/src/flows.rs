//! Flow-chart bindings: the network crate's Sankey exposed to JS.
//!
//! Lives in its own module (one `mod flows;` hook in lib.rs) so the flow
//! surface can grow — chord / force / sunburst next — without churning the
//! core binding file.
//!
//! JS API:
//!   chart.sankey(jsonString);
//!
//! Input JSON:
//!   {
//!     "flows":  [{"source": "from_online", "target": "to_recurring",
//!                 "value": 118, "class": "online"}, …],
//!     "labels": {"from_online": "Online", "to_recurring": "Recurring"},  // optional
//!     "order":  {"from_online": 0, "from_recurring": 1},                 // optional
//!     "stages": [{"index": 0, "label": "This order"},
//!                {"index": 1, "label": "Next order"}]                    // optional
//!   }
//!
//! Nodes and stage indexes are inferred from the flows (first-seen order,
//! longest upstream path); `labels` overrides the humanized node ids and
//! `order` pins vertical position within a stage. Self-transitions need
//! distinct ids per side (e.g. `from_online` → `to_online`) — a Sankey link
//! from a node to itself is degenerate by construction.

use std::collections::HashMap;

use berthacharts_network::{SankeyFlow, SankeySpec, SankeyStage};
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use berthacharts_core::{ChartSize, ChartSpec};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::BerthaChart;

#[derive(Deserialize)]
struct FlowInput {
    source: String,
    target: String,
    value: f32,
    #[serde(default)]
    class: Option<String>,
}

#[derive(Deserialize)]
struct StageInput {
    index: usize,
    label: String,
}

#[derive(Deserialize)]
struct SankeyInput {
    flows: Vec<FlowInput>,
    #[serde(default)]
    labels: HashMap<String, String>,
    #[serde(default)]
    order: HashMap<String, i32>,
    #[serde(default)]
    stages: Vec<StageInput>,
}

impl SankeyInput {
    fn into_spec(self) -> SankeySpec {
        let flows = self
            .flows
            .into_iter()
            .map(|f| {
                let mut flow = SankeyFlow::new(f.source, f.target, f.value);
                if let Some(class) = f.class {
                    flow = flow.with_class(class);
                }
                flow
            })
            .collect();
        let mut spec = SankeySpec::from_flows(flows);
        for node in &mut spec.nodes {
            if let Some(label) = self.labels.get(&node.id) {
                node.label.clone_from(label);
            }
            if let Some(order) = self.order.get(&node.id) {
                node.order = Some(*order);
            }
        }
        spec.stages = self
            .stages
            .into_iter()
            .map(|s| SankeyStage::new(s.index, s.label))
            .collect();
        spec
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl BerthaChart {
    /// Render a Sankey flow diagram from JSON.
    #[wasm_bindgen]
    pub fn sankey(&mut self, json: &str) -> Result<(), JsValue> {
        let input: SankeyInput =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let spec = input.into_spec();
        let size = ChartSize::new(self.logical_w, self.logical_h);
        let chart = spec
            .build_chart(self.workspace.clone(), size)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.render_chart(&chart)
    }
}
