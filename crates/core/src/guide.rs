//! Guides: axes, legends, and overlay-facing chart furniture.
//!
//! Guides are part of the chart specification, but they are not GPU marks.
//! Bindings render them as DOM/SVG overlays so text layout, accessibility,
//! tooltips, and rich interaction remain outside the renderer.

use crate::ids::{DatasetId, MarkId, ScaleId};

/// Axis orientation around the plot area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AxisOrient {
    /// Axis above the plot area.
    Top,
    /// Axis to the right of the plot area.
    Right,
    /// Axis below the plot area.
    Bottom,
    /// Axis to the left of the plot area.
    Left,
}

/// Declarative axis guide bound to a scale.
#[derive(Debug, Clone)]
pub struct AxisGuide {
    /// Scale this axis visualizes.
    pub scale: ScaleId,
    /// Axis side.
    pub orient: AxisOrient,
    /// Optional axis title.
    pub label: Option<String>,
    /// Tick-count hint forwarded to [`crate::Scale::ticks`].
    pub tick_count: usize,
    /// Tick length in screen pixels.
    pub tick_size: f32,
}

impl AxisGuide {
    /// Build an axis for `scale` on `orient`.
    #[must_use]
    pub fn new(scale: ScaleId, orient: AxisOrient) -> Self {
        Self {
            scale,
            orient,
            label: None,
            tick_count: 6,
            tick_size: 5.0,
        }
    }

    /// Set the axis title.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the tick-count hint.
    #[must_use]
    pub const fn with_tick_count(mut self, count: usize) -> Self {
        self.tick_count = count;
        self
    }
}

/// Preferred legend placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LegendAnchor {
    /// Above the chart in normal document flow.
    Top,
    /// Below the chart in normal document flow.
    Bottom,
    /// Top-left inside the chart overlay.
    TopLeft,
    /// Top-right inside the chart overlay.
    TopRight,
    /// Bottom-left inside the chart overlay.
    BottomLeft,
    /// Bottom-right inside the chart overlay.
    BottomRight,
}

/// A single legend row.
#[derive(Debug, Clone)]
pub struct LegendItem {
    /// Display label.
    pub label: String,
    /// Swatch color as RGBA components in `0..=1`.
    pub color: [f32; 4],
}

impl LegendItem {
    /// Build a legend item.
    #[must_use]
    pub fn new(label: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            label: label.into(),
            color,
        }
    }
}

/// Declarative legend guide.
#[derive(Debug, Clone)]
pub struct LegendGuide {
    /// Optional legend title.
    pub title: Option<String>,
    /// Items in display order.
    pub items: Vec<LegendItem>,
    /// Preferred overlay placement.
    pub anchor: LegendAnchor,
}

/// A single field shown in a tooltip.
#[derive(Debug, Clone)]
pub struct TooltipField {
    /// Display label.
    pub label: String,
    /// Dataset column name.
    pub column: String,
    /// Display formatting applied by overlay-capable bindings.
    pub format: TooltipValueFormat,
}

/// Value formatting for tooltip fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TooltipValueFormat {
    /// Infer a compact display from the column dtype.
    Auto,
    /// Numeric value with no fractional digits.
    Integer,
    /// Numeric value with fixed decimal places.
    Number {
        /// Fractional digits to show.
        decimals: u8,
    },
    /// Numeric value with fixed decimal places and a trailing percent sign.
    ///
    /// The value is assumed to already be in percentage-point units.
    Percent {
        /// Fractional digits to show.
        decimals: u8,
    },
    /// Numeric value with fixed decimal places and explicit `+` for positives.
    SignedNumber {
        /// Fractional digits to show.
        decimals: u8,
    },
    /// Signed percentage-point value with a trailing percent sign.
    SignedPercent {
        /// Fractional digits to show.
        decimals: u8,
    },
    /// Text value displayed as a human label.
    Label,
}

impl TooltipField {
    /// Build a tooltip field.
    #[must_use]
    pub fn new(label: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            column: column.into(),
            format: TooltipValueFormat::Auto,
        }
    }

    /// Set value formatting.
    #[must_use]
    pub const fn with_format(mut self, format: TooltipValueFormat) -> Self {
        self.format = format;
        self
    }

    /// Display as an integer.
    #[must_use]
    pub const fn as_integer(self) -> Self {
        self.with_format(TooltipValueFormat::Integer)
    }

    /// Display as a fixed-precision number.
    #[must_use]
    pub const fn as_number(self, decimals: u8) -> Self {
        self.with_format(TooltipValueFormat::Number { decimals })
    }

    /// Display as a fixed-precision percentage-point value.
    #[must_use]
    pub const fn as_percent(self, decimals: u8) -> Self {
        self.with_format(TooltipValueFormat::Percent { decimals })
    }

    /// Display as a signed fixed-precision number.
    #[must_use]
    pub const fn as_signed_number(self, decimals: u8) -> Self {
        self.with_format(TooltipValueFormat::SignedNumber { decimals })
    }

    /// Display as a signed fixed-precision percentage-point value.
    #[must_use]
    pub const fn as_signed_percent(self, decimals: u8) -> Self {
        self.with_format(TooltipValueFormat::SignedPercent { decimals })
    }

    /// Display text as a human label.
    #[must_use]
    pub const fn as_label(self) -> Self {
        self.with_format(TooltipValueFormat::Label)
    }
}

/// Tooltip guide bound to a mark and dataset.
#[derive(Debug, Clone)]
pub struct TooltipGuide {
    /// Mark this tooltip applies to.
    pub mark: MarkId,
    /// Dataset providing row values.
    pub dataset: DatasetId,
    /// Optional column used as the tooltip title.
    pub title_column: Option<String>,
    /// Fields shown in order.
    pub fields: Vec<TooltipField>,
}

/// Label placement priority. Lower-priority labels may be hidden first when
/// the overlay cannot place everything without collisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LabelPriority {
    /// Must be shown whenever possible.
    Required,
    /// Important analytical context.
    Important,
    /// Useful detail that can move to hover/focus at tighter densities.
    Optional,
}

/// Preferred label anchor relative to `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LabelAnchor {
    /// Center on the anchor point.
    Center,
    /// Place above the anchor point.
    Top,
    /// Place below the anchor point.
    Bottom,
    /// Place left of the anchor point.
    Left,
    /// Place right of the anchor point.
    Right,
    /// Place above-left of the anchor point.
    TopLeft,
    /// Place above-right of the anchor point.
    TopRight,
    /// Place below-left of the anchor point.
    BottomLeft,
    /// Place below-right of the anchor point.
    BottomRight,
}

/// Visual treatment for an overlay label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LabelKind {
    /// Direct data label anchored to a mark or derived data position.
    Data,
    /// Node or mark label.
    Node,
    /// Link or line label.
    Flow,
    /// Section/column heading.
    Column,
    /// Free-form analytical annotation.
    Annotation,
}

/// Tooltip content attached directly to an overlay label.
#[derive(Debug, Clone)]
pub struct LabelTooltip {
    /// Tooltip title.
    pub title: String,
    /// Rows shown below the title.
    pub rows: Vec<LabelTooltipRow>,
}

/// A single label-tooltip row.
#[derive(Debug, Clone)]
pub struct LabelTooltipRow {
    /// Display label.
    pub label: String,
    /// Display value.
    pub value: String,
}

impl LabelTooltip {
    /// Build label tooltip content.
    #[must_use]
    pub fn new(title: impl Into<String>, rows: Vec<LabelTooltipRow>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }

    /// Add one row.
    #[must_use]
    pub fn with_row(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push(LabelTooltipRow::new(label, value));
        self
    }
}

impl LabelTooltipRow {
    /// Build one tooltip row.
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// A single overlay label anchored in chart-local pixels.
#[derive(Debug, Clone)]
pub struct LabelItem {
    /// Anchor x in screen-local CSS pixels.
    pub x: f32,
    /// Anchor y in screen-local CSS pixels.
    pub y: f32,
    /// Primary text.
    pub text: String,
    /// Secondary text.
    pub detail: Option<String>,
    /// Preferred placement around `(x, y)`.
    pub anchor: LabelAnchor,
    /// Placement priority.
    pub priority: LabelPriority,
    /// Visual label kind.
    pub kind: LabelKind,
    /// Whether the renderer may try fallback positions to avoid collisions.
    pub allow_reposition: bool,
    /// Optional tooltip content attached to the label itself.
    pub tooltip: Option<LabelTooltip>,
}

impl LabelItem {
    /// Build a label at `(x, y)`.
    #[must_use]
    pub fn new(x: f32, y: f32, text: impl Into<String>) -> Self {
        Self {
            x,
            y,
            text: text.into(),
            detail: None,
            anchor: LabelAnchor::Center,
            priority: LabelPriority::Important,
            kind: LabelKind::Node,
            allow_reposition: true,
            tooltip: None,
        }
    }

    /// Set the secondary text.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the preferred anchor.
    #[must_use]
    pub const fn with_anchor(mut self, anchor: LabelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: LabelPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the visual kind.
    #[must_use]
    pub const fn with_kind(mut self, kind: LabelKind) -> Self {
        self.kind = kind;
        self
    }

    /// Control whether fallback positions may be tried.
    #[must_use]
    pub const fn with_reposition(mut self, allow: bool) -> Self {
        self.allow_reposition = allow;
        self
    }

    /// Attach tooltip content to this label.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: LabelTooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

/// Declarative label guide rendered by overlay-capable bindings.
#[derive(Debug, Clone)]
pub struct LabelGuide {
    /// Labels in author order.
    pub items: Vec<LabelItem>,
    /// Minimum gap between label boxes.
    pub collision_padding: f32,
    /// Maximum labels to display after priority sorting. `None` means all
    /// labels may be attempted.
    pub max_visible: Option<usize>,
}

impl LabelGuide {
    /// Build a label guide.
    #[must_use]
    pub fn new(items: Vec<LabelItem>) -> Self {
        Self {
            items,
            collision_padding: 3.0,
            max_visible: None,
        }
    }

    /// Set collision padding in screen pixels.
    #[must_use]
    pub const fn with_collision_padding(mut self, padding: f32) -> Self {
        self.collision_padding = padding;
        self
    }

    /// Set a hard visible-label budget.
    #[must_use]
    pub const fn with_max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = Some(max_visible);
        self
    }
}

impl TooltipGuide {
    /// Build a tooltip guide for `mark` and `dataset`.
    #[must_use]
    pub fn new(mark: MarkId, dataset: DatasetId, fields: Vec<TooltipField>) -> Self {
        Self {
            mark,
            dataset,
            title_column: None,
            fields,
        }
    }

    /// Set the title column.
    #[must_use]
    pub fn with_title_column(mut self, column: impl Into<String>) -> Self {
        self.title_column = Some(column.into());
        self
    }
}

impl LegendGuide {
    /// Build a legend from items.
    #[must_use]
    pub fn new(items: Vec<LegendItem>) -> Self {
        Self {
            title: None,
            items,
            anchor: LegendAnchor::TopRight,
        }
    }

    /// Set the legend title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set legend placement.
    #[must_use]
    pub const fn with_anchor(mut self, anchor: LegendAnchor) -> Self {
        self.anchor = anchor;
        self
    }
}

/// Any overlay guide known to core.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Guide {
    /// Axis guide.
    Axis(AxisGuide),
    /// Legend guide.
    Legend(LegendGuide),
    /// Tooltip guide.
    Tooltip(TooltipGuide),
    /// Collision-aware overlay labels.
    Labels(LabelGuide),
}
