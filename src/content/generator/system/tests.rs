//! Tests of the system generator's contract over a few whole sectors:
//! what each record is and how it is named. The invariants over the full
//! sweep of sectors are in `tests_invariants.rs`.

use super::worlds::{description, planet_facts};
use super::*;
use crate::content::generator::network::LinkState;
use crate::content::generator::testing;
use crate::sim::body::{Affiliation, BodyClass, GateState};
use crate::sim::planet_facts::{Atmosphere, WorldKind};
use crate::sim::system::{BodyBlueprint, OrbitPlacement};
use crate::sim::units::{EARTH_MASS_KG, KM_PER_AU};

/// A medium sector's gated systems, generated.
fn medium(seed: u64) -> (Sector, Vec<SystemData>) {
    let sector = testing::sector(80, seed);
    let t = GeneratorTunables::default();
    let systems = sector
        .gated
        .iter()
        .map(|g| generate_system(&sector, g.id, &t).unwrap())
        .collect();
    (sector, systems)
}

/// Every record round the star and its moons, depth first.
fn every_record(system: &SystemData) -> Vec<&BodyBlueprint> {
    fn walk<'a>(body: &'a BodyBlueprint, out: &mut Vec<&'a BodyBlueprint>) {
        out.push(body);
        for child in &body.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    for body in &system.planets {
        walk(body, &mut out);
    }
    out
}

fn own_radius(body: &BodyBlueprint) -> f64 {
    match body.orbit {
        OrbitPlacement::Own { radius_km } => radius_km,
        OrbitPlacement::Named(name) => panic!("{} sits in {name}", body.name),
    }
}

/// SYSG-01: an id that is not a gated system's is an error naming it.
#[test]
fn an_ungated_id_is_an_error_naming_it() {
    let sector = testing::sector(30, 1);
    let t = GeneratorTunables::default();
    let ungated = sector
        .entries
        .iter()
        .find(|e| sector.gated_system(e.id).is_none())
        .unwrap();
    let error = generate_system(&sector, ungated.id, &t).unwrap_err();
    assert_eq!(error, NotGated { id: ungated.id });
    assert!(error.to_string().contains(&ungated.id.to_string()));
    let beyond = sector.entries.len() + 5;
    assert_eq!(
        generate_system(&sector, beyond, &t),
        Err(NotGated { id: beyond })
    );
    let gated = sector.gated[0].id;
    assert!(generate_system(&sector, gated, &t).is_ok());
}

/// SYSG-02: a system's seed comes from the sector seed, "system" and its
/// id, so it differs between systems and sectors.
#[test]
fn each_system_draws_from_its_own_seed() {
    assert_eq!(
        system_seed(7, 12),
        derive_indexed_seed(7, "system", 12),
        "seed from the sector seed, \"system\" and the id"
    );
    assert_ne!(system_seed(7, 12), system_seed(7, 13));
    assert_ne!(system_seed(7, 12), system_seed(8, 12));
}

/// SYSG-04: every natural body is neutral and surveyed, and the only
/// records besides planets, moons and belt bodies are the gate wrecks:
/// no ships or stations.
#[test]
fn natural_bodies_are_neutral_and_surveyed_with_no_ships() {
    let (_, systems) = medium(4);
    for system in &systems {
        assert_eq!(system.star_knowledge, Knowledge::Surveyed);
        for body in every_record(system) {
            assert_eq!(body.affiliation, Affiliation::Neutral, "{}", body.name);
            assert_eq!(body.knowledge, Knowledge::Surveyed, "{}", body.name);
            assert!(body.record.is_none() && body.group.is_none());
            assert!(body.propulsion.is_none() && body.orbits.is_empty());
            let class = body.class;
            assert!(
                matches!(
                    class,
                    BodyClass::Planet | BodyClass::Moon | BodyClass::SalvageSite
                ),
                "{}: {class:?}",
                body.name
            );
            if class == BodyClass::SalvageSite {
                assert!(body.name.ends_with(" gate wreck"), "{}", body.name);
            }
        }
        assert!(system.belts.iter().all(|b| b.salvage_sites.is_empty()));
    }
}

/// SYSG-10: the star takes the system's name, its summary's mass and
/// radius, and the key facts Class, Age, Luminosity and Metallicity.
#[test]
fn the_star_is_named_for_the_system_with_its_facts() {
    let (sector, systems) = medium(10);
    for (gated, system) in sector.gated.iter().zip(&systems) {
        let entry = &sector.entries[gated.id];
        let star = entry.primary().unwrap();
        assert_eq!(system.star_name, gated.names.system);
        assert_eq!(
            system.star_key_facts,
            star::key_facts(star, entry.age_gyr, entry.metallicity)
        );
        let labels: Vec<&str> = system.star_key_facts.iter().map(|f| &f.0[..]).collect();
        assert_eq!(labels, ["Class", "Age", "Luminosity", "Metallicity"]);
        assert_eq!(system.star_key_facts[0].1, star.spectral_type);
        assert_eq!(system.star_mass_kg, star.mass_msun * SOLAR_MASS_KG);
        assert_eq!(system.star_radius_km, star.radius_rsun * SOLAR_RADIUS_KM);
    }
}

/// SYSG-20 / SYSG-22: each planet is a record round the star, named by the
/// sector, at its summary orbit with its summary's mass, radius and facts;
/// natural bodies round one parent never share an orbit.
#[test]
fn planets_are_their_summaries_under_their_sector_names() {
    let (sector, systems) = medium(20);
    for (gated, system) in sector.gated.iter().zip(&systems) {
        let summary = sector.summary(gated.id).unwrap();
        for ((planet, record), name) in summary
            .planets
            .iter()
            .zip(&system.planets)
            .zip(&gated.names.planets)
        {
            assert_eq!(&record.name, name);
            assert_eq!(record.class, BodyClass::Planet);
            assert_eq!(own_radius(record), planet.orbit_au * KM_PER_AU);
            assert_eq!(record.mass_kg, planet.mass_earth * EARTH_MASS_KG);
            assert_eq!(record.planet_facts, Some(planet_facts(planet)));
            let mut radii: Vec<f64> = record.children.iter().map(own_radius).collect();
            radii.dedup();
            assert_eq!(radii.len(), record.children.len());
        }
        let mut round_the_star: Vec<f64> = system
            .planets
            .iter()
            .filter(|b| b.class != BodyClass::SalvageSite)
            .map(own_radius)
            .collect();
        round_the_star.sort_by(f64::total_cmp);
        round_the_star.dedup();
        let natural = system
            .planets
            .iter()
            .filter(|b| b.class == BodyClass::Planet);
        assert_eq!(round_the_star.len(), natural.count());
    }
}

/// The starting phases of every natural body, by name.
fn phases(system: &SystemData) -> Vec<(String, f64)> {
    every_record(system)
        .into_iter()
        .filter(|b| b.class != BodyClass::SalvageSite)
        .map(|b| (b.name.clone(), b.phase0_rad))
        .collect()
}

/// SYSG-21: each body's phase comes from its own seed: giving planets
/// other moons leaves the planets' and belt bodies' phases as they were,
/// and so does dropping the outermost planet.
#[test]
fn a_bodys_phase_does_not_depend_on_other_bodies() {
    let t = GeneratorTunables::default();
    let sector = testing::sector(30, 21);
    let no_moons = GeneratorTunables {
        moon_max_count: 0,
        ..t.clone()
    };
    let mut seen_moons = false;
    for gated in &sector.gated {
        let base = phases(&generate_system(&sector, gated.id, &t).unwrap());
        let moonless = phases(&generate_system(&sector, gated.id, &no_moons).unwrap());
        seen_moons |= base.len() > moonless.len();
        for body in &moonless {
            assert!(base.contains(body), "{} moved", body.0);
        }
        assert!(
            base.iter()
                .all(|(_, phase)| (0.0..std::f64::consts::TAU).contains(phase)),
            "phases go round once"
        );
    }
    assert!(seen_moons);

    let mut fewer = sector.clone();
    let gated = sector
        .gated
        .iter()
        .find(|g| sector.summary(g.id).unwrap().planets.len() >= 2)
        .unwrap();
    fewer.summaries[gated.id].as_mut().unwrap().planets.pop();
    let base = phases(&generate_system(&sector, gated.id, &t).unwrap());
    let less = phases(&generate_system(&fewer, gated.id, &t).unwrap());
    assert!(less.len() < base.len());
    for body in &less {
        assert!(base.contains(body), "{} moved", body.0);
    }
}

/// SYSG-22 / SYSG-23 / SYSG-33: every planet, moon and belt body carries
/// planet facts of its kind and the description built from them; a
/// moon's are a rocky planet's, at its planet's distance from the star.
#[test]
fn natural_bodies_carry_facts_and_descriptions_of_their_kind() {
    let (sector, systems) = medium(22);
    let mut kinds = Vec::new();
    for (gated, system) in sector.gated.iter().zip(&systems) {
        for body in every_record(system) {
            if body.class == BodyClass::SalvageSite {
                assert!(body.planet_facts.is_none());
                continue;
            }
            let facts = body.planet_facts.unwrap();
            assert_eq!(body.description, description(&facts));
            kinds.push(facts.kind);
            if body.class == BodyClass::Planet && !gated.names.planets.contains(&body.name) {
                assert_eq!(facts.kind, WorldKind::Rocky, "a belt body is rocky");
            }
        }
        for planet in &system.planets {
            for moon in &planet.children {
                let facts = moon.planet_facts.unwrap();
                assert_eq!(facts.kind, WorldKind::Moon);
                assert_ne!(facts.atmosphere, Atmosphere::Envelope);
                let planet_t = planet.planet_facts.unwrap().equilibrium_temperature_k;
                assert_eq!(facts.equilibrium_temperature_k, planet_t);
            }
        }
    }
    for kind in [WorldKind::Moon, WorldKind::Rocky, WorldKind::GasGiant] {
        assert!(kinds.contains(&kind), "{kind:?}");
    }
}

/// SYSG-34 / SYSG-40: moons are lettered from their planet, nearest
/// first, and belts named from the system.
#[test]
fn moons_and_belts_are_named_from_their_planet_and_system() {
    let (sector, systems) = medium(34);
    for (gated, system) in sector.gated.iter().zip(&systems) {
        for planet in &system.planets {
            for (index, moon) in planet.children.iter().enumerate() {
                assert_eq!(moon.name, moons::moon_name(&planet.name, index));
            }
            let radii: Vec<f64> = planet.children.iter().map(own_radius).collect();
            assert!(radii.is_sorted());
        }
        let names: Vec<&str> = system.belts.iter().map(|b| &b.name[..]).collect();
        assert_eq!(
            names,
            belt_bodies::belt_names(&gated.names.system, system.belts.len())
        );
    }
}

/// SYSG-51 to SYSG-53: each link end is a gate (working or dormant) or a
/// wreck (collapsed), named for the destination, on the ring.
#[test]
fn link_ends_become_gates_or_wrecks_on_the_ring() {
    let t = GeneratorTunables::default();
    let (sector, systems) = medium(51);
    let (mut working, mut dormant, mut wrecks) = (0, 0, 0);
    for (gated, system) in sector.gated.iter().zip(&systems) {
        let ring_km = gated.ring_radius_au * KM_PER_AU;
        for link in sector.links_of(gated.id) {
            let destination = &sector
                .gated_system(link.other_end(gated.id))
                .unwrap()
                .names
                .system;
            if link.state == LinkState::Collapsed {
                wrecks += 1;
                let wreck = system
                    .planets
                    .iter()
                    .find(|b| b.name == format!("{destination} gate wreck"))
                    .unwrap();
                assert_eq!(wreck.class, BodyClass::SalvageSite);
                assert_eq!(own_radius(wreck), ring_km);
                assert_eq!(
                    wreck.key_facts[0].1,
                    format!("Wreck of the gate to {destination}")
                );
                continue;
            }
            let gate = system
                .gates
                .iter()
                .find(|g| &g.destination == destination)
                .unwrap();
            assert_eq!(gate.name, format!("{destination} gate"));
            assert_eq!(gate.distance_km, ring_km);
            assert_eq!(gate.entry_tolerance_kms, t.gate_entry_tolerance_km_s);
            match gate.state {
                GateState::Working => working += 1,
                GateState::Dormant { .. } => dormant += 1,
            }
        }
    }
    assert!(working > 0 && dormant > 0 && wrecks > 0);
}

/// SYSG-71: Corvane and Sol are still the hand-made systems they were:
/// loaded by name, with none of the generator's planet facts.
#[test]
fn the_hand_made_systems_stay_as_they_are() {
    for name in ["corvane", "sol"] {
        let system = crate::content::start::chosen_system(&["--system", name]).unwrap()();
        let records = every_record(&system);
        assert!(records.iter().all(|b| b.planet_facts.is_none()));
        assert!(system.gates.iter().all(|g| g.state == GateState::Working));
    }
}
