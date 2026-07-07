//! Candlestick / OHLC chart spec.
//!
//! Renders open-high-low-close bars — filled-body candlesticks by default,
//! Western OHLC bars optionally — with configurable up/down colours, a price
//! axis, and opt-in overlays (SMA, EMA, Bollinger bands, VWAP) computed by the
//! sibling [`crate::indicators`] module.
//!
//! # OHLC validation
//!
//! Candles are **rejected, never silently clamped**. A build fails when a
//! candle breaks an OHLC invariant (`high >= max(open, close)`,
//! `low <= min(open, close)`, `high >= low`), carries a non-finite price, has an
//! invalid volume, or arrives out of time order. Financial data is only useful
//! if it is exact: clamping a bad tick into a plausible-looking candle would
//! fabricate a price that never traded, so
//! [`CandlestickSpec::try_build_chart`] returns a [`CandlestickError`]
//! pinpointing the offending candle index instead of quietly rewriting it.
//!
//! # Example
//!
//! ```
//! use berthacharts_finance::candlestick::{Candle, CandlestickSpec, Overlay};
//! use berthacharts_finance::core::{ChartSize, Workspace};
//!
//! let candles = vec![
//!     Candle::new(0, 100.0, 105.0, 99.0, 102.0),
//!     Candle::new(1, 102.0, 107.0, 101.0, 106.0),
//!     Candle::new(2, 106.0, 108.0, 104.0, 105.0).with_volume(1_200.0),
//! ];
//! let chart = CandlestickSpec::new(candles)
//!     .with_overlay(Overlay::Sma { window: 2, color: [0.95, 0.72, 0.24, 1.0] })
//!     .try_build_chart(Workspace::new(), ChartSize::new(640, 360))?;
//! assert_eq!(chart.scene().layers.len(), 1);
//! # Ok::<(), berthacharts_finance::candlestick::CandlestickError>(())
//! ```

use std::fmt;
use std::sync::Arc;

use berthacharts_core::{
    AxisGuide, AxisOrient, BlendMode, CartesianCoord, Chart, ChartSize, ChartSpec, Column,
    ColumnData, CoordId, Dataset, DatasetId, Geometry, Guide, Interaction, LabelAnchor, LabelGuide,
    LabelItem, LabelKind, LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem,
    LinePrim, LinearScale, Mark, MarkId, PickCtx, PickHit, Rect, RectPrim, Scale, ScaleId, Scene,
    SnapKind, SnapTarget, SnapTargetSet, TessellateCtx, TooltipField, TooltipGuide, Workspace,
};

use crate::indicators::{bollinger_bands, exponential_moving_average, moving_average, vwap};

const CANDLE_DATASET: DatasetId = DatasetId::new(0);
const OVERLAY_DATASET: DatasetId = DatasetId::new(1);
const CANDLE_MARK: MarkId = MarkId::new(1);
const OVERLAY_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);
/// Fixed price-axis label. Candlestick y is always a price, so this is a
/// domain constant rather than a per-instance option.
const PRICE_AXIS_LABEL: &str = "Price";

/// A single OHLC bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    /// Time index (epoch seconds, or any monotonic numeric — chart uses linear scale).
    pub time: i64,
    /// Opening price.
    pub open: f32,
    /// High price.
    pub high: f32,
    /// Low price.
    pub low: f32,
    /// Closing price.
    pub close: f32,
    /// Optional traded volume for this period. Enables volume tooltips and the
    /// [`Overlay::Vwap`] overlay. `None` means "no volume data for this bar".
    pub volume: Option<f32>,
}

impl Candle {
    /// Build a candle from its four prices. Volume is absent; add it with
    /// [`Candle::with_volume`].
    #[must_use]
    pub const fn new(time: i64, open: f32, high: f32, low: f32, close: f32) -> Self {
        Self {
            time,
            open,
            high,
            low,
            close,
            volume: None,
        }
    }

    /// Attach traded volume to this candle.
    #[must_use]
    pub const fn with_volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }

    /// True when close >= open (typically rendered green). A doji
    /// (`close == open`) counts as up.
    #[must_use]
    pub fn is_up(&self) -> bool {
        self.close >= self.open
    }
}

/// How OHLC data is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleStyle {
    /// Filled-body candlesticks with up/down colours.
    Candlestick,
    /// Western-style OHLC bars: vertical high-low line with tick at open (left) and close (right).
    OhlcBars,
}

/// Optional overlay line computed from close prices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overlay {
    /// Simple moving average of the given window.
    Sma { window: usize, color: [f32; 4] },
    /// Exponential moving average.
    Ema { window: usize, color: [f32; 4] },
    /// Bollinger bands — emits three lines (upper/mid/lower).
    Bollinger {
        /// SMA window.
        window: usize,
        /// Stddev multiplier (typical 2.0).
        k: f32,
        /// Stroke colour for all three bands.
        color: [f32; 4],
    },
    /// Volume-weighted average price over the close series. Requires per-candle
    /// volume (see [`Candle::with_volume`]); the line stays empty when no
    /// candle carries volume.
    Vwap { color: [f32; 4] },
}

/// Candlestick chart configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandlestickOptions {
    /// Rendering style.
    pub style: CandleStyle,
    /// Body width as a fraction of bar spacing (0..1). Typical 0.7.
    pub body_width_ratio: f32,
    /// Up-bar fill (close >= open).
    pub up_color: [f32; 4],
    /// Down-bar fill.
    pub down_color: [f32; 4],
    /// Wick line width in pixels.
    pub wick_width: f32,
    /// Padding inside the plot area (pixels).
    pub padding: f32,
    /// Desired price-axis tick count. Floored at 2.
    pub price_tick_count: usize,
}

impl Default for CandlestickOptions {
    fn default() -> Self {
        Self {
            style: CandleStyle::Candlestick,
            body_width_ratio: 0.7,
            up_color: [0.22, 0.78, 0.50, 1.0],
            down_color: [0.92, 0.36, 0.36, 1.0],
            wick_width: 1.0,
            padding: 24.0,
            price_tick_count: 5,
        }
    }
}

/// Candlestick chart spec.
#[derive(Debug, Clone)]
pub struct CandlestickSpec {
    candles: Vec<Candle>,
    overlays: Vec<Overlay>,
    options: CandlestickOptions,
}

impl CandlestickSpec {
    /// Build a candlestick spec.
    #[must_use]
    pub fn new(candles: Vec<Candle>) -> Self {
        Self {
            candles,
            overlays: Vec::new(),
            options: CandlestickOptions::default(),
        }
    }

    /// Override options.
    #[must_use]
    pub const fn with_options(mut self, options: CandlestickOptions) -> Self {
        self.options = options;
        self
    }

    /// Add an overlay line (SMA / EMA / Bollinger bands / VWAP).
    #[must_use]
    pub fn with_overlay(mut self, overlay: Overlay) -> Self {
        self.overlays.push(overlay);
        self
    }

    /// Compute the reusable screen-space layout without building a chart.
    ///
    /// Validates the candles first, so this returns the same
    /// [`CandlestickError`] a chart build would.
    pub fn layout(&self, size: ChartSize) -> Result<CandlestickLayout, CandlestickError> {
        self.validate()?;
        Ok(compute_layout(
            &self.candles,
            &self.overlays,
            &self.options,
            self.plot_area(size),
        ))
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, CandlestickError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    /// Reject empty input, non-finite prices, invalid volumes, broken OHLC
    /// invariants, and out-of-order timestamps. See the module docs for why
    /// bad candles are rejected rather than clamped.
    fn validate(&self) -> Result<(), CandlestickError> {
        if self.candles.is_empty() {
            return Err(CandlestickError::Empty);
        }
        let mut previous_time: Option<i64> = None;
        for (index, candle) in self.candles.iter().enumerate() {
            if !candle.open.is_finite()
                || !candle.high.is_finite()
                || !candle.low.is_finite()
                || !candle.close.is_finite()
            {
                return Err(CandlestickError::NonFinite { index });
            }
            // high must top open/close/low; low must sit under open/close.
            if candle.high < candle.low
                || candle.high < candle.open
                || candle.high < candle.close
                || candle.low > candle.open
                || candle.low > candle.close
            {
                return Err(CandlestickError::InvariantViolation { index });
            }
            if let Some(volume) = candle.volume {
                if !volume.is_finite() || volume < 0.0 {
                    return Err(CandlestickError::InvalidVolume { index });
                }
            }
            if let Some(previous) = previous_time {
                if candle.time <= previous {
                    return Err(CandlestickError::DisorderedTime { index });
                }
            }
            previous_time = Some(candle.time);
        }
        Ok(())
    }

    /// Padded inner plot rect for `size`. Uses `min` (never `clamp`) so a
    /// collapsing container (0x0 during page teardown) degrades to a 1px rect
    /// instead of inverting clamp bounds — an inverted `f32::clamp` panics, and
    /// a panic here would unwind through the wasm `&mut self` frame and leave a
    /// handle permanently borrowed (see the sibling `charts::bar` note).
    fn plot_area(&self, size: ChartSize) -> Rect {
        let width = size.width as f32;
        let height = size.height as f32;
        let pad = self.options.padding.max(0.0);
        let left = pad.min((width - 1.0).max(0.0));
        let top = pad.min((height - 1.0).max(0.0));
        Rect::new(
            left,
            top,
            (width - left - pad).max(1.0),
            (height - top - pad).max(1.0),
        )
    }
}

/// Computed candle positions in screen coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct CandlestickLayout {
    /// One per input candle, same order.
    pub bars: Vec<CandleBar>,
    /// Overlay polylines (one per Overlay; Bollinger emits three).
    pub overlay_lines: Vec<OverlayLine>,
    /// Plot area used (post-padding).
    pub plot_area: Rect,
    /// Bar width in pixels.
    pub bar_width: f32,
    /// Padded price domain `(min, max)` mapped onto the plot area. Drives the
    /// price axis.
    pub price_range: (f32, f32),
}

/// Positioned candle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleBar {
    /// Centre x in screen pixels.
    pub center_x: f32,
    /// y at open.
    pub y_open: f32,
    /// y at high.
    pub y_high: f32,
    /// y at low.
    pub y_low: f32,
    /// y at close.
    pub y_close: f32,
    /// True when close >= open.
    pub up: bool,
}

/// An overlay polyline.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayLine {
    /// Screen-space points; NaN positions filter out.
    pub points: Vec<[f32; 2]>,
    /// Premultiplied RGBA stroke.
    pub color: [f32; 4],
    /// Stroke width.
    pub width: f32,
}

/// Errors during candlestick build.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CandlestickError {
    /// No candles in the spec.
    Empty,
    /// A candle carries a non-finite (NaN/inf) price.
    NonFinite {
        /// Offending candle index.
        index: usize,
    },
    /// A candle breaks an OHLC invariant: `high >= max(open, close)`,
    /// `low <= min(open, close)`, and `high >= low`.
    InvariantViolation {
        /// Offending candle index.
        index: usize,
    },
    /// A candle carries a non-finite or negative volume.
    InvalidVolume {
        /// Offending candle index.
        index: usize,
    },
    /// Timestamps are not strictly increasing (unsorted or duplicated).
    DisorderedTime {
        /// Index of the candle whose time is not after its predecessor.
        index: usize,
    },
}

impl fmt::Display for CandlestickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "candlestick spec has no candles"),
            Self::NonFinite { index } => {
                write!(f, "candle at index {index} has a non-finite price")
            }
            Self::InvariantViolation { index } => write!(
                f,
                "candle at index {index} violates OHLC invariants \
                 (need high >= max(open, close), low <= min(open, close), high >= low)"
            ),
            Self::InvalidVolume { index } => {
                write!(
                    f,
                    "candle at index {index} has a non-finite or negative volume"
                )
            }
            Self::DisorderedTime { index } => write!(
                f,
                "candle at index {index} is out of order: timestamps must be strictly increasing"
            ),
        }
    }
}

impl std::error::Error for CandlestickError {}

impl ChartSpec for CandlestickSpec {
    type Error = CandlestickError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        let layout = self.layout(size)?;
        let inner = layout.plot_area;
        let viewport = size.viewport_with_plot_area(inner);
        let has_volume = self.candles.iter().any(|c| c.volume.is_some());

        // X stays an identity pixel scale (bars are index-positioned, so a
        // time axis would misrepresent irregular gaps). Y is a real price
        // scale so the left axis reads true price levels; the coord itself is
        // an identity pass-through, so this drives the axis without moving any
        // pre-tessellated mark.
        let x_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (0.0, viewport.width as f64),
            (0.0, viewport.width as f32),
        ));
        let (price_min, price_max) = layout.price_range;
        let y_scale: Arc<dyn Scale> = Arc::new(LinearScale::new(
            (f64::from(price_min), f64::from(price_max)),
            (inner.y + inner.h, inner.y),
        ));
        workspace.upsert_scale(X_SCALE, x_scale);
        workspace.upsert_scale(Y_SCALE, y_scale);
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(candle_dataset(&self.candles, has_volume));
        workspace.upsert_dataset(overlay_dataset(&layout));

        let overlay_mark: Arc<dyn Mark> =
            Arc::new(OverlayMark::new(OVERLAY_MARK, layout.overlay_lines.clone()));
        let candle_mark: Arc<dyn Mark> = Arc::new(CandleMark::new(
            CANDLE_MARK,
            layout.bars.clone(),
            self.options,
            layout.bar_width,
        ));

        let mut scene = Scene::new(viewport);
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![overlay_mark, candle_mark],
            blend: BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        scene.guides.push(Guide::Axis(
            AxisGuide::new(Y_SCALE, AxisOrient::Left)
                .with_label(PRICE_AXIS_LABEL)
                .with_tick_count(self.options.price_tick_count.max(2)),
        ));
        scene
            .guides
            .push(Guide::Tooltip(candle_tooltip(has_volume)));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(legend_items(&self.overlays, self.options))
                .with_title("OHLC")
                .with_anchor(LegendAnchor::Bottom),
        ));
        if let Some(label) = latest_close_label(&layout, &self.candles) {
            scene.guides.push(Guide::Labels(
                LabelGuide::new(vec![label]).with_collision_padding(4.0),
            ));
        }
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(snap_targets(&layout, &self.candles)).with_name("candle closes"),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

fn compute_layout(
    candles: &[Candle],
    overlays: &[Overlay],
    options: &CandlestickOptions,
    inner: Rect,
) -> CandlestickLayout {
    let n = candles.len();

    // Price range from all candle highs/lows + any overlay extents.
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for c in candles {
        y_min = y_min.min(c.low);
        y_max = y_max.max(c.high);
    }

    let closes: Vec<f32> = candles.iter().map(|c| c.close).collect();

    // Pre-compute overlay series.
    let mut overlay_series: Vec<(Vec<f32>, [f32; 4], f32)> = Vec::new();
    for overlay in overlays {
        match *overlay {
            Overlay::Sma { window, color } => {
                let s = moving_average(&closes, window);
                expand_y_range(&s, &mut y_min, &mut y_max);
                overlay_series.push((s, color, 1.5));
            }
            Overlay::Ema { window, color } => {
                let s = exponential_moving_average(&closes, window);
                expand_y_range(&s, &mut y_min, &mut y_max);
                overlay_series.push((s, color, 1.5));
            }
            Overlay::Bollinger { window, k, color } => {
                let bb = bollinger_bands(&closes, window, k);
                expand_y_range(&bb.upper, &mut y_min, &mut y_max);
                expand_y_range(&bb.lower, &mut y_min, &mut y_max);
                overlay_series.push((bb.upper, color, 1.0));
                overlay_series.push((bb.mid, color, 1.2));
                overlay_series.push((bb.lower, color, 1.0));
            }
            Overlay::Vwap { color } => {
                let volumes: Vec<f32> = candles
                    .iter()
                    .map(|c| c.volume.unwrap_or(f32::NAN))
                    .collect();
                let s = vwap(&closes, &volumes);
                expand_y_range(&s, &mut y_min, &mut y_max);
                overlay_series.push((s, color, 1.5));
            }
        }
    }

    if y_max <= y_min {
        y_max = y_min + 1.0;
    }
    // Add ~3% top/bottom padding inside inner rect.
    let span = y_max - y_min;
    y_min -= span * 0.03;
    y_max += span * 0.03;

    let map_y = |price: f32| inner.y + inner.h - (price - y_min) / (y_max - y_min) * inner.h;

    let bar_spacing = inner.w / n as f32;
    let bar_width = (bar_spacing * options.body_width_ratio).max(1.0);

    let bars: Vec<CandleBar> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| CandleBar {
            center_x: inner.x + (i as f32 + 0.5) * bar_spacing,
            y_open: map_y(c.open),
            y_high: map_y(c.high),
            y_low: map_y(c.low),
            y_close: map_y(c.close),
            up: c.is_up(),
        })
        .collect();

    let overlay_lines: Vec<OverlayLine> = overlay_series
        .into_iter()
        .map(|(series, color, width)| {
            let points: Vec<[f32; 2]> = series
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_finite())
                .map(|(i, v)| {
                    let center_x = inner.x + (i as f32 + 0.5) * bar_spacing;
                    [center_x, map_y(*v)]
                })
                .collect();
            OverlayLine {
                points,
                color,
                width,
            }
        })
        .collect();

    CandlestickLayout {
        bars,
        overlay_lines,
        plot_area: inner,
        bar_width,
        price_range: (y_min, y_max),
    }
}

fn expand_y_range(series: &[f32], y_min: &mut f32, y_max: &mut f32) {
    for &v in series {
        if v.is_finite() {
            *y_min = y_min.min(v);
            *y_max = y_max.max(v);
        }
    }
}

fn candle_dataset(candles: &[Candle], has_volume: bool) -> Dataset {
    let len = candles.len();
    let mut t = Vec::with_capacity(len);
    let mut o = Vec::with_capacity(len);
    let mut h = Vec::with_capacity(len);
    let mut l = Vec::with_capacity(len);
    let mut c = Vec::with_capacity(len);
    let mut direction: Vec<Arc<str>> = Vec::with_capacity(len);
    for cd in candles {
        t.push(cd.time);
        o.push(cd.open);
        h.push(cd.high);
        l.push(cd.low);
        c.push(cd.close);
        direction.push(Arc::from(if cd.is_up() { "up" } else { "down" }));
    }
    let mut columns = vec![
        ("time".to_string(), Column::I64(ColumnData::new(t))),
        ("open".to_string(), Column::F32(ColumnData::new(o))),
        ("high".to_string(), Column::F32(ColumnData::new(h))),
        ("low".to_string(), Column::F32(ColumnData::new(l))),
        ("close".to_string(), Column::F32(ColumnData::new(c))),
        (
            "direction".to_string(),
            Column::Utf8(ColumnData::new(direction)),
        ),
    ];
    // Only surface a volume column when some candle actually has volume, so the
    // volume tooltip field never resolves against a dead all-NaN column.
    if has_volume {
        let volume: Vec<f32> = candles
            .iter()
            .map(|cd| cd.volume.unwrap_or(f32::NAN))
            .collect();
        columns.push(("volume".to_string(), Column::F32(ColumnData::new(volume))));
    }
    Dataset::new(CANDLE_DATASET, 1, columns)
}

fn candle_tooltip(has_volume: bool) -> TooltipGuide {
    let mut fields = vec![
        TooltipField::new("Open", "open").as_number(2),
        TooltipField::new("High", "high").as_number(2),
        TooltipField::new("Low", "low").as_number(2),
        TooltipField::new("Close", "close").as_number(2),
        TooltipField::new("Direction", "direction").as_label(),
    ];
    if has_volume {
        fields.push(TooltipField::new("Volume", "volume").as_integer());
    }
    TooltipGuide::new(CANDLE_MARK, CANDLE_DATASET, fields).with_title_column("time")
}

fn legend_items(overlays: &[Overlay], options: CandlestickOptions) -> Vec<LegendItem> {
    let mut items = vec![
        LegendItem::new("Up", options.up_color),
        LegendItem::new("Down", options.down_color),
    ];
    for overlay in overlays {
        match *overlay {
            Overlay::Sma { window, color } => {
                items.push(LegendItem::new(format!("SMA {window}"), color));
            }
            Overlay::Ema { window, color } => {
                items.push(LegendItem::new(format!("EMA {window}"), color));
            }
            Overlay::Bollinger { window, color, .. } => {
                items.push(LegendItem::new(format!("Bollinger {window}"), color));
            }
            Overlay::Vwap { color } => {
                items.push(LegendItem::new("VWAP", color));
            }
        }
    }
    items
}

fn latest_close_label(layout: &CandlestickLayout, candles: &[Candle]) -> Option<LabelItem> {
    let bar = layout.bars.last()?;
    let candle = candles.last()?;
    Some(
        LabelItem::new(bar.center_x, bar.y_close, "Close")
            .with_detail(format!("{:.2}", candle.close))
            .with_anchor(LabelAnchor::Right)
            .with_kind(LabelKind::Data)
            .with_priority(LabelPriority::Required),
    )
}

fn snap_targets(layout: &CandlestickLayout, candles: &[Candle]) -> Vec<SnapTarget> {
    layout
        .bars
        .iter()
        .zip(candles)
        .map(|(bar, candle)| {
            SnapTarget::new(bar.center_x, bar.y_close, SnapKind::Point)
                .with_radius(7.0)
                .with_label(format!("Close {:.1}", candle.close))
                .with_priority(if bar.up { 3 } else { 2 })
        })
        .collect()
}

fn overlay_dataset(layout: &CandlestickLayout) -> Dataset {
    let mut idx = Vec::new();
    let mut x = Vec::new();
    let mut y = Vec::new();
    for (i, line) in layout.overlay_lines.iter().enumerate() {
        for pt in &line.points {
            idx.push(i as i64);
            x.push(pt[0]);
            y.push(pt[1]);
        }
    }
    Dataset::new(
        OVERLAY_DATASET,
        1,
        vec![
            ("overlay".to_string(), Column::I64(ColumnData::new(idx))),
            ("x".to_string(), Column::F32(ColumnData::new(x))),
            ("y".to_string(), Column::F32(ColumnData::new(y))),
        ],
    )
}

#[derive(Debug, Clone)]
struct CandleMark {
    id: MarkId,
    bars: Vec<CandleBar>,
    options: CandlestickOptions,
    bar_width: f32,
}

impl CandleMark {
    fn new(id: MarkId, bars: Vec<CandleBar>, options: CandlestickOptions, bar_width: f32) -> Self {
        Self {
            id,
            bars,
            options,
            bar_width,
        }
    }
}

impl Mark for CandleMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.bars.len() as u64;
        h ^= match self.options.style {
            CandleStyle::Candlestick => 1,
            CandleStyle::OhlcBars => 2,
        };
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        match self.options.style {
            CandleStyle::Candlestick => self.tessellate_candlesticks(),
            CandleStyle::OhlcBars => self.tessellate_ohlc(),
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let (px, py) = point;
        let half = self.bar_width * 0.5;
        for (row, bar) in self.bars.iter().enumerate() {
            if (px - bar.center_x).abs() <= half && py >= bar.y_high && py <= bar.y_low {
                return Some(PickHit {
                    mark: self.id,
                    row: Some(row),
                    distance: 0.0,
                    payload: None,
                });
            }
        }
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.bars.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let half = self.bar_width * 0.5;
        for b in &self.bars {
            min_x = min_x.min(b.center_x - half);
            max_x = max_x.max(b.center_x + half);
            min_y = min_y.min(b.y_high);
            max_y = max_y.max(b.y_low);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl CandleMark {
    fn tessellate_candlesticks(&self) -> Geometry {
        let mut rects = Vec::with_capacity(self.bars.len());
        let mut wicks = Vec::with_capacity(self.bars.len());
        let half = self.bar_width * 0.5;
        for b in &self.bars {
            let color = if b.up {
                self.options.up_color
            } else {
                self.options.down_color
            };
            let (body_top, body_bottom) = if b.up {
                (b.y_close, b.y_open)
            } else {
                (b.y_open, b.y_close)
            };
            let h = (body_bottom - body_top).max(1.0);
            rects.push(RectPrim {
                x: b.center_x - half,
                y: body_top,
                w: self.bar_width,
                h,
                fill: color,
                stroke: color,
                stroke_width: 0.0,
                radius: 0.0,
            });
            wicks.push(LinePrim {
                points: vec![[b.center_x, b.y_high], [b.center_x, b.y_low]],
                stroke: color,
                width: self.options.wick_width,
                dash: None,
                join: 1,
                cap: 1,
            });
        }
        Geometry::Mixed(vec![Geometry::Rects(rects), Geometry::Lines(wicks)])
    }

    fn tessellate_ohlc(&self) -> Geometry {
        let mut wicks = Vec::with_capacity(self.bars.len() * 3);
        let half = self.bar_width * 0.5;
        for b in &self.bars {
            let color = if b.up {
                self.options.up_color
            } else {
                self.options.down_color
            };
            // vertical high-low
            wicks.push(LinePrim {
                points: vec![[b.center_x, b.y_high], [b.center_x, b.y_low]],
                stroke: color,
                width: self.options.wick_width,
                dash: None,
                join: 1,
                cap: 1,
            });
            // open tick (left)
            wicks.push(LinePrim {
                points: vec![[b.center_x - half, b.y_open], [b.center_x, b.y_open]],
                stroke: color,
                width: self.options.wick_width,
                dash: None,
                join: 1,
                cap: 1,
            });
            // close tick (right)
            wicks.push(LinePrim {
                points: vec![[b.center_x, b.y_close], [b.center_x + half, b.y_close]],
                stroke: color,
                width: self.options.wick_width,
                dash: None,
                join: 1,
                cap: 1,
            });
        }
        Geometry::Lines(wicks)
    }
}

#[derive(Debug, Clone)]
struct OverlayMark {
    id: MarkId,
    lines: Vec<OverlayLine>,
}

impl OverlayMark {
    fn new(id: MarkId, lines: Vec<OverlayLine>) -> Self {
        Self { id, lines }
    }
}

impl Mark for OverlayMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.lines.len() as u64;
        for l in &self.lines {
            h ^= l.points.len() as u64;
        }
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        let lines: Vec<LinePrim> = self
            .lines
            .iter()
            .filter(|l| l.points.len() >= 2)
            .map(|l| LinePrim {
                points: l.points.clone(),
                stroke: l.color,
                width: l.width,
                dash: None,
                join: 1,
                cap: 1,
            })
            .collect();
        if lines.is_empty() {
            Geometry::Empty
        } else {
            Geometry::Lines(lines)
        }
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        if self.lines.is_empty() {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for l in &self.lines {
            for p in &l.points {
                min_x = min_x.min(p[0]);
                min_y = min_y.min(p[1]);
                max_x = max_x.max(p[0]);
                max_y = max_y.max(p[1]);
            }
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Guide, SnapKind};

    fn sample_candles(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| {
                let base = 100.0 + i as f32;
                Candle::new(i as i64, base, base + 2.0, base - 1.0, base + 1.0)
            })
            .collect()
    }

    fn build(candles: Vec<Candle>, w: u32, h: u32) -> Result<Chart, CandlestickError> {
        CandlestickSpec::new(candles).try_build_chart(Workspace::new(), ChartSize::new(w, h))
    }

    #[test]
    fn empty_spec_rejected() {
        assert_eq!(
            build(vec![], 400, 300).unwrap_err(),
            CandlestickError::Empty
        );
    }

    #[test]
    fn invariant_violation_rejected() {
        // high < low
        let bad = vec![Candle::new(0, 10.0, 5.0, 12.0, 8.0)];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::InvariantViolation { index: 0 }
        );
    }

    #[test]
    fn invariant_close_above_high_rejected() {
        // close above high
        let bad = vec![Candle::new(0, 10.0, 12.0, 9.0, 13.0)];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::InvariantViolation { index: 0 }
        );
    }

    #[test]
    fn non_finite_price_rejected() {
        let bad = vec![
            Candle::new(0, 10.0, 12.0, 9.0, 11.0),
            Candle::new(1, 11.0, f32::NAN, 10.0, 10.5),
        ];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::NonFinite { index: 1 }
        );
    }

    #[test]
    fn unsorted_time_rejected() {
        let bad = vec![
            Candle::new(0, 10.0, 12.0, 9.0, 11.0),
            Candle::new(2, 11.0, 13.0, 10.0, 12.0),
            Candle::new(1, 12.0, 14.0, 11.0, 13.0),
        ];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::DisorderedTime { index: 2 }
        );
    }

    #[test]
    fn duplicate_time_rejected() {
        let bad = vec![
            Candle::new(0, 10.0, 12.0, 9.0, 11.0),
            Candle::new(1, 11.0, 13.0, 10.0, 12.0),
            Candle::new(1, 12.0, 14.0, 11.0, 13.0),
        ];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::DisorderedTime { index: 2 }
        );
    }

    #[test]
    fn negative_volume_rejected() {
        let bad = vec![Candle::new(0, 10.0, 12.0, 9.0, 11.0).with_volume(-5.0)];
        assert_eq!(
            build(bad, 400, 300).unwrap_err(),
            CandlestickError::InvalidVolume { index: 0 }
        );
    }

    #[test]
    fn single_candle_builds_with_one_snap_target() {
        let chart = build(sample_candles(1), 400, 300).expect("chart");
        assert!(!chart.scene().layers.is_empty());
        assert_eq!(chart.snap_targets().len(), 1);
    }

    #[test]
    fn zero_range_candle_builds_without_panic() {
        // open == high == low == close: a flat, zero-range bar.
        let flat = vec![Candle::new(0, 50.0, 50.0, 50.0, 50.0)];
        let chart = build(flat, 400, 300).expect("flat candle should build");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        // A collapsing container must degrade gracefully, never panic.
        for (w, h) in [(0, 0), (1, 1), (2, 400), (400, 2)] {
            let chart = build(sample_candles(6), w, h).expect("chart");
            assert!(chart.scene().viewport.plot_area.w >= 1.0);
            assert!(chart.scene().viewport.plot_area.h >= 1.0);
        }
    }

    #[test]
    fn candlestick_builds() {
        let chart = build(sample_candles(20), 800, 400).expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn ohlc_style_builds() {
        let chart = CandlestickSpec::new(sample_candles(10))
            .with_options(CandlestickOptions {
                style: CandleStyle::OhlcBars,
                ..CandlestickOptions::default()
            })
            .try_build_chart(Workspace::new(), ChartSize::new(800, 400))
            .expect("chart");
        assert!(!chart.scene().layers.is_empty());
    }

    #[test]
    fn layout_exposes_price_range_and_plot_area() {
        let layout = CandlestickSpec::new(sample_candles(12))
            .layout(ChartSize::new(640, 360))
            .expect("layout");
        let (lo, hi) = layout.price_range;
        assert!(lo < hi, "price range must be non-degenerate");
        assert_eq!(layout.bars.len(), 12);
        assert!(layout.plot_area.w >= 1.0 && layout.plot_area.h >= 1.0);
    }

    #[test]
    fn layout_validates_before_computing() {
        let err = CandlestickSpec::new(vec![])
            .layout(ChartSize::new(320, 200))
            .unwrap_err();
        assert_eq!(err, CandlestickError::Empty);
    }

    #[test]
    fn bollinger_overlay_adds_three_lines() {
        let layout = compute_layout(
            &sample_candles(30),
            &[Overlay::Bollinger {
                window: 5,
                k: 2.0,
                color: [0.6, 0.6, 0.9, 0.8],
            }],
            &CandlestickOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 400.0),
        );
        // 1 Bollinger -> 3 overlay lines.
        assert_eq!(layout.overlay_lines.len(), 3);
    }

    #[test]
    fn sma_and_ema_overlays_both_render() {
        let layout = compute_layout(
            &sample_candles(30),
            &[
                Overlay::Sma {
                    window: 5,
                    color: [1.0, 1.0, 0.0, 0.8],
                },
                Overlay::Ema {
                    window: 10,
                    color: [0.0, 1.0, 1.0, 0.8],
                },
            ],
            &CandlestickOptions::default(),
            Rect::new(0.0, 0.0, 800.0, 400.0),
        );
        assert_eq!(layout.overlay_lines.len(), 2);
    }

    #[test]
    fn vwap_overlay_draws_only_with_volume() {
        let overlays = [Overlay::Vwap {
            color: [0.5, 0.5, 0.9, 0.8],
        }];
        let opts = CandlestickOptions::default();
        let plot = Rect::new(0.0, 0.0, 800.0, 400.0);

        // No volume anywhere -> empty VWAP line.
        let without = compute_layout(&sample_candles(10), &overlays, &opts, plot);
        assert_eq!(without.overlay_lines.len(), 1);
        assert!(without.overlay_lines[0].points.is_empty());

        // With volume -> a drawable polyline of finite points.
        let with_volume: Vec<Candle> = sample_candles(10)
            .into_iter()
            .enumerate()
            .map(|(i, c)| c.with_volume(1_000.0 + i as f32 * 10.0))
            .collect();
        let with = compute_layout(&with_volume, &overlays, &opts, plot);
        assert_eq!(with.overlay_lines.len(), 1);
        assert!(with.overlay_lines[0].points.len() >= 2);
        assert!(with.overlay_lines[0]
            .points
            .iter()
            .all(|p| p[0].is_finite() && p[1].is_finite()));
    }

    #[test]
    fn up_candle_flagged() {
        let c = Candle::new(0, 10.0, 12.0, 9.0, 11.5);
        assert!(c.is_up());
        let d = Candle::new(0, 10.0, 12.0, 9.0, 9.5);
        assert!(!d.is_up());
    }

    #[test]
    fn volume_tooltip_present_only_when_volume_exists() {
        let volume_field_present = |chart: &Chart| {
            chart
                .scene()
                .guides
                .iter()
                .find_map(|guide| match guide {
                    Guide::Tooltip(tooltip) if tooltip.mark == CANDLE_MARK => Some(tooltip),
                    _ => None,
                })
                .expect("candle tooltip")
                .fields
                .iter()
                .any(|field| field.column == "volume")
        };

        let without = build(sample_candles(8), 640, 360).expect("chart");
        assert!(!volume_field_present(&without));

        let with_volume: Vec<Candle> = sample_candles(8)
            .into_iter()
            .map(|c| c.with_volume(1_500.0))
            .collect();
        let with = build(with_volume, 640, 360).expect("chart");
        assert!(volume_field_present(&with));
    }

    #[test]
    fn build_chart_exposes_price_axis() {
        let chart = build(sample_candles(12), 640, 360).expect("chart");
        let axis = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Axis(axis) => Some(axis),
                _ => None,
            })
            .expect("price axis guide");
        assert_eq!(axis.scale, Y_SCALE);
    }

    #[test]
    fn build_chart_exposes_ohlc_tooltip_legend_labels_and_snap_targets() {
        let chart = CandlestickSpec::new(sample_candles(12))
            .with_overlay(Overlay::Sma {
                window: 3,
                color: [0.95, 0.72, 0.24, 1.0],
            })
            .try_build_chart(Workspace::new(), ChartSize::new(640, 360))
            .expect("chart");

        let tooltip = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Tooltip(tooltip) if tooltip.mark == CANDLE_MARK => Some(tooltip),
                _ => None,
            })
            .expect("candle tooltip guide");
        assert_eq!(tooltip.title_column.as_deref(), Some("time"));
        assert_eq!(tooltip.fields.len(), 5);
        assert!(tooltip.fields.iter().any(|field| field.column == "close"));

        let legend = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Legend(legend) => Some(legend),
                _ => None,
            })
            .expect("legend guide");
        assert!(legend.items.iter().any(|item| item.label == "Up"));
        assert!(legend.items.iter().any(|item| item.label == "Down"));
        assert!(legend.items.iter().any(|item| item.label == "SMA 3"));

        let labels = chart
            .scene()
            .guides
            .iter()
            .find_map(|guide| match guide {
                Guide::Labels(labels) => Some(labels),
                _ => None,
            })
            .expect("label guide");
        assert!(labels.items.iter().any(|item| item.text == "Close"));

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 12);
        assert!(targets.iter().all(|target| target.kind == SnapKind::Point));
        assert!(targets
            .iter()
            .any(|target| target.label.as_deref() == Some("Close 111.0")));
    }
}
