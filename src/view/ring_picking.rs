//! Hover and click on an exposure ring (MOT-05, F4-14): a small picking
//! backend that hits a ring anywhere along its length, by distance from
//! the ring as drawn on screen.
//!
//! Gizmo-drawn rings aren't pickable on their own. Each orbiting body gets
//! one invisible target entity carrying `MarkerFor` and the ordinary marker
//! observers, so a ring hit behaves exactly like a hit on the body's marker
//! (tooltip, select, double-click). Hits are reported below the UI's order,
//! so a marker or panel over the ring still wins.

use std::collections::HashMap;

use bevy::picking::PickingSystems;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use super::hover;
use super::motion::{RingBodies, ring_point, ring_strength};
use super::targets::MarkerFor;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::world::ViewNow;
use crate::{Boot, Stage};

/// How close to a ring the pointer must be to hit it, in pixels: the same
/// reach as the 14 px hit squares this replaces.
const RING_HIT_TOLERANCE_PX: f32 = 7.0;
/// Straight segments a ring is traced with for hit-testing.
const RING_TRACE_SEGMENTS: usize = 64;

/// The invisible hit target standing for a body's exposure ring.
#[derive(Component)]
pub struct RingTarget;

/// Each active ring's on-screen segments, by its target entity. Rebuilt
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

/// MOT-05: the ring under `point`, if any is within `tolerance`: the
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

/// One target per orbiting body; gates keep their edge treatment instead.
fn spawn_ring_targets(mut commands: Commands, bodies: Query<(Entity, &BodyClass), With<Orbit>>) {
    for (body, class) in &bodies {
        if *class == BodyClass::Gate {
            continue;
        }
        commands
            .spawn((RingTarget, MarkerFor(body)))
            .observe(hover::on_marker_over)
            .observe(hover::on_marker_out)
            .observe(hover::on_marker_click);
    }
}

/// MOT-05: trace every active exposure ring on screen.
fn trace_rings(
    view: ViewNow,
    bodies: RingBodies,
    targets: Query<(Entity, &MarkerFor), With<RingTarget>>,
    mut paths: ResMut<RingPaths>,
) {
    paths.0.clear();
    for (target, marker_for) in &targets {
        let body = marker_for.0;
        if ring_strength(body, &bodies).is_none() {
            continue;
        }
        let point = |i: usize| {
            let angle = i as f32 / RING_TRACE_SEGMENTS as f32 * std::f32::consts::TAU;
            ring_point(body, angle, &bodies, &view)
        };
        // A point the camera can't project breaks the ring there, rather
        // than joining across the gap.
        let segments: Vec<(Vec2, Vec2)> = (0..RING_TRACE_SEGMENTS)
            .filter_map(|i| Some((point(i)?, point(i + 1)?)))
            .collect();
        paths.0.insert(target, segments);
    }
}

/// The backend: report a hit on a ring's target for each pointer near it.
fn pick_rings(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_q: Query<(Entity, &Camera)>,
    paths: Res<RingPaths>,
    mut output: MessageWriter<PointerHits>,
) {
    let Ok((camera_entity, camera)) = camera_q.single() else {
        return;
    };
    for (pointer, location) in &pointers {
        let Some(location) = &location.location else {
            continue;
        };
        let Some((target, distance)) = ring_under(location.position, &paths, RING_HIT_TOLERANCE_PX)
        else {
            continue;
        };
        let hit = HitData::new(camera_entity, distance, None, None);
        // Below the UI (camera order + 0.5), so markers and panels win.
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

    /// MOT-05 (F4-14): a ring is hittable anywhere along its length, not
    /// only at a few points round it.
    #[test]
    fn mot_05_a_ring_is_hit_anywhere_along_it() {
        let mut world = World::new();
        let (ring, segments) = square(&mut world, 200.0);
        let mut paths = RingPaths::default();
        paths.0.insert(ring, segments);
        // Every point along an edge, including midway between "corners".
        for x in [-190.0_f32, -73.0, 0.0, 41.5, 199.0] {
            let hit = ring_under(Vec2::new(x, 203.0), &paths, RING_HIT_TOLERANCE_PX);
            assert_eq!(hit.map(|(e, _)| e), Some(ring), "missed at x = {x}");
        }
        // But not in the middle of the ring, or just outside the reach.
        assert_eq!(ring_under(Vec2::ZERO, &paths, RING_HIT_TOLERANCE_PX), None);
        assert_eq!(
            ring_under(Vec2::new(0.0, 210.0), &paths, RING_HIT_TOLERANCE_PX),
            None
        );
    }

    /// MOT-05: where two rings are both within reach, the closer wins.
    #[test]
    fn mot_05_the_nearer_of_two_rings_wins() {
        let mut world = World::new();
        let (inner, inner_segments) = square(&mut world, 100.0);
        let (outer, outer_segments) = square(&mut world, 106.0);
        let mut paths = RingPaths::default();
        paths.0.insert(inner, inner_segments);
        paths.0.insert(outer, outer_segments);
        let near_inner = ring_under(Vec2::new(0.0, 101.0), &paths, RING_HIT_TOLERANCE_PX);
        assert_eq!(near_inner.map(|(e, _)| e), Some(inner));
        let near_outer = ring_under(Vec2::new(0.0, 105.0), &paths, RING_HIT_TOLERANCE_PX);
        assert_eq!(near_outer.map(|(e, _)| e), Some(outer));
    }
}
