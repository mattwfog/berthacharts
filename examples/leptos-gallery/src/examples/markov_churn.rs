//! Interactive Markov churn model with transition, cohort, and steady-state views.

use leptos::prelude::*;

use crate::dom_events::event_target_value_as_f32;

#[derive(Debug, Clone, Copy, PartialEq)]
struct MarkovInputs {
    acquisition_quality: f32,
    intervention: f32,
    price_pressure: f32,
    winback: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MarkovState {
    inputs: MarkovInputs,
    matrix: [[f32; 3]; 3],
    steady: [f32; 3],
    month_12_retention: f32,
    expected_ltv: f32,
    half_life: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TransitionCell {
    from: &'static str,
    to: &'static str,
    probability: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct CohortPath {
    label: &'static str,
    d: String,
    class_name: &'static str,
    terminal: f32,
}

#[component]
pub fn View() -> impl IntoView {
    let acquisition_quality = RwSignal::new(62.0_f32);
    let intervention = RwSignal::new(44.0_f32);
    let price_pressure = RwSignal::new(28.0_f32);
    let winback = RwSignal::new(18.0_f32);

    let inputs = move || MarkovInputs {
        acquisition_quality: acquisition_quality.get(),
        intervention: intervention.get(),
        price_pressure: price_pressure.get(),
        winback: winback.get(),
    };
    let state = move || markov_state(inputs());

    let active_label = move || format!("{:.1}%", state().steady[0] * 100.0);
    let risk_label = move || format!("{:.1}%", state().steady[1] * 100.0);
    let churn_label = move || format!("{:.1}%", state().steady[2] * 100.0);
    let retention_label = move || format!("{:.1}%", state().month_12_retention * 100.0);
    let ltv_label = move || format!("${:.0}", state().expected_ltv);

    view! {
        <section id="markov-churn" class="example game-theory decision-example">
            <div class="example-head">
                <div>
                    <h2>"Markov Churn State Machine"</h2>
                    <p>
                        "A restless customer base with active, at-risk, and churned states, shown as a transition heatmap, cohort trajectory, and long-run stationary mix."
                    </p>
                </div>
                <div class="stat-strip game-stat-strip">
                    <span><strong>{active_label}</strong>" steady active"</span>
                    <span><strong>{risk_label}</strong>" steady at risk"</span>
                    <span><strong>{churn_label}</strong>" steady churned"</span>
                    <span><strong>{retention_label}</strong>" month 12 retained"</span>
                    <span><strong>{ltv_label}</strong>" expected LTV"</span>
                </div>
            </div>

            <div class="game-layout">
                <aside class="game-controls" aria-label="Markov churn controls">
                    <div class="game-control-head">
                        <h3>"Transition Levers"</h3>
                        <span>"Tune acquisition, lifecycle pressure, retention treatment, and reactivation."</span>
                    </div>
                    <DecisionSlider label="Acquisition quality" suffix="%" min=0.0 max=100.0 step=1.0 value=acquisition_quality />
                    <DecisionSlider label="Retention intervention" suffix="%" min=0.0 max=100.0 step=1.0 value=intervention />
                    <DecisionSlider label="Price pressure" suffix="%" min=0.0 max=100.0 step=1.0 value=price_pressure />
                    <DecisionSlider label="Winback motion" suffix="%" min=0.0 max=100.0 step=1.0 value=winback />

                    <div class="game-notes">
                        <div>
                            <strong>"Rows sum to one"</strong>
                            <span>"Each row is a one-month transition from the current state."</span>
                        </div>
                        <div>
                            <strong>"Stationary mix"</strong>
                            <span>"Power iteration estimates the long-run customer-state distribution."</span>
                        </div>
                    </div>
                </aside>

                <div class="game-main">
                    <MarkovStory state=state />

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Transition Matrix Heatmap"</h3>
                            <span>"Higher intensity means more probability mass moving from row state to column state next month."</span>
                        </div>
                        <div class="decision-matrix markov-matrix" aria-label="Markov churn transition matrix">
                            <div class="decision-corner">"From \\ To"</div>
                            <div class="decision-axis">"Active"</div>
                            <div class="decision-axis">"At risk"</div>
                            <div class="decision-axis">"Churned"</div>
                            {move || transition_rows(state()).into_iter().map(|(label, cells)| {
                                view! {
                                    <>
                                        <div class="decision-row-axis">{label}</div>
                                        {cells.into_iter().map(|cell| {
                                            view! {
                                                <article class="decision-cell" style=format!("--heat:{:.3};", cell.probability)>
                                                    <span>{cell.from}</span>
                                                    <em>{cell.to}</em>
                                                    <strong>{format!("{:.1}%", cell.probability * 100.0)}</strong>
                                                </article>
                                            }
                                        }).collect_view()}
                                    </>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Cohort Probability Flow"</h3>
                            <span>"A cohort starts active and diffuses across states as the transition matrix compounds."</span>
                        </div>
                        <MarkovCohort state=state />
                    </div>

                    <div class="regime-panel">
                        <div class="game-section-head">
                            <h3>"Churn Regime Diagnostics"</h3>
                            <span>"Half-life, leakage, reactivation, and treatment leverage derived from the current matrix."</span>
                        </div>
                        <div class="regime-grid">
                            {move || markov_regimes(state()).into_iter().map(|cell| {
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
fn MarkovStory<F>(state: F) -> impl IntoView
where
    F: Fn() -> MarkovState + Copy + Send + Sync + 'static,
{
    view! {
        <div class="story-panel markov-story-panel">
            <div class="game-section-head">
                <h3>"What the churn engine is doing"</h3>
                <span>"Read left to right: durable customers, the at-risk fork, then the long-run pool the business settles into."</span>
            </div>
            {move || {
                let markov = state();
                let active_hold = markov.matrix[0][0] * 100.0;
                let risk_rescue = markov.matrix[1][0] * 100.0;
                let risk_churn = markov.matrix[1][2] * 100.0;
                let sink = markov.steady[2] * 100.0;
                let intervention_gap = risk_rescue - risk_churn;
                let fork_class = if intervention_gap >= 0.0 { "story-card story-green" } else { "story-card story-red" };
                let sink_class = if sink > 32.0 { "story-card story-red" } else { "story-card story-green" };
                view! {
                    <div class="story-flow">
                        <article class="story-card story-blue" style=format!("--story-share:{}%;", active_hold)>
                            <span>"1. Hold the base"</span>
                            <strong>{format!("{:.1}% stay active", active_hold)}</strong>
                            <em>{format!("Acquisition quality keeps the first-month leak to {:.1}%.", (1.0 - markov.matrix[0][0]) * 100.0)}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"then"</div>
                        <article class=fork_class style=format!("--story-share:{}%;", risk_rescue.max(risk_churn).clamp(0.0, 100.0))>
                            <span>"2. Fight the fork"</span>
                            <strong>{if intervention_gap >= 0.0 { "Rescue beats churn" } else { "Churn beats rescue" }}</strong>
                            <em>{format!("{:.1}% of at-risk customers recover; {:.1}% churn.", risk_rescue, risk_churn)}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"so"</div>
                        <article class=sink_class style=format!("--story-share:{}%;", (100.0 - sink).clamp(0.0, 100.0))>
                            <span>"3. Long-run destination"</span>
                            <strong>{format!("{:.1}% churned steady pool", sink)}</strong>
                            <em>{format!("The system settles at {:.1}% retained with ${:.0} modeled LTV.", (markov.steady[0] + markov.steady[1]) * 100.0, markov.expected_ltv)}</em>
                            <i><b></b></i>
                        </article>
                    </div>
                }
            }}
        </div>
    }
}

#[component]
fn DecisionSlider(
    #[prop(into)] label: String,
    #[prop(default = "")] prefix: &'static str,
    #[prop(default = "")] suffix: &'static str,
    min: f32,
    max: f32,
    step: f32,
    value: RwSignal<f32>,
) -> impl IntoView {
    let value_label = move || format!("{}{:.0}{}", prefix, value.get(), suffix);

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
fn MarkovCohort<F>(state: F) -> impl IntoView
where
    F: Fn() -> MarkovState + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-plot decision-svg-panel">
            <svg class="phase-svg" viewBox="0 0 620 320" role="img" aria-label="Markov churn cohort flow">
                <rect class="story-zone story-zone-red" x="42" y="204" width="532" height="44" rx="8"></rect>
                <rect class="story-zone story-zone-green" x="42" y="28" width="532" height="132" rx="8"></rect>
                <rect class="phase-frame" x="42" y="28" width="532" height="220" rx="8"></rect>
                <g class="phase-grid">
                    <line x1="42" y1="72" x2="574" y2="72"></line>
                    <line x1="42" y1="116" x2="574" y2="116"></line>
                    <line x1="42" y1="160" x2="574" y2="160"></line>
                    <line x1="42" y1="204" x2="574" y2="204"></line>
                    <line x1="174" y1="28" x2="174" y2="248"></line>
                    <line x1="308" y1="28" x2="308" y2="248"></line>
                    <line x1="442" y1="28" x2="442" y2="248"></line>
                </g>
                {move || cohort_paths(state()).into_iter().map(|path| {
                    view! { <path class=path.class_name d=path.d></path> }
                }).collect_view()}
                {move || {
                    let markov = state();
                    let x = 42.0 + 6.0 * (532.0 / 12.0);
                    let retained = cohort_series(markov.matrix)[6][0] + cohort_series(markov.matrix)[6][1];
                    view! {
                        <g>
                            <line class="story-marker-line" x1=format!("{:.1}", x) y1="28" x2=format!("{:.1}", x) y2="248"></line>
                            <text class="story-marker-label" x=format!("{:.1}", x + 8.0) y="52">{format!("month 6 retained {:.0}%", retained * 100.0)}</text>
                        </g>
                    }
                }}
                {move || cohort_paths(state()).into_iter().enumerate().map(|(idx, path)| {
                    let label_y = match idx {
                        1 => cohort_y(path.terminal) + 24.0,
                        2 => cohort_y(path.terminal) - 12.0,
                        _ => cohort_y(path.terminal),
                    };
                    view! {
                        <text class="decision-line-label" x="560" y=format!("{:.1}", label_y) text-anchor="end">{path.label}</text>
                    }
                }).collect_view()}
                <text class="phase-label phase-label-left" x="22" y="144" transform="rotate(-90 22 144)">"cohort share"</text>
                <text class="phase-label phase-label-bottom" x="270" y="294">"months since signup"</text>
                <text class="phase-tick" x="36" y="268">"0"</text>
                <text class="phase-tick" x="566" y="268">"12"</text>
                <text class="phase-tick" x="18" y="252">"0"</text>
                <text class="phase-tick" x="14" y="34">"100%"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let markov = state();
                    view! {
                        <>
                            <span><strong>{format!("{:.1}", markov.half_life)}</strong>" month retention half-life"</span>
                            <span><strong>{format!("{:.1}%", markov.matrix[1][2] * 100.0)}</strong>" at-risk to churn leak"</span>
                            <span><strong>{format!("{:.1}%", markov.matrix[2][0] * 100.0)}</strong>" churn to active winback"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

fn markov_state(inputs: MarkovInputs) -> MarkovState {
    let quality = inputs.acquisition_quality / 100.0;
    let intervention = inputs.intervention / 100.0;
    let pressure = inputs.price_pressure / 100.0;
    let winback = inputs.winback / 100.0;

    let active_to_risk =
        (0.10 + pressure * 0.18 - quality * 0.07 - intervention * 0.05).clamp(0.025, 0.38);
    let active_to_churn =
        (0.018 + pressure * 0.09 - quality * 0.035 - intervention * 0.025).clamp(0.006, 0.22);
    let risk_to_active =
        (0.18 + intervention * 0.38 + quality * 0.08 - pressure * 0.06).clamp(0.04, 0.70);
    let risk_to_churn =
        (0.19 + pressure * 0.28 - intervention * 0.18 - quality * 0.05).clamp(0.035, 0.62);
    let churn_to_active = (0.018 + winback * 0.19 + intervention * 0.035).clamp(0.004, 0.31);
    let churn_to_risk = (0.012 + winback * 0.10).clamp(0.002, 0.18);

    let matrix = [
        [
            1.0 - active_to_risk - active_to_churn,
            active_to_risk,
            active_to_churn,
        ],
        [
            risk_to_active,
            1.0 - risk_to_active - risk_to_churn,
            risk_to_churn,
        ],
        [
            churn_to_active,
            churn_to_risk,
            1.0 - churn_to_active - churn_to_risk,
        ],
    ];
    let steady = stationary(matrix);
    let retention = cohort_series(matrix)[12][0] + cohort_series(matrix)[12][1];
    let expected_ltv = cohort_series(matrix)
        .iter()
        .enumerate()
        .map(|(month, share)| {
            let discount = 1.0 / (1.0 + 0.012 * month as f32);
            (share[0] * 92.0 + share[1] * 48.0 + share[2] * 7.0) * discount
        })
        .sum::<f32>();
    let half_life = cohort_series(matrix)
        .iter()
        .position(|share| share[0] + share[1] <= 0.5)
        .map(|month| month as f32)
        .unwrap_or(18.0);

    MarkovState {
        inputs,
        matrix,
        steady,
        month_12_retention: retention,
        expected_ltv,
        half_life,
    }
}

fn stationary(matrix: [[f32; 3]; 3]) -> [f32; 3] {
    let mut state = [0.68, 0.22, 0.10];
    for _ in 0..80 {
        state = step(state, matrix);
    }
    state
}

fn cohort_series(matrix: [[f32; 3]; 3]) -> Vec<[f32; 3]> {
    let mut state = [1.0, 0.0, 0.0];
    let mut series = vec![state];
    for _ in 0..18 {
        state = step(state, matrix);
        series.push(state);
    }
    series
}

fn step(state: [f32; 3], matrix: [[f32; 3]; 3]) -> [f32; 3] {
    [
        state[0] * matrix[0][0] + state[1] * matrix[1][0] + state[2] * matrix[2][0],
        state[0] * matrix[0][1] + state[1] * matrix[1][1] + state[2] * matrix[2][1],
        state[0] * matrix[0][2] + state[1] * matrix[1][2] + state[2] * matrix[2][2],
    ]
}

fn transition_rows(markov: MarkovState) -> Vec<(&'static str, Vec<TransitionCell>)> {
    let labels = ["Active", "At risk", "Churned"];
    (0..3)
        .map(|from| {
            (
                labels[from],
                (0..3)
                    .map(|to| TransitionCell {
                        from: labels[from],
                        to: labels[to],
                        probability: markov.matrix[from][to],
                    })
                    .collect(),
            )
        })
        .collect()
}

fn cohort_paths(markov: MarkovState) -> Vec<CohortPath> {
    let series = cohort_series(markov.matrix);
    [
        ("Active", "decision-path decision-blue", 0),
        ("At risk", "decision-path decision-amber", 1),
        ("Churned", "decision-path decision-red", 2),
    ]
    .into_iter()
    .map(|(label, class_name, idx)| {
        let mut d = String::new();
        for (month, share) in series.iter().take(13).enumerate() {
            let x = 42.0 + month as f32 * (532.0 / 12.0);
            let y = cohort_y(share[idx]);
            if month == 0 {
                d.push_str(&format!("M {:.1} {:.1}", x, y));
            } else {
                d.push_str(&format!(" L {:.1} {:.1}", x, y));
            }
        }
        CohortPath {
            label,
            d,
            class_name,
            terminal: series[12][idx],
        }
    })
    .collect()
}

fn cohort_y(value: f32) -> f32 {
    248.0 - value.clamp(0.0, 1.0) * 220.0
}

fn markov_regimes(
    markov: MarkovState,
) -> Vec<(&'static str, &'static str, f32, &'static str, String)> {
    let leakage = (markov.matrix[0][2] + markov.matrix[1][2]) * 50.0;
    let rescue = (markov.matrix[1][0] + markov.matrix[2][0]) * 80.0;
    let pressure =
        (markov.inputs.price_pressure + (1.0 - markov.matrix[0][0]) * 100.0).clamp(0.0, 100.0);
    let durability = (markov.month_12_retention * 100.0).clamp(0.0, 100.0);

    vec![
        (
            "Durability",
            if durability > 64.0 {
                "Sticky cohort"
            } else {
                "Fragile cohort"
            },
            durability,
            "regime-card regime-green",
            format!(
                "Month-12 retained share is {:.1}%.",
                markov.month_12_retention * 100.0
            ),
        ),
        (
            "Leakage",
            if leakage > 24.0 {
                "Churn sink open"
            } else {
                "Leak contained"
            },
            leakage,
            "regime-card regime-red",
            format!(
                "Direct churn pressure totals {:.1} transition points.",
                leakage
            ),
        ),
        (
            "Rescue",
            if rescue > 34.0 {
                "Recovery engine live"
            } else {
                "Recovery underpowered"
            },
            rescue,
            "regime-card regime-blue",
            format!("At-risk and churned rescue paths sum to {:.1}.", rescue),
        ),
        (
            "Pressure",
            if pressure > 56.0 {
                "Volatile base"
            } else {
                "Controlled base"
            },
            pressure,
            "regime-card regime-amber",
            format!("Price plus active-state drift scores {:.1}.", pressure),
        ),
    ]
}
