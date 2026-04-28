use berthacharts_core::{
    Geometry, LinePrim, Mark, MarkId, PickCtx, PickHit, PointPrim, Rect, TessellateCtx,
};

#[derive(Debug, Clone)]
pub(crate) struct GeometryMark {
    id: MarkId,
    geometry: Geometry,
    bounds: Rect,
}

impl GeometryMark {
    pub(crate) fn new(id: MarkId, geometry: Geometry, bounds: Rect) -> Self {
        Self {
            id,
            geometry,
            bounds,
        }
    }
}

impl Mark for GeometryMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        self.geometry.clone()
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.bounds
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PointCollectionMark {
    id: MarkId,
    points: Vec<PointPrim>,
    bounds: Rect,
    hit_slop: f32,
}

impl PointCollectionMark {
    pub(crate) fn new(id: MarkId, points: Vec<PointPrim>, bounds: Rect) -> Self {
        Self {
            id,
            points,
            bounds,
            hit_slop: 3.0,
        }
    }
}

impl Mark for PointCollectionMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.points.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        Geometry::Points(self.points.clone())
    }

    fn pick(&self, _ctx: &PickCtx<'_>, point: (f32, f32)) -> Option<PickHit> {
        self.points
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(row, primitive)| {
                let dx = point.0 - primitive.x;
                let dy = point.1 - primitive.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let radius = primitive.r + self.hit_slop;
                (distance <= radius).then_some(PickHit {
                    mark: self.id,
                    row: Some(row),
                    distance,
                    payload: None,
                })
            })
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.bounds
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LineCollectionMark {
    id: MarkId,
    lines: Vec<LinePrim>,
    bounds: Rect,
}

impl LineCollectionMark {
    pub(crate) fn new(id: MarkId, lines: Vec<LinePrim>, bounds: Rect) -> Self {
        Self { id, lines, bounds }
    }
}

impl Mark for LineCollectionMark {
    fn id(&self) -> MarkId {
        self.id
    }

    fn fingerprint(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= self.id.get();
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= self.lines.len() as u64;
        h
    }

    fn tessellate(&self, _ctx: &TessellateCtx<'_>) -> Geometry {
        Geometry::Lines(self.lines.clone())
    }

    fn pick(&self, _ctx: &PickCtx<'_>, _point: (f32, f32)) -> Option<PickHit> {
        None
    }

    fn bounds(&self, _ctx: &TessellateCtx<'_>) -> Rect {
        self.bounds
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
