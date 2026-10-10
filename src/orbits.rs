//! Circular orbital motion, in real units (SCL-01, SCL-04, ARCH-10).
//!
//! `Orbit` holds the orbital elements in km, days and radians, and a
//! simulation system writes each body's position in km into
//! `OrbitPositionKm` from the clock alone (ARCH-11). A separate *view*
//! system converts those km to render space and writes `Transform`, so the
//! render transform is never the simulation's source of truth.
//!
//! Positions are relative to whatever the body orbits: the star at the
//! origin for a planet, the parent body for a satellite. Bevy's transform
//! propagation adds the parent's world position on top of a satellite's
//! local offset, so planets and moons share the same update logic.

use bevy::math::DVec3;
use bevy::prelude::*;
use std::f64::consts::TAU;

use crate::Stage;
use crate::scale::km_to_render_vec3;
use crate::time_control::GameClock;

/// Orbital elements in real units: a circular, coplanar orbit (SCL-04).
#[derive(Component, Debug, Clone, Copy)]
pub struct Orbit {
    pub radius_km: f64,
    pub period_days: f64,
    pub phase0_rad: f64,
}

/// A body's position in km, relative to the body it orbits. The
/// simulation's source of truth for where things are (ARCH-11).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OrbitPositionKm(pub DVec3);

/// SCL-04 / ARCH-11: where a body on a circular orbit is after
/// `elapsed_days`, in km relative to what it orbits. The orbital plane is
/// XZ, so y is always zero.
pub fn orbit_position_km(orbit: &Orbit, elapsed_days: f64) -> DVec3 {
    let angle = orbit.phase0_rad + orbital_fraction(orbit.period_days, elapsed_days) * TAU;
    let (sin, cos) = angle.sin_cos();
    DVec3::new(orbit.radius_km * cos, 0.0, orbit.radius_km * sin)
}

/// How many orbits a body has completed in `elapsed_days`. A zero or
/// non-finite period would otherwise divide by zero; such a body simply
/// sits at its start phase.
fn orbital_fraction(period_days: f64, elapsed_days: f64) -> f64 {
    if period_days.is_finite() && period_days != 0.0 {
        elapsed_days / period_days
    } else {
        0.0
    }
}

pub struct OrbitsPlugin;

impl Plugin for OrbitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_orbit_positions, project_orbit_positions)
                .chain()
                .in_set(Stage::Orbits),
        );
    }
}

/// ARCH-11: simulation. Clock and orbital elements in, km out.
fn update_orbit_positions(
    clock: Res<GameClock>,
    mut bodies: Query<(&Orbit, &mut OrbitPositionKm)>,
) {
    let elapsed_days = clock.elapsed_days();
    for (orbit, mut position) in &mut bodies {
        position.0 = orbit_position_km(orbit, elapsed_days);
    }
}

/// ARCH-11 / ARCH-12: view. The one place a simulated position becomes a
/// `Transform`, through `scale.rs`.
fn project_orbit_positions(mut bodies: Query<(&OrbitPositionKm, &mut Transform)>) {
    for (position, mut transform) in &mut bodies {
        transform.translation = km_to_render_vec3(position.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOON: Orbit = Orbit {
        radius_km: 400_000.0,
        period_days: 20.0,
        phase0_rad: 0.0,
    };

    /// ARCH-11 / SCL-01: the position is in km, straight from the elements
    /// and the elapsed days - no transform anywhere.
    #[test]
    fn arch_11_position_starts_at_the_start_phase() {
        let position = orbit_position_km(&MOON, 0.0);
        assert!((position.x - 400_000.0).abs() < 1e-6);
        assert!(position.y.abs() < 1e-12, "orbits are coplanar in XZ");
        assert!(position.z.abs() < 1e-6);
    }

    /// SCL-04: a circular orbit keeps its radius, whatever the phase.
    #[test]
    fn scl_04_radius_is_constant_round_the_orbit() {
        for days in [0.0_f64, 1.3, 5.0, 10.0, 17.7, 100.0] {
            let position = orbit_position_km(&MOON, days);
            let radius = (position.x * position.x + position.z * position.z).sqrt();
            assert!(
                (radius - MOON.radius_km).abs() < 1e-6,
                "at {days} d the radius was {radius}"
            );
        }
    }

    /// ARCH-11: one period brings a body back to where it started, and half
    /// a period puts it on the far side.
    #[test]
    fn arch_11_one_period_is_one_full_turn() {
        let start = orbit_position_km(&MOON, 0.0);
        let full = orbit_position_km(&MOON, MOON.period_days);
        assert!((full - start).length() < 1e-6);

        let half = orbit_position_km(&MOON, MOON.period_days / 2.0);
        assert!((half + start).length() < 1e-6, "half a period is opposite");

        let quarter = orbit_position_km(&MOON, MOON.period_days / 4.0);
        assert!(quarter.dot(start).abs() < 1e-3, "a quarter turn is square");
    }

    /// ARCH-11: the start phase offsets the whole orbit, so two bodies on
    /// the same orbit at different phases never coincide.
    #[test]
    fn arch_11_start_phase_offsets_the_orbit() {
        let other = Orbit {
            phase0_rad: std::f64::consts::PI,
            ..MOON
        };
        let a = orbit_position_km(&MOON, 3.0);
        let b = orbit_position_km(&other, 3.0);
        assert!((a + b).length() < 1e-6);
    }

    /// A body with no period (a degenerate record) sits still rather than
    /// dividing by zero.
    #[test]
    fn arch_11_a_zero_period_body_sits_at_its_start_phase() {
        let still = Orbit {
            period_days: 0.0,
            ..MOON
        };
        assert_eq!(
            orbit_position_km(&still, 0.0),
            orbit_position_km(&still, 99.0)
        );
    }
}
