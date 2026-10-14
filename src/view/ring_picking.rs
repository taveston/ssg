//! Hover and click on an orbit ring (HOV-02, HOV-05): a small picking
//! backend that hits any drawn ring, tail or exposure ring anywhere along
//! its length, by distance from it as drawn on screen. Only what is drawn
//! is a target: a ship's tail in dot mode, not the rest of its orbit
//! (MRK-09), and not a ring faded out by its size (MRK-14). A ring
//! carrying the frame body is never a target; on a shared ring, the other
//! members' tails still target those members.
//!
//! Gizmo-drawn rings aren't pickable on their own. Each ring gets one
//! invisible target entity. A natural body's ring, or a named orbit with
//! one member, stands for that body (`MarkerFor`) with the ordinary marker
//! observers, so a hit behaves exactly like a hit on the marker (white
//! ring, select, double-click). A shared orbit's ring stands for the orbit
//! (`OrbitFor`): hovering it highlights the ring and its identity pill, and
//! clicking it selects the parent with that orbit's row revealed. Hits are
//! reported below the markers' and the UI's order, so a marker, pill, label
//! or panel still wins.

use std::collections::HashMap;

use bevy::picking::PickingSystems;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use super::frame::Frame;
use super::hover;
use super::hovered::HoveredBodies;
use super::rings::{FadeView, RingExtent, ring_extent};
use super::scale::km_to_render;
use super::screen::ScreenPositions;
use super::selection::Selection;
use super::shared_orbit::{OrbitRings, Ring, ring_screen_point};
use super::targets::{MarkerFor, OrbitFor};
use super::tunables::Tunables;
use crate::view::world::ViewNow;
use crate::{Boot, Stage};

/// Straight segments a ring is traced with for hit-testing.
const RING_TRACE_SEGMENTS: usize = 64;

/// The invisible hit target standing for a ring, for the body or named
/// orbit the ring stands for.
#[derive(Component)]
pub struct RingTarget(pub Entity);

/// The invisible hit target standing for one member's tail on a shared
/// ring, used only while the frame body is on that ring (HOV-02).
#[derive(Component)]
pub struct MemberTailTarget {
    pub orbit: Entity,
    pub body: Entity,
}

/// Each drawn ring's on-screen segments, by its target entity. Rebuilt
/// every frame; a ring that isn't showing has no entry.
#[derive(Resource, Default)]
pub struct RingPaths(pub HashMap<Entity, Vec<(Vec2, Vec2)>>);

/// The distance from `point` to the nearest of `segments`, or infinity if
/// there are none.
pub fn distance_to_segments(point: Vec2, segments: &[(Vec2, Vec2)]) -> f32 {
    segments
        .iter()
        .map(|&(a, b)| {
            let along = b - a;
            let t = if along.length_squared() > 0.0 {
                ((point - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            point.distance(a + along * t)
        })
        .fold(f32::INFINITY, f32::min)
}

/// HOV-02: the ring under `point`, if any is within `tolerance`: the
/// nearest one, so overlapping rings resolve to the closer line.
pub fn ring_under(point: Vec2, paths: &RingPaths, tolerance: f32) -> Option<(Entity, f32)> {
    paths
        .0
        .iter()
        .map(|(&target, segments)| (target, distance_to_segments(point, segments)))
        .filter(|&(_, distance)| distance <= tolerance)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub struct RingPickingPlugin;

impl Plugin for RingPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RingPaths>()
            .add_systems(Startup, spawn_ring_targets.in_set(Boot::Overlay))
            .add_systems(Update, trace_rings.in_set(Stage::Overlay))
            .add_systems(PreUpdate, pick_rings.in_set(PickingSystems::Backend));
    }
}

/// HOV-02 / HOV-05: one target per ring. A ring with one body on it is
/// that body's; a shared orbit's targets its members together, and has a
/// target per member for its tail, for when the frame body is on it.
fn spawn_ring_targets(mut commands: Commands, rings: OrbitRings) {
    for owner in rings.owners() {
        let Some(ring) = rings.ring(owner) else {
            continue;
        };
        if ring.shared {
            commands
                .spawn((RingTarget(owner), OrbitFor(owner)))
                .observe(hover::on_marker_over)
                .observe(hover::on_marker_out)
                .observe(hover::on_orbit_click);
            for member in &ring.members {
                commands
                    .spawn((
                        MemberTailTarget {
                            orbit: owner,
                            body: member.body,
                        },
                        MarkerFor(member.body),
                    ))
                    .observe(hover::on_marker_over)
                    .observe(hover::on_marker_out)
                    .observe(hover::on_marker_click);
            }
        } else if let Some(member) = ring.members.first() {
            commands
                .spawn((RingTarget(owner), MarkerFor(member.body)))
                .observe(hover::on_marker_over)
                .observe(hover::on_marker_out)
                .observe(hover::on_marker_click);
        }
    }
}

/// The arc of a body's orbit that is drawn, as (start angle, sweep) in
/// radians, running back from `body_angle` for a tail.
pub fn drawn_arc(extent: RingExtent, body_angle: f32, tail_degrees: f32) -> Option<(f32, f32)> {
    match extent {
        RingExtent::Hidden => None,
        RingExtent::Full => Some((0.0, std::f32::consts::TAU)),
        RingExtent::Tail if tail_degrees > 0.0 => {
            let sweep = tail_degrees.to_radians();
            Some((body_angle - sweep, sweep))
        }
        RingExtent::Tail => None,
    }
}

/// HOV-04: the arcs drawn on a ring with bodies at `body_angles`: the whole
/// ring once, or one tail behind each body.
pub fn drawn_arcs(extent: RingExtent, body_angles: &[f32], tail_degrees: f32) -> Vec<(f32, f32)> {
    match extent {
        RingExtent::Tail => body_angles
            .iter()
            .filter_map(|&angle| drawn_arc(extent, angle, tail_degrees))
            .collect(),
        _ => drawn_arc(extent, 0.0, tail_degrees).into_iter().collect(),
    }
}

type RingTargets<'w, 's> = Query<'w, 's, (Entity, &'static RingTarget)>;
type TailTargets<'w, 's> = Query<'w, 's, (Entity, &'static MemberTailTarget)>;

/// HOV-02: trace every drawn ring and tail on screen. A folded satellite's
/// ring isn't drawn, so it has no path (GRP-07: its pill stands for it),
/// and a ring carrying the frame body is left out, as is one faded out by
/// its size (MRK-14). On a shared ring carrying the frame body, each other
/// drawn member's tail is traced for that member instead.
#[allow(clippy::too_many_arguments)]
fn trace_rings(
    view: ViewNow,
    rings: OrbitRings,
    (frame, selection, hovered): (Res<Frame>, Res<Selection>, Res<HoveredBodies>),
    (screen, tunables): (Res<ScreenPositions>, Res<Tunables>),
    camera_q: Query<(&Camera, &Transform, &Projection), With<Camera3d>>,
    (targets, tail_targets): (RingTargets, TailTargets),
    mut paths: ResMut<RingPaths>,
) {
    paths.0.clear();
    let frame_body = frame.non_star_body();
    let fade = camera_q
        .single()
        .ok()
        .and_then(|camera| FadeView::new(camera, &screen, &tunables));
    for (target, ring_target) in &targets {
        let Some(ring) = rings.ring(ring_target.0) else {
            continue;
        };
        if ring.carries(frame_body) {
            continue;
        }
        if faded_out(&ring, fade, &view) {
            continue;
        }
        let angles: Vec<f32> = ring.drawn().map(|m| m.angle).collect();
        let mode = ring.mode();
        let (selected, is_hovered) = ring.highlight(&selection, &hovered);
        let extent = ring_extent(
            ring.class(),
            angles.is_empty(),
            mode.ring_mode,
            selected || is_hovered,
        );
        let segments = trace_arcs(
            &ring,
            &drawn_arcs(extent, &angles, mode.tail_degrees),
            &view,
        );
        if !segments.is_empty() {
            paths.0.insert(target, segments);
        }
    }
    for (target, tail) in &tail_targets {
        let Some(ring) = rings.ring(tail.orbit) else {
            continue;
        };
        if !ring.carries(frame_body) || frame_body == Some(tail.body) {
            continue;
        }
        let mode = ring.mode();
        let Some(member) = ring.drawn().find(|m| m.body == tail.body) else {
            continue;
        };
        if mode.ring_mode || faded_out(&ring, fade, &view) {
            continue;
        }
        let arc = drawn_arc(RingExtent::Tail, member.angle, mode.tail_degrees);
        let segments = trace_arcs(&ring, &Vec::from_iter(arc), &view);
        if !segments.is_empty() {
            paths.0.insert(target, segments);
        }
    }
}

/// MRK-14: whether a ring is too large on screen to be drawn.
fn faded_out(ring: &Ring, fade: Option<FadeView>, view: &ViewNow) -> bool {
    let centre = match ring.centre {
        Some(parent) => view.position(parent),
        None => Some(view.star()),
    };
    fade.zip(centre)
        .is_some_and(|(fade, centre)| fade.of(centre, km_to_render(ring.radius_km)) <= 0.0)
}

/// The on-screen segments along `arcs` of a ring, each (start, sweep).
fn trace_arcs(ring: &Ring, arcs: &[(f32, f32)], view: &ViewNow) -> Vec<(Vec2, Vec2)> {
    arcs.iter()
        .flat_map(|&(start, sweep)| {
            let point = |i: usize| {
                let angle = start + i as f32 / RING_TRACE_SEGMENTS as f32 * sweep;
                ring_screen_point(ring, angle, view)
            };
            // A point the camera can't project breaks the ring there,
            // rather than joining across the gap.
            (0..RING_TRACE_SEGMENTS)
                .filter_map(|i| Some((point(i)?, point(i + 1)?)))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The backend: report a hit on a ring's target for each pointer near it.
fn pick_rings(
    tunables: Res<Tunables>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_q: Query<(Entity, &Camera), With<IsDefaultUiCamera>>,
    paths: Res<RingPaths>,
    mut output: MessageWriter<PointerHits>,
) {
    // Hits are reported against the overlay's camera, as the markers'
    // are, so the two orderings compare.
    let Ok((camera_entity, camera)) = camera_q.single() else {
        return;
    };
    for (pointer, location) in &pointers {
        let Some(location) = &location.location else {
            continue;
        };
        let Some((target, distance)) = ring_under(location.position, &paths, tunables.ring_hit_px)
        else {
            continue;
        };
        let hit = HitData::new(camera_entity, distance, None, None);
        // Below the markers (+0.25) and the UI (+0.5), so they win.
        output.write(PointerHits::new(
            *pointer,
            vec![(target, hit)],
            camera.order as f32,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default `ring_hit_px`.
    const RING_HIT_PX: f32 = 7.0;

    /// A ring traced as a square of four segments round the origin.
    fn square(world: &mut World, half: f32) -> (Entity, Vec<(Vec2, Vec2)>) {
        let corners = [
            Vec2::new(-half, -half),
            Vec2::new(half, -half),
            Vec2::new(half, half),
            Vec2::new(-half, half),
        ];
        let segments = (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect();
        (world.spawn_empty().id(), segments)
    }

    #[test]
    fn distance_to_segments_measures_to_the_nearest_point() {
        let segment = [(Vec2::ZERO, Vec2::new(10.0, 0.0))];
        assert_eq!(distance_to_segments(Vec2::new(5.0, 3.0), &segment), 3.0);
        // Past the end, the distance is to the end point.
        assert_eq!(distance_to_segments(Vec2::new(13.0, 4.0), &segment), 5.0);
        assert!(distance_to_segments(Vec2::ZERO, &[]).is_infinite());
        // A degenerate segment is a point.
        assert_eq!(
            distance_to_segments(Vec2::new(3.0, 4.0), &[(Vec2::ZERO, Vec2::ZERO)]),
            5.0
        );
    }

    /// HOV-02: a ring is hittable anywhere along its length, not
    /// only at a few points round it.
    #[test]
    fn a_ring_is_hit_anywhere_along_it() {
        let mut world = World::new();
        let (ring, segments) = square(&mut world, 200.0);
        let mut paths = RingPaths::default();
        paths.0.insert(ring, segments);
        // Every point along an edge, including midway between "corners".
        for x in [-190.0_f32, -73.0, 0.0, 41.5, 199.0] {
            let hit = ring_under(Vec2::new(x, 203.0), &paths, RING_HIT_PX);
            assert_eq!(hit.map(|(e, _)| e), Some(ring), "missed at x = {x}");
        }
        // But not in the middle of the ring, or just outside the reach.
        assert_eq!(ring_under(Vec2::ZERO, &paths, RING_HIT_PX), None);
        assert_eq!(ring_under(Vec2::new(0.0, 210.0), &paths, RING_HIT_PX), None);
    }

    /// HOV-02 / MRK-09: a full ring is traced all round; a tail runs back
    /// from the body for its length; nothing undrawn is traced.
    #[test]
    fn only_the_drawn_arc_is_traced() {
        use std::f32::consts::TAU;
        assert_eq!(drawn_arc(RingExtent::Full, 1.0, 30.0), Some((0.0, TAU)));
        let (start, sweep) = drawn_arc(RingExtent::Tail, 1.0, 90.0).unwrap();
        assert!((sweep - TAU / 4.0).abs() < 1e-6);
        assert!(
            (start + sweep - 1.0).abs() < 1e-6,
            "the tail ends at the body"
        );
        assert_eq!(drawn_arc(RingExtent::Tail, 1.0, 0.0), None);
        assert_eq!(drawn_arc(RingExtent::Hidden, 1.0, 90.0), None);
    }

    /// HOV-02 / HOV-04: a shared ring in dot mode is traced as one tail
    /// behind each body on it; in ring mode or highlighted, once all round.
    #[test]
    fn a_shared_ring_traces_a_tail_behind_each_body() {
        use std::f32::consts::TAU;
        let tails = drawn_arcs(RingExtent::Tail, &[1.0, 2.5], 90.0);
        assert_eq!(tails.len(), 2);
        assert!((tails[1].0 + tails[1].1 - 2.5).abs() < 1e-6);
        assert_eq!(
            drawn_arcs(RingExtent::Full, &[1.0, 2.5], 90.0),
            [(0.0, TAU)]
        );
        assert!(drawn_arcs(RingExtent::Hidden, &[1.0], 90.0).is_empty());
        assert!(drawn_arcs(RingExtent::Tail, &[], 90.0).is_empty());
    }

    /// HOV-02: where two rings are both within reach, the closer wins.
    #[test]
    fn the_nearer_of_two_rings_wins() {
        let mut world = World::new();
        let (inner, inner_segments) = square(&mut world, 100.0);
        let (outer, outer_segments) = square(&mut world, 106.0);
        let mut paths = RingPaths::default();
        paths.0.insert(inner, inner_segments);
        paths.0.insert(outer, outer_segments);
        let near_inner = ring_under(Vec2::new(0.0, 101.0), &paths, RING_HIT_PX);
        assert_eq!(near_inner.map(|(e, _)| e), Some(inner));
        let near_outer = ring_under(Vec2::new(0.0, 105.0), &paths, RING_HIT_PX);
        assert_eq!(near_outer.map(|(e, _)| e), Some(outer));
    }
}
