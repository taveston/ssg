// Draws a ghost (PLNR-63): a circle's outline, dashed, with an empty
// centre, filling its node. Distances are in logical pixels from the
// node's centre; edges soften over one physical pixel. Mirrors
// `GhostUniform` in `ghost.rs`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct Ghost {
    color: vec4<f32>,
    size_px: f32,
    width_px: f32,
    // Dash length and the gap after it, along the circle.
    dash_px: f32,
}

@group(1) @binding(0)
var<uniform> ghost: Ghost;

const TAU: f32 = 6.28318530718;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * ghost.size_px;
    let scale = in.size.x / ghost.size_px;
    // The outline's middle, kept inside the node.
    let radius = ghost.size_px / 2.0 - ghost.width_px / 2.0;
    let line = abs(length(p) - radius) - ghost.width_px / 2.0;
    // A whole number of dash periods round the circle, each as close to
    // dash + gap as fits, so the dashes meet evenly where they start.
    let around = TAU * radius;
    let periods = max(1.0, round(around / (2.0 * ghost.dash_px)));
    let turn = fract(atan2(p.y, p.x) / TAU + 1.0);
    let on = fract(turn * periods) < 0.5;
    let coverage = select(0.0, clamp(0.5 - line * scale, 0.0, 1.0), on);
    return vec4<f32>(ghost.color.rgb, ghost.color.a * coverage);
}
