//! Sol, the second test system (SOL-01 to SOL-17): the real solar system's
//! Sun, planets, Ceres and main moons, in real units (km, kg, days), so
//! scale, spacing and speed can be judged against figures people know.
//!
//! Orbits are circular, coplanar and prograde, as the simulation requires,
//! so each body sits at its mean distance. Planets start at their mean
//! longitudes at J2000.0, which Sol takes as day 0. There are no gates or
//! salvage sites. Round Earth sit two own stations and two own ships, for
//! trying transfers: a station, a near-future nuclear thermal ship docked
//! at it and a relic fusion-torch ship in Earth's Low band, and a second
//! station in its High band.
//!
//! It is built from the same blueprint types as Corvane (SCL-03), so
//! nothing outside `content/` knows which system it is running.

use crate::content::generator::tunables::GeneratorTunables;
use crate::content::world_facts;
use crate::sim::body::{Affiliation, BodyClass, Knowledge};
use crate::sim::ship::{Drive, Propulsion};
use crate::sim::system::{BeltBlueprint, BodyBlueprint, OrbitPlacement, SystemData, facts};
use crate::sim::track::{ShipClass, ShipRecord};
use crate::sim::units::{KM_PER_AU, SOLAR_MASS_KG, STANDARD_GRAVITY_M_PER_S2};

/// One row of the planet table (SOL-03, SOL-06).
struct PlanetFigures {
    name: &'static str,
    orbit_radius_au: f64,
    radius_km: f64,
    mass_kg: f64,
    /// Mean longitude at J2000.0, Sol's day 0.
    phase0_deg: f64,
    class: &'static str,
    description: &'static str,
}

/// One row of the moon table (SOL-04). Moons are listed outwards from
/// their parent, which sets their starting phases (SOL-06).
struct MoonFigures {
    name: &'static str,
    parent: &'static str,
    orbit_radius_km: f64,
    radius_km: f64,
    mass_kg: f64,
}

/// SOL-03 / SOL-06: the planets, Ceres included, inner to outer.
const PLANETS: [PlanetFigures; 9] = [
    PlanetFigures {
        name: "Mercury",
        orbit_radius_au: 0.387099,
        radius_km: 2_439.7,
        mass_kg: 3.301e23,
        phase0_deg: 252.250,
        class: "Terrestrial planet",
        description: "The smallest planet and the closest to the Sun.",
    },
    PlanetFigures {
        name: "Venus",
        orbit_radius_au: 0.723336,
        radius_km: 6_051.8,
        mass_kg: 4.867e24,
        phase0_deg: 181.979,
        class: "Terrestrial planet",
        description: "A rocky world under a crushing carbon dioxide sky.",
    },
    PlanetFigures {
        name: "Earth",
        orbit_radius_au: 1.00000,
        radius_km: 6_371.0,
        mass_kg: 5.972e24,
        phase0_deg: 100.465,
        class: "Terrestrial planet",
        description: "The third planet, with one large moon.",
    },
    PlanetFigures {
        name: "Mars",
        orbit_radius_au: 1.52371,
        radius_km: 3_389.5,
        mass_kg: 6.417e23,
        phase0_deg: 355.447,
        class: "Terrestrial planet",
        description: "A cold desert planet with two small moons.",
    },
    PlanetFigures {
        name: "Ceres",
        orbit_radius_au: 2.768,
        radius_km: 469.7,
        mass_kg: 9.384e20,
        phase0_deg: 0.0,
        class: "Dwarf planet",
        description: "The largest body in the main asteroid belt.",
    },
    PlanetFigures {
        name: "Jupiter",
        orbit_radius_au: 5.20289,
        radius_km: 69_911.0,
        mass_kg: 1.898e27,
        phase0_deg: 34.396,
        class: "Gas giant",
        description: "The largest planet, with four great moons.",
    },
    PlanetFigures {
        name: "Saturn",
        orbit_radius_au: 9.53668,
        radius_km: 58_232.0,
        mass_kg: 5.683e26,
        phase0_deg: 49.954,
        class: "Gas giant",
        description: "A ringed giant whose largest moon, Titan, has a thick atmosphere.",
    },
    PlanetFigures {
        name: "Uranus",
        orbit_radius_au: 19.1892,
        radius_km: 25_362.0,
        mass_kg: 8.681e25,
        phase0_deg: 313.238,
        class: "Ice giant",
        description: "An ice giant tipped on its side.",
    },
    PlanetFigures {
        name: "Neptune",
        orbit_radius_au: 30.0699,
        radius_km: 24_622.0,
        mass_kg: 1.024e26,
        phase0_deg: 304.880,
        class: "Ice giant",
        description: "The outermost planet, with a captured moon, Triton.",
    },
];

/// SOL-04: the moons, grouped by parent and outwards within each.
const MOONS: [MoonFigures; 21] = [
    moon("Moon", "Earth", 384_400.0, 1_737.4, 7.346e22),
    moon("Phobos", "Mars", 9_375.0, 11.08, 1.062e16),
    moon("Deimos", "Mars", 23_457.0, 6.2, 1.441e15),
    moon("Amalthea", "Jupiter", 181_400.0, 83.5, 2.466e18),
    moon("Io", "Jupiter", 421_800.0, 1_821.5, 8.932e22),
    moon("Europa", "Jupiter", 671_100.0, 1_560.8, 4.800e22),
    moon("Ganymede", "Jupiter", 1_070_400.0, 2_631.2, 1.482e23),
    moon("Callisto", "Jupiter", 1_882_700.0, 2_410.3, 1.076e23),
    moon("Mimas", "Saturn", 185_520.0, 198.2, 3.751e19),
    moon("Enceladus", "Saturn", 238_020.0, 252.0, 1.08e20),
    moon("Tethys", "Saturn", 294_660.0, 531.1, 6.175e20),
    moon("Dione", "Saturn", 377_400.0, 561.4, 1.096e21),
    moon("Rhea", "Saturn", 527_040.0, 763.3, 2.31e21),
    moon("Titan", "Saturn", 1_221_870.0, 2_575.0, 1.346e23),
    moon("Iapetus", "Saturn", 3_560_850.0, 734.5, 1.81e21),
    moon("Ariel", "Uranus", 190_900.0, 578.9, 1.29e21),
    moon("Umbriel", "Uranus", 266_000.0, 584.7, 1.22e21),
    moon("Titania", "Uranus", 436_300.0, 788.9, 3.42e21),
    moon("Oberon", "Uranus", 583_500.0, 761.4, 2.88e21),
    moon("Proteus", "Neptune", 117_647.0, 208.0, 5.0e19),
    moon("Triton", "Neptune", 354_760.0, 1_353.4, 2.14e22),
];

const fn moon(
    name: &'static str,
    parent: &'static str,
    orbit_radius_km: f64,
    radius_km: f64,
    mass_kg: f64,
) -> MoonFigures {
    MoonFigures {
        name,
        parent,
        orbit_radius_km,
        radius_km,
        mass_kg,
    }
}

/// SOL-06: the *i*th moon of a parent, counting outwards from 0, starts
/// this many radians times *i* round its orbit.
const MOON_PHASE_STEP_RAD: f64 = 1.3;

/// SOL-07: the main asteroid belt's edges.
const BELT_INNER_AU: f64 = 2.2;
const BELT_OUTER_AU: f64 = 3.2;

/// The planet the stations and ships of table 3 sit round (SOL-12).
const SHIPS_PLANET: &str = "Earth";

/// SOL-12 / SOL-15 / SOL-16: a ship's or station's kind and drive.
enum Craft {
    /// A station: no drive.
    Station,
    /// A commanded, unarmed cargo ship with an impulsive drive of this
    /// display name, exhaust velocity (km/s), loaded acceleration (g) and
    /// delta-v (km/s).
    Ship {
        drive_name: &'static str,
        exhaust_velocity_kms: f64,
        accel_g: f64,
        delta_v_kms: f64,
    },
}

/// One row of table 3: a station or ship round Earth (SOL-12 to SOL-16).
struct CraftFigures {
    name: &'static str,
    craft: Craft,
    /// Earth's band it is a member of: "Low" or "High" (SOL-13).
    band: &'static str,
    /// Its angle at day 0, in the sense of motion (SOL-14).
    phase0_deg: f64,
    description: &'static str,
}

/// SOL-12 to SOL-16: table 3. Marlin shares Perigee Station's phase, so it
/// starts docked there; Vestige sits a third of the way round the band.
const CRAFT: [CraftFigures; 4] = [
    CraftFigures {
        name: "Perigee Station",
        craft: Craft::Station,
        band: "Low",
        phase0_deg: 0.0,
        description: "A station in low Earth orbit.",
    },
    CraftFigures {
        name: "Marlin",
        // SOL-15: hydrogen nuclear thermal, sized to reach Ceres.
        craft: Craft::Ship {
            drive_name: "hydrogen NTR",
            exhaust_velocity_kms: 9.0,
            accel_g: 0.1,
            delta_v_kms: 15.0,
        },
        band: "Low",
        phase0_deg: 0.0,
        description: "A near-future nuclear thermal ship.",
    },
    CraftFigures {
        name: "Vestige",
        // SOL-16: a relic continuous fusion torch.
        craft: Craft::Ship {
            drive_name: "fusion torch",
            exhaust_velocity_kms: 500.0,
            accel_g: 0.02,
            delta_v_kms: 500.0,
        },
        band: "Low",
        phase0_deg: 120.0,
        description: "A relic fusion-torch ship.",
    },
    CraftFigures {
        name: "Apogee Station",
        craft: Craft::Station,
        band: "High",
        phase0_deg: 240.0,
        description: "A station in high Earth orbit.",
    },
];

/// SOL-12 to SOL-14: a station or ship of table 3: own, surveyed, a member
/// of its Earth band at its phase, with no orbit of its own.
fn craft(figures: &CraftFigures) -> BodyBlueprint {
    let (class, record, propulsion, key_facts) = match figures.craft {
        Craft::Station => (
            BodyClass::Station,
            ShipRecord::new(ShipClass::Station, true),
            None,
            facts(&[("Status", "Active"), ("Owner", "Own")]),
        ),
        Craft::Ship {
            drive_name,
            exhaust_velocity_kms,
            accel_g,
            delta_v_kms,
        } => (
            BodyClass::Ship,
            ShipRecord::new(ShipClass::Cargo, true),
            Some(Propulsion {
                drive: Drive::new(
                    drive_name,
                    true,
                    exhaust_velocity_kms,
                    accel_g * STANDARD_GRAVITY_M_PER_S2,
                ),
                delta_v_kms,
            }),
            vec![],
        ),
    };
    BodyBlueprint {
        name: figures.name.into(),
        class,
        affiliation: Affiliation::Own,
        record: Some(record),
        propulsion,
        group: None,
        description: figures.description.into(),
        key_facts,
        planet_facts: None,
        surface_seed: None,
        look_override: None,
        knowledge: Knowledge::Surveyed,
        orbit: OrbitPlacement::Named(figures.band),
        radius_km: if class == BodyClass::Station {
            1.0
        } else {
            0.1
        },
        phase0_rad: figures.phase0_deg.to_radians(),
        mass_kg: 0.0,
        orbits: vec![],
        children: vec![],
    }
}

/// SOL-04: what a moon says about the ways its drawn orbit differs from
/// its real one. Triton really orbits backwards; Uranus's moons orbit its
/// steeply tilted equator.
fn moon_key_facts(moon: &MoonFigures) -> Vec<(&'static str, &'static str)> {
    let mut facts = vec![("Class", "Moon")];
    if moon.name == "Triton" {
        facts.push(("Real orbit", "retrograde, drawn prograde"));
    }
    if moon.parent == "Uranus" {
        facts.push((
            "Real orbit",
            "round Uranus's tilted equator, drawn in the system plane",
        ));
    }
    facts
}

/// A natural body: neutral, surveyed, in an orbit of its own (SOL-08).
fn natural_body(
    name: &'static str,
    class: BodyClass,
    description: &'static str,
    key_facts: Vec<(&'static str, &'static str)>,
    (orbit_radius_km, phase0_rad): (f64, f64),
    (radius_km, mass_kg): (f64, f64),
    children: Vec<BodyBlueprint>,
) -> BodyBlueprint {
    BodyBlueprint {
        name: name.into(),
        class,
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        group: None,
        description: description.into(),
        key_facts: facts(&key_facts),
        planet_facts: None,
        surface_seed: None,
        look_override: None,
        knowledge: Knowledge::Surveyed,
        orbit: OrbitPlacement::Own {
            radius_km: orbit_radius_km,
        },
        radius_km,
        phase0_rad,
        mass_kg,
        orbits: vec![],
        children,
    }
}

/// SOL-04 / SOL-06: a parent's moons, outwards, each at its starting phase.
fn moons_of(parent: &str) -> Vec<BodyBlueprint> {
    MOONS
        .iter()
        .filter(|moon| moon.parent == parent)
        .enumerate()
        .map(|(i, moon)| {
            natural_body(
                moon.name,
                BodyClass::Moon,
                "A moon of the solar system.",
                moon_key_facts(moon),
                (moon.orbit_radius_km, MOON_PHASE_STEP_RAD * i as f64),
                (moon.radius_km, moon.mass_kg),
                vec![],
            )
        })
        .collect()
}

/// SOL-03 / SOL-12: a planet with its moons, and for Earth the stations
/// and ships of table 3.
fn planet(figures: &PlanetFigures) -> BodyBlueprint {
    let mut children = moons_of(figures.name);
    if figures.name == SHIPS_PLANET {
        children.extend(CRAFT.iter().map(craft));
    }
    natural_body(
        figures.name,
        BodyClass::Planet,
        figures.description,
        vec![("Class", figures.class)],
        (
            figures.orbit_radius_au * KM_PER_AU,
            figures.phase0_deg.to_radians(),
        ),
        (figures.radius_km, figures.mass_kg),
        children,
    )
}

/// SOL-02 to SOL-17: the Sol system as records.
pub fn sol_system() -> SystemData {
    let tunables = GeneratorTunables::default();
    let mut system = SystemData {
        star_name: "Sol".into(),
        star_key_facts: facts(&[("Class", "G2V")]),
        star_mass_kg: SOLAR_MASS_KG,
        star_radius_km: 695_700.0,
        star_knowledge: Knowledge::Surveyed,
        // SURF-60: the Sun, at its real luminosity and age.
        star_facts: world_facts::star_facts(
            world_facts::sol::SPECTRAL_TYPE,
            SOLAR_MASS_KG,
            Some(world_facts::sol::LUMINOSITY_LSUN),
            world_facts::sol::AGE_GYR,
            &tunables,
        ),
        planets: PLANETS.iter().map(planet).collect(),
        belts: vec![BeltBlueprint {
            name: "Main belt".to_string(),
            inner_radius_km: BELT_INNER_AU * KM_PER_AU,
            outer_radius_km: BELT_OUTER_AU * KM_PER_AU,
            mass_kg: None,
            composition: None,
            salvage_sites: vec![],
        }],
        gates: vec![],
    };
    // SURF-63: every planet, Ceres and moon carries the facts of its table.
    world_facts::author_worlds(&mut system, &world_facts::sol::WORLDS, &tunables);
    system
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::orbit::kepler_period_days;
    use crate::sim::transfer::places::{BandRadii, band_radii, hill_radius_km};
    use crate::sim::tunables::SimTunables;

    /// The real sidereal periods in days, from the spec's tables (SOL-05).
    const REAL_PERIODS_DAYS: [(&str, f64); 30] = [
        ("Mercury", 87.969),
        ("Venus", 224.701),
        ("Earth", 365.256),
        ("Mars", 686.980),
        ("Ceres", 1_680.0),
        ("Jupiter", 4_332.589),
        ("Saturn", 10_755.699),
        ("Uranus", 30_685.400),
        ("Neptune", 60_189.018),
        ("Moon", 27.3217),
        ("Phobos", 0.3187),
        ("Deimos", 1.2625),
        ("Amalthea", 0.498179),
        ("Io", 1.769138),
        ("Europa", 3.551181),
        ("Ganymede", 7.154553),
        ("Callisto", 16.689017),
        ("Mimas", 0.942422),
        ("Enceladus", 1.370218),
        ("Tethys", 1.887802),
        ("Dione", 2.736915),
        ("Rhea", 4.517500),
        ("Titan", 15.945421),
        ("Iapetus", 79.330183),
        ("Ariel", 2.520379),
        ("Umbriel", 4.144176),
        ("Titania", 8.705867),
        ("Oberon", 13.463234),
        ("Proteus", 1.122315),
        ("Triton", 5.876854),
    ];

    fn own_radius(body: &BodyBlueprint) -> f64 {
        let OrbitPlacement::Own { radius_km } = body.orbit else {
            panic!("{} has no orbit of its own", body.name);
        };
        radius_km
    }

    fn planet_named(name: &str) -> BodyBlueprint {
        sol_system()
            .planets
            .into_iter()
            .find(|planet| planet.name == name)
            .unwrap_or_else(|| panic!("{name} missing from Sol"))
    }

    /// Every natural body below the star with its parent's mass and its
    /// own orbit radius.
    fn every_orbiting_body(system: &SystemData) -> Vec<(&BodyBlueprint, f64)> {
        system
            .planets
            .iter()
            .flat_map(|planet| {
                std::iter::once((planet, system.star_mass_kg)).chain(
                    planet
                        .children
                        .iter()
                        .filter(|child| child.class == BodyClass::Moon)
                        .map(|moon| (moon, planet.mass_kg)),
                )
            })
            .collect()
    }

    /// The stations and ships of table 3, which sit round Earth.
    fn craft_round_earth() -> Vec<BodyBlueprint> {
        planet_named("Earth")
            .children
            .into_iter()
            .filter(|child| child.class != BodyClass::Moon)
            .collect()
    }

    fn craft_named(name: &str) -> BodyBlueprint {
        craft_round_earth()
            .into_iter()
            .find(|craft| craft.name == name)
            .unwrap_or_else(|| panic!("{name} missing round Earth"))
    }

    /// Earth's gravitational parameter, in km³/day².
    fn earth_mu() -> f64 {
        crate::sim::units::gravitational_parameter_km3_per_day2(planet_named("Earth").mass_kg)
    }

    /// Earth's bands with the default tunables (BAND-01).
    fn earth_bands() -> BandRadii {
        bands(&planet_named("Earth"), SOLAR_MASS_KG)
    }

    /// BAND-01: a body's bands from its radius and its Hill sphere round
    /// its parent.
    fn bands(body: &BodyBlueprint, parent_mass_kg: f64) -> BandRadii {
        let hill = hill_radius_km(own_radius(body), body.mass_kg, parent_mass_kg);
        band_radii(body.radius_km, hill, &SimTunables::default())
    }

    /// SOL-02: the star is the Sun.
    #[test]
    fn the_star_is_the_sun() {
        let system = sol_system();
        assert_eq!(
            (system.star_name, system.star_key_facts),
            ("Sol".to_string(), facts(&[("Class", "G2V")])),
            "star name and class"
        );
        assert_eq!(system.star_mass_kg, SOLAR_MASS_KG);
        assert_eq!(system.star_radius_km, 695_700.0);
        assert_eq!(system.star_knowledge, Knowledge::Surveyed);
    }

    /// SOL-03: the nine planets, inner to outer, round the star with
    /// their table figures and classes.
    #[test]
    fn the_planets_follow_table_one() {
        let system = sol_system();
        let names: Vec<&str> = system.planets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Mercury", "Venus", "Earth", "Mars", "Ceres", "Jupiter", "Saturn", "Uranus",
                "Neptune"
            ]
        );
        let earth = planet_named("Earth");
        assert_eq!(own_radius(&earth), KM_PER_AU);
        assert_eq!((earth.radius_km, earth.mass_kg), (6_371.0, 5.972e24));
        let neptune = planet_named("Neptune");
        assert_eq!(own_radius(&neptune), 30.0699 * KM_PER_AU);
        for (name, class) in [
            ("Mercury", "Terrestrial planet"),
            ("Ceres", "Dwarf planet"),
            ("Jupiter", "Gas giant"),
            ("Uranus", "Ice giant"),
        ] {
            let planet = planet_named(name);
            assert_eq!(planet.class, BodyClass::Planet, "{name}");
            assert_eq!(planet.key_facts, facts(&[("Class", class)]), "{name}");
        }
    }

    /// SOL-04: twenty-one moons on their parents, outwards, with the notes
    /// on Triton's and Uranus's moons' real orbits.
    #[test]
    fn the_moons_follow_table_two() {
        let system = sol_system();
        fn moons_of(planet: &BodyBlueprint) -> Vec<&BodyBlueprint> {
            planet
                .children
                .iter()
                .filter(|child| child.class == BodyClass::Moon)
                .collect()
        }
        let count: usize = system.planets.iter().map(|p| moons_of(p).len()).sum();
        assert_eq!(count, 21);
        for planet in &system.planets {
            let moons = moons_of(planet);
            let radii: Vec<f64> = moons.iter().map(|moon| own_radius(moon)).collect();
            assert!(radii.is_sorted(), "{}'s moons run outwards", planet.name);
            for moon in moons {
                assert_eq!(moon.class, BodyClass::Moon);
                assert_eq!(
                    moon.key_facts[..1],
                    facts(&[("Class", "Moon")]),
                    "{}",
                    moon.name
                );
                let notes = moon.key_facts.len() - 1;
                let expected = usize::from(moon.name == "Triton" || planet.name == "Uranus");
                assert_eq!(notes, expected, "{}'s notes", moon.name);
            }
        }
        let jupiter = planet_named("Jupiter");
        let io = &jupiter.children[1];
        assert_eq!(io.name, "Io");
        assert_eq!(
            (own_radius(io), io.radius_km, io.mass_kg),
            (421_800.0, 1_821.5, 8.932e22)
        );
        let neptune = planet_named("Neptune");
        assert_eq!(
            neptune.children[1].key_facts[1..],
            facts(&[("Real orbit", "retrograde, drawn prograde")])
        );
        let uranus = planet_named("Uranus");
        assert_eq!(
            uranus.children[0].key_facts[1..],
            facts(&[(
                "Real orbit",
                "round Uranus's tilted equator, drawn in the system plane"
            )])
        );
    }

    /// SOL-05: Kepler's third law with the parent's mass alone puts every
    /// period within 1% of the real one.
    #[test]
    fn every_period_is_within_one_percent_of_the_real_one() {
        let system = sol_system();
        let bodies = every_orbiting_body(&system);
        assert_eq!(bodies.len(), REAL_PERIODS_DAYS.len());
        for (body, parent_mass_kg) in bodies {
            let (_, real) = REAL_PERIODS_DAYS
                .iter()
                .find(|(name, _)| *name == body.name)
                .unwrap_or_else(|| panic!("no real period for {}", body.name));
            let period = kepler_period_days(own_radius(body), parent_mass_kg);
            assert!(
                (period - real).abs() < real * 0.01,
                "{}: {period:.4} d against a real {real} d",
                body.name
            );
        }
    }

    /// SOL-06: planets start at their J2000.0 mean longitudes (Ceres at
    /// 0), and the *i*th moon outwards at 1.3 × *i* rad.
    #[test]
    fn bodies_start_at_their_day_zero_phases() {
        assert!((planet_named("Earth").phase0_rad - 100.465_f64.to_radians()).abs() < 1e-12);
        assert!((planet_named("Neptune").phase0_rad - 304.880_f64.to_radians()).abs() < 1e-12);
        assert_eq!(planet_named("Ceres").phase0_rad, 0.0);
        let saturn = planet_named("Saturn");
        let phases: Vec<f64> = saturn.children.iter().map(|m| m.phase0_rad).collect();
        let expected: Vec<f64> = (0..7).map(|i| 1.3 * i as f64).collect();
        assert_eq!(phases, expected);
        assert_eq!(planet_named("Earth").children[0].phase0_rad, 0.0);
    }

    /// SOL-07 / SOL-08: one empty belt, the "Main belt", from 2.2 to
    /// 3.2 AU, no gates, every natural body neutral and surveyed, and the stations and ships own
    /// and surveyed.
    #[test]
    fn sol_holds_neutral_bodies_own_craft_and_an_empty_belt() {
        let system = sol_system();
        let [belt] = system.belts.as_slice() else {
            panic!("Sol has {} belts, not one", system.belts.len());
        };
        assert_eq!(belt.name, "Main belt");
        assert_eq!(belt.inner_radius_km, 2.2 * KM_PER_AU);
        assert_eq!(belt.outer_radius_km, 3.2 * KM_PER_AU);
        assert!(belt.salvage_sites.is_empty());
        assert!(system.gates.is_empty());
        for (body, _) in every_orbiting_body(&system) {
            assert!(
                matches!(body.class, BodyClass::Planet | BodyClass::Moon),
                "{}",
                body.name
            );
            assert!(body.record.is_none() && body.propulsion.is_none());
            assert!(body.orbits.is_empty(), "{} lists orbits", body.name);
            assert_eq!(body.affiliation, Affiliation::Neutral, "{}", body.name);
            assert_eq!(body.knowledge, Knowledge::Surveyed, "{}", body.name);
        }
        let craft = craft_round_earth();
        assert_eq!(craft.len(), 4);
        for body in &craft {
            assert_eq!(body.affiliation, Affiliation::Own, "{}", body.name);
            assert_eq!(body.knowledge, Knowledge::Surveyed, "{}", body.name);
        }
    }

    /// SOL-12: two stations with no drive and two commanded, unarmed cargo
    /// ships with drives, all round Earth.
    #[test]
    fn earth_holds_two_stations_and_two_cargo_ships() {
        let craft = craft_round_earth();
        let names: Vec<&str> = craft.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["Perigee Station", "Marlin", "Vestige", "Apogee Station"]
        );
        for name in ["Perigee Station", "Apogee Station"] {
            let station = craft_named(name);
            assert_eq!(station.class, BodyClass::Station, "{name}");
            assert_eq!(station.record.unwrap().class, ShipClass::Station);
            assert!(station.propulsion.is_none(), "{name} has a drive");
        }
        for name in ["Marlin", "Vestige"] {
            let ship = craft_named(name);
            assert_eq!(ship.class, BodyClass::Ship, "{name}");
            let record = ship.record.unwrap();
            assert_eq!(record.class, ShipClass::Cargo, "{name}");
            assert!(record.commanded && !record.armed, "{name}");
            assert!(ship.propulsion.is_some(), "{name} has no drive");
        }
        // No other planet holds anything but moons.
        for planet in sol_system().planets {
            if planet.name != "Earth" {
                assert!(
                    planet.children.iter().all(|c| c.class == BodyClass::Moon),
                    "{}",
                    planet.name
                );
            }
        }
    }

    /// SOL-13: Perigee Station, Marlin and Vestige are in Earth's Low band
    /// and Apogee Station in its High band, none with an orbit of its own.
    /// Low is 6,871 km out (94.5 min) and High 216,614 km out (11.6 d).
    #[test]
    fn the_craft_sit_in_earths_low_and_high_bands() {
        for (name, band) in [
            ("Perigee Station", "Low"),
            ("Marlin", "Low"),
            ("Vestige", "Low"),
            ("Apogee Station", "High"),
        ] {
            let craft = craft_named(name);
            assert!(
                matches!(craft.orbit, OrbitPlacement::Named(b) if b == band),
                "{name} is not in {band}: {:?}",
                craft.orbit
            );
            assert!(craft.orbits.is_empty() && craft.children.is_empty());
        }
        let bands = earth_bands();
        let earth_mass = planet_named("Earth").mass_kg;
        assert!((bands.low_km - 6_871.0).abs() < 1.0, "{bands:?}");
        let low_minutes = kepler_period_days(bands.low_km, earth_mass) * 24.0 * 60.0;
        assert!((low_minutes - 94.5).abs() < 0.1, "{low_minutes} min");
        let high_km = bands.high_km.expect("Earth has a High band");
        assert!((high_km - 216_614.0).abs() < 1.0, "{bands:?}");
        let high_days = kepler_period_days(high_km, earth_mass);
        assert!((high_days - 11.6).abs() < 0.05, "{high_days} d");
    }

    /// SOL-14: the craft start at table 3's phases: Marlin at Perigee
    /// Station's phase, so docked there, and Vestige a third of the way
    /// round the band, far from both.
    #[test]
    fn the_craft_start_at_table_three_phases() {
        let phase = |name| craft_named(name).phase0_rad;
        assert_eq!(phase("Perigee Station"), 0.0);
        assert_eq!(phase("Marlin"), phase("Perigee Station"));
        assert!((phase("Vestige") - 120_f64.to_radians()).abs() < 1e-12);
        assert!((phase("Apogee Station") - 240_f64.to_radians()).abs() < 1e-12);
        // A third of the way round Low is thousands of km of arc.
        let gap_km = phase("Vestige") * earth_bands().low_km;
        assert!(gap_km > 10_000.0, "{gap_km} km");
    }

    /// The drive of one of table 3's ships, with its acceleration in g.
    fn drive_of(name: &str) -> (bool, f64, f64, f64) {
        let propulsion = craft_named(name).propulsion.unwrap();
        let g = crate::sim::units::accel_km_per_day2_from_m_per_s2(STANDARD_GRAVITY_M_PER_S2);
        (
            propulsion.drive.impulsive,
            propulsion.drive.exhaust_velocity_kms,
            propulsion.drive.accel / g,
            propulsion.delta_v_kms,
        )
    }

    /// SOL-15: Marlin's hydrogen nuclear thermal drive: impulsive, 9.0
    /// km/s exhaust, 0.1 g loaded, 15 km/s of delta-v.
    #[test]
    fn marlin_carries_a_nuclear_thermal_drive() {
        let (impulsive, exhaust, accel_g, delta_v) = drive_of("Marlin");
        assert!(impulsive);
        assert_eq!(exhaust, 9.0);
        assert!((accel_g - 0.1).abs() < 1e-9, "{accel_g} g");
        assert_eq!(delta_v, 15.0);
    }

    /// SOL-16: Vestige's fusion torch: impulsive, 500 km/s exhaust, 20
    /// milli-g loaded, 500 km/s of delta-v.
    #[test]
    fn vestige_carries_a_fusion_torch() {
        let (impulsive, exhaust, accel_g, delta_v) = drive_of("Vestige");
        assert!(impulsive);
        assert_eq!(exhaust, 500.0);
        assert!((accel_g - 0.02).abs() < 1e-9, "{accel_g} g");
        assert_eq!(delta_v, 500.0);
    }

    /// SOL-17: the Hohmann crossing from Earth's Low band to its High band
    /// costs about 4.0 km/s; Marlin can make it three times but not four,
    /// and Vestige more than a hundred times.
    #[test]
    fn marlin_crosses_low_to_high_three_times_and_vestige_many() {
        let bands = earth_bands();
        let trip = orbits::hohmann(earth_mu(), bands.low_km, bands.high_km.unwrap());
        let cost_kms = orbits::km_per_day_to_km_per_s(trip.first_burn + trip.second_burn);
        assert!((cost_kms - 4.0).abs() < 0.05, "{cost_kms} km/s");
        let (.., marlin) = drive_of("Marlin");
        assert!(3.0 * cost_kms <= marlin && 4.0 * cost_kms > marlin);
        let (.., vestige) = drive_of("Vestige");
        assert!(vestige / cost_kms > 100.0);
    }

    /// SOL-09: with the default tunables every body has a Low band, and a
    /// High band but for six small inner moons whose Hill spheres are too
    /// small; Tethys keeps its High band by about 19 km.
    #[test]
    fn every_body_has_its_bands_but_six_small_moons_lack_high() {
        let system = sol_system();
        let tunables = SimTunables::default();
        let star = band_radii(system.star_radius_km, None, &tunables);
        assert!(star.low_km > system.star_radius_km && star.high_km.is_some());
        let no_high = [
            "Phobos",
            "Deimos",
            "Amalthea",
            "Mimas",
            "Enceladus",
            "Proteus",
        ];
        for (body, parent_mass_kg) in every_orbiting_body(&system) {
            let bands = bands(body, parent_mass_kg);
            assert!(bands.low_km > body.radius_km, "{}", body.name);
            assert_eq!(
                bands.high_km.is_none(),
                no_high.contains(&body.name.as_str()),
                "{}: {bands:?}",
                body.name
            );
        }
        let saturn = planet_named("Saturn");
        let tethys = bands(&saturn.children[2], saturn.mass_kg);
        assert!((tethys.low_km - 1_031.1).abs() < 1.0, "{tethys:?}");
        assert!(
            (tethys.high_km.unwrap() - 1_050.0).abs() < 5.0,
            "{tethys:?}"
        );
    }
}
