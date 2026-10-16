//! The frame cue's nodes and the system that lays them out each frame
//! (REF-01 to REF-10): four dotted arms, the two halves of the vertical
//! axis and the "N", placed from this frame's camera and the frame body.
//!
//! `cue_layout` works the cue out from plain values and a projection, so
//! it can be checked without a renderer; `draw_frame_cue` gathers them and
//! applies the result to the nodes.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::geometry::{
    Stroke, StrokeStyle, VerticalLook, arm_extent, arm_ticks, normal_foreshortening,
    north_label_at, vertical_halves,
};
use super::stroke::{CueStroke, stroke_node_box};
use crate::sim::body::PhysicalRadius;
use crate::view::camera::rules::NORTH;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::frame::Frame;
use crate::view::halo::TextHalo;
use crate::view::layers;
use crate::view::markers::{MarkerDrawn, MarkerHalfSize, MarkerShape, box_half};
use crate::view::plot_marks::ScaleBarKm;
use crate::view::scale::km_to_render;
use crate::view::screen::MapView;
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;

/// The arms' directions in the orbital plane, north first: north, east,
/// south, west (REF-01).
pub const ARM_DIRECTIONS: [Vec3; 4] = [NORTH, Vec3::X, Vec3::Z, Vec3::NEG_X];
/// The plane's normal on the camera's side (CAM-01 keeps the camera above
/// the plane).
const UP: Vec3 = Vec3::Y;
/// A step along a direction, as a fraction of the camera's distance from
/// the frame body, short enough to find the direction's screen heading.
const STEP_FRACTION: f32 = 1e-3;
/// The "N"'s size in the data face, and the box it is centred in.
const N_FONT_PX: f32 = 10.0;
const N_BOX_PX: f32 = 12.0;

/// Which part of the cue a node draws.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CuePart {
    /// An arm, by its index in `ARM_DIRECTIONS` (0 is north).
    Arm(usize),
    /// The vertical axis's upper half (REF-06).
    Upper,
    /// The vertical axis's lower half (REF-07).
    Lower,
}

/// The node holding the north arm's "N" (REF-04).
#[derive(Component)]
pub struct CueNorthLabel;

/// What the frame body looks like on screen: its box's half-size
/// (HOV-07) and, when it is drawn as a true-size disc (MRK-07), its true
/// radius in render units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBodyLook {
    pub box_half: f32,
    pub disc_radius: Option<f32>,
}

/// The camera as the cue needs it: where it is, which way it looks, and
/// how it projects a world point to the screen.
pub struct CueCamera<P: Fn(Vec3) -> Option<Vec2>> {
    pub eye: Vec3,
    pub forward: Vec3,
    pub project: P,
}

/// The whole cue for one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CueLayout {
    pub arms: [Option<Stroke>; 4],
    pub upper: Option<Stroke>,
    pub lower: Option<Stroke>,
    pub north_label: Option<Vec2>,
}

/// The unit screen heading of the world step from `from` to `to`, from
/// `centre`, where `from` lands.
fn screen_heading<P: Fn(Vec3) -> Option<Vec2>>(
    camera: &CueCamera<P>,
    centre: Vec2,
    to: Vec3,
) -> Option<Vec2> {
    ((camera.project)(to)? - centre).try_normalize()
}

/// Where every arm and the vertical axis start from: the frame body at
/// `body` (render units), landing at `centre` on screen, with its `look`;
/// the map area the arms stay inside and the vertical axis is sized from,
/// and the ticks' spacing in render units (the scale bar's distance).
#[derive(Clone, Copy)]
struct ArmBase {
    body: Vec3,
    centre: Vec2,
    look: FrameBodyLook,
    map: Rect,
    tick_render: f32,
}

/// REF-01 to REF-05: one arm along `direction` in the plane from the
/// frame body: from the gap round its box to the map's edge or the
/// horizon, ticked at multiples of the ticks' spacing, at `strength`
/// times the fade.
fn arm<P: Fn(Vec3) -> Option<Vec2>>(
    camera: &CueCamera<P>,
    base: &ArmBase,
    direction: Vec3,
    strength: f32,
    t: &Tunables,
) -> Option<Stroke> {
    let ArmBase {
        body,
        centre,
        look,
        map,
        tick_render,
    } = *base;
    let reach = body.distance(camera.eye);
    let dir = screen_heading(camera, centre, body + direction * reach * STEP_FRACTION)?;
    // A direction leading away from the camera meets the horizon at its
    // vanishing point: where any line along it, such as the one through
    // the eye, lands on screen.
    let horizon = (direction.dot(camera.forward) > 0.0)
        .then(|| (camera.project)(camera.eye + direction * reach))
        .flatten()
        .map(|vanish| (vanish - centre).dot(dir));
    let start_px = look.box_half + t.ref_gap_px;
    let (start, end) = arm_extent(centre, dir, start_px, horizon, map)?;
    let along = |k: u32| {
        let point = body + direction * tick_render * k as f32;
        Some(((camera.project)(point)? - centre).dot(dir))
    };
    let ticks = if tick_render > 0.0 {
        arm_ticks(along, start, end)
    } else {
        Vec::new()
    };
    Some(Stroke {
        start: centre + dir * start,
        dir,
        length: end - start,
        alpha_start: t.ref_alpha_near * strength,
        alpha_end: t.ref_alpha_far * strength,
        style: StrokeStyle {
            dot_px: t.ref_dot_px,
            pitch_px: t.ref_dot_pitch_px,
            arrow_px: 0.0,
        },
        ticks: ticks.into_iter().map(|at| at - start).collect(),
        tick_px: t.ref_tick_px,
    })
}

/// REF-06 / REF-07: the vertical axis's halves through the frame body,
/// dotted as the arms are. The upper half starts at the body's pole when
/// it is a true-size disc, and the gap beyond its box otherwise; edge-on
/// it is `ref_up_fraction` of the map area's height long.
fn vertical<P: Fn(Vec3) -> Option<Vec2>>(
    camera: &CueCamera<P>,
    base: &ArmBase,
    t: &Tunables,
) -> (Option<Stroke>, Option<Stroke>) {
    let ArmBase {
        body,
        centre,
        look,
        map,
        ..
    } = *base;
    let reach = body.distance(camera.eye);
    let Some(up) = screen_heading(camera, centre, body + UP * reach * STEP_FRACTION) else {
        return (None, None);
    };
    let upper_from = match look.disc_radius {
        Some(radius) => (camera.project)(body + UP * radius).map_or(0.0, |p| p.distance(centre)),
        None => look.box_half + t.ref_gap_px,
    };
    let shortening = normal_foreshortening((body - camera.eye).normalize_or_zero());
    let vertical_look = VerticalLook {
        up_px: t.ref_up_fraction * map.height(),
        down_fraction: t.ref_down_fraction,
        min_px: t.ref_vertical_min_px,
        dot_px: t.ref_dot_px,
        pitch_px: t.ref_dot_pitch_px,
        arrow_px: t.ref_arrow_px,
        alpha_near: t.ref_alpha_near,
        alpha_far: t.ref_alpha_far,
    };
    vertical_halves(centre, up, upper_from, shortening, &vertical_look)
}

/// REF-01 to REF-08: the cue round the frame body at `body` (render
/// units), seen by `camera`, inside the map area `map`. The north arm is
/// drawn at full strength and the others at `ref_minor_factor` (REF-03);
/// the "N" sits inside the north arm's end (REF-04).
pub fn cue_layout<P: Fn(Vec3) -> Option<Vec2>>(
    camera: &CueCamera<P>,
    body: Vec3,
    look: FrameBodyLook,
    map: Rect,
    tick_render: f32,
    t: &Tunables,
) -> CueLayout {
    let Some(centre) = (camera.project)(body) else {
        return CueLayout::default();
    };
    let base = ArmBase {
        body,
        centre,
        look,
        map,
        tick_render,
    };
    let arms = ARM_DIRECTIONS.map(|direction| {
        let strength = if direction == NORTH {
            1.0
        } else {
            t.ref_minor_factor
        };
        arm(camera, &base, direction, strength, t)
    });
    let (upper, lower) = vertical(camera, &base, t);
    let north_label = arms[0]
        .as_ref()
        .and_then(|north| north_label_at(north, t.ref_n_inset_px));
    CueLayout {
        arms,
        upper,
        lower,
        north_label,
    }
}

/// One stroke node of the cue, in its layer (REF-09). Never a pick target
/// (REF-10).
fn stroke_node(part: CuePart, layer: i32) -> impl Bundle {
    (
        part,
        CueStroke {
            color: colors::REFERENCE,
            stroke: None,
        },
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        UiTransform::default(),
        GlobalZIndex(layer),
        Pickable::IGNORE,
    )
}

/// REF-01 / REF-04 / REF-09: the cue's nodes. The arms and the lower half
/// lie under every natural body, with the "N" just above them; the upper
/// half lies above the bodies and below the symbols.
pub fn spawn_frame_cue(mut commands: Commands, fonts: Res<Fonts>) {
    for index in 0..ARM_DIRECTIONS.len() {
        commands.spawn(stroke_node(CuePart::Arm(index), layers::CUE));
    }
    commands.spawn(stroke_node(CuePart::Lower, layers::CUE));
    commands.spawn(stroke_node(CuePart::Upper, layers::CUE_UPPER));
    commands
        .spawn((
            CueNorthLabel,
            Node {
                position_type: PositionType::Absolute,
                width: px(N_BOX_PX),
                height: px(N_BOX_PX),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(layers::CUE_LABEL),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new("N"),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.data(N_FONT_PX),
            TextColor(colors::REFERENCE),
            TextHalo::ground(),
            Pickable::IGNORE,
        ));
}

/// The markers as drawn this frame, and the bodies' true radii.
#[derive(SystemParam)]
pub struct FrameBodyMarkers<'w, 's> {
    markers: Query<
        'w,
        's,
        (
            &'static MarkerFor,
            &'static MarkerDrawn,
            &'static MarkerShape,
            &'static MarkerHalfSize,
        ),
    >,
    radii: Query<'w, 's, &'static PhysicalRadius>,
}

/// The frame body's look on screen, from its marker (HOV-07, MRK-07).
fn frame_body_look(body: Entity, bodies: &FrameBodyMarkers, t: &Tunables) -> FrameBodyLook {
    let FrameBodyMarkers { markers, radii } = bodies;
    let marker = markers.iter().find(|(marker_for, ..)| marker_for.0 == body);
    let Some((_, drawn, shape, class_half)) = marker else {
        return FrameBodyLook {
            box_half: box_half(0.0, t),
            disc_radius: None,
        };
    };
    let half = drawn.0.map_or(class_half.0.x, |(_, half)| half);
    let is_disc = *shape == MarkerShape::Disc && half > class_half.0.x;
    FrameBodyLook {
        box_half: box_half(half, t),
        disc_radius: is_disc
            .then(|| radii.get(body).ok().map(|r| km_to_render(r.0)))
            .flatten(),
    }
}

/// Places a stroke's node along it, or hides it.
fn place_stroke(
    stroke: Option<Stroke>,
    look: &mut Mut<CueStroke>,
    node: &mut Node,
    turn: &mut UiTransform,
) {
    match &stroke {
        Some(stroke) => {
            let placed = stroke_node_box(stroke);
            node.display = Display::Flex;
            node.left = px(placed.min.x);
            node.top = px(placed.min.y);
            node.width = px(placed.size.x);
            node.height = px(placed.size.y);
            if turn.rotation != placed.rotation {
                turn.rotation = placed.rotation;
            }
        }
        None => node.display = Display::None,
    }
    let wanted = CueStroke {
        color: look.color,
        stroke,
    };
    look.set_if_neq(wanted);
}

/// The cue's stroke nodes, to lay out.
type StrokeNodes<'w, 's> = Query<
    'w,
    's,
    (
        &'static CuePart,
        &'static mut CueStroke,
        &'static mut Node,
        &'static mut UiTransform,
    ),
    Without<CueNorthLabel>,
>;

/// The cue's nodes: its strokes and the "N".
#[derive(SystemParam)]
pub struct CueNodes<'w, 's> {
    strokes: StrokeNodes<'w, 's>,
    labels: Query<'w, 's, &'static mut Node, With<CueNorthLabel>>,
}

/// REF-01 to REF-09: lays the cue out round the frame body for this
/// frame's camera, after the markers (for the body's box) and the scale
/// bar (for the ticks' distance).
pub fn draw_frame_cue(
    view: ViewNow,
    frame: Res<Frame>,
    map_view: MapView,
    scale_km: Res<ScaleBarKm>,
    tunables: Res<Tunables>,
    bodies: FrameBodyMarkers,
    mut nodes: CueNodes,
) {
    let CueNodes { strokes, labels } = &mut nodes;
    let layout = match (view.camera(), view.position(frame.body), map_view.area()) {
        (Some((_, transform)), Some(body), Some(area)) => {
            let camera = CueCamera {
                eye: transform.translation(),
                forward: transform.forward().as_vec3(),
                project: |p| view.project(p),
            };
            let map = area.rect();
            let look = frame_body_look(frame.body, &bodies, &tunables);
            let tick_render = km_to_render(scale_km.0);
            cue_layout(&camera, body, look, map, tick_render, &tunables)
        }
        _ => CueLayout::default(),
    };
    for (part, mut look, mut node, mut turn) in strokes.iter_mut() {
        let stroke = match *part {
            CuePart::Arm(index) => layout.arms.get(index).cloned().flatten(),
            CuePart::Upper => layout.upper.clone(),
            CuePart::Lower => layout.lower.clone(),
        };
        place_stroke(stroke, &mut look, &mut node, &mut turn);
    }
    for mut node in labels.iter_mut() {
        match layout.north_label {
            Some(at) => {
                node.display = Display::Flex;
                node.left = px(at.x - N_BOX_PX / 2.0);
                node.top = px(at.y - N_BOX_PX / 2.0);
            }
            None => node.display = Display::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REF-06: seen edge-on, the upper half is `ref_up_fraction` of the
    /// map area's height, so it follows the window and the inset.
    #[test]
    fn the_upper_half_follows_the_map_height() {
        let t = Tunables::default();
        // Edge-on, looking along the plane at a body at the origin; one
        // render unit is 100 px on screen.
        let camera = CueCamera {
            eye: Vec3::new(0.0, 0.0, 10.0),
            forward: Vec3::NEG_Z,
            project: |p: Vec3| Some(Vec2::new(400.0 + 100.0 * p.x, 300.0 - 100.0 * p.y)),
        };
        let look = FrameBodyLook {
            box_half: 5.0,
            disc_radius: None,
        };
        let upper_for = |height: f32| {
            let map = Rect::new(0.0, 40.0, 800.0, 40.0 + height);
            let layout = cue_layout(&camera, Vec3::ZERO, look, map, 0.0, &t);
            layout.upper.expect("an upper half").length
        };
        for height in [560.0, 300.0] {
            let length = upper_for(height);
            let wanted = t.ref_up_fraction * height;
            assert!(
                (length - wanted).abs() < 1e-3,
                "{length} px, wanted {wanted}"
            );
        }
    }
}
