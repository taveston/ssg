//! Phasing: catching up with something in your own circular orbit.
//!
//! Two things on one circular orbit go round at the same rate, so the
//! gap between them never changes on its own. To close it, a ship burns
//! at one point of the orbit onto a slightly different ellipse (a
//! **phasing orbit**) that still touches the circle at that point, flies
//! whole laps of it, and burns back at the same point just as the other
//! thing comes round to meet it.
//!
//! A smaller ellipse (the **inner** orbit, whose farthest point is the
//! burn point) has a shorter period, so the ship laps ahead; a larger one
//! (the **outer** orbit, whose closest point is the burn point) has a
//! longer period, so the ship falls back and lets the other thing come
//! round. More laps mean a gentler ellipse and smaller burns: the trade
//! is time for fuel.

use std::f64::consts::TAU;

use crate::circular::circular_speed;

/// Which of the two phasing orbits: smaller than the circle, or larger.
///
/// ```
/// use orbits::PhasingSide;
///
/// let side = PhasingSide::Inner;
/// assert_ne!(side, PhasingSide::Outer);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhasingSide {
    /// Inside the circle, with a shorter period: the ship laps ahead. The
    /// burn point is the ellipse's farthest point.
    Inner,
    /// Outside the circle, with a longer period: the ship drops back. The
    /// burn point is the ellipse's closest point.
    Outer,
}

/// How long until the thing ahead next passes the burn point, for the
/// `k`th time: the period times (k − gap), in days. `period_days` is the
/// circular orbit's period and `gap_turns` how far ahead the other thing
/// is, as a fraction of a turn in [0, 1) in the sense of motion. The
/// first meeting (k = 1) is when it has come the rest of the way round.
///
/// ```
/// use orbits::meeting_days;
///
/// // Half a turn ahead on a 2-day orbit: it comes round after 1 day,
/// // then every 2 days after that.
/// assert_eq!(meeting_days(2.0, 0.5, 1), 1.0);
/// assert_eq!(meeting_days(2.0, 0.5, 3), 5.0);
/// ```
pub fn meeting_days(period_days: f64, gap_turns: f64, k: u32) -> f64 {
    period_days * (f64::from(k) - gap_turns)
}

/// A phasing orbit: the ellipse a ship flies whole laps of to meet
/// something in its own circular orbit, with its size and the burn into
/// and out of it.
///
/// ```
/// use orbits::{PhasingSide, km3_per_s2_to_km3_per_day2, phasing_orbit};
///
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let orbit = phasing_orbit(mu, 42_164.0, 0.25, 2, PhasingSide::Outer).unwrap();
/// assert_eq!(orbit.laps, 1);
/// assert_eq!(orbit.closest_km, 42_164.0);
/// assert!(orbit.farthest_km > 42_164.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhasingOrbit {
    /// Smaller or larger than the circle.
    pub side: PhasingSide,
    /// Whole laps of the ellipse flown before meeting.
    pub laps: u32,
    /// One lap of the ellipse, in days.
    pub period_days: f64,
    /// Half the ellipse's longest diameter, in km: its average distance.
    pub semi_major_km: f64,
    /// The ellipse's closest point to the primary, in km.
    pub closest_km: f64,
    /// The ellipse's farthest point from the primary, in km.
    pub farthest_km: f64,
    /// The speed on the ellipse at the burn point, in km/day, along the
    /// circle's direction of motion.
    pub burn_point_speed: f64,
    /// The size of each of the two burns (into the ellipse, and back onto
    /// the circle), in km/day.
    pub burn: f64,
}

impl PhasingOrbit {
    /// Both burns together, in km/day: the whole cost of the phasing.
    ///
    /// ```
    /// use orbits::{PhasingSide, km3_per_s2_to_km3_per_day2, phasing_orbit};
    ///
    /// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
    /// let orbit = phasing_orbit(mu, 42_164.0, 0.25, 2, PhasingSide::Inner).unwrap();
    /// assert_eq!(orbit.delta_v(), 2.0 * orbit.burn);
    /// ```
    pub fn delta_v(&self) -> f64 {
        2.0 * self.burn
    }
}

/// The phasing orbit that meets something `gap_turns` of a turn ahead
/// (in [0, 1), in the sense of motion) on a circular orbit of `radius_km`
/// round a primary of gravitational parameter `mu` (km³/day²), at its
/// `k`th pass of the burn point ([`meeting_days`]). The inner orbit flies
/// k laps in that time; the outer flies k − 1, so it needs k ≥ 2. Each
/// lap takes the meeting time over the laps, and Kepler's third law gives
/// the size: the semi-major axis is the radius times (lap ÷ circle's
/// period)^⅔. Each burn is the difference between the circular speed and
/// the ellipse's speed at the burn point (from the vis-viva equation,
/// v² = μ (2 ÷ r − 1 ÷ a)).
///
/// `None` for k = 0, for an outer orbit with k < 2, and for an inner
/// orbit so small it would pass through the primary's centre. Whether the
/// ellipse clears the primary's surface is the caller's to check, from
/// `closest_km`.
///
/// ```
/// use orbits::{PhasingSide, km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s, phasing_orbit};
///
/// // A geostationary satellite catching one a quarter turn ahead, at its
/// // fourth pass: four laps of a smaller orbit, or three of a larger one.
/// let mu = km3_per_s2_to_km3_per_day2(3.986e5);
/// let inner = phasing_orbit(mu, 42_164.0, 0.25, 4, PhasingSide::Inner).unwrap();
/// let outer = phasing_orbit(mu, 42_164.0, 0.25, 4, PhasingSide::Outer).unwrap();
/// assert_eq!((inner.laps, outer.laps), (4, 3));
/// assert!(inner.period_days < 1.0 && outer.period_days > 1.0);
/// assert!(km_per_day_to_km_per_s(inner.delta_v()) < 0.2);
/// assert!(phasing_orbit(mu, 42_164.0, 0.25, 1, PhasingSide::Outer).is_none());
/// ```
pub fn phasing_orbit(
    mu: f64,
    radius_km: f64,
    gap_turns: f64,
    k: u32,
    side: PhasingSide,
) -> Option<PhasingOrbit> {
    let laps = match side {
        PhasingSide::Inner => k,
        PhasingSide::Outer => k.checked_sub(1)?,
    };
    if laps == 0 || mu <= 0.0 || radius_km <= 0.0 {
        return None;
    }
    let circle_period = TAU * (radius_km.powi(3) / mu).sqrt();
    let period_days = meeting_days(circle_period, gap_turns, k) / f64::from(laps);
    let semi_major_km = radius_km * (period_days / circle_period).powf(2.0 / 3.0);
    let other_end_km = 2.0 * semi_major_km - radius_km;
    if other_end_km <= 0.0 {
        return None;
    }
    let burn_point_speed = (mu * (2.0 / radius_km - 1.0 / semi_major_km)).sqrt();
    let (closest_km, farthest_km) = match side {
        PhasingSide::Inner => (other_end_km, radius_km),
        PhasingSide::Outer => (radius_km, other_end_km),
    };
    Some(PhasingOrbit {
        side,
        laps,
        period_days,
        semi_major_km,
        closest_km,
        farthest_km,
        burn_point_speed,
        burn: (burn_point_speed - circular_speed(mu, radius_km)).abs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circular::{CircularOrbit, circular_period};
    use crate::conic::Conic;
    use crate::state::State;
    use crate::trajectory::Rail;
    use crate::units::km3_per_s2_to_km3_per_day2;

    fn earth_mu() -> f64 {
        km3_per_s2_to_km3_per_day2(3.986e5)
    }

    /// A ship that burns onto the phasing orbit and flies its laps is back
    /// at the burn point just as the thing ahead arrives there.
    #[test]
    fn after_its_laps_the_ship_meets_the_target_at_the_burn_point() {
        let mu = earth_mu();
        let r = 20_000.0;
        let gap = 0.3;
        for (k, side) in [(1, PhasingSide::Inner), (3, PhasingSide::Outer)] {
            let orbit = phasing_orbit(mu, r, gap, k, side).unwrap();
            let ship = CircularOrbit::new(mu, r, 0.0);
            let target = CircularOrbit::new(mu, r, gap * TAU);
            let start = ship.state_at(0.0);
            let after_burn = State::new(
                start.position,
                start.velocity.normalize() * orbit.burn_point_speed,
            );
            let conic = Conic::from_state(mu, after_burn, 0.0, 1e-9).unwrap();
            let meet = meeting_days(ship.period_days, gap, k);
            let ship_then = conic.state_at(meet).position;
            let target_then = target.position_at(meet);
            assert!(
                (ship_then - target_then).length() < 1.0,
                "{side:?} k={k}: {}",
                (ship_then - target_then).length()
            );
            assert!((conic.period_days().unwrap() - orbit.period_days).abs() < 1e-9);
        }
    }

    /// The inner orbit's burn point is its farthest point and the outer
    /// orbit's its closest, and each lap matches Kepler's third law.
    #[test]
    fn the_burn_point_is_an_end_of_the_ellipse() {
        let mu = earth_mu();
        let inner = phasing_orbit(mu, 10_000.0, 0.5, 2, PhasingSide::Inner).unwrap();
        let outer = phasing_orbit(mu, 10_000.0, 0.5, 2, PhasingSide::Outer).unwrap();
        assert_eq!(inner.farthest_km, 10_000.0);
        assert_eq!(outer.closest_km, 10_000.0);
        for orbit in [inner, outer] {
            let expected = circular_period(mu, orbit.semi_major_km);
            assert!((orbit.period_days - expected).abs() < 1e-9);
        }
    }

    /// An inner orbit that would have to dive through the primary's
    /// centre doesn't exist, and nor does k = 0.
    #[test]
    fn impossible_orbits_are_none() {
        let mu = earth_mu();
        assert!(phasing_orbit(mu, 10_000.0, 0.9, 1, PhasingSide::Inner).is_none());
        assert!(phasing_orbit(mu, 10_000.0, 0.5, 0, PhasingSide::Inner).is_none());
    }
}
