//! Comparable month-to-date sales dashboard demo.

use std::sync::Arc;

use berthacharts_charts::bar::{BarChartOptions, BarChartSpec, BarDatum};
use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use leptos::prelude::*;

use crate::chart_canvas::{chart_builder, ChartCanvas};
use crate::chart_chrome::{stage_class, DisplayControls, DisplayToggleButton};
use crate::gallery::{runtime_context, DataProfile};

const TREND_W: u32 = 720;
const TREND_H: u32 = 370;
const MIX_W: u32 = 520;
const MIX_H: u32 = 330;
const DRIVER_W: u32 = 520;
const DRIVER_H: u32 = 320;
const DAYS_ELAPSED: f32 = 14.0;
const DAYS_IN_MONTH: f32 = 30.0;
const MONTH_PLAN: f32 = 1_485.0;
const FULL_MONTH_PRIOR: f32 = 1_362.0;

const CHANNELS: [SalesSlice; 4] = [
    SalesSlice::new("Dine-in", 286.0, 263.0, 280.0),
    SalesSlice::new("Online", 198.0, 168.0, 184.0),
    SalesSlice::new("Catering", 122.0, 106.0, 126.0),
    SalesSlice::new("Delivery", 98.0, 91.0, 94.0),
];

const DAYPARTS: [SalesSlice; 4] = [
    SalesSlice::new("Breakfast", 96.0, 90.0, 92.0),
    SalesSlice::new("Lunch", 214.0, 196.0, 209.0),
    SalesSlice::new("Dinner", 332.0, 294.0, 320.0),
    SalesSlice::new("Late", 62.0, 48.0, 63.0),
];

const CATEGORIES: [SalesSlice; 4] = [
    SalesSlice::new("Core meals", 401.0, 366.0, 398.0),
    SalesSlice::new("Beverage", 118.0, 100.0, 110.0),
    SalesSlice::new("Add-ons", 94.0, 86.0, 90.0),
    SalesSlice::new("Catering menu", 91.0, 76.0, 86.0),
];

const OPERATING_DRIVERS: [OperatingDriver; 4] = [
    OperatingDriver::new("Transactions", 8420.0, 7860.0, 8290.0, DriverFormat::Count),
    OperatingDriver::new("Average check", 83.6, 79.9, 82.5, DriverFormat::Money),
    OperatingDriver::new("Items / check", 2.34, 2.22, 2.29, DriverFormat::Ratio),
    OperatingDriver::new("Promo mix", 11.8, 13.5, 12.4, DriverFormat::Percent),
];

const STORE_COHORTS: [StoreCohort; 4] = [
    StoreCohort::new("Core urban", 18, 318.0, 14.4, 8.0),
    StoreCohort::new("Suburban", 21, 246.0, 8.2, -3.0),
    StoreCohort::new("Travel", 6, 74.0, 19.1, 4.0),
    StoreCohort::new("Watchlist", 5, 66.0, -2.3, -9.0),
];

const FORECAST_SCENARIOS: [ForecastScenario; 3] = [
    ForecastScenario::new(
        "Downside",
        0.91,
        -18.0,
        "traffic fades, promo stays elevated",
    ),
    ForecastScenario::new("Base", 1.00, 0.0, "current daily pace holds"),
    ForecastScenario::new("Upside", 1.08, 24.0, "dinner and digital acceleration hold"),
];

const ECONOMICS: [EconomicsLine; 5] = [
    EconomicsLine::new("Gross sales", 755.0, 741.0),
    EconomicsLine::new("Discounts", -51.0, -57.0),
    EconomicsLine::new("Food cost", -204.0, -201.0),
    EconomicsLine::new("Labor", -176.0, -171.0),
    EconomicsLine::new("Contribution", 324.0, 312.0),
];

const ACTIONS: [ActionItem; 3] = [
    ActionItem::new(
        "Protect dinner pace",
        "Dinner is carrying 50% of comp lift; keep labor above model through day 21.",
    ),
    ActionItem::new(
        "Fix suburban gap",
        "Suburban cohort is positive comp but below plan; push lunch bundles and local CRM.",
    ),
    ActionItem::new(
        "Lean into digital",
        "Online mix gained share with the strongest channel comp; hold paid spend until CAC resets.",
    ),
];

const GROWTH_CHANNELS: [SalesSlice; 4] = [
    SalesSlice::new("New ARR", 412.0, 368.0, 398.0),
    SalesSlice::new("Expansion", 238.0, 205.0, 226.0),
    SalesSlice::new("Self-serve", 86.0, 72.0, 82.0),
    SalesSlice::new("Partners", 74.0, 64.0, 79.0),
];

const GROWTH_SEGMENTS: [SalesSlice; 4] = [
    SalesSlice::new("Enterprise", 352.0, 319.0, 340.0),
    SalesSlice::new("Mid-market", 248.0, 218.0, 236.0),
    SalesSlice::new("SMB", 126.0, 104.0, 119.0),
    SalesSlice::new("Startup", 84.0, 68.0, 90.0),
];

const GROWTH_PRODUCTS: [SalesSlice; 4] = [
    SalesSlice::new("Platform", 458.0, 411.0, 448.0),
    SalesSlice::new("AI add-on", 142.0, 94.0, 126.0),
    SalesSlice::new("Services", 118.0, 112.0, 121.0),
    SalesSlice::new("Marketplace", 92.0, 92.0, 90.0),
];

const GROWTH_OPERATING_DRIVERS: [OperatingDriver; 4] = [
    OperatingDriver::new(
        "Qualified opps",
        1420.0,
        1265.0,
        1360.0,
        DriverFormat::Count,
    ),
    OperatingDriver::new("ACV", 57.0, 53.5, 55.0, DriverFormat::Money),
    OperatingDriver::new("Win rate", 29.4, 27.1, 28.5, DriverFormat::Percent),
    OperatingDriver::new("Sales cycle", 1.08, 1.15, 1.10, DriverFormat::Ratio),
];

const GROWTH_COHORTS: [StoreCohort; 4] = [
    StoreCohort::new("Enterprise", 42, 352.0, 10.3, 12.0),
    StoreCohort::new("Mid-market", 118, 248.0, 13.8, 12.0),
    StoreCohort::new("SMB", 330, 126.0, 21.2, 7.0),
    StoreCohort::new("Watchlist", 28, 84.0, 23.5, -6.0),
];

const GROWTH_SCENARIOS: [ForecastScenario; 3] = [
    ForecastScenario::new("Downside", 0.90, -22.0, "late-stage conversion slips"),
    ForecastScenario::new("Base", 1.00, 0.0, "current pipeline pace holds"),
    ForecastScenario::new(
        "Upside",
        1.10,
        31.0,
        "expansion and self-serve close faster",
    ),
];

const GROWTH_ECONOMICS: [EconomicsLine; 5] = [
    EconomicsLine::new("Gross ARR", 858.0, 832.0),
    EconomicsLine::new("Credits", -24.0, -28.0),
    EconomicsLine::new("Cloud COGS", -98.0, -101.0),
    EconomicsLine::new("Sales cost", -188.0, -181.0),
    EconomicsLine::new("Contribution", 548.0, 522.0),
];

const GROWTH_ACTIONS: [ActionItem; 3] = [
    ActionItem::new(
        "Pull expansion forward",
        "Expansion has the cleanest gap to plan; prioritize renewal-adjacent attach offers.",
    ),
    ActionItem::new(
        "Protect enterprise ACV",
        "Enterprise is ahead, but discounting can erase the contribution advantage.",
    ),
    ActionItem::new(
        "Unblock self-serve",
        "Self-serve is outpacing prior year; keep onboarding and checkout experiments active.",
    ),
];

const OPS_CHANNELS: [SalesSlice; 4] = [
    SalesSlice::new("Resolved", 482.0, 438.0, 470.0),
    SalesSlice::new("Automated", 206.0, 158.0, 190.0),
    SalesSlice::new("Escalated", 92.0, 118.0, 96.0),
    SalesSlice::new("Backlog", 74.0, 86.0, 68.0),
];

const OPS_TIERS: [SalesSlice; 4] = [
    SalesSlice::new("Standard", 382.0, 350.0, 371.0),
    SalesSlice::new("Priority", 244.0, 222.0, 235.0),
    SalesSlice::new("Enterprise", 156.0, 142.0, 150.0),
    SalesSlice::new("Incident", 72.0, 86.0, 68.0),
];

const OPS_CATEGORIES: [SalesSlice; 4] = [
    SalesSlice::new("Billing", 228.0, 216.0, 222.0),
    SalesSlice::new("Access", 194.0, 176.0, 187.0),
    SalesSlice::new("Workflow", 256.0, 221.0, 243.0),
    SalesSlice::new("Reliability", 176.0, 187.0, 172.0),
];

const OPS_OPERATING_DRIVERS: [OperatingDriver; 4] = [
    OperatingDriver::new("Handled cases", 9240.0, 8710.0, 9025.0, DriverFormat::Count),
    OperatingDriver::new("Cost / case", 18.4, 19.2, 18.8, DriverFormat::Money),
    OperatingDriver::new("Touches / case", 1.82, 2.04, 1.90, DriverFormat::Ratio),
    OperatingDriver::new("Automation", 24.1, 18.2, 21.4, DriverFormat::Percent),
];

const OPS_COHORTS: [StoreCohort; 4] = [
    StoreCohort::new("Tier 1", 64, 382.0, 9.1, 11.0),
    StoreCohort::new("Tier 2", 38, 244.0, 9.9, 9.0),
    StoreCohort::new("Specialists", 14, 156.0, 9.9, 6.0),
    StoreCohort::new("Watchlist", 7, 72.0, -16.3, 4.0),
];

const OPS_SCENARIOS: [ForecastScenario; 3] = [
    ForecastScenario::new("Downside", 0.93, -14.0, "incident queue reopens"),
    ForecastScenario::new("Base", 1.00, 0.0, "current resolution pace holds"),
    ForecastScenario::new("Upside", 1.07, 18.0, "automation deflects more cases"),
];

const OPS_ECONOMICS: [EconomicsLine; 5] = [
    EconomicsLine::new("Capacity value", 854.0, 824.0),
    EconomicsLine::new("Rework", -62.0, -70.0),
    EconomicsLine::new("Tooling", -116.0, -112.0),
    EconomicsLine::new("Labor", -344.0, -336.0),
    EconomicsLine::new("Contribution", 332.0, 306.0),
];

const OPS_ACTIONS: [ActionItem; 3] = [
    ActionItem::new(
        "Hold automation gains",
        "Automation is the strongest positive mix shift; route repetitive access cases first.",
    ),
    ActionItem::new(
        "Watch incidents",
        "Incident work is below prior year but still over plan; protect escalation staffing.",
    ),
    ActionItem::new(
        "Reduce touches",
        "Touches per case are improving; keep macros and knowledge-base updates in lockstep.",
    ),
];

#[derive(Debug, Clone, Copy)]
struct CompSalesDataset {
    title: &'static str,
    description: &'static str,
    breakdown_titles: [&'static str; 3],
    mix_shift_label: &'static str,
    mix_focus_index: usize,
    cohort_unit: &'static str,
    days_elapsed: f32,
    days_in_month: f32,
    month_plan: f32,
    full_month_prior: f32,
    channels: [SalesSlice; 4],
    dayparts: [SalesSlice; 4],
    categories: [SalesSlice; 4],
    operating_drivers: [OperatingDriver; 4],
    store_cohorts: [StoreCohort; 4],
    forecast_scenarios: [ForecastScenario; 3],
    economics: [EconomicsLine; 5],
    actions: [ActionItem; 3],
}

fn comp_sales_dataset(profile: DataProfile) -> CompSalesDataset {
    match profile {
        DataProfile::Retail => CompSalesDataset {
            title: "Comp MTD Sales",
            description: "Comparable month-to-date sales pacing against prior year and plan, with mix, contribution, and operating breakdowns.",
            breakdown_titles: ["Channel", "Daypart", "Category"],
            mix_shift_label: "Digital mix shift",
            mix_focus_index: 1,
            cohort_unit: "stores",
            days_elapsed: DAYS_ELAPSED,
            days_in_month: DAYS_IN_MONTH,
            month_plan: MONTH_PLAN,
            full_month_prior: FULL_MONTH_PRIOR,
            channels: CHANNELS,
            dayparts: DAYPARTS,
            categories: CATEGORIES,
            operating_drivers: OPERATING_DRIVERS,
            store_cohorts: STORE_COHORTS,
            forecast_scenarios: FORECAST_SCENARIOS,
            economics: ECONOMICS,
            actions: ACTIONS,
        },
        DataProfile::Growth => CompSalesDataset {
            title: "Growth MTD Revenue",
            description: "Month-to-date revenue pacing across acquisition motions, segments, products, and contribution economics.",
            breakdown_titles: ["Motion", "Segment", "Product"],
            mix_shift_label: "Expansion mix shift",
            mix_focus_index: 1,
            cohort_unit: "accounts",
            days_elapsed: DAYS_ELAPSED,
            days_in_month: DAYS_IN_MONTH,
            month_plan: 1_690.0,
            full_month_prior: 1_548.0,
            channels: GROWTH_CHANNELS,
            dayparts: GROWTH_SEGMENTS,
            categories: GROWTH_PRODUCTS,
            operating_drivers: GROWTH_OPERATING_DRIVERS,
            store_cohorts: GROWTH_COHORTS,
            forecast_scenarios: GROWTH_SCENARIOS,
            economics: GROWTH_ECONOMICS,
            actions: GROWTH_ACTIONS,
        },
        DataProfile::Operations => CompSalesDataset {
            title: "Operations MTD Throughput",
            description: "Month-to-date operational throughput against prior period and plan, with queue, tier, cost, and cohort diagnostics.",
            breakdown_titles: ["Queue", "Tier", "Issue type"],
            mix_shift_label: "Automation mix shift",
            mix_focus_index: 1,
            cohort_unit: "teams",
            days_elapsed: DAYS_ELAPSED,
            days_in_month: DAYS_IN_MONTH,
            month_plan: 1_620.0,
            full_month_prior: 1_485.0,
            channels: OPS_CHANNELS,
            dayparts: OPS_TIERS,
            categories: OPS_CATEGORIES,
            operating_drivers: OPS_OPERATING_DRIVERS,
            store_cohorts: OPS_COHORTS,
            forecast_scenarios: OPS_SCENARIOS,
            economics: OPS_ECONOMICS,
            actions: OPS_ACTIONS,
        },
    }
}

#[component]
pub fn View() -> impl IntoView {
    let runtime = runtime_context();
    let dataset = comp_sales_dataset(runtime.data_profile);
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);
    let show_breakdowns = RwSignal::new(true);
    let selected_scenario = RwSignal::new(1_usize);

    let summary = SalesSummary::from_slices(&dataset.channels);
    let top_driver = top_comp_driver(&dataset.channels);
    let focus_slice = dataset.channels[dataset.mix_focus_index.min(dataset.channels.len() - 1)];
    let mix_shift = focus_slice.mix_shift(summary.mtd_sales, summary.prior_sales);

    let trend_spec = Arc::new(comp_mtd_trend_spec(dataset, summary));
    let primary_breakdown = dataset.breakdown_titles[0];
    let mix_chart_title = format!("Sales mix by {}", primary_breakdown.to_ascii_lowercase());
    let cohort_title = cohort_heading(dataset.cohort_unit);
    let cohort_count_title = cohort_count_heading(dataset.cohort_unit);
    let mix_spec = Arc::new(sales_mix_spec(&dataset.channels, primary_breakdown));
    let driver_spec = Arc::new(comp_driver_spec(&dataset.channels, primary_breakdown));

    let trend_build = chart_builder(trend_spec.clone(), TREND_W, TREND_H, "comp MTD trend spec");
    let mix_build = chart_builder(mix_spec.clone(), MIX_W, MIX_H, "sales mix spec");
    let driver_build = chart_builder(driver_spec.clone(), DRIVER_W, DRIVER_H, "comp driver spec");

    view! {
        <section id="comp-mtd-sales" class="example comp-sales">
            <div class="example-head">
                <div>
                    <h2>{dataset.title}</h2>
                    <p>{dataset.description}</p>
                </div>
                <div class="stat-strip">
                    <span><strong>{format_money_k(summary.mtd_sales)}</strong>" MTD sales"</span>
                    <span><strong>{format_pct(summary.comp_pct)}</strong>" comp"</span>
                    <span><strong>{format_signed_money_k(summary.comp_gap)}</strong>" vs LY"</span>
                    <span><strong>{format_signed_money_k(summary.plan_gap)}</strong>" vs plan"</span>
                </div>
            </div>

            <div class="comp-kpi-grid">
                {kpi_card("Pace projection", format_money_m(summary.forecast_sales(dataset)), format!("{} vs plan", format_signed_money_k(summary.forecast_plan_gap(dataset))))}
                {kpi_card("Daily average", format_money_k(summary.avg_daily_sales(dataset)), format!("{} needed", format_money_k(summary.required_run_rate(dataset))))}
                {kpi_card("Top comp driver", top_driver.name.to_string(), format!("{} vs LY", format_signed_money_k(top_driver.comp_gap())))}
                {kpi_card(dataset.mix_shift_label, format_signed_points(mix_shift), format!("{:.1}% of total", focus_slice.mix_pct(summary.mtd_sales)))}
            </div>

            <div class="comp-decision-grid">
                <div class="comp-scenario-panel">
                    <div class="comp-panel-head">
                        <h3>"Scenario lab"</h3>
                        <div class="comp-segments" aria-label="Forecast scenario">
                            {scenario_button(0, selected_scenario, dataset.forecast_scenarios)}
                            {scenario_button(1, selected_scenario, dataset.forecast_scenarios)}
                            {scenario_button(2, selected_scenario, dataset.forecast_scenarios)}
                        </div>
                    </div>
                    <div class="comp-scenario-output">
                        {scenario_metric(
                            "Month-end sales",
                            move || format_money_m(active_scenario(selected_scenario, dataset.forecast_scenarios).projected_sales(summary, dataset)),
                            move || format!("{} vs plan", format_signed_money_k(active_scenario(selected_scenario, dataset.forecast_scenarios).plan_gap(summary, dataset))),
                        )}
                        {scenario_metric(
                            "Full-month comp",
                            move || format_pct(active_scenario(selected_scenario, dataset.forecast_scenarios).comp_pct(summary, dataset)),
                            move || active_scenario(selected_scenario, dataset.forecast_scenarios).assumption.to_string(),
                        )}
                        {scenario_metric(
                            "Run-rate needed",
                            move || format_money_k(summary.required_run_rate(dataset)),
                            move || format!("{} remaining days", (dataset.days_in_month - dataset.days_elapsed) as u32),
                        )}
                    </div>
                </div>

                <div class="comp-economics-panel">
                    <h3>"Margin bridge"</h3>
                    <div class="comp-economics-head">
                        <span>"Line"</span>
                        <b>"Actual"</b>
                        <b>"Plan"</b>
                        <b>"Gap"</b>
                    </div>
                    {economics_row(dataset.economics[0])}
                    {economics_row(dataset.economics[1])}
                    {economics_row(dataset.economics[2])}
                    {economics_row(dataset.economics[3])}
                    {economics_row(dataset.economics[4])}
                </div>
            </div>

            <DisplayControls label="Comp MTD display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Axes" state=show_axes />
                <DisplayToggleButton label="Legend" state=show_legend />
                <DisplayToggleButton label="Breakdowns" state=show_breakdowns />
            </DisplayControls>

            <div class=move || stage_class("chart-stage comp-sales-stage", &[
                ("hide-data-labels", show_data_labels.get()),
                ("hide-axes", show_axes.get()),
                ("hide-legend", show_legend.get()),
                ("hide-breakdowns", show_breakdowns.get()),
            ])>
                <div class="chart-auto-grid comp-sales-grid">
                    <div class="chart-auto-item comp-sales-chart comp-sales-chart-large">
                        <h3>"MTD pace and forecast"</h3>
                        <ChartCanvas width={TREND_W} height={TREND_H} builder={trend_build.clone()} />
                    </div>
                    <div class="chart-auto-item comp-sales-chart">
                        <h3>{mix_chart_title}</h3>
                        <ChartCanvas width={MIX_W} height={MIX_H} builder={mix_build.clone()} />
                    </div>
                </div>

                <div class="chart-auto-grid comp-secondary-grid">
                    <div class="chart-auto-item comp-sales-chart">
                        <h3>"Comp contribution"</h3>
                        <ChartCanvas width={DRIVER_W} height={DRIVER_H} builder={driver_build.clone()} />
                    </div>
                    <div class="comp-variance-panel">
                        <h3>"Variance stack"</h3>
                        {variance_row("LY sales", format_money_k(summary.prior_sales), "baseline")}
                        {variance_row(dataset.channels[0].name, format_signed_money_k(dataset.channels[0].comp_gap()), "primary lift")}
                        {variance_row(dataset.channels[1].name, format_signed_money_k(dataset.channels[1].comp_gap()), "mix shift")}
                        {variance_row(dataset.channels[2].name, format_signed_money_k(dataset.channels[2].comp_gap()), "secondary driver")}
                        {variance_row(dataset.channels[3].name, format_signed_money_k(dataset.channels[3].comp_gap()), "watch item")}
                        {variance_row("Current sales", format_money_k(summary.mtd_sales), "actual")}
                    </div>
                </div>

                <div class="comp-breakdowns">
                    {breakdown_panel(dataset.breakdown_titles[0], dataset.channels, summary.mtd_sales, summary.prior_sales)}
                    {breakdown_panel(dataset.breakdown_titles[1], dataset.dayparts, summary.mtd_sales, summary.prior_sales)}
                    {breakdown_panel(dataset.breakdown_titles[2], dataset.categories, summary.mtd_sales, summary.prior_sales)}
                </div>

                <div class="comp-intelligence-grid">
                    <div class="comp-driver-panel">
                        <h3>"Operating drivers"</h3>
                        <div class="comp-driver-head">
                            <span>"Metric"</span>
                            <b>"Actual"</b>
                            <b>"Comp"</b>
                            <b>"Plan"</b>
                        </div>
                        {driver_row(dataset.operating_drivers[0])}
                        {driver_row(dataset.operating_drivers[1])}
                        {driver_row(dataset.operating_drivers[2])}
                        {driver_row(dataset.operating_drivers[3])}
                    </div>
                    <div class="comp-cohort-panel">
                        <h3>{cohort_title}</h3>
                        <div class="comp-cohort-head">
                            <span>"Cohort"</span>
                            <b>"Sales"</b>
                            <b>{cohort_count_title}</b>
                            <b>"Comp"</b>
                            <b>"Plan"</b>
                        </div>
                        {cohort_row(dataset.store_cohorts[0], dataset.cohort_unit)}
                        {cohort_row(dataset.store_cohorts[1], dataset.cohort_unit)}
                        {cohort_row(dataset.store_cohorts[2], dataset.cohort_unit)}
                        {cohort_row(dataset.store_cohorts[3], dataset.cohort_unit)}
                    </div>
                    <div class="comp-action-panel">
                        <h3>"Focus queue"</h3>
                        {action_item(dataset.actions[0])}
                        {action_item(dataset.actions[1])}
                        {action_item(dataset.actions[2])}
                    </div>
                </div>
            </div>
        </section>
    }
}

#[derive(Debug, Clone, Copy)]
struct SalesSlice {
    name: &'static str,
    current: f32,
    prior: f32,
    plan: f32,
}

impl SalesSlice {
    const fn new(name: &'static str, current: f32, prior: f32, plan: f32) -> Self {
        Self {
            name,
            current,
            prior,
            plan,
        }
    }

    fn comp_gap(self) -> f32 {
        self.current - self.prior
    }

    fn plan_gap(self) -> f32 {
        self.current - self.plan
    }

    fn comp_pct(self) -> f32 {
        pct_delta(self.current, self.prior)
    }

    fn mix_pct(self, total: f32) -> f32 {
        pct_of(self.current, total)
    }

    fn mix_shift(self, current_total: f32, prior_total: f32) -> f32 {
        self.mix_pct(current_total) - pct_of(self.prior, prior_total)
    }
}

#[derive(Debug, Clone, Copy)]
struct OperatingDriver {
    name: &'static str,
    current: f32,
    prior: f32,
    plan: f32,
    format: DriverFormat,
}

impl OperatingDriver {
    const fn new(
        name: &'static str,
        current: f32,
        prior: f32,
        plan: f32,
        format: DriverFormat,
    ) -> Self {
        Self {
            name,
            current,
            prior,
            plan,
            format,
        }
    }

    fn comp_pct(self) -> f32 {
        pct_delta(self.current, self.prior)
    }

    fn plan_gap_pct(self) -> f32 {
        pct_delta(self.current, self.plan)
    }
}

#[derive(Debug, Clone, Copy)]
enum DriverFormat {
    Count,
    Money,
    Percent,
    Ratio,
}

#[derive(Debug, Clone, Copy)]
struct StoreCohort {
    name: &'static str,
    stores: u32,
    sales: f32,
    comp_pct: f32,
    plan_gap: f32,
}

impl StoreCohort {
    const fn new(
        name: &'static str,
        stores: u32,
        sales: f32,
        comp_pct: f32,
        plan_gap: f32,
    ) -> Self {
        Self {
            name,
            stores,
            sales,
            comp_pct,
            plan_gap,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ForecastScenario {
    name: &'static str,
    run_rate_multiplier: f32,
    incremental_sales: f32,
    assumption: &'static str,
}

impl ForecastScenario {
    const fn new(
        name: &'static str,
        run_rate_multiplier: f32,
        incremental_sales: f32,
        assumption: &'static str,
    ) -> Self {
        Self {
            name,
            run_rate_multiplier,
            incremental_sales,
            assumption,
        }
    }

    fn projected_sales(self, summary: SalesSummary, dataset: CompSalesDataset) -> f32 {
        let remaining_sales = summary.avg_daily_sales(dataset)
            * (dataset.days_in_month - dataset.days_elapsed)
            * self.run_rate_multiplier;
        summary.mtd_sales + remaining_sales + self.incremental_sales
    }

    fn plan_gap(self, summary: SalesSummary, dataset: CompSalesDataset) -> f32 {
        self.projected_sales(summary, dataset) - dataset.month_plan
    }

    fn comp_pct(self, summary: SalesSummary, dataset: CompSalesDataset) -> f32 {
        pct_delta(
            self.projected_sales(summary, dataset),
            dataset.full_month_prior,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct EconomicsLine {
    name: &'static str,
    actual: f32,
    plan: f32,
}

impl EconomicsLine {
    const fn new(name: &'static str, actual: f32, plan: f32) -> Self {
        Self { name, actual, plan }
    }

    fn gap(self) -> f32 {
        self.actual - self.plan
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionItem {
    title: &'static str,
    detail: &'static str,
}

impl ActionItem {
    const fn new(title: &'static str, detail: &'static str) -> Self {
        Self { title, detail }
    }
}

#[derive(Debug, Clone, Copy)]
struct SalesSummary {
    mtd_sales: f32,
    prior_sales: f32,
    plan_sales: f32,
    comp_pct: f32,
    comp_gap: f32,
    plan_gap: f32,
}

impl SalesSummary {
    fn from_slices(slices: &[SalesSlice]) -> Self {
        let mtd_sales = slices.iter().map(|slice| slice.current).sum::<f32>();
        let prior_sales = slices.iter().map(|slice| slice.prior).sum::<f32>();
        let plan_sales = slices.iter().map(|slice| slice.plan).sum::<f32>();

        Self {
            mtd_sales,
            prior_sales,
            plan_sales,
            comp_pct: pct_delta(mtd_sales, prior_sales),
            comp_gap: mtd_sales - prior_sales,
            plan_gap: mtd_sales - plan_sales,
        }
    }

    fn avg_daily_sales(self, dataset: CompSalesDataset) -> f32 {
        self.mtd_sales / dataset.days_elapsed
    }

    fn forecast_sales(self, dataset: CompSalesDataset) -> f32 {
        self.avg_daily_sales(dataset) * dataset.days_in_month
    }

    fn forecast_plan_gap(self, dataset: CompSalesDataset) -> f32 {
        self.forecast_sales(dataset) - dataset.month_plan
    }

    fn required_run_rate(self, dataset: CompSalesDataset) -> f32 {
        (dataset.month_plan - self.mtd_sales).max(0.0)
            / (dataset.days_in_month - dataset.days_elapsed)
    }
}

fn comp_mtd_trend_spec(dataset: CompSalesDataset, summary: SalesSummary) -> LineChartSpec {
    let forecast = summary.forecast_sales(dataset);
    let options = LineChartOptions {
        x_axis_label: "Month day".to_string(),
        y_axis_label: "Cumulative sales ($k)".to_string(),
        x_domain: Some((1.0, dataset.days_in_month)),
        y_domain: Some((0.0, dataset.month_plan.max(forecast) * 1.08)),
        x_tick_count: 6,
        y_tick_count: 5,
        ..LineChartOptions::default()
    };

    let mut points = Vec::new();
    extend_pace_points(&mut points, "This year", summary.mtd_sales, None);
    extend_pace_points(
        &mut points,
        "Last year",
        summary.prior_sales,
        Some((dataset.days_in_month, dataset.full_month_prior)),
    );
    extend_pace_points(
        &mut points,
        "Plan",
        summary.plan_sales,
        Some((dataset.days_in_month, dataset.month_plan)),
    );
    points.extend([
        LineDatum::new("Run-rate forecast", 14.0, summary.mtd_sales)
            .with_label("Current MTD sales"),
        LineDatum::new(
            "Run-rate forecast",
            21.0,
            forecast * 21.0 / dataset.days_in_month,
        )
        .with_label("Day 21 run-rate forecast"),
        LineDatum::new("Run-rate forecast", dataset.days_in_month, forecast)
            .with_label("Day 30 run-rate forecast"),
    ]);

    LineChartSpec::new(points).with_options(options)
}

fn sales_mix_spec(slices: &[SalesSlice], x_axis_label: &str) -> BarChartSpec {
    let options = BarChartOptions {
        x_axis_label: x_axis_label.to_string(),
        y_axis_label: "Sales ($k)".to_string(),
        y_tick_count: 5,
        max_visible_labels: Some(slices.len()),
        ..BarChartOptions::default()
    };

    BarChartSpec::new(
        slices
            .iter()
            .map(|slice| BarDatum::new(slice.name, slice.current))
            .collect(),
    )
    .with_options(options)
}

fn comp_driver_spec(slices: &[SalesSlice], x_axis_label: &str) -> BarChartSpec {
    let options = BarChartOptions {
        x_axis_label: x_axis_label.to_string(),
        y_axis_label: "Comp lift ($k)".to_string(),
        y_tick_count: 4,
        max_visible_labels: Some(slices.len()),
        ..BarChartOptions::default()
    };

    BarChartSpec::new(
        slices
            .iter()
            .map(|slice| BarDatum::new(slice.name, slice.comp_gap().max(0.0)))
            .collect(),
    )
    .with_options(options)
}

fn extend_pace_points(
    points: &mut Vec<LineDatum>,
    series: &'static str,
    day_14_total: f32,
    full_month_total: Option<(f32, f32)>,
) {
    const CURVE_TO_DAY_14: [f32; 14] = [
        0.064, 0.132, 0.196, 0.266, 0.340, 0.413, 0.476, 0.545, 0.615, 0.690, 0.763, 0.840, 0.920,
        1.000,
    ];

    for (index, pct) in CURVE_TO_DAY_14.iter().enumerate() {
        let day = (index + 1) as f32;
        points.push(
            LineDatum::new(series, day, day_14_total * pct).with_label(format!(
                "Day {} {}",
                index + 1,
                series.to_ascii_lowercase()
            )),
        );
    }

    if let Some((last_day, full_month_total)) = full_month_total {
        points.push(
            LineDatum::new(series, 21.0, full_month_total * 0.703)
                .with_label(format!("Day 21 {}", series.to_ascii_lowercase())),
        );
        points.push(
            LineDatum::new(series, last_day, full_month_total).with_label(format!(
                "Day {:.0} {}",
                last_day,
                series.to_ascii_lowercase()
            )),
        );
    }
}

fn kpi_card(label: &'static str, value: String, detail: String) -> impl IntoView {
    view! {
        <div class="comp-kpi">
            <span>{label}</span>
            <strong>{value}</strong>
            <em>{detail}</em>
        </div>
    }
}

fn scenario_button(
    index: usize,
    selected_scenario: RwSignal<usize>,
    scenarios: [ForecastScenario; 3],
) -> impl IntoView {
    let scenario = scenarios[index];
    view! {
        <button
            type="button"
            class=move || if selected_scenario.get() == index {
                "comp-segment is-active"
            } else {
                "comp-segment"
            }
            aria-pressed=move || (selected_scenario.get() == index).to_string()
            on:click=move |_| selected_scenario.set(index)
        >
            {scenario.name}
        </button>
    }
}

fn scenario_metric(
    label: &'static str,
    value: impl Fn() -> String + Copy + Send + 'static,
    detail: impl Fn() -> String + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <div class="comp-scenario-metric">
            <span>{label}</span>
            <strong>{value}</strong>
            <em>{detail}</em>
        </div>
    }
}

fn variance_row(label: &'static str, value: String, detail: &'static str) -> impl IntoView {
    view! {
        <div class="comp-variance-row">
            <span>{label}</span>
            <strong>{value}</strong>
            <em>{detail}</em>
        </div>
    }
}

fn economics_row(line: EconomicsLine) -> impl IntoView {
    view! {
        <div class="comp-economics-row">
            <span>{line.name}</span>
            <strong>{format_signed_money_k(line.actual)}</strong>
            <em>{format_signed_money_k(line.plan)}</em>
            <b>{format_signed_money_k(line.gap())}</b>
        </div>
    }
}

fn breakdown_panel(
    title: &'static str,
    slices: [SalesSlice; 4],
    current_total: f32,
    prior_total: f32,
) -> impl IntoView {
    view! {
        <div class="comp-breakdown-panel">
            <h3>{title}</h3>
            <div class="comp-slice-head">
                <span>"Slice"</span>
                <b>"Sales"</b>
                <b>"Comp"</b>
                <b>"Mix"</b>
                <b>"Plan"</b>
            </div>
            {slice_row(slices[0], current_total, prior_total)}
            {slice_row(slices[1], current_total, prior_total)}
            {slice_row(slices[2], current_total, prior_total)}
            {slice_row(slices[3], current_total, prior_total)}
        </div>
    }
}

fn slice_row(slice: SalesSlice, current_total: f32, prior_total: f32) -> impl IntoView {
    view! {
        <div class="comp-slice-row">
            <span>{slice.name}</span>
            <strong>{format_money_k(slice.current)}</strong>
            <em>{format_pct(slice.comp_pct())}</em>
            <i>{format_signed_points(slice.mix_shift(current_total, prior_total))}</i>
            <b>{format_signed_money_k(slice.plan_gap())}</b>
        </div>
    }
}

fn driver_row(driver: OperatingDriver) -> impl IntoView {
    view! {
        <div class="comp-driver-row">
            <span>{driver.name}</span>
            <strong>{format_driver_value(driver.current, driver.format)}</strong>
            <em>{format_pct(driver.comp_pct())}</em>
            <i>{format_pct(driver.plan_gap_pct())}</i>
        </div>
    }
}

fn cohort_row(cohort: StoreCohort, unit: &'static str) -> impl IntoView {
    view! {
        <div class="comp-cohort-row">
            <span>{cohort.name}</span>
            <strong>{format_money_k(cohort.sales)}</strong>
            <em>{format!("{} {}", cohort.stores, unit)}</em>
            <i>{format_pct(cohort.comp_pct)}</i>
            <b>{format_signed_money_k(cohort.plan_gap)}</b>
        </div>
    }
}

fn action_item(item: ActionItem) -> impl IntoView {
    view! {
        <article class="comp-action-item">
            <strong>{item.title}</strong>
            <em>{item.detail}</em>
        </article>
    }
}

fn top_comp_driver(slices: &[SalesSlice]) -> SalesSlice {
    slices
        .iter()
        .copied()
        .max_by(|left, right| left.comp_gap().total_cmp(&right.comp_gap()))
        .unwrap_or(SalesSlice::new("None", 0.0, 0.0, 0.0))
}

fn active_scenario(
    selected_scenario: RwSignal<usize>,
    scenarios: [ForecastScenario; 3],
) -> ForecastScenario {
    scenarios[selected_scenario
        .get()
        .min(scenarios.len().saturating_sub(1))]
}

fn cohort_heading(unit: &str) -> &'static str {
    match unit {
        "accounts" => "Account cohorts",
        "teams" => "Team cohorts",
        _ => "Store cohorts",
    }
}

fn cohort_count_heading(unit: &str) -> &'static str {
    match unit {
        "accounts" => "Accounts",
        "teams" => "Teams",
        _ => "Stores",
    }
}

fn pct_delta(current: f32, previous: f32) -> f32 {
    if previous.abs() < f32::EPSILON {
        0.0
    } else {
        (current - previous) / previous * 100.0
    }
}

fn pct_of(value: f32, total: f32) -> f32 {
    if total.abs() < f32::EPSILON {
        0.0
    } else {
        value / total * 100.0
    }
}

fn format_driver_value(value: f32, format: DriverFormat) -> String {
    match format {
        DriverFormat::Count => format!("{value:.0}"),
        DriverFormat::Money => format!("${value:.1}"),
        DriverFormat::Percent => format!("{value:.1}%"),
        DriverFormat::Ratio => format!("{value:.2}x"),
    }
}

fn format_money_k(value: f32) -> String {
    if value >= 1_000.0 {
        format_money_m(value)
    } else if value >= 0.0 {
        format!("${value:.0}k")
    } else {
        format!("-${:.0}k", value.abs())
    }
}

fn format_money_m(value: f32) -> String {
    if value >= 0.0 {
        format!("${:.2}m", value / 1_000.0)
    } else {
        format!("-${:.2}m", value.abs() / 1_000.0)
    }
}

fn format_signed_money_k(value: f32) -> String {
    if value >= 0.0 {
        format!("+${value:.0}k")
    } else {
        format!("-${:.0}k", value.abs())
    }
}

fn format_pct(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.1}%")
    } else {
        format!("{value:.1}%")
    }
}

fn format_signed_points(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.1} pts")
    } else {
        format!("{value:.1} pts")
    }
}
