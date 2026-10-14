//! Lambert's problem: given where a ship is now and where it must be a
//! set number of days later, find the coasting path round the primary
//! that joins the two, and so the velocities at each end (XFER-03). This
//! is how a two-burn transfer is planned: the burn at departure is the
//! difference between the path's start velocity and the ship's, and the
//! burn at arrival the difference between the target's velocity and the
//! path's end velocity.
//!
//! Only the direct path is found: the one that goes round less than a
//! full turn in the given sense. The solver uses the universal-variable
//! form (Bate, Mueller and White; Curtis algorithm 5.2), which handles
//! ellipses and hyperbolae alike, and finds its one unknown by bisection.

use glam::DVec3;
use std::f64::consts::{PI, TAU};

use crate::state::{Sense, angle_of, mirror, point_in_plane, wrap_angle};

/// A transfer straight across the primary (180°) makes the solver's
/// formulas degenerate, so the arrival point is turned by this much
/// (radians) in that case. At 1e8 km it moves the point by 1 km.
const HALF_TURN_NUDGE_RAD: f64 = 1e-8;
/// Below this transfer angle the two points are on one ray from the
/// primary and no transfer exists.
const MIN_TRANSFER_ANGLE_RAD: f64 = 1e-9;
/// The unknown is bracketed from this far into the hyperbolic range. Past
/// it, cosh overflows; a transfer this fast is not worth having.
const MOST_HYPERBOLIC: f64 = -1e4;
/// Bisection stops when the bracket is this narrow.
const BISECTION_TOLERANCE: f64 = 1e-13;
/// And gives up after this many halvings.
const BISECTION_MAX_STEPS: usize = 400;

/// The answer to Lambert's problem: the velocity (km/day) a ship needs at
/// the start of the path and the velocity it has at the end.
///
/// ```
/// use glam::DVec3;
/// use orbits::{Sense, lambert};
///
/// // Round a star, from 1e8 km out on +x to 1e8 km out on +z in 100 days.
/// let mu = 1e21;
/// let arc = lambert(mu, DVec3::new(1e8, 0.0, 0.0), DVec3::new(0.0, 0.0, 1e8), 100.0, Sense::Prograde)
///     .unwrap();
/// // The start velocity heads towards +z; the end velocity towards −x.
/// assert!(arc.departure_velocity.z > 0.0);
/// assert!(arc.arrival_velocity.x < 0.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lambert {
    /// Velocity at the start of the path, km/day.
    pub departure_velocity: DVec3,
    /// Velocity at the end of the path, km/day.
    pub arrival_velocity: DVec3,
}

/// Solves Lambert's problem round a primary with gravitational parameter
/// `mu` (km³/day²): the direct coasting path from position `from_km` to
/// position `to_km` taking `days`, going round in `sense`. The path turns
/// through more than 0° and less than 360° in that sense; which of the
/// two ways round the primary it takes is fixed by the sense.
///
/// Returns `None` when there is no such path: the two points lie on one
/// ray from the primary, the time is not positive, or the trip is so
/// fast it falls outside the solver's range.
///
/// ```
/// use glam::DVec3;
/// use orbits::{Conic, Rail, Sense, State, lambert};
///
/// let mu = 1e21;
/// let from = DVec3::new(1e8, 0.0, 0.0);
/// let to = DVec3::new(-0.5e8, 0.0, 1.2e8);
/// let arc = lambert(mu, from, to, 80.0, Sense::Prograde).unwrap();
/// // Coasting from the start with that velocity really does arrive on time.
/// let path = Conic::from_state(mu, State::new(from, arc.departure_velocity), 0.0, 1e-6).unwrap();
/// let end = path.state_at(80.0);
/// assert!((end.position - to).length() < 1.0);
/// assert!((end.velocity - arc.arrival_velocity).length() < 1.0);
///
/// assert!(lambert(mu, from, from * 2.0, 80.0, Sense::Prograde).is_none());
/// ```
pub fn lambert(mu: f64, from_km: DVec3, to_km: DVec3, days: f64, sense: Sense) -> Option<Lambert> {
    if !is_positive(days) || !is_positive(mu) {
        return None;
    }
    // Solve in the frame where the sense is prograde; mirror back after.
    let (from, to) = match sense {
        Sense::Prograde => (from_km, to_km),
        Sense::Retrograde => (mirror(from_km), mirror(to_km)),
    };
    let solved = solve_prograde(mu, from, to, days)?;
    Some(match sense {
        Sense::Prograde => solved,
        Sense::Retrograde => Lambert {
            departure_velocity: mirror(solved.departure_velocity),
            arrival_velocity: mirror(solved.arrival_velocity),
        },
    })
}

/// The prograde solution: the path turning from `from` to `to` through
/// the angle measured from +x towards +z.
fn solve_prograde(mu: f64, from: DVec3, mut to: DVec3, days: f64) -> Option<Lambert> {
    let r1 = from.length();
    let r2 = to.length();
    if !(r1 > 0.0 && r2 > 0.0) || !from.is_finite() || !to.is_finite() {
        return None;
    }
    let mut angle = wrap_angle(angle_of(to) - angle_of(from));
    if angle < MIN_TRANSFER_ANGLE_RAD || TAU - angle < MIN_TRANSFER_ANGLE_RAD {
        return None;
    }
    if (angle - PI).abs() < HALF_TURN_NUDGE_RAD {
        angle = PI + HALF_TURN_NUDGE_RAD;
        to = point_in_plane(r2, angle_of(from) + angle);
    }

    // The geometry enters through one constant, A, which is positive for
    // transfers under half a turn and negative above.
    let a = angle.sin() * (r1 * r2 / (1.0 - angle.cos())).sqrt();
    // y(z) is a length built from the two radii and the unknown z; the time
    // of flight for a z is time(z). The unknown z measures how bound the
    // path is: negative for a hyperbola, zero for a parabola, positive for
    // an ellipse, up to 4π² at a full turn.
    let y = |z: f64| r1 + r2 + a * (z * stumpff_s(z) - 1.0) / stumpff_c(z).sqrt();
    let time = |z: f64| {
        let y = y(z);
        if !is_positive(y) {
            return None;
        }
        Some(((y / stumpff_c(z)).powf(1.5) * stumpff_s(z) + a * y.sqrt()) / mu.sqrt())
    };
    // The time of flight grows with z, so bracket the wanted time and
    // bisect. A z too small to give a positive y counts as "too fast".
    let too_slow = |z: f64| time(z).is_some_and(|t| t >= days);
    let mut hi = 4.0 * PI * PI - 1e-9;
    let mut lo = 0.0;
    while too_slow(lo) {
        lo = if lo == 0.0 { -1.0 } else { lo * 2.0 };
        if lo < MOST_HYPERBOLIC {
            return None;
        }
    }
    for _ in 0..BISECTION_MAX_STEPS {
        let mid = 0.5 * (lo + hi);
        if too_slow(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
        if hi - lo < BISECTION_TOLERANCE * hi.abs().max(1.0) {
            break;
        }
    }
    let z = 0.5 * (lo + hi);
    let y = y(z);
    if !is_positive(y) {
        return None;
    }
    // Lagrange coefficients turn the two positions into the two
    // velocities.
    let f = 1.0 - y / r1;
    let g = a * (y / mu).sqrt();
    let g_dot = 1.0 - y / r2;
    if g == 0.0 || !g.is_finite() {
        return None;
    }
    Some(Lambert {
        departure_velocity: (to - f * from) / g,
        arrival_velocity: (g_dot * to - from) / g,
    })
}

/// A finite number above zero (so not NaN, which fails every comparison).
fn is_positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Below this size of z the Stumpff functions use their series, which
/// avoids cancellation near zero.
const STUMPFF_SERIES_BELOW: f64 = 1e-3;

/// Stumpff's C(z): (1 − cos √z) / z, continued smoothly through z = 0 and
/// into negative z with cosh.
fn stumpff_c(z: f64) -> f64 {
    if z.abs() < STUMPFF_SERIES_BELOW {
        0.5 - z / 24.0 + z * z / 720.0 - z * z * z / 40_320.0 + z * z * z * z / 3_628_800.0
    } else if z > 0.0 {
        (1.0 - z.sqrt().cos()) / z
    } else {
        ((-z).sqrt().cosh() - 1.0) / (-z)
    }
}

/// Stumpff's S(z): (√z − sin √z) / √z³, continued likewise.
fn stumpff_s(z: f64) -> f64 {
    if z.abs() < STUMPFF_SERIES_BELOW {
        1.0 / 6.0 - z / 120.0 + z * z / 5_040.0 - z * z * z / 362_880.0
            + z * z * z * z / 39_916_800.0
    } else if z > 0.0 {
        let s = z.sqrt();
        (s - s.sin()) / (s * s * s)
    } else {
        let s = (-z).sqrt();
        (s.sinh() - s) / (s * s * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circular::{circular_speed, hohmann};
    use crate::conic::Conic;
    use crate::state::State;
    use crate::trajectory::Rail;
    use crate::units::km3_per_s2_to_km3_per_day2;

    fn sun_mu() -> f64 {
        km3_per_s2_to_km3_per_day2(1.327e11)
    }

    /// Coasting from the start with the solved velocity reaches the end
    /// point on time with the solved velocity, for transfer angles all
    /// round the circle and in both senses (XFER-03).
    #[test]
    fn solved_arcs_arrive_on_time_in_both_senses() {
        let mu = sun_mu();
        let from = point_in_plane(1.5e8, 0.2);
        for sense in [Sense::Prograde, Sense::Retrograde] {
            for angle in [0.3, 1.5, 2.9, 3.3, 4.5, 6.0] {
                for days in [40.0, 150.0, 400.0] {
                    let to = point_in_plane(2.2e8, 0.2 + sense.sign() * angle);
                    let arc = lambert(mu, from, to, days, sense).unwrap();
                    let path =
                        Conic::from_state(mu, State::new(from, arc.departure_velocity), 0.0, 1e-6)
                            .unwrap();
                    assert_eq!(path.sense, sense);
                    let end = path.state_at(days);
                    let miss = (end.position - to).length();
                    assert!(
                        miss < 1.0,
                        "{sense:?} {angle} rad {days} d: missed by {miss} km"
                    );
                    let dv = (end.velocity - arc.arrival_velocity).length();
                    assert!(dv < 1.0, "velocity off by {dv} km/day");
                }
            }
        }
    }

    /// At exactly half a turn and the Hohmann flight time, Lambert gives
    /// the Hohmann burns.
    #[test]
    fn a_half_turn_arc_is_the_hohmann_transfer() {
        let mu = sun_mu();
        let (r1, r2) = (1.0e8, 1.6e8);
        let trip = hohmann(mu, r1, r2);
        let from = point_in_plane(r1, 0.7);
        let to = point_in_plane(r2, 0.7 + PI);
        let arc = lambert(mu, from, to, trip.days, Sense::Prograde).unwrap();
        let v1 = circular_speed(mu, r1) * DVec3::new(-(0.7f64).sin(), 0.0, (0.7f64).cos());
        let burn1 = (arc.departure_velocity - v1).length();
        let v2 = -circular_speed(mu, r2) * DVec3::new(-(0.7f64).sin(), 0.0, (0.7f64).cos());
        let burn2 = (v2 - arc.arrival_velocity).length();
        assert!(
            (burn1 - trip.first_burn).abs() < 1e-4 * trip.first_burn,
            "{burn1}"
        );
        assert!(
            (burn2 - trip.second_burn).abs() < 1e-4 * trip.second_burn,
            "{burn2}"
        );
    }

    /// Points on one ray and a non-positive time give no arc; a very fast
    /// trip is still solved, as a hyperbola.
    #[test]
    fn impossible_problems_give_no_arc() {
        let mu = sun_mu();
        let from = DVec3::new(1e8, 0.0, 0.0);
        let to = DVec3::new(0.0, 0.0, 1e8);
        assert!(lambert(mu, from, from * 3.0, 50.0, Sense::Prograde).is_none());
        assert!(lambert(mu, from, to, 0.0, Sense::Prograde).is_none());
        let fast = lambert(mu, from, to, 1.0, Sense::Prograde).unwrap();
        let path =
            Conic::from_state(mu, State::new(from, fast.departure_velocity), 0.0, 1e-6).unwrap();
        assert!(!path.is_bound());
    }

    /// The Stumpff functions are continuous across the series boundary.
    #[test]
    fn stumpff_functions_are_continuous() {
        for z in [STUMPFF_SERIES_BELOW, -STUMPFF_SERIES_BELOW] {
            let inside = z * (1.0 - 1e-9);
            let outside = z * (1.0 + 1e-9);
            assert!((stumpff_c(inside) - stumpff_c(outside)).abs() < 1e-12);
            assert!((stumpff_s(inside) - stumpff_s(outside)).abs() < 1e-12);
        }
    }
}
