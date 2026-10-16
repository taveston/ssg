//! Passing close by a body on a hyperbola: where a ship arriving from far
//! away comes closest, and where a ship leaving has to start from.
//!
//! A ship that falls towards a body from far away, with some speed left
//! over (the **excess speed**, *v∞*: its speed relative to the body
//! before the body's gravity takes hold), follows a **hyperbola**: an
//! open curve that swings round the body once and leaves again. How
//! tightly it swings is its **eccentricity** *e*: just over 1 for a slow
//! pass that wraps far round the body, large for a fast one that is
//! barely bent. The point of the curve nearest the body is where a ship
//! can brake into a circular orbit (capture), and the mirror case, the
//! point a ship in a circular orbit must burn from to leave in a given
//! direction, is where it exits. A ship that doesn't brake goes on round
//! and leaves at the speed it came with, its direction turned by the
//! pass: the faster and farther out, the smaller the turn.
//!
//! Every pass here is **prograde**: it goes round the body the way the
//! angle grows, as a body's own orbits do.

use glam::DVec3;
use std::f64::consts::PI;

use crate::circular::circular_speed;
use crate::state::{angle_of, point_in_plane};

/// The eccentricity of the hyperbola whose closest point to a body is
/// `radius_km` from it, for an excess speed of `v_inf` km/day round a
/// body of gravitational parameter `mu` (km³/day²): 1 + r v∞² ÷ μ. A
/// body with no gravity doesn't bend the path at all, so the result is
/// infinite.
///
/// ```
/// use orbits::{km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day, pass_eccentricity};
///
/// // 5 km/s past an Earth-like planet, 7,000 km from its centre.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let e = pass_eccentricity(mu, 7_000.0, km_per_s_to_km_per_day(5.0));
/// assert!((e - 1.44).abs() < 0.01);
/// ```
pub fn pass_eccentricity(mu: f64, radius_km: f64, v_inf: f64) -> f64 {
    if mu <= 0.0 {
        return f64::INFINITY;
    }
    1.0 + radius_km * v_inf * v_inf / mu
}

/// How far round the body the closest point lies from the direction of
/// the excess velocity, in radians: arccos(1 ÷ e). A fast pass (large e)
/// comes closest nearly a quarter turn from its line of approach; a slow
/// one (e near 1) much sooner.
fn half_bend(mu: f64, radius_km: f64, v_inf: DVec3) -> f64 {
    let e = pass_eccentricity(mu, radius_km, v_inf.length());
    (1.0 / e).clamp(-1.0, 1.0).acos()
}

/// Where a ship arriving at a body comes closest to it, as an angle round
/// the body (radians from +x towards +z): the **capture point**, where a
/// single burn would leave it on a circular orbit of `radius_km`.
/// `v_inf` is the ship's velocity relative to the body as it arrives
/// from far away (km/day), and `mu` the body's gravitational parameter
/// (km³/day²). The ship goes round the body prograde, so the point is
/// arccos(1 ÷ e) short of the direction it is travelling, with e from
/// [`pass_eccentricity`]. The result is not wrapped to any range.
///
/// ```
/// use glam::DVec3;
/// use orbits::{capture_angle, km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day};
///
/// // Arriving along +x at 5 km/s at an Earth-like planet, braking 7,000 km
/// // from its centre: the closest point is about 46° before +x.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let v_inf = DVec3::new(km_per_s_to_km_per_day(5.0), 0.0, 0.0);
/// let angle = capture_angle(mu, 7_000.0, v_inf);
/// assert!((angle.to_degrees() + 46.0).abs() < 1.0);
/// ```
pub fn capture_angle(mu: f64, radius_km: f64, v_inf: DVec3) -> f64 {
    angle_of(v_inf) - half_bend(mu, radius_km, v_inf)
}

/// Where a ship in a circular orbit of `radius_km` must be when it burns
/// to leave a body with the excess velocity `v_inf` (km/day, relative to
/// the body, once far away), as an angle round the body (radians from +x
/// towards +z): the **exit point**. It is the capture point of the same
/// pass run backwards: arccos(1 ÷ e) − π from the direction of departure,
/// with e from [`pass_eccentricity`] and `mu` the body's gravitational
/// parameter (km³/day²). The result is not wrapped to any range.
///
/// ```
/// use glam::DVec3;
/// use orbits::{exit_angle, km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day};
///
/// // Leaving along +x at 5 km/s from 7,000 km out round an Earth-like
/// // planet: burn about 134° before +x.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let v_inf = DVec3::new(km_per_s_to_km_per_day(5.0), 0.0, 0.0);
/// let angle = exit_angle(mu, 7_000.0, v_inf);
/// assert!((angle.to_degrees() + 134.0).abs() < 1.0);
/// ```
pub fn exit_angle(mu: f64, radius_km: f64, v_inf: DVec3) -> f64 {
    angle_of(v_inf) + half_bend(mu, radius_km, v_inf) - PI
}

/// How far a pass turns the ship's direction of travel, in radians: the
/// angle between the excess velocity it arrives with and the one it
/// leaves with, 2 arcsin(1 ÷ e), with e from [`pass_eccentricity`] for a
/// closest distance of `radius_km`, an excess speed of `v_inf` km/day and
/// a body of gravitational parameter `mu` (km³/day²). A slow, close pass
/// turns a long way (up to half a turn as the excess speed falls to
/// nothing); a fast or distant one is barely bent, and a body with no
/// gravity bends nothing.
///
/// ```
/// use orbits::{km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day, turn_angle};
///
/// // 5 km/s past an Earth-like planet, 7,000 km from its centre: the
/// // path turns by about 88°.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let turn = turn_angle(mu, 7_000.0, km_per_s_to_km_per_day(5.0));
/// assert!((turn.to_degrees() - 88.0).abs() < 1.0);
/// ```
pub fn turn_angle(mu: f64, radius_km: f64, v_inf: f64) -> f64 {
    let e = pass_eccentricity(mu, radius_km, v_inf);
    2.0 * (1.0 / e).clamp(-1.0, 1.0).asin()
}

/// The excess velocity a ship leaves a body with after a pass whose
/// closest distance is `radius_km`, given the excess velocity `v_inf` it
/// arrived with (km/day, relative to the body) and the body's
/// gravitational parameter `mu` (km³/day²). The pass goes round the body
/// prograde, so the velocity is turned the way the angle grows, by
/// [`turn_angle`]; its size is unchanged, because the body gives back on
/// the way out what it lent on the way in.
///
/// ```
/// use glam::DVec3;
/// use orbits::{angle_of, km3_per_s2_to_km3_per_day2, km_per_s_to_km_per_day, turned_velocity};
///
/// // Arriving along +x at 5 km/s, 7,000 km from an Earth-like planet's
/// // centre: it leaves at the same speed, about 88° round towards +z.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let v_inf = DVec3::new(km_per_s_to_km_per_day(5.0), 0.0, 0.0);
/// let out = turned_velocity(mu, 7_000.0, v_inf);
/// assert!((out.length() - v_inf.length()).abs() < 1e-6);
/// assert!((angle_of(out).to_degrees() - 88.0).abs() < 1.0);
/// ```
pub fn turned_velocity(mu: f64, radius_km: f64, v_inf: DVec3) -> DVec3 {
    let speed = v_inf.length();
    point_in_plane(speed, angle_of(v_inf) + turn_angle(mu, radius_km, speed))
}

/// The speed of a pass at its closest point, relative to the body, in
/// km/day: √(v∞² + 2μ ÷ r), the excess speed `v_inf` (km/day) plus what
/// the body's gravity (`mu`, km³/day²) adds by the closest distance
/// `radius_km`.
///
/// ```
/// use orbits::{km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s, km_per_s_to_km_per_day, pass_speed};
///
/// // 5 km/s from afar is about 11.8 km/s at 7,000 km from an Earth-like
/// // planet's centre.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let fast = pass_speed(mu, 7_000.0, km_per_s_to_km_per_day(5.0));
/// assert!((km_per_day_to_km_per_s(fast) - 11.8).abs() < 0.1);
/// ```
pub fn pass_speed(mu: f64, radius_km: f64, v_inf: f64) -> f64 {
    (v_inf * v_inf + 2.0 * mu / radius_km).sqrt()
}

/// The speed at which a pass goes by something in a circular orbit of
/// `radius_km` that is at the pass's closest point as the ship gets
/// there, in km/day. The pass is prograde, as the orbit is, so at that
/// point both move the same way and the closing speed is the pass's
/// speed ([`pass_speed`]) less the orbit's circular speed. `v_inf` is the
/// excess speed (km/day) and `mu` the body's gravitational parameter
/// (km³/day²).
///
/// ```
/// use orbits::{closing_speed, km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s, km_per_s_to_km_per_day};
///
/// // 5 km/s from afar, past a station 7,000 km from an Earth-like
/// // planet's centre (circling at 7.5 km/s): it goes by at about 4.2 km/s.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let closing = closing_speed(mu, 7_000.0, km_per_s_to_km_per_day(5.0));
/// assert!((km_per_day_to_km_per_s(closing) - 4.2).abs() < 0.1);
/// ```
pub fn closing_speed(mu: f64, radius_km: f64, v_inf: f64) -> f64 {
    pass_speed(mu, radius_km, v_inf) - circular_speed(mu, radius_km)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conic::Conic;
    use crate::state::State;
    use crate::trajectory::Rail;
    use crate::units::km3_per_s2_to_km3_per_day2;

    fn earth_mu() -> f64 {
        km3_per_s2_to_km3_per_day2(3.986e5)
    }

    /// The capture point is the periapsis of the prograde hyperbola that
    /// arrives with the given excess velocity: a conic started there, at
    /// the hyperbola's periapsis speed and moving prograde, is travelling
    /// along the excess velocity once far away, in the past.
    #[test]
    fn the_capture_point_is_the_closest_point_of_the_arriving_hyperbola() {
        let mu = earth_mu();
        let r = 8_000.0;
        let v_inf = DVec3::new(-300_000.0, 0.0, 200_000.0);
        let angle = capture_angle(mu, r, v_inf);
        let speed = (v_inf.length_squared() + 2.0 * mu / r).sqrt();
        let position = point_in_plane(r, angle);
        let along = point_in_plane(1.0, angle + PI / 2.0);
        let pass = Conic::from_state(mu, State::new(position, speed * along), 0.0, 1e-9).unwrap();
        let far = pass.state_at(-30.0).velocity;
        assert!(
            far.normalize().dot(v_inf.normalize()) > 0.9999,
            "{far} vs {v_inf}"
        );
    }

    /// The exit point is the periapsis of the prograde hyperbola that
    /// leaves with the given excess velocity: far in the future, a conic
    /// started there is travelling along it.
    #[test]
    fn the_exit_point_is_the_closest_point_of_the_leaving_hyperbola() {
        let mu = earth_mu();
        let r = 8_000.0;
        let v_inf = DVec3::new(100_000.0, 0.0, 400_000.0);
        let angle = exit_angle(mu, r, v_inf);
        let speed = (v_inf.length_squared() + 2.0 * mu / r).sqrt();
        let position = point_in_plane(r, angle);
        let along = point_in_plane(1.0, angle + PI / 2.0);
        let pass = Conic::from_state(mu, State::new(position, speed * along), 0.0, 1e-9).unwrap();
        let far = pass.state_at(30.0).velocity;
        assert!(
            far.normalize().dot(v_inf.normalize()) > 0.9999,
            "{far} vs {v_inf}"
        );
    }

    /// A massless body bends nothing: the hyperbola is a straight line,
    /// whose closest point is a quarter turn from its direction.
    #[test]
    fn a_massless_body_bends_nothing() {
        assert!(pass_eccentricity(0.0, 1.0, 1.0).is_infinite());
        let v_inf = DVec3::new(1.0, 0.0, 0.0);
        assert!((capture_angle(0.0, 10.0, v_inf) + PI / 2.0).abs() < 1e-12);
    }
    /// A pass leaves the body along the far end of the same hyperbola it
    /// arrived on: a conic through the capture point is travelling along
    /// the turned velocity far in the future, at the speed it arrived
    /// with.
    #[test]
    fn a_pass_leaves_along_the_turned_velocity() {
        let mu = earth_mu();
        let r = 8_000.0;
        let v_inf = DVec3::new(-300_000.0, 0.0, 200_000.0);
        let angle = capture_angle(mu, r, v_inf);
        let position = point_in_plane(r, angle);
        let along = point_in_plane(1.0, angle + PI / 2.0);
        let speed = pass_speed(mu, r, v_inf.length());
        let pass = Conic::from_state(mu, State::new(position, speed * along), 0.0, 1e-9).unwrap();
        let far = pass.state_at(30.0).velocity;
        let out = turned_velocity(mu, r, v_inf);
        assert!((out.length() - v_inf.length()).abs() < 1e-6);
        assert!(
            far.normalize().dot(out.normalize()) > 0.9999,
            "{far} vs {out}"
        );
        // Turned prograde: from the arrival direction round the way the
        // angle grows.
        let turned = crate::state::wrap_angle(angle_of(out) - angle_of(v_inf));
        assert!((turned - turn_angle(mu, r, v_inf.length())).abs() < 1e-9);
    }

    /// A massless body turns nothing, and a pass with no excess speed
    /// turns half a turn.
    #[test]
    fn the_turn_runs_from_nothing_to_half_a_turn() {
        assert_eq!(turn_angle(0.0, 10.0, 1.0), 0.0);
        assert!((turn_angle(earth_mu(), 7_000.0, 0.0) - PI).abs() < 1e-12);
        let v_inf = DVec3::new(1.0, 0.0, 2.0);
        assert_eq!(
            turned_velocity(0.0, 10.0, v_inf),
            point_in_plane(v_inf.length(), angle_of(v_inf))
        );
    }
}
