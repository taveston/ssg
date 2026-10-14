//! The system as the transfer rules see it: a snapshot of every body, its
//! gravity, its orbit round its parent and its named orbits, and where
//! each ship is (XFER-01, XFER-11). The rules in `places.rs`, `legs.rs`
//! and `plan.rs` are pure functions over this snapshot (ARCH-20);
//! `TreeQuery` gathers it from the entities.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use orbits::{CircularOrbit, Conic};

use super::places::{Band, hill_radius_km};
use crate::sim::body::{BodyClass, Mass, PhysicalRadius};
use crate::sim::named_orbit::{DockingOrbitOf, InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::rail::SegmentKind;
use crate::sim::ship::ShipRail;
use crate::sim::units::gravitational_parameter_km3_per_day2;

/// A star, planet, moon, station, gate or site: what it orbits, its
/// gravity and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyNode {
    /// What it orbits; `None` for the star.
    pub parent: Option<Entity>,
    pub class: BodyClass,
    pub mass_kg: f64,
    /// Gravitational parameter in km³/day²; zero for a massless body.
    pub mu: f64,
    pub radius_km: f64,
    /// Its circular orbit round its parent; `None` for the star.
    pub orbit: Option<CircularOrbit>,
    /// Its Hill radius; `None` for the star or a massless body.
    pub hill_km: Option<f64>,
}

/// A named orbit: the body it circles, its radius and period, and what
/// it stands for (a band, or a place's docking orbit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitNode {
    pub body: Entity,
    pub radius_km: f64,
    pub period_days: f64,
    pub band: Option<Band>,
    pub docking_for: Option<Entity>,
}

impl OrbitNode {
    /// The orbit's circular path at a given phase, round its body.
    pub fn path(&self, phase0_rad: f64) -> CircularOrbit {
        CircularOrbit {
            radius_km: self.radius_km,
            period_days: self.period_days,
            phase0_rad,
        }
    }
}

/// Where a ship is, as far as orders care (RAIL-09, BAND-07).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShipState {
    /// In a named orbit, at its own phase.
    Parked { orbit: Entity, phase0_rad: f64 },
    /// On a free conic round `primary` with no burns to come.
    Coasting { primary: Entity, conic: Conic },
    /// Mid-crossing: on a segment with burns still to come.
    InFlight,
}

/// The snapshot the transfer rules work on.
#[derive(Debug, Clone, Default)]
pub struct SystemTree {
    pub bodies: HashMap<Entity, BodyNode>,
    pub orbits: HashMap<Entity, OrbitNode>,
    pub ships: HashMap<Entity, ShipState>,
}

impl SystemTree {
    /// The body and its ancestors, nearest first, ending at the star.
    pub fn ancestors(&self, body: Entity) -> Vec<Entity> {
        let mut chain = vec![body];
        let mut current = body;
        while let Some(parent) = self.bodies.get(&current).and_then(|node| node.parent) {
            if chain.contains(&parent) {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        chain
    }

    /// XFER-01: the lowest common ancestor of two bodies in the system
    /// tree: the body a crossing between them goes round.
    pub fn lowest_common_ancestor(&self, a: Entity, b: Entity) -> Option<Entity> {
        let b_chain = self.ancestors(b);
        self.ancestors(a)
            .into_iter()
            .find(|body| b_chain.contains(body))
    }

    /// XFER-01: the ancestor of `body` directly below `primary`: the thing
    /// whose orbit round the primary a crossing starts or ends at. `None`
    /// if `body` is the primary itself, or not below it.
    pub fn child_below(&self, primary: Entity, body: Entity) -> Option<Entity> {
        let chain = self.ancestors(body);
        let at = chain.iter().position(|&b| b == primary)?;
        at.checked_sub(1).map(|i| chain[i])
    }

    /// The named orbit standing for a body's band.
    pub fn band_orbit(&self, body: Entity, band: Band) -> Option<Entity> {
        self.orbits
            .iter()
            .find(|(_, node)| node.body == body && node.band == Some(band))
            .map(|(&orbit, _)| orbit)
    }

    /// The docking orbit a station, gate or site offers.
    pub fn docking_orbit(&self, place: Entity) -> Option<Entity> {
        self.orbits
            .iter()
            .find(|(_, node)| node.docking_for == Some(place))
            .map(|(&orbit, _)| orbit)
    }

    /// The gravitational parameter of a body, zero if unknown.
    pub fn mu(&self, body: Entity) -> f64 {
        self.bodies.get(&body).map_or(0.0, |node| node.mu)
    }
}

/// The components a body node is read from.
type BodyRecords<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static BodyClass,
        Option<&'static ChildOf>,
        Option<&'static PhysicalRadius>,
        Option<&'static Mass>,
        Option<&'static Orbit>,
    ),
>;

/// The components a ship's state is read from.
type ShipRecords<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static InNamedOrbit>,
        Option<&'static Orbit>,
        Option<&'static ShipRail>,
    ),
    With<BodyClass>,
>;

/// Gathers the snapshot from the entities.
#[derive(SystemParam)]
pub struct TreeQuery<'w, 's> {
    bodies: BodyRecords<'w, 's>,
    orbits: Query<
        'w,
        's,
        (
            Entity,
            &'static NamedOrbit,
            Option<&'static Band>,
            Option<&'static DockingOrbitOf>,
        ),
    >,
    ships: ShipRecords<'w, 's>,
}

impl TreeQuery<'_, '_> {
    /// The system as it stands: bodies with their gravity and orbits,
    /// named orbits, and each ship's state on `day`.
    pub fn snapshot(&self, day: f64) -> SystemTree {
        let star = self
            .bodies
            .iter()
            .find(|(_, class, ..)| **class == BodyClass::Star)
            .map(|(entity, ..)| entity);
        let mut tree = SystemTree::default();
        for (entity, class, child_of, radius, mass, orbit) in &self.bodies {
            if *class == BodyClass::Ship {
                continue;
            }
            let parent = match child_of {
                Some(child_of) => Some(child_of.parent()),
                None if Some(entity) == star => None,
                None => star,
            };
            let mass_kg = mass.map_or(0.0, |m| m.0);
            tree.bodies.insert(
                entity,
                BodyNode {
                    parent,
                    class: *class,
                    mass_kg,
                    mu: gravitational_parameter_km3_per_day2(mass_kg),
                    radius_km: radius.map_or(0.0, |r| r.0),
                    orbit: orbit.map(CircularOrbit::from),
                    hill_km: None,
                },
            );
        }
        let parent_masses: HashMap<Entity, f64> = tree
            .bodies
            .iter()
            .filter_map(|(&body, node)| {
                let parent = node.parent?;
                Some((body, tree.bodies.get(&parent)?.mass_kg))
            })
            .collect();
        for (body, node) in &mut tree.bodies {
            if let (Some(orbit), Some(&parent_mass)) = (node.orbit, parent_masses.get(body)) {
                node.hill_km = hill_radius_km(orbit.radius_km, node.mass_kg, parent_mass);
            }
        }
        for (entity, named, band, docking) in &self.orbits {
            tree.orbits.insert(
                entity,
                OrbitNode {
                    body: named.parent,
                    radius_km: named.radius_km,
                    period_days: named.period_days,
                    band: band.copied(),
                    docking_for: docking.map(|d| d.0),
                },
            );
        }
        for (entity, in_orbit, orbit, rail) in &self.ships {
            if !self.is_ship(entity) {
                continue;
            }
            tree.ships
                .insert(entity, ship_state(in_orbit, orbit, rail, day));
        }
        tree
    }

    fn is_ship(&self, entity: Entity) -> bool {
        self.bodies
            .get(entity)
            .is_ok_and(|(_, class, ..)| *class == BodyClass::Ship)
    }
}

/// RAIL-09 / BAND-07: a ship is parked if it sits in a named orbit, and
/// otherwise coasting if its rail's current segment is an unbounded conic;
/// anything else is mid-crossing.
pub fn ship_state(
    in_orbit: Option<&InNamedOrbit>,
    orbit: Option<&Orbit>,
    rail: Option<&ShipRail>,
    day: f64,
) -> ShipState {
    if let Some(in_orbit) = in_orbit {
        return ShipState::Parked {
            orbit: in_orbit.0,
            phase0_rad: orbit.map_or(0.0, |o| o.phase0_rad),
        };
    }
    match rail.and_then(|rail| rail.0.segment_at(day)) {
        Some(segment) if segment.end_day.is_none() => match segment.kind {
            SegmentKind::Conic { primary, conic } => ShipState::Coasting { primary, conic },
            _ => ShipState::InFlight,
        },
        _ => ShipState::InFlight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(parent: Option<Entity>) -> BodyNode {
        BodyNode {
            parent,
            class: BodyClass::Planet,
            mass_kg: 0.0,
            mu: 0.0,
            radius_km: 1.0,
            orbit: None,
            hill_km: None,
        }
    }

    /// XFER-01: the primary of a crossing is the lowest common ancestor:
    /// the star for planet to planet, the planet for moon to moon and
    /// band to band; and the start or end of a crossing is the ancestor
    /// directly below the primary.
    #[test]
    fn lowest_common_ancestor_and_the_child_below_it() {
        let mut world = World::new();
        let [star, planet_a, planet_b, moon_a1, moon_a2, submoon] =
            [(); 6].map(|_| world.spawn_empty().id());
        let mut tree = SystemTree::default();
        tree.bodies.insert(star, node(None));
        tree.bodies.insert(planet_a, node(Some(star)));
        tree.bodies.insert(planet_b, node(Some(star)));
        tree.bodies.insert(moon_a1, node(Some(planet_a)));
        tree.bodies.insert(moon_a2, node(Some(planet_a)));
        tree.bodies.insert(submoon, node(Some(moon_a1)));

        assert_eq!(tree.lowest_common_ancestor(planet_a, planet_b), Some(star));
        assert_eq!(
            tree.lowest_common_ancestor(moon_a1, moon_a2),
            Some(planet_a)
        );
        assert_eq!(tree.lowest_common_ancestor(moon_a1, moon_a1), Some(moon_a1));
        assert_eq!(tree.lowest_common_ancestor(submoon, planet_b), Some(star));
        assert_eq!(
            tree.lowest_common_ancestor(submoon, planet_a),
            Some(planet_a)
        );

        assert_eq!(tree.child_below(star, submoon), Some(planet_a));
        assert_eq!(tree.child_below(planet_a, submoon), Some(moon_a1));
        assert_eq!(tree.child_below(planet_a, planet_a), None);
        assert_eq!(tree.child_below(planet_b, moon_a1), None);
    }

    /// RAIL-09 / BAND-07: a ship in a named orbit is parked; one on an
    /// open-ended conic is coasting; one mid-spiral is in flight.
    #[test]
    fn ship_state_follows_its_orbit_or_rail() {
        let mut world = World::new();
        let orbit = world.spawn_empty().id();
        let star = world.spawn_empty().id();
        let in_orbit = InNamedOrbit(orbit);
        let parked = Orbit {
            radius_km: 1.0,
            period_days: 1.0,
            phase0_rad: 0.7,
        };
        assert_eq!(
            ship_state(Some(&in_orbit), Some(&parked), None, 0.0),
            ShipState::Parked {
                orbit,
                phase0_rad: 0.7
            }
        );
        let mu = 1e21;
        let path = CircularOrbit::new(mu, 1e8, 0.0);
        let conic = Conic::from_state(mu, orbits::Rail::state_at(&path, 0.0), 0.0, 1e-6).unwrap();
        let spiral = orbits::Spiral::new(mu, 0.0, 1e8, 0.0, 2e8, 1.0, 300.0).unwrap();
        let rail = ShipRail(
            orbits::Trajectory::new(vec![
                orbits::Segment {
                    start_day: 0.0,
                    end_day: Some(10.0),
                    kind: SegmentKind::Spiral {
                        primary: star,
                        spiral,
                    },
                },
                orbits::Segment {
                    start_day: 10.0,
                    end_day: None,
                    kind: SegmentKind::Conic {
                        primary: star,
                        conic,
                    },
                },
            ])
            .unwrap(),
        );
        assert_eq!(
            ship_state(None, None, Some(&rail), 5.0),
            ShipState::InFlight
        );
        assert_eq!(
            ship_state(None, None, Some(&rail), 15.0),
            ShipState::Coasting {
                primary: star,
                conic
            }
        );
        assert_eq!(ship_state(None, None, None, 0.0), ShipState::InFlight);
    }
}
