//! How fast a ship in flight is moving relative to the star (SYM-11), in
//! km/day like the rails. A rail gives a ship's velocity relative to its
//! segment's primary; each body up the chain adds its own circular
//! orbit's velocity round its parent, until the star, which is at rest.

use bevy::ecs::system::SystemParam;
use bevy::math::DVec3;
use bevy::prelude::*;
use orbits::{CircularOrbit, Rail};

use super::clock::GameClock;
use super::orbit::Orbit;
use super::rail::{SegmentKind, ShipTrajectory};
use super::ship::{InFlight, ShipRail};

/// A ship's velocity relative to the star on `day`, in km/day, from its
/// rail: its velocity round the segment's primary plus the primary's
/// velocity relative to the star. `orbit_of` gives a body's orbit round
/// its parent and that parent, or `None` for the star. `None` if the rail
/// has no segment on `day`.
pub fn star_velocity(
    rail: &ShipTrajectory,
    day: f64,
    orbit_of: impl Fn(Entity) -> Option<(CircularOrbit, Option<Entity>)>,
) -> Option<DVec3> {
    let segment = rail.segment_at(day)?;
    let mut velocity = segment.kind.state_at(day).velocity;
    let mut body = match segment.kind {
        SegmentKind::Parked { primary, .. }
        | SegmentKind::Conic { primary, .. }
        | SegmentKind::Spiral { primary, .. } => Some(primary),
    };
    // A chain deeper than this is a cycle in the data; stop rather than
    // loop forever.
    for _ in 0..64 {
        let Some((orbit, parent)) = body.and_then(&orbit_of) else {
            break;
        };
        velocity += orbit.state_at(day).velocity;
        body = parent;
    }
    Some(velocity)
}

/// Reads the velocity of ships in flight relative to the star, today.
#[derive(SystemParam)]
pub struct FlightVelocities<'w, 's> {
    clock: Res<'w, GameClock>,
    ships: Query<'w, 's, &'static ShipRail, With<InFlight>>,
    bodies: Query<'w, 's, (&'static Orbit, Option<&'static ChildOf>)>,
}

impl FlightVelocities<'_, '_> {
    /// A ship's velocity relative to the star today, in km/day; `None` for
    /// a ship that isn't in flight (parked or orbiting), or anything else.
    pub fn of(&self, ship: Entity) -> Option<DVec3> {
        let rail = self.ships.get(ship).ok()?;
        star_velocity(&rail.0, self.clock.elapsed_days(), |body| {
            let (orbit, child_of) = self.bodies.get(body).ok()?;
            Some((CircularOrbit::from(orbit), child_of.map(ChildOf::parent)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbits::{Conic, Segment};

    const MU: f64 = 1.0e12;

    /// SYM-11: a ship coasting round a moon moves relative to the star at
    /// its own velocity round the moon, plus the moon's round its planet,
    /// plus the planet's round the star.
    #[test]
    fn velocity_adds_up_the_primary_chain() {
        let mut world = World::new();
        let [star, planet, moon] = [(); 3].map(|_| world.spawn_empty().id());
        let planet_orbit = CircularOrbit {
            radius_km: 1.0e8,
            period_days: 300.0,
            phase0_rad: 0.3,
        };
        let moon_orbit = CircularOrbit {
            radius_km: 4.0e5,
            period_days: 20.0,
            phase0_rad: 1.1,
        };
        let start = CircularOrbit::new(MU, 2.0e4, 0.0);
        let conic = Conic::from_state(MU, start.state_at(5.0), 5.0, 1e-6).unwrap();
        let rail = ShipTrajectory::new(vec![Segment {
            start_day: 5.0,
            end_day: None,
            kind: SegmentKind::Conic {
                primary: moon,
                conic,
            },
        }])
        .unwrap();
        let orbit_of = |body: Entity| {
            if body == planet {
                Some((planet_orbit, Some(star)))
            } else if body == moon {
                Some((moon_orbit, Some(planet)))
            } else {
                None
            }
        };
        let day = 7.5;
        let expected = conic.state_at(day).velocity
            + moon_orbit.state_at(day).velocity
            + planet_orbit.state_at(day).velocity;
        let got = star_velocity(&rail, day, orbit_of).unwrap();
        assert!((got - expected).length() < 1e-9, "{got} vs {expected}");
        // Before the rail starts there is nothing to read.
        assert_eq!(star_velocity(&rail, 1.0, orbit_of), None);
    }
}
