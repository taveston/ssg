//! Hover and click on an orbit ring (HOV-02, HOV-05): a small picking
//! backend that hits any drawn ring, tail or exposure ring anywhere along
//! its length, by distance from it as drawn on screen. Only what is drawn
//! is a target: a ship's tail in dot mode, not the rest of its orbit
//! (MRK-09), and not a ring faded out by its size (MRK-14). A ring
//! carrying the frame body is never a target; on a shared ring, the other
//! members' tails still target those members.
//!
//! Gizmo-drawn rings aren't pickable on their own. Each ring gets one
//! invisible target entity, kept in step with its members as ships leave
//! and arrive. A natural body's ring, or a named orbit with
//! one member, stands for that body (`MarkerFor`) with the ordinary marker
//! observers, so a hit behaves exactly like a hit on the marker (white
//! ring, select, double-click). A shared orbit's ring stands for the orbit
//! (`OrbitFor`): hovering it highlights the ring and its identity mark,
//! and clicking it selects the parent with that orbit's row revealed. Hits
//! are reported below the markers' and the UI's order, so a marker,
//! multi-marker, label or panel still wins. A thing joined into a
//! multi-marker draws no tail of its own, so it has none to hit, but the
//! first tray row of a together or overlap marker lays the marker's tail,
//! which is its own to hit (MUL-05).

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::picking::PickingSystems;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use super::{FadeView, RingExtent, ring_extent};
use crate::view::frame::Frame;
use crate::view::hover;
use crate::view::hovered::HoveredBodies;
use crate::view::multi_markers::MultiGathered;
use crate::view::scale::km_to_render;
use crate::view::screen::ScreenPositions;
use crate::view::selection::Selection;
use crate::view::shared_orbit::{OrbitRings, Ring, ring_screen_point, rings_changed};
use crate::view::targets::{MarkerFor, OrbitFor};
use crate::view::tunables::Tunables;
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
            .add_systems(Startup, sync_ring_targets.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (sync_ring_targets.run_if(rings_changed), trace_rings)
                    .chain()
                    .after(MultiGathered)
                    .in_set(Stage::Overlay),
            )
            .add_systems(PreUpdate, pick_rings.in_set(PickingSystems::Backend));
    }
}

/// What a ring's hit target stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TargetFor {
    /// A shared orbit's ring, for the orbit.
    Orbit(Entity),
    /// A ring with one body on it, for that body: (owner, body).
    Body(Entity, Entity),
    /// One member's tail on a shared orbit's ring: (orbit, body).
    Tail(Entity, Entity),
}

/// HOV-02 / HOV-05: the targets the rings want: one per ring. A ring with
/// one body on it is that body's; a shared orbit's targets its members
/// together, and has a target per member for its tail, for when the frame
/// body is on it.
fn wanted_targets(rings: &OrbitRings) -> Vec<TargetFor> {
    let mut wanted = Vec::new();
    for owner in rings.owners() {
        let Some(ring) = rings.ring(owner) else {
            continue;
        };
        if ring.shared {
            wanted.push(TargetFor::Orbit(owner));
            wanted.extend(ring.members.iter().map(|m| TargetFor::Tail(owner, m.body)));
        } else if let Some(member) = ring.members.first() {
            wanted.push(TargetFor::Body(owner, member.body));
        }
    }
    wanted
}

/// The ring targets there are, with what each stands for.
type ExistingTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static RingTarget>,
        Option<&'static MemberTailTarget>,
        Option<&'static MarkerFor>,
    ),
    Or<(With<RingTarget>, With<MemberTailTarget>)>,
>;

/// HOV-02 / HOV-05 / RAIL-09: keep one target per ring as ships come and
/// go: a ship arriving in an orbit is reached through its ring, one that
/// leaves no longer is, and a ring that turns shared or lone again swaps
/// its target. Targets that still stand for the same thing are kept.
fn sync_ring_targets(mut commands: Commands, rings: OrbitRings, existing: ExistingTargets) {
    let wanted = wanted_targets(&rings);
    let mut have = std::collections::HashSet::new();
    for (entity, ring, tail, marker_for) in &existing {
        let stands_for = match (ring, tail, marker_for) {
            (Some(ring), _, Some(body)) => TargetFor::Body(ring.0, body.0),
            (Some(ring), _, None) => TargetFor::Orbit(ring.0),
            (None, Some(tail), _) => TargetFor::Tail(tail.orbit, tail.body),
            (None, None, _) => continue,
        };
        if wanted.contains(&stands_for) && have.insert(stands_for) {
            continue;
        }
        commands.entity(entity).despawn();
    }
    for target in wanted.into_iter().filter(|t| !have.contains(t)) {
        spawn_target(&mut commands, target);
    }
}

/// One ring target, with the observers that make a hit on it behave as a
/// hit on what it stands for.
fn spawn_target(commands: &mut Commands, target: TargetFor) {
    let mut entity = match target {
        TargetFor::Orbit(orbit) => {
            let mut entity = commands.spawn((RingTarget(orbit), OrbitFor(orbit)));
            entity.observe(hover::on_orbit_click);
            entity
        }
        TargetFor::Body(owner, body) => {
            let mut entity = commands.spawn((RingTarget(owner), MarkerFor(body)));
            entity.observe(hover::on_marker_click);
            entity
        }
        TargetFor::Tail(orbit, body) => {
            let mut entity = commands.spawn((MemberTailTarget { orbit, body }, MarkerFor(body)));
            entity.observe(hover::on_marker_click);
            entity
        }
    };
    entity
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out);
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

/// What tracing the rings reads: this frame's view, the rings, the frame,
/// the selection and hover, and the camera for the fade by size.
#[derive(SystemParam)]
struct RingTrace<'w, 's> {
    view: ViewNow<'w, 's>,
    rings: OrbitRings<'w, 's>,
    frame: Res<'w, Frame>,
    selection: Res<'w, Selection>,
    hovered: Res<'w, HoveredBodies>,
    screen: Res<'w, ScreenPositions>,
    tunables: Res<'w, Tunables>,
    cameras:
        Query<'w, 's, (&'static Camera, &'static Transform, &'static Projection), With<Camera3d>>,
}

impl RingTrace<'_, '_> {
    /// MRK-14: how rings fade by their size on screen this frame.
    fn fade(&self) -> Option<FadeView> {
        self.cameras
            .single()
            .ok()
            .and_then(|camera| FadeView::new(camera, &self.screen, &self.tunables))
    }

    /// The segments of `owner`'s ring as drawn: none for a ring carrying
    /// the frame body or faded out by its size.
    fn ring_segments(&self, owner: Entity, fade: Option<FadeView>) -> Vec<(Vec2, Vec2)> {
        let frame_body = self.frame.non_star_body();
        let Some(ring) = self.rings.ring(owner) else {
            return Vec::new();
        };
        if ring.carries(frame_body) || faded_out(&ring, fade, &self.view) {
            return Vec::new();
        }
        let angles = ring.tail_angles();
        let mode = ring.mode();
        let (selected, is_hovered) = ring.highlight(&self.selection, &self.hovered);
        let extent = ring_extent(
            ring.class(),
            ring.drawn().next().is_none(),
            mode.ring_mode,
            selected || is_hovered,
        );
        trace_arcs(
            &ring,
            &drawn_arcs(extent, &angles, mode.tail_degrees),
            &self.view,
        )
    }

    /// The segments of one member's tail on a shared ring, only while the
    /// frame body is another member of it.
    fn tail_segments(&self, tail: &MemberTailTarget, fade: Option<FadeView>) -> Vec<(Vec2, Vec2)> {
        let frame_body = self.frame.non_star_body();
        let Some(ring) = self.rings.ring(tail.orbit) else {
            return Vec::new();
        };
        if !ring.carries(frame_body) || frame_body == Some(tail.body) {
            return Vec::new();
        }
        let mode = ring.mode();
        let Some(member) = ring.tails().find(|m| m.body == tail.body) else {
            return Vec::new();
        };
        if mode.ring_mode || faded_out(&ring, fade, &self.view) {
            return Vec::new();
        }
        let arc = drawn_arc(RingExtent::Tail, member.tail_head, mode.tail_degrees);
        trace_arcs(&ring, &Vec::from_iter(arc), &self.view)
    }
}

/// HOV-02: trace every drawn ring and tail on screen. A folded satellite's
/// ring isn't drawn, so it has no path (GRP-07: its fold marker stands for it),
/// and a ring carrying the frame body is left out, as is one faded out by
/// its size (MRK-14). On a shared ring carrying the frame body, each other
/// drawn member's tail is traced for that member instead.
fn trace_rings(
    trace: RingTrace,
    targets: RingTargets,
    tail_targets: TailTargets,
    mut paths: ResMut<RingPaths>,
) {
    paths.0.clear();
    let fade = trace.fade();
    for (target, ring_target) in &targets {
        let segments = trace.ring_segments(ring_target.0, fade);
        if !segments.is_empty() {
            paths.0.insert(target, segments);
        }
    }
    for (target, tail) in &tail_targets {
        let segments = trace.tail_segments(tail, fade);
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
