// Wet-ink triangles in pixel space.
struct View { size: vec2<f32>, _pad: vec2<f32> };
@group(0) @binding(0) var<uniform> view: View;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_ink(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    let ndc = pos / view.size * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_ink(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}

// Full-screen blit of the ink texture onto a swapchain image (subsurface mode).
@group(0) @binding(0) var ink_tex: texture_2d<f32>;
@group(0) @binding(1) var ink_smp: sampler;

struct BlitOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) i: u32) -> BlitOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: BlitOut;
    out.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_blit(in: BlitOut) -> @location(0) vec4<f32> {
    return textureSample(ink_tex, ink_smp, in.uv);
}
