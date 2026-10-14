//! The orbital plane, a body's state in it, and the sense of motion.
//!
//! Everything in this crate happens in one flat plane: the XZ plane, with
//! y always zero. An angle in the plane is measured from the +x axis
//! towards the +z axis. A body whose angle grows with time goes round in
//! the **prograde** sense; one whose angle shrinks goes round
//! **retrograde**. Positions are in km and velocities in km/day, relative
//! to whatever the body goes round (the "primary"), which sits at the
//! origin.

use glam::DVec3;
use std::f64::consts::TAU;

/// Where something is and how fast it is going at one instant: a
/// position in km and a velocity in km/day, both relative to the primary
/// at the origin and both lying in the orbital plane (y is zero).
///
/// ```
/// use glam::DVec3;
/// use orbits::State;
///
/// // A probe 10,000 km out along +x, moving at 100,000 km/day along +z.
/// let state = State::new(DVec3::new(10_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, 100_000.0));
/// assert_eq!(state.radius(), 10_000.0);
/// assert_eq!(state.speed(), 100_000.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Position in km, relative to the primary.
    pub position: DVec3,
    /// Velocity in km/day, relative to the primary.
    pub velocity: DVec3,
}

impl State {
    /// Bundles a position (km) and a velocity (km/day) into a state.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::State;
    ///
    /// let state = State::new(DVec3::new(5_000.0, 0.0, 0.0), DVec3::ZERO);
    /// assert_eq!(state.position.x, 5_000.0);
    /// ```
    pub fn new(position: DVec3, velocity: DVec3) -> Self {
        Self { position, velocity }
    }

    /// Distance from the primary, in km.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::State;
    ///
    /// let state = State::new(DVec3::new(3_000.0, 0.0, 4_000.0), DVec3::ZERO);
    /// assert_eq!(state.radius(), 5_000.0);
    /// ```
    pub fn radius(&self) -> f64 {
        self.position.length()
    }

    /// Speed relative to the primary, in km/day.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::State;
    ///
    /// let state = State::new(DVec3::X, DVec3::new(0.0, 0.0, 250_000.0));
    /// assert_eq!(state.speed(), 250_000.0);
    /// ```
    pub fn speed(&self) -> f64 {
        self.velocity.length()
    }

    /// The angle of the position in the plane, in radians from +x towards
    /// +z, in the range (-π, π].
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::State;
    ///
    /// let state = State::new(DVec3::new(0.0, 0.0, 7_000.0), DVec3::ZERO);
    /// assert!((state.angle() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    /// ```
    pub fn angle(&self) -> f64 {
        angle_of(self.position)
    }

    /// The signed angular momentum per unit mass, in km²/day: how much the
    /// body is swinging round the primary, as opposed to moving straight
    /// at or away from it. Positive for prograde motion, negative for
    /// retrograde, zero for a body falling straight in or out.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::State;
    ///
    /// let prograde = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, 10.0));
    /// assert_eq!(prograde.angular_momentum(), 10_000.0);
    /// let radial = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 0.0));
    /// assert_eq!(radial.angular_momentum(), 0.0);
    /// ```
    pub fn angular_momentum(&self) -> f64 {
        self.position.x * self.velocity.z - self.position.z * self.velocity.x
    }

    /// Which way the body is going round: prograde if its angle is
    /// growing, retrograde otherwise.
    ///
    /// ```
    /// use glam::DVec3;
    /// use orbits::{Sense, State};
    ///
    /// let state = State::new(DVec3::new(1_000.0, 0.0, 0.0), DVec3::new(0.0, 0.0, -10.0));
    /// assert_eq!(state.sense(), Sense::Retrograde);
    /// ```
    pub fn sense(&self) -> Sense {
        if self.angular_momentum() < 0.0 {
            Sense::Retrograde
        } else {
            Sense::Prograde
        }
    }

    /// The same state seen in a mirror that flips z. Mirroring a
    /// retrograde state gives a prograde one and back, which lets the
    /// maths be written once for the prograde sense.
    pub(crate) fn mirrored(&self) -> Self {
        Self {
            position: mirror(self.position),
            velocity: mirror(self.velocity),
        }
    }
}

/// The direction of travel round the primary. Prograde means the angle
/// (from +x towards +z) grows with time; retrograde means it shrinks. In a
/// system where every planet goes round the same way, that way is
/// prograde.
///
/// ```
/// use orbits::Sense;
///
/// assert_eq!(Sense::Prograde.sign(), 1.0);
/// assert_eq!(Sense::Retrograde.sign(), -1.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sense {
    /// The angle grows with time.
    Prograde,
    /// The angle shrinks with time.
    Retrograde,
}

impl Sense {
    /// +1 for prograde, -1 for retrograde: the sign angles change with.
    ///
    /// ```
    /// use orbits::Sense;
    ///
    /// let rate_rad_per_day = 0.1 * Sense::Retrograde.sign();
    /// assert_eq!(rate_rad_per_day, -0.1);
    /// ```
    pub fn sign(self) -> f64 {
        match self {
            Sense::Prograde => 1.0,
            Sense::Retrograde => -1.0,
        }
    }
}

/// The angle of a point in the plane, in radians from +x towards +z, in
/// the range (-π, π]. The y component is ignored.
///
/// ```
/// use glam::DVec3;
/// use orbits::angle_of;
///
/// assert_eq!(angle_of(DVec3::new(1.0, 0.0, 0.0)), 0.0);
/// assert!((angle_of(DVec3::new(-1.0, 0.0, 0.0)) - std::f64::consts::PI).abs() < 1e-12);
/// ```
pub fn angle_of(position: DVec3) -> f64 {
    position.z.atan2(position.x)
}

/// The point in the plane at a distance `radius_km` from the origin and an
/// angle `angle_rad` from +x towards +z.
///
/// ```
/// use orbits::point_in_plane;
///
/// let p = point_in_plane(2.0, std::f64::consts::FRAC_PI_2);
/// assert!(p.x.abs() < 1e-12);
/// assert_eq!(p.y, 0.0);
/// assert!((p.z - 2.0).abs() < 1e-12);
/// ```
pub fn point_in_plane(radius_km: f64, angle_rad: f64) -> DVec3 {
    let (sin, cos) = angle_rad.sin_cos();
    DVec3::new(radius_km * cos, 0.0, radius_km * sin)
}

/// Brings an angle in radians into the range [0, 2π).
///
/// ```
/// use orbits::wrap_angle;
/// use std::f64::consts::TAU;
///
/// assert!((wrap_angle(-0.5) - (TAU - 0.5)).abs() < 1e-12);
/// assert!((wrap_angle(TAU + 1.0) - 1.0).abs() < 1e-12);
/// ```
pub fn wrap_angle(angle_rad: f64) -> f64 {
    angle_rad.rem_euclid(TAU)
}

/// Brings an angle in radians into the range (-π, π].
pub(crate) fn wrap_to_pi(angle_rad: f64) -> f64 {
    let wrapped = wrap_angle(angle_rad);
    if wrapped > std::f64::consts::PI {
        wrapped - TAU
    } else {
        wrapped
    }
}

/// Flips the z component: the plane seen in a mirror.
pub(crate) fn mirror(v: DVec3) -> DVec3 {
    DVec3::new(v.x, v.y, -v.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mirrored state goes round the other way, and mirroring twice is
    /// the identity.
    #[test]
    fn mirroring_flips_the_sense() {
        let state = State::new(DVec3::new(1.0, 0.0, 2.0), DVec3::new(-3.0, 0.0, 4.0));
        assert_eq!(state.sense(), Sense::Prograde);
        assert_eq!(state.mirrored().sense(), Sense::Retrograde);
        assert_eq!(state.mirrored().mirrored(), state);
    }

    /// wrap_to_pi keeps π itself and moves anything past it negative.
    #[test]
    fn wrap_to_pi_covers_the_half_open_range() {
        assert!((wrap_to_pi(std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-12);
        assert!(wrap_to_pi(4.0) < 0.0);
        assert!((wrap_to_pi(4.0) - (4.0 - TAU)).abs() < 1e-12);
    }

    /// point_in_plane and angle_of undo each other.
    #[test]
    fn point_and_angle_round_trip() {
        for angle in [0.0, 0.7, 2.9, -1.2] {
            let p = point_in_plane(10.0, angle);
            assert!((angle_of(p) - angle).abs() < 1e-12);
            assert!((p.length() - 10.0).abs() < 1e-12);
        }
    }
}
