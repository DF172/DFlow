struct Params {
    texel: vec2<f32>,
    out_inv: vec2<f32>,
    radius: f32,
    taps: u32,
    pad0: u32,
    pad1: u32,
};

@group(0) @binding(0)
var src: texture_2d<f32>;

@group(0) @binding(1)
var src_sampler: sampler;

@group(0) @binding(2)
var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(index & 1u) * 4.0 - 1.0;
    let y = f32((index >> 1u) & 1u) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) location: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = location.xy * params.out_inv;
    let center = textureSampleLevel(src, src_sampler, uv, 0.0);

    if (params.taps == 0u) {
        return center;
    }

    let step = params.texel * params.radius;
    let near = 0.3162162162;
    let far = 0.0702702703;

    var color = center * 0.2270270270;
    color = color + textureSampleLevel(src, src_sampler, uv + step, 0.0) * near;
    color = color + textureSampleLevel(src, src_sampler, uv - step, 0.0) * near;
    color = color + textureSampleLevel(src, src_sampler, uv + step * 2.0, 0.0) * far;
    color = color + textureSampleLevel(src, src_sampler, uv - step * 2.0, 0.0) * far;

    return color;
}
