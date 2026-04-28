//! DOM/SVG guide rendering for chart chrome.
//!
//! The WebGL renderer owns marks. This module owns browser-readable overlays:
//! axes, legends, data labels, and HTML tooltip markup.

use berthacharts_core::{
    AxisOrient, Chart, Column, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind, LabelTooltip,
    LegendAnchor, LegendGuide, PickHit, Rect, Tick, TooltipField, TooltipGuide, TooltipValueFormat,
};

#[derive(Debug, Default)]
pub struct RenderedGuides {
    pub overlay: String,
    pub flow_top: String,
    pub flow_bottom: String,
}

pub fn render_guides_html(chart: &Chart, width: u32, height: u32) -> RenderedGuides {
    let scene = chart.scene();
    let plot = scene.viewport.plot_area;
    let mut svg =
        format!("<svg class=\"guide-svg\" viewBox=\"0 0 {width} {height}\" aria-hidden=\"true\">");
    let mut overlay_html = String::new();
    let mut flow_top = String::new();
    let mut flow_bottom = String::new();

    for guide in &scene.guides {
        match guide {
            Guide::Axis(axis) => {
                let Some(scale) = chart.workspace().scale(axis.scale) else {
                    continue;
                };
                let ticks = scale.ticks(axis.tick_count);
                render_axis_svg(
                    &mut svg,
                    axis.orient,
                    axis.tick_size,
                    axis.label.as_deref(),
                    &ticks,
                    plot,
                );
            }
            Guide::Legend(legend) => {
                let target = match legend.anchor {
                    LegendAnchor::Top => &mut flow_top,
                    LegendAnchor::Bottom => &mut flow_bottom,
                    _ => &mut overlay_html,
                };
                render_legend_html(target, legend);
            }
            Guide::Labels(labels) => {
                render_label_guide_html(&mut overlay_html, labels, width, height)
            }
            Guide::Tooltip(_) => {}
            _ => {}
        }
    }

    svg.push_str("</svg>");
    svg.push_str(&overlay_html);
    RenderedGuides {
        overlay: svg,
        flow_top,
        flow_bottom,
    }
}

pub fn render_tooltip_html(chart: &Chart, hit: &PickHit) -> Option<String> {
    let row = hit.row?;
    let guide = chart.scene().guides.iter().find_map(|guide| match guide {
        Guide::Tooltip(tooltip) if tooltip.mark == hit.mark => Some(tooltip),
        _ => None,
    })?;

    let title = guide
        .title_column
        .as_deref()
        .and_then(|column| read_column_auto_string(chart, guide, column, row))
        .unwrap_or_else(|| format!("Row {}", row + 1));

    let mut html = format!(
        "<div class=\"chart-tooltip-title\">{}</div>",
        escape_html(&title)
    );
    for field in &guide.fields {
        if let Some(value) = read_tooltip_field_string(chart, guide, field, row) {
            html.push_str(&format!(
                "<div class=\"chart-tooltip-row\"><span>{}</span><strong>{}</strong></div>",
                escape_html(&field.label),
                escape_html(&value),
            ));
        }
    }
    Some(html)
}

fn render_legend_html(html: &mut String, legend: &LegendGuide) {
    let title = legend.title.as_deref().unwrap_or("Legend");
    html.push_str(&format!(
        "<div class=\"guide-legend {}\"><div class=\"guide-legend-title\">{}</div>",
        legend_anchor_class(legend.anchor),
        escape_html(title)
    ));
    for item in &legend.items {
        html.push_str(&format!(
            "<div class=\"guide-legend-item\"><span class=\"guide-swatch\" style=\"background:{}\"></span><span>{}</span></div>",
            rgba_css(item.color),
            escape_html(&item.label)
        ));
    }
    html.push_str("</div>");
}

#[derive(Debug, Clone, Copy)]
struct LabelBox {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl LabelBox {
    fn overlaps(self, other: Self, padding: f32) -> bool {
        self.x < other.x + other.w + padding
            && self.x + self.w + padding > other.x
            && self.y < other.y + other.h + padding
            && self.y + self.h + padding > other.y
    }
}

fn render_label_guide_html(html: &mut String, guide: &LabelGuide, width: u32, height: u32) {
    let mut order: Vec<usize> = (0..guide.items.len()).collect();
    order.sort_by_key(|index| (guide.items[*index].priority, *index));

    let budget = guide.max_visible.unwrap_or(guide.items.len());
    let mut placed: Vec<(usize, LabelBox)> = Vec::new();

    for index in order {
        if placed.len() >= budget {
            break;
        }
        let item = &guide.items[index];
        let size = estimate_label_size(item);
        let Some(rect) = place_label(
            item,
            size,
            width as f32,
            height as f32,
            &placed,
            guide.collision_padding,
        ) else {
            continue;
        };
        placed.push((index, rect));
    }

    placed.sort_by_key(|(index, _)| *index);
    for (index, rect) in placed {
        let item = &guide.items[index];
        let label_class = if item.tooltip.is_some() {
            format!("{} guide-label-has-tooltip", label_kind_class(item.kind))
        } else {
            label_kind_class(item.kind).to_string()
        };
        html.push_str(&format!(
            "<span class=\"guide-label {}\" style=\"left:{:.1}px;top:{:.1}px\"{}>",
            label_class,
            rect.x,
            rect.y,
            if item.tooltip.is_some() {
                " tabindex=\"0\""
            } else {
                ""
            },
        ));
        html.push_str(&format!("<strong>{}</strong>", escape_html(&item.text)));
        if let Some(detail) = &item.detail {
            html.push_str(&format!("<em>{}</em>", escape_html(detail)));
        }
        if let Some(tooltip) = &item.tooltip {
            render_label_tooltip_html(html, tooltip, item.y < 190.0);
        }
        html.push_str("</span>");
    }
}

fn render_label_tooltip_html(html: &mut String, tooltip: &LabelTooltip, open_below: bool) {
    let class = if open_below {
        "guide-label-tooltip guide-label-tooltip-below"
    } else {
        "guide-label-tooltip"
    };
    html.push_str(&format!("<span class=\"{class}\">"));
    html.push_str(&format!(
        "<span class=\"chart-tooltip-title\">{}</span>",
        escape_html(&tooltip.title)
    ));
    for row in &tooltip.rows {
        html.push_str(&format!(
            "<span class=\"chart-tooltip-row\"><span>{}</span><strong>{}</strong></span>",
            escape_html(&row.label),
            escape_html(&row.value),
        ));
    }
    html.push_str("</span>");
}

fn place_label(
    item: &LabelItem,
    size: (f32, f32),
    width: f32,
    height: f32,
    placed: &[(usize, LabelBox)],
    padding: f32,
) -> Option<LabelBox> {
    for anchor in label_anchor_candidates(item) {
        let rect = label_box_for_anchor(item.x, item.y, size.0, size.1, anchor, width, height);
        if placed
            .iter()
            .all(|(_, other)| !rect.overlaps(*other, padding))
        {
            return Some(rect);
        }
    }
    None
}

fn label_anchor_candidates(item: &LabelItem) -> Vec<LabelAnchor> {
    if !item.allow_reposition {
        return vec![item.anchor];
    }

    let candidates = [
        item.anchor,
        LabelAnchor::Top,
        LabelAnchor::Bottom,
        LabelAnchor::Right,
        LabelAnchor::Left,
        LabelAnchor::TopRight,
        LabelAnchor::TopLeft,
        LabelAnchor::BottomRight,
        LabelAnchor::BottomLeft,
        LabelAnchor::Center,
    ];
    let mut out = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn label_box_for_anchor(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    anchor: LabelAnchor,
    width: f32,
    height: f32,
) -> LabelBox {
    let offset = 6.0;
    let (left, top) = match anchor {
        LabelAnchor::Center => (x - w * 0.5, y - h * 0.5),
        LabelAnchor::Top => (x - w * 0.5, y - h - offset),
        LabelAnchor::Bottom => (x - w * 0.5, y + offset),
        LabelAnchor::Left => (x - w - offset, y - h * 0.5),
        LabelAnchor::Right => (x + offset, y - h * 0.5),
        LabelAnchor::TopLeft => (x - w - offset, y - h - offset),
        LabelAnchor::TopRight => (x + offset, y - h - offset),
        LabelAnchor::BottomLeft => (x - w - offset, y + offset),
        LabelAnchor::BottomRight => (x + offset, y + offset),
        _ => (x - w * 0.5, y - h * 0.5),
    };
    clamp_label_box(
        LabelBox {
            x: left,
            y: top,
            w,
            h,
        },
        width,
        height,
    )
}

fn clamp_label_box(rect: LabelBox, width: f32, height: f32) -> LabelBox {
    let margin = 2.0;
    LabelBox {
        x: rect.x.clamp(margin, (width - rect.w - margin).max(margin)),
        y: rect.y.clamp(margin, (height - rect.h - margin).max(margin)),
        ..rect
    }
}

fn estimate_label_size(item: &LabelItem) -> (f32, f32) {
    let primary = item.text.chars().count() as f32;
    let detail = item
        .detail
        .as_deref()
        .map_or(0.0, |text| text.chars().count() as f32);
    match item.kind {
        LabelKind::Column => ((primary.max(detail) * 5.7 + 14.0).clamp(48.0, 118.0), 31.0),
        LabelKind::Data | LabelKind::Flow => (primary * 7.0 + detail * 5.8 + 25.0, 22.0),
        LabelKind::Annotation => ((primary.max(detail) * 6.2 + 18.0).clamp(72.0, 180.0), 34.0),
        LabelKind::Node | _ => (primary * 6.7 + detail * 5.8 + 25.0, 24.0),
    }
}

fn label_kind_class(kind: LabelKind) -> &'static str {
    match kind {
        LabelKind::Node => "guide-label-node",
        LabelKind::Data => "guide-label-data",
        LabelKind::Flow => "guide-label-flow",
        LabelKind::Column => "guide-label-column",
        LabelKind::Annotation => "guide-label-annotation",
        _ => "guide-label-node",
    }
}

fn read_tooltip_field_string(
    chart: &Chart,
    guide: &TooltipGuide,
    field: &TooltipField,
    row: usize,
) -> Option<String> {
    let dataset = chart.workspace().dataset(guide.dataset)?;
    let column = dataset.column(&field.column)?;
    format_column_value(column.as_ref(), row, field.format)
}

fn read_column_auto_string(
    chart: &Chart,
    guide: &TooltipGuide,
    column: &str,
    row: usize,
) -> Option<String> {
    let dataset = chart.workspace().dataset(guide.dataset)?;
    let column = dataset.column(column)?;
    format_column_value(column.as_ref(), row, TooltipValueFormat::Auto)
}

fn format_column_value(column: &Column, row: usize, format: TooltipValueFormat) -> Option<String> {
    if let Column::Utf8(data) = column {
        let value = data.values.get(row)?;
        return Some(match format {
            TooltipValueFormat::Label => format_label(value),
            _ => value.to_string(),
        });
    }

    if let Column::Bool(data) = column {
        let value = *data.values.get(row)?;
        return Some(match format {
            TooltipValueFormat::Label => format_label(if value { "true" } else { "false" }),
            _ => value.to_string(),
        });
    }

    let value = column.read_f64(row)?;
    Some(match format {
        TooltipValueFormat::Auto => format_number(value),
        TooltipValueFormat::Integer => format_fixed(value, 0),
        TooltipValueFormat::Number { decimals } => format_fixed(value, decimals),
        TooltipValueFormat::Percent { decimals } => {
            format!("{}%", format_fixed(value, decimals))
        }
        TooltipValueFormat::SignedNumber { decimals } => format_signed(value, decimals),
        TooltipValueFormat::SignedPercent { decimals } => {
            format!("{}%", format_signed(value, decimals))
        }
        TooltipValueFormat::Label => format_number(value),
        _ => format_number(value),
    })
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn format_fixed(value: f64, decimals: u8) -> String {
    let precision = decimals as usize;
    format!("{value:.precision$}")
}

fn format_signed(value: f64, decimals: u8) -> String {
    let formatted = format_fixed(value.abs(), decimals);
    if value >= 0.0 {
        format!("+{formatted}")
    } else {
        format!("-{formatted}")
    }
}

fn format_label(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::with_capacity(value.len());
    out.push(first.to_ascii_uppercase());
    out.push_str(chars.as_str());
    out
}

fn render_axis_svg(
    svg: &mut String,
    orient: AxisOrient,
    tick_size: f32,
    label: Option<&str>,
    ticks: &[Tick],
    plot: Rect,
) {
    let x0 = plot.x;
    let x1 = plot.x + plot.w;
    let y0 = plot.y;
    let y1 = plot.y + plot.h;

    match orient {
        AxisOrient::Bottom => {
            svg.push_str(&format!(
                "<g class=\"guide-axis guide-axis-bottom\"><line x1=\"{x0}\" y1=\"{y1}\" x2=\"{x1}\" y2=\"{y1}\" />"
            ));
            for tick in ticks {
                let x = tick.position;
                let label = escape_html(&tick.label);
                svg.push_str(&format!(
                    "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{}\" /><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\">{label}</text>",
                    y1 + tick_size,
                    y1 + tick_size + 14.0,
                ));
            }
            if let Some(label) = label {
                svg.push_str(&format!(
                    "<text class=\"guide-axis-title\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                    x0 + plot.w * 0.5,
                    y1 + 34.0,
                    escape_html(label),
                ));
            }
            svg.push_str("</g>");
        }
        AxisOrient::Top => {
            svg.push_str(&format!(
                "<g class=\"guide-axis guide-axis-top\"><line x1=\"{x0}\" y1=\"{y0}\" x2=\"{x1}\" y2=\"{y0}\" />"
            ));
            for tick in ticks {
                let x = tick.position;
                let label = escape_html(&tick.label);
                svg.push_str(&format!(
                    "<line x1=\"{x}\" y1=\"{y0}\" x2=\"{x}\" y2=\"{}\" /><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\">{label}</text>",
                    y0 - tick_size,
                    y0 - tick_size - 4.0,
                ));
            }
            svg.push_str("</g>");
        }
        AxisOrient::Left => {
            svg.push_str(&format!(
                "<g class=\"guide-axis guide-axis-left\"><line x1=\"{x0}\" y1=\"{y0}\" x2=\"{x0}\" y2=\"{y1}\" />"
            ));
            for tick in ticks {
                let y = tick.position;
                let label = escape_html(&tick.label);
                svg.push_str(&format!(
                    "<line x1=\"{x0}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" /><text x=\"{}\" y=\"{}\" text-anchor=\"end\">{label}</text>",
                    x0 - tick_size,
                    x0 - tick_size - 4.0,
                    y + 4.0,
                ));
            }
            if let Some(label) = label {
                svg.push_str(&format!(
                    "<text class=\"guide-axis-title\" transform=\"translate({}, {}) rotate(-90)\" text-anchor=\"middle\">{}</text>",
                    14.0,
                    y0 + plot.h * 0.5,
                    escape_html(label),
                ));
            }
            svg.push_str("</g>");
        }
        AxisOrient::Right => {
            svg.push_str(&format!(
                "<g class=\"guide-axis guide-axis-right\"><line x1=\"{x1}\" y1=\"{y0}\" x2=\"{x1}\" y2=\"{y1}\" />"
            ));
            for tick in ticks {
                let y = tick.position;
                let label = escape_html(&tick.label);
                svg.push_str(&format!(
                    "<line x1=\"{x1}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" /><text x=\"{}\" y=\"{}\">{label}</text>",
                    x1 + tick_size,
                    x1 + tick_size + 4.0,
                    y + 4.0,
                ));
            }
            svg.push_str("</g>");
        }
        _ => {}
    }
}

fn legend_anchor_class(anchor: LegendAnchor) -> &'static str {
    match anchor {
        LegendAnchor::Top => "legend-flow-top",
        LegendAnchor::Bottom => "legend-flow-bottom",
        LegendAnchor::TopLeft => "legend-top-left",
        LegendAnchor::TopRight => "legend-top-right",
        LegendAnchor::BottomLeft => "legend-bottom-left",
        LegendAnchor::BottomRight => "legend-bottom-right",
        _ => "legend-top-right",
    }
}

fn rgba_css(color: [f32; 4]) -> String {
    let [r, g, b, a] = color;
    format!(
        "rgba({}, {}, {}, {:.3})",
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        a.clamp(0.0, 1.0),
    )
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
