//! Rectangle mark — one rect per data row.
//!
//! Channels:
//! - `x` / `y` — top-left corner in domain space
//! - `x2` / `y2` — bottom-right corner in domain space
//! - `fill` — color
//!
//! The mark projects each corner through the layer's coord and emits
//! [`RectPrim`](crate::geometry::RectPrim) instances for the renderer.

use crate::channel::{ColorChannel, NumberChannel};
use crate::coord::Unprojected;
use crate::dataset::DatasetId;
use crate::geometry::{Geometry, RectPrim};
use crate::ids::MarkId;
use crate::mark::{Mark, PickCtx, PickHit, TessellateCtx};
use crate::scene::Rect;

/// Simple rectangle mark.
#[derive(Debug, Clone)]
pub struct RectMark {
    /// Stable identifier.
    pub id: MarkId,
    /// Source dataset. Constants-only marks may leave this pointing at an
    /// empty dataset — iteration is driven by its row count.
    pub dataset: DatasetId,
    /// Left-edge channel.
    pub x: NumberChannel,
    /// Top-edge channel.
    pub y: NumberChannel,
    /// Right-edge channel.
    pub x2: NumberChannel,
    /// Bottom-edge channel.
    pub y2: NumberChannel,
    /// Fill color channel.
    pub fill: ColorChannel,
    /// Stroke color channel.
    pub stroke: ColorChannel,
    /// Stroke width in pixels.
    pub stroke_width: f32,
    /// Corner radius in pixels.
    pub radius: f32,
}

impl RectMark {
    /// Build a rect mark with a dataset-driven x/y/x2/y2 and constant styling.
    #[must_use]
    pub fn new(
        id: MarkId,
        dataset: DatasetId,
        x: NumberChannel,
        y: NumberChannel,
        x2: NumberChannel,
        y2: NumberChannel,
        fill: [f32; 4],
    ) -> Self {
        Self {
            id,
            dataset,
            x,
            y,
            x2,
            y2,
            fill: ColorChannel::Constant(fill),
            stroke: ColorChannel::Constant([0.0; 4]),
            stroke_width: 0.0,
            radius: 0.0,
        }
    }
}

fn resolve_number(ch: &NumberChannel, ctx: &TessellateCtx<'_>, row: usize) -> f32 {
    match ch {
        NumberChannel::Constant(v) => *v,
        NumberChannel::Column {
            dataset,
            name,
            scale,
        } => {
            let Some(ds) = ctx.datasets.get(*dataset) else {
                return f32::NAN;
            };
            let Some(col) = ds.column(name) else {
                return f32::NAN;
            };
            let Some(val) = col.read_f64(row) else {
                return f32::NAN;
            };
            let Some(sc) = ctx.scales.get(*scale) else {
                return f32::NAN;
            };
            sc.project(val)
        }
        NumberChannel::Offset { base, offset } => resolve_number(base, ctx, row) + offset,
    }
}

fn resolve_number_pick(ch: &NumberChannel, ctx: &PickCtx<'_>, row: usize) -> f32 {
    match ch {
        NumberChannel::Constant(v) => *v,
        NumberChannel::Column {
            dataset,
            name,
            scale,
        } => {
            let Some(ds) = ctx.datasets.get(*dataset) else {
                return f32::NAN;
            };
            let Some(col) = ds.column(name) else {
                return f32::NAN;
            };
            let Some(val) = col.read_f64(row) else {
                return f32::NAN;
            };
            let Some(sc) = ctx.scales.get(*scale) else {
                return f32::NAN;
            };
            sc.project(val)
        }
        NumberChannel::Offset { base, offset } => resolve_number_pick(base, ctx, row) + offset,
    }
}

fn resolve_color(ch: &ColorChannel, ctx: &TessellateCtx<'_>, row: usize) -> [f32; 4] {
    match ch {
        ColorChannel::Constant(c) => *c,
        // Palette-through-scale color resolution lands alongside color scales
        // (v0.1.1). For now the column case renders as fully transparent so
        // the pipeline still makes progress.
        ColorChannel::Column { .. } => [0.0; 4],
        ColorChannel::RgbaColumns {
            dataset,
            r,
            g,
            b,
            a,
        } => {
            let Some(ds) = ctx.datasets.get(*dataset) else {
                return [0.0; 4];
            };
            let read = |name: &str| {
                ds.column(name)
                    .and_then(|col| col.read_f64(row))
                    .map(|v| v.clamp(0.0, 1.0) as f32)
            };
            [
                read(r).unwrap_or(0.0),
                read(g).unwrap_or(0.0),
                read(b).unwrap_or(0.0),
                a.as_deref().and_then(read).unwrap_or(1.0),
            ]
        }
    }
}

impl Mark for RectMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= u64::from(self.dataset.get());
        h = h.wrapping_mul(0x0100_0000_01b3);
        self.x.hash_into(&mut h);
        self.y.hash_into(&mut h);
        self.x2.hash_into(&mut h);
        self.y2.hash_into(&mut h);
        self.fill.hash_into(&mut h);
        self.stroke.hash_into(&mut h);
        h ^= u64::from(self.stroke_width.to_bits());
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= u64::from(self.radius.to_bits());
        h
    }

    fn tessellate(&self, ctx: &TessellateCtx<'_>) -> Geometry {
        let row_count = ctx.datasets.get(self.dataset).map_or(0, |d| d.len());

        // Allow constant-only marks to emit a single rect when there's no
        // dataset or the dataset is empty.
        let n = if row_count == 0 && is_all_constant(self) {
            1
        } else {
            row_count
        };
        if n == 0 {
            return Geometry::Empty;
        }

        let mut rects = Vec::with_capacity(n);
        for row in 0..n {
            let u1 = resolve_number(&self.x, ctx, row);
            let v1 = resolve_number(&self.y, ctx, row);
            let u2 = resolve_number(&self.x2, ctx, row);
            let v2 = resolve_number(&self.y2, ctx, row);

            let p1 = ctx.coord.project(Unprojected {
                u: f64::from(u1),
                v: f64::from(v1),
            });
            let p2 = ctx.coord.project(Unprojected {
                u: f64::from(u2),
                v: f64::from(v2),
            });

            let x = p1.x.min(p2.x);
            let y = p1.y.min(p2.y);
            let w = (p2.x - p1.x).abs();
            let h = (p2.y - p1.y).abs();

            rects.push(RectPrim {
                x,
                y,
                w,
                h,
                fill: resolve_color(&self.fill, ctx, row),
                stroke: resolve_color(&self.stroke, ctx, row),
                stroke_width: self.stroke_width,
                radius: self.radius,
            });
        }
        Geometry::Rects(rects)
    }

    fn pick(&self, ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        let row_count = ctx.datasets.get(self.dataset).map_or(0, |d| d.len());
        let n = if row_count == 0 && is_all_constant(self) {
            1
        } else {
            row_count
        };

        for row in (0..n).rev() {
            let u1 = resolve_number_pick(&self.x, ctx, row);
            let v1 = resolve_number_pick(&self.y, ctx, row);
            let u2 = resolve_number_pick(&self.x2, ctx, row);
            let v2 = resolve_number_pick(&self.y2, ctx, row);

            let p1 = ctx.coord.project(Unprojected {
                u: f64::from(u1),
                v: f64::from(v1),
            });
            let p2 = ctx.coord.project(Unprojected {
                u: f64::from(u2),
                v: f64::from(v2),
            });
            let x = p1.x.min(p2.x);
            let y = p1.y.min(p2.y);
            let w = (p2.x - p1.x).abs();
            let h = (p2.y - p1.y).abs();
            if Rect::new(x, y, w, h).contains(point) {
                return Some(PickHit {
                    mark: self.id,
                    row: if row_count == 0 { None } else { Some(row) },
                    distance: 0.0,
                    payload: None,
                });
            }
        }

        None
    }

    fn bounds(&self, ctx: &TessellateCtx<'_>) -> Rect {
        match self.tessellate(ctx) {
            Geometry::Rects(rs) if !rs.is_empty() => {
                let mut xmin = f32::INFINITY;
                let mut ymin = f32::INFINITY;
                let mut xmax = f32::NEG_INFINITY;
                let mut ymax = f32::NEG_INFINITY;
                for r in rs {
                    xmin = xmin.min(r.x);
                    ymin = ymin.min(r.y);
                    xmax = xmax.max(r.x + r.w);
                    ymax = ymax.max(r.y + r.h);
                }
                Rect::new(xmin, ymin, xmax - xmin, ymax - ymin)
            }
            _ => Rect::ZERO,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn is_all_constant(m: &RectMark) -> bool {
    is_constant_number(&m.x)
        && is_constant_number(&m.y)
        && is_constant_number(&m.x2)
        && is_constant_number(&m.y2)
        && matches!(m.fill, ColorChannel::Constant(_))
}

fn is_constant_number(ch: &NumberChannel) -> bool {
    match ch {
        NumberChannel::Constant(_) => true,
        NumberChannel::Offset { base, .. } => is_constant_number(base),
        NumberChannel::Column { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ScaleId;

    #[test]
    fn fingerprint_tracks_channel_configuration() {
        let base = RectMark::new(
            MarkId::new(1),
            DatasetId::new(1),
            NumberChannel::Column {
                dataset: DatasetId::new(1),
                name: "x".into(),
                scale: ScaleId::new(1),
            },
            NumberChannel::Constant(0.0),
            NumberChannel::Constant(10.0),
            NumberChannel::Constant(10.0),
            [1.0, 0.0, 0.0, 1.0],
        );
        let mut changed = base.clone();
        changed.x = NumberChannel::Column {
            dataset: DatasetId::new(1),
            name: "x2".into(),
            scale: ScaleId::new(1),
        };

        assert_ne!(base.fingerprint(), changed.fingerprint());
    }
}
