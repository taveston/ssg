//! Circular orbits and the small formulas built on them: Kepler's third
//! law, circular speed, the cost of escaping, the Hill radius, the
//! Hohmann transfer and the synodic period.
//!
//! A circular orbit is the simplest path round a primary: constant
//! distance, constant speed, one full turn per period. Planets and
//! parking orbits are circular here; everything else in the crate is
//! measured against them.

use glam::DVec3;
use std::f64::consts::{PI, SQRT_2, TAU};

use crate::state::{State, point_in_plane};
use crate::trajectory::Rail;

/// The speed of a body on a circular orbit of `radius_km` round a primary
/// with gravitational parameter `mu` (km³/day²), in km/day. This is the
/// speed at which gravity exactly bends the path into a circle.
///
/// ```
/// use orbits::{circular_speed, km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s};
///
/// // Low orbit round an Earth-like planet: about 7.8 km/s.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let v = km_per_day_to_km_per_s(circular_speed(mu, 6_571.0));
/// assert!((v - 7.79).abs() < 0.01);
/// ```
pub fn circular_speed(mu: f64, radius_km: f64) -> f64 {
    (mu / radius_km).sqrt()
}

/// How long one turn of a circular orbit takes, in days: Kepler's third
/// law. The period grows with the radius to the power 3/2 and shrinks
/// with the square root of the primary's gravitational parameter `mu`
/// (km³/day²).
///
/// ```
/// use orbits::{circular_period, km3_per_s2_to_km3_per_day2};
///
/// // The Moon, 384,400 km from an Earth-like planet, takes about 27.4 days.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let days = circular_period(mu, 384_400.0);
/// assert!((days - 27.4).abs() < 0.1);
/// ```
pub fn circular_period(mu: f64, radius_km: f64) -> f64 {
    TAU * (radius_km.powi(3) / mu).sqrt()
}

/// The radius, in km, of the circular orbit with a given period in days:
/// Kepler's third law the other way round. Use it to place something so
/// that it goes round in a chosen time.
///
/// ```
/// use orbits::{circular_period, circular_radius_for_period, km3_per_s2_to_km3_per_day2};
///
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let r = circular_radius_for_period(mu, 1.0);
/// assert!((circular_period(mu, r) - 1.0).abs() < 1e-9);
/// ```
pub fn circular_radius_for_period(mu: f64, period_days: f64) -> f64 {
    (mu * (period_days / TAU).powi(2)).cbrt()
}

/// The change of speed, in km/day, that takes a body from a circular
/// orbit of `radius_km` to just barely escaping the primary (gravitational
/// parameter `mu`, km³/day²): the circular speed times √2 − 1. Paying it
/// again on arrival captures a body arriving from far away into the same
/// circular orbit.
///
/// ```
/// use orbits::{escape_cost, km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s};
///
/// // Leaving low orbit round an Earth-like planet costs about 3.2 km/s.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let cost = km_per_day_to_km_per_s(escape_cost(mu, 6_571.0));
/// assert!((cost - 3.23).abs() < 0.01);
/// ```
pub fn escape_cost(mu: f64, radius_km: f64) -> f64 {
    circular_speed(mu, radius_km) * (SQRT_2 - 1.0)
}

/// The Hill radius, in km: how far from a body its own gravity wins over
/// its parent's, so that satellites inside it stay bound. For a body of
/// `mass` on a circular orbit of `orbit_radius_km` round a parent of
/// `parent_mass`, it is the orbit radius times the cube root of
/// mass / (3 × parent mass). The two masses only need the same unit.
///
/// ```
/// use orbits::hill_radius;
///
/// // An Earth-mass planet 1.496e8 km from a Sun-mass star: about 1.5 million km.
/// let r = hill_radius(1.496e8, 1.0, 333_000.0);
/// assert!((r - 1.5e6).abs() < 0.01e6);
/// ```
pub fn hill_radius(orbit_radius_km: f64, mass: f64, parent_mass: f64) -> f64 {
    orbit_radius_km * (mass / (3.0 * parent_mass)).cbrt()
}

/// The two burns and the flight time of a Hohmann transfer: the cheapest
/// two-burn trip between two circular orbits, which follows half an
/// ellipse touching both. Speeds are in km/day.
///
/// ```
/// use orbits::{hohmann, km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s};
///
/// // From a 1 AU orbit to a 1.5 AU orbit round a Sun-like star.
/// let mu = km3_per_s2_to_km3_per_day2(1.327e11);
/// let trip = hohmann(mu, 1.496e8, 2.244e8);
/// assert!((trip.days - 255.0).abs() < 1.0);
/// let total = km_per_day_to_km_per_s(trip.first_burn + trip.second_burn);
/// assert!((total - 5.4).abs() < 0.1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hohmann {
    /// The change of speed at departure, in km/day.
    pub first_burn: f64,
    /// The change of speed at arrival, in km/day.
    pub second_burn: f64,
    /// The time spent on the half ellipse, in days.
    pub days: f64,
}

/// Solves the Hohmann transfer from a circular orbit of `from_km` to one
/// of `to_km` round a primary with gravitational parameter `mu`
/// (km³/day²). Works inward as well as outward; both burns are reported
/// as positive sizes.
///
/// ```
/// use orbits::{hohmann, km3_per_s2_to_km3_per_day2};
///
/// let mu = km3_per_s2_to_km3_per_day2(1.327e11);
/// let out = hohmann(mu, 1.496e8, 2.244e8);
/// let back = hohmann(mu, 2.244e8, 1.496e8);
/// assert!((out.days - back.days).abs() < 1e-9);
/// assert!((out.first_burn - back.second_burn).abs() < 1e-6);
/// ```
pub fn hohmann(mu: f64, from_km: f64, to_km: f64) -> Hohmann {
    let semi_major_km = (from_km + to_km) / 2.0;
    // Speed on the transfer ellipse at a distance r, from the vis-viva
    // equation: v² = μ (2/r − 1/a).
    let speed_on_ellipse = |r: f64| (mu * (2.0 / r - 1.0 / semi_major_km)).sqrt();
    Hohmann {
        first_burn: (speed_on_ellipse(from_km) - circular_speed(mu, from_km)).abs(),
        second_burn: (circular_speed(mu, to_km) - speed_on_ellipse(to_km)).abs(),
        days: PI * (semi_major_km.powi(3) / mu).sqrt(),
    }
}

/// The synodic period, in days: how long between two moments when two
/// bodies on circular orbits line up the same way again. It is the time
/// between transfer windows from one to the other. Two bodies with the
/// same period never change their lineup, so the result is infinite.
///
/// ```
/// use orbits::synodic_period;
///
/// // Earth (365 d) and Mars (687 d) line up about every 780 days.
/// let days = synodic_period(365.25, 687.0);
/// assert!((days - 780.0).abs() < 1.0);
/// assert!(synodic_period(100.0, 100.0).is_infinite());
/// ```
pub fn synodic_period(period_a_days: f64, period_b_days: f64) -> f64 {
    let rate = (1.0 / period_a_days - 1.0 / period_b_days).abs();
    1.0 / rate
}

/// A circular orbit in the plane: its radius in km, its period in days
/// and the angle (radians from +x towards +z) its body has on day zero.
/// Motion is prograde: the angle grows with time. A zero or non-finite
/// period means the body does not move at all.
///
/// ```
/// use orbits::{CircularOrbit, km3_per_s2_to_km3_per_day2};
///
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let moon = CircularOrbit::new(mu, 384_400.0, 0.0);
/// // A quarter period on, the moon is a quarter turn round.
/// let p = moon.position_at(moon.period_days / 4.0);
/// assert!(p.x.abs() < 1.0);
/// assert!((p.z - 384_400.0).abs() < 1.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircularOrbit {
    /// Distance from the primary, in km.
    pub radius_km: f64,
    /// Time for one full turn, in days.
    pub period_days: f64,
    /// The angle of the body on day zero, in radians.
    pub phase0_rad: f64,
}

impl CircularOrbit {
    /// A circular orbit of `radius_km` round a primary with gravitational
    /// parameter `mu` (km³/day²), with the period from Kepler's third
    /// law and the body at `phase0_rad` on day zero.
    ///
    /// ```
    /// use orbits::{CircularOrbit, km3_per_s2_to_km3_per_day2};
    ///
    /// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
    /// let moon = CircularOrbit::new(mu, 384_400.0, 1.0);
    /// assert!((moon.period_days - 27.4).abs() < 0.1);
    /// ```
    pub fn new(mu: f64, radius_km: f64, phase0_rad: f64) -> Self {
        Self {
            radius_km,
            period_days: circular_period(mu, radius_km),
            phase0_rad,
        }
    }

    /// The body's angle on `day`, in radians: the day-zero angle plus one
    /// full turn per period elapsed.
    ///
    /// ```
    /// use orbits::CircularOrbit;
    /// use std::f64::consts::PI;
    ///
    /// let orbit = CircularOrbit { radius_km: 1_000.0, period_days: 10.0, phase0_rad: 0.0 };
    /// assert!((orbit.angle_at(5.0) - PI).abs() < 1e-12);
    /// ```
    pub fn angle_at(&self, day: f64) -> f64 {
        self.phase0_rad + self.turns_by(day) * TAU
    }

    /// The body's position on `day`, in km relative to the primary.
    ///
    /// ```
    /// use orbits::CircularOrbit;
    ///
    /// let orbit = CircularOrbit { radius_km: 1_000.0, period_days: 10.0, phase0_rad: 0.0 };
    /// let p = orbit.position_at(10.0);
    /// assert!((p.x - 1_000.0).abs() < 1e-9);
    /// assert!(p.z.abs() < 1e-9);
    /// ```
    pub fn position_at(&self, day: f64) -> DVec3 {
        point_in_plane(self.radius_km, self.angle_at(day))
    }

    /// How many turns the body has made by `day`. A zero or non-finite
    /// period would divide by zero; such a body simply sits still.
    fn turns_by(&self, day: f64) -> f64 {
        if self.period_days.is_finite() && self.period_days != 0.0 {
            day / self.period_days
        } else {
            0.0
        }
    }

    /// The speed round the orbit in km/day: the circumference per period.
    fn speed(&self) -> f64 {
        if self.period_days.is_finite() && self.period_days != 0.0 {
            TAU * self.radius_km / self.period_days
        } else {
            0.0
        }
    }
}

impl Rail for CircularOrbit {
    /// Position and velocity on `day`. The velocity points along the
    /// circle in the prograde sense.
    fn state_at(&self, day: f64) -> State {
        let angle = self.angle_at(day);
        let (sin, cos) = angle.sin_cos();
        let speed = self.speed();
        State {
            position: DVec3::new(self.radius_km * cos, 0.0, self.radius_km * sin),
            velocity: DVec3::new(-speed * sin, 0.0, speed * cos),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::km3_per_s2_to_km3_per_day2;

    /// An Earth-like gravitational parameter, in km³/day².
    fn earth_mu() -> f64 {
        km3_per_s2_to_km3_per_day2(3.986e5)
    }

    /// Kepler's third law: period squared goes as radius cubed, so
    /// quadrupling the radius multiplies the period by eight (SCL-04).
    #[test]
    fn period_follows_keplers_third_law() {
        let base = circular_period(earth_mu(), 10_000.0);
        let quadrupled = circular_period(earth_mu(), 40_000.0);
        assert!((quadrupled / base - 8.0).abs() < 1e-9);
    }

    /// The radius for a period is the inverse of the period for a radius.
    #[test]
    fn radius_for_period_inverts_period() {
        let r = circular_radius_for_period(earth_mu(), 27.4);
        assert!((r - 384_400.0).abs() < 2_000.0, "got {r}");
    }

    /// The escape cost is the circular speed times √2 − 1.
    #[test]
    fn escape_cost_is_a_fixed_fraction_of_circular_speed() {
        let v = circular_speed(earth_mu(), 7_000.0);
        assert!((escape_cost(earth_mu(), 7_000.0) / v - (SQRT_2 - 1.0)).abs() < 1e-12);
    }

    /// The circular-orbit state's velocity is tangential, prograde and of
    /// the circular speed.
    #[test]
    fn circular_state_moves_prograde_at_circular_speed() {
        let orbit = CircularOrbit::new(earth_mu(), 20_000.0, 0.3);
        let state = orbit.state_at(2.5);
        assert!(state.position.dot(state.velocity).abs() < 1e-3);
        assert!(state.angular_momentum() > 0.0);
        let expected = circular_speed(earth_mu(), 20_000.0);
        assert!((state.speed() - expected).abs() < 1e-6 * expected);
    }

    /// A body with no period sits at its start phase instead of dividing
    /// by zero.
    #[test]
    fn a_zero_period_body_sits_still() {
        let still = CircularOrbit {
            radius_km: 100.0,
            period_days: 0.0,
            phase0_rad: 0.4,
        };
        assert_eq!(still.position_at(0.0), still.position_at(99.0));
        assert_eq!(still.state_at(3.0).velocity, DVec3::ZERO);
    }

    /// Hohmann from a lower to a higher orbit: both burns speed the ship
    /// up, and the flight takes half the transfer ellipse's period.
    #[test]
    fn hohmann_outward_takes_half_an_ellipse_period() {
        let mu = earth_mu();
        let trip = hohmann(mu, 7_000.0, 42_000.0);
        let half_period = circular_period(mu, (7_000.0 + 42_000.0) / 2.0) / 2.0;
        assert!((trip.days - half_period).abs() < 1e-9);
        assert!(trip.first_burn > 0.0 && trip.second_burn > 0.0);
        assert!(trip.first_burn > trip.second_burn);
    }

    /// Two bodies with equal periods never line up differently.
    #[test]
    fn synodic_period_of_equal_periods_is_infinite() {
        assert!(synodic_period(12.0, 12.0).is_infinite());
        assert!((synodic_period(12.0, 24.0) - 24.0).abs() < 1e-9);
    }
}
