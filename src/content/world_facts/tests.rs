//! Tests of the hand-made systems' authored facts (SURF-60 to SURF-63).

use super::*;
use crate::content::corvane::hand_made_system;
use crate::content::sol::sol_system;
use std::collections::HashSet;

/// Every planet and moon of `system`, depth first.
fn natural_bodies(system: &SystemData) -> Vec<&BodyBlueprint> {
    fn walk<'a>(body: &'a BodyBlueprint, out: &mut Vec<&'a BodyBlueprint>) {
        if matches!(body.class, BodyClass::Planet | BodyClass::Moon) {
            out.push(body);
        }
        for child in &body.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    for planet in &system.planets {
        walk(planet, &mut out);
    }
    out
}

/// The natural body called `name`'s facts.
fn facts_of(system: &SystemData, name: &str) -> PlanetFacts {
    natural_bodies(system)
        .into_iter()
        .find(|body| body.name == name)
        .and_then(|body| body.planet_facts)
        .unwrap_or_else(|| panic!("{name} has no facts"))
}

/// The kind, atmosphere, water, locking and activity of a world.
type Summary = (WorldKind, Atmosphere, Water, bool, Option<Activity>);

fn summary(facts: PlanetFacts) -> Summary {
    (
        facts.kind,
        facts.atmosphere,
        facts.water,
        facts.locked,
        facts.activity,
    )
}

/// SURF-60: every planet and moon of Corvane and Sol carries planet facts
/// and a surface seed, and no two worlds of a system share a seed; every
/// row of each table names a world.
#[test]
fn every_hand_made_world_carries_facts_and_its_own_seed() {
    for (system, rows) in [(hand_made_system(), 19), (sol_system(), 30)] {
        let worlds = natural_bodies(&system);
        assert_eq!(
            worlds.len(),
            rows,
            "{}: one row per world",
            system.star_name
        );
        let mut seeds = HashSet::new();
        for world in worlds {
            assert!(world.planet_facts.is_some(), "{} has no facts", world.name);
            let seed = world.surface_seed.expect("a surface seed");
            assert!(seeds.insert(seed), "{} shares a seed", world.name);
        }
    }
}

/// SURF-61 / SURF-63: Earth carries the lava look override, and no other
/// world of either system carries one.
#[test]
fn only_earth_carries_a_look_override() {
    for system in [hand_made_system(), sol_system()] {
        for world in natural_bodies(&system) {
            let expected = (world.name == "Earth").then_some(LookOverride::Lava);
            assert_eq!(world.look_override, expected, "{}", world.name);
        }
    }
}

/// SURF-60: Corvane is a K2V star of 5.1 Gyr whose luminosity is the
/// main-sequence luminosity of its 0.78 solar masses (SGEN-22), about
/// 0.37; Sol is a G2V star of 1.0 solar luminosity and 4.6 Gyr.
#[test]
fn the_stars_carry_their_type_luminosity_and_age() {
    let t = GeneratorTunables::default();
    let corvane = hand_made_system().star_facts;
    let expected = main_sequence_luminosity(0.78, &t.star_mass_luminosity);
    assert_eq!(corvane.spectral_type, "K2V");
    assert!((corvane.luminosity_lsun - expected).abs() < 1e-12);
    assert!((corvane.luminosity_lsun - 0.37).abs() < 0.01);
    assert_eq!(corvane.age_gyr, 5.1);

    let sol = sol_system().star_facts;
    assert_eq!(
        (sol.spectral_type.as_str(), sol.luminosity_lsun, sol.age_gyr),
        ("G2V", 1.0, 4.6)
    );
}

/// SURF-60 / SGEN-38 / SGEN-39: gravity and equilibrium temperature follow
/// the rules from mass, radius, orbit and star: Earth has 1 g and about
/// 255 K, a moon is as warm as its planet, and Halden IV, 2 AU from
/// Corvane, comes out near 140 K.
#[test]
fn gravity_and_temperature_follow_the_rules() {
    let sol = sol_system();
    let earth = facts_of(&sol, "Earth");
    assert!((earth.gravity_g - 1.0).abs() < 0.01, "{}", earth.gravity_g);
    assert!((earth.equilibrium_temperature_k - 255.0).abs() < 1.0);
    let moon = facts_of(&sol, "Moon");
    assert_eq!(
        moon.equilibrium_temperature_k,
        earth.equilibrium_temperature_k
    );
    assert!((moon.gravity_g - 0.165).abs() < 0.005, "{}", moon.gravity_g);

    let corvane = hand_made_system();
    let halden = facts_of(&corvane, "Halden IV");
    assert!((halden.equilibrium_temperature_k - 140.0).abs() < 5.0);
    let moon = facts_of(&corvane, "Halden IVc");
    assert_eq!(
        moon.equilibrium_temperature_k,
        halden.equilibrium_temperature_k
    );
}

/// SURF-62: Corvane's worlds have the facts of table 1: among them a dark
/// locked Pell, a locked Iver, an active Halden IV, an ice moon Halden
/// IVc, a giant Morrow with an extreme Morrow I and an active Morrow II,
/// and ice on Morrow V, VI and IX only.
#[test]
fn corvanes_worlds_follow_table_one() {
    use Activity::*;
    use Atmosphere::*;
    use WorldKind::{GasGiant, Moon, Rocky};
    let system = hand_made_system();
    let rows: [(&str, Summary); 9] = [
        ("Pell", (Rocky, Airless, Water::None, true, Some(Dead))),
        ("Iver", (Rocky, Thin, Water::Ice, true, Some(Dead))),
        ("Halden IV", (Rocky, Thin, Water::Ice, false, Some(Active))),
        ("Halden IVb", (Moon, Airless, Water::None, true, Some(Dead))),
        ("Halden IVc", (Moon, Airless, Water::Ice, true, Some(Dead))),
        ("Morrow", (GasGiant, Envelope, Water::None, false, None)),
        (
            "Morrow I",
            (Moon, Airless, Water::None, true, Some(Extreme)),
        ),
        (
            "Morrow II",
            (Moon, Airless, Water::None, true, Some(Active)),
        ),
        ("Morrow III", (Moon, Airless, Water::None, true, Some(Dead))),
    ];
    for (name, expected) in rows {
        assert_eq!(summary(facts_of(&system, name)), expected, "{name}");
    }
    let icy: Vec<&str> = natural_bodies(&system)
        .into_iter()
        .filter(|b| b.name.starts_with("Morrow "))
        .filter(|b| b.planet_facts.is_some_and(|f| f.water == Water::Ice))
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(icy, ["Morrow V", "Morrow VI", "Morrow IX"]);
}

/// SURF-63: Sol's worlds have the facts of table 2: among them a locked,
/// dense, active Venus, Earth's real facts, the giants' envelopes, an
/// extreme Io, an active Europa and Triton, and a thin-aired, icy Titan.
#[test]
fn sols_worlds_follow_table_two() {
    use Activity::*;
    use Atmosphere::*;
    use WorldKind::{GasGiant, IceGiant, Moon, Rocky};
    let system = sol_system();
    let rows: [(&str, Summary); 14] = [
        ("Mercury", (Rocky, Airless, Water::None, true, Some(Dead))),
        ("Venus", (Rocky, Dense, Water::None, true, Some(Active))),
        ("Earth", (Rocky, Dense, Water::Surface, false, Some(Active))),
        ("Mars", (Rocky, Thin, Water::Ice, false, Some(Dead))),
        ("Ceres", (Rocky, Airless, Water::Ice, false, Some(Dead))),
        ("Saturn", (GasGiant, Envelope, Water::None, false, None)),
        ("Neptune", (IceGiant, Envelope, Water::None, false, None)),
        ("Moon", (Moon, Airless, Water::None, true, Some(Dead))),
        ("Io", (Moon, Airless, Water::None, true, Some(Extreme))),
        ("Europa", (Moon, Airless, Water::Ice, true, Some(Active))),
        ("Callisto", (Moon, Airless, Water::Ice, true, Some(Dead))),
        ("Titan", (Moon, Thin, Water::Ice, true, Some(Dead))),
        ("Triton", (Moon, Thin, Water::Ice, true, Some(Active))),
        ("Proteus", (Moon, Airless, Water::Ice, true, Some(Dead))),
    ];
    for (name, expected) in rows {
        assert_eq!(summary(facts_of(&system, name)), expected, "{name}");
    }
}
