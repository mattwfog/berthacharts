//! Appointment scheduling heatmap built through the public chart spec.

use std::sync::Arc;

use berthacharts_charts::heatmap::{HeatmapCell, HeatmapOptions, HeatmapSpec};
use berthacharts_core::{ChartSize, ChartSpec};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 770;
const H: u32 = 506;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_headers = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let slots = appointment_slots();
    let insights = scheduling_insights(&slots);
    let recommendations = insights.recommendations.clone();
    let day_loads = insights.day_loads.clone();
    let slot_cards = insights.slot_cards.clone();
    let metrics = insights.metrics.clone();
    let action_rows = insights.action_rows.clone();
    let spec = Arc::new(appointment_heatmap_spec(&slots));
    let summary = spec.summary();
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo heatmap spec should be valid")
    });

    view! {
        <section id="appointment-heatmap" class="example">
            <div class="example-head">
                <div>
                    <h2>"Appointment Scheduling Heatmap"</h2>
                    <p>
                        "Booking pressure by day and start time, normalized against appointment capacity and compared with each day's baseline."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>{summary.cells.to_string()}</strong>" slots"</span>
                    <span><strong>{insights.avg_pressure.clone()}</strong>" avg pressure"</span>
                    <span><strong>{insights.open_seats.clone()}</strong>" open seats"</span>
                    <span><strong>{insights.peak_slot.clone()}</strong>" peak slot"</span>
                </div>
            </div>
            <DisplayControls label="Appointment heatmap display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Headers" state=show_headers />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class="appointment-metrics">
                {metrics.into_iter().map(|metric| {
                    view! {
                        <article>
                            <span>{metric.label}</span>
                            <strong>{metric.value}</strong>
                            <em>{metric.detail}</em>
                        </article>
                    }
                }).collect_view()}
            </div>
            <div class=move || heatmap_stage_class(
                show_data_labels.get(),
                show_headers.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
            <div class="appointment-insights">
                <div class="appointment-insight-block appointment-priorities">
                    <span>"Recommended scheduling moves"</span>
                    {recommendations.into_iter().map(|item| {
                        view! {
                            <article>
                                <strong>{item.title}</strong>
                                <em>{item.detail}</em>
                            </article>
                        }
                    }).collect_view()}
                </div>
                <div class="appointment-insight-block">
                    <span>"Day load"</span>
                    <div class="appointment-day-loads">
                        {day_loads.into_iter().map(|day| {
                            view! {
                                <div class="appointment-day-load">
                                    <strong>{day.day}</strong>
                                    <div class="appointment-load-track" aria-label=day.label.clone()>
                                        <i class="is-booked" style=format!("width:{}%", day.booked_pct)></i>
                                        <i class="is-open" style=format!("width:{}%", day.open_pct)></i>
                                        <i class="is-backlog" style=format!("width:{}%", day.backlog_pct)></i>
                                    </div>
                                    <em>{day.label}</em>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
            <div class="appointment-slot-grid">
                {slot_cards.into_iter().map(|slot| {
                    view! {
                        <article class=slot.class_name>
                            <span>{slot.label}</span>
                            <strong>{slot.headline}</strong>
                            <em>{slot.detail}</em>
                        </article>
                    }
                }).collect_view()}
            </div>
            <div class="appointment-action-queue">
                <div class="appointment-action-head">
                    <span>"Slot"</span><b>"Move"</b><b>"Priority"</b><b>"Why"</b>
                </div>
                {action_rows.into_iter().map(|row| {
                    view! {
                        <div class=row.class_name>
                            <span>{row.slot}</span>
                            <strong>{row.action}</strong>
                            <em>{row.priority}</em>
                            <i>{row.reason}</i>
                        </div>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

#[derive(Debug, Clone, Copy)]
struct AppointmentSlot {
    time: &'static str,
    day: &'static str,
    requests: u16,
    booked: u16,
    capacity: u16,
    no_show_risk: f32,
}

#[derive(Debug, Clone)]
struct SchedulingInsights {
    avg_pressure: String,
    open_seats: String,
    peak_slot: String,
    metrics: Vec<DecisionMetric>,
    recommendations: Vec<InsightItem>,
    day_loads: Vec<DayLoad>,
    slot_cards: Vec<SlotCard>,
    action_rows: Vec<ActionRow>,
}

#[derive(Debug, Clone)]
struct DecisionMetric {
    label: String,
    value: String,
    detail: String,
}

#[derive(Debug, Clone)]
struct InsightItem {
    title: String,
    detail: String,
}

#[derive(Debug, Clone)]
struct DayLoad {
    day: &'static str,
    booked_pct: u8,
    open_pct: u8,
    backlog_pct: u8,
    label: String,
}

#[derive(Debug, Clone)]
struct SlotCard {
    class_name: &'static str,
    label: String,
    headline: String,
    detail: String,
}

#[derive(Debug, Clone)]
struct ActionRow {
    class_name: &'static str,
    slot: String,
    action: String,
    priority: String,
    reason: String,
}

fn appointment_heatmap_spec(slots: &[AppointmentSlot]) -> HeatmapSpec {
    let times = [
        "8 AM", "9 AM", "10 AM", "11 AM", "12 PM", "1 PM", "2 PM", "3 PM", "4 PM", "5 PM", "6 PM",
    ];
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut cells = Vec::with_capacity(times.len() * days.len());

    for slot in slots {
        cells.push(
            HeatmapCell::new(slot.time, slot.day, booking_pressure(slot))
                .with_label_detail(format!("{}/{}", slot.booked, slot.capacity))
                .with_tooltip_detail_1(format!("{}", slot.requests))
                .with_tooltip_detail_2(format!("{}/{}", slot.booked, slot.capacity))
                .with_tooltip_detail_3(format!("{:.0}%", slot.no_show_risk * 100.0)),
        );
    }

    let options = HeatmapOptions {
        cell_padding: 4.0,
        signal_threshold: 0.10,
        legend_title: "Pressure vs day avg".to_string(),
        row_tooltip_label: "Time".to_string(),
        value_tooltip_label: "Pressure".to_string(),
        delta_tooltip_label: "Vs day avg".to_string(),
        signal_tooltip_label: "Slot type".to_string(),
        tooltip_detail_1_label: Some("Requests".to_string()),
        tooltip_detail_2_label: Some("Booked/cap".to_string()),
        tooltip_detail_3_label: Some("No-show risk".to_string()),
        strong_signal_label: "capacity pressure".to_string(),
        watch_signal_label: "open capacity".to_string(),
        neutral_signal_label: "balanced".to_string(),
        show_signal_glyphs: false,
        show_label_details: false,
        max_visible_labels: Some(times.len() * days.len()),
    };

    HeatmapSpec::new(cells)
        .with_rows(times.to_vec())
        .with_columns(days.to_vec())
        .with_options(options)
}

fn appointment_slots() -> Vec<AppointmentSlot> {
    const TIMES: [&str; 11] = [
        "8 AM", "9 AM", "10 AM", "11 AM", "12 PM", "1 PM", "2 PM", "3 PM", "4 PM", "5 PM", "6 PM",
    ];
    const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const REQUESTS: [[u16; 7]; 11] = [
        [8, 9, 10, 10, 9, 5, 3],
        [12, 13, 14, 15, 13, 7, 4],
        [15, 16, 18, 17, 15, 9, 5],
        [14, 15, 17, 18, 15, 10, 6],
        [10, 12, 12, 13, 12, 8, 4],
        [12, 13, 14, 15, 14, 9, 5],
        [16, 17, 19, 18, 16, 10, 6],
        [15, 16, 17, 17, 15, 8, 5],
        [11, 12, 13, 13, 12, 6, 4],
        [9, 11, 12, 13, 12, 4, 2],
        [5, 7, 9, 10, 8, 2, 1],
    ];
    const BOOKED: [[u16; 7]; 11] = [
        [7, 8, 8, 9, 8, 4, 2],
        [10, 11, 12, 13, 11, 6, 3],
        [13, 14, 15, 15, 13, 8, 4],
        [12, 13, 15, 15, 13, 9, 5],
        [8, 10, 10, 11, 10, 7, 3],
        [10, 11, 12, 13, 12, 8, 4],
        [14, 15, 16, 16, 14, 9, 5],
        [13, 14, 15, 15, 13, 7, 4],
        [9, 10, 11, 11, 10, 5, 3],
        [8, 9, 10, 11, 10, 3, 1],
        [4, 6, 7, 8, 7, 1, 1],
    ];
    const CAPACITY: [u16; 7] = [16, 16, 18, 18, 17, 12, 8];

    let mut slots = Vec::with_capacity(TIMES.len() * DAYS.len());
    for (row, time) in TIMES.iter().enumerate() {
        for (column, day) in DAYS.iter().enumerate() {
            slots.push(AppointmentSlot {
                time: *time,
                day: *day,
                requests: REQUESTS[row][column],
                booked: BOOKED[row][column],
                capacity: CAPACITY[column],
                no_show_risk: no_show_risk(row, column),
            });
        }
    }
    slots
}

fn scheduling_insights(slots: &[AppointmentSlot]) -> SchedulingInsights {
    let avg_pressure = if slots.is_empty() {
        String::from("0%")
    } else {
        format!(
            "{:.0}%",
            slots.iter().map(booking_pressure).sum::<f32>() / slots.len() as f32 * 100.0
        )
    };
    let peak = slots
        .iter()
        .max_by(|a, b| booking_pressure(a).total_cmp(&booking_pressure(b)));
    let best_open = slots
        .iter()
        .filter(|slot| booking_pressure(slot) <= 0.68 && slot.no_show_risk <= 0.12)
        .max_by(|a, b| slot_yield(a).total_cmp(&slot_yield(b)));
    let no_show = slots
        .iter()
        .max_by(|a, b| a.no_show_risk.total_cmp(&b.no_show_risk));
    let backlog = slots
        .iter()
        .filter(|slot| slot.requests > slot.capacity)
        .count();
    let open_seats: u16 = slots
        .iter()
        .map(|slot| slot.capacity.saturating_sub(slot.booked))
        .sum();
    let metrics = decision_metrics(slots);

    SchedulingInsights {
        avg_pressure,
        open_seats: open_seats.to_string(),
        peak_slot: peak.map(slot_label).unwrap_or_else(|| "n/a".to_string()),
        metrics,
        recommendations: vec![
            InsightItem {
                title: String::from("Add overflow capacity"),
                detail: peak
                    .map(|slot| {
                        format!(
                            "{} is at {:.0}% pressure with {} requests for {} seats.",
                            slot_label(slot),
                            booking_pressure(slot) * 100.0,
                            slot.requests,
                            slot.capacity
                        )
                    })
                    .unwrap_or_else(|| String::from("No capacity pressure detected.")),
            },
            InsightItem {
                title: String::from("Route flexible appointments"),
                detail: best_open
                    .map(|slot| {
                        format!(
                            "{} has {} open seats and low no-show risk.",
                            slot_label(slot),
                            slot.capacity.saturating_sub(slot.booked)
                        )
                    })
                    .unwrap_or_else(|| String::from("No low-risk open capacity found.")),
            },
            InsightItem {
                title: String::from("Protect confirmation workflow"),
                detail: no_show
                    .map(|slot| {
                        format!(
                            "{} carries the highest no-show risk at {:.0}%.",
                            slot_label(slot),
                            slot.no_show_risk * 100.0
                        )
                    })
                    .unwrap_or_else(|| String::from("No no-show risk data available.")),
            },
            InsightItem {
                title: String::from("Watch backlog"),
                detail: format!("{backlog} slots have more requests than scheduled capacity."),
            },
        ],
        day_loads: day_loads(slots),
        slot_cards: slot_cards(slots),
        action_rows: action_rows(slots),
    }
}

fn decision_metrics(slots: &[AppointmentSlot]) -> Vec<DecisionMetric> {
    let capacity: u16 = slots.iter().map(|slot| slot.capacity).sum();
    let booked: u16 = slots.iter().map(|slot| slot.booked).sum();
    let requests: u16 = slots.iter().map(|slot| slot.requests).sum();
    let backlog: u16 = slots
        .iter()
        .map(|slot| slot.requests.saturating_sub(slot.capacity))
        .sum();
    let expected_no_shows: f32 = slots
        .iter()
        .map(|slot| slot.booked as f32 * slot.no_show_risk)
        .sum();
    let expected_attendance: f32 = slots
        .iter()
        .map(|slot| slot.booked as f32 * (1.0 - slot.no_show_risk))
        .sum();
    let fill = percent(booked as f32, capacity as f32);
    let risk_adjusted_fill = percent(expected_attendance, capacity as f32);

    vec![
        DecisionMetric {
            label: String::from("Fill rate"),
            value: format!("{fill:.0}%"),
            detail: format!("{booked}/{capacity} booked capacity"),
        },
        DecisionMetric {
            label: String::from("Unmet demand"),
            value: backlog.to_string(),
            detail: format!("{requests} requests across the week"),
        },
        DecisionMetric {
            label: String::from("Expected no-shows"),
            value: format!("{expected_no_shows:.1}"),
            detail: String::from("risk-weighted scheduled visits"),
        },
        DecisionMetric {
            label: String::from("Risk-adjusted fill"),
            value: format!("{risk_adjusted_fill:.0}%"),
            detail: String::from("expected arrivals / capacity"),
        },
    ]
}

fn action_rows(slots: &[AppointmentSlot]) -> Vec<ActionRow> {
    let mut ranked: Vec<_> = slots
        .iter()
        .map(|slot| {
            let backlog = slot.requests.saturating_sub(slot.capacity);
            let open = slot.capacity.saturating_sub(slot.booked);
            let pressure = booking_pressure(slot);
            let score = backlog as f32 * 2.4
                + pressure * 1.4
                + slot.no_show_risk * 1.2
                + if open >= 4 { 0.45 } else { 0.0 };
            (score, slot)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));

    ranked
        .into_iter()
        .take(6)
        .map(|(_, slot)| action_row(slot))
        .collect()
}

fn action_row(slot: &AppointmentSlot) -> ActionRow {
    let backlog = slot.requests.saturating_sub(slot.capacity);
    let open = slot.capacity.saturating_sub(slot.booked);
    let pressure = booking_pressure(slot);

    if backlog > 0 {
        return ActionRow {
            class_name: "appointment-action-row is-critical",
            slot: slot_label(slot),
            action: String::from("Add capacity"),
            priority: String::from("P1"),
            reason: format!(
                "{backlog} excess requests at {:.0}% pressure",
                pressure * 100.0
            ),
        };
    }
    if slot.no_show_risk >= 0.14 && open > 0 {
        return ActionRow {
            class_name: "appointment-action-row is-risk",
            slot: slot_label(slot),
            action: String::from("Overbook lightly"),
            priority: String::from("P2"),
            reason: format!(
                "{:.0}% no-show risk with {open} open seats",
                slot.no_show_risk * 100.0
            ),
        };
    }
    if open >= 4 && pressure < 0.70 {
        return ActionRow {
            class_name: "appointment-action-row is-open",
            slot: slot_label(slot),
            action: String::from("Backfill"),
            priority: String::from("P2"),
            reason: format!("{open} open seats at {:.0}% pressure", pressure * 100.0),
        };
    }
    ActionRow {
        class_name: "appointment-action-row",
        slot: slot_label(slot),
        action: String::from("Protect"),
        priority: String::from("P3"),
        reason: format!("{} booked / {} capacity", slot.booked, slot.capacity),
    }
}

fn slot_cards(slots: &[AppointmentSlot]) -> Vec<SlotCard> {
    let best_growth = slots
        .iter()
        .filter(|slot| slot.capacity > slot.booked && booking_pressure(slot) >= 0.80)
        .max_by_key(|slot| slot.capacity.saturating_sub(slot.booked));
    let overbook = slots
        .iter()
        .filter(|slot| slot.no_show_risk >= 0.14 && slot.capacity > slot.booked)
        .max_by(|a, b| a.no_show_risk.total_cmp(&b.no_show_risk));
    let protect = slots
        .iter()
        .max_by(|a, b| booking_pressure(a).total_cmp(&booking_pressure(b)));
    let quiet = slots
        .iter()
        .filter(|slot| slot.capacity.saturating_sub(slot.booked) >= 4)
        .min_by(|a, b| booking_pressure(a).total_cmp(&booking_pressure(b)));

    vec![
        SlotCard {
            class_name: "appointment-slot-card is-growth",
            label: String::from("Best growth slot"),
            headline: best_growth
                .map(slot_label)
                .unwrap_or_else(|| String::from("No slot")),
            detail: best_growth
                .map(|slot| {
                    format!(
                        "{} open seats, {:.0}% pressure",
                        slot.capacity.saturating_sub(slot.booked),
                        booking_pressure(slot) * 100.0
                    )
                })
                .unwrap_or_else(|| String::from("No high-pressure open capacity.")),
        },
        SlotCard {
            class_name: "appointment-slot-card is-risk",
            label: String::from("Overbook candidate"),
            headline: overbook
                .map(slot_label)
                .unwrap_or_else(|| String::from("No slot")),
            detail: overbook
                .map(|slot| {
                    format!(
                        "{:.0}% no-show risk, {} open seats",
                        slot.no_show_risk * 100.0,
                        slot.capacity.saturating_sub(slot.booked)
                    )
                })
                .unwrap_or_else(|| String::from("No high-risk open slot.")),
        },
        SlotCard {
            class_name: "appointment-slot-card is-protect",
            label: String::from("Protect access"),
            headline: protect
                .map(slot_label)
                .unwrap_or_else(|| String::from("No slot")),
            detail: protect
                .map(|slot| format!("{} requests, {} scheduled", slot.requests, slot.booked))
                .unwrap_or_else(|| String::from("No pressure slot.")),
        },
        SlotCard {
            class_name: "appointment-slot-card is-open",
            label: String::from("Open capacity"),
            headline: quiet
                .map(slot_label)
                .unwrap_or_else(|| String::from("No slot")),
            detail: quiet
                .map(|slot| {
                    format!(
                        "{} open seats, {:.0}% pressure",
                        slot.capacity.saturating_sub(slot.booked),
                        booking_pressure(slot) * 100.0
                    )
                })
                .unwrap_or_else(|| String::from("No available quiet capacity.")),
        },
    ]
}

fn day_loads(slots: &[AppointmentSlot]) -> Vec<DayLoad> {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .map(|day| {
            let day_slots: Vec<_> = slots.iter().filter(|slot| slot.day == day).collect();
            let capacity: u16 = day_slots.iter().map(|slot| slot.capacity).sum();
            let booked: u16 = day_slots.iter().map(|slot| slot.booked).sum();
            let backlog: u16 = day_slots
                .iter()
                .map(|slot| slot.requests.saturating_sub(slot.capacity))
                .sum();
            let open = capacity.saturating_sub(booked);
            let base = (capacity + backlog).max(1) as f32;
            DayLoad {
                day,
                booked_pct: (booked as f32 / base * 100.0).round() as u8,
                open_pct: (open as f32 / base * 100.0).round() as u8,
                backlog_pct: (backlog as f32 / base * 100.0).round() as u8,
                label: format!("{booked}/{capacity} +{backlog}"),
            }
        })
        .collect()
}

fn booking_pressure(slot: &AppointmentSlot) -> f32 {
    if slot.capacity == 0 {
        0.0
    } else {
        (slot.requests as f32 / slot.capacity as f32).clamp(0.0, 1.0)
    }
}

fn slot_yield(slot: &AppointmentSlot) -> f32 {
    (slot.requests as f32 / slot.capacity.max(1) as f32) * (1.0 - slot.no_show_risk)
}

fn percent(numerator: f32, denominator: f32) -> f32 {
    if denominator <= 0.0 {
        0.0
    } else {
        numerator / denominator * 100.0
    }
}

fn slot_label(slot: &AppointmentSlot) -> String {
    format!("{} {}", slot.day, slot.time)
}

fn no_show_risk(row: usize, column: usize) -> f32 {
    let late_day = if row >= 9 { 0.05 } else { 0.0 };
    let weekend = if column >= 5 { 0.06 } else { 0.0 };
    let lunch = if row == 4 { 0.03 } else { 0.0 };
    0.06 + late_day + weekend + lunch
}

fn heatmap_stage_class(show_data_labels: bool, show_headers: bool, show_legend: bool) -> String {
    let mut class = String::from("chart-stage heatmap-stage appointment-heatmap-stage");
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_headers {
        class.push_str(" hide-column-labels");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}
