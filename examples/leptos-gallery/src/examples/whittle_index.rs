//! Interactive Whittle index chart for restless retention bandits.

use leptos::prelude::*;

use crate::dom_events::event_target_value_as_f32;

#[derive(Debug, Clone, Copy, PartialEq)]
struct WhittleInputs {
    budget: f32,
    subsidy: f32,
    decay: f32,
    rescue_lift: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Arm {
    name: &'static str,
    belief: f32,
    reward: f32,
    volatility: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ArmScore {
    arm: Arm,
    index: f32,
    active: bool,
    rank: usize,
}

#[component]
pub fn View() -> impl IntoView {
    let budget = RwSignal::new(2.0_f32);
    let subsidy = RwSignal::new(18.0_f32);
    let decay = RwSignal::new(42.0_f32);
    let rescue_lift = RwSignal::new(58.0_f32);

    let inputs = move || WhittleInputs {
        budget: budget.get(),
        subsidy: subsidy.get(),
        decay: decay.get(),
        rescue_lift: rescue_lift.get(),
    };
    let scores = move || arm_scores(inputs());
    let threshold = move || active_threshold(scores());
    let active_count = move || {
        scores()
            .iter()
            .filter(|score| score.active)
            .count()
            .to_string()
    };
    let top_arm = move || {
        scores()
            .first()
            .map(|score| score.arm.name)
            .unwrap_or("None")
    };
    let threshold_label = move || format!("{:.1}", threshold());
    let spread_label = move || {
        let ranked = scores();
        let top = ranked.first().map(|score| score.index).unwrap_or_default();
        let bottom = ranked.last().map(|score| score.index).unwrap_or_default();
        format!("{:.1}", top - bottom)
    };

    view! {
        <section id="whittle-index" class="example game-theory decision-example">
            <div class="example-head">
                <div>
                    <h2>"Whittle Index Dispatch Surface"</h2>
                    <p>
                        "A restless-bandit allocation view that ranks customer states by marginal activation value, then draws the policy threshold across belief curves."
                    </p>
                </div>
                <div class="stat-strip game-stat-strip">
                    <span><strong>{active_count}</strong>" arms activated"</span>
                    <span><strong>{top_arm}</strong>" top index"</span>
                    <span><strong>{threshold_label}</strong>" policy threshold"</span>
                    <span><strong>{spread_label}</strong>" index spread"</span>
                </div>
            </div>

            <div class="game-layout">
                <aside class="game-controls" aria-label="Whittle index controls">
                    <div class="game-control-head">
                        <h3>"Restless Bandit Levers"</h3>
                        <span>"Set capacity and how quickly untreated states decay."</span>
                    </div>
                    <WhittleSlider label="Activation budget" min=1.0 max=5.0 step=1.0 value=budget />
                    <WhittleSlider label="Passive subsidy" min=0.0 max=40.0 step=1.0 value=subsidy />
                    <WhittleSlider label="State decay" suffix="%" min=0.0 max=100.0 step=1.0 value=decay />
                    <WhittleSlider label="Rescue lift" suffix="%" min=0.0 max=100.0 step=1.0 value=rescue_lift />

                    <div class="game-notes">
                        <div>
                            <strong>"Index policy"</strong>
                            <span>"Activate arms whose Whittle index clears the capacity threshold."</span>
                        </div>
                        <div>
                            <strong>"Belief axis"</strong>
                            <span>"Higher belief means higher probability the customer is recoverable."</span>
                        </div>
                    </div>
                </aside>

                <div class="game-main">
                    <WhittleStory scores=scores threshold=threshold />

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Index Curves by State Belief"</h3>
                            <span>"Each curve shows the marginal value of activating an arm as belief changes."</span>
                        </div>
                        <WhittleSurface inputs=inputs threshold=threshold />
                    </div>

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Ranked Dispatch Board"</h3>
                            <span>"The top arms consume budget; inactive arms are left to passive dynamics."</span>
                        </div>
                        <div class="whittle-board">
                            {move || scores().into_iter().map(|score| {
                                view! {
                                    <article class=if score.active { "whittle-arm is-active" } else { "whittle-arm" }>
                                        <span>{format!("#{}", score.rank)}</span>
                                        <strong>{score.arm.name}</strong>
                                        <em>{format!("belief {:.0}% / volatility {:.0}%", score.arm.belief * 100.0, score.arm.volatility * 100.0)}</em>
                                        <i style=format!("--bar:{}%;", (score.index / 92.0 * 100.0).clamp(0.0, 100.0))><b></b></i>
                                        <small>{format!("{:.1} index", score.index)}</small>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="regime-panel">
                        <div class="game-section-head">
                            <h3>"Policy Regime Diagnostics"</h3>
                            <span>"Capacity, subsidy, decay, and rescue lift create the operating mode."</span>
                        </div>
                        <div class="regime-grid">
                            {move || whittle_regimes(inputs(), scores()).into_iter().map(|cell| {
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
fn WhittleStory<F, G>(scores: F, threshold: G) -> impl IntoView
where
    F: Fn() -> Vec<ArmScore> + Copy + Send + Sync + 'static,
    G: Fn() -> f32 + Copy + Send + Sync + 'static,
{
    view! {
        <div class="story-panel whittle-story-panel">
            <div class="game-section-head">
                <h3>"Who gets saved, and who waits"</h3>
                <span>"The budget creates a hard frontier: funded arms sit above the line, the first unfunded arm is the opportunity cost."</span>
            </div>
            {move || {
                let ranked = scores();
                let funded = ranked
                    .iter()
                    .filter(|score| score.active)
                    .map(|score| score.arm.name)
                    .collect::<Vec<_>>()
                    .join(" + ");
                let shadow = ranked
                    .iter()
                    .find(|score| !score.active)
                    .copied();
                let shadow_gap = shadow.map(|score| threshold() - score.index).unwrap_or(0.0).max(0.0);
                let top = ranked.first().copied();
                view! {
                    <div class="story-flow">
                        <article class="story-card story-green" style=format!("--story-share:{}%;", threshold().clamp(0.0, 92.0) / 92.0 * 100.0)>
                            <span>"Fund now"</span>
                            <strong>{if funded.is_empty() { "No arm clears budget".to_string() } else { funded }}</strong>
                            <em>{format!("Lowest funded index is {:.1}; these states lose value fastest if untreated.", threshold())}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"border"</div>
                        <article class="story-card story-amber" style=format!("--story-share:{}%;", (100.0 - shadow_gap * 2.2).clamp(4.0, 100.0))>
                            <span>"Shadow price"</span>
                            <strong>{shadow.map(|score| score.arm.name).unwrap_or("No waitlist")}</strong>
                            <em>{format!("The next arm is {:.1} index points below the cutline.", shadow_gap)}</em>
                            <i><b></b></i>
                        </article>
                        <div class="story-link">"because"</div>
                        <article class="story-card story-blue" style=format!("--story-share:{}%;", top.map(|score| score.index / 92.0 * 100.0).unwrap_or(0.0))>
                            <span>"Value signal"</span>
                            <strong>{top.map(|score| score.arm.name).unwrap_or("No signal")}</strong>
                            <em>{top.map(|score| format!("High belief plus volatility creates a {:.1} index.", score.index)).unwrap_or_else(|| "No arms available.".to_string())}</em>
                            <i><b></b></i>
                        </article>
                    </div>
                }
            }}
        </div>
    }
}

#[component]
fn WhittleSlider(
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
fn WhittleSurface<F, G>(inputs: F, threshold: G) -> impl IntoView
where
    F: Fn() -> WhittleInputs + Copy + Send + Sync + 'static,
    G: Fn() -> f32 + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-plot decision-svg-panel">
            <svg class="phase-svg" viewBox="0 0 620 330" role="img" aria-label="Whittle index surface">
                <rect class="phase-frame" x="54" y="34" width="500" height="220" rx="8"></rect>
                <g class="phase-grid">
                    <line x1="54" y1="78" x2="554" y2="78"></line>
                    <line x1="54" y1="122" x2="554" y2="122"></line>
                    <line x1="54" y1="166" x2="554" y2="166"></line>
                    <line x1="54" y1="210" x2="554" y2="210"></line>
                    <line x1="154" y1="34" x2="154" y2="254"></line>
                    <line x1="254" y1="34" x2="254" y2="254"></line>
                    <line x1="354" y1="34" x2="354" y2="254"></line>
                    <line x1="454" y1="34" x2="454" y2="254"></line>
                </g>
                {move || {
                    let y = whittle_y(threshold());
                    view! {
                        <g>
                            <rect class="story-zone story-zone-green" x="54" y="34" width="500" height=format!("{:.1}", (y - 34.0).max(0.0)) rx="8"></rect>
                            <line class="best-response-line col-threshold" x1="54" y1=format!("{:.1}", y) x2="554" y2=format!("{:.1}", y)></line>
                            <text class="story-marker-label" x="64" y=format!("{:.1}", (y - 8.0).max(48.0))>"funded zone"</text>
                        </g>
                    }
                }}
                {move || base_arms().into_iter().map(|arm| {
                    view! { <path class=arm.class_name d=curve_path(arm, inputs())></path> }
                }).collect_view()}
                {move || arm_scores(inputs()).into_iter().map(|score| {
                    view! {
                        <g>
                            <circle
                                class=if score.active { "whittle-dot is-active" } else { "whittle-dot" }
                                cx=format!("{:.1}", whittle_x(score.arm.belief))
                                cy=format!("{:.1}", whittle_y(score.index))
                                r="6"
                            ></circle>
                            <text class="decision-line-label" x=format!("{:.1}", whittle_x(score.arm.belief) + 9.0) y=format!("{:.1}", whittle_y(score.index) - 7.0)>{score.arm.name}</text>
                        </g>
                    }
                }).collect_view()}
                <text class="phase-label phase-label-left" x="24" y="158" transform="rotate(-90 24 158)">"Whittle index"</text>
                <text class="phase-label phase-label-bottom" x="258" y="304">"recoverable belief"</text>
                <text class="phase-tick" x="50" y="274">"0"</text>
                <text class="phase-tick" x="546" y="274">"1"</text>
                <text class="phase-tick" x="24" y="258">"0"</text>
                <text class="phase-tick" x="20" y="40">"90"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let ranked = arm_scores(inputs());
                    let active_value = ranked.iter().filter(|score| score.active).map(|score| score.index).sum::<f32>();
                    view! {
                        <>
                            <span><strong>{format!("{:.1}", active_value)}</strong>" active index mass"</span>
                            <span><strong>{format!("{:.1}", threshold())}</strong>" lowest funded index"</span>
                            <span><strong>{format!("{:.0}%", inputs().decay)}</strong>" passive decay setting"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

fn base_arms() -> Vec<Arm> {
    vec![
        Arm {
            name: "VIP save",
            belief: 0.82,
            reward: 78.0,
            volatility: 0.18,
            class_name: "decision-path decision-green",
        },
        Arm {
            name: "Power user",
            belief: 0.68,
            reward: 64.0,
            volatility: 0.22,
            class_name: "decision-path decision-blue",
        },
        Arm {
            name: "At-risk core",
            belief: 0.54,
            reward: 70.0,
            volatility: 0.48,
            class_name: "decision-path decision-red",
        },
        Arm {
            name: "Dormant",
            belief: 0.34,
            reward: 46.0,
            volatility: 0.66,
            class_name: "decision-path decision-amber",
        },
        Arm {
            name: "New user",
            belief: 0.46,
            reward: 52.0,
            volatility: 0.35,
            class_name: "decision-path decision-purple",
        },
    ]
}

fn arm_scores(inputs: WhittleInputs) -> Vec<ArmScore> {
    let mut scored = base_arms()
        .into_iter()
        .map(|arm| {
            let index = whittle_index(arm, arm.belief, inputs);
            ArmScore {
                arm,
                index,
                active: false,
                rank: 0,
            }
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.index
            .partial_cmp(&a.index)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let budget = inputs.budget.round() as usize;
    for (idx, score) in scored.iter_mut().enumerate() {
        score.rank = idx + 1;
        score.active = idx < budget;
    }
    scored
}

fn active_threshold(scores: Vec<ArmScore>) -> f32 {
    scores
        .iter()
        .filter(|score| score.active)
        .last()
        .map(|score| score.index)
        .unwrap_or_default()
}

fn whittle_index(arm: Arm, belief: f32, inputs: WhittleInputs) -> f32 {
    let decay_penalty = arm.volatility * inputs.decay * 0.34;
    let rescue_bonus = belief * inputs.rescue_lift * (0.42 + arm.volatility * 0.36);
    let passive_value = inputs.subsidy * (1.0 - belief) * 0.55;
    (arm.reward * belief + rescue_bonus + decay_penalty - passive_value).clamp(0.0, 92.0)
}

fn curve_path(arm: Arm, inputs: WhittleInputs) -> String {
    let mut d = String::new();
    for step in 0..=30 {
        let belief = step as f32 / 30.0;
        let x = whittle_x(belief);
        let y = whittle_y(whittle_index(arm, belief, inputs));
        if step == 0 {
            d.push_str(&format!("M {:.1} {:.1}", x, y));
        } else {
            d.push_str(&format!(" L {:.1} {:.1}", x, y));
        }
    }
    d
}

fn whittle_x(belief: f32) -> f32 {
    54.0 + belief.clamp(0.0, 1.0) * 500.0
}

fn whittle_y(index: f32) -> f32 {
    254.0 - (index / 92.0).clamp(0.0, 1.0) * 220.0
}

fn whittle_regimes(
    inputs: WhittleInputs,
    scores: Vec<ArmScore>,
) -> Vec<(&'static str, &'static str, f32, &'static str, String)> {
    let capacity = inputs.budget / 5.0 * 100.0;
    let concentration = scores
        .first()
        .zip(scores.last())
        .map(|(top, bottom)| ((top.index - bottom.index) / 92.0 * 100.0).clamp(0.0, 100.0))
        .unwrap_or_default();
    let urgency = (inputs.decay * 0.62 + inputs.rescue_lift * 0.28).clamp(0.0, 100.0);
    let passivity = (inputs.subsidy / 40.0 * 100.0).clamp(0.0, 100.0);

    vec![
        (
            "Capacity",
            if capacity > 55.0 {
                "Broad activation"
            } else {
                "Scarce activation"
            },
            capacity,
            "regime-card regime-blue",
            format!(
                "{:.0} of 5 arms can be activated this period.",
                inputs.budget
            ),
        ),
        (
            "Urgency",
            if urgency > 56.0 {
                "Treat now"
            } else {
                "Can wait"
            },
            urgency,
            "regime-card regime-red",
            format!("Decay and rescue lift combine to {:.1}.", urgency),
        ),
        (
            "Separation",
            if concentration > 36.0 {
                "Clear ranking"
            } else {
                "Index tie zone"
            },
            concentration,
            "regime-card regime-green",
            format!(
                "Top-to-bottom spread is {:.1} index points.",
                concentration * 0.92
            ),
        ),
        (
            "Passivity",
            if passivity > 50.0 {
                "High passive subsidy"
            } else {
                "Low passive value"
            },
            passivity,
            "regime-card regime-amber",
            format!("Passive subsidy is set to {:.0}.", inputs.subsidy),
        ),
    ]
}
