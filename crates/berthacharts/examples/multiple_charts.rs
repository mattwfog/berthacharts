//! Build multiple first-party chart specs through one public import path.

use berthacharts::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let size = ChartSize::new(720, 360);

    let line = LineChartSpec::new(vec![
        LineDatum::new("Actual", 1.0, 12.0),
        LineDatum::new("Actual", 2.0, 18.0),
        LineDatum::new("Actual", 3.0, 21.0),
        LineDatum::new("Plan", 1.0, 14.0),
        LineDatum::new("Plan", 2.0, 17.0),
        LineDatum::new("Plan", 3.0, 20.0),
    ])
    .build(size)?;

    let scatter = ScatterPlotSpec::new(vec![
        ScatterDatum::new("A", 0.2, 0.8).with_group("north"),
        ScatterDatum::new("B", 0.5, 0.4).with_group("south"),
        ScatterDatum::new("C", 0.8, 0.7).with_group("north"),
    ])
    .build(size)?;

    println!(
        "line: {} guide(s); scatter: {} guide(s)",
        line.scene().guides.len(),
        scatter.scene().guides.len()
    );

    Ok(())
}
