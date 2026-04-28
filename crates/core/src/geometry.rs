//! Renderer-neutral geometry primitives.
//!
//! Marks do not emit GPU draw calls. They emit [`Geometry`] — a tight set of
//! primitives the renderer knows how to draw. This keeps the renderer trait
//! surface tiny and lets the same mark run on any backend (wgpu today, SVG
//! export tomorrow).

/// Output of [`crate::Mark::tessellate`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Geometry {
    /// Axis-aligned rectangles (bars, cells, ticks).
    Rects(Vec<RectPrim>),
    /// Filled triangles with per-vertex color.
    Triangles(Vec<TrianglePrim>),
    /// Points drawn as instanced quads (scatter).
    Points(Vec<PointPrim>),
    /// Polyline strips — renderer expands via a wide-line shader.
    Lines(Vec<LinePrim>),
    /// SVG-style path commands — renderer tessellates on upload.
    Paths(Vec<PathPrim>),
    /// Composite of multiple geometries in a single mark.
    Mixed(Vec<Geometry>),
    /// No geometry (mark culled, or placeholder).
    Empty,
}

/// Axis-aligned rectangle primitive in screen pixels.
#[derive(Debug, Clone, Copy)]
pub struct RectPrim {
    /// Top-left x.
    pub x: f32,
    /// Top-left y.
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
    /// Pre-multiplied fill color.
    pub fill: [f32; 4],
    /// Pre-multiplied stroke color.
    pub stroke: [f32; 4],
    /// Stroke width in pixels (0 disables).
    pub stroke_width: f32,
    /// Corner radius in pixels.
    pub radius: f32,
}

/// Filled triangle primitive in screen pixels.
#[derive(Debug, Clone, Copy)]
pub struct TrianglePrim {
    /// First vertex `[x, y]`.
    pub a: [f32; 2],
    /// Second vertex `[x, y]`.
    pub b: [f32; 2],
    /// Third vertex `[x, y]`.
    pub c: [f32; 2],
    /// Fill color.
    pub fill: [f32; 4],
}

/// Point primitive — rendered as an instanced shape.
#[derive(Debug, Clone, Copy)]
pub struct PointPrim {
    /// Center x.
    pub x: f32,
    /// Center y.
    pub y: f32,
    /// Radius in pixels.
    pub r: f32,
    /// Shape id (0=circle, 1=square, 2=triangle, 3=diamond, ...).
    pub shape: u32,
    /// Fill color.
    pub fill: [f32; 4],
    /// Stroke color.
    pub stroke: [f32; 4],
    /// Stroke width.
    pub stroke_width: f32,
}

/// Polyline primitive — renderer draws as a wide-line strip.
#[derive(Debug, Clone)]
pub struct LinePrim {
    /// Ordered vertex positions in screen pixels.
    pub points: Vec<[f32; 2]>,
    /// Stroke color.
    pub stroke: [f32; 4],
    /// Stroke width in pixels.
    pub width: f32,
    /// Optional dash pattern (on, off, on, off, ...).
    pub dash: Option<Vec<f32>>,
    /// Line-join style (0=miter, 1=round, 2=bevel).
    pub join: u32,
    /// Line-cap style (0=butt, 1=round, 2=square).
    pub cap: u32,
}

/// Path primitive — renderer tessellates via `lyon`.
#[derive(Debug, Clone)]
pub struct PathPrim {
    /// Path commands in screen pixels.
    pub commands: Vec<PathCommand>,
    /// Fill color (alpha=0 disables fill).
    pub fill: [f32; 4],
    /// Stroke color.
    pub stroke: [f32; 4],
    /// Stroke width.
    pub stroke_width: f32,
}

/// A single path command (subset of SVG path ops).
///
/// Positions are absolute screen pixels. The renderer tessellates commands
/// into triangle strips on upload.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
#[allow(missing_docs)] // field names are standard SVG-path coordinate names
pub enum PathCommand {
    /// Move pen to absolute position.
    MoveTo { x: f32, y: f32 },
    /// Straight line to absolute position.
    LineTo { x: f32, y: f32 },
    /// Quadratic Bézier — `(cx, cy)` is the control point, `(x, y)` the endpoint.
    QuadTo { cx: f32, cy: f32, x: f32, y: f32 },
    /// Cubic Bézier — `(c1, c2)` are control points, `(x, y)` the endpoint.
    CubicTo {
        c1x: f32,
        c1y: f32,
        c2x: f32,
        c2y: f32,
        x: f32,
        y: f32,
    },
    /// Close current sub-path.
    Close,
}
