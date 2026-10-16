// Draws one stroke of the frame cue (REF-01, REF-05, REF-06): round dots
// at a fixed pitch, or a solid line ending in a filled arrowhead, with
// ticks across it, fading from its start to its end. The node lies along
// the stroke, turned to its direction, so x runs along the stroke and y
// across it. Positions are logical pixels; edges soften over one physical
// pixel. Mirrors `CueStrokeUniform` in `stroke.rs`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

const MAX_TICKS: u32 = 64u;

struct CueStroke {
    color: vec4<f32>,
    node_size: vec2<f32>,
    length_px: f32,
    alpha_start: f32,
    alpha_end: f32,
    width_px: f32,
    // 0 for a solid line.
    pitch_px: f32,
    tick_px: f32,
    tick_width_px: f32,
    tick_count: u32,
    arrow_px: f32,
    arrow_half_px: f32,
    ticks: array<vec4<f32>, 16>,
}

@group(1) @binding(0)
var<uniform> cue: CueStroke;

// How much of a pixel lies inside an edge `d` logical pixels away
// (positive inside), with `scale` physical pixels per logical pixel.
fn cover(d: f32, scale: f32) -> f32 {
    return clamp(d * scale + 0.5, 0.0, 1.0);
}

// The stroke's opacity `along` pixels from its start.
fn fade(along: f32) -> f32 {
    let t = clamp(along / max(cue.length_px, 1e-3), 0.0, 1.0);
    return mix(cue.alpha_start, cue.alpha_end, t);
}

// Coverage of the dots: the nearest dot's centre, with the first dot
// touching the start and none past the end.
fn dots(along: f32, across: f32, scale: f32) -> f32 {
    let r = cue.width_px / 2.0;
    let last = floor(max(cue.length_px - cue.width_px, 0.0) / cue.pitch_px);
    let k = clamp(round((along - r) / cue.pitch_px), 0.0, last);
    let centre = r + k * cue.pitch_px;
    return cover(r - length(vec2<f32>(along - centre, across)), scale);
}

// Coverage of the solid line, up to the arrowhead's base, and of the
// arrowhead, whose tip is the stroke's end.
fn solid(along: f32, across: f32, scale: f32) -> f32 {
    let line_end = cue.length_px - cue.arrow_px;
    let line = cover(cue.width_px / 2.0 - abs(across), scale)
        * cover(min(along, line_end - along), scale);
    if cue.arrow_px <= 0.0 {
        return line;
    }
    // Distance back from the tip; the head widens linearly to its base.
    let back = cue.length_px - along;
    let slope = cue.arrow_half_px / cue.arrow_px;
    let edge = (back * slope - abs(across)) / sqrt(1.0 + slope * slope);
    let head = cover(min(edge, min(back, cue.arrow_px - back)), scale);
    return max(line, head);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * cue.node_size;
    let scale = in.size.x / cue.node_size.x;
    let along = p.x + cue.length_px / 2.0;
    let across = p.y;
    var line = 0.0;
    if cue.pitch_px > 0.0 {
        line = dots(along, across, scale);
    } else {
        line = solid(along, across, scale);
    }
    var alpha = fade(along) * line;
    let half_tick = cue.tick_px / 2.0;
    for (var i = 0u; i < min(cue.tick_count, MAX_TICKS); i++) {
        let at = cue.ticks[i / 4u][i % 4u];
        let c = cover(cue.tick_width_px / 2.0 - abs(along - at), scale)
            * cover(half_tick - abs(across), scale);
        alpha = max(alpha, fade(at) * c);
    }
    return vec4<f32>(cue.color.rgb, alpha);
}
