//! Hover and click on markers (MRK-08): a small picking backend that hits
//! a marker anywhere within `min_hit_radius_px` of its centre (or its drawn
//! size, if larger), so small markers stay easy to click. Where hit areas
//! overlap, the marker whose centre is nearest the pointer wins.
//!
//! Each body with a marker gets one invisible target entity carrying
//! `MarkerFor` and the ordinary marker observers. The drawn marker node
//! itself takes no clicks, so the UI's rectangle test can't override the
//! nearest-centre rule. Hits are reported above orbit rings and below the
//! UI, so pills, labels and panels still win (HOV-02).

use bevy::picking::PickingSystems;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use super::hover;
use super::markers::MarkerHalfSize;
use super::targets::MarkerFor;
use super::tunables::Tunables;
use crate::sim::body::BodyClass;
use crate::{Boot, Stage};

/// Between the rings' order (the camera's) and the UI's (+0.5), so a
/// marker beats a ring and a pill, label or panel beats a marker (HOV-02).
const MARKER_ORDER_OFFSET: f32 = 0.25;
const _: () = assert!(MARKER_ORDER_OFFSET > 0.0 && MARKER_ORDER_OFFSET < 0.5);

/// The invisible hit target standing for a body's marker.
#[derive(Component)]
pub struct MarkerTarget;

/// One marker's hit area this frame: its target, centre and reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitSpot {
    pub target: Entity,
    pub centre: Vec2,
    pub radius: f32,
}

/// Every shown marker's hit area, rebuilt each frame.
#[derive(Resource, Default)]
pub struct MarkerSpots(pub Vec<HitSpot>);

/// MRK-08: a marker's reach - its drawn half-size, but never less than
/// `min_hit_radius_px`.
pub fn hit_radius(drawn_half: f32, min_hit_radius_px: f32) -> f32 {
    drawn_half.max(min_hit_radius_px)
}

/// MRK-08: the marker hit at `point`: of those whose reach covers it, the
/// one whose centre is nearest.
pub fn marker_under(point: Vec2, spots: &[HitSpot]) -> Option<(Entity, f32)> {
    spots
        .iter()
        .map(|spot| (spot.target, spot.centre.distance(point), spot.radius))
        .filter(|&(_, distance, radius)| distance <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(target, distance, _)| (target, distance))
}

pub struct MarkerPickingPlugin;

impl Plugin for MarkerPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarkerSpots>()
            .add_systems(Startup, spawn_marker_targets.in_set(Boot::Overlay))
            .add_systems(Update, find_marker_spots.in_set(Stage::Overlay))
            .add_systems(PreUpdate, pick_markers.in_set(PickingSystems::Backend));
    }
}

/// One target per body that has a marker.
fn spawn_marker_targets(mut commands: Commands, bodies: Query<Entity, With<BodyClass>>) {
    for body in &bodies {
        commands
            .spawn((MarkerTarget, MarkerFor(body)))
            .observe(hover::on_marker_over)
            .observe(hover::on_marker_out)
            .observe(hover::on_marker_click);
    }
}

/// MRK-08: where each shown marker is, from its node as laid out this
/// frame, matched to its body's target.
fn find_marker_spots(
    tunables: Res<Tunables>,
    markers: Query<(&MarkerFor, &Node), With<MarkerHalfSize>>,
    targets: Query<(Entity, &MarkerFor), With<MarkerTarget>>,
    mut spots: ResMut<MarkerSpots>,
) {
    let target_of: std::collections::HashMap<Entity, Entity> = targets
        .iter()
        .map(|(target, for_)| (for_.0, target))
        .collect();
    spots.0.clear();
    for (marker_for, node) in &markers {
        let (Display::Flex, Val::Px(left), Val::Px(top), Val::Px(width)) =
            (node.display, node.left, node.top, node.width)
        else {
            continue;
        };
        let Some(&target) = target_of.get(&marker_for.0) else {
            continue;
        };
        let half = width / 2.0;
        spots.0.push(HitSpot {
            target,
            centre: Vec2::new(left + half, top + half),
            radius: hit_radius(half, tunables.min_hit_radius_px),
        });
    }
}

/// The backend: report a hit on the nearest marker for each pointer.
fn pick_markers(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_q: Query<(Entity, &Camera)>,
    spots: Res<MarkerSpots>,
    mut output: MessageWriter<PointerHits>,
) {
    let Ok((camera_entity, camera)) = camera_q.single() else {
        return;
    };
    for (pointer, location) in &pointers {
        let Some(location) = &location.location else {
            continue;
        };
        let Some((target, distance)) = marker_under(location.position, &spots.0) else {
            continue;
        };
        let hit = HitData::new(camera_entity, distance, None, None);
        output.write(PointerHits::new(
            *pointer,
            vec![(target, hit)],
            camera.order as f32 + MARKER_ORDER_OFFSET,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(world: &mut World, centre: Vec2, radius: f32) -> HitSpot {
        HitSpot {
            target: world.spawn_empty().id(),
            centre,
            radius,
        }
    }

    /// MRK-08: a 3 px moon is still hit 10 px from its centre, not 11.
    #[test]
    fn a_small_marker_has_the_minimum_reach() {
        let mut world = World::new();
        let moon = spot(&mut world, Vec2::ZERO, hit_radius(3.0, 10.0));
        assert_eq!(
            marker_under(Vec2::new(10.0, 0.0), &[moon]).map(|h| h.0),
            Some(moon.target)
        );
        assert_eq!(marker_under(Vec2::new(11.0, 0.0), &[moon]), None);
        assert_eq!(hit_radius(15.0, 10.0), 15.0, "a big marker keeps its size");
    }

    /// MRK-08: where two reaches overlap, the nearer centre wins, whatever
    /// the order.
    #[test]
    fn the_nearest_centre_wins() {
        let mut world = World::new();
        let a = spot(&mut world, Vec2::ZERO, 10.0);
        let b = spot(&mut world, Vec2::new(12.0, 0.0), 10.0);
        for spots in [[a, b], [b, a]] {
            assert_eq!(
                marker_under(Vec2::new(5.0, 0.0), &spots).map(|h| h.0),
                Some(a.target)
            );
            assert_eq!(
                marker_under(Vec2::new(7.0, 0.0), &spots).map(|h| h.0),
                Some(b.target)
            );
        }
    }
}
