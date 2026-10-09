//! Circular orbital motion and the orbit-path gizmo rings.
//!
//! `OrbitalBody` positions its entity directly: for a root-level body (a
//! planet) that is a world position around the star at the origin; for a
//! body spawned as a child of another orbital body (a moon, parented to its
//! planet) it is a local offset, and Bevy's own transform propagation adds
//! the parent's current world position on top - so planets and moons use
//! the exact same update logic.

use bevy::prelude::*;
use std::f64::consts::TAU;

use crate::time_control::GameClock;

#[derive(Component)]
pub struct OrbitalBody {
    pub orbit_radius: f32,
    pub period_days: f64,
    pub phase0_rad: f64,
}

pub struct OrbitsPlugin;

impl Plugin for OrbitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_orbital_positions, draw_orbit_paths));
    }
}

fn update_orbital_positions(clock: Res<GameClock>, mut query: Query<(&OrbitalBody, &mut Transform)>) {
    for (body, mut transform) in &mut query {
        let angle = body.phase0_rad + (clock.elapsed_days / body.period_days) * TAU;
        let (sin, cos) = angle.sin_cos();
        transform.translation = Vec3::new(body.orbit_radius * cos as f32, 0.0, body.orbit_radius * sin as f32);
    }
}

fn draw_orbit_paths(
    mut gizmos: Gizmos,
    bodies: Query<(&OrbitalBody, Option<&ChildOf>)>,
    transforms: Query<&GlobalTransform>,
) {
    // Lay the circle flat in the XZ plane (same plane the orbits move in).
    let ring_orientation = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for (body, child_of) in &bodies {
        let center = child_of
            .and_then(|c| transforms.get(c.parent()).ok())
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO);
        gizmos.circle(
            Isometry3d::new(center, ring_orientation),
            body.orbit_radius,
            Color::srgba(0.5, 0.55, 0.65, 0.6),
        );
    }
}
