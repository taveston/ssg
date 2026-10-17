//! Hover and click on markers (MRK-08): a small picking backend that hits
//! a marker anywhere within `min_hit_radius_px` of its centre (or its drawn
//! size, if larger, so a true-size disc is hit anywhere on it), so small
//! markers stay easy to click. Where hit areas overlap, the marker whose
//! centre is nearest the pointer wins.
//!
//! Each body with a marker gets one invisible target entity carrying
//! `MarkerFor` and the ordinary marker observers. The drawn marker node
//! itself takes no clicks, so the UI's rectangle test can't override the
//! nearest-centre rule. A multi-marker is hit anywhere on either of its
//! two frames too (MUL-11), through its own target (`view/multi.rs`).
//! Hits are reported above orbit rings and below the UI, so labels and
//! panels still win (HOV-02).

use bevy::picking::PickingSystems;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use super::MarkerHalfSize;
use crate::sim::body::BodyClass;
use crate::view::hover;
use crate::view::multi::{BackFrame, MultiNode};
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;
use crate::{Boot, Stage};

/// Between the rings' order (the camera's) and the UI's (+0.5), so a
/// marker beats a ring and a label or panel beats a marker (HOV-02).
const MARKER_ORDER_OFFSET: f32 = 0.25;
const _: () = assert!(MARKER_ORDER_OFFSET > 0.0 && MARKER_ORDER_OFFSET < 0.5);

/// The invisible hit target standing for a body's marker.
#[derive(Component)]
pub struct MarkerTarget;

/// One marker's hit area this frame: its target, centre and reach, and
/// for a multi-marker its frames' squares (MUL-11): `half` from the
/// centre, and again round the back frame, `back` from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitSpot {
    pub target: Entity,
    pub centre: Vec2,
    pub radius: f32,
    pub frames: Option<(f32, Vec2)>,
}

impl HitSpot {
    /// MRK-08 / MUL-11: whether `point` is in reach: within the radius,
    /// or on either of a multi-marker's frames.
    fn covers(&self, point: Vec2) -> bool {
        let in_square = |centre: Vec2, half: f32| {
            let d = (point - centre).abs();
            d.x <= half && d.y <= half
        };
        self.centre.distance(point) <= self.radius
            || self.frames.is_some_and(|(half, back)| {
                in_square(self.centre, half) || in_square(self.centre + back, half)
            })
    }
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
        .filter(|spot| spot.covers(point))
        .map(|spot| (spot.target, spot.centre.distance(point)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub struct MarkerPickingPlugin;

impl Plugin for MarkerPickingPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<MarkerSpots>()
            .add_load_systems(Boot::Overlay, spawn_marker_targets)
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

/// The drawn multi-markers, by node, with their targets.
type MultiNodes<'w, 's> = Query<'w, 's, (&'static MultiNode, &'static Node, &'static BackFrame)>;

/// MRK-08 / MUL-11: where each shown marker is, from its node as laid out
/// this frame, matched to its body's target; and each multi-marker, hit on
/// either frame. A station marker's back frame is part of its hit area.
fn find_marker_spots(
    tunables: Res<Tunables>,
    markers: Query<(&MarkerFor, &Node, Option<&BackFrame>), With<MarkerHalfSize>>,
    multis: MultiNodes,
    targets: Query<(Entity, &MarkerFor), With<MarkerTarget>>,
    mut spots: ResMut<MarkerSpots>,
) {
    let target_of: std::collections::HashMap<Entity, Entity> = targets
        .iter()
        .map(|(target, for_)| (for_.0, target))
        .collect();
    spots.0.clear();
    for (marker_for, node, back) in &markers {
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
            frames: back.and_then(|b| b.0).map(|b| (half, b)),
        });
    }
    // A fold marker's pip is drawn small, but is hit at least
    // `min_hit_radius_px` round its centre all the same (GRP-12).
    for (multi, node, back) in &multis {
        let (Display::Flex, Val::Px(left), Val::Px(top), Val::Px(width)) =
            (node.display, node.left, node.top, node.width)
        else {
            continue;
        };
        let half = width / 2.0;
        spots.0.push(HitSpot {
            target: multi.target,
            centre: Vec2::new(left + half, top + half),
            radius: hit_radius(half, tunables.min_hit_radius_px),
            frames: Some((half, back.0.unwrap_or_default())),
        });
    }
}

/// The backend: report a hit on the nearest marker for each pointer.
/// Hits are reported against the overlay's camera, which draws the
/// markers, as the rings' and the UI's are, so the orderings compare.
fn pick_markers(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_q: Query<(Entity, &Camera), With<IsDefaultUiCamera>>,
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
            frames: None,
        }
    }

    /// MUL-11: a multi-marker is hit anywhere on its front or back frame
    /// (each `ship_symbol_px` square), and at least `min_hit_radius_px`
    /// round the front frame's centre; not beside the frames.
    #[test]
    fn a_multi_marker_is_hit_on_either_frame() {
        let mut world = World::new();
        let t = Tunables::default();
        let half = t.ship_symbol_px / 2.0;
        let back = crate::view::multi::back_offset(t.back_frame_offset_px);
        let multi = HitSpot {
            frames: Some((half, back)),
            ..spot(
                &mut world,
                Vec2::ZERO,
                hit_radius(half, t.min_hit_radius_px),
            )
        };
        let hit = |p: Vec2| marker_under(p, &[multi]).map(|h| h.0);
        // The front frame's corner, past the round reach.
        assert_eq!(hit(Vec2::new(-half, half)), Some(multi.target));
        // The back frame's far corner.
        assert_eq!(hit(back + Vec2::new(half, -half)), Some(multi.target));
        // Beside both frames.
        assert_eq!(hit(Vec2::new(-half - 1.0, 0.0)), None);
        assert_eq!(hit(Vec2::new(0.0, half + 1.0)), None);
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

    /// MRK-08 / MRK-07 / MRK-11: with the game's two cameras (map and
    /// overlay), a body drawn as an 80 px true-size disc is hit anywhere
    /// on the disc out to the outer edge of its rim (the node's edge, as
    /// the rim is drawn inside it), and not beyond; the hit sits above the
    /// overlay camera's rings and below its UI.
    #[test]
    fn a_true_size_disc_is_hit_out_to_its_rim() {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::ecs::system::RunSystemOnce;
        use bevy::picking::pointer::{Location, PointerMap};

        let mut app = App::new();
        app.add_message::<PointerHits>()
            .init_resource::<PointerMap>()
            .reset_with_system::<MarkerSpots>()
            .init_resource::<Tunables>();
        let world = app.world_mut();
        world
            .run_system_once(crate::view::camera::rig::spawn_camera)
            .unwrap();
        let body = world.spawn_empty().id();
        let target = world.spawn((MarkerTarget, MarkerFor(body))).id();
        let centre = Vec2::new(400.0, 300.0);
        world.spawn((
            MarkerFor(body),
            MarkerHalfSize(Vec2::splat(6.0)),
            Node {
                display: Display::Flex,
                left: px(centre.x - 40.0),
                top: px(centre.y - 40.0),
                width: px(80.0),
                height: px(80.0),
                ..default()
            },
        ));
        world.spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target: NormalizedRenderTarget::None {
                    width: 1600,
                    height: 900,
                },
                position: centre + Vec2::new(30.0, 0.0),
            }),
        ));
        world.run_system_once(find_marker_spots).unwrap();
        world.run_system_once(pick_markers).unwrap();

        let hits: Vec<PointerHits> = world
            .resource_mut::<Messages<PointerHits>>()
            .drain()
            .collect();
        assert_eq!(hits.len(), 1, "one hit for the one pointer");
        assert_eq!(hits[0].picks[0].0, target);
        let mut ui = world.query_filtered::<&Camera, With<IsDefaultUiCamera>>();
        let ui_order = ui.single(world).unwrap().order as f32;
        assert!(hits[0].order > ui_order && hits[0].order < ui_order + 0.5);

        // On the rim's outer edge, and just past it.
        let spots = &world.resource::<MarkerSpots>().0;
        let rim = centre + Vec2::new(0.0, 40.0);
        assert_eq!(marker_under(rim, spots).map(|h| h.0), Some(target));
        assert_eq!(marker_under(rim + Vec2::new(0.0, 0.5), spots), None);
    }
}
