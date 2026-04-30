//! Interactive game-theory visualization with payoff, equilibrium, and dynamics views.

use leptos::prelude::*;

use crate::dom_events::event_target_value_as_f32;

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameInputs {
    entry_cost: f32,
    defense_cost: f32,
    demand_lift: f32,
    moat_strength: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameState {
    inputs: GameInputs,
    row: [[f32; 2]; 2],
    col: [[f32; 2]; 2],
    mixed_defend: f32,
    mixed_enter: f32,
    row_value: f32,
    col_value: f32,
    pure_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PayoffCell {
    row_strategy: &'static str,
    col_strategy: &'static str,
    row_payoff: f32,
    col_payoff: f32,
    is_equilibrium: bool,
    class_name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct RegimeCell {
    label: &'static str,
    title: &'static str,
    detail: String,
    share: f32,
    class_name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct Trajectory {
    d: String,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FieldArrow {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    class_name: &'static str,
}

#[component]
pub fn View() -> impl IntoView {
    let entry_cost = RwSignal::new(26.0_f32);
    let defense_cost = RwSignal::new(18.0_f32);
    let demand_lift = RwSignal::new(12.0_f32);
    let moat_strength = RwSignal::new(16.0_f32);

    let inputs = move || GameInputs {
        entry_cost: entry_cost.get(),
        defense_cost: defense_cost.get(),
        demand_lift: demand_lift.get(),
        moat_strength: moat_strength.get(),
    };
    let state = move || game_state(inputs());

    let defend_label = move || format!("{:.0}%", state().mixed_defend * 100.0);
    let enter_label = move || format!("{:.0}%", state().mixed_enter * 100.0);
    let row_value_label = move || format!("{:+.1}", state().row_value);
    let col_value_label = move || format!("{:+.1}", state().col_value);
    let pure_label = move || state().pure_count.to_string();
    let pressure_label = move || {
        let game = state();
        format!("{:.1}", strategic_pressure(&game))
    };

    view! {
        <section id="game-theory" class="example game-theory">
            <div class="example-head">
                <div>
                    <h2>"Game Theory Equilibrium Map"</h2>
                    <p>
                        "An interactive entry-deterrence game with payoff surfaces, pure and mixed Nash equilibria, best-response thresholds, and replicator dynamics."
                    </p>
                </div>
                <div class="stat-strip game-stat-strip">
                    <span><strong>{defend_label}</strong>" defend mix"</span>
                    <span><strong>{enter_label}</strong>" entry mix"</span>
                    <span><strong>{row_value_label}</strong>" incumbent value"</span>
                    <span><strong>{col_value_label}</strong>" entrant value"</span>
                    <span><strong>{pure_label}</strong>" pure NE"</span>
                    <span><strong>{pressure_label}</strong>" pressure"</span>
                </div>
            </div>

            <div class="game-layout">
                <aside class="game-controls" aria-label="Game theory controls">
                    <div class="game-control-head">
                        <h3>"Strategic Levers"</h3>
                        <span>"Change incentives and watch the equilibrium move."</span>
                    </div>
                    <GameSlider label="Entry cost" prefix="$" min=4.0 max=52.0 step=1.0 value=entry_cost />
                    <GameSlider label="Defense cost" prefix="$" min=2.0 max=44.0 step=1.0 value=defense_cost />
                    <GameSlider label="Demand lift" prefix="$" min=-12.0 max=34.0 step=1.0 value=demand_lift />
                    <GameSlider label="Moat strength" prefix="$" min=0.0 max=36.0 step=1.0 value=moat_strength />

                    <div class="game-notes">
                        <div>
                            <strong>"Row player"</strong>
                            <span>"Incumbent chooses Defend or Accommodate."</span>
                        </div>
                        <div>
                            <strong>"Column player"</strong>
                            <span>"Entrant chooses Enter or Stay out."</span>
                        </div>
                    </div>
                </aside>

                <div class="game-main">
                    <div class="payoff-panel">
                        <div class="game-section-head">
                            <h3>"Payoff Matrix"</h3>
                            <span>"Cell values are incumbent / entrant payoff. Highlighted cells are pure Nash equilibria."</span>
                        </div>
                        <div class="payoff-matrix" aria-label="Game payoff matrix">
                            <div class="payoff-corner">"Incumbent \\ Entrant"</div>
                            <div class="payoff-axis">"Enter"</div>
                            <div class="payoff-axis">"Stay out"</div>
                            {move || payoff_cells(state()).into_iter().map(|cell| {
                                view! {
                                    <article class=cell.class_name>
                                        <span>{cell.row_strategy}</span>
                                        <em>{cell.col_strategy}</em>
                                        <strong>{format!("{:+.0} / {:+.0}", cell.row_payoff, cell.col_payoff)}</strong>
                                        <i>{if cell.is_equilibrium { "Nash equilibrium" } else { "best responses diverge" }}</i>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="phase-panel">
                        <div class="game-section-head">
                            <h3>"Best Response Phase Portrait"</h3>
                            <span>"The mixed equilibrium sits where both players are indifferent; paths show adaptive strategy pressure."</span>
                        </div>
                        <GamePhase state=state />
                    </div>

                    <div class="regime-panel">
                        <div class="game-section-head">
                            <h3>"Regime Diagnostics"</h3>
                            <span>"Strategic interpretation derived from payoff dominance and local dynamics."</span>
                        </div>
                        <div class="regime-grid">
                            {move || regime_cells(state()).into_iter().map(|cell| {
                                view! {
                                    <article class=cell.class_name style=format!("--regime-share:{}%;", cell.share)>
                                        <span>{cell.label}</span>
                                        <strong>{cell.title}</strong>
                                        <em>{cell.detail}</em>
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
fn GameSlider(
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
fn GamePhase<F>(state: F) -> impl IntoView
where
    F: Fn() -> GameState + Copy + Send + Sync + 'static,
{
    view! {
        <div class="phase-plot">
            <svg class="phase-svg" viewBox="0 0 620 360" role="img" aria-label="Replicator dynamics phase portrait">
                <defs>
                    <marker id="game-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
                        <path d="M 0 0 L 10 5 L 0 10 z"></path>
                    </marker>
                </defs>
                <rect class="phase-frame" x="54" y="28" width="500" height="280" rx="8"></rect>
                <g class="phase-grid">
                    <line x1="54" y1="84" x2="554" y2="84"></line>
                    <line x1="54" y1="140" x2="554" y2="140"></line>
                    <line x1="54" y1="196" x2="554" y2="196"></line>
                    <line x1="54" y1="252" x2="554" y2="252"></line>
                    <line x1="154" y1="28" x2="154" y2="308"></line>
                    <line x1="254" y1="28" x2="254" y2="308"></line>
                    <line x1="354" y1="28" x2="354" y2="308"></line>
                    <line x1="454" y1="28" x2="454" y2="308"></line>
                </g>
                {move || field_arrows(state()).into_iter().map(|arrow| {
                    view! {
                        <line
                            class=arrow.class_name
                            x1=format!("{:.1}", arrow.x1)
                            y1=format!("{:.1}", arrow.y1)
                            x2=format!("{:.1}", arrow.x2)
                            y2=format!("{:.1}", arrow.y2)
                            marker-end="url(#game-arrow)"
                        ></line>
                    }
                }).collect_view()}
                {move || {
                    let game = state();
                    let x = phase_x(game.mixed_defend);
                    let y = phase_y(game.mixed_enter);
                    view! {
                        <g>
                            <line class="best-response-line row-threshold" x1=format!("{:.1}", x) y1="28" x2=format!("{:.1}", x) y2="308"></line>
                            <line class="best-response-line col-threshold" x1="54" y1=format!("{:.1}", y) x2="554" y2=format!("{:.1}", y)></line>
                        </g>
                    }
                }}
                {move || trajectories(state()).into_iter().map(|trajectory| {
                    view! { <path class=trajectory.class_name d=trajectory.d></path> }
                }).collect_view()}
                {move || {
                    let game = state();
                    view! {
                        <circle
                            class="mixed-equilibrium-dot"
                            cx=format!("{:.1}", phase_x(game.mixed_defend))
                            cy=format!("{:.1}", phase_y(game.mixed_enter))
                            r="7"
                        ></circle>
                    }
                }}
                <text class="phase-label phase-label-left" x="24" y="176" transform="rotate(-90 24 176)">"Pr(entrant enters)"</text>
                <text class="phase-label phase-label-bottom" x="248" y="344">"Pr(incumbent defends)"</text>
                <text class="phase-tick" x="50" y="326">"0"</text>
                <text class="phase-tick" x="548" y="326">"1"</text>
                <text class="phase-tick" x="31" y="312">"0"</text>
                <text class="phase-tick" x="31" y="34">"1"</text>
            </svg>
            <div class="phase-readout">
                {move || {
                    let game = state();
                    view! {
                        <>
                            <span><strong>{format!("{:.1}%", game.mixed_defend * 100.0)}</strong>" incumbent defend threshold"</span>
                            <span><strong>{format!("{:.1}%", game.mixed_enter * 100.0)}</strong>" entrant entry threshold"</span>
                            <span><strong>{classify_equilibrium(game)}</strong>" local dynamic"</span>
                        </>
                    }
                }}
            </div>
        </div>
    }
}

fn game_state(inputs: GameInputs) -> GameState {
    let row = [
        [
            34.0 + inputs.moat_strength * 0.55 - inputs.defense_cost + inputs.demand_lift * 0.24,
            68.0 + inputs.moat_strength * 0.28 - inputs.defense_cost * 0.62,
        ],
        [
            50.0 + inputs.demand_lift * 0.62 - inputs.moat_strength * 0.18,
            76.0 + inputs.demand_lift * 0.18,
        ],
    ];
    let col = [
        [
            31.0 + inputs.demand_lift * 0.52 - inputs.entry_cost - inputs.moat_strength * 0.65,
            13.0,
        ],
        [
            52.0 + inputs.demand_lift * 0.66 - inputs.entry_cost + inputs.moat_strength * 0.12,
            16.0,
        ],
    ];
    let mixed_enter = indifference_probability(
        row[1][1] - row[0][1],
        row[0][0] - row[1][0] - row[0][1] + row[1][1],
    );
    let mixed_defend = indifference_probability(
        col[1][1] - col[1][0],
        col[0][0] - col[0][1] - col[1][0] + col[1][1],
    );
    let row_value = expected_row(row, mixed_defend, mixed_enter);
    let col_value = expected_col(col, mixed_defend, mixed_enter);
    let pure_count = pure_equilibria(row, col).len();

    GameState {
        inputs,
        row,
        col,
        mixed_defend,
        mixed_enter,
        row_value,
        col_value,
        pure_count,
    }
}

fn indifference_probability(numerator: f32, denominator: f32) -> f32 {
    if denominator.abs() < 0.001 {
        0.5
    } else {
        (numerator / denominator).clamp(0.02, 0.98)
    }
}

fn payoff_cells(game: GameState) -> Vec<PayoffCell> {
    let pure = pure_equilibria(game.row, game.col);
    let labels = [
        ("Defend", "Enter"),
        ("Defend", "Stay out"),
        ("Accommodate", "Enter"),
        ("Accommodate", "Stay out"),
    ];
    labels
        .into_iter()
        .enumerate()
        .map(|(idx, (row_strategy, col_strategy))| {
            let row_idx = if idx < 2 { 0 } else { 1 };
            let col_idx = idx % 2;
            let is_equilibrium = pure.iter().any(|item| *item == (row_idx, col_idx));
            PayoffCell {
                row_strategy,
                col_strategy,
                row_payoff: game.row[row_idx][col_idx],
                col_payoff: game.col[row_idx][col_idx],
                is_equilibrium,
                class_name: if is_equilibrium {
                    "payoff-cell is-equilibrium"
                } else {
                    "payoff-cell"
                },
            }
        })
        .collect()
}

fn pure_equilibria(row: [[f32; 2]; 2], col: [[f32; 2]; 2]) -> Vec<(usize, usize)> {
    let mut equilibria = Vec::new();
    for r in 0..2 {
        for c in 0..2 {
            let row_best = row[r][c] >= row[1 - r][c] - 0.001;
            let col_best = col[r][c] >= col[r][1 - c] - 0.001;
            if row_best && col_best {
                equilibria.push((r, c));
            }
        }
    }
    equilibria
}

fn regime_cells(game: GameState) -> Vec<RegimeCell> {
    let entry_gap =
        col_expected_enter(game, game.mixed_defend) - col_expected_out(game, game.mixed_defend);
    let defense_gap = row_expected_defend(game, game.mixed_enter)
        - row_expected_accommodate(game, game.mixed_enter);
    let deterrence = (game.inputs.moat_strength + game.inputs.entry_cost
        - game.inputs.defense_cost * 0.45)
        .clamp(0.0, 88.0);
    let contestability = (58.0 + game.inputs.demand_lift
        - game.inputs.entry_cost
        - game.inputs.moat_strength * 0.42)
        .clamp(0.0, 88.0);
    let volatility = (strategic_pressure(&game) * 12.0).clamp(0.0, 88.0);
    let surplus = ((game.row_value + game.col_value) / 1.45).clamp(0.0, 88.0);

    vec![
        RegimeCell {
            label: "Deterrence",
            title: if deterrence > 44.0 {
                "Credible barrier"
            } else {
                "Porous barrier"
            },
            detail: format!(
                "Moat plus entry friction nets {:.0} payoff points.",
                deterrence
            ),
            share: deterrence,
            class_name: "regime-card regime-blue",
        },
        RegimeCell {
            label: "Contestability",
            title: if contestability > 44.0 {
                "Entry remains live"
            } else {
                "Entry suppressed"
            },
            detail: format!(
                "Entrant upside sits {:+.1} from the stay-out option.",
                entry_gap
            ),
            share: contestability,
            class_name: "regime-card regime-green",
        },
        RegimeCell {
            label: "Adaptation",
            title: classify_equilibrium(game),
            detail: format!(
                "Best-response imbalance is {:+.1} for defense and {:+.1} for entry.",
                defense_gap, entry_gap
            ),
            share: volatility,
            class_name: "regime-card regime-amber",
        },
        RegimeCell {
            label: "Joint value",
            title: if surplus > 52.0 {
                "High surplus game"
            } else {
                "Thin surplus game"
            },
            detail: format!(
                "Mixed expected total payoff is {:.1}.",
                game.row_value + game.col_value
            ),
            share: surplus,
            class_name: "regime-card regime-red",
        },
    ]
}

fn field_arrows(game: GameState) -> Vec<FieldArrow> {
    let mut arrows = Vec::new();
    for xi in 1..8 {
        for yi in 1..6 {
            let p = xi as f32 / 8.0;
            let q = yi as f32 / 6.0;
            let (dp, dq) = replicator_step(game, p, q);
            let magnitude = (dp * dp + dq * dq).sqrt();
            let scale = if magnitude > 0.001 {
                28.0 / magnitude.min(0.18)
            } else {
                0.0
            };
            let x1 = phase_x(p);
            let y1 = phase_y(q);
            arrows.push(FieldArrow {
                x1,
                y1,
                x2: (x1 + dp * scale).clamp(58.0, 550.0),
                y2: (y1 - dq * scale).clamp(32.0, 304.0),
                class_name: if magnitude > 0.07 {
                    "phase-arrow is-strong"
                } else {
                    "phase-arrow"
                },
            });
        }
    }
    arrows
}

fn trajectories(game: GameState) -> Vec<Trajectory> {
    let seeds = [
        (0.12, 0.18),
        (0.22, 0.78),
        (0.48, 0.36),
        (0.72, 0.86),
        (0.88, 0.22),
    ];
    seeds
        .into_iter()
        .enumerate()
        .map(|(idx, (mut p, mut q))| {
            let mut d = format!("M {:.1} {:.1}", phase_x(p), phase_y(q));
            for _ in 0..34 {
                let (dp, dq) = replicator_step(game, p, q);
                p = (p + dp * 0.72).clamp(0.015, 0.985);
                q = (q + dq * 0.72).clamp(0.015, 0.985);
                d.push_str(&format!(" L {:.1} {:.1}", phase_x(p), phase_y(q)));
            }
            Trajectory {
                d,
                class_name: if idx == 2 {
                    "phase-trajectory is-primary"
                } else {
                    "phase-trajectory"
                },
            }
        })
        .collect()
}

fn replicator_step(game: GameState, p_defend: f32, q_enter: f32) -> (f32, f32) {
    let row_gap = row_expected_defend(game, q_enter) - row_expected_accommodate(game, q_enter);
    let col_gap = col_expected_enter(game, p_defend) - col_expected_out(game, p_defend);
    (
        p_defend * (1.0 - p_defend) * row_gap / 48.0,
        q_enter * (1.0 - q_enter) * col_gap / 48.0,
    )
}

fn row_expected_defend(game: GameState, q_enter: f32) -> f32 {
    q_enter * game.row[0][0] + (1.0 - q_enter) * game.row[0][1]
}

fn row_expected_accommodate(game: GameState, q_enter: f32) -> f32 {
    q_enter * game.row[1][0] + (1.0 - q_enter) * game.row[1][1]
}

fn col_expected_enter(game: GameState, p_defend: f32) -> f32 {
    p_defend * game.col[0][0] + (1.0 - p_defend) * game.col[1][0]
}

fn col_expected_out(game: GameState, p_defend: f32) -> f32 {
    p_defend * game.col[0][1] + (1.0 - p_defend) * game.col[1][1]
}

fn expected_row(row: [[f32; 2]; 2], p_defend: f32, q_enter: f32) -> f32 {
    p_defend * q_enter * row[0][0]
        + p_defend * (1.0 - q_enter) * row[0][1]
        + (1.0 - p_defend) * q_enter * row[1][0]
        + (1.0 - p_defend) * (1.0 - q_enter) * row[1][1]
}

fn expected_col(col: [[f32; 2]; 2], p_defend: f32, q_enter: f32) -> f32 {
    p_defend * q_enter * col[0][0]
        + p_defend * (1.0 - q_enter) * col[0][1]
        + (1.0 - p_defend) * q_enter * col[1][0]
        + (1.0 - p_defend) * (1.0 - q_enter) * col[1][1]
}

fn strategic_pressure(game: &GameState) -> f32 {
    let defense_gap = row_expected_defend(*game, game.mixed_enter)
        - row_expected_accommodate(*game, game.mixed_enter);
    let entry_gap =
        col_expected_enter(*game, game.mixed_defend) - col_expected_out(*game, game.mixed_defend);
    (defense_gap.abs() + entry_gap.abs() + game.pure_count as f32 * 0.9).max(0.1)
}

fn classify_equilibrium(game: GameState) -> &'static str {
    let row_slope = game.row[0][0] - game.row[1][0] - game.row[0][1] + game.row[1][1];
    let col_slope = game.col[0][0] - game.col[0][1] - game.col[1][0] + game.col[1][1];
    if row_slope * col_slope < -8.0 {
        "stable attractor"
    } else if row_slope * col_slope > 8.0 {
        "coordination saddle"
    } else {
        "near neutral"
    }
}

fn phase_x(p: f32) -> f32 {
    54.0 + p.clamp(0.0, 1.0) * 500.0
}

fn phase_y(q: f32) -> f32 {
    308.0 - q.clamp(0.0, 1.0) * 280.0
}
