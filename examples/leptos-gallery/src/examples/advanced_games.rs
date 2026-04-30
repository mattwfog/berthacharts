//! Sophisticated game-theory examples with narrative decision views.

use leptos::prelude::*;

use crate::dom_events::event_target_value_as_f32;

#[derive(Debug, Clone, Copy, PartialEq)]
struct AdvancedInputs {
    information_friction: f32,
    commitment_power: f32,
    force_asymmetry: f32,
    prize_skew: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SignalState {
    prior_quality: f32,
    signal_posterior: f32,
    no_signal_posterior: f32,
    invest_threshold: f32,
    separating_score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StackelbergState {
    leader_quantity: f32,
    follower_quantity: f32,
    cournot_quantity: f32,
    leader_profit: f32,
    cournot_profit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Front {
    name: &'static str,
    value: f32,
    blue: f32,
    red: f32,
    edge: f32,
}

#[component]
pub fn View() -> impl IntoView {
    let information_friction = RwSignal::new(54.0_f32);
    let commitment_power = RwSignal::new(68.0_f32);
    let force_asymmetry = RwSignal::new(6.0_f32);
    let prize_skew = RwSignal::new(46.0_f32);

    let inputs = move || AdvancedInputs {
        information_friction: information_friction.get(),
        commitment_power: commitment_power.get(),
        force_asymmetry: force_asymmetry.get(),
        prize_skew: prize_skew.get(),
    };
    let signal = move || signal_state(inputs());
    let stackelberg = move || stackelberg_state(inputs());
    let fronts = move || blotto_fronts(inputs());

    let signal_label = move || {
        if signal().separating_score > 58.0 {
            "separating"
        } else if signal().separating_score > 42.0 {
            "semi-pooling"
        } else {
            "pooling"
        }
    };
    let commitment_label = move || {
        let stack = stackelberg();
        format!(
            "{:+.1}%",
            (stack.leader_profit / stack.cournot_profit - 1.0) * 100.0
        )
    };
    let swing_label = move || {
        let decisive = fronts()
            .iter()
            .filter(|front| front.edge.abs() < 10.0)
            .count();
        decisive.to_string()
    };
    let value_label = move || {
        format!(
            "{:.0}",
            fronts().iter().map(|front| front.value).sum::<f32>()
        )
    };

    view! {
        <section id="advanced-games" class="example game-theory decision-example advanced-games">
            <div class="example-head">
                <div>
                    <h2>"Advanced Game Theory Lab"</h2>
                    <p>
                        "Signaling, Stackelberg commitment, and Colonel Blotto allocation views that show how beliefs, credible moves, and contested fronts change strategic outcomes."
                    </p>
                </div>
                <div class="stat-strip game-stat-strip">
                    <span><strong>{signal_label}</strong>" signal regime"</span>
                    <span><strong>{commitment_label}</strong>" leader premium"</span>
                    <span><strong>{swing_label}</strong>" swing fronts"</span>
                    <span><strong>{value_label}</strong>" prize mass"</span>
                </div>
            </div>

            <div class="game-layout">
                <aside class="game-controls" aria-label="Advanced game theory controls">
                    <div class="game-control-head">
                        <h3>"Strategic Environment"</h3>
                        <span>"The same market conditions ripple through information, commitment, and allocation games."</span>
                    </div>
                    <AdvancedSlider label="Information friction" suffix="%" min=0.0 max=100.0 step=1.0 value=information_friction />
                    <AdvancedSlider label="Commitment power" suffix="%" min=0.0 max=100.0 step=1.0 value=commitment_power />
                    <AdvancedSlider label="Force asymmetry" suffix="%" min=-40.0 max=40.0 step=1.0 value=force_asymmetry />
                    <AdvancedSlider label="Prize skew" suffix="%" min=0.0 max=100.0 step=1.0 value=prize_skew />

                    <div class="game-notes">
                        <div>
                            <strong>"Read as sequence"</strong>
                            <span>"Players first shape beliefs, then commit, then fight across scarce fronts."</span>
                        </div>
                        <div>
                            <strong>"Sophisticated games"</strong>
                            <span>"Each panel exposes a different strategic failure mode: pooling, overcommitment, or thin allocation."</span>
                        </div>
                    </div>
                </aside>

                <div class="game-main">
                    <div class="story-panel">
                        <div class="game-section-head">
                            <h3>"Strategic story across games"</h3>
                            <span>"A richer strategic model asks what the rival believes, what they can credibly commit to, and where scarce resources actually decide the game."</span>
                        </div>
                        <div class="story-flow">
                            <article class="story-card story-blue" style=move || format!("--story-share:{}%;", signal().separating_score)>
                                <span>"Beliefs"</span>
                                <strong>{move || signal_label()}</strong>
                                <em>{move || format!("A costly signal moves posterior belief from {:.0}% to {:.0}%.", signal().prior_quality * 100.0, signal().signal_posterior * 100.0)}</em>
                                <i><b></b></i>
                            </article>
                            <div class="story-link">"then"</div>
                            <article class="story-card story-green" style=move || format!("--story-share:{}%;", (stackelberg().leader_profit / stackelberg().cournot_profit * 50.0).clamp(0.0, 100.0))>
                                <span>"Commitment"</span>
                                <strong>{move || format!("{:.0} / {:.0}", stackelberg().leader_quantity, stackelberg().follower_quantity)}</strong>
                                <em>"The leader chooses quantity first, forcing the follower onto its best-response curve."</em>
                                <i><b></b></i>
                            </article>
                            <div class="story-link">"then"</div>
                            <article class="story-card story-amber" style=move || format!("--story-share:{}%;", contested_share(&fronts()))>
                                <span>"Allocation"</span>
                                <strong>{move || format!("{} contested fronts", swing_label())}</strong>
                                <em>"The highest-value fronts are not always decisive; the story is where both sides are close enough to swing."</em>
                                <i><b></b></i>
                            </article>
                        </div>
                    </div>

                    <FrameworkPanel inputs=inputs signal=signal stackelberg=stackelberg fronts=fronts />

                    <div class="advanced-game-grid">
                        <SignalingPanel state=signal />
                        <StackelbergPanel state=stackelberg />
                        <BlottoPanel fronts=fronts />
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn FrameworkPanel<I, S, T, B>(inputs: I, signal: S, stackelberg: T, fronts: B) -> impl IntoView
where
    I: Fn() -> AdvancedInputs + Copy + Send + Sync + 'static,
    S: Fn() -> SignalState + Copy + Send + Sync + 'static,
    T: Fn() -> StackelbergState + Copy + Send + Sync + 'static,
    B: Fn() -> Vec<Front> + Copy + Send + Sync + 'static,
{
    view! {
        <div class="framework-panel">
            <div class="game-section-head">
                <h3>"Frameworks for using the games"</h3>
                <span>"These are reusable lenses: identify the strategic object, diagnose the failure mode, then choose the smallest rule change that moves equilibrium behavior."</span>
            </div>
            <div class="framework-grid">
                <article class="framework-card">
                    <span>"GIST frame"</span>
                    <strong>"Goal - Information - Sequence - Threats"</strong>
                    <em>"Use this before choosing a model. If private information dominates, use signaling. If timing dominates, use Stackelberg. If scarce resources dominate, use Blotto."</em>
                    <div class="framework-steps">
                        <b>"1 Goal"</b>
                        <b>"2 Info"</b>
                        <b>"3 Sequence"</b>
                        <b>"4 Threats"</b>
                    </div>
                </article>
                <article class="framework-card">
                    <span>"Information design"</span>
                    <strong>{move || information_recommendation(signal())}</strong>
                    <em>{move || format!("Current posterior gap is {:.0} points. Change signal cost, verification, or disclosure until the bad type cannot cheaply mimic the good type.", (signal().signal_posterior - signal().no_signal_posterior) * 100.0)}</em>
                    <FrameworkMeter value=move || signal().separating_score label="separation" />
                </article>
                <article class="framework-card">
                    <span>"Commitment design"</span>
                    <strong>{move || commitment_recommendation(stackelberg())}</strong>
                    <em>{move || format!("The leader premium is {:+.1}%. Use capacity, contracts, launch timing, or public roadmaps only when the commitment is observable and costly to reverse.", (stackelberg().leader_profit / stackelberg().cournot_profit - 1.0) * 100.0)}</em>
                    <FrameworkMeter value=move || ((stackelberg().leader_profit / stackelberg().cournot_profit - 1.0) * 420.0).clamp(0.0, 100.0) label="credibility" />
                </article>
                <article class="framework-card">
                    <span>"Contest design"</span>
                    <strong>{move || contest_recommendation(&fronts())}</strong>
                    <em>{move || format!("{} fronts are within 10 allocation points. Fund the swing fronts, not just the largest fronts, unless prize skew overwhelms contestability.", fronts().iter().filter(|front| front.edge.abs() < 10.0).count())}</em>
                    <FrameworkMeter value=move || contested_share(&fronts()) label="swing value" />
                </article>
            </div>
            <div class="framework-diagnostics">
                {move || {
                    let env = inputs();
                    let dominant = dominant_framework(signal(), stackelberg(), &fronts());
                    view! {
                        <>
                            <span><strong>{dominant}</strong>" dominant lens"</span>
                            <span><strong>{format!("{:.0}%", env.information_friction)}</strong>" information opacity"</span>
                            <span><strong>{format!("{:.0}%", env.commitment_power)}</strong>" commitment credibility"</span>
                            <span><strong>{format!("{:+.0}%", env.force_asymmetry)}</strong>" force asymmetry"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn FrameworkMeter<V>(value: V, label: &'static str) -> impl IntoView
where
    V: Fn() -> f32 + Copy + Send + Sync + 'static,
{
    view! {
        <i class="framework-meter" style=move || format!("--framework-share:{}%;", value().clamp(0.0, 100.0))>
            <b></b>
            <span>{label}</span>
        </i>
    }
}

#[component]
fn AdvancedSlider(
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
fn SignalingPanel<F>(state: F) -> impl IntoView
where
    F: Fn() -> SignalState + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-panel">
            <div class="game-section-head">
                <h3>"Bayesian Signaling Game"</h3>
                <span>"The receiver invests only when the observed signal pushes posterior belief past the action threshold."</span>
            </div>
            <svg class="advanced-svg" viewBox="0 0 560 260" role="img" aria-label="Bayesian signaling game belief map">
                <rect class="phase-frame" x="42" y="44" width="472" height="96" rx="8"></rect>
                <rect class="story-zone story-zone-red" x="42" y="44" width="236" height="96" rx="8"></rect>
                <rect class="story-zone story-zone-green" x="278" y="44" width="236" height="96" rx="8"></rect>
                <line class="phase-grid-line" x1="42" y1="92" x2="514" y2="92"></line>
                {move || {
                    let game = state();
                    let threshold_x = belief_x(game.invest_threshold);
                    view! {
                        <g>
                            <line class="best-response-line col-threshold" x1=format!("{:.1}", threshold_x) y1="36" x2=format!("{:.1}", threshold_x) y2="154"></line>
                            <text class="story-marker-label" x=format!("{:.1}", threshold_x + 8.0) y="38">"invest threshold"</text>
                            <BeliefDot label="prior" x=belief_x(game.prior_quality) y=92.0 class_name="belief-dot belief-prior" />
                            <BeliefDot label="signal" x=belief_x(game.signal_posterior) y=70.0 class_name="belief-dot belief-signal" />
                            <BeliefDot label="no signal" x=belief_x(game.no_signal_posterior) y=118.0 class_name="belief-dot belief-muted" />
                        </g>
                    }
                }}
                <text class="phase-tick" x="38" y="164">"0"</text>
                <text class="phase-tick" x="502" y="164">"1"</text>
                <text class="phase-label phase-label-bottom" x="218" y="190">"receiver posterior belief"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let game = state();
                    view! {
                        <>
                            <span><strong>{format!("{:.0}%", game.signal_posterior * 100.0)}</strong>" belief after signal"</span>
                            <span><strong>{format!("{:.0}%", game.no_signal_posterior * 100.0)}</strong>" belief without signal"</span>
                            <span><strong>{format!("{:.0}", game.separating_score)}</strong>" separation score"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn BeliefDot(label: &'static str, x: f32, y: f32, class_name: &'static str) -> impl IntoView {
    view! {
        <g>
            <circle class=class_name cx=format!("{:.1}", x) cy=format!("{:.1}", y) r="7"></circle>
            <text class="decision-line-label" x=format!("{:.1}", x + 10.0) y=format!("{:.1}", y - 8.0)>{label}</text>
        </g>
    }
}

#[component]
fn StackelbergPanel<F>(state: F) -> impl IntoView
where
    F: Fn() -> StackelbergState + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-panel">
            <div class="game-section-head">
                <h3>"Stackelberg Commitment Game"</h3>
                <span>"The leader moves first; the follower slides down the best response, creating a measurable commitment premium."</span>
            </div>
            <svg class="advanced-svg" viewBox="0 0 560 260" role="img" aria-label="Stackelberg commitment response chart">
                <rect class="phase-frame" x="54" y="30" width="438" height="156" rx="8"></rect>
                <g class="phase-grid">
                    <line x1="54" y1="82" x2="492" y2="82"></line>
                    <line x1="54" y1="134" x2="492" y2="134"></line>
                    <line x1="200" y1="30" x2="200" y2="186"></line>
                    <line x1="346" y1="30" x2="346" y2="186"></line>
                </g>
                <path class="decision-path decision-blue" d="M 54 34 L 492 186"></path>
                {move || {
                    let game = state();
                    let sx = stack_x(game.leader_quantity);
                    let sy = stack_y(game.follower_quantity);
                    let cx = stack_x(game.cournot_quantity);
                    let cy = stack_y(game.cournot_quantity);
                    view! {
                        <g>
                            <line class="story-marker-line" x1=format!("{:.1}", sx) y1="30" x2=format!("{:.1}", sx) y2=format!("{:.1}", sy)></line>
                            <circle class="whittle-dot is-active" cx=format!("{:.1}", sx) cy=format!("{:.1}", sy) r="7"></circle>
                            <text class="story-marker-label" x=format!("{:.1}", sx + 10.0) y=format!("{:.1}", sy - 10.0)>"Stackelberg"</text>
                            <circle class="whittle-dot" cx=format!("{:.1}", cx) cy=format!("{:.1}", cy) r="6"></circle>
                            <text class="decision-line-label" x=format!("{:.1}", cx + 10.0) y=format!("{:.1}", cy + 16.0)>"Cournot reference"</text>
                        </g>
                    }
                }}
                <text class="phase-label phase-label-left" x="24" y="118" transform="rotate(-90 24 118)">"follower quantity"</text>
                <text class="phase-label phase-label-bottom" x="226" y="230">"leader quantity"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let game = state();
                    view! {
                        <>
                            <span><strong>{format!("{:.1}", game.leader_quantity)}</strong>" committed output"</span>
                            <span><strong>{format!("{:.1}", game.follower_quantity)}</strong>" follower response"</span>
                            <span><strong>{format!("{:+.1}%", (game.leader_profit / game.cournot_profit - 1.0) * 100.0)}</strong>" profit premium"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn BlottoPanel<F>(fronts: F) -> impl IntoView
where
    F: Fn() -> Vec<Front> + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-panel">
            <div class="game-section-head">
                <h3>"Colonel Blotto Allocation Game"</h3>
                <span>"Resources are split across battlefields; high-value fronts matter only if the allocation gap is close."</span>
            </div>
            <div class="blotto-fronts">
                {move || fronts().into_iter().map(|front| {
                    let edge_class = if front.edge >= 12.0 {
                        "blotto-front is-blue"
                    } else if front.edge <= -12.0 {
                        "blotto-front is-red"
                    } else {
                        "blotto-front is-swing"
                    };
                    view! {
                        <article class=edge_class style=format!("--blue:{}%; --red:{}%; --value:{}%;", front.blue, front.red, front.value)>
                            <div>
                                <span>{front.name}</span>
                                <strong>{format!("{:+.1}", front.edge)}</strong>
                            </div>
                            <i class="blotto-bar-blue"><b></b></i>
                            <i class="blotto-bar-red"><b></b></i>
                            <em>{format!("{:.0} prize value", front.value)}</em>
                        </article>
                    }
                }).collect_view()}
            </div>
            <div class="phase-readout">
                {move || {
                    let current = fronts();
                    let blue_value = current.iter().filter(|front| front.edge > 0.0).map(|front| front.value).sum::<f32>().max(0.0);
                    let red_value = current.iter().filter(|front| front.edge < 0.0).map(|front| front.value).sum::<f32>().max(0.0);
                    let swing = current.iter().filter(|front| front.edge.abs() < 10.0).count();
                    view! {
                        <>
                            <span><strong>{format!("{:.0}", blue_value)}</strong>" blue controlled value"</span>
                            <span><strong>{format!("{:.0}", red_value)}</strong>" red controlled value"</span>
                            <span><strong>{swing.to_string()}</strong>" swing fronts"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

fn signal_state(inputs: AdvancedInputs) -> SignalState {
    let friction = inputs.information_friction / 100.0;
    let prior_quality =
        (0.42 + inputs.prize_skew * 0.0016 - inputs.force_asymmetry * 0.001).clamp(0.18, 0.82);
    let high_signal_rate = (0.56 + friction * 0.33).clamp(0.52, 0.94);
    let low_signal_rate = (0.44 - friction * 0.24).clamp(0.08, 0.48);
    let signal_posterior = bayes(prior_quality, high_signal_rate, low_signal_rate);
    let no_signal_posterior = bayes(prior_quality, 1.0 - high_signal_rate, 1.0 - low_signal_rate);
    let invest_threshold = (0.54 + inputs.force_asymmetry * 0.0018).clamp(0.42, 0.68);
    let separating_score = ((signal_posterior - no_signal_posterior) * 160.0
        + (signal_posterior - invest_threshold) * 80.0)
        .clamp(0.0, 100.0);

    SignalState {
        prior_quality,
        signal_posterior,
        no_signal_posterior,
        invest_threshold,
        separating_score,
    }
}

fn bayes(prior: f32, high_likelihood: f32, low_likelihood: f32) -> f32 {
    let numerator = prior * high_likelihood;
    let denominator = numerator + (1.0 - prior) * low_likelihood;
    (numerator / denominator.max(0.001)).clamp(0.0, 1.0)
}

fn stackelberg_state(inputs: AdvancedInputs) -> StackelbergState {
    let market_size = 118.0 + inputs.prize_skew * 0.20;
    let cost = 26.0 - inputs.commitment_power * 0.045;
    let capacity = (market_size - cost).max(24.0);
    let commitment = inputs.commitment_power / 100.0;
    let cournot_quantity = capacity / 3.0;
    let leader_quantity = (cournot_quantity + commitment * capacity / 6.0).clamp(18.0, 72.0);
    let follower_quantity = ((capacity - leader_quantity) / 2.0).clamp(8.0, 58.0);
    let price = (market_size - leader_quantity - follower_quantity).max(cost + 4.0);
    let leader_profit = (price - cost) * leader_quantity;
    let cournot_price = market_size - cournot_quantity * 2.0;
    let cournot_profit = (cournot_price - cost) * cournot_quantity;

    StackelbergState {
        leader_quantity,
        follower_quantity,
        cournot_quantity,
        leader_profit,
        cournot_profit,
    }
}

fn blotto_fronts(inputs: AdvancedInputs) -> Vec<Front> {
    let skew = inputs.prize_skew / 100.0;
    let blue_total = 100.0 + inputs.force_asymmetry;
    let red_total = 100.0 - inputs.force_asymmetry * 0.55;
    let values = [
        ("Core", 34.0 + skew * 38.0),
        ("Growth", 26.0 + (1.0 - (skew - 0.45).abs()) * 20.0),
        ("Channel", 22.0 + (1.0 - skew) * 26.0),
        ("Enterprise", 18.0 + skew * 30.0),
        ("Long tail", 14.0 + (1.0 - skew) * 18.0),
    ];
    let blue_weights = values
        .iter()
        .enumerate()
        .map(|(idx, (_, value))| value.sqrt() * (1.0 + idx as f32 * 0.035))
        .collect::<Vec<_>>();
    let red_weights = values
        .iter()
        .enumerate()
        .map(|(idx, (_, value))| value.sqrt() * (1.16 - idx as f32 * 0.045))
        .collect::<Vec<_>>();
    let blue_sum = blue_weights.iter().sum::<f32>();
    let red_sum = red_weights.iter().sum::<f32>();

    values
        .into_iter()
        .enumerate()
        .map(|(idx, (name, value))| {
            let blue = blue_weights[idx] / blue_sum * blue_total;
            let red = red_weights[idx] / red_sum * red_total;
            Front {
                name,
                value,
                blue,
                red,
                edge: blue - red,
            }
        })
        .collect()
}

fn contested_share(fronts: &[Front]) -> f32 {
    let contested = fronts
        .iter()
        .filter(|front| front.edge.abs() < 10.0)
        .map(|front| front.value)
        .sum::<f32>();
    let total = fronts.iter().map(|front| front.value).sum::<f32>().max(1.0);
    contested / total * 100.0
}

fn information_recommendation(signal: SignalState) -> &'static str {
    if signal.separating_score > 62.0 {
        "Let the signal screen quality"
    } else if signal.signal_posterior > signal.invest_threshold {
        "Partial separation; add verification"
    } else {
        "Pooling risk; raise signal cost"
    }
}

fn commitment_recommendation(stack: StackelbergState) -> &'static str {
    let premium = stack.leader_profit / stack.cournot_profit - 1.0;
    if premium > 0.12 {
        "Commit early and visibly"
    } else if premium > 0.03 {
        "Commit only with option value"
    } else {
        "Do not overpay for first move"
    }
}

fn contest_recommendation(fronts: &[Front]) -> &'static str {
    let swing = fronts
        .iter()
        .filter(|front| front.edge.abs() < 10.0)
        .count();
    if swing >= 4 {
        "Win by reallocating swing fronts"
    } else if swing >= 2 {
        "Protect the decisive fronts"
    } else {
        "Contest already polarized"
    }
}

fn dominant_framework(
    signal: SignalState,
    stack: StackelbergState,
    fronts: &[Front],
) -> &'static str {
    let information_score = signal.separating_score;
    let commitment_score =
        ((stack.leader_profit / stack.cournot_profit - 1.0) * 420.0).clamp(0.0, 100.0);
    let contest_score = contested_share(fronts);

    if information_score >= commitment_score && information_score >= contest_score {
        "Information design"
    } else if commitment_score >= contest_score {
        "Commitment design"
    } else {
        "Contest design"
    }
}

fn belief_x(value: f32) -> f32 {
    42.0 + value.clamp(0.0, 1.0) * 472.0
}

fn stack_x(value: f32) -> f32 {
    54.0 + (value / 76.0).clamp(0.0, 1.0) * 438.0
}

fn stack_y(value: f32) -> f32 {
    186.0 - (value / 58.0).clamp(0.0, 1.0) * 156.0
}
