// Draws the sky's stars (SKY-09 to SKY-12, SKY-16, SKY-18, SKY-21). Each
// star is one record in a storage buffer and a square in the mesh whose
// six vertices carry only a slot: which record, and which corner. The
// record's direction is turned by the camera's rotation alone and
// projected at infinity, so panning and zooming never move it; the square
// is then spread to the star's reach in screen pixels and a Gaussian is
// shaded in it, brightest at the centre. The light is added in the
// display's sRGB levels over the ground, as an opacity would blend there.
// Mirrors `projection.rs` and `look.rs`, and `StarsUniform` and
// `star_records` in `draw.rs`.

import bevy_pbr::render::mesh_view_bindings::view;

struct SkyStars {
    // The ground colour, sRGB.
    ground: vec4<f32>,
    // The map area in physical pixels: min x, min y, max x, max y.
    clip: vec4<f32>,
    // Physical pixels per logical pixel.
    px_scale: f32,
    // A star is shaded out to this many standard deviations, plus this
    // many pixels.
    reach_sigmas: f32,
    reach_extra_px: f32,
    // What every star's opacity is multiplied by in the layer shown: 1 in
    // the system view, `sector_sky_factor` in the sector view.
    factor: f32,
}

struct Star {
    // The star's direction in render space, then its softness in logical
    // pixels.
    direction_sigma: vec4<f32>,
    // The star's colour, sRGB, then its opacity.
    colour_opacity: vec4<f32>,
}

@group(3) @binding(0)
var<uniform> sky: SkyStars;
@group(3) @binding(1)
var<storage, read> stars: array<Star>;

struct Vertex {
    // 4 * star + corner: the star's record, and which corner of its
    // square, bit 0 set on the right and bit 1 at the top.
    @location(0) slot: u32,
}

// A corner's offset from the star's centre, each axis -1 or 1.
fn corner_of(slot: u32) -> vec2<f32> {
    let bits = vec2<u32>(slot & 1u, (slot >> 1u) & 1u);
    return vec2<f32>(bits) * 2.0 - 1.0;
}

struct StarOut {
    @builtin(position) position: vec4<f32>,
    // This point's offset from the star's centre, in physical pixels.
    @location(0) offset: vec2<f32>,
    @location(1) sigma: f32,
    @location(2) opacity: f32,
    @location(3) colour: vec3<f32>,
}

@vertex
fn vertex(in: Vertex) -> StarOut {
    let star = stars[in.slot >> 2u];
    let corner = corner_of(in.slot);
    // Turned by the camera's rotation only: w = 0 drops its position.
    let towards = (view.view_from_world * vec4<f32>(star.direction_sigma.xyz, 0.0)).xyz;
    // At infinity: depth 0, the far end of the reversed depth range. A
    // direction behind the camera gets w < 0 and is clipped away.
    var clip = view.clip_from_view * vec4<f32>(towards, 0.0);
    let sigma = star.direction_sigma.w * sky.px_scale;
    let reach = sky.reach_sigmas * sigma + sky.reach_extra_px;
    let offset = corner * reach;
    clip.x += offset.x * 2.0 / view.viewport.z * clip.w;
    clip.y -= offset.y * 2.0 / view.viewport.w * clip.w;
    var out: StarOut;
    out.position = clip;
    out.offset = offset;
    out.sigma = sigma;
    out.opacity = star.colour_opacity.w * sky.factor;
    out.colour = star.colour_opacity.rgb;
    return out;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn outside_map(p: vec2<f32>) -> bool {
    return p.x < sky.clip.x || p.y < sky.clip.y || p.x >= sky.clip.z || p.y >= sky.clip.w;
}

@fragment
fn fragment(in: StarOut) -> @location(0) vec4<f32> {
    if outside_map(in.position.xy) {
        discard;
    }
    let r2 = dot(in.offset, in.offset);
    let a = in.opacity * exp(-r2 / (2.0 * in.sigma * in.sigma));
    let ground = sky.ground.rgb;
    let over = ground + a * (in.colour - ground);
    return vec4<f32>(to_linear(over) - to_linear(ground), 0.0);
}
