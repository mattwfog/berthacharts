//! Minimal bar chart build using the public facade crate.

use berthacharts::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chart = BarChartSpec::new(vec![
        BarDatum::new("Jan", 18.0),
        BarDatum::new("Feb", 24.0),
        BarDatum::new("Mar", 31.0),
        BarDatum::new("Apr", 28.0),
    ])
    .with_target(25.0)
    .build(ChartSize::new(800, 420))?;

    println!(
        "built {} layer(s), {} guide(s), {} snap target(s)",
        chart.scene().layers.len(),
        chart.scene().guides.len(),
        chart.snap_targets().len()
    );

    Ok(())
}
