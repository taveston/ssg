//! Headless tests of a generated system (SYSG-05, SYSG-22, SYSG-35,
//! SYSG-55, INS-08): it spawns through the same simulation as Corvane and
//! Sol, with the counts its records give, every named orbit resolved, and
//! its planet facts and gate states in the inset.

use bevy::ecs::system::SystemState;

use super::*;
use crate::content::generator::system::generate_system;
use crate::content::generator::testing;
use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::body::{Belt, GateState, Mass};
use crate::sim::named_orbit::DockingOrbitOf;
use crate::sim::orbit::kepler_period_days;
use crate::sim::planet_facts::PlanetFacts;
use crate::sim::system::SystemData;
use crate::view::inset::content::{InsetBodies, gather_content};

/// Every record of a system, depth first: what spawns as a body besides
/// the star and the gates.
fn records(system: &SystemData) -> Vec<&BodyBlueprint> {
    fn walk<'a>(body: &'a BodyBlueprint, out: &mut Vec<&'a BodyBlueprint>) {
        out.push(body);
        body.children.iter().for_each(|child| walk(child, out));
    }
    let mut out = Vec::new();
    system.planets.iter().for_each(|body| walk(body, &mut out));
    out
}

/// A generated system with something of every kind: moons, a belt with
/// a large body, a working gate, a dormant one and a wreck.
fn rich_system() -> SystemData {
    let t = GeneratorTunables::default();
    for seed in testing::check_seeds(20) {
        let sector = testing::sector(80, seed);
        for gated in &sector.gated {
            let system = generate_system(&sector, gated.id, &t).unwrap();
            let planets = sector.summary(gated.id).unwrap().planets.len();
            let all = records(&system);
            let has = |class| all.iter().any(|b| b.class == class);
            let belt_body = system.planets[planets..]
                .iter()
                .any(|b| b.class == BodyClass::Planet);
            let gate = |dormant: bool| {
                system
                    .gates
                    .iter()
                    .any(|g| matches!(g.state, GateState::Dormant { .. }) == dormant)
            };
            if has(BodyClass::Moon)
                && has(BodyClass::SalvageSite)
                && belt_body
                && gate(true)
                && gate(false)
            {
                return system;
            }
        }
    }
    panic!("no generated system has every kind of record");
}

/// A headless app with the simulation alone and `system` waiting to
/// spawn.
fn spawned(system: SystemData) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimPlugin))
        .insert_resource(SystemDataResource(Some(system)));
    run(&mut app, 2);
    app
}

/// SYSG-05 / SYSG-22: a generated system spawns with one body per record,
/// its star and its gates, one belt per belt record, planet facts on
/// every planet, moon and belt body, and gate states on its gates.
#[test]
fn a_generated_system_spawns_with_its_counts() {
    let system = rich_system();
    let records = records(&system);
    let worlds = records.iter().filter(|b| b.planet_facts.is_some()).count();
    let (belts, gates) = (system.belts.len(), system.gates.len());
    let mut app = spawned(system.clone());
    let world = app.world_mut();
    let bodies = world.query::<&BodyClass>().iter(world).count();
    assert_eq!(bodies, 1 + records.len() + gates);
    assert_eq!(world.query::<&Belt>().iter(world).count(), belts);
    let mut classes = world.query::<(&BodyClass, Has<PlanetFacts>, Has<GateState>)>();
    let mut with_facts = 0;
    for (class, facts, state) in classes.iter(world) {
        with_facts += usize::from(facts);
        assert_eq!(state, *class == BodyClass::Gate, "{class:?}");
        if facts {
            assert!(matches!(class, BodyClass::Planet | BodyClass::Moon));
        }
    }
    assert_eq!(with_facts, worlds);
}

/// SYSG-05: every named orbit of a generated system resolves: each names
/// a spawned body, every star, planet and moon has its Low band, and
/// every gate and wreck its docking orbit.
#[test]
fn a_generated_systems_named_orbits_resolve() {
    let mut app = spawned(rich_system());
    let world = app.world_mut();
    let bodies: Vec<(Entity, BodyClass)> = world
        .query::<(Entity, &BodyClass)>()
        .iter(world)
        .map(|(entity, class)| (entity, *class))
        .collect();
    let orbits: Vec<(NamedOrbit, Option<Band>, Option<DockingOrbitOf>)> = world
        .query::<(&NamedOrbit, Option<&Band>, Option<&DockingOrbitOf>)>()
        .iter(world)
        .map(|(orbit, band, docking)| (orbit.clone(), band.copied(), docking.copied()))
        .collect();
    for (orbit, ..) in &orbits {
        assert!(bodies.iter().any(|(entity, _)| *entity == orbit.parent));
        assert!(
            orbit.radius_km > 0.0 && orbit.period_days > 0.0,
            "{orbit:?}"
        );
    }
    for (entity, class) in &bodies {
        let has_low = orbits
            .iter()
            .any(|(o, band, _)| o.parent == *entity && *band == Some(Band::Low));
        let docks = orbits
            .iter()
            .any(|(.., docking)| *docking == Some(DockingOrbitOf(*entity)));
        match class {
            BodyClass::Star | BodyClass::Planet | BodyClass::Moon => assert!(has_low, "{class:?}"),
            BodyClass::Gate | BodyClass::SalvageSite => assert!(docks, "{class:?}"),
            other => panic!("a generated system has no {other:?}"),
        }
    }
}

/// SYSG-35: a moon's period follows from Kepler's third law with its
/// planet's mass.
#[test]
fn a_generated_moons_period_follows_its_planets_mass() {
    let mut app = spawned(rich_system());
    let world = app.world_mut();
    let moons: Vec<(Orbit, Entity)> = world
        .query::<(&BodyClass, &Orbit, &ChildOf)>()
        .iter(world)
        .filter(|(class, ..)| **class == BodyClass::Moon)
        .map(|(_, orbit, child_of)| (*orbit, child_of.parent()))
        .collect();
    assert!(!moons.is_empty());
    for (orbit, planet) in moons {
        let mass = world.get::<Mass>(planet).unwrap().0;
        let period = kepler_period_days(orbit.radius_km, mass);
        assert!((orbit.period_days - period).abs() <= 1e-12 * period);
    }
}

/// INS-08 / SYSG-55: a generated planet's inset lists its Type,
/// Gravity, Equilibrium temperature, Atmosphere and Water after its
/// radius; a gate's lists its State.
#[test]
fn a_generated_systems_inset_lists_planet_facts_and_gate_state() {
    let system = rich_system();
    let planet = system.planets[0].name.clone();
    let gates: Vec<(String, GateState)> = system
        .gates
        .iter()
        .map(|g| (g.name.clone(), g.state))
        .collect();
    let mut app = spawned(system);
    let planet = body(&mut app, &planet);
    let gates: Vec<(Entity, GateState)> = gates
        .iter()
        .map(|(name, state)| (body(&mut app, name), *state))
        .collect();
    let tunables = Tunables::default();
    let mut state = SystemState::<InsetBodies>::new(app.world_mut());
    let bodies = state.get(app.world()).unwrap();
    let labels = |entity| -> Vec<String> {
        let content = gather_content(entity, &bodies, &tunables).unwrap();
        content.facts.into_iter().map(|(label, _)| label).collect()
    };
    assert_eq!(
        labels(planet),
        [
            "Orbit",
            "Period",
            "Mean radius",
            "Type",
            "Gravity",
            "Equilibrium temperature",
            "Atmosphere",
            "Water"
        ]
    );
    for (gate, gate_state) in gates {
        let content = gather_content(gate, &bodies, &tunables).unwrap();
        let state_row = content.facts.last().unwrap();
        assert_eq!(state_row.0, "State");
        let dormant = matches!(gate_state, GateState::Dormant { .. });
        assert_eq!(state_row.1.starts_with("Dormant"), dormant);
    }
}
