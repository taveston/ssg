//! The frame body's place in the system, when the frame is not the star's
//! (MRK-14, MRK-15). Its own orbit is drawn only as a short tail ending at
//! it, which shows which way it travels without a ring crossing the view;
//! and a bearing tick, labelled with the parent's name, points from it
//! towards its parent.
//!
//! The tail is a world-space gizmo like the other rings (`rings.rs` leaves
//! the frame body out); the tick and its label are UI nodes. Neither is a
//! hover or click target (HOV-02).

use bevy::prelude::*;

use super::colors;
use super::fold::SatelliteFoldState;
use super::fonts::Fonts;
use super::frame::Frame;
use super::hovered::HoveredBodies;
use super::layers;
use super::plot_marks::screen_angle;
use super::rings::{DIM_RING_ALPHA, RingExtent, ring_extent, ring_highlight};
use super::scale::km_to_render;
use super::selection::Selection;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::orbit::{Orbit, OrbitPositionKm};
use crate::{Boot, Stage};

/// Straight pieces the frame body's tail is drawn with.
const TAIL_SEGMENTS: usize = 32;
/// A short step, as a fraction of the camera's distance from the frame
/// body, used to find directions and scales on screen near it. Short enough
/// to stay in front of the camera however close it is.
const PROBE_FRACTION: f32 = 1e-3;
/// The bearing label's font size and its gap beyond the tick's outer end.
const BEARING_LABEL_FONT_PX: f32 = 11.0;
const BEARING_LABEL_GAP_PX: f32 = 4.0;
const TICK_LINE_PX: f32 = 1.0;

/// The retained gizmo that draws the frame body's tail.
#[derive(Component)]
struct FrameTail;
/// The bearing tick and its label.
#[derive(Component)]
pub struct BearingTick;
#[derive(Component)]
pub struct BearingLabel;

pub struct FrameOrbitPlugin;

impl Plugin for FrameOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_frame_orbit.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (draw_frame_tail, place_bearing_tick).in_set(Stage::Overlay),
            );
    }
}

fn spawn_frame_orbit(
    mut commands: Commands,
    mut assets: ResMut<Assets<GizmoAsset>>,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
) {
    commands.spawn((
        FrameTail,
        Gizmo {
            handle: assets.add(GizmoAsset::new()),
            line_config: GizmoLineConfig {
                width: tunables.orbit_ring_width_px,
                ..default()
            },
            ..default()
        },
    ));
    commands.spawn((
        BearingTick,
        GlobalZIndex(layers::LABELS),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: px(TICK_LINE_PX),
            height: px(tunables.bearing_tick_px),
            display: Display::None,
            ..default()
        },
        UiTransform::IDENTITY,
        BackgroundColor(colors::NEUTRAL),
    ));
    commands.spawn((
        BearingLabel,
        GlobalZIndex(layers::LABELS),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Text::new(""),
        TextLayout::linebreak(LineBreak::NoWrap),
        fonts.data(BEARING_LABEL_FONT_PX),
        TextColor(colors::SECONDARY),
    ));
}

/// MRK-14: the tail's sweep, in radians, for an arc `arc_px` long on
/// screen where the orbit covers `px_per_rad` pixels a radian. Never more
/// than a full turn.
pub fn frame_tail_sweep(arc_px: f32, px_per_rad: f32) -> f32 {
    if px_per_rad <= 0.0 || !px_per_rad.is_finite() {
        return 0.0;
    }
    (arc_px / px_per_rad).min(std::f32::consts::TAU)
}

/// MRK-14: the tail's alpha `along` of the way from the frame body (0) to
/// its far end (1): `full` at the body, fading to transparent.
pub fn frame_tail_alpha(along: f32, full: f32) -> f32 {
    full * (1.0 - along.clamp(0.0, 1.0))
}

/// MRK-14: outside the star's frame, the frame body's orbit as a tail
/// `frame_orbit_arc_px` long on screen, ending at the body.
#[allow(clippy::too_many_arguments)]
fn draw_frame_tail(
    view: ViewNow,
    frame: Res<Frame>,
    tunables: Res<Tunables>,
    (selection, hovered): (Res<Selection>, Res<HoveredBodies>),
    bodies: Query<(&Orbit, &OrbitPositionKm, Option<&ChildOf>)>,
    tails: Query<&Gizmo, With<FrameTail>>,
    mut assets: ResMut<Assets<GizmoAsset>>,
) {
    for gizmo in &tails {
        let Some(mut asset) = assets.get_mut(&gizmo.handle) else {
            continue;
        };
        asset.clear();
        let Some(body) = frame.non_star_body() else {
            continue;
        };
        let Ok((orbit, local, child_of)) = bodies.get(body) else {
            continue;
        };
        let centre = parent_position(child_of, &view);
        let radius = km_to_render(orbit.radius_km);
        let body_angle = local.0.z.atan2(local.0.x) as f32;
        let world_at =
            |angle: f32| centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        let Some(probe_rad) = probe_length(&view, world_at(body_angle)).map(|l| l / radius) else {
            continue;
        };
        let probe = (
            view.project(world_at(body_angle)),
            view.project(world_at(body_angle - probe_rad)),
        );
        let (Some(a), Some(b)) = probe else {
            continue;
        };
        let sweep = frame_tail_sweep(tunables.frame_orbit_arc_px, a.distance(b) / probe_rad);
        // HOV-03: hovered or selected, the tail takes the white highlight.
        let colour = ring_highlight(
            selection.is_selected(body),
            hovered.contains(body),
            tunables.orbit_selected_alpha,
            tunables.orbit_hover_alpha,
        )
        .unwrap_or(colors::ORBIT_LINE.with_alpha(DIM_RING_ALPHA));
        let full = colour.alpha();
        for i in 0..TAIL_SEGMENTS {
            let along = |i: usize| i as f32 / TAIL_SEGMENTS as f32;
            let (from, to) = (along(i), along(i + 1));
            asset.line_gradient(
                world_at(body_angle - from * sweep),
                world_at(body_angle - to * sweep),
                colour.with_alpha(frame_tail_alpha(from, full)),
                colour.with_alpha(frame_tail_alpha(to, full)),
            );
        }
    }
}

/// A step near `at` short enough to stay in front of the camera.
fn probe_length(view: &ViewNow, at: Vec3) -> Option<f32> {
    let (_, camera) = view.camera()?;
    Some(camera.translation().distance(at) * PROBE_FRACTION)
}

/// Where a body's parent is: the star has no entry of its own.
fn parent_position(child_of: Option<&ChildOf>, view: &ViewNow) -> Vec3 {
    child_of
        .and_then(|c| view.position(c.parent()))
        .unwrap_or(view.star())
}

/// MRK-15: the bearing tick's inner and outer ends on screen. It points
/// from the frame body at `body` towards `toward` (a point on the way to
/// the parent), starting on the outermost drawn satellite orbit's point
/// `on_ring` if there is one, else `radius_px` out.
pub fn bearing_tick(
    body: Vec2,
    toward: Vec2,
    on_ring: Option<Vec2>,
    radius_px: f32,
    tick_px: f32,
) -> Option<(Vec2, Vec2)> {
    let inner = on_ring.unwrap_or(body + (toward - body).normalize_or_zero() * radius_px);
    let dir = (inner - body).normalize_or_zero();
    (dir != Vec2::ZERO).then_some((inner, inner + dir * tick_px))
}

/// What the bearing tick reads about the frame body and its satellites.
type TickBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static Orbit,
        &'static BodyClass,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
    ),
>;

/// MRK-15: outside the star's frame, a neutral tick pointing from the
/// frame body towards its parent, on its outermost drawn satellite orbit,
/// labelled with the parent's name.
#[allow(clippy::type_complexity)]
fn place_bearing_tick(
    view: ViewNow,
    frame: Res<Frame>,
    tunables: Res<Tunables>,
    bodies: TickBodies,
    names: Query<&DisplayName>,
    mut ticks: Query<(&mut Node, &mut UiTransform), (With<BearingTick>, Without<BearingLabel>)>,
    mut labels: Query<(&mut Node, &mut Text, &ComputedNode), With<BearingLabel>>,
) {
    let placed = frame
        .non_star_body()
        .and_then(|body| tick_ends(body, &view, &bodies, &tunables).map(|ends| (body, ends)));
    for (mut node, mut transform) in &mut ticks {
        let Some((_, (inner, outer))) = placed else {
            node.display = Display::None;
            continue;
        };
        let mid = (inner + outer) / 2.0;
        node.display = Display::Flex;
        node.height = px(tunables.bearing_tick_px);
        node.left = px(mid.x - TICK_LINE_PX / 2.0);
        node.top = px(mid.y - tunables.bearing_tick_px / 2.0);
        transform.rotation = Rot2::radians(screen_angle(inner, outer));
    }
    for (mut node, mut text, computed) in &mut labels {
        let parent_name = placed.and_then(|(body, _)| {
            let parent = bodies.get(body).ok()?.2.map_or(frame.star, ChildOf::parent);
            names.get(parent).ok().map(|name| name.0.clone())
        });
        let (Some((_, (inner, outer))), Some(name)) = (placed, parent_name) else {
            node.display = Display::None;
            continue;
        };
        if text.0 != name {
            text.0 = name;
        }
        // Centred beyond the tick's outer end, clear of it on that side.
        let size = computed.size() * computed.inverse_scale_factor();
        let dir = (outer - inner).normalize_or_zero();
        let reach = (dir.x.abs() * size.x + dir.y.abs() * size.y) / 2.0 + BEARING_LABEL_GAP_PX;
        let centre = outer + dir * reach;
        node.display = Display::Flex;
        node.left = px(centre.x - size.x / 2.0);
        node.top = px(centre.y - size.y / 2.0);
    }
}

/// The tick's ends for this frame body, if it has a parent and both it and
/// the way to the parent project.
fn tick_ends(
    body: Entity,
    view: &ViewNow,
    bodies: &TickBodies,
    tunables: &Tunables,
) -> Option<(Vec2, Vec2)> {
    let (_, _, child_of, _) = bodies.get(body).ok()?;
    let at = view.position(body)?;
    let parent_at = parent_position(child_of, view);
    let toward_parent = (parent_at - at).normalize_or_zero();
    let body_screen = view.project(at)?;
    let step = probe_length(view, at)?;
    let toward = view.project(at + toward_parent * step)?;
    let outermost = bodies
        .iter()
        .filter(|(_, class, child_of, fold)| {
            child_of.is_some_and(|c| c.parent() == body)
                && ring_extent(
                    **class,
                    fold.is_some_and(|f| f.effective_hidden),
                    false,
                    false,
                ) != RingExtent::Hidden
        })
        .map(|(orbit, ..)| km_to_render(orbit.radius_km))
        .fold(None, |most: Option<f32>, r| {
            Some(most.map_or(r, |m| m.max(r)))
        });
    let on_ring = match outermost {
        Some(radius) => Some(view.project(at + toward_parent * radius)?),
        None => None,
    };
    bearing_tick(
        body_screen,
        toward,
        on_ring,
        tunables.bearing_tick_radius_px,
        tunables.bearing_tick_px,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-14: the tail is as long on screen as asked, up to a full turn.
    #[test]
    fn the_frame_tail_sweeps_its_screen_length() {
        let sweep = frame_tail_sweep(160.0, 400.0);
        assert!((sweep * 400.0 - 160.0).abs() < 1e-4);
        assert_eq!(frame_tail_sweep(160.0, 1.0), std::f32::consts::TAU);
        assert_eq!(frame_tail_sweep(160.0, 0.0), 0.0);
    }

    /// MRK-14: full ring alpha at the body, transparent at the far end.
    #[test]
    fn the_frame_tail_fades_away_from_the_body() {
        assert_eq!(frame_tail_alpha(0.0, 0.35), 0.35);
        assert!((frame_tail_alpha(0.5, 0.35) - 0.175).abs() < 1e-6);
        assert_eq!(frame_tail_alpha(1.0, 0.35), 0.0);
    }

    /// MRK-15: the tick sits on the outermost drawn satellite orbit and
    /// points outward, towards the parent.
    #[test]
    fn the_bearing_tick_sits_on_the_outermost_orbit() {
        let body = Vec2::new(100.0, 100.0);
        let toward = Vec2::new(90.0, 100.0);
        let (inner, outer) =
            bearing_tick(body, toward, Some(Vec2::new(20.0, 100.0)), 60.0, 6.0).unwrap();
        assert_eq!(inner, Vec2::new(20.0, 100.0));
        assert_eq!(outer, Vec2::new(14.0, 100.0));
    }

    /// MRK-15: with no satellite orbit drawn, it sits
    /// `bearing_tick_radius_px` out towards the parent.
    #[test]
    fn the_bearing_tick_falls_back_to_its_radius() {
        let body = Vec2::new(100.0, 100.0);
        let toward = Vec2::new(100.0, 110.0);
        let (inner, outer) = bearing_tick(body, toward, None, 60.0, 6.0).unwrap();
        assert_eq!(inner, Vec2::new(100.0, 160.0));
        assert_eq!(outer, Vec2::new(100.0, 166.0));
    }
}
