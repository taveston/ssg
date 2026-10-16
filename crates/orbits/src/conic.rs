//! Conics: the paths a body coasts along under one primary's gravity
//! alone. A bound body follows an ellipse (a closed, repeating loop); a
//! body with enough speed to leave follows a hyperbola (one pass, never
//! to return). This module builds a conic from a body's position and
//! velocity on some day and reads its position and velocity back on any
//! other day, by solving Kepler's equation (RAIL-05, RAIL-06).
//!
//! The parabola, the knife-edge case between the two, is never produced:
//! a state that close to the edge is nudged to the nearer side.

use glam::DVec3;
use std::f64::consts::TAU;

use crate::state::{Sense, State, angle_of, mirror, wrap_to_pi};
use crate::trajectory::Rail;

/// Newton's method on Kepler's equation stops when a step is this small.
const KEPLER_TOLERANCE: f64 = 1e-14;
/// And gives up after this many steps (it normally takes under ten).
const KEPLER_MAX_STEPS: usize = 64;

/// An ellipse or hyperbola round a primary, as the elements that fix it:
///
/// - `mu`: the primary's gravitational parameter, km³/day².
/// - `semi_latus_rectum_km` (*p*): the distance from the primary to the
///   path, measured at right angles to the long axis. It sets the size.
/// - `eccentricity` (*e*): how stretched the shape is. 0 is a circle,
///   below 1 an ellipse, above 1 a hyperbola.
/// - `periapsis_angle_rad` (ω): the angle, in the sense of motion, of the
///   periapsis: the point of the path closest to the primary.
/// - `periapsis_day` (*t_p*): a day on which the body is at periapsis.
/// - `sense`: prograde or retrograde.
///
/// Build one with [`Conic::from_state`] rather than by hand.
///
/// ```
/// use glam::DVec3;
/// use orbits::{Conic, Rail, State, km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day};
///
/// // A probe 7,000 km from an Earth-like planet, moving sideways at 9 km/s:
/// // faster than circular, so an ellipse that climbs away from here.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let start = State::new(
///     DVec3::new(7_000.0, 0.0, 0.0),
///     DVec3::new(0.0, 0.0, km_per_s_to_km_per_day(9.0)),
/// );
/// let conic = Conic::from_state(mu, start, 0.0, 1e-6).unwrap();
/// assert!(conic.is_bound());
/// assert!((conic.periapsis_radius_km() - 7_000.0).abs() < 1e-6);
/// // Half a period later it is at the far end, the apoapsis.
/// let far = conic.state_at(conic.period_days().unwrap() / 2.0);
/// assert!((far.radius() - conic.apoapsis_radius_km().unwrap()).abs() < 1e-3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conic {
    /// The primary's gravitational parameter, km³/day².
    pub mu: f64,
    /// The semi-latus rectum *p*, km.
    pub semi_latus_rectum_km: f64,
    /// The eccentricity *e*: never exactly 1.
    pub eccentricity: f64,
    /// The angle of periapsis ω, radians, measured in the sense of motion
    /// (so for a retrograde conic it is measured from +x towards −z).
    pub periapsis_angle_rad: f64,
    /// A day on which the body is at periapsis.
    pub periapsis_day: f64,
    /// Which way round the body goes.
    pub sense: Sense,
}

impl Conic {
    /// The conic a body is on, given its state on `day` round a primary
    /// with gravitational parameter `mu` (km³/day²). A state whose
    /// eccentricity is within `parabola_epsilon` of 1 is nudged to the
    /// nearer side, so the result is always an ellipse or a hyperbola.
    ///
    /// Returns `None` for a state that has no conic: a body at the
    /// primary, or one moving straight towards or away from it, or a
    /// non-finite state.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::{Conic, Sense, State};
    ///
    /// let mu = 1e12;
    /// let fast = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, -1e5));
    /// let conic = Conic::from_state(mu, fast, 0.0, 1e-6).unwrap();
    /// assert_eq!(conic.sense, Sense::Retrograde);
    /// assert!(!conic.is_bound());
    ///
    /// let radial = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 0.0));
    /// assert!(Conic::from_state(mu, radial, 0.0, 1e-6).is_none());
    /// ```
    pub fn from_state(mu: f64, state: State, day: f64, parabola_epsilon: f64) -> Option<Self> {
        let sense = state.sense();
        // Work in the frame where motion is prograde; mirror back on
        // evaluation.
        let state = match sense {
            Sense::Prograde => state,
            Sense::Retrograde => state.mirrored(),
        };
        let r = state.position;
        let v = state.velocity;
        let radius = r.length();
        let h = state.angular_momentum();
        if !(h > 0.0 && radius > 0.0 && mu > 0.0) || !r.is_finite() || !v.is_finite() {
            return None;
        }
        let mut p = h * h / mu;
        // The eccentricity vector points at periapsis and has length e.
        let e_vec = ((v.length_squared() - mu / radius) * r - r.dot(v) * v) / mu;
        let mut e = e_vec.length();
        let omega = angle_of(e_vec);
        let nu = wrap_to_pi(angle_of(r) - omega);
        if (e - 1.0).abs() < parabola_epsilon {
            // Nudge away from the parabola, keeping the body's distance
            // on this day exact so the path still passes through it.
            e = if e < 1.0 {
                1.0 - parabola_epsilon
            } else {
                1.0 + parabola_epsilon
            };
            p = radius * (1.0 + e * nu.cos());
        }
        let shape = Shape::new(mu, p, e);
        let periapsis_day = day - shape.mean_anomaly_at_true(nu) / shape.mean_motion;
        Some(Self {
            mu,
            semi_latus_rectum_km: p,
            eccentricity: e,
            periapsis_angle_rad: omega,
            periapsis_day,
            sense,
        })
    }

    /// Whether the body stays with the primary: true for an ellipse,
    /// false for a hyperbola.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::{Conic, State};
    ///
    /// let mu = 1e12;
    /// let slow = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, 3e4));
    /// assert!(Conic::from_state(mu, slow, 0.0, 1e-6).unwrap().is_bound());
    /// ```
    pub fn is_bound(&self) -> bool {
        self.eccentricity < 1.0
    }

    /// The closest the path comes to the primary, in km.
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// let conic = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 0.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 0.0,
    ///     sense: Sense::Prograde,
    /// };
    /// assert_eq!(conic.periapsis_radius_km(), 1_000.0);
    /// ```
    pub fn periapsis_radius_km(&self) -> f64 {
        self.semi_latus_rectum_km / (1.0 + self.eccentricity)
    }

    /// The farthest the path gets from the primary, in km; `None` for a
    /// hyperbola, which has no far end.
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// let conic = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 0.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 0.0,
    ///     sense: Sense::Prograde,
    /// };
    /// assert_eq!(conic.apoapsis_radius_km(), Some(3_000.0));
    /// ```
    pub fn apoapsis_radius_km(&self) -> Option<f64> {
        self.is_bound()
            .then(|| self.semi_latus_rectum_km / (1.0 - self.eccentricity))
    }

    /// How long one loop of an ellipse takes, in days; `None` for a
    /// hyperbola, which never repeats.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::{Conic, State, circular_period, circular_speed};
    ///
    /// // A circular state gives a conic whose period is Kepler's.
    /// let mu = 1e12;
    /// let v = circular_speed(mu, 2_000.0);
    /// let state = State::new(DVec3::new(2_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, v));
    /// let conic = Conic::from_state(mu, state, 0.0, 1e-6).unwrap();
    /// assert!((conic.period_days().unwrap() - circular_period(mu, 2_000.0)).abs() < 1e-9);
    /// ```
    pub fn period_days(&self) -> Option<f64> {
        self.is_bound().then(|| TAU / self.shape().mean_motion)
    }

    /// Whether the body passes periapsis, its closest approach, at any
    /// time from `from_day` to `to_day` inclusive. On an ellipse that
    /// happens once a period; on a hyperbola once ever.
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// let conic = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 1.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 10.0,
    ///     sense: Sense::Prograde,
    /// };
    /// assert!(conic.passes_periapsis_between(5.0, 15.0));
    /// assert!(!conic.passes_periapsis_between(11.0, 1e9));
    /// ```
    pub fn passes_periapsis_between(&self, from_day: f64, to_day: f64) -> bool {
        match self.period_days() {
            Some(period) => {
                let turns = ((from_day - self.periapsis_day) / period).ceil();
                self.periapsis_day + turns * period <= to_day
            }
            None => from_day <= self.periapsis_day && self.periapsis_day <= to_day,
        }
    }

    /// The true anomaly on `day`: the angle, in the sense of motion, from
    /// periapsis round to the body, in radians in (-π, π].
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// let conic = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 0.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 3.0,
    ///     sense: Sense::Prograde,
    /// };
    /// assert!(conic.true_anomaly_at(3.0).abs() < 1e-12);
    /// let half = conic.period_days().unwrap() / 2.0;
    /// assert!((conic.true_anomaly_at(3.0 + half).abs() - std::f64::consts::PI).abs() < 1e-9);
    /// ```
    pub fn true_anomaly_at(&self, day: f64) -> f64 {
        let shape = self.shape();
        let mean = shape.mean_motion * (day - self.periapsis_day);
        shape.true_anomaly_at_mean(mean)
    }

    /// The true anomaly, from 0 to π, at which the path is `radius_km`
    /// from the primary. The body is that far out twice per loop: at this
    /// angle past periapsis on the way out, and at minus it on the way
    /// in. `None` if the path never comes to that distance (closer than
    /// periapsis, farther than an ellipse's apoapsis), or for a circle,
    /// which has no periapsis to measure from.
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// // An ellipse from 1,000 km to 3,000 km out.
    /// let conic = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 0.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 0.0,
    ///     sense: Sense::Prograde,
    /// };
    /// // 1,500 km out is a quarter turn past periapsis.
    /// let nu = conic.true_anomaly_at_radius(1_500.0).unwrap();
    /// assert!((nu - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    /// assert!(conic.true_anomaly_at_radius(500.0).is_none());
    /// assert!(conic.true_anomaly_at_radius(4_000.0).is_none());
    /// ```
    pub fn true_anomaly_at_radius(&self, radius_km: f64) -> Option<f64> {
        let e = self.eccentricity;
        if !(e > 0.0 && radius_km > 0.0) {
            return None;
        }
        // r = p / (1 + e cos ν), solved for cos ν.
        let cos_nu = (self.semi_latus_rectum_km / radius_km - 1.0) / e;
        (-1.0..=1.0).contains(&cos_nu).then(|| cos_nu.acos())
    }

    /// The first day after `after_day` on which the body is at true
    /// anomaly `nu` (radians, in the sense of motion from periapsis). On
    /// an ellipse that comes round once a period, so there is always one;
    /// a hyperbola passes each angle at most once, so `None` if it already
    /// has, or if the angle lies beyond the hyperbola's arms.
    ///
    /// ```
    /// use orbits::{Conic, Sense};
    ///
    /// let ellipse = Conic {
    ///     mu: 1e12,
    ///     semi_latus_rectum_km: 1_500.0,
    ///     eccentricity: 0.5,
    ///     periapsis_angle_rad: 0.0,
    ///     periapsis_day: 2.0,
    ///     sense: Sense::Prograde,
    /// };
    /// let period = ellipse.period_days().unwrap();
    /// // At periapsis on day 2, and again one period later.
    /// assert!((ellipse.next_day_at_true_anomaly(0.0, 1.9).unwrap() - 2.0).abs() < 1e-9);
    /// let again = ellipse.next_day_at_true_anomaly(0.0, 2.1).unwrap();
    /// assert!((again - (2.0 + period)).abs() < 1e-9);
    ///
    /// let hyperbola = Conic { eccentricity: 1.5, ..ellipse };
    /// assert!(hyperbola.next_day_at_true_anomaly(0.0, 1.0).is_some());
    /// assert!(hyperbola.next_day_at_true_anomaly(0.0, 2.5).is_none());
    /// ```
    pub fn next_day_at_true_anomaly(&self, nu: f64, after_day: f64) -> Option<f64> {
        let shape = self.shape();
        let nu = wrap_to_pi(nu);
        if !self.is_bound() {
            // A hyperbola's arms run out to ±arccos(−1/e).
            if nu.abs() >= (-1.0 / self.eccentricity).acos() {
                return None;
            }
            let day = self.periapsis_day + shape.mean_anomaly_at_true(nu) / shape.mean_motion;
            return (day > after_day).then_some(day);
        }
        let period = TAU / shape.mean_motion;
        let first = self.periapsis_day + shape.mean_anomaly_at_true(nu) / shape.mean_motion;
        let laps = ((after_day - first) / period).floor() + 1.0;
        let day = first + laps * period;
        // Rounding can land a whole period late or leave the day at
        // `after_day`; step back or on one period to the first after it.
        if day - period > after_day {
            Some(day - period)
        } else if day > after_day {
            Some(day)
        } else {
            Some(day + period)
        }
    }

    fn shape(&self) -> Shape {
        Shape::new(self.mu, self.semi_latus_rectum_km, self.eccentricity)
    }
}

impl Rail for Conic {
    /// Position (km) and velocity (km/day) on `day`, from Kepler's
    /// equation: the distance is *p* / (1 + *e* cos ν) at true anomaly ν,
    /// and the velocity splits into a radial part √(μ/p) *e* sin ν and a
    /// part along the path √(μ/p) (1 + *e* cos ν).
    fn state_at(&self, day: f64) -> State {
        let nu = self.true_anomaly_at(day);
        let e = self.eccentricity;
        let p = self.semi_latus_rectum_km;
        let (sin_nu, cos_nu) = nu.sin_cos();
        let radius = p / (1.0 + e * cos_nu);
        let angle = self.periapsis_angle_rad + nu;
        let (sin, cos) = angle.sin_cos();
        let radial = DVec3::new(cos, 0.0, sin);
        let along = DVec3::new(-sin, 0.0, cos);
        let scale = (self.mu / p).sqrt();
        let position = radius * radial;
        let velocity = scale * (e * sin_nu * radial + (1.0 + e * cos_nu) * along);
        match self.sense {
            Sense::Prograde => State { position, velocity },
            Sense::Retrograde => State {
                position: mirror(position),
                velocity: mirror(velocity),
            },
        }
    }
}

/// The parts of a conic's shape that Kepler's equation needs: whether it
/// is an ellipse or a hyperbola, its eccentricity and its mean motion
/// (the average rate of the anomaly, rad/day).
struct Shape {
    eccentricity: f64,
    mean_motion: f64,
}

impl Shape {
    fn new(mu: f64, p: f64, e: f64) -> Self {
        // The semi-major axis a = p / (1 − e²) is negative for a hyperbola;
        // the mean motion uses its size either way.
        let a = (p / (1.0 - e * e)).abs();
        Self {
            eccentricity: e,
            mean_motion: (mu / a.powi(3)).sqrt(),
        }
    }

    /// The mean anomaly for a true anomaly: Kepler's equation forwards,
    /// which needs no iteration.
    fn mean_anomaly_at_true(&self, nu: f64) -> f64 {
        let e = self.eccentricity;
        if e < 1.0 {
            let eccentric = 2.0
                * ((1.0 - e).sqrt() * (nu / 2.0).sin()).atan2((1.0 + e).sqrt() * (nu / 2.0).cos());
            eccentric - e * eccentric.sin()
        } else {
            let hyperbolic = 2.0 * (((e - 1.0) / (e + 1.0)).sqrt() * (nu / 2.0).tan()).atanh();
            e * hyperbolic.sinh() - hyperbolic
        }
    }

    /// The true anomaly for a mean anomaly: Kepler's equation backwards,
    /// solved by Newton's method.
    fn true_anomaly_at_mean(&self, mean: f64) -> f64 {
        let e = self.eccentricity;
        if e < 1.0 {
            let mean = wrap_to_pi(mean);
            let eccentric = solve_elliptic(mean, e);
            2.0 * ((1.0 + e).sqrt() * (eccentric / 2.0).sin())
                .atan2((1.0 - e).sqrt() * (eccentric / 2.0).cos())
        } else {
            let hyperbolic = solve_hyperbolic(mean, e);
            2.0 * ((e + 1.0).sqrt() * (hyperbolic / 2.0).sinh())
                .atan2((e - 1.0).sqrt() * (hyperbolic / 2.0).cosh())
        }
    }
}

/// Solves E − e sin E = M for the eccentric anomaly E, with M in (-π, π].
fn solve_elliptic(mean: f64, e: f64) -> f64 {
    // Danby's starting guess converges for every eccentricity below 1.
    let mut eccentric = mean + 0.85 * e * mean.sin().signum();
    for _ in 0..KEPLER_MAX_STEPS {
        let step = (eccentric - e * eccentric.sin() - mean) / (1.0 - e * eccentric.cos());
        eccentric -= step;
        if step.abs() < KEPLER_TOLERANCE {
            break;
        }
    }
    eccentric
}

/// Solves e sinh F − F = M for the hyperbolic anomaly F.
fn solve_hyperbolic(mean: f64, e: f64) -> f64 {
    let mut hyperbolic = (mean / e).asinh();
    for _ in 0..KEPLER_MAX_STEPS {
        let step = (e * hyperbolic.sinh() - hyperbolic - mean) / (e * hyperbolic.cosh() - 1.0);
        hyperbolic -= step;
        if step.abs() < KEPLER_TOLERANCE * hyperbolic.abs().max(1.0) {
            break;
        }
    }
    hyperbolic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::point_in_plane;
    use crate::units::{km_per_s_to_km_per_day, km3_per_s2_to_km3_per_day2};

    /// A Sun-like gravitational parameter, in km³/day².
    fn sun_mu() -> f64 {
        km3_per_s2_to_km3_per_day2(1.327e11)
    }

    /// A state at 1 AU, at `angle`, with speed `speed_kms` turned
    /// `tilt` radians from the tangent, in the given sense.
    fn state(angle: f64, speed_kms: f64, tilt: f64, sense: Sense) -> State {
        let position = point_in_plane(1.496e8, angle);
        let along = DVec3::new(-angle.sin(), 0.0, angle.cos()) * sense.sign();
        let radial = DVec3::new(angle.cos(), 0.0, angle.sin());
        let speed = km_per_s_to_km_per_day(speed_kms);
        State::new(position, speed * (tilt.cos() * along + tilt.sin() * radial))
    }

    fn assert_close(a: State, b: State) {
        let dp = (a.position - b.position).length();
        let dv = (a.velocity - b.velocity).length();
        assert!(dp < 1.0, "positions differ by {dp} km");
        assert!(dv < 1.0, "velocities differ by {dv} km/day");
    }

    /// RAIL-06: state → conic → state reproduces the state within 1 km
    /// and 1 km/day, for ellipses and hyperbolae in both senses.
    #[test]
    fn state_round_trips_through_a_conic_in_both_senses() {
        for sense in [Sense::Prograde, Sense::Retrograde] {
            for (speed, tilt) in [
                (29.8, 0.0),
                (25.0, 0.4),
                (34.0, -0.7),
                (50.0, 0.3),
                (60.0, 1.2),
            ] {
                let original = state(1.1, speed, tilt, sense);
                let conic = Conic::from_state(sun_mu(), original, 100.0, 1e-6).unwrap();
                assert_eq!(conic.sense, sense);
                assert_eq!(
                    conic.is_bound(),
                    speed < 42.1,
                    "escape speed at 1 AU is 42.1"
                );
                assert_close(conic.state_at(100.0), original);
            }
        }
    }

    /// A conic rebuilt from a later state is the same conic, and the
    /// motion is in the sense the state said (RAIL-05, RAIL-06).
    #[test]
    fn propagating_and_rebuilding_keeps_the_elements() {
        for sense in [Sense::Prograde, Sense::Retrograde] {
            for speed in [30.0, 48.0] {
                let conic =
                    Conic::from_state(sun_mu(), state(0.3, speed, 0.5, sense), 0.0, 1e-6).unwrap();
                let later = conic.state_at(40.0);
                assert_eq!(later.sense(), sense);
                let rebuilt = Conic::from_state(sun_mu(), later, 40.0, 1e-6).unwrap();
                let rel = |a: f64, b: f64| ((a - b) / b).abs();
                assert!(rel(rebuilt.semi_latus_rectum_km, conic.semi_latus_rectum_km) < 1e-9);
                assert!(rel(rebuilt.eccentricity, conic.eccentricity) < 1e-9);
                assert!(
                    wrap_to_pi(rebuilt.periapsis_angle_rad - conic.periapsis_angle_rad).abs()
                        < 1e-9
                );
                assert_close(rebuilt.state_at(0.0), conic.state_at(0.0));
            }
        }
    }

    /// An ellipse's distance from the primary stays between periapsis and
    /// apoapsis, and comes back to the start after one period.
    #[test]
    fn an_ellipse_repeats_every_period() {
        let conic =
            Conic::from_state(sun_mu(), state(0.0, 25.0, 0.3, Sense::Prograde), 5.0, 1e-6).unwrap();
        let period = conic.period_days().unwrap();
        let start = conic.state_at(5.0);
        assert_close(conic.state_at(5.0 + period), start);
        for i in 0..50 {
            let r = conic.state_at(5.0 + period * i as f64 / 50.0).radius();
            assert!(r >= conic.periapsis_radius_km() - 1e-3);
            assert!(r <= conic.apoapsis_radius_km().unwrap() + 1e-3);
        }
    }

    /// A hyperbola keeps going out, never returns, and has no period.
    #[test]
    fn a_hyperbola_never_returns() {
        let conic =
            Conic::from_state(sun_mu(), state(0.0, 60.0, 0.0, Sense::Prograde), 0.0, 1e-6).unwrap();
        assert!(conic.period_days().is_none());
        assert!(conic.apoapsis_radius_km().is_none());
        let mut last = 0.0;
        for day in [10.0, 100.0, 1_000.0, 10_000.0] {
            let r = conic.state_at(day).radius();
            assert!(r > last);
            last = r;
        }
    }

    /// RAIL-05: a state on the parabolic edge is nudged to the nearer side
    /// and still passes through its own position.
    #[test]
    fn a_parabolic_state_is_nudged_off_the_edge() {
        let escape = (2.0 * sun_mu() / 1.496e8).sqrt();
        let below = State::new(
            point_in_plane(1.496e8, 0.0),
            DVec3::new(0.0, 0.0, escape * (1.0 - 1e-9)),
        );
        let above = State::new(
            point_in_plane(1.496e8, 0.0),
            DVec3::new(0.0, 0.0, escape * (1.0 + 1e-9)),
        );
        let eps = 1e-6;
        let ellipse = Conic::from_state(sun_mu(), below, 0.0, eps).unwrap();
        let hyperbola = Conic::from_state(sun_mu(), above, 0.0, eps).unwrap();
        assert!((ellipse.eccentricity - (1.0 - eps)).abs() < 1e-15);
        assert!((hyperbola.eccentricity - (1.0 + eps)).abs() < 1e-15);
        for (conic, state) in [(ellipse, below), (hyperbola, above)] {
            let read = conic.state_at(0.0);
            assert!((read.position - state.position).length() < 1e-3);
        }
    }

    /// Periapsis passages repeat each period on an ellipse and happen once
    /// on a hyperbola.
    #[test]
    fn periapsis_passages_follow_the_period() {
        // Faster than circular and moving along the circle: at periapsis.
        let ellipse =
            Conic::from_state(sun_mu(), state(0.0, 34.0, 0.0, Sense::Prograde), 0.0, 1e-6).unwrap();
        let period = ellipse.period_days().unwrap();
        assert!(ellipse.passes_periapsis_between(-1.0, 1.0));
        assert!(!ellipse.passes_periapsis_between(1.0, period - 1.0));
        assert!(ellipse.passes_periapsis_between(1.0, period + 1.0));
        assert!(ellipse.passes_periapsis_between(2.5 * period, 4.0 * period));

        let hyperbola =
            Conic::from_state(sun_mu(), state(0.0, 60.0, 0.0, Sense::Prograde), 7.0, 1e-6).unwrap();
        assert!(hyperbola.passes_periapsis_between(6.0, 8.0));
        assert!(!hyperbola.passes_periapsis_between(8.0, 1e6));
    }

    /// The days a conic reaches a distance are the days it really is that
    /// far out, going out at +ν and coming in at −ν, each the first after
    /// the day asked from.
    #[test]
    fn the_day_at_a_distance_is_where_the_path_is_that_far_out() {
        let ellipse =
            Conic::from_state(sun_mu(), state(0.4, 25.0, 0.3, Sense::Prograde), 5.0, 1e-6).unwrap();
        let hyperbola = Conic::from_state(
            sun_mu(),
            state(0.4, 50.0, 0.3, Sense::Retrograde),
            5.0,
            1e-6,
        )
        .unwrap();
        for conic in [ellipse, hyperbola] {
            let r = 1.2 * conic.periapsis_radius_km();
            let nu = conic.true_anomaly_at_radius(r).unwrap();
            for sign in [1.0, -1.0] {
                let Some(day) = conic.next_day_at_true_anomaly(sign * nu, 5.0) else {
                    assert!(!conic.is_bound(), "an ellipse always comes round");
                    continue;
                };
                assert!(day > 5.0);
                let there = conic.state_at(day).radius();
                assert!((there - r).abs() < 1e-3 * r, "{there} vs {r}");
                let heading_out = conic
                    .state_at(day)
                    .position
                    .dot(conic.state_at(day).velocity);
                assert_eq!(heading_out > 0.0, sign > 0.0);
            }
        }
        let later = ellipse.next_day_at_true_anomaly(0.3, 400.0).unwrap();
        let period = ellipse.period_days().unwrap();
        assert!(later > 400.0 && later <= 400.0 + period);
    }

    /// A degenerate state (at the primary, radial, or non-finite) gives no
    /// conic rather than nonsense.
    #[test]
    fn degenerate_states_give_no_conic() {
        let mu = sun_mu();
        let at_origin = State::new(DVec3::ZERO, DVec3::X);
        let radial = State::new(DVec3::X * 1e8, DVec3::X * 1e5);
        let nan = State::new(DVec3::X * f64::NAN, DVec3::Z);
        assert!(Conic::from_state(mu, at_origin, 0.0, 1e-6).is_none());
        assert!(Conic::from_state(mu, radial, 0.0, 1e-6).is_none());
        assert!(Conic::from_state(mu, nan, 0.0, 1e-6).is_none());
    }
}
