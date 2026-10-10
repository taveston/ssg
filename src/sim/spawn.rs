//! Spawns a `SystemData` as simulation entities (SCL-03): the star, a tree
//! of planets, moons, stations and parked ships, the belt and its salvage
//! sites, and the gates.
//!
//! Only simulation components go on here. A satellite is an `Orbit` with a
//! `ChildOf` naming what it orbits; nothing else marks it out, so the view
//! can find satellites structurally. Everything visible - transforms,
//! markers, the belt's mesh, fold state - is added by the view.

use bevy::prelude::*;

use super::body::{Belt, BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership};
use super::orbit::{Orbit, OrbitPositionKm, kepler_period_days, orbit_position_km};
use super::system::{BodyBlueprint, GateBlueprint, SystemData, SystemDataResource};

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
        Ownership::Neutral,
        star_knowledge,
    ));

    for planet in planets {
        spawn_body(&mut commands, planet, None, star_mass_kg);
    }

    commands.spawn((
        Name::new("Belt"),
        Belt {
            inner_radius_km: belt.inner_radius_km,
            outer_radius_km: belt.outer_radius_km,
        },
    ));
    for site in belt.salvage_sites {
        spawn_body(&mut commands, site, None, star_mass_kg);
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
        Ownership::Neutral,
        Knowledge::Surveyed { imaged_day: 0.0 },
        orbit_components(orbit),
    ));
}

/// An orbiting body's simulation state at the epoch.
fn orbit_components(orbit: Orbit) -> (Orbit, OrbitPositionKm) {
    (orbit, OrbitPositionKm(orbit_position_km(&orbit, 0.0)))
}

fn spawn_body(
    commands: &mut Commands,
    blueprint: BodyBlueprint,
    parent: Option<Entity>,
    central_mass_kg: f64,
) -> Entity {
    let BodyBlueprint {
        name,
        class,
        ownership,
        description,
        key_facts,
        knowledge,
        orbit_radius_km,
        phase0_rad,
        mass_kg,
        children,
    } = blueprint;

    let orbit = Orbit {
        radius_km: orbit_radius_km,
        period_days: kepler_period_days(orbit_radius_km, central_mass_kg),
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
        ownership,
        knowledge,
        orbit_components(orbit),
    ));
    if let Some(parent) = parent {
        // What it orbits: positions are relative to this body (ARCH-10).
        entity_commands.insert(ChildOf(parent));
    }
    let entity = entity_commands.id();

    for child in children {
        spawn_body(commands, child, Some(entity), mass_kg);
    }

    entity
}
