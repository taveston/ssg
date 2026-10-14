//! Circular orbital motion, in real units (SCL-01, SCL-04, ARCH-10,
//! ARCH-11).
//!
//! `Orbit` holds the orbital elements in km, days and radians, and
//! `update_orbit_positions` writes each body's position in km into
//! `OrbitPositionKm` from the clock alone. The maths is the `orbits`
//! crate's (ARCH-40); this module holds the components and turns a mass
//! into the gravitational parameter the crate wants. Turning a position
//! into something on screen is the view's job (`view/projection.rs`);
//! nothing here knows a render unit exists.
//!
//! Positions are relative to whatever the body orbits: the star at the
//! origin for a planet, the parent body (`ChildOf`) for a satellite.

use bevy::math::DVec3;
use bevy::prelude::*;
use orbits::CircularOrbit;

use super::clock::GameClock;
use super::units::gravitational_parameter_km3_per_day2;

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

impl From<&Orbit> for CircularOrbit {
    /// The same elements as the `orbits` crate's circular orbit; the
    /// component exists so the simulation can attach them to an entity.
    fn from(orbit: &Orbit) -> Self {
        CircularOrbit {
            radius_km: orbit.radius_km,
            period_days: orbit.period_days,
            phase0_rad: orbit.phase0_rad,
        }
    }
}

/// SCL-04: Kepler's third law - the period of a circular orbit of
/// `radius_km` round a point mass of `central_mass_kg`, in days. Used
/// uniformly for planets round the star and satellites round their parent.
pub fn kepler_period_days(radius_km: f64, central_mass_kg: f64) -> f64 {
    orbits::circular_period(
        gravitational_parameter_km3_per_day2(central_mass_kg),
        radius_km,
    )
}

/// SCL-04 / ARCH-11: where a body on a circular orbit is after
/// `elapsed_days`, in km relative to what it orbits. The orbital plane is
/// XZ, so y is always zero. A body with a zero or non-finite period sits
/// at its start phase.
pub fn orbit_position_km(orbit: &Orbit, elapsed_days: f64) -> DVec3 {
    CircularOrbit::from(orbit).position_at(elapsed_days)
}

/// ARCH-11: clock and orbital elements in, km out.
pub fn update_orbit_positions(
    clock: Res<GameClock>,
    mut bodies: Query<(&Orbit, &mut OrbitPositionKm)>,
) {
    let elapsed_days = clock.elapsed_days();
    for (orbit, mut position) in &mut bodies {
        position.0 = orbit_position_km(orbit, elapsed_days);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sim::units::EARTH_MASS_KG;

    /// SCL-04: Kepler's third law - the period squared goes as the radius
    /// cubed, so quadrupling the radius multiplies the period by eight.
    #[test]
    fn period_follows_keplers_third_law() {
        let base = kepler_period_days(10_000.0, EARTH_MASS_KG);
        let quadrupled = kepler_period_days(40_000.0, EARTH_MASS_KG);
        assert!(
            (quadrupled / base - 8.0).abs() < 1e-6,
            "expected 8x, got {}",
            quadrupled / base
        );
    }

    /// SCL-04: and against a known orbit, so a wrong constant would show:
    /// the Moon's sidereal month is about 27.4 days round a lone Earth.
    #[test]
    fn period_matches_the_moon() {
        let days = kepler_period_days(384_400.0, EARTH_MASS_KG);
        assert!((days - 27.4).abs() < 0.1, "got {days} d");
    }

    /// SCL-04: a heavier parent pulls its satellites round faster.
    #[test]
    fn heavier_parent_means_shorter_period() {
        let light = kepler_period_days(50_000.0, EARTH_MASS_KG);
        let heavy = kepler_period_days(50_000.0, 4.0 * EARTH_MASS_KG);
        assert!(heavy < light);
        assert!(
            (light / heavy - 2.0).abs() < 1e-6,
            "period goes as 1/sqrt(M)"
        );
    }

    const MOON: Orbit = Orbit {
        radius_km: 400_000.0,
        period_days: 20.0,
        phase0_rad: 0.0,
    };

    /// ARCH-11 / SCL-01: the position is in km, straight from the elements
    /// and the elapsed days - no transform anywhere.
    #[test]
    fn position_starts_at_the_start_phase() {
        let position = orbit_position_km(&MOON, 0.0);
        assert!((position.x - 400_000.0).abs() < 1e-6);
        assert!(position.y.abs() < 1e-12, "orbits are coplanar in XZ");
        assert!(position.z.abs() < 1e-6);
    }

    /// SCL-04: a circular orbit keeps its radius, whatever the phase.
    #[test]
    fn radius_is_constant_round_the_orbit() {
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
    fn one_period_is_one_full_turn() {
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
    fn start_phase_offsets_the_orbit() {
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
    /// Covers ARCH-11.
    #[test]
    fn a_zero_period_body_sits_at_its_start_phase() {
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
