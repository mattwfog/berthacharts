//! Interactive growth model demo composed from reusable line chart specs.

use std::sync::Arc;

use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};
use crate::dom_events::event_target_value_as_f32;

const W: u32 = 620;
const H: u32 = 330;

#[derive(Debug, Clone, Copy, PartialEq)]
struct GrowthInputs {
    months: u32,
    starting_customers: f32,
    monthly_leads: f32,
    lead_growth_rate: f32,
    conversion_rate: f32,
    sales_capacity: f32,
    churn_rate: f32,
    expansion_rate: f32,
    arpa: f32,
    arpa_growth_rate: f32,
    gross_margin: f32,
    cac: f32,
    starting_cash: f32,
    fixed_opex: f32,
    opex_growth_rate: f32,
    revenue_multiple: f32,
    planned_raise: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Scenario {
    name: &'static str,
    acquisition_multiplier: f32,
    churn_multiplier: f32,
    expansion_multiplier: f32,
    pricing_multiplier: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GrowthPoint {
    month: u32,
    customers: f32,
    new_customers: f32,
    churned_customers: f32,
    net_new_customers: f32,
    arpa: f32,
    mrr: f32,
    gross_profit: f32,
    acquisition_spend: f32,
    operating_expense: f32,
    ebitda: f32,
    ebitda_margin: f32,
    cash_balance: f32,
    runway_months: f32,
    rule_of_40: f32,
    enterprise_value: f32,
    burn_multiple: f32,
    ltv_cac: f32,
    payback_months: f32,
    nrr: f32,
    capacity_utilization: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScenarioSummary {
    scenario: Scenario,
    latest: GrowthPoint,
    previous: GrowthPoint,
}

const SCENARIOS: [Scenario; 3] = [
    Scenario {
        name: "Conservative",
        acquisition_multiplier: 0.82,
        churn_multiplier: 1.18,
        expansion_multiplier: 0.72,
        pricing_multiplier: 0.74,
    },
    Scenario {
        name: "Base",
        acquisition_multiplier: 1.0,
        churn_multiplier: 1.0,
        expansion_multiplier: 1.0,
        pricing_multiplier: 1.0,
    },
    Scenario {
        name: "Upside",
        acquisition_multiplier: 1.18,
        churn_multiplier: 0.82,
        expansion_multiplier: 1.28,
        pricing_multiplier: 1.22,
    },
];

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);
    let show_diagnostics = RwSignal::new(true);

    let months = RwSignal::new(24.0_f32);
    let starting_customers = RwSignal::new(180.0_f32);
    let monthly_leads = RwSignal::new(420.0_f32);
    let lead_growth_rate = RwSignal::new(2.5_f32);
    let conversion_rate = RwSignal::new(8.0_f32);
    let sales_capacity = RwSignal::new(64.0_f32);
    let churn_rate = RwSignal::new(2.8_f32);
    let expansion_rate = RwSignal::new(1.1_f32);
    let arpa = RwSignal::new(140.0_f32);
    let arpa_growth_rate = RwSignal::new(0.6_f32);
    let gross_margin = RwSignal::new(78.0_f32);
    let cac = RwSignal::new(860.0_f32);
    let starting_cash = RwSignal::new(650_000.0_f32);
    let fixed_opex = RwSignal::new(68_000.0_f32);
    let opex_growth_rate = RwSignal::new(1.4_f32);
    let revenue_multiple = RwSignal::new(7.0_f32);
    let planned_raise = RwSignal::new(1_200_000.0_f32);

    let inputs = move || GrowthInputs {
        months: months.get().round().clamp(6.0, 36.0) as u32,
        starting_customers: starting_customers.get(),
        monthly_leads: monthly_leads.get(),
        lead_growth_rate: lead_growth_rate.get() / 100.0,
        conversion_rate: conversion_rate.get() / 100.0,
        sales_capacity: sales_capacity.get(),
        churn_rate: churn_rate.get() / 100.0,
        expansion_rate: expansion_rate.get() / 100.0,
        arpa: arpa.get(),
        arpa_growth_rate: arpa_growth_rate.get() / 100.0,
        gross_margin: gross_margin.get() / 100.0,
        cac: cac.get(),
        starting_cash: starting_cash.get(),
        fixed_opex: fixed_opex.get(),
        opex_growth_rate: opex_growth_rate.get() / 100.0,
        revenue_multiple: revenue_multiple.get(),
        planned_raise: planned_raise.get(),
    };

    let base_latest = move || latest_summary(inputs(), SCENARIOS[1]);
    let base_projection = move || project_growth(inputs(), SCENARIOS[1]);
    let final_mrr_label = move || format_currency(base_latest().latest.mrr);
    let arr_label = move || format_currency(base_latest().latest.mrr * 12.0);
    let efficiency_label = move || format!("{:.1}x", base_latest().latest.ltv_cac);
    let runway_label = move || format_runway(base_latest().latest.runway_months);
    let rule_label = move || format!("{:.0}", base_latest().latest.rule_of_40);
    let valuation_label = move || format_currency(base_latest().latest.enterprise_value);
    let dilution_label = move || format!("{:.1}%", dilution(inputs(), SCENARIOS[1]) * 100.0);
    let break_even_label = move || format_break_even(base_projection());
    let month_growth_label = move || {
        let summary = base_latest();
        if summary.previous.mrr > 0.0 {
            format!(
                "{:+.1}%",
                ((summary.latest.mrr / summary.previous.mrr) - 1.0) * 100.0
            )
        } else {
            "+0.0%".to_string()
        }
    };

    let scenario_rows = move || {
        SCENARIOS
            .into_iter()
            .map(|scenario| scenario_row(inputs(), scenario))
            .collect_view()
    };
    let sensitivity_rows = move || sensitivity_rows(inputs()).into_iter().collect_view();

    let revenue_chart = move || {
        growth_chart_view(
            revenue_spec(inputs()),
            show_data_labels,
            show_axes,
            show_legend,
        )
    };
    let customer_chart = move || {
        growth_chart_view(
            customer_spec(inputs()),
            show_data_labels,
            show_axes,
            show_legend,
        )
    };
    let economics_chart = move || {
        growth_chart_view(
            economics_spec(inputs()),
            show_data_labels,
            show_axes,
            show_legend,
        )
    };

    view! {
        <section id="growth-model" class="example growth-model">
            <div class="example-head">
                <div>
                    <h2>"Growth Model"</h2>
                    <p>
                        "A live SaaS forecast with acquisition capacity, pricing expansion, retention, gross margin, CAC, and scenario diagnostics."
                    </p>
                </div>
                <div class="stat-strip growth-stat-strip">
                    <span><strong>{final_mrr_label}</strong>" exit MRR"</span>
                    <span><strong>{arr_label}</strong>" implied ARR"</span>
                    <span><strong>{month_growth_label}</strong>" last MoM"</span>
                    <span><strong>{efficiency_label}</strong>" LTV:CAC"</span>
                    <span><strong>{runway_label}</strong>" runway"</span>
                    <span><strong>{rule_label}</strong>" Rule 40"</span>
                </div>
            </div>

            <div class="growth-layout">
                <div class="growth-controls" aria-label="Growth model variables">
                    <div class="growth-control-group">
                        <h3>"Plan"</h3>
                        <GrowthSlider label="Months" min=6.0 max=36.0 step=1.0 value=months decimals=0 />
                        <GrowthSlider label="Starting customers" min=20.0 max=800.0 step=10.0 value=starting_customers decimals=0 />
                    </div>
                    <div class="growth-control-group">
                        <h3>"Acquisition"</h3>
                        <GrowthSlider label="Monthly leads" min=50.0 max=1500.0 step=10.0 value=monthly_leads decimals=0 />
                        <GrowthSlider label="Lead growth" suffix="%" min=-5.0 max=12.0 step=0.1 value=lead_growth_rate decimals=1 />
                        <GrowthSlider label="Conversion" suffix="%" min=1.0 max=24.0 step=0.1 value=conversion_rate decimals=1 />
                        <GrowthSlider label="Sales capacity" min=8.0 max=220.0 step=2.0 value=sales_capacity decimals=0 />
                    </div>
                    <div class="growth-control-group">
                        <h3>"Revenue"</h3>
                        <GrowthSlider label="ARPA" prefix="$" min=25.0 max=600.0 step=5.0 value=arpa decimals=0 />
                        <GrowthSlider label="ARPA growth" suffix="%" min=-2.0 max=5.0 step=0.1 value=arpa_growth_rate decimals=1 />
                        <GrowthSlider label="Expansion" suffix="%" min=0.0 max=6.0 step=0.1 value=expansion_rate decimals=1 />
                    </div>
                    <div class="growth-control-group">
                        <h3>"Efficiency"</h3>
                        <GrowthSlider label="Logo churn" suffix="%" min=0.0 max=12.0 step=0.1 value=churn_rate decimals=1 />
                        <GrowthSlider label="Gross margin" suffix="%" min=35.0 max=92.0 step=1.0 value=gross_margin decimals=0 />
                        <GrowthSlider label="CAC" prefix="$" min=100.0 max=4500.0 step=25.0 value=cac decimals=0 />
                    </div>
                    <div class="growth-control-group">
                        <h3>"Finance"</h3>
                        <GrowthSlider label="Cash balance" prefix="$" min=0.0 max=5000000.0 step=25000.0 value=starting_cash decimals=0 />
                        <GrowthSlider label="Monthly opex" prefix="$" min=5000.0 max=450000.0 step=2500.0 value=fixed_opex decimals=0 />
                        <GrowthSlider label="Opex growth" suffix="%" min=-3.0 max=8.0 step=0.1 value=opex_growth_rate decimals=1 />
                    </div>
                    <div class="growth-control-group">
                        <h3>"Capital"</h3>
                        <GrowthSlider label="ARR multiple" suffix="x" min=1.0 max=18.0 step=0.5 value=revenue_multiple decimals=1 />
                        <GrowthSlider label="Planned raise" prefix="$" min=0.0 max=8000000.0 step=50000.0 value=planned_raise decimals=0 />
                    </div>
                </div>

                <div class="growth-chart-column">
                    <DisplayControls label="Growth model display options">
                        <DisplayToggleButton label="Data labels" state=show_data_labels />
                        <DisplayToggleButton label="Axes" state=show_axes />
                        <DisplayToggleButton label="Legend" state=show_legend />
                        <DisplayToggleButton label="Diagnostics" state=show_diagnostics />
                    </DisplayControls>

                    <div class=move || diagnostics_class(show_diagnostics.get())>
                        <div class="growth-diagnostic-head">
                            <span>"Scenario"</span>
                            <span>"Exit MRR"</span>
                            <span>"Customers"</span>
                            <span>"Net adds"</span>
                            <span>"NRR"</span>
                            <span>"LTV:CAC"</span>
                            <span>"Runway"</span>
                            <span>"Rule 40"</span>
                            <span>"Valuation"</span>
                        </div>
                        {scenario_rows}
                    </div>

                    <div class="growth-operating-strip">
                        <OperatingMetric label="Cash" value=move || format_currency(base_latest().latest.cash_balance) detail=move || format!("EBITDA {}", format_currency(base_latest().latest.ebitda)) />
                        <OperatingMetric label="Spend" value=move || format_currency(base_latest().latest.acquisition_spend) detail=move || format!("{:.0}% capacity", base_latest().latest.capacity_utilization * 100.0) />
                        <OperatingMetric label="Margin" value=move || format!("{:.0}%", base_latest().latest.ebitda_margin * 100.0) detail=move || format!("Gross profit {}", format_currency(base_latest().latest.gross_profit)) />
                        <OperatingMetric label="Payback" value=move || format!("{:.1} mo", base_latest().latest.payback_months) detail=move || format!("ARPA {}", format_currency(base_latest().latest.arpa)) />
                        <OperatingMetric label="Valuation" value=valuation_label detail=move || format!("Dilution {}", dilution_label()) />
                        <OperatingMetric label="Break even" value=break_even_label detail=move || format!("Burn {:.1}x", base_latest().latest.burn_multiple) />
                    </div>

                    <div class="growth-sensitivity">
                        <div class="growth-sensitivity-head">
                            <span>"Lever"</span>
                            <span>"Exit ARR"</span>
                            <span>"Runway"</span>
                            <span>"Rule 40"</span>
                        </div>
                        {sensitivity_rows}
                    </div>

                    <div class="growth-chart-grid">
                        <div>
                            <h3>"Recurring revenue"</h3>
                            {revenue_chart}
                        </div>
                        <div>
                            <h3>"Active customers"</h3>
                            {customer_chart}
                        </div>
                        <div>
                            <h3>"Unit economics"</h3>
                            {economics_chart}
                        </div>
                        <div>
                            <h3>"Cash balance"</h3>
                            {move || growth_chart_view(
                                cash_spec(inputs()),
                                show_data_labels,
                                show_axes,
                                show_legend,
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn GrowthSlider(
    #[prop(into)] label: String,
    #[prop(default = String::new(), into)] prefix: String,
    #[prop(default = String::new(), into)] suffix: String,
    min: f32,
    max: f32,
    step: f32,
    value: RwSignal<f32>,
    decimals: usize,
) -> impl IntoView {
    let display_value = {
        let prefix = prefix.clone();
        let suffix = suffix.clone();
        move || {
            format!(
                "{}{}{}",
                prefix,
                format_fixed(value.get(), decimals),
                suffix
            )
        }
    };

    view! {
        <label class="growth-control">
            <span>
                <strong>{label}</strong>
                <em>{display_value}</em>
            </span>
            <input
                type="range"
                min=format!("{min}")
                max=format!("{max}")
                step=format!("{step}")
                prop:value=move || format!("{:.2}", value.get())
                on:input=move |ev| {
                    value.set(event_target_value_as_f32(&ev, value.get_untracked()).clamp(min, max));
                }
                on:change=move |ev| {
                    value.set(event_target_value_as_f32(&ev, value.get_untracked()).clamp(min, max));
                }
            />
        </label>
    }
}

#[component]
fn OperatingMetric(
    #[prop(into)] label: String,
    value: impl Fn() -> String + Copy + Send + Sync + 'static,
    detail: impl Fn() -> String + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="growth-operating-metric">
            <span>{label}</span>
            <strong>{value}</strong>
            <em>{detail}</em>
        </div>
    }
}

fn growth_chart_view(
    spec: LineChartSpec,
    show_data_labels: RwSignal<bool>,
    show_axes: RwSignal<bool>,
    show_legend: RwSignal<bool>,
) -> impl IntoView {
    let spec = Arc::new(spec);
    let build_spec = spec.clone();
    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("growth model spec should be valid")
    });

    view! {
        <div class=move || chart_stage_class(
            "chart-stage growth-chart-stage",
            show_data_labels.get(),
            show_axes.get(),
            show_legend.get(),
        )>
            <ChartCanvas width={W} height={H} builder={build} />
        </div>
    }
}

fn scenario_row(inputs: GrowthInputs, scenario: Scenario) -> impl IntoView {
    let summary = latest_summary(inputs, scenario);
    view! {
        <div class="growth-diagnostic-row">
            <strong>{scenario.name}</strong>
            <span>{format_currency(summary.latest.mrr)}</span>
            <span>{format!("{:.0}", summary.latest.customers)}</span>
            <span>{format!("{:+.0}", summary.latest.net_new_customers)}</span>
            <span>{format!("{:.0}%", summary.latest.nrr * 100.0)}</span>
            <span>{format!("{:.1}x", summary.latest.ltv_cac)}</span>
            <span>{format_runway(summary.latest.runway_months)}</span>
            <span>{format!("{:.0}", summary.latest.rule_of_40)}</span>
            <span>{format_currency(summary.latest.enterprise_value)}</span>
        </div>
    }
}

fn sensitivity_rows(inputs: GrowthInputs) -> Vec<impl IntoView> {
    let baseline = latest_summary(inputs, SCENARIOS[1]).latest;
    [
        (
            "Conversion +1pt",
            GrowthInputs {
                conversion_rate: (inputs.conversion_rate + 0.01).min(0.60),
                ..inputs
            },
        ),
        (
            "Churn -0.5pt",
            GrowthInputs {
                churn_rate: (inputs.churn_rate - 0.005).max(0.0),
                ..inputs
            },
        ),
        (
            "ARPA +$25",
            GrowthInputs {
                arpa: inputs.arpa + 25.0,
                ..inputs
            },
        ),
        (
            "CAC -$100",
            GrowthInputs {
                cac: (inputs.cac - 100.0).max(0.0),
                ..inputs
            },
        ),
        (
            "Opex -$5k",
            GrowthInputs {
                fixed_opex: (inputs.fixed_opex - 5_000.0).max(0.0),
                ..inputs
            },
        ),
        (
            "Leads +10%",
            GrowthInputs {
                monthly_leads: inputs.monthly_leads * 1.10,
                ..inputs
            },
        ),
    ]
    .into_iter()
    .map(|(label, adjusted)| {
        let latest = latest_summary(adjusted, SCENARIOS[1]).latest;
        let arr_delta = latest.mrr * 12.0 - baseline.mrr * 12.0;
        let runway_delta = latest.runway_months - baseline.runway_months;
        let rule_delta = latest.rule_of_40 - baseline.rule_of_40;
        view! {
            <div class="growth-sensitivity-row">
                <strong>{label}</strong>
                <span>{format_signed_currency(arr_delta)}</span>
                <span>{format_signed_months(runway_delta)}</span>
                <span>{format!("{:+.0}", rule_delta)}</span>
            </div>
        }
    })
    .collect()
}

fn latest_summary(inputs: GrowthInputs, scenario: Scenario) -> ScenarioSummary {
    let projection = project_growth(inputs, scenario);
    let latest = projection.last().copied().unwrap_or_else(|| empty_point(0));
    let previous = projection
        .iter()
        .rev()
        .nth(1)
        .copied()
        .unwrap_or_else(|| empty_point(0));
    ScenarioSummary {
        scenario,
        latest,
        previous,
    }
}

fn revenue_spec(inputs: GrowthInputs) -> LineChartSpec {
    let data = scenario_data(inputs, |point| point.mrr / 1000.0);
    let max_y = data.iter().map(|datum| datum.y).fold(0.0, f32::max);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "MRR ($k)".to_string(),
        x_domain: Some((0.0, inputs.months as f32)),
        y_domain: Some((0.0, nice_upper(max_y))),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn customer_spec(inputs: GrowthInputs) -> LineChartSpec {
    let data = scenario_data(inputs, |point| point.customers);
    let max_y = data.iter().map(|datum| datum.y).fold(0.0, f32::max);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "Active customers".to_string(),
        x_domain: Some((0.0, inputs.months as f32)),
        y_domain: Some((0.0, nice_upper(max_y))),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn economics_spec(inputs: GrowthInputs) -> LineChartSpec {
    let data = scenario_data(inputs, |point| point.ltv_cac);
    let max_y = data.iter().map(|datum| datum.y).fold(0.0, f32::max);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "LTV:CAC".to_string(),
        x_domain: Some((0.0, inputs.months as f32)),
        y_domain: Some((0.0, nice_upper(max_y.max(1.0)))),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn cash_spec(inputs: GrowthInputs) -> LineChartSpec {
    let data = scenario_data(inputs, |point| point.cash_balance / 1000.0);
    let max_y = data.iter().map(|datum| datum.y).fold(0.0, f32::max);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "Cash ($k)".to_string(),
        x_domain: Some((0.0, inputs.months as f32)),
        y_domain: Some((0.0, nice_upper(max_y.max(1.0)))),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn scenario_data(inputs: GrowthInputs, value: fn(GrowthPoint) -> f32) -> Vec<LineDatum> {
    SCENARIOS
        .into_iter()
        .flat_map(|scenario| {
            project_growth(inputs, scenario)
                .into_iter()
                .map(move |point| {
                    LineDatum::new(scenario.name, point.month as f32, value(point))
                        .with_label(format!("{} month {}", scenario.name, point.month))
                })
        })
        .collect()
}

fn project_growth(inputs: GrowthInputs, scenario: Scenario) -> Vec<GrowthPoint> {
    let mut points = Vec::with_capacity(inputs.months as usize + 1);
    let mut customers = inputs.starting_customers.max(0.0);
    let mut arpa = inputs.arpa.max(0.0);
    let mut mrr = customers * arpa;
    let mut cash_balance = inputs.starting_cash.max(0.0);
    let churn = (inputs.churn_rate * scenario.churn_multiplier).clamp(0.0, 0.95);
    let expansion = (inputs.expansion_rate * scenario.expansion_multiplier).max(0.0);
    let arpa_growth = inputs.arpa_growth_rate * scenario.pricing_multiplier;
    let nrr = (1.0 - churn + expansion + arpa_growth).max(0.0);

    points.push(point_for_month(
        0,
        customers,
        0.0,
        0.0,
        0.0,
        arpa,
        mrr,
        cash_balance,
        inputs,
        nrr,
        inputs.fixed_opex.max(0.0),
        0.0,
    ));

    for month in 1..=inputs.months {
        let previous_mrr = mrr;
        let lead_multiplier = (1.0 + inputs.lead_growth_rate).powi(month as i32 - 1);
        let leads =
            inputs.monthly_leads.max(0.0) * lead_multiplier * scenario.acquisition_multiplier;
        let sales_capacity = inputs.sales_capacity.max(0.0) * scenario.acquisition_multiplier;
        let demand = leads * inputs.conversion_rate.max(0.0);
        let new_customers = demand.min(sales_capacity);
        let capacity_utilization = if sales_capacity > 0.0 {
            (new_customers / sales_capacity).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let churned_customers = customers * churn;

        arpa = (arpa * (1.0 + arpa_growth)).max(0.0);
        customers = (customers - churned_customers + new_customers).max(0.0);
        mrr = (mrr * nrr + new_customers * arpa).max(0.0);
        let operating_expense =
            inputs.fixed_opex.max(0.0) * (1.0 + inputs.opex_growth_rate).powi(month as i32 - 1);
        let acquisition_spend = new_customers * inputs.cac.max(0.0);
        let gross_profit = mrr * inputs.gross_margin;
        let ebitda = gross_profit - operating_expense - acquisition_spend;
        cash_balance = (cash_balance + ebitda).max(0.0);

        points.push(
            point_for_month(
                month,
                customers,
                new_customers,
                churned_customers,
                acquisition_spend,
                arpa,
                mrr,
                cash_balance,
                inputs,
                nrr,
                operating_expense,
                rule_of_40(previous_mrr, mrr, ebitda),
            )
            .with_capacity_utilization(capacity_utilization),
        );
    }

    points
}

fn rule_of_40(previous_mrr: f32, mrr: f32, ebitda: f32) -> f32 {
    let annualized_growth = if previous_mrr > 0.0 && mrr > 0.0 {
        ((mrr / previous_mrr).powf(12.0) - 1.0) * 100.0
    } else {
        0.0
    };
    let margin = if mrr > 0.0 {
        (ebitda / mrr) * 100.0
    } else {
        0.0
    };
    annualized_growth + margin
}

fn dilution(inputs: GrowthInputs, scenario: Scenario) -> f32 {
    let latest = latest_summary(inputs, scenario).latest;
    let raise = inputs.planned_raise.max(0.0);
    let pre_money = latest.enterprise_value.max(0.0);
    if raise <= 0.0 || pre_money <= 0.0 {
        0.0
    } else {
        raise / (pre_money + raise)
    }
}

fn break_even_month(points: &[GrowthPoint]) -> Option<u32> {
    points
        .iter()
        .find(|point| point.month > 0 && point.ebitda >= 0.0)
        .map(|point| point.month)
}

fn runway_months(cash_balance: f32, ebitda: f32) -> f32 {
    if ebitda < 0.0 {
        (cash_balance / -ebitda).min(99.0)
    } else {
        99.0
    }
}

fn point_for_month(
    month: u32,
    customers: f32,
    new_customers: f32,
    churned_customers: f32,
    acquisition_spend: f32,
    arpa: f32,
    mrr: f32,
    cash_balance: f32,
    inputs: GrowthInputs,
    nrr: f32,
    operating_expense: f32,
    rule_of_40: f32,
) -> GrowthPoint {
    let gross_profit = mrr * inputs.gross_margin;
    let ebitda = gross_profit - operating_expense - acquisition_spend;
    let ebitda_margin = if mrr > 0.0 { ebitda / mrr } else { 0.0 };
    let gross_profit_per_customer = arpa * inputs.gross_margin;
    let ltv = if inputs.churn_rate > 0.0 {
        gross_profit_per_customer / inputs.churn_rate
    } else {
        gross_profit_per_customer * 60.0
    };
    let ltv_cac = if inputs.cac > 0.0 {
        ltv / inputs.cac
    } else {
        0.0
    };
    let payback_months = if gross_profit_per_customer > 0.0 {
        inputs.cac / gross_profit_per_customer
    } else {
        0.0
    };
    let new_arr = new_customers * arpa * 12.0;
    let burn_multiple = if ebitda < 0.0 && new_arr > 0.0 {
        -ebitda / new_arr
    } else {
        0.0
    };
    let sales_capacity = inputs.sales_capacity.max(0.0);
    let capacity_utilization = if sales_capacity > 0.0 {
        (new_customers / sales_capacity).clamp(0.0, 1.0)
    } else {
        0.0
    };

    GrowthPoint {
        month,
        customers,
        new_customers,
        churned_customers,
        net_new_customers: new_customers - churned_customers,
        arpa,
        mrr,
        gross_profit,
        acquisition_spend,
        operating_expense,
        ebitda,
        ebitda_margin,
        cash_balance,
        runway_months: runway_months(cash_balance, ebitda),
        rule_of_40,
        enterprise_value: mrr * 12.0 * inputs.revenue_multiple.max(0.0),
        burn_multiple,
        ltv_cac,
        payback_months,
        nrr,
        capacity_utilization,
    }
}

trait GrowthPointExt {
    fn with_capacity_utilization(self, capacity_utilization: f32) -> Self;
}

impl GrowthPointExt for GrowthPoint {
    fn with_capacity_utilization(mut self, capacity_utilization: f32) -> Self {
        self.capacity_utilization = capacity_utilization;
        self
    }
}

fn empty_point(month: u32) -> GrowthPoint {
    GrowthPoint {
        month,
        customers: 0.0,
        new_customers: 0.0,
        churned_customers: 0.0,
        net_new_customers: 0.0,
        arpa: 0.0,
        mrr: 0.0,
        gross_profit: 0.0,
        acquisition_spend: 0.0,
        operating_expense: 0.0,
        ebitda: 0.0,
        ebitda_margin: 0.0,
        cash_balance: 0.0,
        runway_months: 0.0,
        rule_of_40: 0.0,
        enterprise_value: 0.0,
        burn_multiple: 0.0,
        ltv_cac: 0.0,
        payback_months: 0.0,
        nrr: 0.0,
        capacity_utilization: 0.0,
    }
}

fn chart_stage_class(
    base: &'static str,
    show_data_labels: bool,
    show_axes: bool,
    show_legend: bool,
) -> String {
    let mut class = String::from(base);
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_axes {
        class.push_str(" hide-axes");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}

fn diagnostics_class(show_diagnostics: bool) -> &'static str {
    if show_diagnostics {
        "growth-diagnostics"
    } else {
        "growth-diagnostics hide-growth-diagnostics"
    }
}

fn nice_upper(value: f32) -> f32 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let magnitude = 10_f32.powf(value.log10().floor());
    (value * 1.12 / magnitude).ceil() * magnitude
}

fn format_currency(value: f32) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("${:.0}k", value / 1_000.0)
    } else {
        format!("${:.0}", value)
    }
}

fn format_signed_currency(value: f32) -> String {
    if value >= 0.0 {
        format!("+{}", format_currency(value))
    } else {
        format!("-{}", format_currency(value.abs()))
    }
}

fn format_runway(value: f32) -> String {
    if value >= 99.0 {
        "Profitable".to_string()
    } else {
        format!("{value:.1} mo")
    }
}

fn format_break_even(points: Vec<GrowthPoint>) -> String {
    break_even_month(&points)
        .map(|month| format!("M{month}"))
        .unwrap_or_else(|| "After plan".to_string())
}

fn format_signed_months(value: f32) -> String {
    if value.abs() >= 98.0 {
        "+Profitable".to_string()
    } else {
        format!("{value:+.1} mo")
    }
}

fn format_fixed(value: f32, decimals: usize) -> String {
    match decimals {
        0 => format!("{value:.0}"),
        1 => format!("{value:.1}"),
        2 => format!("{value:.2}"),
        _ => format!("{value:.3}"),
    }
}
