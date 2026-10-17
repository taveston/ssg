// Draws the band's glow (SKY-13 to SKY-15, SKY-18): one triangle over
// the viewport. Each pixel finds its direction through the camera (its
// rotation and field of view only), then the glow there: a normal curve
// across the band, the core weight along it, lanes and a mottle from
// value noise, and a dither of at most a level or so before the display
// rounds it. The light is added in the display's sRGB levels over the
// ground. Mirrors `glow.rs` and `band.rs`, and `GlowUniform` in `draw.rs`.

import bevy_pbr::render::mesh_view_bindings::view;

struct SkyGlow {
    ground: vec4<f32>,
    tone: vec4<f32>,
    clip: vec4<f32>,
    normal: vec4<f32>,
    core: vec4<f32>,
    along: vec4<f32>,
    lane_offset: vec4<f32>,
    mottle_offset: vec4<f32>,
    alpha: f32,
    sigma: f32,
    cutoff: f32,
    core_sigma: f32,
    core_floor: f32,
    lane_depth: f32,
    mottle: f32,
    lane_scale: f32,
    lane_from: f32,
    lane_to: f32,
    lane_sigma: f32,
    lane_cutoff: f32,
    mottle_scale: f32,
    dither_levels: f32,
    // What the glow's opacity is multiplied by in the layer shown: 1 in
    // the system view, `sector_sky_factor` in the sector view.
    factor: f32,
    lane_octaves: u32,
    mottle_octaves: u32,
    dither_seed: u32,
}

@group(3) @binding(0)
var<uniform> glow: SkyGlow;

const OCTAVE_STEP: f32 = 2.03;
const LEVELS: f32 = 255.0;

@vertex
fn vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    // At infinity: depth 0, the far end of the reversed depth range.
    return vec4<f32>(position.xy, 0.0, 1.0);
}

// A whole-number grid point hashed to [0, 1), as `lattice_hash`.
fn lattice_hash(i: i32, j: i32, k: i32) -> f32 {
    var h = bitcast<u32>(i) * 374761393u + bitcast<u32>(j) * 668265263u
        + bitcast<u32>(k) * 2147483647u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    h = h ^ (h >> 16u);
    return f32(h >> 8u) / 16777216.0;
}

fn value_noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = p - cell;
    let s = f * f * (3.0 - 2.0 * f);
    let c = vec3<i32>(cell);
    let a = mix(
        mix(lattice_hash(c.x, c.y, c.z), lattice_hash(c.x + 1, c.y, c.z), s.x),
        mix(lattice_hash(c.x, c.y + 1, c.z), lattice_hash(c.x + 1, c.y + 1, c.z), s.x),
        s.y,
    );
    let b = mix(
        mix(lattice_hash(c.x, c.y, c.z + 1), lattice_hash(c.x + 1, c.y, c.z + 1), s.x),
        mix(lattice_hash(c.x, c.y + 1, c.z + 1), lattice_hash(c.x + 1, c.y + 1, c.z + 1), s.x),
        s.y,
    );
    return mix(a, b, s.z);
}

fn fractal_noise(p: vec3<f32>, octaves: u32) -> f32 {
    var sum = 0.0;
    var weight = 0.5;
    var total = 0.0;
    var scale = 1.0;
    for (var i = 0u; i < octaves; i++) {
        sum += weight * value_noise(p * scale);
        total += weight;
        weight *= 0.5;
        scale *= OCTAVE_STEP;
    }
    return sum / total;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// This pixel's direction in the sky frame (east, north, up).
fn sky_direction(pixel: vec2<f32>) -> vec3<f32> {
    let uv = (pixel - view.viewport.xy) / view.viewport.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let near = view.view_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    let world = normalize((view.world_from_view * vec4<f32>(near.xyz / near.w, 0.0)).xyz);
    return vec3<f32>(world.x, -world.z, world.y);
}

// The glow's opacity at a direction, as `glow_opacity`.
fn glow_opacity(d: vec3<f32>) -> f32 {
    let latitude = asin(clamp(dot(d, glow.normal.xyz), -1.0, 1.0));
    if abs(latitude) > glow.cutoff {
        return 0.0;
    }
    let longitude = atan2(dot(d, glow.along.xyz), dot(d, glow.core.xyz));
    let across = exp(-latitude * latitude / (2.0 * glow.sigma * glow.sigma));
    let along = glow.core_floor + (1.0 - glow.core_floor)
        * exp(-longitude * longitude / (2.0 * glow.core_sigma * glow.core_sigma));
    var lane = 0.0;
    if abs(latitude) <= glow.lane_cutoff {
        let lanes = fractal_noise(d * glow.lane_scale + glow.lane_offset.xyz, glow.lane_octaves);
        let lane_across = exp(-latitude * latitude / (2.0 * glow.lane_sigma * glow.lane_sigma));
        lane = smoothstep(glow.lane_from, glow.lane_to, lanes) * lane_across;
    }
    let mottle = fractal_noise(d * glow.mottle_scale + glow.mottle_offset.xyz, glow.mottle_octaves);
    let factor = (1.0 - glow.lane_depth * lane) * (1.0 + glow.mottle * (2.0 * mottle - 1.0));
    return glow.alpha * glow.factor * across * along * factor;
}

// A triangular dither between -levels and levels, as `dither`.
fn dither(pixel: vec2<u32>) -> f32 {
    let x = bitcast<i32>(pixel.x);
    let y = bitcast<i32>(pixel.y);
    let a = lattice_hash(x, y, bitcast<i32>(glow.dither_seed));
    let b = lattice_hash(y ^ 0x5bd1, x, bitcast<i32>(glow.dither_seed + 1u));
    return (a + b - 1.0) * glow.dither_levels;
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = position.xy;
    if p.x < glow.clip.x || p.y < glow.clip.y || p.x >= glow.clip.z || p.y >= glow.clip.w {
        discard;
    }
    let a = glow_opacity(sky_direction(p));
    if a <= 0.0 {
        discard;
    }
    let ground = glow.ground.rgb;
    let lift = a * (glow.tone.rgb - ground);
    // The dither fades in over the glow's first level, so the plain
    // ground round the band gets none.
    let strength = min(1.0, max(lift.r, max(lift.g, lift.b)) * LEVELS);
    let noise = dither(vec2<u32>(p)) * strength / LEVELS;
    let over = clamp(ground + lift + vec3<f32>(noise), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(to_linear(over) - to_linear(ground), 0.0);
}
