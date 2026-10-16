// Draws one part of a ship or station symbol (SYM-02 to SYM-08) as vector
// geometry, evaluated per screen pixel at the size it is drawn (SYM-17).
// Shapes are signed distances in logical pixels from the node's centre
// (y down): negative inside. The part's coverage is antialiased over one
// physical pixel. Mirrors `SymbolMask` in `shape.rs`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct SymbolShape {
    color: vec4<f32>,
    // 0 frame, 1 fill, 2 triangle, 3 drop, 4 crate, 5 unit box, 6 head,
    // 7 chevron.
    mask: u32,
    // 0 circle, 1 square, 2 diamond, 3 quatrefoil.
    shape: u32,
    dashed: u32,
    crossed: u32,
    stroke_px: f32,
    size_px: f32,
}

@group(1) @binding(0)
var<uniform> part: SymbolShape;

const TAU: f32 = 6.28318530718;
// Dashes round a dashed frame, and the part of each period drawn (SYM-08).
const DASHES: f32 = 12.0;
const DASH_ON: f32 = 0.55;
// The square frame sits a little inside the symbol square, so it doesn't
// look bigger than a circle of the same box (NATO practice).
const SQUARE_SCALE: f32 = 0.85;
// A quatrefoil is four circles of this radius (as a fraction of the
// frame's half-size), their centres the rest of the way out.
const QUATREFOIL_LOBE: f32 = 0.6;
// The icon's half-size, as a fraction of the frame's.
const ICON_SCALE: f32 = 0.45;
// The north dial head's half-width, as a fraction of its half-length.
const HEAD_WIDTH: f32 = 0.7;

fn box_sdf(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let d = abs(p) - half;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn segment_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
    return distance(p, a + ab * t);
}

fn triangle_sdf(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    let d = min(segment_distance(p, a, b), min(segment_distance(p, b, c), segment_distance(p, c, a)));
    // Inside when on the same side of all three edges.
    let s1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let s2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let s3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
    let inside = (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0);
    return select(d, -d, inside);
}

fn frame_sdf(p: vec2<f32>, half: f32) -> f32 {
    switch part.shape {
        case 1u: { return box_sdf(p, vec2<f32>(half * SQUARE_SCALE)); }
        case 2u: { return (abs(p.x) + abs(p.y) - half) / sqrt(2.0); }
        case 3u: {
            let lobe = half * QUATREFOIL_LOBE;
            let out = half - lobe;
            let a = distance(p, vec2<f32>(out, 0.0));
            let b = distance(p, vec2<f32>(-out, 0.0));
            let c = distance(p, vec2<f32>(0.0, out));
            let d = distance(p, vec2<f32>(0.0, -out));
            return min(min(a, b), min(c, d)) - lobe;
        }
        default: { return length(p) - half; }
    }
}

// Distance outside a line of the stroke's width centred on `sdf`'s edge.
fn line(sdf: f32) -> f32 {
    return abs(sdf) - part.stroke_px / 2.0;
}

fn on_dash(p: vec2<f32>) -> bool {
    let turn = fract(atan2(p.y, p.x) / TAU + 1.0);
    return fract(turn * DASHES) < DASH_ON;
}

// Signed distance outside the covered region of the part.
fn region(p: vec2<f32>) -> f32 {
    // To the middle of the outline, so the stroke stays inside the node.
    let half = part.size_px / 2.0 - part.stroke_px / 2.0;
    let icon = half * ICON_SCALE;
    switch part.mask {
        case 0u: {
            if part.dashed != 0u && !on_dash(p) {
                return 1e6;
            }
            return line(frame_sdf(p, half));
        }
        case 1u: { return frame_sdf(p, half); }
        case 2u: {
            return triangle_sdf(p, vec2<f32>(0.0, -icon), vec2<f32>(icon, icon * 0.8), vec2<f32>(-icon, icon * 0.8));
        }
        case 3u: {
            let bulb = distance(p, vec2<f32>(0.0, icon * 0.3)) - icon * 0.6;
            let tip = triangle_sdf(p, vec2<f32>(0.0, -icon), vec2<f32>(icon * 0.52, 0.0), vec2<f32>(-icon * 0.52, 0.0));
            return line(min(bulb, tip));
        }
        case 4u: {
            let box_line = line(box_sdf(p, vec2<f32>(icon, icon * 0.75)));
            let middle = line(segment_distance(p, vec2<f32>(-icon, 0.0), vec2<f32>(icon, 0.0)));
            return min(box_line, middle);
        }
        case 6u: {
            // The north dial's head: tip at the top edge, base at the
            // bottom, its width this fraction of its length.
            let h = part.size_px / 2.0;
            let w = h * HEAD_WIDTH;
            return triangle_sdf(p, vec2<f32>(0.0, -h), vec2<f32>(w, h), vec2<f32>(-w, h));
        }
        case 7u: {
            // An open arrowhead, tip up: two strokes from the lower
            // corners to the tip, half as long as it is wide.
            let tip = vec2<f32>(0.0, -half / 2.0);
            let left = segment_distance(p, vec2<f32>(-half, half / 2.0), tip);
            let right = segment_distance(p, vec2<f32>(half, half / 2.0), tip);
            return min(left, right) - part.stroke_px / 2.0;
        }
        default: {
            let corner = vec2<f32>(icon * 1.1, icon * 0.65);
            var d = line(box_sdf(p, corner));
            if part.crossed != 0u {
                let other = vec2<f32>(corner.x, -corner.y);
                d = min(d, line(segment_distance(p, -corner, corner)));
                d = min(d, line(segment_distance(p, -other, other)));
            }
            return d;
        }
    }
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - vec2<f32>(0.5)) * part.size_px;
    // Physical pixels per logical pixel, so edges soften over one screen
    // pixel at any UI scale.
    let scale = in.size.x / part.size_px;
    let coverage = clamp(0.5 - region(p) * scale, 0.0, 1.0);
    return vec4<f32>(part.color.rgb, part.color.a * coverage);
}
