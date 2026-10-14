// Draws the near arc of a satellite's ring over its true-size parent
// (MRK-12): a run of line segments, each with a colour at either end,
// clipped to the parent's disc. Positions are logical pixels from the
// node's centre (y down); edges soften over one physical pixel. Mirrors
// `NearArcUniform` in `near_arcs.rs`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

const MAX_SEGMENTS: u32 = 128u;

struct NearArc {
    ends: array<vec4<f32>, 128>,
    from_colors: array<vec4<f32>, 128>,
    to_colors: array<vec4<f32>, 128>,
    count: u32,
    half_px: f32,
    width_px: f32,
}

@group(1) @binding(0)
var<uniform> arc: NearArc;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * 2.0 * arc.half_px;
    // Physical pixels per logical pixel.
    let scale = in.size.x / (2.0 * arc.half_px);
    var best = vec4<f32>(0.0);
    for (var i = 0u; i < min(arc.count, MAX_SEGMENTS); i++) {
        let a = arc.ends[i].xy;
        let b = arc.ends[i].zw;
        let ab = b - a;
        let len2 = max(dot(ab, ab), 1e-6);
        let t = clamp(dot(p - a, ab) / len2, 0.0, 1.0);
        let d = distance(p, a + ab * t) * scale;
        let coverage = clamp(arc.width_px / 2.0 - d + 0.5, 0.0, 1.0);
        let color = mix(arc.from_colors[i], arc.to_colors[i], t);
        let alpha = color.a * coverage;
        if alpha > best.a {
            best = vec4<f32>(color.rgb, alpha);
        }
    }
    // Only over the disc: outside it the ring's own stroke shows.
    let inside = clamp(0.5 - (length(p) - arc.half_px) * scale, 0.0, 1.0);
    return vec4<f32>(best.rgb, best.a * inside);
}
