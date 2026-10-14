//! The game's segment kinds (RAIL-02, RAIL-03, RAIL-04): what a ship's
//! trajectory is made of. The maths is the `orbits` crate's; this module
//! adds the game's identities to it: which named orbit a parked ship
//! sits in, and which body a conic or spiral goes round.
//!
//! A segment's state is relative to its primary, in km and km/day
//! (RAIL-03). Reading a trajectory on a day is `Trajectory::state_at`, a
//! pure function of the trajectory and the day (RAIL-04).

use bevy::prelude::*;
use orbits::{CircularOrbit, Conic, Rail, Spiral, State, Trajectory};

/// RAIL-02: one kind of stretch of a ship's path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentKind {
    /// At rest in a named orbit (SCL-07): the orbit's circular path at the
    /// ship's own phase, round the orbit's body.
    Parked {
        orbit: Entity,
        primary: Entity,
        path: CircularOrbit,
    },
    /// Coasting on an ellipse or hyperbola round `primary`.
    Conic { primary: Entity, conic: Conic },
    /// A low-thrust crossing round `primary` (RAIL-07).
    Spiral { primary: Entity, spiral: Spiral },
}

impl SegmentKind {
    /// RAIL-09: the named orbit a parked segment sits in; `None` for a
    /// segment in flight.
    pub fn parked_in(&self) -> Option<Entity> {
        match self {
            SegmentKind::Parked { orbit, .. } => Some(*orbit),
            _ => None,
        }
    }
}

impl Rail for SegmentKind {
    fn state_at(&self, day: f64) -> State {
        match self {
            SegmentKind::Parked { path, .. } => path.state_at(day),
            SegmentKind::Conic { conic, .. } => conic.state_at(day),
            SegmentKind::Spiral { spiral, .. } => spiral.state_at(day),
        }
    }
}

/// A ship's trajectory: the crate's trajectory over the game's segment
/// kinds (RAIL-01).
pub type ShipTrajectory = Trajectory<SegmentKind>;

/// RAIL-03: delta-v figures are kept in km/s while the rails work in
/// km/day; this is the one conversion between them, for a figure leaving
/// the rails.
pub fn km_per_day_to_km_per_s(km_per_day: f64) -> f64 {
    orbits::km_per_day_to_km_per_s(km_per_day)
}

/// RAIL-03: and the same conversion the other way, for a figure entering
/// them.
pub fn km_per_s_to_km_per_day(km_per_s: f64) -> f64 {
    orbits::km_per_s_to_km_per_day(km_per_s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbits::Segment;

    const MU: f64 = 1e21;

    /// RAIL-02 / RAIL-04: a trajectory of the game's kinds reads each
    /// segment's state on its own days, relative to that segment's primary.
    #[test]
    fn each_kind_is_read_on_its_own_days() {
        let mut world = World::new();
        let planet = world.spawn_empty().id();
        let star = world.spawn_empty().id();
        let orbit = world.spawn_empty().id();
        let parked = CircularOrbit::new(MU, 1e5, 0.3);
        let coast = Conic::from_state(MU, parked.state_at(10.0), 10.0, 1e-6).unwrap();
        let spiral = Spiral::new(MU, 20.0, 1e8, 0.0, 2e8, 1.0, 300.0).unwrap();
        let trip = ShipTrajectory::new(vec![
            Segment {
                start_day: 0.0,
                end_day: Some(10.0),
                kind: SegmentKind::Parked {
                    orbit,
                    primary: planet,
                    path: parked,
                },
            },
            Segment {
                start_day: 10.0,
                end_day: Some(20.0),
                kind: SegmentKind::Conic {
                    primary: star,
                    conic: coast,
                },
            },
            Segment {
                start_day: 20.0,
                end_day: None,
                kind: SegmentKind::Spiral {
                    primary: star,
                    spiral,
                },
            },
        ])
        .unwrap();
        assert_eq!(trip.state_at(5.0).unwrap(), parked.state_at(5.0));
        assert_eq!(trip.segment_at(5.0).unwrap().kind.parked_in(), Some(orbit));
        assert_eq!(trip.state_at(15.0).unwrap(), coast.state_at(15.0));
        assert_eq!(trip.segment_at(15.0).unwrap().kind.parked_in(), None);
        assert_eq!(trip.state_at(25.0).unwrap(), spiral.state_at(25.0));
        assert!(
            matches!(trip.segment_at(25.0).unwrap().kind, SegmentKind::Spiral { primary, .. } if primary == star)
        );
        let _ = planet;
    }

    /// RAIL-03: the two speed conversions undo each other.
    #[test]
    fn speed_conversions_round_trip() {
        let v = 3.3;
        assert!((km_per_day_to_km_per_s(km_per_s_to_km_per_day(v)) - v).abs() < 1e-12);
    }
}
