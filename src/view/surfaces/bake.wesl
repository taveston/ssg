// Bakes one face of a body's surface into the cubemap strip (SURF-30):
// for each texel, the direction on the sphere it stands for, then the
// look's layers at that direction. Row 0 holds the albedo (linear RGB)
// and the height in units of the radius; row 1, for a body that glows,
// the emissive light (linear RGB). Nothing here depends on the light,
// the clock or the camera: those are the globe pass's (`globe.wesl`).
//
// The layers are ported from mock-ups 29 and 30. Their constants (noise
// frequencies, crater scales and depths, palette colours, plume shapes)
// shape each look and are tuning, not tunables. Mirrors `BakeParams` in
// `bake.rs` and the cube layout in `cube.rs`.

struct BakeParams {
    // Where the surface noise is sampled: the body's seed as an offset.
    seed_offset: vec4<f32>,
    // Whether the finer crater scales are drawn: the second to fifth
    // scales, the coarsest always being drawn (SURF-25, SURF-34).
    crater_scales: vec4<f32>,
    // The hazy look's palette, display space (SURF-17).
    haze_colour: vec4<f32>,
    // The sub-Neptune's haze colour and, in w, how far toward it the
    // envelope colour is mixed (SURF-12).
    haze_mix: vec4<f32>,
    look: u32,
    face_px: u32,
    rows: u32,
    face: u32,
    octaves: f32,
    crater_density: f32,
    ray_density: f32,
    ray_full_age: f32,
    ray_max_age: f32,
    // The equilibrium temperature the surface temperature rule reads.
    teq: f32,
    // 1 when the surface temperature varies with the angle from the star
    // (a locked, airless planet), else 0 (SURF-21).
    mapped: u32,
    peak_factor: f32,
    night_floor: f32,
    melt_t: f32,
    glow_t: f32,
    heat_ramp: f32,
    // The cosine of the latitude a polar cap reaches down to; above 1
    // for no caps (SURF-18).
    cap_edge: f32,
    knowledge: f32,
}

@group(0) @binding(0) var<uniform> params: BakeParams;
@group(0) @binding(1) var strip: texture_storage_2d<rgba16float, write>;

const LOOK_AIRLESS: u32 = 0u;
const LOOK_COLD: u32 = 1u;
const LOOK_DENSE: u32 = 2u;
const LOOK_ICY: u32 = 3u;
const LOOK_LAVA: u32 = 4u;
const LOOK_ICE_GIANT: u32 = 5u;
const LOOK_GAS_GIANT: u32 = 6u;
const LOOK_VOLCANIC: u32 = 7u;
const LOOK_HAZY: u32 = 8u;
const LOOK_SUB_NEPTUNE: u32 = 9u;

// A rotation applied between noise octaves, so their grids never line up.
const ROT: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.00, 0.80, 0.60),
    vec3<f32>(-0.80, 0.36, -0.48),
    vec3<f32>(-0.60, -0.48, 0.64),
);
// The ray craters sit on a grid this many cells across the sphere.
const RAY_GRID: f32 = 2.2;
// How strongly hot rock glows at full strength, against the albedo
// (SURF-20).
const GLOW_STRENGTH: f32 = 0.15;

// Per-texel state the layers read: the octave count, the crater scale
// weights, the crater density, the molten share and the local surface
// temperature.
var<private> g_oct: f32;
var<private> w_c1: f32;
var<private> w_c2: f32;
var<private> w_c3: f32;
var<private> w_c4: f32;
var<private> w_c5: f32;
var<private> g_dens: f32;
var<private> g_melt: f32;
var<private> g_tl: f32;

fn sq(x: f32) -> f32 {
    return x * x;
}

fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// Value noise: random values on a grid of whole-number points, blended
// smoothly in between.
fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash13(i);
    let b = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let g = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let h = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let k = hash13(i + vec3<f32>(1.0, 1.0, 1.0));
    return mix(
        mix(mix(a, b, f.x), mix(c, d, f.x), f.y),
        mix(mix(e, g, f.x), mix(h, k, f.x), f.y),
        f.z,
    );
}

// Layered noise. The octave count is fractional, and the mean stays near
// 0.5 whatever the count, so a body seen from far away has the same large
// forms as the same body surveyed (SURF-26).
fn fbm(p0: vec3<f32>, oct: f32) -> f32 {
    var p = p0;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 10; i++) {
        let w = clamp(oct - f32(i), 0.0, 1.0);
        if (w <= 0.0) {
            break;
        }
        s += a * w * vnoise(p);
        p = ROT * p * 2.02 + vec3<f32>(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    return s + a;
}

// Ridged noise: sharp crests where the noise crosses its middle.
fn ridged(p0: vec3<f32>, oct: f32) -> f32 {
    var p = p0;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 10; i++) {
        let w = clamp(oct - f32(i), 0.0, 1.0);
        if (w <= 0.0) {
            break;
        }
        var n = 1.0 - abs(2.0 * vnoise(p) - 1.0);
        n *= n;
        s += a * w * n;
        p = ROT * p * 2.1 + vec3<f32>(3.1, 1.4, 7.7);
        a *= 0.5;
    }
    return s;
}

// A point moved by noise, so forms drawn from it are warped.
fn warp(p: vec3<f32>, f: f32, amt: f32) -> vec3<f32> {
    return p + amt * vec3<f32>(
        fbm(p * f + vec3<f32>(3.0, 1.0, 7.0), 3.0),
        fbm(p * f + vec3<f32>(11.0, 5.0, 2.0), 3.0),
        fbm(p * f + vec3<f32>(7.0, 13.0, 9.0), 3.0),
    );
}

// Distances to the nearest and second-nearest feature point. Their
// difference is small along cell edges.
fn worley(p: vec3<f32>) -> vec2<f32> {
    let ci = floor(p);
    var f1 = 9.0;
    var f2 = 9.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let c = ci + vec3<f32>(f32(x), f32(y), f32(z));
                let d = c + hash33(c) - p;
                let l = dot(d, d);
                if (l < f1) {
                    f2 = f1;
                    f1 = l;
                } else if (l < f2) {
                    f2 = l;
                }
            }
        }
    }
    return sqrt(vec2<f32>(f1, f2));
}

// Craters on a jittered grid: a bowl, a raised rim and a little ejecta.
// Heights are in units of the planet radius.
fn crater_layer(p: vec3<f32>, scale: f32, dens: f32, depth: f32) -> f32 {
    let q = p * scale;
    let ci = floor(q);
    var h = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let c = ci + vec3<f32>(f32(x), f32(y), f32(z));
                if (hash13(c + vec3<f32>(11.3, 2.7, 5.1)) > dens) {
                    continue;
                }
                let cen = c + 0.25 + 0.5 * hash33(c + vec3<f32>(3.7, 8.1, 1.3));
                let rad = mix(0.14, 0.34, hash13(c + vec3<f32>(9.1, 4.2, 6.6)));
                let d = length(q - cen) / rad;
                if (d > 2.2) {
                    continue;
                }
                let bowl = select(0.0, -(1.0 - d * d), d < 1.0);
                let rim = exp(-sq((d - 1.0) * 3.2)) * 0.5;
                // Flat inside the rim, so there is no step at d = 1, and
                // faded out before the cut-off at 2.2.
                let ej = 0.1 * exp(-max(d - 1.0, 0.0) * 2.2) * (1.0 - smoothstep(1.5, 2.2, d));
                h += (bowl + rim + ej) * rad * depth;
            }
        }
    }
    return h / scale;
}

// ---- crater rays (SURF-22) ----
// Fresh impact craters with bright ray systems. The parent craters sit on
// a coarse jittered grid over the sphere, so a ray can run a long way
// across the disc. Each parent has its own age, drawn up to the ray
// ages' limit, and its rays fade as it ages.
struct RayCell {
    ok: bool,
    cdir: vec3<f32>,
    rc: f32,
    vis: f32,
    reach: f32,
    hc: f32,
}

fn ray_cell(c: vec3<f32>) -> RayCell {
    var cell: RayCell;
    cell.ok = false;
    let o = params.seed_offset.xyz * 3.7;
    if (hash13(c + o + vec3<f32>(21.7, 4.4, 8.8)) > params.ray_density) {
        return cell;
    }
    let cen = (c + 0.15 + 0.7 * hash33(c + o + vec3<f32>(5.1, 2.2, 9.3))) / RAY_GRID;
    let l = length(cen);
    if (abs(l - 1.0) > 0.22) {
        return cell;
    }
    cell.ok = true;
    cell.cdir = cen / l;
    cell.hc = hash13(c + o + vec3<f32>(1.9, 7.3, 3.3));
    cell.rc = mix(0.04, 0.1, cell.hc);
    let age = hash13(c + o + vec3<f32>(6.6, 1.3, 2.9)) * params.ray_max_age;
    cell.vis = 1.0 - smoothstep(params.ray_full_age, params.ray_max_age, age);
    cell.reach = mix(0.5, 0.95, hash13(c + o + vec3<f32>(8.8, 3.1, 5.7)));
    return cell;
}

// Height of the parent craters: a bowl, a rim and a little ejecta, like
// the grid craters.
fn ray_crater_h(pn: vec3<f32>) -> f32 {
    if (params.ray_density <= 0.0) {
        return 0.0;
    }
    let ci = floor(pn * RAY_GRID);
    var h = 0.0;
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            for (var z = -2; z <= 2; z++) {
                let cell = ray_cell(ci + vec3<f32>(f32(x), f32(y), f32(z)));
                if (!cell.ok) {
                    continue;
                }
                let d = acos(clamp(dot(pn, cell.cdir), -1.0, 1.0)) / cell.rc;
                if (d > 2.2) {
                    continue;
                }
                let bowl = select(0.0, -(1.0 - d * d), d < 1.0);
                let rim = exp(-sq((d - 1.0) * 3.2)) * 0.5;
                let ej = 0.1 * exp(-max(d - 1.0, 0.0) * 2.2) * (1.0 - smoothstep(1.5, 2.2, d));
                h += (bowl + rim + ej) * cell.rc * 0.3;
            }
        }
    }
    return h;
}

// Albedo brightening, 0 to 1: a bright blanket round the rim and streaks
// that thin out with distance. Albedo only, no relief.
fn ray_albedo(pn: vec3<f32>) -> f32 {
    if (params.ray_density <= 0.0) {
        return 0.0;
    }
    let ci = floor(pn * RAY_GRID);
    var a = 0.0;
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            for (var z = -2; z <= 2; z++) {
                let cell = ray_cell(ci + vec3<f32>(f32(x), f32(y), f32(z)));
                if (!cell.ok) {
                    continue;
                }
                let cs = dot(pn, cell.cdir);
                let ang = acos(clamp(cs, -1.0, 1.0));
                if (ang > cell.reach) {
                    continue;
                }
                let v = pn - cell.cdir * cs;
                let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(cell.cdir.y) < 0.9);
                let t1 = normalize(cross(cell.cdir, up));
                let t2 = cross(cell.cdir, t1);
                let az = atan2(dot(v, t2), dot(v, t1));
                let rn = vnoise(vec3<f32>(cos(az) * 3.5, sin(az) * 3.5, cell.hc * 40.0)) * 0.6
                    + vnoise(vec3<f32>(cos(az) * 9.0, sin(az) * 9.0, cell.hc * 40.0 + 5.0)) * 0.4;
                let streak = smoothstep(0.52, 0.78, rn);
                let fall = exp(-ang / (cell.reach * 0.5)) * (1.0 - smoothstep(cell.reach * 0.55, cell.reach, ang));
                let blanket = exp(-sq(max(ang - cell.rc, 0.0) / (cell.rc * 1.7)))
                    * (1.0 - smoothstep(cell.rc * 0.8, cell.rc * 1.0, ang) * 0.35);
                let s = (streak * fall * step(cell.rc * 1.1, ang) * 0.7 + blanket * 0.85) * cell.vis;
                a = max(a, s);
            }
        }
    }
    return a;
}

// SURF-20: the glow's colour at a surface temperature: dull red at the
// glow point, orange midway, pale yellow at the melt point plus the ramp.
// Mirrors `glow_colour` in `chooser.rs`.
fn glow_colour(t_k: f32) -> vec3<f32> {
    let t = clamp((t_k - params.glow_t) / (params.melt_t + params.heat_ramp - params.glow_t), 0.0, 1.0);
    let red = vec3<f32>(0.30, 0.03, 0.01);
    let orange = vec3<f32>(1.0, 0.34, 0.05);
    let yellow = vec3<f32>(1.0, 0.85, 0.35);
    if (t < 0.5) {
        return mix(red, orange, t * 2.0);
    }
    return mix(orange, yellow, (t - 0.5) * 2.0);
}

// SURF-20: how strongly the surface glows at a temperature, from nothing
// at the glow point to full at the melt point.
fn glow_strength(t_k: f32) -> f32 {
    return clamp((t_k - params.glow_t) / (params.melt_t - params.glow_t), 0.0, 1.0);
}

// Paterae: irregular dark pits, some with a glowing lava lake. x is the
// pit, y the glow.
fn paterae(p: vec3<f32>) -> vec2<f32> {
    let q = p * 5.5;
    let ci = floor(q);
    var pit = 0.0;
    var gl = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let c = ci + vec3<f32>(f32(x), f32(y), f32(z));
                if (hash13(c + vec3<f32>(41.0, 3.0, 9.0)) > 0.34) {
                    continue;
                }
                let cen = c + 0.2 + 0.6 * hash33(c + vec3<f32>(2.2, 6.1, 4.4));
                let rd = mix(0.14, 0.3, hash13(c + vec3<f32>(7.7, 1.1, 3.3)));
                let d = length(q - cen) / rd;
                let act = step(0.45, hash13(c + vec3<f32>(3.3, 9.9, 1.1)));
                let pt = 1.0 - smoothstep(0.75, 1.0, d);
                pit = max(pit, pt);
                gl = max(gl, act * (exp(-sq((d - 0.82) * 7.0)) * (1.0 - smoothstep(0.95, 1.2, d)) + pt * 0.3));
            }
        }
    }
    return vec2<f32>(pit, gl);
}

// The molten sea of a lava world (SURF-15): the crust is rafts of noise,
// molten where the plate noise is under a threshold that rises with the
// melt; `edge` is the glowing rim where the sea meets rock.
struct Sea {
    open: f32,
    edge: f32,
    live: f32,
}

fn sea(p: vec3<f32>) -> Sea {
    var out: Sea;
    let plate = fbm(warp(p * 3.2, 2.5, 0.35), 5.0);
    let thr = 0.22 + 0.55 * g_melt;
    out.live = step(0.02, g_melt);
    out.open = smoothstep(thr, thr - 0.05, plate) * out.live;
    out.edge = exp(-sq((plate - thr) * 16.0)) * out.live;
    return out;
}

// The cratered relief of rock: the grid craters at every scale that is
// drawn, with the maria flattening the coarser ones.
fn rock_craters(p: vec3<f32>, mk: f32) -> f32 {
    var h = 0.0;
    if (w_c1 > 0.001) { h += mk * w_c1 * crater_layer(p, 5.0, g_dens, 0.28); }
    if (w_c2 > 0.001) { h += mk * w_c2 * crater_layer(p, 13.0, g_dens * 1.1, 0.26); }
    if (w_c3 > 0.001) { h += mk * w_c3 * crater_layer(p, 34.0, g_dens * 1.2, 0.24); }
    if (w_c4 > 0.001) { h += w_c4 * crater_layer(p, 90.0, g_dens * 1.2, 0.22); }
    if (w_c5 > 0.001) { h += w_c5 * crater_layer(p, 240.0, g_dens * 1.2, 0.2); }
    return h;
}

// ---- height, in units of the radius ----
fn height(p: vec3<f32>, pn: vec3<f32>) -> f32 {
    var h = 0.0;
    let look = params.look;
    if (look == LOOK_AIRLESS || look == LOOK_LAVA) {
        let m = smoothstep(0.50, 0.60, fbm(p * 1.3 + vec3<f32>(4.0, 1.0, 2.0), 4.0));
        let mk = 1.0 - 0.65 * m;
        h = (fbm(p * 2.4, g_oct) - 0.5) * 0.035 * (1.0 - 0.75 * m);
        h += rock_craters(p, mk);
        if (look == LOOK_AIRLESS) {
            h += ray_crater_h(pn);
        }
        // The sea is flat: a molten surface keeps no craters (SURF-15).
        if (look == LOOK_LAVA) {
            h *= 1.0 - sea(p).open;
        }
    } else if (look == LOOK_COLD) {
        let w = warp(p * 2.0, 1.5, 0.5);
        h = (fbm(w * 1.4, g_oct) - 0.5) * 0.04 + ridged(w * 0.9 + vec3<f32>(2.0), min(g_oct, 6.0)) * 0.02;
        let cn = abs(fbm(p * 1.1 + vec3<f32>(8.0, 2.0, 5.0), 5.0) - 0.5);
        h -= 0.015 * smoothstep(0.035, 0.0, cn);
        // Eroded craters: shallower, and fewer at each scale.
        if (w_c1 > 0.001) { h += w_c1 * crater_layer(p, 6.0, g_dens * 0.5, 0.15); }
        if (w_c2 > 0.001) { h += w_c2 * crater_layer(p, 15.0, g_dens * 0.6, 0.13); }
        if (w_c3 > 0.001) { h += w_c3 * crater_layer(p, 38.0, g_dens * 0.7, 0.11); }
        if (w_c4 > 0.001) { h += w_c4 * crater_layer(p, 90.0, g_dens * 0.7, 0.1); }
        if (w_c5 > 0.001) { h += w_c5 * crater_layer(p, 240.0, g_dens * 0.7, 0.09); }
    } else if (look == LOOK_ICY) {
        // An icy moon: soft relief and craters a little shallower than on
        // rock, as ice relaxes.
        h = (fbm(p * 2.6, g_oct) - 0.5) * 0.02;
        if (w_c1 > 0.001) { h += w_c1 * crater_layer(p, 6.0, g_dens, 0.2); }
        if (w_c2 > 0.001) { h += w_c2 * crater_layer(p, 15.0, g_dens, 0.18); }
        if (w_c3 > 0.001) { h += w_c3 * crater_layer(p, 38.0, g_dens, 0.16); }
        if (w_c4 > 0.001) { h += w_c4 * crater_layer(p, 90.0, g_dens, 0.14); }
        if (w_c5 > 0.001) { h += w_c5 * crater_layer(p, 240.0, g_dens, 0.12); }
        h += ray_crater_h(pn);
    } else if (look == LOOK_VOLCANIC) {
        // Resurfaced by eruptions: no craters, broad low relief, a few
        // mountains and the pits of the paterae.
        h = (fbm(p * 3.0, g_oct) - 0.5) * 0.03 + ridged(warp(p * 2.0, 1.5, 0.4), min(g_oct, 5.0)) * 0.012;
        h -= paterae(p).x * 0.012 * smoothstep(0.15, 0.55, params.knowledge);
    }
    return h;
}

// A gas giant: bands sheared by latitude, a storm oval, a few small white
// ovals (SURF-10).
fn gas_colour(pn: vec3<f32>) -> vec3<f32> {
    let off = params.seed_offset.xyz;
    let lat = pn.y;
    // Differential rotation: shear the noise by latitude.
    let sh = 2.4 * sin(lat * 9.0 + 1.3) + 1.1 * sin(lat * 23.0 + 0.4);
    let cs = cos(sh);
    let sn = sin(sh);
    let q = vec3<f32>(cs * pn.x + sn * pn.z, pn.y, -sn * pn.x + cs * pn.z) + off;
    let band_n = vnoise(vec3<f32>(lat * 6.5, off.x, 0.0)) * 0.6 + vnoise(vec3<f32>(lat * 15.0, off.y, 3.0)) * 0.4;
    let turb = fbm(vec3<f32>(q.x * 5.0, q.y * 26.0, q.z * 5.0), g_oct);
    let fine = fbm(vec3<f32>(q.x * 16.0, q.y * 80.0, q.z * 16.0) + 7.0, g_oct);
    let t = band_n + (turb - 0.5) * 0.5 + (fine - 0.5) * 0.16;
    let dk = vec3<f32>(0.40, 0.25, 0.17);
    let md = vec3<f32>(0.72, 0.54, 0.36);
    let lt = vec3<f32>(0.94, 0.87, 0.74);
    var c = select(mix(md, lt, smoothstep(0.5, 0.78, t)), mix(dk, md, smoothstep(0.18, 0.5, t)), t < 0.5);
    let lon = atan2(pn.x, pn.z);
    let la = asin(clamp(pn.y, -1.0, 1.0));
    var dl = lon - 0.9 - off.x;
    dl = atan2(sin(dl), cos(dl));
    let d = vec2<f32>(dl / 0.36, (la + 0.32) / 0.17);
    let e = dot(d, d);
    let spot = smoothstep(1.0, 0.25, e);
    let ang = 2.4 * (1.0 - clamp(e, 0.0, 1.0));
    let dr = vec2<f32>(cos(ang) * d.x - sin(ang) * d.y, sin(ang) * d.x + cos(ang) * d.y);
    let sw = fbm(vec3<f32>(dr * 3.0, 1.0) + off, 5.0);
    c = mix(c, vec3<f32>(0.74, 0.36, 0.24) * (0.8 + 0.4 * sw), spot * 0.9);
    c += vec3<f32>(0.25, 0.2, 0.15) * smoothstep(0.7, 1.0, e) * smoothstep(1.4, 1.0, e) * 0.5;
    for (var i = 0; i < 5; i++) {
        let hh = hash33(vec3<f32>(f32(i) * 3.1 + off.x, 7.7, off.y));
        let l0 = hh.x * 6.28 - 3.14;
        let a0 = (hh.y - 0.5) * 1.4;
        let dl2 = atan2(sin(lon - l0), cos(lon - l0));
        let d2 = vec2<f32>(dl2 / 0.09, (la - a0) / 0.05);
        c = mix(c, vec3<f32>(0.95, 0.92, 0.86), smoothstep(1.0, 0.3, dot(d2, d2)) * 0.7 * step(0.35, hh.z));
    }
    return c;
}

// An ice giant: soft blue bands, bright streaks in one latitude band
// (SURF-11).
fn ice_giant_colour(pn: vec3<f32>) -> vec3<f32> {
    let off = params.seed_offset.xyz;
    let lat = pn.y;
    let q = vec3<f32>(pn.x * 2.0, pn.y * 7.0, pn.z * 2.0) + off;
    let t = vnoise(vec3<f32>(lat * 5.0, off.z, 1.0)) * 0.5 + 0.5 * fbm(q, min(g_oct, 4.0));
    let c = mix(vec3<f32>(0.30, 0.55, 0.72), vec3<f32>(0.52, 0.78, 0.86), smoothstep(0.25, 0.7, t));
    let st = smoothstep(0.62, 0.8, fbm(vec3<f32>(pn.x * 3.0, pn.y * 18.0, pn.z * 3.0) + off * 2.0, g_oct))
        * smoothstep(0.9, 0.3, abs(lat - 0.35));
    return mix(c, vec3<f32>(0.9, 0.97, 1.0), st * 0.5);
}

// An opaque cloud deck with bands and swirls, in a palette from its dark
// to its light tone (SURF-16, SURF-17).
fn deck_colour(pn: vec3<f32>, dark: vec3<f32>, light: vec3<f32>, streak: vec3<f32>) -> vec3<f32> {
    let off = params.seed_offset.xyz;
    let w = pn * 1.8 + off + 0.55 * vec3<f32>(
        fbm(pn * 2.0 + off, 4.0),
        fbm(pn * 2.0 + off + 5.0, 4.0),
        fbm(pn * 2.0 + off + 9.0, 4.0),
    );
    let t = fbm(vec3<f32>(w.x * 2.2, w.y * 6.5, w.z * 2.2), g_oct) * 0.75 + 0.12 * sin(w.y * 6.0 + 2.5 * fbm(w * 3.0, 4.0));
    let c = mix(dark, light, smoothstep(0.2, 0.65, t + 0.1));
    let st = smoothstep(0.58, 0.78, fbm(vec3<f32>(w.x * 3.0, w.y * 14.0, w.z * 3.0) + 3.0, g_oct));
    return mix(c, streak, st * 0.35);
}

struct Shade {
    alb: vec3<f32>,
    emi: vec3<f32>,
}

// The albedo (display space, converted to linear at the end) and the
// emissive light (linear) at a point: `p` is the direction plus the seed
// offset, `pn` the direction itself, `h` the height there.
fn shade(p: vec3<f32>, pn: vec3<f32>, h: f32) -> Shade {
    var out: Shade;
    out.emi = vec3<f32>(0.0);
    var alb = vec3<f32>(0.5);
    let look = params.look;
    if (look == LOOK_AIRLESS) {
        // Grey cratered rock with darker plains (SURF-19).
        let m = smoothstep(0.50, 0.60, fbm(p * 1.3 + vec3<f32>(4.0, 1.0, 2.0), 4.0));
        alb = mix(vec3<f32>(0.55, 0.53, 0.50), vec3<f32>(0.20, 0.20, 0.22), m * 0.85);
        alb *= 0.75 + 0.5 * fbm(p * 7.0, g_oct);
        alb *= 1.0 + clamp(h * 14.0, -0.45, 0.5);
        alb = mix(alb, vec3<f32>(0.9, 0.89, 0.86), ray_albedo(pn) * 0.85);
    } else if (look == LOOK_COLD) {
        // Warped ridged relief, dark patches, a polar cap if the water is
        // ice (SURF-18).
        let t = clamp(h * 16.0 + 0.5, 0.0, 1.0);
        let lowc = vec3<f32>(0.30, 0.19, 0.14);
        let midc = vec3<f32>(0.60, 0.39, 0.27);
        let hic = vec3<f32>(0.80, 0.66, 0.52);
        var c = select(mix(midc, hic, (t - 0.5) * 2.0), mix(lowc, midc, t * 2.0), t < 0.5);
        let dk = smoothstep(0.46, 0.62, fbm(p * 1.7 + vec3<f32>(5.0, 3.0, 1.0), 4.0));
        c = mix(c, vec3<f32>(0.22, 0.21, 0.22), dk * 0.65);
        c *= 0.86 + 0.28 * fbm(p * 13.0, g_oct);
        // The cap's edge is the cap latitude's cosine, moved by noise.
        let edge = params.cap_edge + 0.05 * (fbm(p * 5.0 + vec3<f32>(2.0), 4.0) - 0.5);
        let cap = smoothstep(edge, edge + 0.02, abs(pn.y));
        alb = mix(c, vec3<f32>(0.90, 0.93, 0.96), cap * 0.92);
    } else if (look == LOOK_DENSE) {
        alb = deck_colour(pn, vec3<f32>(0.60, 0.46, 0.26), vec3<f32>(0.95, 0.86, 0.60), vec3<f32>(0.50, 0.38, 0.24));
    } else if (look == LOOK_HAZY) {
        let haze = params.haze_colour.rgb;
        alb = deck_colour(pn, haze * 0.62, haze * 1.12, haze * 0.5);
    } else if (look == LOOK_ICY) {
        // Bright ice with patches of dark, dusty terrain, fresh-ice crater
        // rims, a few faint lineae (SURF-19).
        let dkp = smoothstep(0.48, 0.64, fbm(p * 1.7 + vec3<f32>(2.0, 6.0, 3.0), 4.0));
        var c = mix(vec3<f32>(0.80, 0.82, 0.84), vec3<f32>(0.40, 0.36, 0.33), dkp * 0.75);
        c *= 0.84 + 0.32 * fbm(p * 8.0, g_oct);
        c = mix(c, vec3<f32>(0.93, 0.95, 0.97), clamp(h * 45.0, 0.0, 0.65));
        c *= 1.0 - clamp(-h * 10.0, 0.0, 0.25);
        c = mix(c, vec3<f32>(0.97, 0.98, 1.0), ray_albedo(pn) * 0.7);
        let cn = abs(fbm(warp(p * 1.5, 1.2, 0.6) + vec3<f32>(4.0), 4.0) - 0.5);
        c = mix(c, vec3<f32>(0.55, 0.40, 0.33), smoothstep(0.012, 0.0, cn) * 0.22);
        alb = c;
    } else if (look == LOOK_LAVA) {
        // A rocky body heated by its star: baked rock, and a molten sea
        // where it melts, with crust rafts and a glowing edge (SURF-15).
        let m = smoothstep(0.50, 0.60, fbm(p * 1.3 + vec3<f32>(4.0, 1.0, 2.0), 4.0));
        var rock = mix(vec3<f32>(0.42, 0.34, 0.29), vec3<f32>(0.17, 0.14, 0.13), m * 0.85);
        rock *= 0.75 + 0.5 * fbm(p * 7.0, g_oct);
        rock *= 1.0 + clamp(h * 14.0, -0.45, 0.5);
        let s = sea(p);
        let open = s.open;
        let edge = s.edge;
        let live = s.live;
        let turb = 0.7 + 0.5 * fbm(p * 14.0 + params.seed_offset.xyz, min(g_oct, 7.0));
        alb = mix(mix(rock, vec3<f32>(0.10, 0.085, 0.08), g_melt * 0.8), vec3<f32>(0.04, 0.03, 0.03), open);
        let sea_t = max(g_tl, params.melt_t);
        // A cooling skin: darker streaks on the sea.
        let cool = smoothstep(0.45, 0.7, fbm(warp(p * 9.0, 2.0, 0.4), min(g_oct, 7.0)));
        out.emi = glow_colour(sea_t * (0.85 + 0.3 * turb)) * open * (0.55 + 0.9 * turb) * mix(1.0, 0.22, cool * 0.85);
        out.emi += glow_colour(sea_t) * edge * (1.0 - open) * 0.7 * live;
        // The solid rock glows by its own temperature (SURF-20).
        out.emi += glow_colour(g_tl) * glow_strength(g_tl) * GLOW_STRENGTH * (1.0 - open) * (0.5 + 0.8 * fbm(p * 9.0, 4.0));
    } else if (look == LOOK_VOLCANIC) {
        // Sulphur and frost patchwork, dark paterae, some with a glowing
        // lake (SURF-13).
        let t = fbm(p * 3.0 + vec3<f32>(1.0, 5.0, 2.0), min(g_oct, 8.0));
        let t2 = fbm(warp(p * 5.0, 2.0, 0.4), 5.0);
        var c = mix(vec3<f32>(0.82, 0.76, 0.38), vec3<f32>(0.78, 0.46, 0.20), smoothstep(0.4, 0.7, t2));
        c = mix(c, vec3<f32>(0.92, 0.92, 0.88), smoothstep(0.55, 0.72, t) * 0.8);
        c = mix(c, vec3<f32>(0.55, 0.22, 0.14), smoothstep(0.62, 0.8, fbm(p * 4.0 + vec3<f32>(9.0), 4.0)) * 0.7);
        c *= (0.8 + 0.4 * fbm(p * 10.0, g_oct)) * (0.85 + 0.3 * fbm(p * 45.0, g_oct));
        let pt = paterae(p);
        let vis = smoothstep(0.15, 0.55, params.knowledge);
        c = mix(c, vec3<f32>(0.10, 0.085, 0.085), pt.x * 0.9 * vis);
        alb = c;
        out.emi = vec3<f32>(1.0, 0.42, 0.1) * pt.y * vis * 1.5;
    } else if (look == LOOK_ICE_GIANT) {
        alb = ice_giant_colour(pn);
    } else if (look == LOOK_SUB_NEPTUNE) {
        // The ice giant look mixed toward the haze colour (SURF-12).
        alb = mix(ice_giant_colour(pn), params.haze_mix.rgb, params.haze_mix.w);
    } else {
        alb = gas_colour(pn);
    }
    // SURF-20: every rocky look but lava (which has its own above) glows
    // where its surface is hot.
    if (look == LOOK_AIRLESS || look == LOOK_COLD || look == LOOK_ICY || look == LOOK_VOLCANIC || look == LOOK_HAZY) {
        out.emi += glow_colour(g_tl) * glow_strength(g_tl) * GLOW_STRENGTH * (0.5 + 0.8 * fbm(p * 9.0, 4.0));
    }
    out.alb = pow(max(alb, vec3<f32>(0.0)), vec3<f32>(2.2));
    return out;
}

// SURF-21: the surface temperature at a direction: on a locked, airless
// body the dayside peak falling with the fourth root of the cosine of
// the angle from the substellar point (+Z), the night floor beyond 90°;
// elsewhere the equilibrium temperature everywhere.
fn surface_temperature(pn: vec3<f32>) -> f32 {
    if (params.mapped == 0u) {
        return params.teq;
    }
    let lit = params.peak_factor * params.teq * pow(max(pn.z, 0.0), 0.25);
    return max(lit, params.night_floor);
}

// The face coordinate at a texel's centre, the one-texel border included
// (`texel_coordinate` in `cube.rs`).
fn texel_coordinate(i: u32) -> f32 {
    return -1.0 + (f32(i) + 0.5 - 1.0) * 2.0 / (f32(params.face_px) - 2.0);
}

// The direction of a point on a face (`face_direction` in `cube.rs`).
fn face_direction(face: u32, s: f32, t: f32) -> vec3<f32> {
    var d: vec3<f32>;
    switch face {
        case 0u: { d = vec3<f32>(1.0, t, -s); }
        case 1u: { d = vec3<f32>(-1.0, t, s); }
        case 2u: { d = vec3<f32>(s, 1.0, -t); }
        case 3u: { d = vec3<f32>(s, -1.0, t); }
        case 4u: { d = vec3<f32>(s, t, 1.0); }
        default: { d = vec3<f32>(-s, t, -1.0); }
    }
    return normalize(d);
}

@compute @workgroup_size(8, 8, 1)
fn bake(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = params.face_px;
    if (id.x >= w || id.y >= w * params.rows) {
        return;
    }
    let row = id.y / w;
    let y = id.y - row * w;
    let pn = face_direction(params.face, texel_coordinate(id.x), texel_coordinate(y));
    let p = pn + params.seed_offset.xyz;

    g_oct = min(params.octaves, 10.0);
    w_c1 = 1.0;
    w_c2 = params.crater_scales.x;
    w_c3 = params.crater_scales.y;
    w_c4 = params.crater_scales.z;
    w_c5 = params.crater_scales.w;
    g_dens = params.crater_density;
    g_tl = surface_temperature(pn);
    g_melt = 0.0;
    if (params.look == LOOK_LAVA) {
        g_melt = smoothstep(params.melt_t, params.melt_t + params.heat_ramp, g_tl);
    }

    var texel: vec4<f32>;
    if (row == 0u) {
        let h = height(p, pn);
        let s = shade(p, pn, h);
        texel = vec4<f32>(s.alb, h);
    } else {
        let s = shade(p, pn, 0.0);
        texel = vec4<f32>(s.emi, 0.0);
    }
    textureStore(strip, vec2<i32>(i32(params.face * w + id.x), i32(id.y)), texel);
}
