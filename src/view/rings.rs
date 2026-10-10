//! Orbit rings (MRK-05) and exposure rings (MOT-02, MOT-07).
//!
//! Rings are drawn for planets and for any *unfolded* satellite (GRP-01).
//! A body in exposure-ring mode (see `motion.rs`) gets a second, brighter
//! and thicker ring drawn over the dim one, white while it is selected
//! (MOT-07).

use bevy::prelude::*;

use crate::Stage;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::fold::SatelliteFoldState;
use crate::view::motion_mode::MotionModeState;
use crate::view::scale::km_to_render;
use crate::view::tunables::Tunables;

/// How many straight segments make up a drawn orbit ring.
const RING_SEGMENTS: usize = 128;
/// A point must be at least this far in front of the camera to be drawn.
const NEAR_EPSILON: f32 = 0.01;
/// MRK-05: orbits are *faint* rings.
const DIM_RING_ALPHA: f32 = 0.35;
/// Below this strength an exposure ring isn't worth drawing.
const MIN_RING_STRENGTH: f32 = 0.01;

/// Its own gizmo group so exposure rings can carry a thicker line width
/// than the ordinary orbit rings (MOT-02).
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ExposureRingGizmos;

pub struct RingsPlugin;

impl Plugin for RingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<ExposureRingGizmos>()
            .add_systems(Startup, configure_exposure_gizmos)
            // Drawn with the rest of the overlay, once grouping and motion
            // have decided what shows.
            .add_systems(Update, draw_orbit_rings.in_set(Stage::Overlay));
    }
}

/// MOT-02: exposure rings are `exposure_ring_width_px` wide.
fn configure_exposure_gizmos(mut config_store: ResMut<GizmoConfigStore>, tunables: Res<Tunables>) {
    let (config, _) = config_store.config_mut::<ExposureRingGizmos>();
    config.line.width = tunables.exposure_ring_width_px;
}

#[allow(clippy::type_complexity)]
fn draw_orbit_rings(
    mut gizmos: Gizmos,
    mut exposure: Gizmos<ExposureRingGizmos>,
    selection: Res<crate::view::selection::Selection>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    bodies: Query<(
        Entity,
        &Orbit,
        &BodyClass,
        Option<&ChildOf>,
        Option<&SatelliteFoldState>,
        Option<&MotionModeState>,
    )>,
    transforms: Query<&GlobalTransform>,
) {
    let Ok((_, camera_transform)) = camera_q.single() else {
        return;
    };
    let camera_pos = camera_transform.translation();
    let camera_forward = camera_transform.forward().as_vec3();

    for (entity, orbit, class, child_of, fold, motion) in &bodies {
        // Gates and salvage sites sit on star orbits that would just add
        // clutter; gates have their own edge indicators instead.
        if matches!(class, BodyClass::Gate | BodyClass::SalvageSite) {
            continue;
        }
        // GRP-01: a folded satellite draws nothing - no marker, label,
        // ring or trail.
        if fold.map(|f| f.effective_hidden).unwrap_or(false) {
            continue;
        }

        let center = child_of
            .and_then(|c| transforms.get(c.parent()).ok())
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO);
        let radius = km_to_render(orbit.radius_km);

        let dim = crate::view::colors::ORBIT_LINE.with_alpha(DIM_RING_ALPHA);
        let strength = motion.map(MotionModeState::ring_strength).unwrap_or(0.0);
        let bright = (strength > MIN_RING_STRENGTH).then(|| {
            // MOT-07: a selected object in ring mode keeps its ring
            // highlighted.
            let color = if selection.is_open(entity) {
                crate::view::colors::SELECTION_RING
            } else {
                crate::view::colors::NEUTRAL
            };
            color.with_alpha(strength)
        });

        for (from, to) in visible_ring_segments(center, radius, camera_pos, camera_forward) {
            gizmos.line(from, to, dim);
            // MOT-02: the body is moving too fast to read as a position, so
            // the ring itself carries it.
            if let Some(bright) = bright {
                exposure.line(from, to, bright);
            }
        }
    }
}

/// The straight segments of a ring that lie in front of the camera.
///
/// Drawn segment by segment rather than as one circle, so that the part of
/// a ring that falls behind the camera is simply left out instead of taking
/// the whole ring with it. Zooming in on a planet puts the camera on its own
/// star orbit, and a whole-circle draw would either vanish or smear through
/// the near plane.
fn visible_ring_segments(
    center: Vec3,
    radius: f32,
    camera_pos: Vec3,
    camera_forward: Vec3,
) -> impl Iterator<Item = (Vec3, Vec3)> {
    let point_at = move |i: usize| {
        let angle = (i % RING_SEGMENTS) as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
        center + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
    };
    let in_front = move |p: Vec3| (p - camera_pos).dot(camera_forward) > NEAR_EPSILON;
    (0..RING_SEGMENTS)
        .map(move |i| (point_at(i), point_at(i + 1)))
        .filter(move |(from, to)| in_front(*from) && in_front(*to))
}
