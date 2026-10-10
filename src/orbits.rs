//! Circular orbital motion and the orbit rings.
//!
//! `OrbitalBody` positions its entity directly: for a root-level body (a
//! planet) that is a world position around the star at the origin; for a
//! body spawned as a child of another orbital body (a moon, parented to its
//! planet) it is a local offset, and Bevy's own transform propagation adds
//! the parent's current world position on top - so planets and moons use
//! the exact same update logic.
//!
//! Rings are drawn for planets and for any *unfolded* satellite. A body in
//! exposure-ring mode (see `motion.rs`) gets a second, brighter and thicker
//! ring drawn over the dim one.

use bevy::prelude::*;
use std::f64::consts::TAU;

use crate::Stage;
use crate::grouping::SatelliteFoldState;
use crate::motion::MotionModeState;
use crate::system_data::BodyClass;
use crate::time_control::GameClock;
use crate::tunables::Tunables;

#[derive(Component)]
pub struct OrbitalBody {
    pub orbit_radius: f32,
    pub period_days: f64,
    pub phase0_rad: f64,
}

/// How many straight segments make up a drawn orbit ring.
const RING_SEGMENTS: usize = 128;
/// A point must be at least this far in front of the camera to be drawn.
const NEAR_EPSILON: f32 = 0.01;

/// Its own gizmo group so exposure rings can carry a thicker line width
/// than the ordinary orbit rings.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ExposureRingGizmos;

pub struct OrbitsPlugin;

impl Plugin for OrbitsPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<ExposureRingGizmos>()
            .add_systems(Startup, configure_exposure_gizmos)
            .add_systems(
                Update,
                (update_orbital_positions, draw_orbit_paths)
                    .chain()
                    .in_set(Stage::Orbits),
            );
    }
}

fn configure_exposure_gizmos(mut config_store: ResMut<GizmoConfigStore>, tunables: Res<Tunables>) {
    let (config, _) = config_store.config_mut::<ExposureRingGizmos>();
    config.line.width = tunables.exposure_ring_width_px;
}

fn update_orbital_positions(
    clock: Res<GameClock>,
    mut query: Query<(&OrbitalBody, &mut Transform)>,
) {
    for (body, mut transform) in &mut query {
        let angle = body.phase0_rad + (clock.elapsed_days / body.period_days) * TAU;
        let (sin, cos) = angle.sin_cos();
        transform.translation = Vec3::new(
            body.orbit_radius * cos as f32,
            0.0,
            body.orbit_radius * sin as f32,
        );
    }
}

#[allow(clippy::type_complexity)]
fn draw_orbit_paths(
    mut gizmos: Gizmos,
    mut exposure: Gizmos<ExposureRingGizmos>,
    inspections: Res<crate::hover_inspect::Inspections>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    bodies: Query<(
        Entity,
        &OrbitalBody,
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

    for (entity, body, class, child_of, fold, motion) in &bodies {
        // Gates and salvage sites sit on star orbits that would just add
        // clutter; gates have their own edge indicators instead.
        if matches!(class, BodyClass::Gate | BodyClass::SalvageSite) {
            continue;
        }
        // A folded satellite draws nothing: no marker, label, ring, trail.
        if fold.map(|f| f.effective_hidden).unwrap_or(false) {
            continue;
        }

        let center = child_of
            .and_then(|c| transforms.get(c.parent()).ok())
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO);

        // Drawn segment by segment rather than as one circle, so that the
        // part of a ring that falls behind the camera is simply left out
        // instead of taking the whole ring with it. Zooming in on a planet
        // puts the camera on its own star orbit, and a whole-circle draw
        // would either vanish or smear through the near plane.
        let dim = crate::colors::ORBIT_LINE.with_alpha(0.35);
        let strength = motion.map(MotionModeState::ring_strength).unwrap_or(0.0);
        let bright = (strength > 0.01).then(|| {
            let color = if inspections.is_open(entity) {
                crate::colors::SELECTION_RING
            } else {
                crate::colors::NEUTRAL
            };
            color.with_alpha(strength)
        });

        let point_at = |i: usize| {
            let angle = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
            center
                + Vec3::new(
                    body.orbit_radius * angle.cos(),
                    0.0,
                    body.orbit_radius * angle.sin(),
                )
        };
        let in_front = |p: Vec3| (p - camera_pos).dot(camera_forward) > NEAR_EPSILON;

        let mut previous = point_at(0);
        for i in 1..=RING_SEGMENTS {
            let current = point_at(i % RING_SEGMENTS);
            if in_front(previous) && in_front(current) {
                gizmos.line(previous, current, dim);
                // Exposure-ring mode: the body is moving too fast to read
                // as a position, so the ring itself carries it.
                if let Some(bright) = bright {
                    exposure.line(previous, current, bright);
                }
            }
            previous = current;
        }
    }
}
