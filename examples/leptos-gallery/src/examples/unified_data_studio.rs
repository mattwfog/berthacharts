//! Unified multi-dataset chart dashboard demo.

use std::sync::Arc;

use berthacharts_charts::bar::{BarChartOptions, BarChartSpec, BarDatum};
use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use berthacharts_charts::scatter::{ScatterDatum, ScatterPlotOptions, ScatterPlotSpec};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const TREND_W: u32 = 720;
const TREND_H: u32 = 350;
const BAR_W: u32 = 520;
const BAR_H: u32 = 310;
const SCATTER_W: u32 = 620;
const SCATTER_H: u32 = 350;

const REGIONS: [&str; 4] = ["West", "Northeast", "South", "Central"];
const CHANNELS: [&str; 4] = ["Lifecycle", "Partner", "Paid search", "Paid social"];

const MARKET_ROWS: [MarketRow; 24] = [
    MarketRow::new(
        "Northeast",
        "Paid search",
        1,
        142.0,
        1840.0,
        910.0,
        88.0,
        31.0,
        5.8,
        62.0,
    ),
    MarketRow::new(
        "Northeast",
        "Partner",
        2,
        151.0,
        1910.0,
        940.0,
        83.0,
        28.0,
        5.2,
        66.0,
    ),
    MarketRow::new(
        "Northeast",
        "Paid social",
        3,
        158.0,
        2070.0,
        990.0,
        91.0,
        34.0,
        5.9,
        64.0,
    ),
    MarketRow::new(
        "Northeast",
        "Lifecycle",
        4,
        171.0,
        2180.0,
        1040.0,
        74.0,
        24.0,
        4.6,
        70.0,
    ),
    MarketRow::new(
        "Northeast",
        "Partner",
        5,
        184.0,
        2290.0,
        1085.0,
        78.0,
        22.0,
        4.3,
        73.0,
    ),
    MarketRow::new(
        "Northeast",
        "Lifecycle",
        6,
        196.0,
        2390.0,
        1130.0,
        70.0,
        20.0,
        4.1,
        75.0,
    ),
    MarketRow::new(
        "South",
        "Paid social",
        1,
        118.0,
        1760.0,
        840.0,
        96.0,
        44.0,
        7.4,
        52.0,
    ),
    MarketRow::new(
        "South",
        "Paid search",
        2,
        126.0,
        1810.0,
        872.0,
        102.0,
        46.0,
        7.8,
        50.0,
    ),
    MarketRow::new(
        "South", "Partner", 3, 137.0, 1880.0, 902.0, 92.0, 39.0, 6.9, 55.0,
    ),
    MarketRow::new(
        "South",
        "Paid social",
        4,
        149.0,
        1975.0,
        934.0,
        104.0,
        41.0,
        6.6,
        57.0,
    ),
    MarketRow::new(
        "South",
        "Lifecycle",
        5,
        155.0,
        2035.0,
        950.0,
        81.0,
        33.0,
        5.7,
        61.0,
    ),
    MarketRow::new(
        "South", "Partner", 6, 166.0, 2115.0, 984.0, 88.0, 31.0, 5.3, 64.0,
    ),
    MarketRow::new(
        "West", "Partner", 1, 164.0, 1940.0, 1010.0, 79.0, 26.0, 4.9, 72.0,
    ),
    MarketRow::new(
        "West",
        "Lifecycle",
        2,
        176.0,
        2040.0,
        1068.0,
        71.0,
        24.0,
        4.4,
        76.0,
    ),
    MarketRow::new(
        "West",
        "Paid search",
        3,
        181.0,
        2120.0,
        1092.0,
        86.0,
        27.0,
        4.7,
        74.0,
    ),
    MarketRow::new(
        "West",
        "Lifecycle",
        4,
        194.0,
        2250.0,
        1135.0,
        68.0,
        21.0,
        3.9,
        79.0,
    ),
    MarketRow::new(
        "West", "Partner", 5, 207.0, 2385.0, 1192.0, 73.0, 19.0, 3.7, 82.0,
    ),
    MarketRow::new(
        "West",
        "Lifecycle",
        6,
        219.0,
        2460.0,
        1238.0,
        66.0,
        17.0,
        3.4,
        84.0,
    ),
    MarketRow::new(
        "Central",
        "Paid search",
        1,
        104.0,
        1580.0,
        760.0,
        69.0,
        35.0,
        6.8,
        58.0,
    ),
    MarketRow::new(
        "Central", "Partner", 2, 112.0, 1660.0, 792.0, 66.0, 32.0, 6.1, 62.0,
    ),
    MarketRow::new(
        "Central",
        "Paid social",
        3,
        121.0,
        1735.0,
        826.0,
        76.0,
        34.0,
        6.4,
        60.0,
    ),
    MarketRow::new(
        "Central", "Partner", 4, 130.0, 1810.0, 864.0, 68.0, 30.0, 5.8, 64.0,
    ),
    MarketRow::new(
        "Central",
        "Lifecycle",
        5,
        143.0,
        1905.0,
        914.0,
        58.0,
        26.0,
        5.0,
        69.0,
    ),
    MarketRow::new(
        "Central",
        "Lifecycle",
        6,
        151.0,
        1985.0,
        948.0,
        54.0,
        23.0,
        4.7,
        71.0,
    ),
];

const INVENTORY_ROWS: [InventoryRow; 6] = [
    InventoryRow::new("Core", "Meals", 612.0, 18.0, 4.2, 2.8),
    InventoryRow::new("Fresh", "Meals", 404.0, 9.0, 5.6, 4.1),
    InventoryRow::new("Limited", "Specials", 282.0, 7.0, 6.3, 6.8),
    InventoryRow::new("Beverage", "Attach", 211.0, 22.0, 3.1, 1.9),
    InventoryRow::new("Merch", "Attach", 128.0, 31.0, 7.4, 8.2),
    InventoryRow::new("Wholesale", "Partner", 246.0, 12.0, 8.1, 5.6),
];

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);
    let show_diagnostics = RwSignal::new(true);
    let lens = RwSignal::new(FusionLens::Revenue);

    let summary = move || FusionSummary::from_rows(&MARKET_ROWS, &INVENTORY_ROWS, lens.get());
    let trend_chart = move || {
        fusion_chart_view(
            Arc::new(omni_signal_spec(lens.get())),
            TREND_W,
            TREND_H,
            "fusion trend spec",
        )
    };
    let channel_chart = move || {
        fusion_chart_view(
            Arc::new(channel_yield_spec(lens.get())),
            BAR_W,
            BAR_H,
            "channel yield spec",
        )
    };
    let frontier_chart = move || {
        fusion_chart_view(
            Arc::new(frontier_spec(lens.get())),
            SCATTER_W,
            SCATTER_H,
            "market frontier spec",
        )
    };
    let inventory_chart = move || {
        fusion_chart_view(
            Arc::new(inventory_risk_spec(lens.get())),
            BAR_W,
            BAR_H,
            "inventory risk spec",
        )
    };
    let allocation_chart = move || {
        fusion_chart_view(
            Arc::new(allocation_spec(lens.get())),
            BAR_W,
            BAR_H,
            "allocation spec",
        )
    };
    let regional_rows = move || {
        REGIONS
            .into_iter()
            .map(|region| regional_row(region, lens.get()))
            .collect_view()
    };
    let action_stack = move || {
        ranked_actions(lens.get())
            .into_iter()
            .map(|action| action_item(action.title, action.detail, action.impact))
            .collect_view()
    };
    let relationship_tiles = move || {
        metric_relationships(lens.get())
            .into_iter()
            .map(relationship_tile)
            .collect_view()
    };

    view! {
        <section id="unified-data-studio" class="example unified-data-studio">
            <div class="example-head">
                <div>
                    <h2>"Unified Data Studio"</h2>
                    <p>
                        "Coordinated charts generated from joined regional sales, media spend, order volume, support load, fulfillment quality, NPS, and inventory risk data."
                    </p>
                </div>
                <div class="stat-strip fusion-stat-strip">
                    <span><strong>{move || format_money_k(summary().revenue)}</strong>" revenue"</span>
                    <span><strong>{move || format!("{:.1}x", summary().media_roas)}</strong>" media ROAS"</span>
                    <span><strong>{move || format!("{:.1}%", summary().margin_rate)}</strong>" margin"</span>
                    <span><strong>{move || format!("{:.0}", summary().nps)}</strong>" NPS"</span>
                    <span><strong>{move || format!("{:.0}", summary().portfolio_score)}</strong>" score"</span>
                </div>
            </div>

            <div class="fusion-lensbar" aria-label="Unified data model lens">
                <FusionLensButton label="Revenue" value=FusionLens::Revenue lens=lens />
                <FusionLensButton label="Efficiency" value=FusionLens::Efficiency lens=lens />
                <FusionLensButton label="Risk" value=FusionLens::Risk lens=lens />
            </div>

            <div class="fusion-kpi-grid">
                {fusion_kpi("Joined rows", move || MARKET_ROWS.len().to_string(), move || "region x week x channel".to_string())}
                {fusion_kpi("Top signal", move || summary().best_region.to_string(), move || format!("{} score", format!("{:.0}", summary().best_region_score)))}
                {fusion_kpi("Demand covered", move || format!("{:.0}%", summary().demand_coverage), move || "orders vs available stock".to_string())}
                {fusion_kpi("Model confidence", move || format!("{:.0}%", summary().confidence * 100.0), move || summary().confidence_detail)}
            </div>

            <DisplayControls label="Unified data display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Axes" state=show_axes />
                <DisplayToggleButton label="Legend" state=show_legend />
                <DisplayToggleButton label="Diagnostics" state=show_diagnostics />
            </DisplayControls>

            <div class=move || fusion_stage_class(
                show_data_labels.get(),
                show_axes.get(),
                show_legend.get(),
                show_diagnostics.get(),
            )>
                <div class="fusion-chart-grid">
                    <div class="fusion-chart fusion-chart-wide">
                        <h3>{move || format!("{} signal index", lens.get().label())}</h3>
                        {trend_chart}
                    </div>
                    <div class="fusion-chart">
                        <h3>{move || format!("{} channel yield", lens.get().label())}</h3>
                        {channel_chart}
                    </div>
                    <div class="fusion-chart">
                        <h3>"Market frontier"</h3>
                        {frontier_chart}
                    </div>
                    <div class="fusion-chart">
                        <h3>{move || format!("{} allocation", lens.get().label())}</h3>
                        {allocation_chart}
                    </div>
                    <div class="fusion-chart">
                        <h3>"Inventory exposure"</h3>
                        {inventory_chart}
                    </div>
                </div>

                <div class="fusion-diagnostics">
                    <div class="fusion-panel">
                        <h3>"Regional join"</h3>
                        <div class="fusion-table-head">
                            <span>"Region"</span>
                            <b>"Revenue"</b>
                            <b>"ROAS"</b>
                            <b>"NPS"</b>
                            <b>"Score"</b>
                        </div>
                        {regional_rows}
                    </div>
                    <div class="fusion-panel">
                        <h3>"Signal relationships"</h3>
                        <div class="fusion-relationship-grid">
                            {relationship_tiles}
                        </div>
                    </div>
                    <div class="fusion-panel fusion-action-panel">
                        <h3>"Ranked action stack"</h3>
                        {action_stack}
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn FusionLensButton(
    #[prop(into)] label: String,
    value: FusionLens,
    lens: RwSignal<FusionLens>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || {
                if lens.get() == value {
                    "fusion-lens is-active"
                } else {
                    "fusion-lens"
                }
            }
            aria-pressed=move || (lens.get() == value).to_string()
            on:click=move |_| lens.set(value)
        >
            {label}
        </button>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FusionLens {
    Revenue,
    Efficiency,
    Risk,
}

impl FusionLens {
    const fn label(self) -> &'static str {
        match self {
            Self::Revenue => "Revenue",
            Self::Efficiency => "Efficiency",
            Self::Risk => "Risk",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MarketRow {
    region: &'static str,
    channel: &'static str,
    week: u32,
    revenue: f32,
    orders: f32,
    margin: f32,
    spend: f32,
    tickets: f32,
    fulfillment_hours: f32,
    nps: f32,
}

impl MarketRow {
    const fn new(
        region: &'static str,
        channel: &'static str,
        week: u32,
        revenue: f32,
        orders: f32,
        margin: f32,
        spend: f32,
        tickets: f32,
        fulfillment_hours: f32,
        nps: f32,
    ) -> Self {
        Self {
            region,
            channel,
            week,
            revenue,
            orders,
            margin,
            spend,
            tickets,
            fulfillment_hours,
            nps,
        }
    }

    fn roas(self) -> f32 {
        ratio(self.revenue, self.spend)
    }

    fn profit_per_order(self) -> f32 {
        ratio(self.margin, self.orders)
    }

    fn spend_per_order(self) -> f32 {
        ratio(self.spend, self.orders)
    }

    fn service_drag(self) -> f32 {
        self.tickets / 10.0 + self.fulfillment_hours
    }
}

#[derive(Debug, Clone, Copy)]
struct InventoryRow {
    sku: &'static str,
    category: &'static str,
    revenue: f32,
    stock_days: f32,
    supplier_delay: f32,
    return_rate: f32,
}

impl InventoryRow {
    const fn new(
        sku: &'static str,
        category: &'static str,
        revenue: f32,
        stock_days: f32,
        supplier_delay: f32,
        return_rate: f32,
    ) -> Self {
        Self {
            sku,
            category,
            revenue,
            stock_days,
            supplier_delay,
            return_rate,
        }
    }

    fn risk_score(self) -> f32 {
        let velocity = self.revenue / 12.0;
        let scarcity = (28.0 - self.stock_days).max(0.0) * 2.4;
        velocity + scarcity + self.supplier_delay * 5.0 + self.return_rate * 2.1
    }

    fn revenue_density(self) -> f32 {
        ratio(self.revenue, self.stock_days.max(1.0))
    }

    fn recovery_score(self) -> f32 {
        (96.0 - self.supplier_delay * 5.2 - self.return_rate * 3.1 + self.stock_days * 0.55)
            .clamp(0.0, 100.0)
    }
}

#[derive(Debug, Clone)]
struct FusionSummary {
    revenue: f32,
    media_roas: f32,
    margin_rate: f32,
    nps: f32,
    demand_coverage: f32,
    portfolio_score: f32,
    confidence: f32,
    confidence_detail: String,
    best_region: &'static str,
    best_region_score: f32,
}

impl FusionSummary {
    fn from_rows(market: &[MarketRow], inventory: &[InventoryRow], lens: FusionLens) -> Self {
        let revenue = market.iter().map(|row| row.revenue).sum::<f32>();
        let margin = market.iter().map(|row| row.margin).sum::<f32>();
        let spend = market.iter().map(|row| row.spend).sum::<f32>();
        let orders = market.iter().map(|row| row.orders).sum::<f32>();
        let nps = weighted_avg(market.iter().map(|row| (row.nps, row.revenue)));
        let stock_days = inventory.iter().map(|row| row.stock_days).sum::<f32>();
        let best = REGIONS
            .into_iter()
            .map(|region| {
                let score = region_profile(region).score(lens);
                (region, score)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or(("West", 0.0));
        let profiles: Vec<RegionProfile> = REGIONS.into_iter().map(region_profile).collect();
        let portfolio_score = weighted_avg(
            profiles
                .iter()
                .map(|profile| (profile.score(lens), profile.revenue)),
        );
        let confidence = model_confidence(lens, market, inventory);

        Self {
            revenue,
            media_roas: ratio(revenue, spend),
            margin_rate: ratio(margin, revenue) * 100.0,
            nps,
            demand_coverage: ratio(stock_days * 126.0, orders) * 100.0,
            portfolio_score,
            confidence,
            confidence_detail: confidence_detail(lens, confidence),
            best_region: best.0,
            best_region_score: best.1,
        }
    }
}

fn fusion_chart_view<T>(spec: Arc<T>, width: u32, height: u32, label: &'static str) -> impl IntoView
where
    T: ChartSpec + Send + Sync + 'static,
    T::Error: std::fmt::Debug,
{
    let build: BuildChart = Arc::new(move |ws| {
        spec.build_chart(ws, ChartSize::new(width, height))
            .expect(label)
    });

    view! {
        <ChartCanvas width=width height=height builder=build />
    }
}

fn omni_signal_spec(lens: FusionLens) -> LineChartSpec {
    let data = (1..=6)
        .flat_map(|week| {
            let rows = rows_for_week(week);
            let revenue = rows.iter().map(|row| row.revenue).sum::<f32>();
            let orders = rows.iter().map(|row| row.orders).sum::<f32>();
            let spend = rows.iter().map(|row| row.spend).sum::<f32>();
            let nps = weighted_avg(rows.iter().map(|row| (row.nps, row.revenue)));
            let drag = rows.iter().map(|row| row.service_drag()).sum::<f32>() / rows.len() as f32;
            let roas = ratio(revenue, spend);

            let mut points = vec![
                LineDatum::new("Revenue index", week as f32, index_value(revenue, 528.0))
                    .with_label(format!("Week {week} revenue")),
                LineDatum::new("Order index", week as f32, index_value(orders, 7120.0))
                    .with_label(format!("Week {week} orders")),
                LineDatum::new("ROAS index", week as f32, index_value(roas, 1.62))
                    .with_label(format!("Week {week} ROAS")),
                LineDatum::new("NPS index", week as f32, index_value(nps, 60.0))
                    .with_label(format!("Week {week} NPS")),
                LineDatum::new("Drag index", week as f32, index_value(drag, 10.2))
                    .with_label(format!("Week {week} service drag")),
            ];
            points.push(
                LineDatum::new("Composite", week as f32, weekly_score(week, lens))
                    .with_label(format!("Week {week} {} composite", lens.label())),
            );
            points
        })
        .collect();

    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Week".to_string(),
        y_axis_label: "Index, week 1 = 100".to_string(),
        x_domain: Some((1.0, 6.0)),
        y_domain: Some((55.0, 160.0)),
        x_tick_count: 6,
        y_tick_count: 5,
        padding_right: 128.0,
        ..LineChartOptions::default()
    })
}

fn channel_yield_spec(lens: FusionLens) -> BarChartSpec {
    BarChartSpec::new(
        CHANNELS
            .into_iter()
            .map(|channel| {
                let rows = rows_for_channel(channel);
                let margin = rows.iter().map(|row| row.margin).sum::<f32>();
                let spend = rows.iter().map(|row| row.spend).sum::<f32>();
                let revenue = rows.iter().map(|row| row.revenue).sum::<f32>();
                let orders = rows.iter().map(|row| row.orders).sum::<f32>();
                let drag =
                    rows.iter().map(|row| row.service_drag()).sum::<f32>() / rows.len() as f32;
                let value = match lens {
                    FusionLens::Revenue => ratio(margin, spend),
                    FusionLens::Efficiency => ratio(margin, orders) * 100.0,
                    FusionLens::Risk => ratio(revenue, drag) / 10.0,
                };
                BarDatum::new(channel, value)
            })
            .collect(),
    )
    .with_options(BarChartOptions {
        x_axis_label: "Channel".to_string(),
        y_axis_label: match lens {
            FusionLens::Revenue => "Gross margin / spend",
            FusionLens::Efficiency => "Margin cents / order",
            FusionLens::Risk => "Revenue / drag",
        }
        .to_string(),
        y_max: Some(match lens {
            FusionLens::Revenue => 14.0,
            FusionLens::Efficiency => 62.0,
            FusionLens::Risk => 18.0,
        }),
        target: Some(match lens {
            FusionLens::Revenue => 8.0,
            FusionLens::Efficiency => 50.0,
            FusionLens::Risk => 12.0,
        }),
        y_tick_count: 5,
        max_visible_labels: Some(CHANNELS.len()),
        ..BarChartOptions::default()
    })
}

fn frontier_spec(lens: FusionLens) -> ScatterPlotSpec {
    let data = REGIONS
        .into_iter()
        .flat_map(|region| {
            rows_for_region(region).into_iter().map(move |row| {
                let (x, y, radius) = match lens {
                    FusionLens::Revenue => (
                        row.spend_per_order(),
                        row.profit_per_order(),
                        (row.roas() * 1.4).clamp(4.6, 9.0),
                    ),
                    FusionLens::Efficiency => (
                        row.service_drag(),
                        row.roas(),
                        (row.profit_per_order() * 12.0).clamp(4.6, 9.0),
                    ),
                    FusionLens::Risk => (
                        row.fulfillment_hours,
                        row.tickets / row.orders * 1000.0,
                        (row.revenue / 26.0).clamp(4.6, 9.0),
                    ),
                };
                ScatterDatum::new(format!("{} wk{}", row.region, row.week), x, y)
                    .with_group(row.region)
                    .with_radius(radius)
            })
        })
        .collect();

    ScatterPlotSpec::new(data).with_options(ScatterPlotOptions {
        x_axis_label: match lens {
            FusionLens::Revenue => "Media spend / order",
            FusionLens::Efficiency => "Service drag",
            FusionLens::Risk => "Fulfillment hours",
        }
        .to_string(),
        y_axis_label: match lens {
            FusionLens::Revenue => "Gross margin / order",
            FusionLens::Efficiency => "ROAS",
            FusionLens::Risk => "Tickets / 1k orders",
        }
        .to_string(),
        x_domain: Some(match lens {
            FusionLens::Revenue => (0.025, 0.075),
            FusionLens::Efficiency => (5.0, 13.2),
            FusionLens::Risk => (3.0, 8.4),
        }),
        y_domain: Some(match lens {
            FusionLens::Revenue => (0.43, 0.55),
            FusionLens::Efficiency => (1.1, 2.8),
            FusionLens::Risk => (7.0, 28.0),
        }),
        x_tick_count: 5,
        y_tick_count: 5,
        max_visible_labels: 6,
        ..ScatterPlotOptions::default()
    })
}

fn inventory_risk_spec(lens: FusionLens) -> BarChartSpec {
    BarChartSpec::new(
        INVENTORY_ROWS
            .iter()
            .map(|row| {
                let value = match lens {
                    FusionLens::Revenue => row.revenue_density(),
                    FusionLens::Efficiency => row.recovery_score(),
                    FusionLens::Risk => row.risk_score(),
                };
                BarDatum::new(format!("{} {}", row.sku, row.category), value)
            })
            .collect(),
    )
    .with_options(BarChartOptions {
        x_axis_label: "SKU family".to_string(),
        y_axis_label: match lens {
            FusionLens::Revenue => "Revenue density",
            FusionLens::Efficiency => "Recovery score",
            FusionLens::Risk => "Risk score",
        }
        .to_string(),
        y_max: Some(match lens {
            FusionLens::Revenue => 54.0,
            FusionLens::Efficiency => 92.0,
            FusionLens::Risk => 150.0,
        }),
        target: Some(match lens {
            FusionLens::Revenue => 32.0,
            FusionLens::Efficiency => 72.0,
            FusionLens::Risk => 100.0,
        }),
        y_tick_count: 5,
        max_visible_labels: Some(INVENTORY_ROWS.len()),
        ..BarChartOptions::default()
    })
}

fn allocation_spec(lens: FusionLens) -> BarChartSpec {
    let profiles: Vec<RegionProfile> = REGIONS.into_iter().map(region_profile).collect();
    let max_score = profiles
        .iter()
        .map(|profile| profile.score(lens))
        .fold(0.0, f32::max);

    BarChartSpec::new(
        profiles
            .into_iter()
            .map(|profile| BarDatum::new(profile.region, profile.score(lens)))
            .collect(),
    )
    .with_options(BarChartOptions {
        x_axis_label: "Region".to_string(),
        y_axis_label: format!("{} score", lens.label()),
        y_max: Some(nice_upper(max_score.max(1.0))),
        target: Some(match lens {
            FusionLens::Revenue => 110.0,
            FusionLens::Efficiency => 92.0,
            FusionLens::Risk => 78.0,
        }),
        y_tick_count: 5,
        max_visible_labels: Some(REGIONS.len()),
        ..BarChartOptions::default()
    })
}

fn regional_row(region: &'static str, lens: FusionLens) -> impl IntoView {
    let profile = region_profile(region);

    view! {
        <div class="fusion-table-row">
            <span>{region}</span>
            <strong>{format_money_k(profile.revenue)}</strong>
            <em>{format!("{:.1}x", profile.roas())}</em>
            <i>{format!("{:.0}", profile.nps)}</i>
            <b>{format!("{:.0}", profile.score(lens))}</b>
        </div>
    }
}

fn fusion_kpi(
    label: &'static str,
    value: impl Fn() -> String + Copy + Send + 'static,
    detail: impl Fn() -> String + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <div class="fusion-kpi">
            <span>{label}</span>
            <strong>{move || value()}</strong>
            <em>{move || detail()}</em>
        </div>
    }
}

fn action_item(title: &'static str, detail: &'static str, impact: f32) -> impl IntoView {
    view! {
        <article class="fusion-action-item">
            <strong>{title}<span>{format!("{impact:.0}")}</span></strong>
            <em>{detail}</em>
        </article>
    }
}

#[derive(Debug, Clone, Copy)]
struct RegionProfile {
    region: &'static str,
    revenue: f32,
    margin: f32,
    spend: f32,
    orders: f32,
    nps: f32,
    drag: f32,
    revenue_momentum: f32,
}

impl RegionProfile {
    fn roas(self) -> f32 {
        ratio(self.revenue, self.spend)
    }

    fn margin_rate(self) -> f32 {
        ratio(self.margin, self.revenue) * 100.0
    }

    fn score(self, lens: FusionLens) -> f32 {
        match lens {
            FusionLens::Revenue => {
                self.revenue / 8.5 + self.revenue_momentum * 1.8 + self.roas() * 10.0
            }
            FusionLens::Efficiency => {
                self.margin_rate() * 1.35 + self.roas() * 13.0 + self.nps * 0.18 - self.drag * 1.4
            }
            FusionLens::Risk => {
                112.0 - self.drag * 4.4 + self.nps * 0.34 + self.margin_rate() * 0.42
            }
        }
        .max(0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct FusionAction {
    title: &'static str,
    detail: &'static str,
    impact: f32,
}

#[derive(Debug, Clone, Copy)]
struct RelationshipTile {
    label: &'static str,
    value: f32,
    detail: &'static str,
}

fn relationship_tile(tile: RelationshipTile) -> impl IntoView {
    let strength = tile.value.abs().clamp(0.0, 1.0);
    let style = format!("--relationship-strength:{strength:.2}");
    view! {
        <div class="fusion-relationship" style=style>
            <span>{tile.label}</span>
            <strong>{format_signed_decimal(tile.value)}</strong>
            <em>{tile.detail}</em>
            <i></i>
        </div>
    }
}

fn ranked_actions(lens: FusionLens) -> Vec<FusionAction> {
    let west = region_profile("West").score(lens);
    let south = region_profile("South").score(lens);
    let central = region_profile("Central").score(lens);
    let northeast = region_profile("Northeast").score(lens);
    match lens {
        FusionLens::Revenue => vec![
            FusionAction {
                title: "Move incremental spend West",
                detail: "West has the strongest revenue momentum and keeps media efficiency above the portfolio line.",
                impact: west,
            },
            FusionAction {
                title: "Package Northeast lifecycle offers",
                detail: "Northeast has improving NPS and low support load, making expansion revenue less fragile.",
                impact: northeast,
            },
            FusionAction {
                title: "Limit Southern acquisition bursts",
                detail: "South has demand, but support and fulfillment drag are absorbing too much contribution.",
                impact: south,
            },
        ],
        FusionLens::Efficiency => vec![
            FusionAction {
                title: "Rebalance to lifecycle and partner",
                detail: "Owned and partner channels convert margin with fewer service touches than paid social.",
                impact: channel_score("Lifecycle", lens),
            },
            FusionAction {
                title: "Protect Central CAC discipline",
                detail: "Central is smaller but efficient enough to scale through partner inventory bundles.",
                impact: central,
            },
            FusionAction {
                title: "Audit South service leakage",
                detail: "High ticket volume is masking demand quality and suppressing contribution per order.",
                impact: south,
            },
        ],
        FusionLens::Risk => vec![
            FusionAction {
                title: "Expedite limited inventory",
                detail: "Limited drops combine short stock cover, supplier delay, and elevated return exposure.",
                impact: sku_risk("Limited"),
            },
            FusionAction {
                title: "De-risk Southern fulfillment",
                detail: "South has the highest ticket and delay profile, so demand acceleration needs service capacity first.",
                impact: 100.0 - south,
            },
            FusionAction {
                title: "Hold West as the control group",
                detail: "West has enough demand quality to anchor the baseline while higher-risk regions are corrected.",
                impact: west,
            },
        ],
    }
}

fn metric_relationships(lens: FusionLens) -> Vec<RelationshipTile> {
    match lens {
        FusionLens::Revenue => vec![
            RelationshipTile {
                label: "Revenue x orders",
                value: correlation_by_region(|p| p.revenue, |p| p.orders),
                detail: "demand scale",
            },
            RelationshipTile {
                label: "Revenue x NPS",
                value: correlation_by_region(|p| p.revenue, |p| p.nps),
                detail: "quality tailwind",
            },
            RelationshipTile {
                label: "Revenue x drag",
                value: correlation_by_region(|p| p.revenue, |p| p.drag),
                detail: "operational load",
            },
        ],
        FusionLens::Efficiency => vec![
            RelationshipTile {
                label: "ROAS x margin",
                value: correlation_by_region(|p| p.roas(), |p| p.margin_rate()),
                detail: "spend quality",
            },
            RelationshipTile {
                label: "NPS x drag",
                value: correlation_by_region(|p| p.nps, |p| p.drag),
                detail: "service penalty",
            },
            RelationshipTile {
                label: "Score x ROAS",
                value: correlation_by_region(|p| p.score(FusionLens::Efficiency), |p| p.roas()),
                detail: "model fit",
            },
        ],
        FusionLens::Risk => vec![
            RelationshipTile {
                label: "Drag x NPS",
                value: correlation_by_region(|p| p.drag, |p| p.nps),
                detail: "experience risk",
            },
            RelationshipTile {
                label: "Stock x delay",
                value: inventory_correlation(|row| row.stock_days, |row| row.supplier_delay),
                detail: "supply exposure",
            },
            RelationshipTile {
                label: "Returns x risk",
                value: inventory_correlation(|row| row.return_rate, |row| row.risk_score()),
                detail: "quality leakage",
            },
        ],
    }
}

fn region_profile(region: &'static str) -> RegionProfile {
    let rows = rows_for_region(region);
    let revenue = rows.iter().map(|row| row.revenue).sum::<f32>();
    let margin = rows.iter().map(|row| row.margin).sum::<f32>();
    let spend = rows.iter().map(|row| row.spend).sum::<f32>();
    let orders = rows.iter().map(|row| row.orders).sum::<f32>();
    let nps = weighted_avg(rows.iter().map(|row| (row.nps, row.revenue)));
    let drag = rows.iter().map(|row| row.service_drag()).sum::<f32>() / rows.len() as f32;
    let first_half = rows
        .iter()
        .filter(|row| row.week <= 3)
        .map(|row| row.revenue)
        .sum::<f32>();
    let second_half = rows
        .iter()
        .filter(|row| row.week >= 4)
        .map(|row| row.revenue)
        .sum::<f32>();

    RegionProfile {
        region,
        revenue,
        margin,
        spend,
        orders,
        nps,
        drag,
        revenue_momentum: pct_delta(second_half, first_half),
    }
}

fn channel_score(channel: &str, lens: FusionLens) -> f32 {
    let rows = rows_for_channel(channel);
    let revenue = rows.iter().map(|row| row.revenue).sum::<f32>();
    let margin = rows.iter().map(|row| row.margin).sum::<f32>();
    let spend = rows.iter().map(|row| row.spend).sum::<f32>();
    let orders = rows.iter().map(|row| row.orders).sum::<f32>();
    let drag = rows.iter().map(|row| row.service_drag()).sum::<f32>() / rows.len() as f32;
    match lens {
        FusionLens::Revenue => ratio(revenue, spend) * 10.0,
        FusionLens::Efficiency => ratio(margin, orders) * 140.0 + ratio(margin, spend) * 4.0,
        FusionLens::Risk => ratio(revenue, drag),
    }
}

fn sku_risk(sku: &str) -> f32 {
    INVENTORY_ROWS
        .iter()
        .find(|row| row.sku == sku)
        .map_or(0.0, |row| row.risk_score())
}

fn weekly_score(week: u32, lens: FusionLens) -> f32 {
    let rows = rows_for_week(week);
    let revenue = rows.iter().map(|row| row.revenue).sum::<f32>();
    let margin = rows.iter().map(|row| row.margin).sum::<f32>();
    let spend = rows.iter().map(|row| row.spend).sum::<f32>();
    let orders = rows.iter().map(|row| row.orders).sum::<f32>();
    let nps = weighted_avg(rows.iter().map(|row| (row.nps, row.revenue)));
    let drag = rows.iter().map(|row| row.service_drag()).sum::<f32>() / rows.len() as f32;
    match lens {
        FusionLens::Revenue => index_value(revenue + margin * 0.35, 850.0),
        FusionLens::Efficiency => index_value(ratio(margin, spend) * 100.0 + nps, 850.0),
        FusionLens::Risk => {
            (140.0 - drag * 7.0 + nps * 0.35 + ratio(margin, orders) * 40.0).clamp(55.0, 155.0)
        }
    }
}

fn model_confidence(lens: FusionLens, market: &[MarketRow], inventory: &[InventoryRow]) -> f32 {
    let density = (market.len() as f32 / 24.0).min(1.0);
    let inventory_cover = (inventory.len() as f32 / 6.0).min(1.0);
    let agreement = match lens {
        FusionLens::Revenue => correlation_by_region(|p| p.revenue, |p| p.score(lens)).abs(),
        FusionLens::Efficiency => correlation_by_region(|p| p.roas(), |p| p.score(lens)).abs(),
        FusionLens::Risk => {
            inventory_correlation(|row| row.return_rate, |row| row.risk_score()).abs()
        }
    };
    (0.46 + density * 0.18 + inventory_cover * 0.14 + agreement * 0.22).clamp(0.0, 0.98)
}

fn confidence_detail(lens: FusionLens, confidence: f32) -> String {
    let source = match lens {
        FusionLens::Revenue => "demand + margin agreement",
        FusionLens::Efficiency => "ROAS + contribution fit",
        FusionLens::Risk => "inventory + service alignment",
    };
    format!("{source}, {:.0}% calibrated", confidence * 100.0)
}

fn correlation_by_region(
    x: impl Fn(RegionProfile) -> f32,
    y: impl Fn(RegionProfile) -> f32,
) -> f32 {
    let profiles: Vec<RegionProfile> = REGIONS.into_iter().map(region_profile).collect();
    correlation(
        profiles.iter().map(|profile| x(*profile)).collect(),
        profiles.iter().map(|profile| y(*profile)).collect(),
    )
}

fn inventory_correlation(x: impl Fn(InventoryRow) -> f32, y: impl Fn(InventoryRow) -> f32) -> f32 {
    correlation(
        INVENTORY_ROWS.iter().map(|row| x(*row)).collect(),
        INVENTORY_ROWS.iter().map(|row| y(*row)).collect(),
    )
}

fn correlation(xs: Vec<f32>, ys: Vec<f32>) -> f32 {
    if xs.len() != ys.len() || xs.len() < 2 {
        return 0.0;
    }
    let n = xs.len() as f32;
    let x_mean = xs.iter().sum::<f32>() / n;
    let y_mean = ys.iter().sum::<f32>() / n;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for (x, y) in xs.into_iter().zip(ys) {
        let dx = x - x_mean;
        let dy = y - y_mean;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if sxx <= f32::EPSILON || syy <= f32::EPSILON {
        0.0
    } else {
        sxy / (sxx * syy).sqrt()
    }
}

fn rows_for_week(week: u32) -> Vec<MarketRow> {
    MARKET_ROWS
        .iter()
        .copied()
        .filter(|row| row.week == week)
        .collect()
}

fn rows_for_region(region: &str) -> Vec<MarketRow> {
    MARKET_ROWS
        .iter()
        .copied()
        .filter(|row| row.region == region)
        .collect()
}

fn rows_for_channel(channel: &str) -> Vec<MarketRow> {
    MARKET_ROWS
        .iter()
        .copied()
        .filter(|row| row.channel == channel)
        .collect()
}

fn weighted_avg(values: impl Iterator<Item = (f32, f32)>) -> f32 {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (value, weight) in values {
        numerator += value * weight;
        denominator += weight;
    }
    ratio(numerator, denominator)
}

fn index_value(value: f32, baseline: f32) -> f32 {
    ratio(value, baseline) * 100.0
}

fn pct_delta(current: f32, previous: f32) -> f32 {
    if previous.abs() < f32::EPSILON {
        0.0
    } else {
        (current / previous - 1.0) * 100.0
    }
}

fn nice_upper(value: f32) -> f32 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let magnitude = 10_f32.powf(value.log10().floor());
    (value * 1.12 / magnitude).ceil() * magnitude
}

fn ratio(numerator: f32, denominator: f32) -> f32 {
    if denominator.abs() < f32::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

fn format_money_k(value: f32) -> String {
    if value >= 1_000.0 {
        format!("${:.2}m", value / 1_000.0)
    } else {
        format!("${value:.0}k")
    }
}

fn format_signed_decimal(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.2}")
    } else {
        format!("{value:.2}")
    }
}

fn fusion_stage_class(
    show_data_labels: bool,
    show_axes: bool,
    show_legend: bool,
    show_diagnostics: bool,
) -> String {
    let mut class = String::from("chart-stage fusion-stage");
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_axes {
        class.push_str(" hide-axes");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    if !show_diagnostics {
        class.push_str(" hide-fusion-diagnostics");
    }
    class
}
