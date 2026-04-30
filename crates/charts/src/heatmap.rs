//! Heatmap chart spec with direct labels, signal glyphs, tooltips, legend, and
//! semantic cell-center snap targets.

use std::sync::Arc;

use berthacharts_core::{
    CartesianCoord, Chart, ChartSize, ChartSpec, ColorChannel, Column, ColumnData, CoordId,
    Dataset, DatasetId, Geometry, Guide, LabelAnchor, LabelGuide, LabelItem, LabelKind,
    LabelPriority, Layer, LayerId, LegendAnchor, LegendGuide, LegendItem, LinearScale, Mark,
    MarkId, NumberChannel, PointPrim, Rect, RectMark, Scale, ScaleId, Scene, SnapKind, SnapTarget,
    SnapTargetSet, TooltipField, TooltipGuide, Workspace,
};

use crate::mark::GeometryMark;

const DATASET: DatasetId = DatasetId::new(0);
const CELL_MARK: MarkId = MarkId::new(1);
const GLYPH_MARK: MarkId = MarkId::new(2);
const LAYER: LayerId = LayerId::new(0);
const X_SCALE: ScaleId = ScaleId::new(1);
const Y_SCALE: ScaleId = ScaleId::new(2);
const COORD: CoordId = CoordId::new(0);

/// One heatmap cell.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapCell {
    /// Row label.
    pub row: String,
    /// Column label.
    pub column: String,
    /// Cell value, usually a fraction in `0..=1`.
    pub value: f32,
    /// Optional comparison baseline for signal classification.
    pub baseline: Option<f32>,
    /// Optional secondary text for direct labels.
    pub label_detail: Option<String>,
    /// Optional first domain-specific tooltip detail.
    pub tooltip_detail_1: Option<String>,
    /// Optional second domain-specific tooltip detail.
    pub tooltip_detail_2: Option<String>,
    /// Optional third domain-specific tooltip detail.
    pub tooltip_detail_3: Option<String>,
}

impl HeatmapCell {
    /// Build one heatmap cell.
    #[must_use]
    pub fn new(row: impl Into<String>, column: impl Into<String>, value: f32) -> Self {
        Self {
            row: row.into(),
            column: column.into(),
            value,
            baseline: None,
            label_detail: None,
            tooltip_detail_1: None,
            tooltip_detail_2: None,
            tooltip_detail_3: None,
        }
    }

    /// Set an explicit comparison baseline.
    #[must_use]
    pub const fn with_baseline(mut self, baseline: f32) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Set custom secondary text for the direct label.
    #[must_use]
    pub fn with_label_detail(mut self, detail: impl Into<String>) -> Self {
        self.label_detail = Some(detail.into());
        self
    }

    /// Set the first domain-specific tooltip detail.
    #[must_use]
    pub fn with_tooltip_detail_1(mut self, detail: impl Into<String>) -> Self {
        self.tooltip_detail_1 = Some(detail.into());
        self
    }

    /// Set the second domain-specific tooltip detail.
    #[must_use]
    pub fn with_tooltip_detail_2(mut self, detail: impl Into<String>) -> Self {
        self.tooltip_detail_2 = Some(detail.into());
        self
    }

    /// Set the third domain-specific tooltip detail.
    #[must_use]
    pub fn with_tooltip_detail_3(mut self, detail: impl Into<String>) -> Self {
        self.tooltip_detail_3 = Some(detail.into());
        self
    }
}

/// Layout and guide options for a heatmap.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapOptions {
    /// Inset inside each grid cell.
    pub cell_padding: f32,
    /// Absolute delta threshold for positive/negative signal glyphs.
    pub signal_threshold: f32,
    /// Legend title.
    pub legend_title: String,
    /// Tooltip label for row values.
    pub row_tooltip_label: String,
    /// Tooltip label for the primary value.
    pub value_tooltip_label: String,
    /// Tooltip label for the value-baseline delta.
    pub delta_tooltip_label: String,
    /// Tooltip label for the signal classification.
    pub signal_tooltip_label: String,
    /// Optional label for the first domain-specific tooltip detail.
    pub tooltip_detail_1_label: Option<String>,
    /// Optional label for the second domain-specific tooltip detail.
    pub tooltip_detail_2_label: Option<String>,
    /// Optional label for the third domain-specific tooltip detail.
    pub tooltip_detail_3_label: Option<String>,
    /// Label for cells above baseline by at least `signal_threshold`.
    pub strong_signal_label: String,
    /// Label for cells below baseline by at least `signal_threshold`.
    pub watch_signal_label: String,
    /// Label for cells within the signal threshold.
    pub neutral_signal_label: String,
    /// Whether to draw per-cell signal glyphs for above/below-baseline cells.
    pub show_signal_glyphs: bool,
    /// Whether direct cell labels should include their secondary detail line.
    pub show_label_details: bool,
    /// Maximum visible data labels.
    pub max_visible_labels: Option<usize>,
}

impl Default for HeatmapOptions {
    fn default() -> Self {
        Self {
            cell_padding: 8.0,
            signal_threshold: 0.07,
            legend_title: "Signal".to_string(),
            row_tooltip_label: "Row".to_string(),
            value_tooltip_label: "Score".to_string(),
            delta_tooltip_label: "Delta".to_string(),
            signal_tooltip_label: "Signal".to_string(),
            tooltip_detail_1_label: None,
            tooltip_detail_2_label: None,
            tooltip_detail_3_label: None,
            strong_signal_label: "above baseline".to_string(),
            watch_signal_label: "watch".to_string(),
            neutral_signal_label: "neutral".to_string(),
            show_signal_glyphs: false,
            show_label_details: true,
            max_visible_labels: None,
        }
    }
}

/// Reusable heatmap specification.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapSpec {
    /// Cells in author order. Rows and columns are inferred first-seen.
    pub cells: Vec<HeatmapCell>,
    /// Optional explicit row order.
    pub rows: Vec<String>,
    /// Optional explicit column order.
    pub columns: Vec<String>,
    /// Layout and guide options.
    pub options: HeatmapOptions,
}

impl HeatmapSpec {
    /// Build a heatmap spec from sparse or dense cells.
    #[must_use]
    pub fn new(cells: Vec<HeatmapCell>) -> Self {
        Self {
            cells,
            rows: Vec::new(),
            columns: Vec::new(),
            options: HeatmapOptions::default(),
        }
    }

    /// Set explicit row order.
    #[must_use]
    pub fn with_rows(mut self, rows: Vec<impl Into<String>>) -> Self {
        self.rows = rows.into_iter().map(Into::into).collect();
        self
    }

    /// Set explicit column order.
    #[must_use]
    pub fn with_columns(mut self, columns: Vec<impl Into<String>>) -> Self {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// Set options wholesale.
    #[must_use]
    pub fn with_options(mut self, options: HeatmapOptions) -> Self {
        self.options = options;
        self
    }

    /// Return resolved row order.
    #[must_use]
    pub fn resolved_rows(&self) -> Vec<String> {
        if self.rows.is_empty() {
            infer_rows(&self.cells)
        } else {
            self.rows.clone()
        }
    }

    /// Return resolved column order.
    #[must_use]
    pub fn resolved_columns(&self) -> Vec<String> {
        if self.columns.is_empty() {
            infer_columns(&self.cells)
        } else {
            self.columns.clone()
        }
    }

    /// Compute signal counts from the resolved baselines.
    #[must_use]
    pub fn summary(&self) -> HeatmapSummary {
        let columns = self.resolved_columns();
        let baselines = resolved_column_baselines(&self.cells, &columns);
        let mut summary = HeatmapSummary::default();
        for cell in &self.cells {
            let baseline = cell
                .baseline
                .unwrap_or_else(|| baseline_for(&columns, &baselines, &cell.column));
            let delta = cell.value - baseline;
            summary.cells += 1;
            if delta >= self.options.signal_threshold {
                summary.strong += 1;
            } else if delta <= -self.options.signal_threshold {
                summary.watch += 1;
            } else {
                summary.neutral += 1;
            }
        }
        summary
    }

    /// Compile this spec into a chart.
    pub fn try_build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, HeatmapError> {
        <Self as ChartSpec>::build_chart(self, workspace, size)
    }

    fn validate(&self) -> Result<(), HeatmapError> {
        if self.cells.is_empty() {
            return Err(HeatmapError::EmptyData);
        }
        for cell in &self.cells {
            if cell.row.trim().is_empty() {
                return Err(HeatmapError::EmptyRow);
            }
            if cell.column.trim().is_empty() {
                return Err(HeatmapError::EmptyColumn);
            }
            if !cell.value.is_finite() {
                return Err(HeatmapError::NonFiniteValue {
                    row: cell.row.clone(),
                    column: cell.column.clone(),
                    value: cell.value,
                });
            }
            if cell.baseline.is_some_and(|baseline| !baseline.is_finite()) {
                return Err(HeatmapError::NonFiniteBaseline {
                    row: cell.row.clone(),
                    column: cell.column.clone(),
                    value: cell.baseline.unwrap_or_default(),
                });
            }
        }
        Ok(())
    }
}

/// Signal counts for a heatmap.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HeatmapSummary {
    /// Number of cells.
    pub cells: usize,
    /// Cells within the signal threshold.
    pub neutral: usize,
    /// Cells below negative threshold.
    pub watch: usize,
    /// Cells above positive threshold.
    pub strong: usize,
}

/// Error building a heatmap.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum HeatmapError {
    /// No cells were supplied.
    EmptyData,
    /// A row label was empty.
    EmptyRow,
    /// A column label was empty.
    EmptyColumn,
    /// A cell value was non-finite.
    NonFiniteValue {
        /// Row label.
        row: String,
        /// Column label.
        column: String,
        /// Bad value.
        value: f32,
    },
    /// A baseline was non-finite.
    NonFiniteBaseline {
        /// Row label.
        row: String,
        /// Column label.
        column: String,
        /// Bad value.
        value: f32,
    },
}

impl std::fmt::Display for HeatmapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyData => write!(f, "heatmap requires at least one cell"),
            Self::EmptyRow => write!(f, "heatmap row labels cannot be empty"),
            Self::EmptyColumn => write!(f, "heatmap column labels cannot be empty"),
            Self::NonFiniteValue { row, column, value } => write!(
                f,
                "heatmap value for `{row}` / `{column}` is not finite: {value}"
            ),
            Self::NonFiniteBaseline { row, column, value } => write!(
                f,
                "heatmap baseline for `{row}` / `{column}` is not finite: {value}"
            ),
        }
    }
}

impl std::error::Error for HeatmapError {}

impl ChartSpec for HeatmapSpec {
    type Error = HeatmapError;

    fn build_chart(
        &self,
        workspace: Arc<Workspace>,
        size: ChartSize,
    ) -> Result<Chart, Self::Error> {
        self.validate()?;

        let rows = self.resolved_rows();
        let columns = self.resolved_columns();
        let baselines = resolved_column_baselines(&self.cells, &columns);
        let header = heatmap_header_layout(size, &rows, &columns);
        let grid_w = (size.width as f32 - header.row_width).max(columns.len().max(1) as f32);
        let grid_h = (size.height as f32 - header.column_height).max(rows.len().max(1) as f32);
        let cell_w = grid_w / columns.len().max(1) as f32;
        let cell_h = grid_h / rows.len().max(1) as f32;
        let pad = self
            .options
            .cell_padding
            .min(cell_w * 0.35)
            .min(cell_h * 0.35)
            .max(0.0);

        let mut x1 = Vec::with_capacity(self.cells.len());
        let mut y1 = Vec::with_capacity(self.cells.len());
        let mut x2 = Vec::with_capacity(self.cells.len());
        let mut y2 = Vec::with_capacity(self.cells.len());
        let mut r = Vec::with_capacity(self.cells.len());
        let mut g = Vec::with_capacity(self.cells.len());
        let mut b = Vec::with_capacity(self.cells.len());
        let mut score_col = Vec::with_capacity(self.cells.len());
        let mut delta_col = Vec::with_capacity(self.cells.len());
        let mut row_col = Vec::with_capacity(self.cells.len());
        let mut column_col = Vec::with_capacity(self.cells.len());
        let mut signal_col = Vec::with_capacity(self.cells.len());
        let mut detail_1_col = Vec::with_capacity(self.cells.len());
        let mut detail_2_col = Vec::with_capacity(self.cells.len());
        let mut detail_3_col = Vec::with_capacity(self.cells.len());
        let mut labels = Vec::with_capacity(self.cells.len());
        let mut glyphs = Vec::new();
        let mut snap_targets = Vec::with_capacity(self.cells.len());

        for cell in &self.cells {
            let row_index = index_of(&rows, &cell.row).unwrap_or(0);
            let column_index = index_of(&columns, &cell.column).unwrap_or(0);
            let baseline = cell
                .baseline
                .unwrap_or_else(|| baseline_for(&columns, &baselines, &cell.column));
            let delta = cell.value - baseline;
            let signal = signal_label(delta, &self.options);
            let [cr, cg, cb] = heatmap_color(cell.value, delta);
            let left = header.row_width + column_index as f32 * cell_w;
            let top = header.column_height + row_index as f32 * cell_h;
            let center_x = left + cell_w * 0.5;
            let center_y = top + cell_h * 0.5;

            x1.push(left + pad);
            y1.push(top + pad);
            x2.push(left + cell_w - pad);
            y2.push(top + cell_h - pad);
            r.push(cr);
            g.push(cg);
            b.push(cb);
            score_col.push(cell.value * 100.0);
            delta_col.push(delta * 100.0);
            row_col.push(Arc::<str>::from(cell.row.clone()));
            column_col.push(Arc::<str>::from(cell.column.clone()));
            signal_col.push(Arc::<str>::from(signal));
            detail_1_col.push(Arc::<str>::from(
                cell.tooltip_detail_1.clone().unwrap_or_default(),
            ));
            detail_2_col.push(Arc::<str>::from(
                cell.tooltip_detail_2.clone().unwrap_or_default(),
            ));
            detail_3_col.push(Arc::<str>::from(
                cell.tooltip_detail_3.clone().unwrap_or_default(),
            ));
            let label_detail = if self.options.show_label_details {
                Some(
                    cell.label_detail
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| format_delta(delta * 100.0)),
                )
            } else {
                None
            };
            labels.push(heatmap_data_label(
                center_x,
                if label_detail.is_some() {
                    center_y + cell_h * 0.18
                } else {
                    center_y
                },
                cell.value,
                delta,
                self.options.signal_threshold,
                label_detail,
            ));
            snap_targets.push(
                SnapTarget::new(center_x, center_y, SnapKind::Center)
                    .with_radius(7.0)
                    .with_label(format!("{} {}", cell.row, cell.column)),
            );

            if self.options.show_signal_glyphs && delta.abs() >= self.options.signal_threshold {
                let positive = delta > 0.0;
                let glyph_x_offset = (cell_w * 0.18).clamp(8.0, 16.0);
                let glyph_y_offset = (cell_h * 0.28).clamp(8.0, 16.0);
                glyphs.push(PointPrim {
                    x: left + cell_w - pad - glyph_x_offset,
                    y: top + pad + glyph_y_offset,
                    r: 5.4,
                    shape: if positive { 3 } else { 2 },
                    fill: if positive {
                        rgba(0.08, 0.64, 0.46, 0.95)
                    } else {
                        rgba(0.86, 0.31, 0.28, 0.92)
                    },
                    stroke: rgba(0.07, 0.10, 0.14, 0.55),
                    stroke_width: 1.0,
                });
            }
        }

        workspace.upsert_scale(
            X_SCALE,
            Arc::new(LinearScale::new(
                (0.0, size.width as f64),
                (0.0, size.width as f32),
            )) as Arc<dyn Scale>,
        );
        workspace.upsert_scale(
            Y_SCALE,
            Arc::new(LinearScale::new(
                (0.0, size.height as f64),
                (0.0, size.height as f32),
            )) as Arc<dyn Scale>,
        );
        workspace.upsert_coord(COORD, Arc::new(CartesianCoord::new(X_SCALE, Y_SCALE)));
        workspace.upsert_dataset(Dataset::new(
            DATASET,
            1,
            vec![
                ("x1".into(), Column::F32(ColumnData::new(x1))),
                ("y1".into(), Column::F32(ColumnData::new(y1))),
                ("x2".into(), Column::F32(ColumnData::new(x2))),
                ("y2".into(), Column::F32(ColumnData::new(y2))),
                ("r".into(), Column::F32(ColumnData::new(r))),
                ("g".into(), Column::F32(ColumnData::new(g))),
                ("b".into(), Column::F32(ColumnData::new(b))),
                ("score".into(), Column::F32(ColumnData::new(score_col))),
                ("delta".into(), Column::F32(ColumnData::new(delta_col))),
                ("row".into(), Column::Utf8(ColumnData::new(row_col))),
                ("column".into(), Column::Utf8(ColumnData::new(column_col))),
                ("signal".into(), Column::Utf8(ColumnData::new(signal_col))),
                (
                    "detail_1".into(),
                    Column::Utf8(ColumnData::new(detail_1_col)),
                ),
                (
                    "detail_2".into(),
                    Column::Utf8(ColumnData::new(detail_2_col)),
                ),
                (
                    "detail_3".into(),
                    Column::Utf8(ColumnData::new(detail_3_col)),
                ),
            ],
        ));

        let mut cells = RectMark::new(
            CELL_MARK,
            DATASET,
            NumberChannel::Column {
                dataset: DATASET,
                name: "x1".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "y1".into(),
                scale: Y_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "x2".into(),
                scale: X_SCALE,
            },
            NumberChannel::Column {
                dataset: DATASET,
                name: "y2".into(),
                scale: Y_SCALE,
            },
            [0.42, 0.70, 0.95, 1.0],
        );
        cells.fill = ColorChannel::RgbaColumns {
            dataset: DATASET,
            r: "r".into(),
            g: "g".into(),
            b: "b".into(),
            a: None,
        };

        let mut scene = Scene::new(size.full_viewport());
        scene.layers.push(Layer {
            id: LAYER,
            coord: COORD,
            marks: vec![
                Arc::new(cells) as Arc<dyn Mark>,
                Arc::new(GeometryMark::new(
                    GLYPH_MARK,
                    Geometry::Points(glyphs),
                    Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
                )) as Arc<dyn Mark>,
            ],
            blend: berthacharts_core::BlendMode::Normal,
            opacity: 1.0,
            z: 0,
            clip: None,
        });
        let mut tooltip_fields = vec![
            TooltipField::new(self.options.row_tooltip_label.clone(), "row"),
            TooltipField::new(self.options.value_tooltip_label.clone(), "score").as_percent(0),
            TooltipField::new(self.options.delta_tooltip_label.clone(), "delta")
                .as_signed_percent(0),
            TooltipField::new(self.options.signal_tooltip_label.clone(), "signal").as_label(),
        ];
        if let Some(label) = &self.options.tooltip_detail_1_label {
            tooltip_fields.push(TooltipField::new(label.clone(), "detail_1").as_label());
        }
        if let Some(label) = &self.options.tooltip_detail_2_label {
            tooltip_fields.push(TooltipField::new(label.clone(), "detail_2").as_label());
        }
        if let Some(label) = &self.options.tooltip_detail_3_label {
            tooltip_fields.push(TooltipField::new(label.clone(), "detail_3").as_label());
        }
        scene.guides.push(Guide::Tooltip(
            TooltipGuide::new(CELL_MARK, DATASET, tooltip_fields).with_title_column("column"),
        ));
        let label_count = self
            .options
            .max_visible_labels
            .unwrap_or(labels.len())
            .min(labels.len());
        scene.guides.push(Guide::Labels(
            LabelGuide::new(heatmap_header_labels(
                &rows,
                &columns,
                header.row_width,
                header.column_height,
                cell_w,
                cell_h,
            ))
            .with_collision_padding(0.0),
        ));
        scene.guides.push(Guide::Labels(
            LabelGuide::new(labels)
                .with_collision_padding(0.0)
                .with_max_visible(label_count),
        ));
        scene.guides.push(Guide::Legend(
            LegendGuide::new(vec![
                LegendItem::new(
                    self.options.strong_signal_label.clone(),
                    [0.13, 0.66, 0.47, 1.0],
                ),
                LegendItem::new(
                    self.options.watch_signal_label.clone(),
                    [0.85, 0.37, 0.33, 1.0],
                ),
                LegendItem::new(
                    self.options.neutral_signal_label.clone(),
                    [0.56, 0.70, 0.82, 1.0],
                ),
            ])
            .with_title(self.options.legend_title.clone())
            .with_anchor(LegendAnchor::Bottom),
        ));
        scene
            .interactions
            .push(berthacharts_core::Interaction::SnapTargets(
                SnapTargetSet::new(snap_targets).with_name("cell centers"),
            ));

        let mut chart = Chart::new(workspace, scene.viewport);
        chart.set_scene(scene);
        Ok(chart)
    }
}

#[derive(Debug, Clone, Copy)]
struct HeatmapHeaderLayout {
    row_width: f32,
    column_height: f32,
}

fn heatmap_header_layout(
    size: ChartSize,
    rows: &[String],
    columns: &[String],
) -> HeatmapHeaderLayout {
    let row_chars = rows
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let column_chars = columns
        .iter()
        .map(|column| column.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let row_width = (row_chars * 8.0 + 42.0)
        .clamp(72.0, 112.0)
        .min(size.width as f32 * 0.24);
    let column_height = (column_chars * 0.4 + 24.0)
        .clamp(24.0, 34.0)
        .min(size.height as f32 * 0.12);

    HeatmapHeaderLayout {
        row_width,
        column_height,
    }
}

fn heatmap_header_labels(
    rows: &[String],
    columns: &[String],
    row_width: f32,
    column_height: f32,
    cell_w: f32,
    cell_h: f32,
) -> Vec<LabelItem> {
    let mut labels = Vec::with_capacity(rows.len() + columns.len());

    for (index, column) in columns.iter().enumerate() {
        labels.push(
            LabelItem::new(
                row_width + index as f32 * cell_w + cell_w * 0.5,
                column_height * 0.5,
                column,
            )
            .with_kind(LabelKind::Column)
            .with_priority(LabelPriority::Required)
            .with_anchor(LabelAnchor::Center)
            .with_reposition(false),
        );
    }

    for (index, row) in rows.iter().enumerate() {
        labels.push(
            LabelItem::new(
                row_width - 8.0,
                column_height + index as f32 * cell_h + cell_h * 0.5,
                row,
            )
            .with_kind(LabelKind::Column)
            .with_priority(LabelPriority::Required)
            .with_anchor(LabelAnchor::Left)
            .with_reposition(false),
        );
    }

    labels
}

fn infer_rows(cells: &[HeatmapCell]) -> Vec<String> {
    let mut rows = Vec::new();
    for cell in cells {
        if !rows.contains(&cell.row) {
            rows.push(cell.row.clone());
        }
    }
    rows
}

fn infer_columns(cells: &[HeatmapCell]) -> Vec<String> {
    let mut columns = Vec::new();
    for cell in cells {
        if !columns.contains(&cell.column) {
            columns.push(cell.column.clone());
        }
    }
    columns
}

fn index_of(items: &[String], value: &str) -> Option<usize> {
    items.iter().position(|item| item == value)
}

fn resolved_column_baselines(cells: &[HeatmapCell], columns: &[String]) -> Vec<f32> {
    columns
        .iter()
        .map(|column| {
            let mut sum = 0.0;
            let mut count = 0usize;
            for cell in cells {
                if &cell.column == column {
                    sum += cell.value;
                    count += 1;
                }
            }
            if count == 0 {
                0.0
            } else {
                sum / count as f32
            }
        })
        .collect()
}

fn baseline_for(columns: &[String], baselines: &[f32], column: &str) -> f32 {
    index_of(columns, column)
        .and_then(|index| baselines.get(index).copied())
        .unwrap_or_default()
}

fn signal_label(delta: f32, options: &HeatmapOptions) -> &str {
    if delta >= options.signal_threshold {
        &options.strong_signal_label
    } else if delta <= -options.signal_threshold {
        &options.watch_signal_label
    } else {
        &options.neutral_signal_label
    }
}

fn heatmap_data_label(
    x: f32,
    y: f32,
    score: f32,
    delta: f32,
    threshold: f32,
    detail: Option<String>,
) -> LabelItem {
    let mut label = LabelItem::new(x, y, format!("{:.0}", score * 100.0))
        .with_kind(LabelKind::Data)
        .with_priority(if delta.abs() >= threshold {
            LabelPriority::Required
        } else {
            LabelPriority::Important
        })
        .with_anchor(LabelAnchor::Center)
        .with_reposition(false);
    if let Some(detail) = detail {
        label = label.with_detail(detail);
    }
    label
}

fn format_delta(value: f32) -> String {
    if value >= 0.0 {
        format!("+{value:.0}")
    } else {
        format!("{value:.0}")
    }
}

fn heatmap_color(score: f32, delta: f32) -> [f32; 3] {
    if delta < -0.07 {
        let t = ((score - 0.45) / 0.18).clamp(0.0, 1.0);
        [0.80 + 0.08 * t, 0.43 + 0.12 * t, 0.38 + 0.10 * t]
    } else {
        let t = ((score - 0.50) / 0.35).clamp(0.0, 1.0);
        [0.40 - 0.16 * t, 0.58 + 0.22 * t, 0.82 - 0.18 * t]
    }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r * a, g * a, b * a, a]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_spec() -> HeatmapSpec {
        let segments = ["SMB", "Midmarket", "Enterprise"];
        let metrics = ["Conversion", "Activation", "Retention", "Revenue"];
        let scores = [
            [0.82, 0.71, 0.57, 0.67],
            [0.76, 0.64, 0.51, 0.56],
            [0.67, 0.60, 0.47, 0.50],
        ];
        let baselines = [0.70, 0.66, 0.58, 0.60];
        let mut cells = Vec::new();
        for (row, segment) in segments.iter().enumerate() {
            for (column, metric) in metrics.iter().enumerate() {
                cells.push(
                    HeatmapCell::new(*segment, *metric, scores[row][column])
                        .with_baseline(baselines[column]),
                );
            }
        }
        HeatmapSpec::new(cells)
            .with_rows(segments.to_vec())
            .with_columns(metrics.to_vec())
    }

    #[test]
    fn heatmap_spec_builds_chart_with_guides_and_snap_targets() {
        let chart = demo_spec()
            .try_build_chart(Workspace::new(), ChartSize::new(420, 320))
            .unwrap();

        assert_eq!(chart.scene().layers.len(), 1);
        assert!(!chart.scene().guides.is_empty());
        assert_eq!(chart.snap_targets().len(), 12);
    }

    #[test]
    fn heatmap_spec_supports_sparse_cells() {
        let spec = HeatmapSpec::new(vec![
            HeatmapCell::new("A", "one", 0.8),
            HeatmapCell::new("B", "two", 0.6),
        ]);

        let chart = spec
            .try_build_chart(Workspace::new(), ChartSize::new(220, 160))
            .unwrap();

        assert_eq!(chart.snap_targets().len(), 2);
    }

    #[test]
    fn heatmap_summary_counts_signals() {
        let summary = demo_spec().summary();

        assert_eq!(summary.cells, 12);
        assert_eq!(summary.strong, 1);
        assert_eq!(summary.watch, 2);
    }

    #[test]
    fn heatmap_rejects_non_finite_values() {
        let err = HeatmapSpec::new(vec![HeatmapCell::new("A", "B", f32::INFINITY)])
            .try_build_chart(Workspace::new(), ChartSize::new(220, 160))
            .unwrap_err();

        assert!(matches!(err, HeatmapError::NonFiniteValue { .. }));
    }
}
