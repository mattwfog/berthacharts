//! Interactive Thompson sampling chart with posterior densities and allocation race.

use leptos::prelude::*;

use crate::dom_events::event_target_value_as_f32;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ThompsonInputs {
    prior_strength: f32,
    exploration: f32,
    lift: f32,
    traffic: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Variant {
    name: &'static str,
    observed_success: f32,
    observed_failure: f32,
    lift_bias: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Posterior {
    variant: Variant,
    alpha: f32,
    beta: f32,
    mean: f32,
    uncertainty: f32,
    allocation: f32,
}

#[component]
pub fn View() -> impl IntoView {
    let prior_strength = RwSignal::new(18.0_f32);
    let exploration = RwSignal::new(56.0_f32);
    let lift = RwSignal::new(24.0_f32);
    let traffic = RwSignal::new(4200.0_f32);

    let inputs = move || ThompsonInputs {
        prior_strength: prior_strength.get(),
        exploration: exploration.get(),
        lift: lift.get(),
        traffic: traffic.get(),
    };
    let posteriors = move || posterior_state(inputs());
    let winner = move || {
        posteriors()
            .into_iter()
            .max_by(|a, b| {
                a.allocation
                    .partial_cmp(&b.allocation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|posterior| posterior.variant.name)
            .unwrap_or("None")
    };
    let best_mean = move || {
        posteriors()
            .iter()
            .map(|posterior| posterior.mean)
            .fold(0.0_f32, f32::max)
    };
    let expected_wins = move || {
        posteriors()
            .iter()
            .map(|posterior| posterior.mean * posterior.allocation * inputs().traffic)
            .sum::<f32>()
    };
    let entropy = move || allocation_entropy(&posteriors());

    view! {
        <section id="thompson-sampling" class="example game-theory decision-example">
            <div class="example-head">
                <div>
                    <h2>"Thompson Sampling Posterior Race"</h2>
                    <p>
                        "A Bayesian multi-arm experiment view with beta posterior densities, stochastic allocation shares, regret pressure, and simulated draw traces."
                    </p>
                </div>
                <div class="stat-strip game-stat-strip">
                    <span><strong>{winner}</strong>" allocation leader"</span>
                    <span><strong>{move || format!("{:.2}%", best_mean() * 100.0)}</strong>" best mean"</span>
                    <span><strong>{move || format!("{:.0}", expected_wins())}</strong>" expected wins"</span>
                    <span><strong>{move || format!("{:.2}", entropy())}</strong>" allocation entropy"</span>
                </div>
            </div>

            <div class="game-layout">
                <aside class="game-controls" aria-label="Thompson sampling controls">
                    <div class="game-control-head">
                        <h3>"Experiment Levers"</h3>
                        <span>"Control prior weight, exploration temperature, treatment lift, and traffic volume."</span>
                    </div>
                    <ThompsonSlider label="Prior strength" min=2.0 max=60.0 step=1.0 value=prior_strength />
                    <ThompsonSlider label="Exploration" suffix="%" min=0.0 max=100.0 step=1.0 value=exploration />
                    <ThompsonSlider label="Treatment lift" suffix="%" min=-20.0 max=60.0 step=1.0 value=lift />
                    <ThompsonSlider label="Daily traffic" min=800.0 max=9000.0 step=100.0 value=traffic />

                    <div class="game-notes">
                        <div>
                            <strong>"Posterior density"</strong>
                            <span>"Each curve is a beta posterior over conversion probability."</span>
                        </div>
                        <div>
                            <strong>"Allocation share"</strong>
                            <span>"Traffic shifts toward arms with high posterior mean and high useful uncertainty."</span>
                        </div>
                    </div>
                </aside>

                <div class="game-main">
                    <ThompsonStory posteriors=posteriors />

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Beta Posterior Densities"</h3>
                            <span>"Wide curves still get sampled; narrow curves dominate only when their means separate."</span>
                        </div>
                        <PosteriorChart inputs=inputs />
                    </div>

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Traffic Allocation Race"</h3>
                            <span>"Allocation bars show the current Thompson policy after posterior uncertainty is priced in."</span>
                        </div>
                        <div class="thompson-board">
                            {move || posteriors().into_iter().map(|posterior| {
                                view! {
                                    <article class="thompson-arm">
                                        <span>{posterior.variant.name}</span>
                                        <strong>{format!("{:.1}%", posterior.allocation * 100.0)}</strong>
                                        <em>{format!("mean {:.2}% / alpha {:.0} / beta {:.0}", posterior.mean * 100.0, posterior.alpha, posterior.beta)}</em>
                                        <i style=format!("--bar:{}%;", posterior.allocation * 100.0)><b class=posterior.variant.class_name></b></i>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="regime-panel">
                        <div class="game-section-head">
                            <h3>"Bayesian Regime Diagnostics"</h3>
                            <span>"Experiment mode inferred from posterior separation, uncertainty, entropy, and lift."</span>
                        </div>
                        <div class="regime-grid">
                            {move || thompson_regimes(inputs(), posteriors()).into_iter().map(|cell| {
                                view! {
                                    <article class=cell.3 style=format!("--regime-share:{}%;", cell.2)>
                                        <span>{cell.0}</span>
                                        <strong>{cell.1}</strong>
                                        <em>{cell.4}</em>
                                        <i><b></b></i>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn ThompsonStory<F>(posteriors: F) -> impl IntoView
where
    F: Fn() -> Vec<Posterior> + Copy + Send + Sync + 'static,
{
    view! {
        <div class="story-panel thompson-story-panel">
            <div class="game-section-head">
                <h3>"What the experiment believes now"</h3>
                <span>"The posterior story moves from evidence, to overlap risk, to the traffic decision."</span>
            </div>
            {move || {
                let ranked = ranked_posteriors(posteriors());
                let leader = ranked.first().copied();
                let runner_up = ranked.get(1).copied();
                let gap = leader
                    .zip(runner_up)
                    .map(|(lead, next)| (lead.mean - next.mean) * 100.0)
                    .unwrap_or_default();
                let entropy = allocation_entropy(&ranked);
                let confidence = (gap / 2.4 * 100.0).clamp(0.0, 100.0);
                let overlap_class = if entropy > 1.15 { "story-card story-amber" } else { "story-card story-green" };
                view! {
                    <div class="story-flow">
                        <article class="story-card story-blue" style=format!("--story-share:{}%;", confidence)>
                            <span>"Evidence leader"</span>
                            <strong>{leader.map(|posterior| posterior.variant.name).unwrap_or("None")}</strong>
                            <em>{leader.map(|posterior| format!("Posterior mean is {:.2}% with {:.3} sigma.", posterior.mean * 100.0, posterior.uncertainty)).unwrap_or_else(|| "No posterior evidence yet.".to_string())}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"but"</div>
                        <article class=overlap_class style=format!("--story-share:{}%;", (entropy / 1.39 * 100.0).clamp(0.0, 100.0))>
                            <span>"Overlap risk"</span>
                            <strong>{if gap < 0.8 { "Keep learning" } else { "Leader separating" }}</strong>
                            <em>{format!("Top two means are separated by {:.2} points; allocation entropy is {:.2}.", gap, entropy)}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"therefore"</div>
                        <article class="story-card story-green" style=format!("--story-share:{}%;", leader.map(|posterior| posterior.allocation * 100.0).unwrap_or(0.0))>
                            <span>"Traffic decision"</span>
                            <strong>{leader.map(|posterior| format!("{:.1}% to {}", posterior.allocation * 100.0, posterior.variant.name)).unwrap_or_else(|| "No traffic assigned".to_string())}</strong>
                            <em>{runner_up.map(|posterior| format!("Runner-up still receives {:.1}% while the posterior overlap remains live.", posterior.allocation * 100.0)).unwrap_or_else(|| "No runner-up arm.".to_string())}</em>
                            <i><b></b></i>
                        </article>
                    </div>
                }
            }}
        </div>
    }
}

#[component]
fn ThompsonSlider(
    #[prop(into)] label: String,
    #[prop(default = "")] suffix: &'static str,
    min: f32,
    max: f32,
    step: f32,
    value: RwSignal<f32>,
) -> impl IntoView {
    let value_label = move || format!("{:.0}{}", value.get(), suffix);

    view! {
        <label class="game-control">
            <span>
                <strong>{label}</strong>
                <em>{value_label}</em>
            </span>
            <input
                type="range"
                min=min
                max=max
                step=step
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value_as_f32(&ev, value.get()))
            />
        </label>
    }
}

#[component]
fn PosteriorChart<F>(inputs: F) -> impl IntoView
where
    F: Fn() -> ThompsonInputs + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-plot decision-svg-panel">
            <svg class="phase-svg" viewBox="0 0 620 330" role="img" aria-label="Thompson sampling posterior densities">
                {move || {
                    let ranked = ranked_posteriors(posterior_state(inputs()));
                    let leader = ranked.first().copied();
                    let next = ranked.get(1).copied();
                    let (x1, width) = leader
                        .zip(next)
                        .map(|(lead, runner)| {
                            let left = thompson_x(runner.mean.min(lead.mean));
                            let right = thompson_x(runner.mean.max(lead.mean));
                            (left, (right - left).max(4.0))
                        })
                        .unwrap_or((46.0, 0.0));
                    view! {
                        <rect class="story-zone story-zone-amber" x=format!("{:.1}", x1) y="34" width=format!("{:.1}", width) height="220" rx="8"></rect>
                    }
                }}
                <rect class="phase-frame" x="46" y="34" width="520" height="220" rx="8"></rect>
                <g class="phase-grid">
                    <line x1="46" y1="78" x2="566" y2="78"></line>
                    <line x1="46" y1="122" x2="566" y2="122"></line>
                    <line x1="46" y1="166" x2="566" y2="166"></line>
                    <line x1="46" y1="210" x2="566" y2="210"></line>
                    <line x1="150" y1="34" x2="150" y2="254"></line>
                    <line x1="254" y1="34" x2="254" y2="254"></line>
                    <line x1="358" y1="34" x2="358" y2="254"></line>
                    <line x1="462" y1="34" x2="462" y2="254"></line>
                </g>
                {move || posterior_state(inputs()).into_iter().map(|posterior| {
                    view! {
                        <path class=posterior.variant.class_name d=density_path(posterior)></path>
                    }
                }).collect_view()}
                {move || {
                    let ranked = ranked_posteriors(posterior_state(inputs()));
                    let leader = ranked.first().copied();
                    view! {
                        <text class="story-marker-label" x="394" y="54">
                            {leader.map(|posterior| format!("leader: {}", posterior.variant.name)).unwrap_or_else(|| "leader: none".to_string())}
                        </text>
                    }
                }}
                {move || posterior_state(inputs()).into_iter().map(|posterior| {
                    view! {
                        <g>
                            <line class="thompson-mean-line" x1=format!("{:.1}", thompson_x(posterior.mean)) y1="34" x2=format!("{:.1}", thompson_x(posterior.mean)) y2="254"></line>
                            <circle class="whittle-dot is-active" cx=format!("{:.1}", thompson_x(posterior.mean)) cy="252" r="5"></circle>
                            <text class="decision-line-label" x=format!("{:.1}", thompson_x(posterior.mean) + 7.0) y="274">{posterior.variant.name}</text>
                        </g>
                    }
                }).collect_view()}
                {move || trace_points(inputs()).into_iter().map(|(x, y, class_name)| {
                    view! { <circle class=class_name cx=format!("{:.1}", x) cy=format!("{:.1}", y) r="2.8"></circle> }
                }).collect_view()}
                <text class="phase-label phase-label-left" x="22" y="152" transform="rotate(-90 22 152)">"posterior density"</text>
                <text class="phase-label phase-label-bottom" x="250" y="306">"conversion probability"</text>
                <text class="phase-tick" x="40" y="274">"0%"</text>
                <text class="phase-tick" x="548" y="274">"18%"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let posterior = posterior_state(inputs());
                    let avg_uncertainty = posterior.iter().map(|item| item.uncertainty).sum::<f32>() / posterior.len() as f32;
                    view! {
                        <>
                            <span><strong>{format!("{:.3}", avg_uncertainty)}</strong>" mean posterior uncertainty"</span>
                            <span><strong>{format!("{:.1}%", allocation_entropy(&posterior) * 100.0 / 1.39)}</strong>" exploration entropy"</span>
                            <span><strong>{format!("{:.0}", inputs().traffic)}</strong>" traffic allocated per day"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

fn variants() -> Vec<Variant> {
    vec![
        Variant {
            name: "Control",
            observed_success: 112.0,
            observed_failure: 1688.0,
            lift_bias: 0.00,
            class_name: "decision-path decision-blue",
        },
        Variant {
            name: "Offer A",
            observed_success: 129.0,
            observed_failure: 1621.0,
            lift_bias: 0.55,
            class_name: "decision-path decision-green",
        },
        Variant {
            name: "Offer B",
            observed_success: 118.0,
            observed_failure: 1542.0,
            lift_bias: 0.82,
            class_name: "decision-path decision-red",
        },
        Variant {
            name: "Holdout",
            observed_success: 74.0,
            observed_failure: 1186.0,
            lift_bias: -0.15,
            class_name: "decision-path decision-amber",
        },
    ]
}

fn posterior_state(inputs: ThompsonInputs) -> Vec<Posterior> {
    let prior_alpha = 2.0 + inputs.prior_strength * 0.065;
    let prior_beta = 34.0 + inputs.prior_strength * 0.92;
    let mut raw = variants()
        .into_iter()
        .map(|variant| {
            let synthetic_lift = inputs.lift * variant.lift_bias;
            let alpha = prior_alpha + variant.observed_success + synthetic_lift.max(-18.0);
            let beta = (prior_beta + variant.observed_failure - synthetic_lift.min(32.0)).max(10.0);
            let mean = alpha / (alpha + beta);
            let uncertainty =
                ((alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0))).sqrt();
            let score = mean + uncertainty * (inputs.exploration / 100.0) * 2.8;
            (variant, alpha, beta, mean, uncertainty, score)
        })
        .collect::<Vec<_>>();
    let score_sum = raw.iter().map(|item| item.5).sum::<f32>().max(0.001);
    raw.drain(..)
        .map(
            |(variant, alpha, beta, mean, uncertainty, score)| Posterior {
                variant,
                alpha,
                beta,
                mean,
                uncertainty,
                allocation: score / score_sum,
            },
        )
        .collect()
}

fn ranked_posteriors(mut posteriors: Vec<Posterior>) -> Vec<Posterior> {
    posteriors.sort_by(|a, b| {
        b.mean
            .partial_cmp(&a.mean)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    posteriors
}

fn density_path(posterior: Posterior) -> String {
    let mode =
        ((posterior.alpha - 1.0) / (posterior.alpha + posterior.beta - 2.0)).clamp(0.001, 0.179);
    let peak = beta_log_shape(mode, posterior.alpha, posterior.beta);
    let mut d = String::new();
    for step in 0..=80 {
        let rate = 0.001 + step as f32 / 80.0 * 0.179;
        let density = (beta_log_shape(rate, posterior.alpha, posterior.beta) - peak)
            .clamp(-50.0, 0.0)
            .exp();
        let x = thompson_x(rate);
        let y = 254.0 - density.clamp(0.0, 1.0) * 210.0;
        if step == 0 {
            d.push_str(&format!("M {:.1} {:.1}", x, y));
        } else {
            d.push_str(&format!(" L {:.1} {:.1}", x, y));
        }
    }
    d
}

fn beta_log_shape(x: f32, alpha: f32, beta: f32) -> f32 {
    (alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln()
}

fn trace_points(inputs: ThompsonInputs) -> Vec<(f32, f32, &'static str)> {
    posterior_state(inputs)
        .into_iter()
        .enumerate()
        .flat_map(|(idx, posterior)| {
            (0..12).map(move |round| {
                let wobble =
                    ((round as f32 * 1.7 + idx as f32 * 2.3).sin()) * posterior.uncertainty * 2.4;
                let draw = (posterior.mean + wobble).clamp(0.001, 0.18);
                let y = 246.0 - (round as f32 % 6.0) * 8.0 - idx as f32 * 2.0;
                (thompson_x(draw), y, "thompson-trace")
            })
        })
        .collect()
}

fn thompson_x(rate: f32) -> f32 {
    46.0 + (rate / 0.18).clamp(0.0, 1.0) * 520.0
}

fn allocation_entropy(posteriors: &[Posterior]) -> f32 {
    posteriors
        .iter()
        .filter(|posterior| posterior.allocation > 0.0001)
        .map(|posterior| -posterior.allocation * posterior.allocation.ln())
        .sum()
}

fn thompson_regimes(
    inputs: ThompsonInputs,
    posteriors: Vec<Posterior>,
) -> Vec<(&'static str, &'static str, f32, &'static str, String)> {
    let max_mean = posteriors
        .iter()
        .map(|item| item.mean)
        .fold(0.0_f32, f32::max);
    let min_mean = posteriors
        .iter()
        .map(|item| item.mean)
        .fold(1.0_f32, f32::min);
    let separation = ((max_mean - min_mean) / 0.035 * 100.0).clamp(0.0, 100.0);
    let uncertainty = (posteriors.iter().map(|item| item.uncertainty).sum::<f32>()
        / posteriors.len() as f32
        / 0.009
        * 100.0)
        .clamp(0.0, 100.0);
    let entropy = (allocation_entropy(&posteriors) / 1.39 * 100.0).clamp(0.0, 100.0);
    let lift = ((inputs.lift + 20.0) / 80.0 * 100.0).clamp(0.0, 100.0);

    vec![
        (
            "Separation",
            if separation > 54.0 {
                "Winner emerging"
            } else {
                "Posterior overlap"
            },
            separation,
            "regime-card regime-green",
            format!(
                "Best-minus-worst mean gap is {:.2} points.",
                (max_mean - min_mean) * 100.0
            ),
        ),
        (
            "Uncertainty",
            if uncertainty > 44.0 {
                "Still learning"
            } else {
                "Posterior tight"
            },
            uncertainty,
            "regime-card regime-blue",
            format!(
                "Average posterior sigma is {:.3}.",
                uncertainty / 100.0 * 0.009
            ),
        ),
        (
            "Entropy",
            if entropy > 70.0 {
                "Exploratory traffic"
            } else {
                "Exploit skew"
            },
            entropy,
            "regime-card regime-amber",
            format!("Allocation entropy scores {:.1}.", entropy),
        ),
        (
            "Lift prior",
            if inputs.lift > 12.0 {
                "Treatment favored"
            } else {
                "Neutral prior"
            },
            lift,
            "regime-card regime-red",
            format!("Treatment lift lever is {:.0}%.", inputs.lift),
        ),
    ]
}
