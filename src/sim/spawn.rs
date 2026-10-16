//! Spawns a `SystemData` as simulation entities (SCL-03): the star, a tree
//! of planets, moons, stations and parked ships, each body's named orbits
//! (SCL-07), the belt and its salvage sites, and the gates.
//!
//! Every star, planet and moon gets its two bands as named orbits
//! (BAND-01, PLAN-30), and every gate and salvage site a docking orbit of
//! its own (BAND-02), so content never spells those out. A station sits in
//! one of its parent's bands, as a parked ship does, and offers no orbit
//! of its own. A ship with propulsion gets its drive and delta-v account
//! (PLAN-20).
//!
//! Only simulation components go on here. A satellite is an `Orbit` with a
//! `ChildOf` naming what it orbits; nothing else marks it out, so the view
//! can find satellites structurally. A ship or station in a named orbit
//! also carries `InNamedOrbit`, and its `Orbit` radius and period are the
//! named orbit's. Everything visible - transforms, markers, the belt's
//! mesh, fold state - is added by the view.

use bevy::prelude::*;

use super::body::{
    Affiliation, Belt, BodyClass, Description, DisplayName, GateDestination, GateEntryTolerance,
    KeyFacts, Knowledge, Mass, PhysicalRadius, StellarMass,
};
use super::named_orbit::{DockingOrbitOf, InNamedOrbit, NamedOrbit};
use super::orbit::{Orbit, OrbitPositionKm, kepler_period_days, orbit_position_km};
use super::ship::{DeltaV, Propulsion};
use super::system::{
    BodyBlueprint, GateBlueprint, NamedOrbitBlueprint, OrbitPlacement, SystemData,
    SystemDataResource,
};
use super::transfer::places::{Band, band_radii, hill_radius_km};
use super::tunables::SimTunables;

/// What every body's spawn needs besides its own record.
struct Context<'a> {
    star: Entity,
    tunables: &'a SimTunables,
}

/// Spawns the waiting system, once. With no content plugin there is
/// nothing waiting, and the simulation simply starts empty.
pub fn spawn_system(
    mut commands: Commands,
    tunables: Res<SimTunables>,
    system_data: Option<ResMut<SystemDataResource>>,
) {
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

    let star = commands
        .spawn((
            Name::new(star_name),
            DisplayName(star_name.to_string()),
            Description("The system's star.".to_string()),
            KeyFacts(vec![("Class".to_string(), star_class.to_string())]),
            BodyClass::Star,
            PhysicalRadius(star_radius_km),
            StellarMass(star_mass_kg),
            Mass(star_mass_kg),
            Affiliation::Neutral,
            star_knowledge,
        ))
        .id();
    let context = Context {
        star,
        tunables: &tunables,
    };

    // The star offers only its bands: everything natural round it has an
    // orbit of its own.
    let star_bands = spawn_bands(
        &mut commands,
        star,
        star_radius_km,
        None,
        star_mass_kg,
        &context,
    );
    let round_the_star = Placements::round(&planets, &star_bands);
    for (planet, placed) in planets.into_iter().zip(round_the_star) {
        spawn_body(&mut commands, planet, placed, None, star_mass_kg, &context);
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
        spawn_body(&mut commands, site, placed, None, star_mass_kg, &context);
    }

    for gate in gates {
        spawn_gate(&mut commands, gate, star_mass_kg, &context);
    }
}

fn spawn_gate(commands: &mut Commands, gate: GateBlueprint, star_mass_kg: f64, context: &Context) {
    let orbit = Orbit {
        radius_km: gate.distance_km,
        period_days: kepler_period_days(gate.distance_km, star_mass_kg),
        phase0_rad: gate.angle_deg.to_radians(),
    };
    let entity = commands
        .spawn((
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
            GateEntryTolerance(gate.entry_tolerance_kms),
            PhysicalRadius(gate.radius_km),
            Mass(0.0),
            Affiliation::Neutral,
            Knowledge::Surveyed,
            orbit_components(orbit),
        ))
        .id();
    spawn_docking_orbit(commands, entity, gate.name, context.star, &orbit);
}

/// BAND-02: the orbit a gate or salvage site offers a ship sent to it:
/// its own, as a named orbit of what it orbits, named after it.
fn spawn_docking_orbit(
    commands: &mut Commands,
    place: Entity,
    name: &'static str,
    parent: Entity,
    orbit: &Orbit,
) {
    commands.spawn((
        Name::new(name),
        NamedOrbit {
            parent,
            name: name.to_string(),
            radius_km: orbit.radius_km,
            period_days: orbit.period_days,
        },
        DockingOrbitOf(place),
    ));
}

/// BAND-01 / PLAN-30: a star, planet or moon's two bands as named orbits,
/// and its derived radii on the body. Returns the bands as spawned, for
/// the body's children to sit in.
fn spawn_bands(
    commands: &mut Commands,
    body: Entity,
    radius_km: f64,
    hill_km: Option<f64>,
    mass_kg: f64,
    context: &Context,
) -> Vec<SpawnedOrbit<'static>> {
    let radii = band_radii(radius_km, hill_km, context.tunables);
    commands.entity(body).insert(radii);
    [Band::Low, Band::High]
        .into_iter()
        .filter_map(|band| Some((band, radii.radius_km(band)?)))
        .map(|(band, radius_km)| {
            let entity = commands
                .spawn((
                    Name::new(band.name()),
                    NamedOrbit {
                        parent: body,
                        name: band.name().to_string(),
                        radius_km,
                        period_days: kepler_period_days(radius_km, mass_kg),
                    },
                    band,
                ))
                .id();
            (band.name(), entity, radius_km)
        })
        .collect()
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
    /// radius. Content that names an orbit that isn't there, or gives a
    /// station an orbit of its own, is a content error, reported where it
    /// is first used.
    fn round(children: &[BodyBlueprint], orbits: &[SpawnedOrbit]) -> Self {
        Self(
            children
                .iter()
                .map(|child| {
                    resolve_placement(child.class, child.orbit, orbits)
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

/// SCL-07: what a placement comes to among the parent's `orbits`. A
/// station is placed in a named orbit exactly as a ship is, never in an
/// orbit of its own.
pub fn resolve_placement(
    class: BodyClass,
    placement: OrbitPlacement,
    orbits: &[SpawnedOrbit],
) -> Result<Placed, String> {
    match placement {
        OrbitPlacement::Own { .. } if class == BodyClass::Station => {
            Err("a station sits in a named orbit, not one of its own".to_string())
        }
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
    context: &Context,
) -> Entity {
    let BodyBlueprint {
        name,
        class,
        affiliation,
        record,
        propulsion,
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
        Mass(mass_kg),
        affiliation,
        knowledge,
        orbit_components(orbit),
    ));
    if let Some(record) = record {
        entity_commands.insert(record);
    }
    if let Some(Propulsion { drive, delta_v_kms }) = propulsion {
        // PLAN-20: a ship that can be ordered to move.
        entity_commands.insert((drive, DeltaV::full(delta_v_kms)));
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

    let mut spawned = spawn_named_orbits(commands, entity, &orbits, mass_kg);
    match class {
        BodyClass::Star | BodyClass::Planet | BodyClass::Moon => {
            let hill_km = hill_radius_km(placed.radius_km, mass_kg, central_mass_kg);
            spawned.extend(spawn_bands(
                commands, entity, radius_km, hill_km, mass_kg, context,
            ));
        }
        BodyClass::Gate | BodyClass::SalvageSite => {
            let round = parent.unwrap_or(context.star);
            spawn_docking_orbit(commands, entity, name, round, &orbit);
        }
        // A station is a place in its parent's band, as a parked ship is
        // (SCL-07, BAND-02): it offers no orbit of its own.
        BodyClass::Station | BodyClass::Ship => {}
    }
    let placements = Placements::round(&children, &spawned);
    for (child, placed) in children.into_iter().zip(placements) {
        spawn_body(commands, child, placed, Some(entity), mass_kg, context);
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
        let ship = BodyClass::Ship;
        assert_eq!(
            resolve_placement(ship, OrbitPlacement::Named("High"), &orbits),
            Ok(Placed {
                radius_km: 20_000.0,
                named: Some(high)
            })
        );
        assert_eq!(
            resolve_placement(ship, OrbitPlacement::Own { radius_km: 500.0 }, &orbits),
            Ok(Placed {
                radius_km: 500.0,
                named: None
            })
        );
        assert!(resolve_placement(ship, OrbitPlacement::Named("Low"), &orbits).is_err());
    }

    /// SCL-07: a station is placed in a band exactly as a ship is, and can
    /// be placed nowhere else: an orbit of its own is a content error.
    #[test]
    fn a_station_is_placed_only_in_a_named_orbit() {
        let mut world = World::new();
        let low = world.spawn_empty().id();
        let orbits = [("Low", low, 7_200.0)];
        let station = BodyClass::Station;
        assert_eq!(
            resolve_placement(station, OrbitPlacement::Named("Low"), &orbits),
            Ok(Placed {
                radius_km: 7_200.0,
                named: Some(low)
            })
        );
        let own = OrbitPlacement::Own {
            radius_km: 45_000.0,
        };
        assert!(resolve_placement(station, own, &orbits).is_err());
    }
}
