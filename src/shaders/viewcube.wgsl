// ViewCube shader — theme-driven navigation cube.
//
// Every vertex carries a *role*, not a literal colour: the theme decides what
// surface / faint / ink / muted / accent mean, so the same geometry reads
// correctly on the light sheet and on the dark ink theme.
//
//   role 0 = surface (cube faces, chamfers, corners)
//   role 1 = faint   (silhouette outline, compass ring)
//   role 2 = ink
//   role 3 = muted
//   role 4 = accent  (hover)

struct Uniforms {
    view_proj:     mat4x4<f32>,
    rotation:      mat4x4<f32>,
    hover_region:  vec4<f32>,
    ink_color:     vec4<f32>,
    faint_color:   vec4<f32>,
    muted_color:   vec4<f32>,
    accent_color:  vec4<f32>,
    surface_color: vec4<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

fn role_color(role: f32) -> vec3<f32> {
    if role < 0.5 {
        return u.surface_color.rgb;
    }
    if role < 1.5 {
        return u.faint_color.rgb;
    }
    if role < 2.5 {
        return u.ink_color.rgb;
    }
    if role < 3.5 {
        return u.muted_color.rgb;
    }
    return u.accent_color.rgb;
}

// ── Cube body ─────────────────────────────────────────────────────────────

struct VIn {
    @location(0) pos:      vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) role:     f32,
    @location(3) region_f: f32,
}

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal:   vec3<f32>,
    @location(1) role:     f32,
    @location(2) region_f: f32,
}

@vertex
fn vs_main(in: VIn) -> VOut {
    var out: VOut;
    let rotated = (u.rotation * vec4<f32>(in.pos, 1.0)).xyz;
    out.clip     = u.view_proj * vec4<f32>(rotated, 1.0);
    out.normal   = (u.rotation * vec4<f32>(in.normal, 0.0)).xyz;
    out.role     = in.role;
    out.region_f = in.region_f;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    var rgb = role_color(in.role);
    let normal = normalize(in.normal);
    // `region_f` is region / 25, so one region is 0.04 apart; half a step
    // matches the hovered region alone and nothing on either side of it.
    let hovered = u.hover_region.x >= 0.0 && abs(in.region_f - u.hover_region.x) < 0.02;

    if in.role < 0.5 {
        // The outline carries the silhouette; this shading carries the bevel.
        // At a three-quarter view the three faces share one tone, so the only
        // depth cue inside the silhouette is the brighter chamfer/corner band.
        rgb = rgb * (0.84 + 0.16 * clamp(normal.z, 0.0, 1.0));
        if hovered {
            rgb = mix(rgb, u.accent_color.rgb, 0.20);
        }
    }
    return vec4<f32>(rgb, 1.0);
}

// ── Outlines ──────────────────────────────────────────────────────────────

struct LineIn {
    @location(0) pos:      vec3<f32>,
    @location(1) normal_a: vec3<f32>,
    @location(2) normal_b: vec3<f32>,
    @location(3) region_a: f32,
    @location(4) region_b: f32,
    @location(5) always:   f32,
}

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) show:  f32,
}

@vertex
fn line_vs_main(in: LineIn) -> LineOut {
    var out: LineOut;
    let rotated = (u.rotation * vec4<f32>(in.pos, 1.0)).xyz;
    out.clip = u.view_proj * vec4<f32>(rotated, 1.0);

    let na = normalize((u.rotation * vec4<f32>(in.normal_a, 0.0)).xyz);
    let nb = normalize((u.rotation * vec4<f32>(in.normal_b, 0.0)).xyz);
    // A silhouette edge separates a face pointing at the camera from one
    // pointing away. Interior creases between two visible faces stay hidden so
    // the body reads as one solid instead of a wireframe of blocks.
    let silhouette = (na.z > 0.0) != (nb.z > 0.0);
    let hovered = u.hover_region.x >= 0.0
        && (abs(in.region_a - u.hover_region.x) < 0.02
            || abs(in.region_b - u.hover_region.x) < 0.02);
    let show = in.always > 0.5 || silhouette || hovered;

    out.color = select(u.faint_color.rgb, u.accent_color.rgb, hovered);
    out.show  = select(0.0, 1.0, show);
    return out;
}

@fragment
fn line_fs_main(in: LineOut) -> @location(0) vec4<f32> {
    if in.show < 0.5 {
        discard;
    }
    return vec4<f32>(in.color, 1.0);
}
