//! Spawns a `SystemData` as simulation entities (SCL-03): the star, a tree
//! of planets, moons, stations and parked ships, each body's named orbits
//! (SCL-07), the belt and its salvage sites, and the gates.
//!
//! Only simulation components go on here. A satellite is an `Orbit` with a
//! `ChildOf` naming what it orbits; nothing else marks it out, so the view
//! can find satellites structurally. A ship or station in a named orbit
//! also carries `InNamedOrbit`, and its `Orbit` radius and period are the
//! named orbit's. Everything visible - transforms, markers, the belt's
//! mesh, fold state - is added by the view.

use bevy::prelude::*;

use super::body::{
    Affiliation, Belt, BodyClass, Description, DisplayName, GateDestination, KeyFacts, Knowledge,
    PhysicalRadius, StellarMass,
};
use super::named_orbit::{InNamedOrbit, NamedOrbit};
use super::orbit::{Orbit, OrbitPositionKm, kepler_period_days, orbit_position_km};
use super::system::{
    BodyBlueprint, GateBlueprint, NamedOrbitBlueprint, OrbitPlacement, SystemData,
    SystemDataResource,
};

/// Spawns the waiting system, once. With no content plugin there is
/// nothing waiting, and the simulation simply starts empty.
pub fn spawn_system(mut commands: Commands, system_data: Option<ResMut<SystemDataResource>>) {
    let Some(system_data) = system_data.and_then(|mut waiting| waiting.0.take()) else {
        return;
    };
    let SystemData {
        star_name,
        star_class,
        star_mass_kg,
        star_radius_km,
        star_knowledge,
        planets,
        belt,
        gates,
    } = system_data;

    commands.spawn((
        Name::new(star_name),
        DisplayName(star_name.to_string()),
        Description("The system's star.".to_string()),
        KeyFacts(vec![("Class".to_string(), star_class.to_string())]),
        BodyClass::Star,
        PhysicalRadius(star_radius_km),
        StellarMass(star_mass_kg),
        Affiliation::Neutral,
        star_knowledge,
    ));

    // The star offers no named orbits: everything round it has an orbit
    // of its own.
    let round_the_star = Placements::round(&planets, &[]);
    for (planet, placed) in planets.into_iter().zip(round_the_star) {
        spawn_body(&mut commands, planet, placed, None, star_mass_kg);
    }

    commands.spawn((
        Name::new("Belt"),
        Belt {
            inner_radius_km: belt.inner_radius_km,
            outer_radius_km: belt.outer_radius_km,
        },
    ));
    let in_the_belt = Placements::round(&belt.salvage_sites, &[]);
    for (site, placed) in belt.salvage_sites.into_iter().zip(in_the_belt) {
        spawn_body(&mut commands, site, placed, None, star_mass_kg);
    }

    for gate in gates {
        spawn_gate(&mut commands, gate, star_mass_kg);
    }
}

fn spawn_gate(commands: &mut Commands, gate: GateBlueprint, star_mass_kg: f64) {
    let orbit = Orbit {
        radius_km: gate.distance_km,
        period_days: kepler_period_days(gate.distance_km, star_mass_kg),
        phase0_rad: gate.angle_deg.to_radians(),
    };
    commands.spawn((
        Name::new(gate.name),
        DisplayName(gate.name.to_string()),
        Description(format!(
            "A gate linking to the {} system.",
            gate.destination
        )),
        KeyFacts(vec![(
            "Destination".to_string(),
            gate.destination.to_string(),
        )]),
        BodyClass::Gate,
        GateDestination(gate.destination.to_string()),
        PhysicalRadius(gate.radius_km),
        Affiliation::Neutral,
        Knowledge::Surveyed,
        orbit_components(orbit),
    ));
}

/// An orbiting body's simulation state at the epoch.
fn orbit_components(orbit: Orbit) -> (Orbit, OrbitPositionKm) {
    (orbit, OrbitPositionKm(orbit_position_km(&orbit, 0.0)))
}

/// A record's placement resolved to a radius, and the named orbit it
/// references if it does (SCL-07).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub radius_km: f64,
    pub named: Option<Entity>,
}

/// A parent's named orbits as spawned: name, entity and radius.
pub type SpawnedOrbit<'a> = (&'a str, Entity, f64);

/// The resolved placements of one parent's children, in their order.
struct Placements(Vec<Placed>);

impl Placements {
    /// SCL-07: resolve each child's placement against the parent's named
    /// orbits: an own radius stands as it is; a named orbit gives its
    /// radius. Content that names an orbit that isn't there is a content
    /// error, reported where it is first used.
    fn round(children: &[BodyBlueprint], orbits: &[SpawnedOrbit]) -> Self {
        Self(
            children
                .iter()
                .map(|child| {
                    resolve_placement(child.orbit, orbits)
                        .unwrap_or_else(|error| panic!("{}: {error}", child.name))
                })
                .collect(),
        )
    }
}

impl IntoIterator for Placements {
    type Item = Placed;
    type IntoIter = std::vec::IntoIter<Placed>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// SCL-07: what a placement comes to among the parent's `orbits`.
pub fn resolve_placement(
    placement: OrbitPlacement,
    orbits: &[SpawnedOrbit],
) -> Result<Placed, String> {
    match placement {
        OrbitPlacement::Own { radius_km } => Ok(Placed {
            radius_km,
            named: None,
        }),
        OrbitPlacement::Named(name) => orbits
            .iter()
            .find(|(orbit_name, ..)| *orbit_name == name)
            .map(|&(_, entity, radius_km)| Placed {
                radius_km,
                named: Some(entity),
            })
            .ok_or_else(|| format!("no named orbit {name:?} to sit in")),
    }
}

/// SCL-07: a body's named orbits as entities, each with its period from
/// the body's mass (SCL-04).
fn spawn_named_orbits(
    commands: &mut Commands,
    parent: Entity,
    orbits: &[NamedOrbitBlueprint],
    mass_kg: f64,
) -> Vec<(&'static str, Entity, f64)> {
    orbits
        .iter()
        .map(|orbit| {
            let entity = commands
                .spawn((
                    Name::new(orbit.name),
                    NamedOrbit {
                        parent,
                        name: orbit.name.to_string(),
                        radius_km: orbit.radius_km,
                        period_days: kepler_period_days(orbit.radius_km, mass_kg),
                    },
                ))
                .id();
            (orbit.name, entity, orbit.radius_km)
        })
        .collect()
}

fn spawn_body(
    commands: &mut Commands,
    blueprint: BodyBlueprint,
    placed: Placed,
    parent: Option<Entity>,
    central_mass_kg: f64,
) -> Entity {
    let BodyBlueprint {
        name,
        class,
        affiliation,
        record,
        description,
        key_facts,
        knowledge,
        orbit: _,
        radius_km,
        phase0_rad,
        mass_kg,
        orbits,
        children,
    } = blueprint;

    let orbit = Orbit {
        radius_km: placed.radius_km,
        period_days: kepler_period_days(placed.radius_km, central_mass_kg),
        phase0_rad,
    };

    let mut entity_commands = commands.spawn((
        Name::new(name),
        DisplayName(name.to_string()),
        Description(description.to_string()),
        KeyFacts(
            key_facts
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ),
        class,
        PhysicalRadius(radius_km),
        affiliation,
        knowledge,
        orbit_components(orbit),
    ));
    if let Some(record) = record {
        entity_commands.insert(record);
    }
    if let Some(parent) = parent {
        // What it orbits: positions are relative to this body (ARCH-10).
        entity_commands.insert(ChildOf(parent));
    }
    if let Some(named) = placed.named {
        // SCL-07: the orbit it shares with whatever else sits there.
        entity_commands.insert(InNamedOrbit(named));
    }
    let entity = entity_commands.id();

    let spawned = spawn_named_orbits(commands, entity, &orbits, mass_kg);
    let placements = Placements::round(&children, &spawned);
    for (child, placed) in children.into_iter().zip(placements) {
        spawn_body(commands, child, placed, Some(entity), mass_kg);
    }

    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SCL-07: a record in a named orbit takes that orbit's radius and
    /// references it; one with an orbit of its own references nothing; an
    /// orbit the parent doesn't offer is an error.
    #[test]
    fn a_named_orbit_gives_its_radius_and_entity() {
        let mut world = World::new();
        let high = world.spawn_empty().id();
        let orbits = [("High", high, 20_000.0)];
        assert_eq!(
            resolve_placement(OrbitPlacement::Named("High"), &orbits),
            Ok(Placed {
                radius_km: 20_000.0,
                named: Some(high)
            })
        );
        assert_eq!(
            resolve_placement(OrbitPlacement::Own { radius_km: 500.0 }, &orbits),
            Ok(Placed {
                radius_km: 500.0,
                named: None
            })
        );
        assert!(resolve_placement(OrbitPlacement::Named("Low"), &orbits).is_err());
    }
}
