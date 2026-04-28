// Rect instanced pipeline — fill only for v0.1.
// Stroke / radius fields are forwarded through but ignored in the fragment
// stage; they land in v0.1.1 alongside signed-distance rounded-rect support.

struct Viewport {
    width: f32,
    height: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) unit_pos: vec2<f32>, // quad corner in [0,1]^2
}

struct InstanceInput {
    @location(1) rect:         vec4<f32>, // x, y, w, h  (screen pixels, y-down)
    @location(2) fill:         vec4<f32>, // rgba (pre-multiplied)
    @location(3) stroke:       vec4<f32>, // rgba (reserved)
    @location(4) stroke_width: f32,
    @location(5) radius:       f32,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(v: VertexInput, inst: InstanceInput) -> VsOut {
    let px = inst.rect.x + v.unit_pos.x * inst.rect.z;
    let py = inst.rect.y + v.unit_pos.y * inst.rect.w;

    let cx = (px / viewport.width) * 2.0 - 1.0;
    let cy = 1.0 - (py / viewport.height) * 2.0;

    var out: VsOut;
    out.clip = vec4<f32>(cx, cy, 0.0, 1.0);
    out.color = inst.fill;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
