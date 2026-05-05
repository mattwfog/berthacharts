//! Interactive state space model suite for filtered states, forecasts, and diagnostics.

use std::sync::Arc;

use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};
use crate::dom_events::event_target_value_as_f32;

const W: u32 = 620;
const H: u32 = 320;
const FORECAST_STEPS: u32 = 12;
const MODELS: [ModelKind; 4] = [
    ModelKind::LocalLevel,
    ModelKind::LocalTrend,
    ModelKind::SeasonalTrend,
    ModelKind::RegressionSeasonal,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelKind {
    LocalLevel,
    LocalTrend,
    SeasonalTrend,
    RegressionSeasonal,
}

impl ModelKind {
    const fn label(self) -> &'static str {
        match self {
            Self::LocalLevel => "Local level",
            Self::LocalTrend => "Local trend",
            Self::SeasonalTrend => "Seasonal trend",
            Self::RegressionSeasonal => "Dynamic regression",
        }
    }

    const fn short_label(self) -> &'static str {
        match self {
            Self::LocalLevel => "Level",
            Self::LocalTrend => "Trend",
            Self::SeasonalTrend => "Seasonal",
            Self::RegressionSeasonal => "Regression",
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::LocalLevel => "local-level",
            Self::LocalTrend => "local-trend",
            Self::SeasonalTrend => "seasonal-trend",
            Self::RegressionSeasonal => "regression-seasonal",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::LocalLevel => "A one-state baseline that only tracks the current level.",
            Self::LocalTrend => "Adds a slope state so the estimate can carry momentum.",
            Self::SeasonalTrend => "Adds monthly seasonal states around a local trend.",
            Self::RegressionSeasonal => {
                "Adds a time-varying campaign response on top of seasonality."
            }
        }
    }

    const fn has_trend(self) -> bool {
        !matches!(self, Self::LocalLevel)
    }

    const fn has_seasonality(self) -> bool {
        matches!(self, Self::SeasonalTrend | Self::RegressionSeasonal)
    }

    const fn has_regressor(self) -> bool {
        matches!(self, Self::RegressionSeasonal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SsmInputs {
    months: u32,
    process_noise: f32,
    observation_noise: f32,
    trend_flex: f32,
    seasonal_strength: f32,
    regressor_strength: f32,
    shock_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ObservationPoint {
    month: u32,
    observed: f32,
    latent: f32,
    regressor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StatePoint {
    month: u32,
    observed: f32,
    latent: f32,
    filtered: f32,
    residual: f32,
    level: f32,
    trend: f32,
    seasonal: f32,
    regression: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ForecastPoint {
    month: u32,
    forecast: f32,
    lower: f32,
    upper: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct ModelRun {
    kind: ModelKind,
    points: Vec<StatePoint>,
    forecast: Vec<ForecastPoint>,
    mae: f32,
    rmse: f32,
    bias: f32,
    forecast_end: f32,
    interval_width: f32,
    stability: f32,
    responsiveness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RelationshipNode {
    label: &'static str,
    detail: &'static str,
    x: f32,
    y: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RelationshipEdge {
    label: &'static str,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    lx: f32,
    ly: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct PhaseCard {
    marker_id: &'static str,
    title: &'static str,
    description: &'static str,
    x_label: &'static str,
    y_label: &'static str,
    threshold_x: f32,
    threshold_y: f32,
    marker_x: f32,
    marker_y: f32,
    paths: Vec<PhasePath>,
    arrows: Vec<PhaseArrow>,
    metrics: Vec<PhaseMetric>,
}

#[derive(Debug, Clone, PartialEq)]
struct PhasePath {
    d: String,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PhaseArrow {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct PhaseMetric {
    value: String,
    label: &'static str,
    detail: &'static str,
}

#[component]
pub fn View() -> impl IntoView {
    let selected = RwSignal::new(ModelKind::RegressionSeasonal);
    let show_data_labels = RwSignal::new(false);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let months = RwSignal::new(48.0_f32);
    let process_noise = RwSignal::new(34.0_f32);
    let observation_noise = RwSignal::new(22.0_f32);
    let trend_flex = RwSignal::new(46.0_f32);
    let seasonal_strength = RwSignal::new(58.0_f32);
    let regressor_strength = RwSignal::new(50.0_f32);
    let shock_size = RwSignal::new(-18.0_f32);

    let inputs = move || SsmInputs {
        months: months.get().round().clamp(24.0, 72.0) as u32,
        process_noise: process_noise.get() / 100.0,
        observation_noise: observation_noise.get() / 100.0,
        trend_flex: trend_flex.get() / 100.0,
        seasonal_strength: seasonal_strength.get() / 100.0,
        regressor_strength: regressor_strength.get() / 100.0,
        shock_size: shock_size.get(),
    };

    let selected_run = move || fit_model(inputs(), selected.get());
    let best_run = move || best_model(inputs());
    let final_forecast = move || format!("{:.1}", selected_run().forecast_end);
    let best_label = move || best_run().kind.label();
    let mae_label = move || format!("{:.2}", selected_run().mae);
    let width_label = move || format!("{:.1}", selected_run().interval_width);
    let bias_label = move || format_signed(selected_run().bias, 2);

    view! {
        <section id="state-space-models" class="example state-space-models">
            <div class="example-head">
                <div>
                    <h2>"State Space Model Suite"</h2>
                    <p>
                        "A live suite of local level, local trend, seasonal, and dynamic regression state-space models with filtered states, forecast intervals, residuals, and fit diagnostics."
                    </p>
                </div>
                <div class="stat-strip ssm-stat-strip">
                    <span><strong>{best_label}</strong>" best fit"</span>
                    <span><strong>{mae_label}</strong>" selected MAE"</span>
                    <span><strong>{bias_label}</strong>" residual bias"</span>
                    <span><strong>{final_forecast}</strong>" final forecast"</span>
                    <span><strong>{width_label}</strong>" interval width"</span>
                </div>
            </div>

            <div class="ssm-layout">
                <aside class="ssm-controls" aria-label="State space model controls">
                    <div class="ssm-control-head">
                        <h3>"Model Family"</h3>
                        <span>"Switch between state specifications and compare how each absorbs noise, trend, seasonality, and intervention effects."</span>
                    </div>
                    <div class="ssm-model-tabs">
                        {MODELS.into_iter().map(|kind| {
                            view! {
                                <button
                                    type="button"
                                    class=move || if selected.get() == kind { "ssm-model-tab is-active" } else { "ssm-model-tab" }
                                    on:click=move |_| selected.set(kind)
                                >
                                    <strong>{kind.label()}</strong>
                                    <span>{kind.description()}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>

                    <div class="ssm-control-group">
                        <h3>"Signal"</h3>
                        <SsmSlider label="Months" min=24.0 max=72.0 step=1.0 value=months decimals=0 />
                        <SsmSlider label="Process noise" suffix="%" min=0.0 max=100.0 step=1.0 value=process_noise decimals=0 />
                        <SsmSlider label="Observation noise" suffix="%" min=0.0 max=100.0 step=1.0 value=observation_noise decimals=0 />
                    </div>
                    <div class="ssm-control-group">
                        <h3>"State Dynamics"</h3>
                        <SsmSlider label="Trend flexibility" suffix="%" min=0.0 max=100.0 step=1.0 value=trend_flex decimals=0 />
                        <SsmSlider label="Seasonality" suffix="%" min=0.0 max=100.0 step=1.0 value=seasonal_strength decimals=0 />
                        <SsmSlider label="Regressor lift" suffix="%" min=0.0 max=100.0 step=1.0 value=regressor_strength decimals=0 />
                        <SsmSlider label="Structural shock" min=-40.0 max=40.0 step=1.0 value=shock_size decimals=0 />
                    </div>
                </aside>

                <div class="ssm-main">
                    <DisplayControls label="State space display options">
                        <DisplayToggleButton label="Data labels" state=show_data_labels />
                        <DisplayToggleButton label="Axes" state=show_axes />
                        <DisplayToggleButton label="Legend" state=show_legend />
                    </DisplayControls>

                    <div class="ssm-diagnostics">
                        <div class="ssm-diagnostic-head">
                            <span>"Model"</span>
                            <span>"MAE"</span>
                            <span>"RMSE"</span>
                            <span>"Bias"</span>
                            <span>"Final"</span>
                            <span>"Width"</span>
                        </div>
                        {move || model_rows(inputs(), selected.get()).into_iter().collect_view()}
                    </div>

                    <div class="ssm-state-grid">
                        {move || state_cells(selected_run()).into_iter().collect_view()}
                    </div>

                    <div class="ssm-relationship-grid">
                        {move || relationship_graphs(selected.get()).into_iter().collect_view()}
                    </div>

                    <div class="ssm-phase-grid">
                        {move || phase_portrait_cards(selected_run(), inputs()).into_iter().collect_view()}
                    </div>

                    <div class="ssm-chart-grid">
                        <div class="ssm-chart-wide">
                            <h3>"Filtered state and forecast interval"</h3>
                            {move || ssm_chart_view(
                                selected_spec(selected_run()),
                                show_data_labels,
                                show_axes,
                                show_legend,
                            )}
                        </div>
                        <div>
                            <h3>"Residual signature"</h3>
                            {move || ssm_chart_view(
                                residual_spec(inputs()),
                                show_data_labels,
                                show_axes,
                                show_legend,
                            )}
                        </div>
                        <div>
                            <h3>"Model forecast comparison"</h3>
                            {move || ssm_chart_view(
                                comparison_spec(inputs()),
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
fn SsmSlider(
    #[prop(into)] label: String,
    #[prop(default = String::new(), into)] suffix: String,
    min: f32,
    max: f32,
    step: f32,
    value: RwSignal<f32>,
    decimals: usize,
) -> impl IntoView {
    let display_value = {
        let suffix = suffix.clone();
        move || format!("{}{}", format_fixed(value.get(), decimals), suffix)
    };

    view! {
        <label class="ssm-control">
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

fn ssm_chart_view(
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
            .expect("state space chart spec should be valid")
    });

    view! {
        <div class=move || chart_stage_class(
            "chart-stage ssm-chart-stage",
            show_data_labels.get(),
            show_axes.get(),
            show_legend.get(),
        )>
            <ChartCanvas width={W} height={H} builder={build} />
        </div>
    }
}

fn model_rows(inputs: SsmInputs, selected: ModelKind) -> Vec<impl IntoView> {
    MODELS
        .into_iter()
        .map(|kind| {
            let run = fit_model(inputs, kind);
            view! {
                <div class=if kind == selected { "ssm-diagnostic-row is-selected" } else { "ssm-diagnostic-row" }>
                    <strong>{kind.short_label()}</strong>
                    <span>{format!("{:.2}", run.mae)}</span>
                    <span>{format!("{:.2}", run.rmse)}</span>
                    <span>{format_signed(run.bias, 2)}</span>
                    <span>{format!("{:.1}", run.forecast_end)}</span>
                    <span>{format!("{:.1}", run.interval_width)}</span>
                </div>
            }
        })
        .collect()
}

fn state_cells(run: ModelRun) -> Vec<impl IntoView> {
    let latest = run.points.last().copied().unwrap_or_else(empty_state_point);
    [
        (
            "Level",
            latest.level,
            run.stability,
            "Filtered baseline state after the latest observation.",
        ),
        (
            "Trend",
            latest.trend,
            run.responsiveness,
            "Slope carried into the forecast horizon.",
        ),
        (
            "Seasonal",
            latest.seasonal,
            run.kind.has_seasonality().then_some(run.stability).unwrap_or(0.0),
            "Current seasonal state contribution.",
        ),
        (
            "Regressor",
            latest.regression,
            run.kind.has_regressor().then_some(run.responsiveness).unwrap_or(0.0),
            "Current dynamic intervention contribution.",
        ),
    ]
    .into_iter()
    .map(|(label, value, strength, detail)| {
        view! {
            <article class="ssm-state-card" style=format!("--state-share:{}%;", (strength * 100.0).clamp(0.0, 100.0))>
                <span>{label}</span>
                <strong>{format_signed(value, 1)}</strong>
                <em>{detail}</em>
                <i><b></b></i>
            </article>
        }
    })
    .collect()
}

fn relationship_graphs(selected: ModelKind) -> Vec<impl IntoView> {
    MODELS
        .into_iter()
        .map(|kind| relationship_graph(kind, selected == kind))
        .collect()
}

fn relationship_graph(kind: ModelKind, active: bool) -> impl IntoView {
    let nodes = relationship_nodes(kind);
    let edges = relationship_edges(kind);
    let marker_id = format!("ssm-arrow-{}", kind.slug());
    let marker_url = format!("url(#{marker_id})");

    view! {
        <article class=if active { "ssm-relationship-card is-active" } else { "ssm-relationship-card" }>
            <div class="ssm-relationship-head">
                <strong>{kind.label()}</strong>
                <span>{relationship_caption(kind)}</span>
            </div>
            <svg
                class="ssm-relationship-svg"
                viewBox="0 0 320 190"
                role="img"
                aria-label=format!("{} intra-state relationship graph", kind.label())
            >
                <defs>
                    <marker
                        id=marker_id
                        viewBox="0 0 10 10"
                        refX="8"
                        refY="5"
                        markerWidth="6"
                        markerHeight="6"
                        orient="auto-start-reverse"
                    >
                        <path d="M 0 0 L 10 5 L 0 10 z"></path>
                    </marker>
                </defs>
                {edges.into_iter().map(|edge| {
                    view! {
                        <g>
                            <line
                                class=edge.class_name
                                x1=format!("{:.1}", edge.x1)
                                y1=format!("{:.1}", edge.y1)
                                x2=format!("{:.1}", edge.x2)
                                y2=format!("{:.1}", edge.y2)
                                marker-end=marker_url.clone()
                            ></line>
                            <text class="ssm-relationship-edge-label" x=format!("{:.1}", edge.lx) y=format!("{:.1}", edge.ly)>{edge.label}</text>
                        </g>
                    }
                }).collect_view()}
                {nodes.into_iter().map(|node| {
                    view! {
                        <g class=node.class_name transform=format!("translate({:.1} {:.1})", node.x, node.y)>
                            <rect x="-42" y="-19" width="84" height="38" rx="7"></rect>
                            <text class="ssm-relationship-node-label" y="-3" text-anchor="middle">{node.label}</text>
                            <text class="ssm-relationship-node-detail" y="12" text-anchor="middle">{node.detail}</text>
                        </g>
                    }
                }).collect_view()}
            </svg>
        </article>
    }
}

fn relationship_caption(kind: ModelKind) -> &'static str {
    match kind {
        ModelKind::LocalLevel => "Observation updates a single latent level state.",
        ModelKind::LocalTrend => "Level and slope jointly form the transition prior.",
        ModelKind::SeasonalTrend => "Trend and seasonal states combine before observation.",
        ModelKind::RegressionSeasonal => "A dynamic driver joins trend and seasonality.",
    }
}

fn relationship_nodes(kind: ModelKind) -> Vec<RelationshipNode> {
    let mut nodes = vec![
        RelationshipNode {
            label: "Level",
            detail: "state",
            x: 74.0,
            y: 84.0,
            class_name: "ssm-relationship-node state-node",
        },
        RelationshipNode {
            label: "Observed",
            detail: "y_t",
            x: 258.0,
            y: 108.0,
            class_name: "ssm-relationship-node observed-node",
        },
        RelationshipNode {
            label: "Forecast",
            detail: "y_t+1",
            x: 258.0,
            y: 42.0,
            class_name: "ssm-relationship-node forecast-node",
        },
    ];
    if kind.has_trend() {
        nodes.push(RelationshipNode {
            label: "Trend",
            detail: "slope",
            x: 74.0,
            y: 146.0,
            class_name: "ssm-relationship-node state-node",
        });
    }
    if kind.has_seasonality() {
        nodes.push(RelationshipNode {
            label: "Seasonal",
            detail: "cycle",
            x: 160.0,
            y: 48.0,
            class_name: "ssm-relationship-node component-node",
        });
    }
    if kind.has_regressor() {
        nodes.push(RelationshipNode {
            label: "Driver",
            detail: "beta x",
            x: 160.0,
            y: 150.0,
            class_name: "ssm-relationship-node component-node",
        });
    }
    nodes
}

fn relationship_edges(kind: ModelKind) -> Vec<RelationshipEdge> {
    let mut edges = vec![
        RelationshipEdge {
            label: "predict",
            x1: 111.0,
            y1: 76.0,
            x2: 217.0,
            y2: 48.0,
            lx: 154.0,
            ly: 54.0,
            class_name: "ssm-relationship-edge state-edge",
        },
        RelationshipEdge {
            label: "update",
            x1: 218.0,
            y1: 108.0,
            x2: 116.0,
            y2: 90.0,
            lx: 154.0,
            ly: 114.0,
            class_name: "ssm-relationship-edge observed-edge",
        },
        RelationshipEdge {
            label: "emit",
            x1: 112.0,
            y1: 86.0,
            x2: 216.0,
            y2: 104.0,
            lx: 156.0,
            ly: 96.0,
            class_name: "ssm-relationship-edge state-edge",
        },
    ];
    if kind.has_trend() {
        edges.extend([
            RelationshipEdge {
                label: "carry",
                x1: 74.0,
                y1: 126.0,
                x2: 74.0,
                y2: 104.0,
                lx: 82.0,
                ly: 120.0,
                class_name: "ssm-relationship-edge state-edge",
            },
            RelationshipEdge {
                label: "slope",
                x1: 112.0,
                y1: 138.0,
                x2: 216.0,
                y2: 110.0,
                lx: 158.0,
                ly: 136.0,
                class_name: "ssm-relationship-edge state-edge",
            },
        ]);
    }
    if kind.has_seasonality() {
        edges.push(RelationshipEdge {
            label: "add",
            x1: 190.0,
            y1: 60.0,
            x2: 218.0,
            y2: 94.0,
            lx: 194.0,
            ly: 82.0,
            class_name: "ssm-relationship-edge component-edge",
        });
    }
    if kind.has_regressor() {
        edges.push(RelationshipEdge {
            label: "explain",
            x1: 190.0,
            y1: 142.0,
            x2: 218.0,
            y2: 118.0,
            lx: 188.0,
            ly: 138.0,
            class_name: "ssm-relationship-edge component-edge",
        });
    }
    edges
}

fn phase_portrait_cards(run: ModelRun, inputs: SsmInputs) -> Vec<impl IntoView> {
    phase_cards(run, inputs)
        .into_iter()
        .map(phase_card_view)
        .collect()
}

fn phase_card_view(card: PhaseCard) -> impl IntoView {
    let marker_url = format!("url(#{})", card.marker_id);
    let threshold_x = phase_x(card.threshold_x);
    let threshold_y = phase_y(card.threshold_y);
    let marker_x = phase_x(card.marker_x);
    let marker_y = phase_y(card.marker_y);

    view! {
        <article class="ssm-phase-card">
            <div class="ssm-phase-head">
                <h3>{card.title}</h3>
                <span>{card.description}</span>
            </div>
            <svg class="ssm-phase-svg" viewBox="0 0 620 360" role="img" aria-label=card.title>
                <defs>
                    <marker
                        id=card.marker_id
                        viewBox="0 0 10 10"
                        refX="8"
                        refY="5"
                        markerWidth="6"
                        markerHeight="6"
                        orient="auto-start-reverse"
                    >
                        <path d="M 0 0 L 10 5 L 0 10 z"></path>
                    </marker>
                </defs>
                <rect class="ssm-phase-plot" x="70" y="54" width="490" height="218" rx="8"></rect>
                <g class="ssm-phase-grid-lines">
                    <line x1="70" y1="108" x2="560" y2="108"></line>
                    <line x1="70" y1="163" x2="560" y2="163"></line>
                    <line x1="70" y1="218" x2="560" y2="218"></line>
                    <line x1="168" y1="54" x2="168" y2="272"></line>
                    <line x1="266" y1="54" x2="266" y2="272"></line>
                    <line x1="364" y1="54" x2="364" y2="272"></line>
                    <line x1="462" y1="54" x2="462" y2="272"></line>
                </g>
                {card.paths.into_iter().map(|path| {
                    view! { <path class=path.class_name d=path.d></path> }
                }).collect_view()}
                {card.arrows.into_iter().map(|arrow| {
                    view! {
                        <line
                            class=arrow.class_name
                            x1=format!("{:.1}", arrow.x1)
                            y1=format!("{:.1}", arrow.y1)
                            x2=format!("{:.1}", arrow.x2)
                            y2=format!("{:.1}", arrow.y2)
                            marker-end=marker_url.clone()
                        ></line>
                    }
                }).collect_view()}
                <line class="ssm-phase-threshold-x" x1=format!("{:.1}", threshold_x) y1="54" x2=format!("{:.1}", threshold_x) y2="272"></line>
                <line class="ssm-phase-threshold-y" x1="70" y1=format!("{:.1}", threshold_y) x2="560" y2=format!("{:.1}", threshold_y)></line>
                <circle class="ssm-phase-marker" cx=format!("{:.1}", marker_x) cy=format!("{:.1}", marker_y) r="7"></circle>
                <text class="ssm-phase-label-left" x="34" y="170" transform="rotate(-90 34 170)">{card.y_label}</text>
                <text class="ssm-phase-label-bottom" x="262" y="320">{card.x_label}</text>
                <text class="ssm-phase-tick" x="65" y="292">"0"</text>
                <text class="ssm-phase-tick" x="554" y="292">"1"</text>
                <text class="ssm-phase-tick" x="48" y="276">"0"</text>
                <text class="ssm-phase-tick" x="48" y="61">"1"</text>
            </svg>
            <div class="ssm-phase-metrics">
                {card.metrics.into_iter().map(|metric| {
                    view! {
                        <article>
                            <strong>{metric.value}</strong>
                            <span>{metric.label}</span>
                            <em>{metric.detail}</em>
                        </article>
                    }
                }).collect_view()}
            </div>
        </article>
    }
}

fn phase_cards(run: ModelRun, inputs: SsmInputs) -> Vec<PhaseCard> {
    let latest = run.points.last().copied().unwrap_or_else(empty_state_point);
    let residual_pressure = (run.bias.abs() / 10.0).clamp(0.02, 0.98);
    let gain_pressure = run.responsiveness.clamp(0.02, 0.98);
    let trend_pressure = ((latest.trend + 6.0) / 12.0).clamp(0.02, 0.98);
    let seasonal_pressure = ((latest.seasonal + 18.0) / 36.0).clamp(0.02, 0.98);
    let shock_pressure = ((inputs.shock_size + 40.0) / 80.0).clamp(0.02, 0.98);
    let regression_pressure = (latest.regression.abs() / 18.0).clamp(0.02, 0.98);
    let interval_pressure = (run.interval_width / 80.0).clamp(0.02, 0.98);
    let stability_pressure = run.stability.clamp(0.02, 0.98);

    vec![
        PhaseCard {
            marker_id: "ssm-phase-filter-arrow",
            title: "Filter Update Phase Portrait",
            description: "Arrows converge where residual surprise and Kalman-style update gain balance.",
            x_label: "standardized residual pressure",
            y_label: "update gain",
            threshold_x: residual_pressure,
            threshold_y: gain_pressure,
            marker_x: residual_pressure,
            marker_y: gain_pressure,
            paths: phase_contours(residual_pressure, gain_pressure, "ssm-phase-path-primary"),
            arrows: converging_field(residual_pressure, gain_pressure),
            metrics: vec![
                PhaseMetric {
                    value: format!("{:.1}%", gain_pressure * 100.0),
                    label: "update gain",
                    detail: "current filter responsiveness",
                },
                PhaseMetric {
                    value: format_signed(run.bias, 2),
                    label: "bias center",
                    detail: "mean one-step residual",
                },
                PhaseMetric {
                    value: phase_model_label(run.kind).to_string(),
                    label: "active family",
                    detail: "selected state specification",
                },
            ],
        },
        PhaseCard {
            marker_id: "ssm-phase-coupling-arrow",
            title: "State Coupling Phase Portrait",
            description: "Rotational flow shows how level, trend, and seasonal states pull each other.",
            x_label: "trend carry",
            y_label: "seasonal pressure",
            threshold_x: trend_pressure,
            threshold_y: seasonal_pressure,
            marker_x: trend_pressure,
            marker_y: seasonal_pressure,
            paths: phase_contours(trend_pressure, seasonal_pressure, "ssm-phase-path-secondary"),
            arrows: rotational_field(trend_pressure, seasonal_pressure),
            metrics: vec![
                PhaseMetric {
                    value: format_signed(latest.trend, 1),
                    label: "trend state",
                    detail: "latest slope contribution",
                },
                PhaseMetric {
                    value: format_signed(latest.seasonal, 1),
                    label: "seasonal state",
                    detail: "current cycle contribution",
                },
                PhaseMetric {
                    value: if run.kind.has_seasonality() { "coupled" } else { "muted" }.to_string(),
                    label: "local dynamic",
                    detail: "seasonal state availability",
                },
            ],
        },
        PhaseCard {
            marker_id: "ssm-phase-shock-arrow",
            title: "Shock Absorption Phase Portrait",
            description: "The field shows whether structural shocks are explained by state motion or the driver.",
            x_label: "structural shock",
            y_label: "driver absorption",
            threshold_x: shock_pressure,
            threshold_y: regression_pressure,
            marker_x: shock_pressure,
            marker_y: regression_pressure,
            paths: phase_contours(shock_pressure, regression_pressure, "ssm-phase-path-primary"),
            arrows: saddle_field(shock_pressure, regression_pressure),
            metrics: vec![
                PhaseMetric {
                    value: format_signed(inputs.shock_size, 0),
                    label: "shock size",
                    detail: "structural break setting",
                },
                PhaseMetric {
                    value: format_signed(latest.regression, 1),
                    label: "driver effect",
                    detail: "dynamic regression contribution",
                },
                PhaseMetric {
                    value: if run.kind.has_regressor() { "explained" } else { "latent" }.to_string(),
                    label: "shock route",
                    detail: "where unexplained movement lands",
                },
            ],
        },
        PhaseCard {
            marker_id: "ssm-phase-forecast-arrow",
            title: "Forecast Uncertainty Phase Portrait",
            description: "Paths move toward the operating frontier where stability offsets interval growth.",
            x_label: "interval pressure",
            y_label: "state stability",
            threshold_x: interval_pressure,
            threshold_y: stability_pressure,
            marker_x: interval_pressure,
            marker_y: stability_pressure,
            paths: phase_contours(interval_pressure, stability_pressure, "ssm-phase-path-secondary"),
            arrows: frontier_field(interval_pressure, stability_pressure),
            metrics: vec![
                PhaseMetric {
                    value: format!("{:.1}", run.interval_width),
                    label: "interval width",
                    detail: "final forecast band",
                },
                PhaseMetric {
                    value: format!("{:.0}%", run.stability * 100.0),
                    label: "state stability",
                    detail: "inverse of update gain",
                },
                PhaseMetric {
                    value: format!("{:.1}", run.forecast_end),
                    label: "terminal point",
                    detail: "last forecast value",
                },
            ],
        },
    ]
}

fn phase_model_label(kind: ModelKind) -> &'static str {
    match kind {
        ModelKind::LocalLevel => "Level",
        ModelKind::LocalTrend => "Trend",
        ModelKind::SeasonalTrend => "Seasonal",
        ModelKind::RegressionSeasonal => "Reg.",
    }
}

fn converging_field(cx: f32, cy: f32) -> Vec<PhaseArrow> {
    vector_field(|x, y| {
        let dx = (cx - x) * 0.26;
        let dy = (cy - y) * 0.26;
        (dx, dy)
    })
}

fn rotational_field(cx: f32, cy: f32) -> Vec<PhaseArrow> {
    vector_field(|x, y| {
        let dx = -(y - cy) * 0.18 + (cx - x) * 0.08;
        let dy = (x - cx) * 0.18 + (cy - y) * 0.08;
        (dx, dy)
    })
}

fn saddle_field(cx: f32, cy: f32) -> Vec<PhaseArrow> {
    vector_field(|x, y| {
        let dx = (cx - x) * 0.22;
        let dy = (y - cy) * 0.16;
        (dx, dy)
    })
}

fn frontier_field(cx: f32, cy: f32) -> Vec<PhaseArrow> {
    vector_field(|x, y| {
        let dx = (cx - x) * 0.12 + 0.025;
        let dy = (cy - y) * 0.24 - (x - cx).max(0.0) * 0.08;
        (dx, dy)
    })
}

fn vector_field(direction: impl Fn(f32, f32) -> (f32, f32)) -> Vec<PhaseArrow> {
    let xs = [0.14, 0.28, 0.42, 0.56, 0.70, 0.84];
    let ys = [0.20, 0.36, 0.52, 0.68, 0.84];
    let mut arrows = Vec::with_capacity(xs.len() * ys.len());
    for x in xs {
        for y in ys {
            let (dx, dy) = direction(x, y);
            let x2 = (x + dx).clamp(0.04, 0.96);
            let y2 = (y + dy).clamp(0.04, 0.96);
            let magnitude = (dx * dx + dy * dy).sqrt();
            arrows.push(PhaseArrow {
                x1: phase_x(x),
                y1: phase_y(y),
                x2: phase_x(x2),
                y2: phase_y(y2),
                class_name: if magnitude > 0.085 {
                    "ssm-phase-arrow is-strong"
                } else {
                    "ssm-phase-arrow"
                },
            });
        }
    }
    arrows
}

fn phase_contours(cx: f32, cy: f32, class_name: &'static str) -> Vec<PhasePath> {
    [0.18_f32, 0.36, 0.56]
        .into_iter()
        .map(|spread| {
            let left = (cx - spread).clamp(0.02, 0.98);
            let right = (cx + spread * 0.95).clamp(0.02, 0.98);
            let top = (cy + spread * 0.60).clamp(0.02, 0.98);
            let bottom = (cy - spread * 0.58).clamp(0.02, 0.98);
            PhasePath {
                d: format!(
                    "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                    phase_x(left),
                    phase_y(top),
                    phase_x((left + cx) * 0.5),
                    phase_y((top + cy).min(0.98)),
                    phase_x((right + cx) * 0.5),
                    phase_y((bottom + cy).max(0.02)),
                    phase_x(right),
                    phase_y(bottom),
                ),
                class_name,
            }
        })
        .collect()
}

fn phase_x(value: f32) -> f32 {
    70.0 + value.clamp(0.0, 1.0) * 490.0
}

fn phase_y(value: f32) -> f32 {
    272.0 - value.clamp(0.0, 1.0) * 218.0
}

fn selected_spec(run: ModelRun) -> LineChartSpec {
    let max_month = run.points.last().map(|point| point.month).unwrap_or(0) + FORECAST_STEPS;
    let mut data = Vec::new();

    for point in &run.points {
        data.push(LineDatum::new(
            "Observed",
            point.month as f32,
            point.observed,
        ));
        data.push(LineDatum::new(
            "Filtered",
            point.month as f32,
            point.filtered,
        ));
        data.push(LineDatum::new(
            "Latent signal",
            point.month as f32,
            point.latent,
        ));
    }

    if let Some(last) = run.points.last() {
        data.push(LineDatum::new("Forecast", last.month as f32, last.filtered));
        data.push(LineDatum::new(
            "Upper interval",
            last.month as f32,
            last.filtered,
        ));
        data.push(LineDatum::new(
            "Lower interval",
            last.month as f32,
            last.filtered,
        ));
    }

    for point in &run.forecast {
        data.push(LineDatum::new(
            "Forecast",
            point.month as f32,
            point.forecast,
        ));
        data.push(LineDatum::new(
            "Upper interval",
            point.month as f32,
            point.upper,
        ));
        data.push(LineDatum::new(
            "Lower interval",
            point.month as f32,
            point.lower,
        ));
    }

    let (min_y, max_y) = data_domain(&data);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "Index".to_string(),
        x_domain: Some((0.0, max_month as f32)),
        y_domain: Some(padded_domain(min_y, max_y)),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn residual_spec(inputs: SsmInputs) -> LineChartSpec {
    let mut data = Vec::new();
    for kind in MODELS {
        let run = fit_model(inputs, kind);
        for point in &run.points {
            data.push(LineDatum::new(
                kind.short_label(),
                point.month as f32,
                point.residual,
            ));
        }
    }
    let max_abs = data
        .iter()
        .map(|datum| datum.y.abs())
        .fold(1.0_f32, f32::max);

    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "Residual".to_string(),
        x_domain: Some((0.0, inputs.months as f32)),
        y_domain: Some((-max_abs * 1.18, max_abs * 1.18)),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn comparison_spec(inputs: SsmInputs) -> LineChartSpec {
    let mut data = Vec::new();
    for kind in MODELS {
        let run = fit_model(inputs, kind);
        if let Some(last) = run.points.last() {
            data.push(LineDatum::new(
                kind.short_label(),
                last.month as f32,
                last.filtered,
            ));
        }
        for point in &run.forecast {
            data.push(LineDatum::new(
                kind.short_label(),
                point.month as f32,
                point.forecast,
            ));
        }
    }

    let (min_y, max_y) = data_domain(&data);
    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: "Forecast".to_string(),
        x_domain: Some((
            inputs.months as f32,
            (inputs.months + FORECAST_STEPS) as f32,
        )),
        y_domain: Some(padded_domain(min_y, max_y)),
        x_tick_count: 5,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn best_model(inputs: SsmInputs) -> ModelRun {
    MODELS
        .into_iter()
        .map(|kind| fit_model(inputs, kind))
        .min_by(|a, b| a.mae.total_cmp(&b.mae))
        .unwrap_or_else(|| fit_model(inputs, ModelKind::LocalLevel))
}

fn fit_model(inputs: SsmInputs, kind: ModelKind) -> ModelRun {
    let observations = observations(inputs);
    let first = observations.first().copied().unwrap_or(ObservationPoint {
        month: 0,
        observed: 100.0,
        latent: 100.0,
        regressor: 0.0,
    });

    let mut level = first.observed;
    let mut trend = 0.0_f32;
    let mut regression = 0.0_f32;
    let mut seasonal = [0.0_f32; 12];
    let mut points = Vec::with_capacity(observations.len());
    let alpha =
        (0.10 + inputs.process_noise * 0.48 - inputs.observation_noise * 0.10).clamp(0.05, 0.72);
    let beta = if kind.has_trend() {
        (0.012 + inputs.trend_flex * 0.070 + inputs.process_noise * 0.040).clamp(0.006, 0.16)
    } else {
        0.0
    };
    let gamma = if kind.has_seasonality() {
        (0.025 + inputs.seasonal_strength * 0.11 + inputs.process_noise * 0.035).clamp(0.015, 0.24)
    } else {
        0.0
    };
    let delta = if kind.has_regressor() {
        (0.006 + inputs.regressor_strength * 0.030).clamp(0.004, 0.055)
    } else {
        0.0
    };

    for obs in observations {
        let season_index = (obs.month as usize) % seasonal.len();
        let predicted_level = level + trend;
        let season = if kind.has_seasonality() {
            seasonal[season_index]
        } else {
            0.0
        };
        let regression_effect = if kind.has_regressor() {
            regression * obs.regressor
        } else {
            0.0
        };
        let predicted = predicted_level + season + regression_effect;
        let residual = obs.observed - predicted;

        level = predicted_level + alpha * residual;
        trend = if kind.has_trend() {
            (trend + beta * residual).clamp(-6.0, 6.0)
        } else {
            0.0
        };
        if kind.has_seasonality() {
            seasonal[season_index] += gamma * residual;
            center_seasonal(&mut seasonal);
        }
        if kind.has_regressor() {
            regression += delta * residual * obs.regressor / (1.0 + obs.regressor * obs.regressor);
            regression = regression.clamp(-16.0, 24.0);
        } else {
            regression = 0.0;
        }

        let filtered_season = if kind.has_seasonality() {
            seasonal[season_index]
        } else {
            0.0
        };
        let filtered_regression = if kind.has_regressor() {
            regression * obs.regressor
        } else {
            0.0
        };
        let filtered = level + filtered_season + filtered_regression;
        points.push(StatePoint {
            month: obs.month,
            observed: obs.observed,
            latent: obs.latent,
            filtered,
            residual,
            level,
            trend,
            seasonal: filtered_season,
            regression: filtered_regression,
        });
    }

    let forecast = forecast_points(inputs, kind, level, trend, regression, seasonal);
    let mae = mean_abs_error(&points);
    let rmse = rmse(&points);
    let bias = if points.is_empty() {
        0.0
    } else {
        points.iter().map(|point| point.residual).sum::<f32>() / points.len() as f32
    };
    let forecast_end = forecast.last().map(|point| point.forecast).unwrap_or(level);
    let interval_width = forecast
        .last()
        .map(|point| point.upper - point.lower)
        .unwrap_or(0.0);
    let stability = (1.0 - alpha).clamp(0.0, 1.0);
    let responsiveness = (alpha + beta + gamma + delta).clamp(0.0, 1.0);

    ModelRun {
        kind,
        points,
        forecast,
        mae,
        rmse,
        bias,
        forecast_end,
        interval_width,
        stability,
        responsiveness,
    }
}

fn observations(inputs: SsmInputs) -> Vec<ObservationPoint> {
    (0..=inputs.months)
        .map(|month| {
            let t = month as f32;
            let baseline = 96.0 + t * 0.72;
            let trend_bend = ((t - 18.0) / 9.0).tanh() * 5.0;
            let seasonal = inputs.seasonal_strength
                * (8.5 * (std::f32::consts::TAU * t / 12.0).sin()
                    + 3.2 * (std::f32::consts::TAU * t / 6.0).cos());
            let regressor = campaign_signal(month);
            let campaign = regressor * inputs.regressor_strength * 14.0;
            let shock = if month >= inputs.months / 2 {
                inputs.shock_size * (-((t - inputs.months as f32 / 2.0) / 13.0)).exp()
            } else {
                0.0
            };
            let latent = baseline + trend_bend + seasonal + campaign + shock;
            let noise = deterministic_noise(month, 17.0) * (2.0 + inputs.observation_noise * 18.0);
            ObservationPoint {
                month,
                observed: latent + noise,
                latent,
                regressor,
            }
        })
        .collect()
}

fn forecast_points(
    inputs: SsmInputs,
    kind: ModelKind,
    mut level: f32,
    trend: f32,
    regression: f32,
    seasonal: [f32; 12],
) -> Vec<ForecastPoint> {
    (1..=FORECAST_STEPS)
        .map(|step| {
            let month = inputs.months + step;
            level += if kind.has_trend() { trend } else { 0.0 };
            let season = if kind.has_seasonality() {
                seasonal[(month as usize) % seasonal.len()]
            } else {
                0.0
            };
            let regressor = if kind.has_regressor() {
                regression * campaign_signal(month)
            } else {
                0.0
            };
            let forecast = level + season + regressor;
            let growth = step as f32 / FORECAST_STEPS as f32;
            let model_factor = match kind {
                ModelKind::LocalLevel => 1.35,
                ModelKind::LocalTrend => 1.15,
                ModelKind::SeasonalTrend => 1.05,
                ModelKind::RegressionSeasonal => 0.95,
            };
            let half_width = (6.0 + inputs.observation_noise * 22.0)
                * (1.0 + growth * (0.45 + inputs.process_noise * 1.6))
                * model_factor;
            ForecastPoint {
                month,
                forecast,
                lower: forecast - half_width,
                upper: forecast + half_width,
            }
        })
        .collect()
}

fn campaign_signal(month: u32) -> f32 {
    let t = month as f32;
    let first = (-(t - 16.0).powi(2) / 42.0).exp();
    let second = 0.72 * (-(t - 39.0).powi(2) / 56.0).exp();
    first + second
}

fn deterministic_noise(month: u32, phase: f32) -> f32 {
    let x = month as f32 + phase;
    ((x * 12.9898).sin() * 0.62 + (x * 5.233).cos() * 0.38).clamp(-1.0, 1.0)
}

fn center_seasonal(seasonal: &mut [f32; 12]) {
    let mean = seasonal.iter().sum::<f32>() / seasonal.len() as f32;
    for value in seasonal {
        *value -= mean;
    }
}

fn mean_abs_error(points: &[StatePoint]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    points.iter().map(|point| point.residual.abs()).sum::<f32>() / points.len() as f32
}

fn rmse(points: &[StatePoint]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    (points
        .iter()
        .map(|point| point.residual * point.residual)
        .sum::<f32>()
        / points.len() as f32)
        .sqrt()
}

fn empty_state_point() -> StatePoint {
    StatePoint {
        month: 0,
        observed: 0.0,
        latent: 0.0,
        filtered: 0.0,
        residual: 0.0,
        level: 0.0,
        trend: 0.0,
        seasonal: 0.0,
        regression: 0.0,
    }
}

fn data_domain(data: &[LineDatum]) -> (f32, f32) {
    data.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_y, max_y), datum| (min_y.min(datum.y), max_y.max(datum.y)),
    )
}

fn padded_domain(min_y: f32, max_y: f32) -> (f32, f32) {
    if !min_y.is_finite() || !max_y.is_finite() {
        return (0.0, 1.0);
    }
    let span = (max_y - min_y).abs().max(1.0);
    (min_y - span * 0.14, max_y + span * 0.14)
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

fn format_signed(value: f32, decimals: usize) -> String {
    match decimals {
        1 => format!("{value:+.1}"),
        2 => format!("{value:+.2}"),
        _ => format!("{value:+.0}"),
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
