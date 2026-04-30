//! Planning workspace demo with Kanban, roadmap, release health, and prioritization views.

use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanningView {
    Delivery,
    Risk,
    Capacity,
}

#[component]
pub fn View() -> impl IntoView {
    let selected_view = RwSignal::new(PlanningView::Delivery);
    let show_risks = RwSignal::new(true);

    let columns = kanban_columns();
    let roadmap = roadmap_items();
    let milestones = milestone_items();
    let releases = release_rows();
    let dependencies = dependency_rows();
    let matrix = priority_matrix();
    let teams = team_rows();
    let workstreams = workstream_rows();
    let summary = planning_summary(&columns, &roadmap, &releases);

    let active_view_label = move || match selected_view.get() {
        PlanningView::Delivery => "Delivery forecast",
        PlanningView::Risk => "Risk posture",
        PlanningView::Capacity => "Capacity allocation",
    };
    let active_view_detail = move || match selected_view.get() {
        PlanningView::Delivery => {
            "Focuses on committed milestones, release readiness, and WIP aging."
        }
        PlanningView::Risk => {
            "Highlights blocked work, unstable dependencies, and low-confidence roadmap items."
        }
        PlanningView::Capacity => "Shows staffing pressure, review load, and workstream balance.",
    };

    view! {
        <section id="planning-workspace" class="example planning-workspace">
            <div class="example-head">
                <div>
                    <h2>"Planning Workspace"</h2>
                    <p>
                        "A product operating view combining a Kanban system, roadmap timeline, release readiness, dependency tracking, team capacity, and prioritization."
                    </p>
                </div>
                <div class="stat-strip planning-stat-strip">
                    <span><strong>{summary.total_cards.to_string()}</strong>" cards"</span>
                    <span><strong>{summary.total_points.to_string()}</strong>" points"</span>
                    <span><strong>{summary.blocked_cards.to_string()}</strong>" blocked"</span>
                    <span><strong>{summary.next_release}</strong>" next release"</span>
                    <span><strong>{summary.confidence}</strong>" confidence"</span>
                    <span><strong>{summary.readiness}</strong>" readiness"</span>
                </div>
            </div>

            <div class="planning-command-bar">
                <div class="planning-mode-group" role="group" aria-label="Planning lens">
                    <button
                        type="button"
                        class=move || planning_mode_class(selected_view.get() == PlanningView::Delivery)
                        on:click=move |_| selected_view.set(PlanningView::Delivery)
                    >
                        "Delivery"
                    </button>
                    <button
                        type="button"
                        class=move || planning_mode_class(selected_view.get() == PlanningView::Risk)
                        on:click=move |_| selected_view.set(PlanningView::Risk)
                    >
                        "Risk"
                    </button>
                    <button
                        type="button"
                        class=move || planning_mode_class(selected_view.get() == PlanningView::Capacity)
                        on:click=move |_| selected_view.set(PlanningView::Capacity)
                    >
                        "Capacity"
                    </button>
                </div>
                <div class="planning-active-lens">
                    <strong>{active_view_label}</strong>
                    <span>{active_view_detail}</span>
                </div>
                <button
                    type="button"
                    class=move || planning_toggle_class(show_risks.get())
                    on:click=move |_| show_risks.update(|value| *value = !*value)
                >
                    "Risk overlay"
                </button>
            </div>

            <div class=move || planning_shell_class(selected_view.get(), show_risks.get())>
                <div class="planning-primary">
                    <div class="planning-operating-strip">
                        {workstreams.into_iter().map(|stream| {
                            view! {
                                <article class=stream.class_name>
                                    <span>{stream.label}</span>
                                    <strong>{stream.value}</strong>
                                    <em>{stream.detail}</em>
                                    <i><b style=format!("width:{}%", stream.progress_pct)></b></i>
                                </article>
                            }
                        }).collect_view()}
                    </div>

                    <div class="planning-section-head">
                        <h3>"Kanban Board"</h3>
                        <span>"WIP, ownership, health, aging, effort, and release alignment by lane"</span>
                    </div>
                    <div class="kanban-board" aria-label="Product Kanban board">
                        {columns.into_iter().map(|column| {
                            let task_count = column.tasks.len();
                            let column_points: u16 = column.tasks.iter().map(|task| task.points).sum();
                            view! {
                                <section class="kanban-column">
                                    <div class="kanban-column-head">
                                        <div>
                                            <h4>{column.name}</h4>
                                            <span>{column.focus}</span>
                                        </div>
                                        <strong>{format!("{} / {}", task_count, column.capacity)}</strong>
                                    </div>
                                    <div class="kanban-wip">
                                        <span>{format!("{} pts committed", column_points)}</span>
                                        <i style=format!("--wip-pct:{}%;", wip_pct(task_count, column.capacity))></i>
                                    </div>
                                    <div class="kanban-card-stack">
                                        {column.tasks.into_iter().map(|task| {
                                            view! {
                                                <article class=task.class_name>
                                                    <div class="kanban-card-top">
                                                        <span>{task.priority}</span>
                                                        <em>{format!("{} pts", task.points)}</em>
                                                    </div>
                                                    <strong>{task.title}</strong>
                                                    <p>{task.detail}</p>
                                                    <div class="kanban-card-meta">
                                                        <span>{task.release}</span>
                                                        <span>{task.health}</span>
                                                        <span>{format!("{}d age", task.age_days)}</span>
                                                    </div>
                                                    <div class="kanban-card-risk">
                                                        <span>"Risk"</span>
                                                        <i><b style=format!("width:{}%", task.risk_pct)></b></i>
                                                    </div>
                                                    <div class="kanban-card-foot">
                                                        <span>{task.owner}</span>
                                                        <b>{task.due}</b>
                                                    </div>
                                                </article>
                                            }
                                        }).collect_view()}
                                    </div>
                                </section>
                            }
                        }).collect_view()}
                    </div>

                    <div class="planning-section-head roadmap-head">
                        <h3>"Roadmap Chart"</h3>
                        <span>"Quarterly delivery windows, confidence, milestones, and committed checkpoints"</span>
                    </div>
                    <div class="roadmap-chart" aria-label="Roadmap timeline">
                        <div class="roadmap-axis">
                            <span>"May"</span>
                            <span>"Jun"</span>
                            <span>"Jul"</span>
                            <span>"Aug"</span>
                            <span>"Sep"</span>
                            <span>"Oct"</span>
                        </div>
                        <div class="roadmap-lanes">
                            {roadmap.into_iter().map(|item| {
                                view! {
                                    <div class="roadmap-lane">
                                        <span>{item.lane}</span>
                                        <div class="roadmap-track">
                                            <article
                                                class=item.class_name
                                                style=format!(
                                                    "left:{}%; width:{}%; --roadmap-progress:{}%; --roadmap-risk:{}%;",
                                                    item.start_pct,
                                                    item.width_pct,
                                                    item.progress_pct,
                                                    item.risk_pct,
                                                )
                                            >
                                                <div>
                                                    <strong>{item.title}</strong>
                                                    <em>{item.window}</em>
                                                </div>
                                                <b>{format!("{}%", item.confidence_pct)}</b>
                                                <i></i>
                                            </article>
                                            {item.checkpoints.into_iter().map(|checkpoint| {
                                                view! {
                                                    <span
                                                        class=checkpoint.class_name
                                                        style=format!("left:{}%;", checkpoint.position_pct)
                                                    >
                                                        {checkpoint.label}
                                                    </span>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="milestone-strip" aria-label="Roadmap milestones">
                        {milestones.into_iter().map(|milestone| {
                            view! {
                                <article class=milestone.class_name>
                                    <span>{milestone.date}</span>
                                    <strong>{milestone.title}</strong>
                                    <em>{milestone.detail}</em>
                                </article>
                            }
                        }).collect_view()}
                    </div>
                </div>

                <aside class="planning-side">
                    <div class="planning-panel">
                        <h3>"Release Readiness"</h3>
                        <div class="release-list">
                            {releases.into_iter().map(|release| {
                                view! {
                                    <div class="release-row">
                                        <div>
                                            <strong>{release.name}</strong>
                                            <span>{release.scope}</span>
                                        </div>
                                        <em>{format!("{}%", release.readiness)}</em>
                                        <i><b style=format!("width:{}%", release.readiness)></b></i>
                                        <div class="release-diagnostics">
                                            <span><b>"Tests"</b><em>{format!("{}%", release.tests_pct)}</em></span>
                                            <span><b>"Docs"</b><em>{format!("{}%", release.docs_pct)}</em></span>
                                            <span><b>"Risk"</b><em>{release.risk}</em></span>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="planning-panel">
                        <h3>"Team Capacity"</h3>
                        <div class="team-capacity-list">
                            {teams.into_iter().map(|team| {
                                view! {
                                    <div class="team-capacity-row">
                                        <div>
                                            <strong>{team.name}</strong>
                                            <span>{team.focus}</span>
                                        </div>
                                        <em>{format!("{}%", team.load_pct)}</em>
                                        <i><b style=format!("width:{}%", team.load_pct)></b></i>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="planning-panel">
                        <h3>"Dependencies"</h3>
                        <div class="dependency-list">
                            {dependencies.into_iter().map(|dep| {
                                view! {
                                    <article class=dep.class_name>
                                        <span>{dep.status}</span>
                                        <strong>{dep.name}</strong>
                                        <em>{dep.detail}</em>
                                        <b>{dep.owner}</b>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="planning-panel priority-panel">
                        <h3>"Priority Matrix"</h3>
                        <div class="priority-matrix" aria-label="Impact and effort matrix">
                            {matrix.into_iter().map(|cell| {
                                view! {
                                    <article class=cell.class_name>
                                        <span>{cell.label}</span>
                                        <strong>{cell.title}</strong>
                                        <em>{cell.detail}</em>
                                        <i style=format!(
                                            "left:{}%; bottom:{}%;",
                                            cell.effort_pct,
                                            cell.impact_pct,
                                        )></i>
                                    </article>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                </aside>
            </div>
        </section>
    }
}

#[derive(Debug, Clone)]
struct KanbanColumn {
    name: &'static str,
    focus: &'static str,
    capacity: usize,
    tasks: Vec<KanbanTask>,
}

#[derive(Debug, Clone, Copy)]
struct KanbanTask {
    title: &'static str,
    detail: &'static str,
    owner: &'static str,
    due: &'static str,
    release: &'static str,
    priority: &'static str,
    health: &'static str,
    points: u16,
    age_days: u8,
    risk_pct: u8,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct RoadmapItem {
    lane: &'static str,
    title: &'static str,
    window: &'static str,
    start_pct: u8,
    width_pct: u8,
    progress_pct: u8,
    confidence_pct: u8,
    risk_pct: u8,
    class_name: &'static str,
    checkpoints: &'static [Checkpoint],
}

#[derive(Debug, Clone, Copy)]
struct Checkpoint {
    label: &'static str,
    position_pct: u8,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct ReleaseRow {
    name: &'static str,
    scope: &'static str,
    readiness: u8,
    tests_pct: u8,
    docs_pct: u8,
    risk: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct DependencyRow {
    name: &'static str,
    detail: &'static str,
    owner: &'static str,
    status: &'static str,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct PriorityCell {
    label: &'static str,
    title: &'static str,
    detail: &'static str,
    impact_pct: u8,
    effort_pct: u8,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct TeamRow {
    name: &'static str,
    focus: &'static str,
    load_pct: u8,
}

#[derive(Debug, Clone, Copy)]
struct WorkstreamRow {
    label: &'static str,
    value: &'static str,
    detail: &'static str,
    progress_pct: u8,
    class_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct MilestoneItem {
    date: &'static str,
    title: &'static str,
    detail: &'static str,
    class_name: &'static str,
}

#[derive(Debug, Clone)]
struct PlanningSummary {
    total_cards: usize,
    total_points: u16,
    blocked_cards: usize,
    next_release: String,
    confidence: String,
    readiness: String,
}

const PLANNING_CHECKPOINTS: [Checkpoint; 2] = [
    Checkpoint {
        label: "Spec",
        position_pct: 18,
        class_name: "roadmap-checkpoint is-done",
    },
    Checkpoint {
        label: "RC",
        position_pct: 78,
        class_name: "roadmap-checkpoint is-watch",
    },
];

const ANALYTICS_CHECKPOINTS: [Checkpoint; 2] = [
    Checkpoint {
        label: "API",
        position_pct: 42,
        class_name: "roadmap-checkpoint is-watch",
    },
    Checkpoint {
        label: "Docs",
        position_pct: 88,
        class_name: "roadmap-checkpoint",
    },
];

const COLLAB_CHECKPOINTS: [Checkpoint; 2] = [
    Checkpoint {
        label: "Schema",
        position_pct: 22,
        class_name: "roadmap-checkpoint is-risk",
    },
    Checkpoint {
        label: "Beta",
        position_pct: 84,
        class_name: "roadmap-checkpoint",
    },
];

const SCALE_CHECKPOINTS: [Checkpoint; 2] = [
    Checkpoint {
        label: "Bench",
        position_pct: 28,
        class_name: "roadmap-checkpoint",
    },
    Checkpoint {
        label: "GA",
        position_pct: 92,
        class_name: "roadmap-checkpoint",
    },
];

fn kanban_columns() -> Vec<KanbanColumn> {
    vec![
        KanbanColumn {
            name: "Discovery",
            focus: "Evidence and sizing",
            capacity: 5,
            tasks: vec![
                KanbanTask {
                    title: "Saved planning views",
                    detail: "Persist filters, annotations, selected lens, and stakeholder readouts.",
                    owner: "Platform",
                    due: "May 10",
                    release: "May pack",
                    priority: "P1",
                    health: "healthy",
                    points: 5,
                    age_days: 3,
                    risk_pct: 18,
                    class_name: "kanban-card priority-high health-good",
                },
                KanbanTask {
                    title: "Template gallery taxonomy",
                    detail: "Promoted starter boards for revenue, ops, cohorts, maps, and planning.",
                    owner: "Design",
                    due: "May 14",
                    release: "Templates",
                    priority: "P2",
                    health: "watch",
                    points: 3,
                    age_days: 6,
                    risk_pct: 36,
                    class_name: "kanban-card priority-medium health-watch",
                },
            ],
        },
        KanbanColumn {
            name: "Ready",
            focus: "Clear to build",
            capacity: 4,
            tasks: vec![
                KanbanTask {
                    title: "Roadmap chart spec",
                    detail: "Timeline marks with grouped lanes, milestones, status bands, and tooltips.",
                    owner: "Charts",
                    due: "May 17",
                    release: "Chart API",
                    priority: "P1",
                    health: "watch",
                    points: 8,
                    age_days: 5,
                    risk_pct: 44,
                    class_name: "kanban-card priority-high health-watch",
                },
                KanbanTask {
                    title: "CSV import guardrails",
                    detail: "Column type hints, missing value policy, and preview warnings.",
                    owner: "Data",
                    due: "May 20",
                    release: "Templates",
                    priority: "P2",
                    health: "healthy",
                    points: 5,
                    age_days: 2,
                    risk_pct: 24,
                    class_name: "kanban-card priority-medium health-good",
                },
            ],
        },
        KanbanColumn {
            name: "In Progress",
            focus: "Limit active work",
            capacity: 3,
            tasks: vec![
                KanbanTask {
                    title: "Kanban operating view",
                    detail: "Board lanes, WIP indicators, health badges, owner badges, and card density.",
                    owner: "Frontend",
                    due: "May 06",
                    release: "May pack",
                    priority: "P0",
                    health: "at risk",
                    points: 8,
                    age_days: 9,
                    risk_pct: 66,
                    class_name: "kanban-card priority-critical health-risk",
                },
                KanbanTask {
                    title: "Release dashboard",
                    detail: "Readiness bars, dependency flags, risk overlay, and confidence scoring.",
                    owner: "Product",
                    due: "May 08",
                    release: "May pack",
                    priority: "P1",
                    health: "healthy",
                    points: 5,
                    age_days: 4,
                    risk_pct: 28,
                    class_name: "kanban-card priority-high health-good",
                },
            ],
        },
        KanbanColumn {
            name: "Review",
            focus: "Verify quality",
            capacity: 3,
            tasks: vec![
                KanbanTask {
                    title: "Mobile board polish",
                    detail: "Compact lanes, readable cards, stable horizontal scrolling, and no overlap.",
                    owner: "QA",
                    due: "May 11",
                    release: "May pack",
                    priority: "P2",
                    health: "watch",
                    points: 2,
                    age_days: 7,
                    risk_pct: 38,
                    class_name: "kanban-card priority-medium health-watch",
                },
                KanbanTask {
                    title: "Accessibility pass",
                    detail: "Keyboard focus, aria labels, contrast review, and reduced motion handling.",
                    owner: "Frontend",
                    due: "May 12",
                    release: "May pack",
                    priority: "P1",
                    health: "blocked",
                    points: 3,
                    age_days: 8,
                    risk_pct: 74,
                    class_name: "kanban-card priority-high health-blocked",
                },
            ],
        },
    ]
}

fn roadmap_items() -> Vec<RoadmapItem> {
    vec![
        RoadmapItem {
            lane: "Planning",
            title: "Board workspace",
            window: "May",
            start_pct: 2,
            width_pct: 24,
            progress_pct: 72,
            confidence_pct: 82,
            risk_pct: 28,
            class_name: "roadmap-item roadmap-build",
            checkpoints: &PLANNING_CHECKPOINTS,
        },
        RoadmapItem {
            lane: "Analytics",
            title: "Roadmap chart API",
            window: "May-Jun",
            start_pct: 18,
            width_pct: 30,
            progress_pct: 44,
            confidence_pct: 61,
            risk_pct: 47,
            class_name: "roadmap-item roadmap-data",
            checkpoints: &ANALYTICS_CHECKPOINTS,
        },
        RoadmapItem {
            lane: "Collaboration",
            title: "Saved annotations",
            window: "Jun-Jul",
            start_pct: 36,
            width_pct: 28,
            progress_pct: 28,
            confidence_pct: 43,
            risk_pct: 72,
            class_name: "roadmap-item roadmap-collab",
            checkpoints: &COLLAB_CHECKPOINTS,
        },
        RoadmapItem {
            lane: "Scale",
            title: "Large dataset pass",
            window: "Aug-Oct",
            start_pct: 62,
            width_pct: 34,
            progress_pct: 12,
            confidence_pct: 58,
            risk_pct: 52,
            class_name: "roadmap-item roadmap-scale",
            checkpoints: &SCALE_CHECKPOINTS,
        },
    ]
}

fn release_rows() -> Vec<ReleaseRow> {
    vec![
        ReleaseRow {
            name: "May planning pack",
            scope: "Kanban, roadmap, dependencies",
            readiness: 78,
            tests_pct: 64,
            docs_pct: 71,
            risk: "watch",
        },
        ReleaseRow {
            name: "Chart templates",
            scope: "Reusable operating dashboards",
            readiness: 54,
            tests_pct: 48,
            docs_pct: 39,
            risk: "medium",
        },
        ReleaseRow {
            name: "Collaboration beta",
            scope: "Saved views and comments",
            readiness: 31,
            tests_pct: 22,
            docs_pct: 18,
            risk: "high",
        },
    ]
}

fn dependency_rows() -> Vec<DependencyRow> {
    vec![
        DependencyRow {
            name: "Roadmap mark primitives",
            detail: "Needed before the timeline graduates from CSS demo to chart spec.",
            owner: "Charts",
            status: "watch",
            class_name: "dependency-item dependency-watch",
        },
        DependencyRow {
            name: "Annotation storage schema",
            detail: "Blocks comment threads and saved callouts.",
            owner: "Platform",
            status: "risk",
            class_name: "dependency-item dependency-risk",
        },
        DependencyRow {
            name: "Mobile layout QA",
            detail: "Required for release candidate signoff.",
            owner: "QA",
            status: "ready",
            class_name: "dependency-item dependency-ready",
        },
    ]
}

fn priority_matrix() -> Vec<PriorityCell> {
    vec![
        PriorityCell {
            label: "High impact / low effort",
            title: "Release health bars",
            detail: "Ship with the board.",
            impact_pct: 82,
            effort_pct: 22,
            class_name: "priority-cell priority-now",
        },
        PriorityCell {
            label: "High impact / high effort",
            title: "Native roadmap spec",
            detail: "Plan into chart core.",
            impact_pct: 84,
            effort_pct: 76,
            class_name: "priority-cell priority-plan",
        },
        PriorityCell {
            label: "Low impact / low effort",
            title: "Lane color presets",
            detail: "Batch with polish.",
            impact_pct: 34,
            effort_pct: 26,
            class_name: "priority-cell priority-batch",
        },
        PriorityCell {
            label: "Low impact / high effort",
            title: "Full drag/drop",
            detail: "Defer until persistence exists.",
            impact_pct: 28,
            effort_pct: 80,
            class_name: "priority-cell priority-defer",
        },
    ]
}

fn team_rows() -> Vec<TeamRow> {
    vec![
        TeamRow {
            name: "Frontend",
            focus: "Board UX and responsive QA",
            load_pct: 92,
        },
        TeamRow {
            name: "Charts",
            focus: "Roadmap primitives",
            load_pct: 78,
        },
        TeamRow {
            name: "Platform",
            focus: "Persistence and schema",
            load_pct: 86,
        },
        TeamRow {
            name: "QA",
            focus: "Release certification",
            load_pct: 68,
        },
    ]
}

fn workstream_rows() -> Vec<WorkstreamRow> {
    vec![
        WorkstreamRow {
            label: "Committed",
            value: "36 pts",
            detail: "31 pts planned capacity",
            progress_pct: 116,
            class_name: "planning-operating-card is-over",
        },
        WorkstreamRow {
            label: "Cycle time",
            value: "5.6d",
            detail: "1.2d faster than last cycle",
            progress_pct: 64,
            class_name: "planning-operating-card is-good",
        },
        WorkstreamRow {
            label: "Review load",
            value: "2.4x",
            detail: "Frontend and QA bottleneck",
            progress_pct: 88,
            class_name: "planning-operating-card is-watch",
        },
        WorkstreamRow {
            label: "Forecast",
            value: "May 16",
            detail: "P75 ship date",
            progress_pct: 78,
            class_name: "planning-operating-card is-good",
        },
    ]
}

fn milestone_items() -> Vec<MilestoneItem> {
    vec![
        MilestoneItem {
            date: "May 03",
            title: "Scope freeze",
            detail: "No new P1s without owner tradeoff.",
            class_name: "milestone-card is-done",
        },
        MilestoneItem {
            date: "May 09",
            title: "Release candidate",
            detail: "Board and roadmap views ready for QA.",
            class_name: "milestone-card is-watch",
        },
        MilestoneItem {
            date: "May 16",
            title: "Planning pack ship",
            detail: "Templates, docs, and signoff complete.",
            class_name: "milestone-card",
        },
    ]
}

fn planning_summary(
    columns: &[KanbanColumn],
    roadmap: &[RoadmapItem],
    releases: &[ReleaseRow],
) -> PlanningSummary {
    let total_cards = columns.iter().map(|column| column.tasks.len()).sum();
    let total_points = columns
        .iter()
        .flat_map(|column| column.tasks.iter())
        .map(|task| task.points)
        .sum();
    let blocked_cards = columns
        .iter()
        .flat_map(|column| column.tasks.iter())
        .filter(|task| task.health == "blocked" || task.risk_pct >= 70)
        .count();
    let next_release = releases
        .first()
        .map(|release| release.name.to_string())
        .unwrap_or_else(|| "No release".to_string());
    let avg_progress = roadmap
        .iter()
        .map(|item| item.confidence_pct as u16)
        .sum::<u16>() as f32
        / roadmap.len().max(1) as f32;
    let avg_readiness = releases
        .iter()
        .map(|release| release.readiness as u16)
        .sum::<u16>() as f32
        / releases.len().max(1) as f32;

    PlanningSummary {
        total_cards,
        total_points,
        blocked_cards,
        next_release,
        confidence: format!("{avg_progress:.0}%"),
        readiness: format!("{avg_readiness:.0}%"),
    }
}

fn planning_mode_class(active: bool) -> &'static str {
    if active {
        "planning-mode is-active"
    } else {
        "planning-mode"
    }
}

fn planning_toggle_class(active: bool) -> &'static str {
    if active {
        "planning-risk-toggle is-active"
    } else {
        "planning-risk-toggle"
    }
}

fn planning_shell_class(view: PlanningView, show_risks: bool) -> String {
    let mut class = String::from("planning-layout");
    match view {
        PlanningView::Delivery => class.push_str(" is-delivery-view"),
        PlanningView::Risk => class.push_str(" is-risk-view"),
        PlanningView::Capacity => class.push_str(" is-capacity-view"),
    }
    if !show_risks {
        class.push_str(" hide-risk-overlay");
    }
    class
}

fn wip_pct(task_count: usize, capacity: usize) -> usize {
    if capacity == 0 {
        0
    } else {
        ((task_count * 100) / capacity).min(100)
    }
}
