//! Annotation primitives: reference lines, bands, text callouts, arrows.
//!
//! Annotations are first-class because users edit them. Each annotation is
//! a self-contained mark that overlays an existing chart; consumers add them
//! by inserting an `AnnotationLayer` mark into their scene.

use berthacharts_core::{
    Geometry, LabelItem, LabelPriority, LinePrim, Mark, MarkId, PathCommand, PathPrim, PickCtx,
    PickHit, Rect, RectPrim, TessellateCtx,
};

/// Where a reference line sits in coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AxisRef {
    /// Horizontal line at constant y (pixels).
    Horizontal(f32),
    /// Vertical line at constant x (pixels).
    Vertical(f32),
}

/// A reference line spanning the plot.
#[derive(Debug, Clone)]
pub struct ReferenceLine {
    /// Axis reference (horizontal at y / vertical at x).
    pub axis: AxisRef,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
    /// Stroke width in pixels.
    pub width: f32,
    /// Optional dash pattern.
    pub dash: Option<Vec<f32>>,
}

impl ReferenceLine {
    /// Horizontal reference at y.
    #[must_use]
    pub fn horizontal(y: f32) -> Self {
        Self {
            axis: AxisRef::Horizontal(y),
            color: [0.85, 0.6, 0.2, 0.9],
            width: 1.2,
            dash: Some(vec![6.0, 4.0]),
        }
    }

    /// Vertical reference at x.
    #[must_use]
    pub fn vertical(x: f32) -> Self {
        Self {
            axis: AxisRef::Vertical(x),
            color: [0.85, 0.6, 0.2, 0.9],
            width: 1.2,
            dash: Some(vec![6.0, 4.0]),
        }
    }

    /// Override colour.
    #[must_use]
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Override width.
    #[must_use]
    pub const fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set a custom dash pattern (`None` for solid).
    #[must_use]
    pub fn with_dash(mut self, dash: Option<Vec<f32>>) -> Self {
        self.dash = dash;
        self
    }
}

/// A reference band — translucent rectangle spanning the plot between two
/// values along one axis.
#[derive(Debug, Clone)]
pub struct ReferenceBand {
    /// Axis the band spans.
    pub axis: BandAxis,
    /// Premultiplied RGBA fill.
    pub fill: [f32; 4],
    /// Optional border stroke.
    pub stroke: [f32; 4],
    /// Border width (0 = no border).
    pub stroke_width: f32,
}

/// Whether a reference band is horizontal or vertical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandAxis {
    /// Horizontal band from y1 to y2.
    Horizontal { y1: f32, y2: f32 },
    /// Vertical band from x1 to x2.
    Vertical { x1: f32, x2: f32 },
}

impl ReferenceBand {
    /// Horizontal band spanning the plot vertically between y1 and y2.
    #[must_use]
    pub fn horizontal(y1: f32, y2: f32) -> Self {
        Self {
            axis: BandAxis::Horizontal { y1, y2 },
            fill: [0.95, 0.85, 0.4, 0.18],
            stroke: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
        }
    }

    /// Vertical band spanning the plot horizontally between x1 and x2.
    #[must_use]
    pub fn vertical(x1: f32, x2: f32) -> Self {
        Self {
            axis: BandAxis::Vertical { x1, x2 },
            fill: [0.95, 0.85, 0.4, 0.18],
            stroke: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
        }
    }

    /// Override fill.
    #[must_use]
    pub const fn with_fill(mut self, fill: [f32; 4]) -> Self {
        self.fill = fill;
        self
    }

    /// Add a border.
    #[must_use]
    pub const fn with_border(mut self, stroke: [f32; 4], width: f32) -> Self {
        self.stroke = stroke;
        self.stroke_width = width;
        self
    }
}

/// An arrow from `from` to `to` in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow {
    /// Tail point.
    pub from: [f32; 2],
    /// Tip point.
    pub to: [f32; 2],
    /// Stroke colour.
    pub color: [f32; 4],
    /// Stroke width.
    pub width: f32,
    /// Length of each arrowhead leg (pixels).
    pub head_size: f32,
}

impl Arrow {
    /// Build a default-styled arrow.
    #[must_use]
    pub fn new(from: [f32; 2], to: [f32; 2]) -> Self {
        Self {
            from,
            to,
            color: [0.2, 0.2, 0.25, 1.0],
            width: 1.5,
            head_size: 8.0,
        }
    }

    /// Override colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// A bracket comparing two endpoints with an optional vertical lift, used to
/// annotate "p < 0.001 between A and B" style group comparisons.
#[derive(Debug, Clone)]
pub struct StatBracket {
    /// Left endpoint of the bracket base.
    pub from: [f32; 2],
    /// Right endpoint of the bracket base.
    pub to: [f32; 2],
    /// Lift in pixels (height of the bracket "legs").
    pub lift: f32,
    /// Stroke colour.
    pub color: [f32; 4],
    /// Stroke width.
    pub width: f32,
    /// Optional label text. Rendered through a `LabelItem` (use
    /// [`AnnotationLayer::label_items`]).
    pub label: Option<String>,
}

impl StatBracket {
    /// Build a bracket with default styling.
    #[must_use]
    pub fn new(from: [f32; 2], to: [f32; 2]) -> Self {
        Self {
            from,
            to,
            lift: 12.0,
            color: [0.2, 0.2, 0.25, 1.0],
            width: 1.2,
            label: None,
        }
    }

    /// Attach a label string.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Override colour.
    #[must_use]
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// A text callout: a label anchored at one point, optionally with a leader
/// line to a different "target" point. Text rendering happens via the
/// `LabelGuide` (call [`AnnotationLayer::label_items`] to extract them).
#[derive(Debug, Clone)]
pub struct TextCallout {
    /// Label anchor (where the text sits).
    pub anchor: [f32; 2],
    /// Optional leader-line target. `None` = no leader.
    pub target: Option<[f32; 2]>,
    /// Label text.
    pub text: String,
    /// Leader stroke colour.
    pub leader_color: [f32; 4],
    /// Leader stroke width.
    pub leader_width: f32,
}

impl TextCallout {
    /// Build a callout at `anchor` with the given text.
    #[must_use]
    pub fn new(anchor: [f32; 2], text: impl Into<String>) -> Self {
        Self {
            anchor,
            target: None,
            text: text.into(),
            leader_color: [0.4, 0.4, 0.45, 0.8],
            leader_width: 1.0,
        }
    }

    /// Point a leader line at the given target.
    #[must_use]
    pub fn pointing_at(mut self, target: [f32; 2]) -> Self {
        self.target = Some(target);
        self
    }
}

/// Confidence ribbon: filled polygon between an upper polyline and a lower
/// polyline. Typical use: confidence intervals around a trend line.
#[derive(Debug, Clone)]
pub struct ConfidenceRibbon {
    /// Upper bound polyline (must have the same length as `lower`).
    pub upper: Vec<[f32; 2]>,
    /// Lower bound polyline.
    pub lower: Vec<[f32; 2]>,
    /// Premultiplied RGBA fill.
    pub fill: [f32; 4],
    /// Optional border stroke.
    pub stroke: [f32; 4],
    /// Stroke width.
    pub stroke_width: f32,
}

impl ConfidenceRibbon {
    /// Build a ribbon from upper/lower polylines.
    #[must_use]
    pub fn new(upper: Vec<[f32; 2]>, lower: Vec<[f32; 2]>) -> Self {
        Self {
            upper,
            lower,
            fill: [0.45, 0.55, 0.85, 0.18],
            stroke: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
        }
    }

    /// Override fill colour.
    #[must_use]
    pub const fn with_fill(mut self, fill: [f32; 4]) -> Self {
        self.fill = fill;
        self
    }
}

/// A composite annotation layer that renders all attached annotations in one mark.
#[derive(Debug, Clone)]
pub struct AnnotationLayer {
    id: MarkId,
    plot: Rect,
    lines: Vec<ReferenceLine>,
    bands: Vec<ReferenceBand>,
    arrows: Vec<Arrow>,
    brackets: Vec<StatBracket>,
    callouts: Vec<TextCallout>,
    ribbons: Vec<ConfidenceRibbon>,
}

impl AnnotationLayer {
    /// Build an empty annotation layer bound to a plot region.
    #[must_use]
    pub fn new(id: MarkId, plot: Rect) -> Self {
        Self {
            id,
            plot,
            lines: Vec::new(),
            bands: Vec::new(),
            arrows: Vec::new(),
            brackets: Vec::new(),
            callouts: Vec::new(),
            ribbons: Vec::new(),
        }
    }

    /// Add a reference line.
    #[must_use]
    pub fn with_line(mut self, line: ReferenceLine) -> Self {
        self.lines.push(line);
        self
    }

    /// Add a reference band.
    #[must_use]
    pub fn with_band(mut self, band: ReferenceBand) -> Self {
        self.bands.push(band);
        self
    }

    /// Add an arrow.
    #[must_use]
    pub fn with_arrow(mut self, arrow: Arrow) -> Self {
        self.arrows.push(arrow);
        self
    }

    /// Add a stat bracket.
    #[must_use]
    pub fn with_bracket(mut self, bracket: StatBracket) -> Self {
        self.brackets.push(bracket);
        self
    }

    /// Add a text callout.
    #[must_use]
    pub fn with_callout(mut self, callout: TextCallout) -> Self {
        self.callouts.push(callout);
        self
    }

    /// Add a confidence ribbon.
    #[must_use]
    pub fn with_ribbon(mut self, ribbon: ConfidenceRibbon) -> Self {
        self.ribbons.push(ribbon);
        self
    }

    /// Extract label items from text callouts + bracketed labels for the
    /// consumer's `LabelGuide`. Mark tessellation handles only the geometric
    /// parts (leader lines, bracket shapes) — text always goes through the
    /// DOM overlay per the bertha core invariant.
    #[must_use]
    pub fn label_items(&self) -> Vec<LabelItem> {
        let mut items = Vec::new();
        for c in &self.callouts {
            items.push(
                LabelItem::new(c.anchor[0], c.anchor[1], c.text.clone())
                    .with_priority(LabelPriority::Important),
            );
        }
        for b in &self.brackets {
            if let Some(text) = &b.label {
                let mx = (b.from[0] + b.to[0]) * 0.5;
                let my = (b.from[1] + b.to[1]) * 0.5 - b.lift - 4.0;
                items.push(
                    LabelItem::new(mx, my, text.clone()).with_priority(LabelPriority::Important),
                );
            }
        }
        items
    }

    /// Total number of annotations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
            + self.bands.len()
            + self.arrows.len()
            + self.brackets.len()
            + self.callouts.len()
            + self.ribbons.len()
    }

    /// True when no annotations are attached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Mark for AnnotationLayer {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.lines.len() as u64;
        h ^= (self.bands.len() as u64).rotate_left(13);
        h ^= (self.arrows.len() as u64).rotate_left(31);
        h ^= (self.brackets.len() as u64).rotate_left(7);
        h ^= (self.callouts.len() as u64).rotate_left(19);
        h ^= (self.ribbons.len() as u64).rotate_left(23);
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let mut rects: Vec<RectPrim> = Vec::new();
        let mut lines: Vec<LinePrim> = Vec::new();
        let mut paths: Vec<PathPrim> = Vec::new();

        for band in &self.bands {
            let (x, y, w, h) = match band.axis {
                BandAxis::Horizontal { y1, y2 } => {
                    let top = y1.min(y2);
                    let bot = y1.max(y2);
                    (self.plot.x, top, self.plot.w, bot - top)
                }
                BandAxis::Vertical { x1, x2 } => {
                    let l = x1.min(x2);
                    let r = x1.max(x2);
                    (l, self.plot.y, r - l, self.plot.h)
                }
            };
            rects.push(RectPrim {
                x,
                y,
                w,
                h,
                fill: band.fill,
                stroke: band.stroke,
                stroke_width: band.stroke_width,
                radius: 0.0,
            });
        }

        for line in &self.lines {
            let pts = match line.axis {
                AxisRef::Horizontal(y) => {
                    vec![[self.plot.x, y], [self.plot.x + self.plot.w, y]]
                }
                AxisRef::Vertical(x) => {
                    vec![[x, self.plot.y], [x, self.plot.y + self.plot.h]]
                }
            };
            lines.push(LinePrim {
                points: pts,
                stroke: line.color,
                width: line.width,
                dash: line.dash.clone(),
                join: 1,
                cap: 1,
            });
        }

        for arrow in &self.arrows {
            paths.push(arrow_path(arrow));
        }

        for bracket in &self.brackets {
            for ln in bracket_lines(bracket) {
                lines.push(ln);
            }
        }

        for callout in &self.callouts {
            if let Some(target) = callout.target {
                lines.push(LinePrim {
                    points: vec![callout.anchor, target],
                    stroke: callout.leader_color,
                    width: callout.leader_width,
                    dash: None,
                    join: 1,
                    cap: 1,
                });
            }
        }

        for ribbon in &self.ribbons {
            if let Some(path) = ribbon_path(ribbon) {
                paths.push(path);
            }
        }

        let mut parts = Vec::new();
        if !rects.is_empty() {
            parts.push(Geometry::Rects(rects));
        }
        if !lines.is_empty() {
            parts.push(Geometry::Lines(lines));
        }
        if !paths.is_empty() {
            parts.push(Geometry::Paths(paths));
        }
        match parts.len() {
            0 => Geometry::Empty,
            1 => parts.into_iter().next().unwrap(),
            _ => Geometry::Mixed(parts),
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        // v0.1: annotations are decorative — interaction lives in the consumer overlay.
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.plot
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn arrow_path(arrow: &Arrow) -> PathPrim {
    let dx = arrow.to[0] - arrow.from[0];
    let dy = arrow.to[1] - arrow.from[1];
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    // perpendicular
    let px = -uy;
    let py = ux;
    let head = arrow.head_size;
    // base of arrowhead: head pixels back from the tip along the line.
    let base_x = arrow.to[0] - ux * head;
    let base_y = arrow.to[1] - uy * head;
    let left = [base_x + px * head * 0.45, base_y + py * head * 0.45];
    let right = [base_x - px * head * 0.45, base_y - py * head * 0.45];
    PathPrim {
        commands: vec![
            PathCommand::MoveTo {
                x: arrow.from[0],
                y: arrow.from[1],
            },
            PathCommand::LineTo {
                x: base_x,
                y: base_y,
            },
            PathCommand::MoveTo {
                x: arrow.to[0],
                y: arrow.to[1],
            },
            PathCommand::LineTo {
                x: left[0],
                y: left[1],
            },
            PathCommand::LineTo {
                x: right[0],
                y: right[1],
            },
            PathCommand::Close,
        ],
        fill: arrow.color,
        stroke: arrow.color,
        stroke_width: arrow.width,
    }
}

fn bracket_lines(bracket: &StatBracket) -> Vec<LinePrim> {
    let base_y = bracket.from[1].min(bracket.to[1]);
    let top_y = base_y - bracket.lift;
    let left_x = bracket.from[0];
    let right_x = bracket.to[0];
    let make = |pts: Vec<[f32; 2]>| LinePrim {
        points: pts,
        stroke: bracket.color,
        width: bracket.width,
        dash: None,
        join: 1,
        cap: 1,
    };
    vec![
        // Left leg (drop from top to base)
        make(vec![[left_x, top_y], [left_x, base_y]]),
        // Horizontal span
        make(vec![[left_x, top_y], [right_x, top_y]]),
        // Right leg
        make(vec![[right_x, top_y], [right_x, base_y]]),
    ]
}

fn ribbon_path(ribbon: &ConfidenceRibbon) -> Option<PathPrim> {
    if ribbon.upper.len() < 2 || ribbon.upper.len() != ribbon.lower.len() {
        return None;
    }
    let mut commands = Vec::with_capacity(ribbon.upper.len() * 2 + 2);
    let first = ribbon.upper[0];
    commands.push(PathCommand::MoveTo {
        x: first[0],
        y: first[1],
    });
    for p in &ribbon.upper[1..] {
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    for p in ribbon.lower.iter().rev() {
        commands.push(PathCommand::LineTo { x: p[0], y: p[1] });
    }
    commands.push(PathCommand::Close);
    Some(PathPrim {
        commands,
        fill: ribbon.fill,
        stroke: ribbon.stroke,
        stroke_width: ribbon.stroke_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_layer_reports_empty() {
        let layer = AnnotationLayer::new(MarkId::new(1), Rect::new(0.0, 0.0, 400.0, 300.0));
        assert!(layer.is_empty());
        assert_eq!(layer.len(), 0);
    }

    #[test]
    fn layer_counts_attached_annotations() {
        let layer = AnnotationLayer::new(MarkId::new(1), Rect::new(0.0, 0.0, 400.0, 300.0))
            .with_line(ReferenceLine::horizontal(100.0))
            .with_band(ReferenceBand::vertical(50.0, 150.0))
            .with_arrow(Arrow::new([10.0, 10.0], [100.0, 100.0]));
        assert_eq!(layer.len(), 3);
        assert!(!layer.is_empty());
    }

    #[test]
    fn bracket_emits_three_segments() {
        let lines = bracket_lines(&StatBracket::new([0.0, 100.0], [50.0, 100.0]));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn bracket_label_surfaces_in_label_items() {
        let layer = AnnotationLayer::new(MarkId::new(1), Rect::new(0.0, 0.0, 400.0, 300.0))
            .with_bracket(StatBracket::new([10.0, 100.0], [50.0, 100.0]).with_label("p < 0.001"));
        let items = layer.label_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "p < 0.001");
    }

    #[test]
    fn ribbon_path_traces_upper_then_lower_reverse() {
        let ribbon = ConfidenceRibbon::new(
            vec![[0.0, 0.0], [10.0, 0.0], [20.0, 0.0]],
            vec![[0.0, 10.0], [10.0, 10.0], [20.0, 10.0]],
        );
        let path = ribbon_path(&ribbon).expect("ribbon path");
        // 1 MoveTo + 2 LineTo (upper) + 3 LineTo (lower reversed) + Close = 7
        assert_eq!(path.commands.len(), 7);
    }

    #[test]
    fn ribbon_with_mismatched_lengths_returns_none() {
        let r = ConfidenceRibbon::new(vec![[0.0, 0.0], [1.0, 0.0]], vec![[0.0, 1.0]]);
        assert!(ribbon_path(&r).is_none());
    }

    #[test]
    fn callout_label_extracted() {
        let layer = AnnotationLayer::new(MarkId::new(1), Rect::new(0.0, 0.0, 400.0, 300.0))
            .with_callout(TextCallout::new([20.0, 30.0], "outlier"));
        let items = layer.label_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "outlier");
    }

    #[test]
    fn fingerprint_changes_when_annotations_added() {
        let base = AnnotationLayer::new(MarkId::new(1), Rect::new(0.0, 0.0, 400.0, 300.0));
        let with_line = base.clone().with_line(ReferenceLine::horizontal(50.0));
        assert_ne!(base.fingerprint(), with_line.fingerprint());
    }
}
