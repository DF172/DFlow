struct Glass {
    // Physical pixel origin of the plate inside the frame.
    plate_origin: vec2<f32>,
    // Physical pixel size of the plate (= the viewport this draw is clipped to).
    plate_size: vec2<f32>,
    radius: f32,
    edge: f32,
    // Physical pixel size of the whole frame the backdrop was taken from.
    frame_size: vec2<f32>,
    saturate: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
    tint: vec4<f32>,
    edge_color: vec4<f32>,
    hi_top: vec4<f32>,
    hi_bottom: vec4<f32>,
};

@group(0) @binding(0)
var backdrop: texture_2d<f32>;

@group(0) @binding(1)
var backdrop_sampler: sampler;

@group(0) @binding(2)
var<uniform> glass: Glass;

// A full-viewport triangle. The pass already carries the plate's viewport and
// scissor, so coverage comes from the clip rect and this shader only has to
// decide which of those pixels belong to the rounded plate.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(index & 1u) * 4.0 - 1.0;
    let y = f32((index >> 1u) & 1u) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Signed distance to the plate's boundary: negative inside, zero on the rim.
fn plate_sd(local: vec2<f32>) -> f32 {
    let half = glass.plate_size * 0.5;
    let r = min(glass.radius, min(half.x, half.y));
    let q = abs(local - half) - (half - vec2<f32>(r));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let local = position.xy;
    let sd = plate_sd(local);
    let aa = max(fwidth(sd), 0.5);
    let cover = 1.0 - smoothstep(-aa, aa, sd);
    if (cover <= 0.001) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // What was painted under this pixel, already blurred by the backdrop chain.
    let uv = (glass.plate_origin + local) / glass.frame_size;
    let raw = textureSampleLevel(backdrop, backdrop_sampler, uv, 0.0).rgb;

    // Blur alone desaturates; the reference pushes colour back by the same
    // amount everywhere, around Rec.709 luma.
    let luma = dot(raw, vec3<f32>(0.2126, 0.7152, 0.0722));
    var rgb = mix(vec3<f32>(luma), raw, glass.saturate);

    // The fill decides legibility, so it composites over the read backdrop
    // rather than tinting it additively.
    rgb = mix(rgb, glass.tint.rgb, glass.tint.a);

    // Top-face specular: a vertical ramp, strongest at the rim that catches
    // the light.
    let down = clamp(local.y / max(glass.plate_size.y, 1.0), 0.0, 1.0);
    let hi = mix(glass.hi_top, glass.hi_bottom, down);
    rgb = mix(rgb, hi.rgb, hi.a);

    // The rim lives inside the boundary, over the last `edge` pixels: `sd` runs
    // from -edge at the inner foot of the band to 0 on the boundary itself, so
    // the mask is the ramp as-is and not its complement.
    let rim = smoothstep(0.0, 1.0, clamp((sd + glass.edge) / glass.edge, 0.0, 1.0));
    rgb = mix(rgb, glass.edge_color.rgb, glass.edge_color.a * rim);

    return vec4<f32>(rgb, cover);
}
