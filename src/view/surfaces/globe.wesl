// Draws a globe from its bake each frame (SURF-40): the per-frame pass
// only. Each pixel of the disc is a direction on the sphere; the bake
// gives its albedo, height and emissive light, and this pass adds the
// star's light with a sharp terminator (SURF-23), the normal from the
// height, the limb glow and rim of an atmosphere (SURF-24), the plumes
// of a volcanic world (SURF-13), an anti-aliased edge (SURF-42) and, for
// an unsurveyed body, the telescope frame (SURF-25). No surface noise is
// evaluated here. Mirrors `GlobeUniform` in `shading.rs` and the cube
// layout in `cube.rs`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct Globe {
    // The body frame to the view frame, as three columns.
    view_from_body_x: vec4<f32>,
    view_from_body_y: vec4<f32>,
    view_from_body_z: vec4<f32>,
    // The direction toward the star, in the view frame.
    light: vec4<f32>,
    // The star's light, linear RGB times its strength.
    light_colour: vec4<f32>,
    // The atmosphere's colour, linear RGB; w is the halo's strength, 0
    // for a look with none.
    halo_colour: vec4<f32>,
    // The colour behind the globe, display space.
    ground: vec4<f32>,
    // The disc's radius as a share of half the node's width.
    disc_fraction: f32,
    wrap: f32,
    ambient: f32,
    // The halo falls off over this much of the radius.
    halo_width: f32,
    terminator_width: f32,
    telescope: u32,
    telescope_block: f32,
    desaturate: f32,
    // The grain, as a share of full white.
    grain: f32,
    grain_seed: u32,
    face_px: u32,
    rows: u32,
    plumes: u32,
    // The node's width in logical pixels.
    node_px: f32,
    // Past this radius, in disc units, the node is transparent.
    opaque_radius: f32,
    // SURF-34: the face drawn from the close-in strip, 6 for none, and
    // that strip's face size.
    close_face: u32,
    close_px: u32,
}

@group(1) @binding(0) var<uniform> globe: Globe;
@group(1) @binding(1) var strip: texture_2d<f32>;
@group(1) @binding(2) var strip_sampler: sampler;
// SURF-34: one face baked again finer, a strip one face wide.
@group(1) @binding(3) var close_strip: texture_2d<f32>;
@group(1) @binding(4) var close_sampler: sampler;

const BUMP: f32 = 1.0;
// The faint line banding of the telescope frame, in display levels of
// 255, and its blue cast.
const BAND_LEVELS: f32 = 2.0;
const BLUE_CAST_LEVELS: f32 = 4.0;

fn view_from_body() -> mat3x3<f32> {
    return mat3x3<f32>(globe.view_from_body_x.xyz, globe.view_from_body_y.xyz, globe.view_from_body_z.xyz);
}

struct FacePoint {
    face: u32,
    s: f32,
    t: f32,
}

// The face a direction lands on and where on it (`face_point` in
// `cube.rs`): the face of its largest axis.
fn face_point(d: vec3<f32>) -> FacePoint {
    let a = abs(d);
    var face: u32;
    var s: f32;
    var t: f32;
    if (a.x >= a.y && a.x >= a.z) {
        let m = max(a.x, 1e-9);
        if (d.x > 0.0) { face = 0u; s = -d.z / m; t = d.y / m; }
        else { face = 1u; s = d.z / m; t = d.y / m; }
    } else if (a.y >= a.z) {
        let m = max(a.y, 1e-9);
        if (d.y > 0.0) { face = 2u; s = d.x / m; t = -d.z / m; }
        else { face = 3u; s = d.x / m; t = d.z / m; }
    } else {
        let m = max(a.z, 1e-9);
        if (d.z > 0.0) { face = 4u; s = d.x / m; t = d.y / m; }
        else { face = 5u; s = -d.x / m; t = d.y / m; }
    }
    return FacePoint(face, s, t);
}

// Where a point of a face is sampled in a strip `columns` faces wide
// whose faces are `face_px` square, the face being in column `column`:
// the face's coordinates mapped over the face less its border
// (`strip_texel` in `cube.rs`).
fn strip_uv(p: FacePoint, row: u32, column: u32, columns: f32, face_px: u32) -> vec2<f32> {
    let w = f32(face_px);
    let inner = w - 2.0;
    let x = f32(column) * w + 1.0 + (p.s + 1.0) * inner / 2.0;
    let y = f32(row) * w + 1.0 + (p.t + 1.0) * inner / 2.0;
    return vec2<f32>(x / (columns * w), y / (f32(globe.rows) * w));
}

// The bake at a direction: from the close-in strip on its face (SURF-34),
// from the cube's strip elsewhere.
fn sample_strip(d: vec3<f32>, row: u32) -> vec4<f32> {
    let p = face_point(d);
    if (p.face == globe.close_face) {
        let uv = strip_uv(p, row, 0u, 1.0, globe.close_px);
        return textureSampleLevel(close_strip, close_sampler, uv, 0.0);
    }
    let uv = strip_uv(p, row, p.face, 6.0, globe.face_px);
    return textureSampleLevel(strip, strip_sampler, uv, 0.0);
}

// The face size the bake at a direction was baked at.
fn texel_px(d: vec3<f32>) -> u32 {
    return select(globe.face_px, globe.close_px, face_point(d).face == globe.close_face);
}

// The globe's light at a point `p` of the node, in disc units (the disc
// is the unit circle), with `aa` the width of a pixel in those units:
// linear light, over nothing.
fn globe_light(p: vec2<f32>, aa: f32) -> vec3<f32> {
    let r = length(p);
    let l = globe.light.xyz;
    var col = vec3<f32>(0.0);
    // SURF-42: the disc's edge covers partial pixels by their coverage.
    let cover = clamp((1.0 - r) / aa + 0.5, 0.0, 1.0);
    let halo = globe.halo_colour.w;
    if (halo > 0.0) {
        // SURF-24: the limb glow outside the disc, on the star's side.
        let limb = vec3<f32>(p / max(r, 1e-4), 0.0);
        let g = exp(-max(r - 1.0, 0.0) / (globe.halo_width * 0.45));
        let lf = clamp(dot(limb, l) * 0.9 + 0.35, 0.0, 1.0);
        col += globe.halo_colour.rgb * g * lf * halo * 0.9 * (1.0 - cover);
    }
    if (globe.plumes == 1u) {
        // SURF-13: two eruption plumes over the limb, a thin stem and an
        // umbrella, lit only where the limb is lit.
        let ang = atan2(p.y, p.x);
        let hgt = r - 1.0;
        for (var i = 0; i < 2; i++) {
            let a0 = select(3.5, 1.15, i == 0);
            let da = atan2(sin(ang - a0), cos(ang - a0)) * r;
            let w = 0.012 + 0.09 * hgt;
            let fan = exp(-(da / w) * (da / w)) * exp(-hgt / 0.1) * smoothstep(0.0, 0.008, hgt)
                * (1.0 - smoothstep(0.18, 0.28, hgt));
            let lit = clamp(dot(vec2<f32>(cos(a0), sin(a0)), l.xy) * 0.9 + 0.55, 0.0, 1.0);
            col += vec3<f32>(0.65, 0.75, 1.0) * fan * 0.9 * lit * (1.0 - cover);
        }
    }
    if (cover > 0.0) {
        let rr = min(r, 0.99999);
        let n = vec3<f32>(p / max(r, 1e-4) * rr, sqrt(1.0 - rr * rr));
        let m = view_from_body();
        let d = transpose(m) * n;
        let base = sample_strip(d, 0u);
        let alb = base.rgb;
        // The shading normal from the height: its gradient along two
        // tangents, a pixel or a texel apart, whichever is larger.
        let eps = max(0.5 * aa, 2.0 / f32(texel_px(d)));
        let ax = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(d.y) < 0.95);
        let t1 = normalize(cross(ax, d));
        let t2 = cross(d, t1);
        let h1 = sample_strip(normalize(d + t1 * eps), 0u).a;
        let h2 = sample_strip(normalize(d + t2 * eps), 0u).a;
        let g = vec2<f32>(h1 - base.a, h2 - base.a) / eps;
        let nb = m * normalize(d - BUMP * (g.x * t1 + g.y * t2));
        // SURF-23: a sharp terminator, with a wrap on looks with an
        // atmosphere, and only the ambient light on the night side.
        let ndl = dot(n, l);
        let wrap = globe.wrap;
        let diff = clamp((dot(nb, l) + wrap) / (1.0 + wrap), 0.0, 1.0)
            * smoothstep(-0.005, globe.terminator_width, ndl + wrap * 0.5);
        var emi = vec3<f32>(0.0);
        if (globe.rows > 1u) {
            emi = sample_strip(d, 1u).rgb;
        }
        var surf = alb * diff * globe.light_colour.rgb + alb * globe.ambient + emi;
        if (halo > 0.0) {
            // SURF-24: the rim inside the limb, on the star's side.
            let rim = pow(1.0 - n.z, 3.5);
            let lf = clamp(ndl * 1.2 + 0.1, 0.0, 1.0);
            surf += globe.halo_colour.rgb * rim * lf * halo * 0.55;
        }
        col += surf * cover;
    }
    return col;
}

// The globe at a point, as a display-space colour over the ground.
fn globe_display(p: vec2<f32>, aa: f32) -> vec3<f32> {
    var col = globe_light(p, aa);
    col = 1.0 - exp(-col * 1.25);
    col = pow(col, vec3<f32>(1.0 / 2.2));
    let ground = globe.ground.rgb;
    return ground + col * (1.0 - ground);
}

// The point of the node a uv stands for, in disc units: y up.
fn disc_point(uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>((uv.x - 0.5) * 2.0, (0.5 - uv.y) * 2.0) / globe.disc_fraction;
}

fn hash(p: vec2<u32>, seed: u32) -> f32 {
    var h = p.x * 374761393u + p.y * 668265263u + seed * 2246822519u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    h = h ^ (h >> 16u);
    return f32(h >> 8u) / 16777216.0;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = disc_point(in.uv);
    let r = length(p);
    let aa = max(fwidth(r), 1e-5);
    var display: vec3<f32>;
    if (globe.telescope == 0u) {
        display = globe_display(p, aa);
    } else {
        // SURF-25: the telescope frame: the globe at a block's resolution,
        // blended back up between the blocks' centres, desaturated, with
        // grain and faint banding.
        let block = globe.telescope_block / globe.node_px;
        let cell = in.uv / block - 0.5;
        let i0 = floor(cell);
        let f = fract(cell);
        let c00 = globe_display(disc_point((i0 + vec2<f32>(0.5, 0.5)) * block), aa);
        let c10 = globe_display(disc_point((i0 + vec2<f32>(1.5, 0.5)) * block), aa);
        let c01 = globe_display(disc_point((i0 + vec2<f32>(0.5, 1.5)) * block), aa);
        let c11 = globe_display(disc_point((i0 + vec2<f32>(1.5, 1.5)) * block), aa);
        display = mix(mix(c00, c10, f.x), mix(c01, c11, f.x), f.y);
        let lum = dot(display, vec3<f32>(0.3, 0.59, 0.11));
        display = mix(display, vec3<f32>(lum), globe.desaturate);
        let pixel = vec2<u32>(in.position.xy);
        let grain = (hash(pixel, globe.grain_seed) - 0.5) * globe.grain;
        let band = sin(in.position.y * 1.7) * BAND_LEVELS / 255.0;
        display += vec3<f32>(grain + band) + vec3<f32>(0.0, 0.0, BLUE_CAST_LEVELS / 255.0);
    }
    let out = pow(clamp(display, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2));
    let alpha = select(0.0, 1.0, r <= globe.opaque_radius);
    return vec4<f32>(out, alpha);
}
