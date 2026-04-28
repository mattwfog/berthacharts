struct Viewport {
    width: f32,
    height: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(v: VertexInput) -> VsOut {
    let cx = (v.pos.x / viewport.width) * 2.0 - 1.0;
    let cy = 1.0 - (v.pos.y / viewport.height) * 2.0;

    var out: VsOut;
    out.clip = vec4<f32>(cx, cy, 0.0, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
