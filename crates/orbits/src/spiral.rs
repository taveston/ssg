//! The low-thrust spiral: the path of a ship whose engine is too weak to
//! make short burns and instead pushes gently for the whole trip, so that
//! it winds slowly out (or in) from one circular orbit to another
//! (RAIL-07, XFER-06).
//!
//! The model is Edelbaum's estimate. The total change of speed needed to
//! go between two circular orbits this way is simply the difference of
//! their circular speeds, and the trip takes that difference divided by
//! the ship's acceleration. Along the way the ship is always nearly on a
//! circular orbit: its speed falls (climbing) or rises (descending) at a
//! steady rate, its distance is whatever circular orbit has that speed,
//! and it goes round at the local circular rate plus a small constant
//! correction that makes it arrive at the right angle.

use glam::DVec3;
use std::f64::consts::TAU;

use crate::circular::circular_speed;
use crate::state::State;
use crate::trajectory::Rail;

/// A low-thrust spiral from a circular orbit of radius `r0_km` at angle
/// `theta0_rad` to one of radius `r1_km`, arriving at angle `theta1_rad`,
/// pushed at a constant `accel` (km/day²) from `start_day`. Its length in
/// days follows from the two radii and the acceleration.
///
/// ```
/// use orbits::{Rail, Spiral, circular_speed};
///
/// // Round a star (μ = 1e21 km³/day²), from 1e8 km out to 1.5e8 km out,
/// // at an acceleration of 400 km/day², starting on day 10.
/// let mu = 1e21;
/// let spiral = Spiral::new(mu, 10.0, 1e8, 0.0, 1.5e8, 2.0, 400.0).unwrap();
/// let expected_days = (circular_speed(mu, 1e8) - circular_speed(mu, 1.5e8)) / 400.0;
/// assert!((spiral.duration_days() - expected_days).abs() < 1e-9);
/// // It starts on the inner orbit and ends on the outer one, at the asked angle.
/// assert!((spiral.state_at(10.0).radius() - 1e8).abs() < 1e-3);
/// let end = spiral.state_at(spiral.end_day());
/// assert!((end.radius() - 1.5e8).abs() < 1e-3);
/// assert!((end.angle() - 2.0).abs() < 1e-9);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spiral {
    /// The primary's gravitational parameter, km³/day².
    pub mu: f64,
    /// The day the spiral starts.
    pub start_day: f64,
    /// Radius of the starting circular orbit, km.
    pub r0_km: f64,
    /// Angle at the start, radians.
    pub theta0_rad: f64,
    /// Radius of the final circular orbit, km.
    pub r1_km: f64,
    /// Angle at the end, radians (as given; the spiral may wind any number
    /// of extra turns to get there).
    pub theta1_rad: f64,
    /// Speed at the start, km/day: the circular speed at `r0_km`.
    pub v0: f64,
    /// The constant acceleration, km/day².
    pub accel: f64,
    /// The spiral's length, days.
    duration_days: f64,
    /// The constant angular rate added to the circular rate so the
    /// spiral lands at `theta1_rad`, rad/day.
    phasing_rate: f64,
}

impl Spiral {
    /// Builds the spiral from the start orbit (`r0_km`, `theta0_rad`) to
    /// the end orbit (`r1_km`, at `theta1_rad` on the end day) round a
    /// primary with gravitational parameter `mu` (km³/day²), pushed at
    /// `accel` km/day² from `start_day`. The end angle is taken modulo a
    /// full turn: the spiral winds to the copy of `theta1_rad` nearest the
    /// angle it would reach with no correction, so the correction stays
    /// small.
    ///
    /// Returns `None` if the acceleration or a radius is not positive, or
    /// the radii are equal (there is nothing to spiral between).
    ///
    /// ```
    /// use orbits::Spiral;
    ///
    /// let mu = 1e21;
    /// assert!(Spiral::new(mu, 0.0, 1e8, 0.0, 1.5e8, 0.0, 400.0).is_some());
    /// assert!(Spiral::new(mu, 0.0, 1e8, 0.0, 1e8, 0.0, 400.0).is_none());
    /// assert!(Spiral::new(mu, 0.0, 1e8, 0.0, 1.5e8, 0.0, 0.0).is_none());
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mu: f64,
        start_day: f64,
        r0_km: f64,
        theta0_rad: f64,
        r1_km: f64,
        theta1_rad: f64,
        accel: f64,
    ) -> Option<Self> {
        if !(mu > 0.0 && r0_km > 0.0 && r1_km > 0.0 && accel > 0.0) || r0_km == r1_km {
            return None;
        }
        let v0 = circular_speed(mu, r0_km);
        let v1 = circular_speed(mu, r1_km);
        let duration_days = (v0 - v1).abs() / accel;
        let mut spiral = Self {
            mu,
            start_day,
            r0_km,
            theta0_rad,
            r1_km,
            theta1_rad,
            v0,
            accel,
            duration_days,
            phasing_rate: 0.0,
        };
        // With no correction the spiral reaches some angle; pick the copy
        // of the target angle nearest it and correct by the difference
        // spread over the whole trip.
        let uncorrected = theta0_rad + spiral.swept_angle(duration_days);
        let turns = ((theta1_rad - uncorrected) / TAU).round();
        spiral.theta1_rad = theta1_rad - turns * TAU;
        spiral.phasing_rate = (spiral.theta1_rad - uncorrected) / duration_days;
        Some(spiral)
    }

    /// How long the spiral takes, in days: the change of speed divided by
    /// the acceleration.
    ///
    /// ```
    /// use orbits::Spiral;
    ///
    /// let spiral = Spiral::new(1e21, 0.0, 1e8, 0.0, 4e8, 0.0, 1000.0).unwrap();
    /// // Circular speeds are 3.162e6 and 1.581e6 km/day: a difference of 1.581e6.
    /// assert!((spiral.duration_days() - 1581.1).abs() < 0.1);
    /// ```
    pub fn duration_days(&self) -> f64 {
        self.duration_days
    }

    /// The day the spiral ends and the ship is on the final orbit.
    ///
    /// ```
    /// use orbits::Spiral;
    ///
    /// let spiral = Spiral::new(1e21, 5.0, 1e8, 0.0, 4e8, 0.0, 1000.0).unwrap();
    /// assert!((spiral.end_day() - (5.0 + spiral.duration_days())).abs() < 1e-9);
    /// ```
    pub fn end_day(&self) -> f64 {
        self.start_day + self.duration_days
    }

    /// The total change of speed the spiral costs, in km/day: the
    /// difference between the two circular speeds (Edelbaum's estimate).
    ///
    /// ```
    /// use orbits::{Spiral, circular_speed};
    ///
    /// let spiral = Spiral::new(1e21, 0.0, 1e8, 0.0, 4e8, 0.0, 1000.0).unwrap();
    /// let expected = circular_speed(1e21, 1e8) - circular_speed(1e21, 4e8);
    /// assert!((spiral.delta_v() - expected).abs() < 1e-6);
    /// ```
    pub fn delta_v(&self) -> f64 {
        self.duration_days * self.accel
    }

    /// Whether the spiral climbs to a higher orbit (true) or descends to a
    /// lower one (false).
    ///
    /// ```
    /// use orbits::Spiral;
    ///
    /// assert!(Spiral::new(1e21, 0.0, 1e8, 0.0, 4e8, 0.0, 1000.0).unwrap().climbing());
    /// assert!(!Spiral::new(1e21, 0.0, 4e8, 0.0, 1e8, 0.0, 1000.0).unwrap().climbing());
    /// ```
    pub fn climbing(&self) -> bool {
        self.r1_km > self.r0_km
    }

    /// The sign of the speed's change: −1 climbing (slowing), +1
    /// descending (speeding up).
    fn speed_slope(&self) -> f64 {
        if self.climbing() { -1.0 } else { 1.0 }
    }

    /// Speed `s` days into the spiral, km/day.
    fn speed_at(&self, s: f64) -> f64 {
        self.v0 + self.speed_slope() * self.accel * s
    }

    /// The angle swept by going round at the local circular rate for the
    /// first `s` days, radians. The circular rate at speed v is v³/μ, and
    /// with v changing linearly its integral has a closed form.
    fn swept_angle(&self, s: f64) -> f64 {
        let v = self.speed_at(s);
        (v.powi(4) - self.v0.powi(4)) / (4.0 * self.speed_slope() * self.accel * self.mu)
    }
}

impl Rail for Spiral {
    /// Position (km) and velocity (km/day) on `day`. The radius is the
    /// circular orbit of the current speed, μ/v²; the angle is the start
    /// angle plus the swept angle plus the phasing correction; the
    /// velocity is the curve's rate of change. Before the start the
    /// state is the start state; after the end it is the end state.
    fn state_at(&self, day: f64) -> State {
        let s = (day - self.start_day).clamp(0.0, self.duration_days);
        let v = self.speed_at(s);
        let radius = self.mu / (v * v);
        let angle = self.theta0_rad + self.swept_angle(s) + self.phasing_rate * s;
        let (sin, cos) = angle.sin_cos();
        let radial = DVec3::new(cos, 0.0, sin);
        let along = DVec3::new(-sin, 0.0, cos);
        // dr/ds = −2μ/v³ · dv/ds, and dθ/ds = v³/μ + the phasing rate.
        let radius_rate = -2.0 * self.mu / v.powi(3) * self.speed_slope() * self.accel;
        let angle_rate = v.powi(3) / self.mu + self.phasing_rate;
        State {
            position: radius * radial,
            velocity: radius_rate * radial + radius * angle_rate * along,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::wrap_to_pi;

    const MU: f64 = 1e21;

    /// The spiral starts on the inner circle and ends on the outer one at
    /// the asked angle, climbing or descending (RAIL-07).
    #[test]
    fn spiral_joins_the_two_circles_at_the_asked_angle() {
        for (r0, r1) in [(1e8, 2e8), (2e8, 1e8)] {
            let spiral = Spiral::new(MU, 3.0, r0, 0.5, r1, 4.0, 300.0).unwrap();
            let start = spiral.state_at(3.0);
            assert!((start.radius() - r0).abs() < 1e-3);
            assert!((start.angle() - 0.5).abs() < 1e-9);
            let end = spiral.state_at(spiral.end_day());
            assert!((end.radius() - r1).abs() < 1e-3);
            assert!(wrap_to_pi(end.angle() - 4.0).abs() < 1e-9);
        }
    }

    /// The radius changes monotonically in between, and the angle always
    /// grows: the spiral winds prograde.
    #[test]
    fn radius_is_monotonic_and_the_angle_keeps_growing() {
        let spiral = Spiral::new(MU, 0.0, 1e8, 0.0, 3e8, 1.0, 200.0).unwrap();
        let mut last_radius = 0.0;
        let mut last_angle = f64::NEG_INFINITY;
        let n = 200;
        for i in 0..=n {
            let s = spiral.duration_days() * i as f64 / n as f64;
            let v = spiral.speed_at(s);
            let r = MU / (v * v);
            let angle = spiral.theta0_rad + spiral.swept_angle(s) + spiral.phasing_rate * s;
            assert!(r > last_radius);
            assert!(angle > last_angle);
            last_radius = r;
            last_angle = angle;
        }
    }

    /// The velocity is the time derivative of the position, checked by a
    /// finite difference.
    #[test]
    fn velocity_matches_the_change_of_position() {
        let spiral = Spiral::new(MU, 0.0, 1e8, 0.3, 2.5e8, 2.0, 250.0).unwrap();
        let h = 1e-3;
        for day in [10.0, 100.0, 500.0] {
            let before = spiral.state_at(day - h).position;
            let after = spiral.state_at(day + h).position;
            let numeric = (after - before) / (2.0 * h);
            let analytic = spiral.state_at(day).velocity;
            let err = (numeric - analytic).length() / analytic.length();
            assert!(err < 1e-6, "day {day}: relative error {err}");
        }
    }

    /// The phasing correction is small: the end angle is the copy of the
    /// target nearest the uncorrected arrival, never more than half a
    /// turn away.
    #[test]
    fn phasing_correction_is_at_most_half_a_turn_over_the_trip() {
        for theta1 in [0.0, 1.0, 3.0, -2.0, 20.0] {
            let spiral = Spiral::new(MU, 0.0, 1e8, 0.0, 2e8, theta1, 300.0).unwrap();
            let correction = spiral.phasing_rate * spiral.duration_days();
            assert!(correction.abs() <= std::f64::consts::PI + 1e-9);
            assert!(wrap_to_pi(spiral.theta1_rad - theta1).abs() < 1e-9);
        }
    }

    /// Outside its span the spiral holds its end states rather than
    /// running off the curve.
    #[test]
    fn outside_its_span_the_spiral_holds_its_ends() {
        let spiral = Spiral::new(MU, 5.0, 1e8, 0.0, 2e8, 1.0, 300.0).unwrap();
        assert_eq!(spiral.state_at(-100.0), spiral.state_at(5.0));
        assert_eq!(spiral.state_at(1e9), spiral.state_at(spiral.end_day()));
    }
}
