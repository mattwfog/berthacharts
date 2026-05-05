//! Framework-oriented state space visualizations.

use std::sync::Arc;

use berthacharts_charts::line::{LineChartOptions, LineChartSpec, LineDatum};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};
use crate::dom_events::event_target_value_as_f32;

const W: u32 = 620;
const H: u32 = 300;

#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameworkInputs {
    horizon: u32,
    observation_noise: f32,
    process_noise: f32,
    intervention: f32,
    regime_pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SignalPoint {
    month: u32,
    observed: f32,
    latent: f32,
    driver: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameworkSummary {
    kalman_gain: f32,
    trend_slope: f32,
    seasonal_share: f32,
    high_regime_probability: f32,
}

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(false);
    let show_axes = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let horizon = RwSignal::new(54.0_f32);
    let observation_noise = RwSignal::new(30.0_f32);
    let process_noise = RwSignal::new(38.0_f32);
    let intervention = RwSignal::new(52.0_f32);
    let regime_pressure = RwSignal::new(46.0_f32);

    let inputs = move || FrameworkInputs {
        horizon: horizon.get().round().clamp(30.0, 84.0) as u32,
        observation_noise: observation_noise.get() / 100.0,
        process_noise: process_noise.get() / 100.0,
        intervention: intervention.get() / 100.0,
        regime_pressure: regime_pressure.get() / 100.0,
    };
    let summary = move || framework_summary(inputs());

    view! {
        <section id="state-space-frameworks" class="example state-space-frameworks">
            <div class="example-head">
                <div>
                    <h2>"State Space Frameworks"</h2>
                    <p>
                        "Separate framework views for Kalman filtering, dynamic linear models, structural decomposition, and Markov-switching regimes."
                    </p>
                </div>
                <div class="stat-strip ssm-stat-strip">
                    <span><strong>{move || format!("{:.2}", summary().kalman_gain)}</strong>" Kalman gain"</span>
                    <span><strong>{move || format_signed(summary().trend_slope, 2)}</strong>" DLM slope"</span>
                    <span><strong>{move || format!("{:.0}%", summary().seasonal_share * 100.0)}</strong>" seasonal share"</span>
                    <span><strong>{move || format!("{:.0}%", summary().high_regime_probability * 100.0)}</strong>" high regime"</span>
                </div>
            </div>

            <div class="framework-layout">
                <aside class="framework-controls" aria-label="State space framework controls">
                    <div class="framework-control-head">
                        <h3>"Shared Signal"</h3>
                        <span>"Drive all framework demos from the same synthetic observed series so their assumptions are comparable."</span>
                    </div>
                    <FrameworkSlider label="Horizon" min=30.0 max=84.0 step=1.0 value=horizon decimals=0 />
                    <FrameworkSlider label="Observation noise" suffix="%" min=0.0 max=100.0 step=1.0 value=observation_noise decimals=0 />
                    <FrameworkSlider label="Process noise" suffix="%" min=0.0 max=100.0 step=1.0 value=process_noise decimals=0 />
                    <FrameworkSlider label="Intervention lift" suffix="%" min=0.0 max=100.0 step=1.0 value=intervention decimals=0 />
                    <FrameworkSlider label="Regime pressure" suffix="%" min=0.0 max=100.0 step=1.0 value=regime_pressure decimals=0 />

                    <div class="framework-notes">
                        <article>
                            <strong>"Kalman"</strong>
                            <span>"Prediction-update recursion, uncertainty, and adaptive gain."</span>
                        </article>
                        <article>
                            <strong>"DLM"</strong>
                            <span>"Level and slope states estimated through a dynamic linear model."</span>
                        </article>
                        <article>
                            <strong>"Structural"</strong>
                            <span>"Trend, seasonal, and intervention components split into addends."</span>
                        </article>
                        <article>
                            <strong>"Switching"</strong>
                            <span>"Filtered probability of moving between low and high regimes."</span>
                        </article>
                    </div>
                </aside>

                <div class="framework-main">
                    <DisplayControls label="State space framework display options">
                        <DisplayToggleButton label="Data labels" state=show_data_labels />
                        <DisplayToggleButton label="Axes" state=show_axes />
                        <DisplayToggleButton label="Legend" state=show_legend />
                    </DisplayControls>

                    <div class="framework-card-grid">
                        <FrameworkMetric
                            label="Recursive update"
                            value=move || format!("{:.2}", summary().kalman_gain)
                            detail=move || "Latest gain balances model prior against the new observation.".to_string()
                        />
                        <FrameworkMetric
                            label="State vector"
                            value=move || format_signed(summary().trend_slope, 2)
                            detail=move || "The DLM slope state carries momentum into each prediction.".to_string()
                        />
                        <FrameworkMetric
                            label="Additive split"
                            value=move || format!("{:.0}%", summary().seasonal_share * 100.0)
                            detail=move || "Structural decomposition exposes how much movement is seasonal.".to_string()
                        />
                        <FrameworkMetric
                            label="Latent regime"
                            value=move || format!("{:.0}%", summary().high_regime_probability * 100.0)
                            detail=move || "Switching models keep a filtered probability over hidden states.".to_string()
                        />
                    </div>

                    <div class="framework-chart-grid">
                        <div>
                            <h3>"Kalman filter recursion"</h3>
                            {move || framework_chart_view(kalman_spec(inputs()), show_data_labels, show_axes, show_legend)}
                        </div>
                        <div>
                            <h3>"Dynamic linear model states"</h3>
                            {move || framework_chart_view(dlm_spec(inputs()), show_data_labels, show_axes, show_legend)}
                        </div>
                        <div>
                            <h3>"Structural decomposition"</h3>
                            {move || framework_chart_view(structural_spec(inputs()), show_data_labels, show_axes, show_legend)}
                        </div>
                        <div>
                            <h3>"Markov-switching regimes"</h3>
                            {move || framework_chart_view(switching_spec(inputs()), show_data_labels, show_axes, show_legend)}
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn FrameworkSlider(
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
        <label class="framework-control">
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
fn FrameworkMetric(
    #[prop(into)] label: String,
    value: impl Fn() -> String + Copy + Send + Sync + 'static,
    detail: impl Fn() -> String + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <article class="framework-metric">
            <span>{label}</span>
            <strong>{value}</strong>
            <em>{detail}</em>
        </article>
    }
}

fn framework_chart_view(
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
            .expect("state space framework chart spec should be valid")
    });

    view! {
        <div class=move || chart_stage_class(
            "chart-stage framework-chart-stage",
            show_data_labels.get(),
            show_axes.get(),
            show_legend.get(),
        )>
            <ChartCanvas width={W} height={H} builder={build} />
        </div>
    }
}

fn kalman_spec(inputs: FrameworkInputs) -> LineChartSpec {
    let signal = signal(inputs);
    let mut posterior = signal.first().map(|point| point.observed).unwrap_or(100.0);
    let mut variance = 18.0 + inputs.observation_noise * 34.0;
    let q = 1.5 + inputs.process_noise * 10.0;
    let r = 6.0 + inputs.observation_noise * 36.0;
    let mut data = Vec::new();

    for point in signal {
        let prior = posterior + point.driver * inputs.intervention * 1.6;
        let prior_variance = variance + q;
        let gain = prior_variance / (prior_variance + r);
        posterior = prior + gain * (point.observed - prior);
        variance = (1.0 - gain) * prior_variance;

        data.push(LineDatum::new(
            "Observed",
            point.month as f32,
            point.observed,
        ));
        data.push(LineDatum::new("Prior", point.month as f32, prior));
        data.push(LineDatum::new("Posterior", point.month as f32, posterior));
        data.push(LineDatum::new(
            "Gain x100",
            point.month as f32,
            gain * 100.0,
        ));
    }

    line_spec(data, inputs.horizon, "Index")
}

fn dlm_spec(inputs: FrameworkInputs) -> LineChartSpec {
    let signal = signal(inputs);
    let mut level = signal.first().map(|point| point.observed).unwrap_or(100.0);
    let mut slope = 0.3_f32;
    let alpha = (0.16 + inputs.process_noise * 0.42).clamp(0.08, 0.68);
    let beta = (0.02 + inputs.process_noise * 0.08).clamp(0.01, 0.14);
    let mut data = Vec::new();

    for point in signal {
        let predicted = level + slope;
        let residual = point.observed - predicted;
        level = predicted + alpha * residual;
        slope = (slope + beta * residual).clamp(-4.0, 4.0);

        data.push(LineDatum::new(
            "Observed",
            point.month as f32,
            point.observed,
        ));
        data.push(LineDatum::new("Level", point.month as f32, level));
        data.push(LineDatum::new(
            "Slope state",
            point.month as f32,
            100.0 + slope * 18.0,
        ));
        data.push(LineDatum::new(
            "One-step forecast",
            point.month as f32,
            level + slope,
        ));
    }

    line_spec(data, inputs.horizon, "State")
}

fn structural_spec(inputs: FrameworkInputs) -> LineChartSpec {
    let mut data = Vec::new();
    for point in signal(inputs) {
        let t = point.month as f32;
        let trend = 92.0 + t * 0.64 + ((t - 24.0) / 11.0).tanh() * 4.8;
        let seasonal = seasonal_component(t) * (0.45 + inputs.process_noise * 0.35);
        let intervention = point.driver * inputs.intervention * 18.0;
        data.push(LineDatum::new("Observed", t, point.observed));
        data.push(LineDatum::new("Trend", t, trend));
        data.push(LineDatum::new("Trend + seasonal", t, trend + seasonal));
        data.push(LineDatum::new(
            "Full structure",
            t,
            trend + seasonal + intervention,
        ));
    }

    line_spec(data, inputs.horizon, "Component")
}

fn switching_spec(inputs: FrameworkInputs) -> LineChartSpec {
    let signal = signal(inputs);
    let mut high_probability = 0.24 + inputs.regime_pressure * 0.28;
    let mut low_mean = 98.0_f32;
    let mut high_mean = 122.0_f32 + inputs.intervention * 12.0;
    let mut data = Vec::new();

    for point in signal {
        let pressure = logistic((point.observed - 111.0) / 7.5 + inputs.regime_pressure - 0.42);
        high_probability = (high_probability * 0.72 + pressure * 0.28).clamp(0.02, 0.98);
        low_mean += 0.08 * (point.observed - low_mean) * (1.0 - high_probability);
        high_mean += 0.08 * (point.observed - high_mean) * high_probability;
        let expected = low_mean * (1.0 - high_probability) + high_mean * high_probability;

        data.push(LineDatum::new(
            "Observed",
            point.month as f32,
            point.observed,
        ));
        data.push(LineDatum::new("Regime mean", point.month as f32, expected));
        data.push(LineDatum::new(
            "High regime %",
            point.month as f32,
            high_probability * 100.0,
        ));
        data.push(LineDatum::new(
            "Low regime %",
            point.month as f32,
            (1.0 - high_probability) * 100.0,
        ));
    }

    line_spec(data, inputs.horizon, "Index / probability")
}

fn framework_summary(inputs: FrameworkInputs) -> FrameworkSummary {
    let signal = signal(inputs);
    let mut posterior = signal.first().map(|point| point.observed).unwrap_or(100.0);
    let mut variance = 18.0 + inputs.observation_noise * 34.0;
    let q = 1.5 + inputs.process_noise * 10.0;
    let r = 6.0 + inputs.observation_noise * 36.0;
    let mut latest_gain = 0.0;
    let mut level = posterior;
    let mut slope = 0.3_f32;
    let mut high_probability = 0.24 + inputs.regime_pressure * 0.28;
    let mut total_move = 0.0_f32;
    let mut seasonal_move = 0.0_f32;

    for point in signal {
        let prior = posterior + point.driver * inputs.intervention * 1.6;
        let prior_variance = variance + q;
        latest_gain = prior_variance / (prior_variance + r);
        posterior = prior + latest_gain * (point.observed - prior);
        variance = (1.0 - latest_gain) * prior_variance;

        let predicted = level + slope;
        let residual = point.observed - predicted;
        level = predicted + (0.16 + inputs.process_noise * 0.42).clamp(0.08, 0.68) * residual;
        slope = (slope + (0.02 + inputs.process_noise * 0.08).clamp(0.01, 0.14) * residual)
            .clamp(-4.0, 4.0);

        let t = point.month as f32;
        total_move += (point.observed - point.latent).abs() + point.latent.abs() * 0.04;
        seasonal_move += seasonal_component(t).abs() * (0.45 + inputs.process_noise * 0.35);

        let pressure = logistic((point.observed - 111.0) / 7.5 + inputs.regime_pressure - 0.42);
        high_probability = (high_probability * 0.72 + pressure * 0.28).clamp(0.02, 0.98);
    }

    FrameworkSummary {
        kalman_gain: latest_gain,
        trend_slope: slope,
        seasonal_share: if total_move > 0.0 {
            (seasonal_move / total_move).clamp(0.0, 1.0)
        } else {
            0.0
        },
        high_regime_probability: high_probability,
    }
}

fn signal(inputs: FrameworkInputs) -> Vec<SignalPoint> {
    (0..=inputs.horizon)
        .map(|month| {
            let t = month as f32;
            let trend = 92.0 + t * 0.64 + ((t - 24.0) / 11.0).tanh() * 4.8;
            let seasonal = seasonal_component(t) * (0.45 + inputs.process_noise * 0.35);
            let driver = intervention_driver(t);
            let regime = if t > inputs.horizon as f32 * 0.58 {
                inputs.regime_pressure * 13.0
            } else {
                0.0
            };
            let latent = trend + seasonal + driver * inputs.intervention * 18.0 + regime;
            let noise = deterministic_noise(month, 4.0) * (2.5 + inputs.observation_noise * 18.0);
            SignalPoint {
                month,
                observed: latent + noise,
                latent,
                driver,
            }
        })
        .collect()
}

fn line_spec(data: Vec<LineDatum>, horizon: u32, y_axis_label: &'static str) -> LineChartSpec {
    let (min_y, max_y) = data.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_y, max_y), datum| (min_y.min(datum.y), max_y.max(datum.y)),
    );
    let span = (max_y - min_y).abs().max(1.0);

    LineChartSpec::new(data).with_options(LineChartOptions {
        x_axis_label: "Month".to_string(),
        y_axis_label: y_axis_label.to_string(),
        x_domain: Some((0.0, horizon as f32)),
        y_domain: Some((min_y - span * 0.14, max_y + span * 0.14)),
        x_tick_count: 7,
        y_tick_count: 5,
        padding_right: 112.0,
        ..LineChartOptions::default()
    })
}

fn seasonal_component(t: f32) -> f32 {
    7.8 * (std::f32::consts::TAU * t / 12.0).sin() + 2.6 * (std::f32::consts::TAU * t / 6.0).cos()
}

fn intervention_driver(t: f32) -> f32 {
    (-(t - 20.0).powi(2) / 46.0).exp() + 0.74 * (-(t - 45.0).powi(2) / 74.0).exp()
}

fn deterministic_noise(month: u32, phase: f32) -> f32 {
    let x = month as f32 + phase;
    ((x * 11.713).sin() * 0.58 + (x * 4.917).cos() * 0.42).clamp(-1.0, 1.0)
}

fn logistic(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
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
