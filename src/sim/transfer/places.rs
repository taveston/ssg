//! Places and bands (BAND-01 to BAND-04, BAND-08, PLAN-30): where a ship
//! can be sent, the two bands every massive body offers, and what a place
//! charges for leaving and arriving. Pure functions of plain values
//! (ARCH-20); the tree they are applied to is `tree.rs`.
//!
//! A band is a named orbit (SCL-07) a body derives from its size and
//! gravity: Low just above the surface, High a few radii out but inside
//! the Hill sphere. Leaving a band costs the delta-v to escape its
//! circular orbit, and takes the days a ship at escape speed needs to
//! coast out to the Hill radius; arriving costs and takes the same.

use bevy::prelude::*;
use orbits::{Conic, escape_cost};

use super::super::rail::km_per_day_to_km_per_s;
use super::super::tunables::SimTunables;

/// BAND-01: the two bands of a star, planet or moon.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    Low,
    High,
}

impl Band {
    /// The band's name as a named orbit (SCL-07).
    pub fn name(self) -> &'static str {
        match self {
            Band::Low => "Low",
            Band::High => "High",
        }
    }
}

/// PLAN-30: a body's derived band radii. A body whose High radius would
/// not clear its Low radius has no High band.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BandRadii {
    pub low_km: f64,
    pub high_km: Option<f64>,
}

impl BandRadii {
    /// The radius of a band, if the body has it.
    pub fn radius_km(&self, band: Band) -> Option<f64> {
        match band {
            Band::Low => Some(self.low_km),
            Band::High => self.high_km,
        }
    }
}

/// BAND-02: a destination. It resolves to one named orbit and the body
/// that charges for leaving and arriving; never a bare orbit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Place {
    /// A band of a star, planet or moon.
    Body { body: Entity, band: Band },
    /// The band a station occupies, at the station's phase (SCL-07).
    Station { station: Entity },
    /// A gate's own orbit.
    Gate { gate: Entity },
    /// A belt or salvage site's own orbit.
    Site { site: Entity },
    /// Wherever a ship is on the arrival day.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "ordered by the planner UI, not yet specced")
    )]
    Ship { ship: Entity },
    /// No destination: stop burning (PLAN-13).
    Hold,
}

/// BAND-01: the Hill radius of a body of `mass_kg` orbiting `orbit_radius_km`
/// from a parent of `parent_mass_kg`: the distance within which its own
/// gravity, not the parent's, holds a satellite. `None` without a massive
/// parent (the star has no Hill radius).
pub fn hill_radius_km(orbit_radius_km: f64, mass_kg: f64, parent_mass_kg: f64) -> Option<f64> {
    (parent_mass_kg > 0.0 && mass_kg > 0.0)
        .then(|| orbits::hill_radius(orbit_radius_km, mass_kg, parent_mass_kg))
}

/// BAND-01: a body's bands from its mean radius and Hill radius. Low is
/// `low_band_altitude_km` above the surface. High is `high_band_radii`
/// mean radii out, or `high_band_hill_fraction` of the Hill radius if that
/// is less; a body with no Hill radius (the star) takes the first figure.
/// A High radius that would not clear Low means no High band.
pub fn band_radii(mean_radius_km: f64, hill_km: Option<f64>, tunables: &SimTunables) -> BandRadii {
    let low_km = mean_radius_km + tunables.low_band_altitude_km;
    let by_radii = tunables.high_band_radii * mean_radius_km;
    let high = match hill_km {
        Some(hill) => by_radii.min(tunables.high_band_hill_fraction * hill),
        None => by_radii,
    };
    BandRadii {
        low_km,
        high_km: (high > low_km).then_some(high),
    }
}

/// BAND-03: the cost in km/s of leaving (or arriving at) a circular orbit
/// of `radius_km` round a body of gravitational parameter `mu` (km³/day²):
/// the difference between circular and escape speed. A body with no
/// gravity charges nothing.
pub fn departure_cost_kms(mu: f64, radius_km: f64) -> f64 {
    if mu <= 0.0 || radius_km <= 0.0 {
        return 0.0;
    }
    km_per_day_to_km_per_s(escape_cost(mu, radius_km))
}

/// BAND-04: the days a ship leaving a circular orbit of `radius_km` at
/// escape speed takes to reach the body's Hill radius, and so the time
/// each end of a crossing adds. Zero without a massive body or a Hill
/// sphere to cross.
pub fn approach_days(mu: f64, radius_km: f64, hill_km: Option<f64>) -> f64 {
    let Some(hill_km) = hill_km else {
        return 0.0;
    };
    if mu <= 0.0 || radius_km <= 0.0 || hill_km <= radius_km {
        return 0.0;
    }
    let escape_speed_km_per_day = (2.0 * mu / radius_km).sqrt();
    (hill_km - radius_km) / escape_speed_km_per_day
}

/// BAND-08: a coasting ship on a conic round the star is escaping if the
/// conic is unbound, or bound but reaching past `lost_beyond_km`.
pub fn is_escaping(conic: &Conic, lost_beyond_km: f64) -> bool {
    match conic.apoapsis_radius_km() {
        None => true,
        Some(apoapsis) => apoapsis > lost_beyond_km,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::units::gravitational_parameter_km3_per_day2;
    use crate::sim::units::{EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG};
    use orbits::{State, km_per_s_to_km_per_day};

    fn earth_mu() -> f64 {
        gravitational_parameter_km3_per_day2(EARTH_MASS_KG)
    }

    /// BAND-01: Low is the altitude above the surface; High is 34 radii
    /// unless half the Hill radius is less; the star, with no Hill radius,
    /// takes 34 radii.
    #[test]
    fn bands_follow_radius_and_hill_sphere() {
        let tunables = SimTunables::default();
        let star = band_radii(500_000.0, None, &tunables);
        assert_eq!(star.low_km, 500_500.0);
        assert_eq!(star.high_km, Some(17_000_000.0));

        let roomy = band_radii(6_000.0, Some(1_000_000.0), &tunables);
        assert_eq!(roomy.high_km, Some(204_000.0));

        let cramped = band_radii(6_000.0, Some(50_000.0), &tunables);
        assert_eq!(cramped.high_km, Some(25_000.0));
        assert_eq!(cramped.radius_km(Band::High), Some(25_000.0));
        assert_eq!(cramped.radius_km(Band::Low), Some(6_500.0));
    }

    /// BAND-01: a body whose High radius would not clear its Low radius
    /// has no High band.
    #[test]
    fn a_tiny_moon_has_no_high_band() {
        let tunables = SimTunables::default();
        let moon = band_radii(300.0, Some(1_200.0), &tunables);
        assert_eq!(moon.low_km, 800.0);
        assert_eq!(moon.high_km, None);
        assert_eq!(moon.radius_km(Band::High), None);
    }

    /// BAND-01: the Hill radius follows the cube root of the mass ratio,
    /// and there is none without a massive parent or a massive body.
    #[test]
    fn hill_radius_needs_a_massive_parent() {
        let hill = hill_radius_km(KM_PER_AU, EARTH_MASS_KG, SOLAR_MASS_KG).unwrap();
        assert!((hill - 1_496_000.0).abs() < 5_000.0, "{hill}");
        assert_eq!(hill_radius_km(KM_PER_AU, EARTH_MASS_KG, 0.0), None);
        assert_eq!(hill_radius_km(KM_PER_AU, 0.0, SOLAR_MASS_KG), None);
    }

    /// BAND-03: leaving a 6,871 km orbit of an Earth-mass body costs about
    /// 3.2 km/s (circular 7.6 km/s times √2 − 1); a massless body charges
    /// nothing.
    #[test]
    fn departure_cost_is_the_escape_margin() {
        let cost = departure_cost_kms(earth_mu(), 6_871.0);
        assert!((cost - 3.15).abs() < 0.05, "{cost}");
        assert_eq!(departure_cost_kms(0.0, 6_871.0), 0.0);
    }

    /// BAND-04: approach days are the Hill radius less the orbit radius,
    /// over the escape speed: about 1.6 days from 6,871 km up to Earth's
    /// Hill radius at 10.8 km/s. Zero without a Hill sphere or gravity.
    #[test]
    fn approach_days_cover_the_hill_sphere_at_escape_speed() {
        let days = approach_days(earth_mu(), 6_871.0, Some(1_496_000.0));
        assert!((days - 1.6).abs() < 0.05, "{days}");
        assert_eq!(approach_days(earth_mu(), 6_871.0, None), 0.0);
        assert_eq!(approach_days(0.0, 6_871.0, Some(1e6)), 0.0);
        assert_eq!(approach_days(earth_mu(), 2e6, Some(1e6)), 0.0);
    }

    /// BAND-08: a hyperbola is escaping; an ellipse is escaping only if its
    /// far point lies beyond the loss distance.
    #[test]
    fn escaping_means_unbound_or_reaching_too_far() {
        let mu = gravitational_parameter_km3_per_day2(SOLAR_MASS_KG);
        let r = bevy::math::DVec3::new(KM_PER_AU, 0.0, 0.0);
        let circular = orbits::circular_speed(mu, KM_PER_AU);
        let along = bevy::math::DVec3::new(0.0, 0.0, 1.0);
        let lost = 6.0 * KM_PER_AU;
        let bound = Conic::from_state(mu, State::new(r, circular * along), 0.0, 1e-6).unwrap();
        assert!(!is_escaping(&bound, lost));
        let fast =
            Conic::from_state(mu, State::new(r, 1.35 * circular * along), 0.0, 1e-6).unwrap();
        assert!(fast.is_bound() && is_escaping(&fast, lost));
        let escape = km_per_s_to_km_per_day(50.0);
        let unbound = Conic::from_state(mu, State::new(r, escape * along), 0.0, 1e-6).unwrap();
        assert!(!unbound.is_bound() && is_escaping(&unbound, lost));
    }
}
