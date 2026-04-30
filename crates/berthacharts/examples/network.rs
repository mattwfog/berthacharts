//! Build a Sankey chart with the optional `network` feature.

use berthacharts::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chart = SankeySpec::from_flows(vec![
        SankeyFlow::new("Lead", "Qualified", 120.0).with_class("pipeline"),
        SankeyFlow::new("Qualified", "Won", 48.0).with_class("pipeline"),
        SankeyFlow::new("Qualified", "Lost", 72.0).with_class("pipeline"),
    ])
    .build(ChartSize::new(900, 420))?;

    println!(
        "sankey: {} layer(s), {} snap target(s)",
        chart.scene().layers.len(),
        chart.snap_targets().len()
    );

    Ok(())
}
