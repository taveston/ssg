//! Sol, the second test system (SOL-01 to SOL-11): the real solar system's
//! Sun, planets, Ceres and main moons, in real units (km, kg, days), so
//! scale, spacing and speed can be judged against figures people know.
//!
//! Orbits are circular, coplanar and prograde, as the simulation requires,
//! so each body sits at its mean distance. Planets start at their mean
//! longitudes at J2000.0, which Sol takes as day 0. There are no stations,
//! ships, gates or salvage sites: Sol is for judging the natural bodies.
//!
//! It is built from the same blueprint types as Corvane (SCL-03), so
//! nothing outside `content/` knows which system it is running.

use crate::sim::body::{Affiliation, BodyClass, Knowledge};
use crate::sim::system::{BeltBlueprint, BodyBlueprint, OrbitPlacement, SystemData};
use crate::sim::units::{KM_PER_AU, SOLAR_MASS_KG};

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
        name,
        class,
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        description,
        key_facts,
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

fn planet(figures: &PlanetFigures) -> BodyBlueprint {
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
        moons_of(figures.name),
    )
}

/// SOL-02 to SOL-08: the Sol system as records.
pub fn sol_system() -> SystemData {
    SystemData {
        star_name: "Sol",
        star_class: "G2V",
        star_mass_kg: SOLAR_MASS_KG,
        star_radius_km: 695_700.0,
        star_knowledge: Knowledge::Surveyed,
        planets: PLANETS.iter().map(planet).collect(),
        belt: BeltBlueprint {
            inner_radius_km: BELT_INNER_AU * KM_PER_AU,
            outer_radius_km: BELT_OUTER_AU * KM_PER_AU,
            salvage_sites: vec![],
        },
        gates: vec![],
    }
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
                std::iter::once((planet, system.star_mass_kg))
                    .chain(planet.children.iter().map(|moon| (moon, planet.mass_kg)))
            })
            .collect()
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
            (system.star_name, system.star_class),
            ("Sol", "G2V"),
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
        let names: Vec<&str> = system.planets.iter().map(|p| p.name).collect();
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
            assert_eq!(planet.key_facts, [("Class", class)], "{name}");
        }
    }

    /// SOL-04: twenty-one moons on their parents, outwards, with the notes
    /// on Triton's and Uranus's moons' real orbits.
    #[test]
    fn the_moons_follow_table_two() {
        let system = sol_system();
        let count: usize = system.planets.iter().map(|p| p.children.len()).sum();
        assert_eq!(count, 21);
        for planet in &system.planets {
            let radii: Vec<f64> = planet.children.iter().map(own_radius).collect();
            assert!(radii.is_sorted(), "{}'s moons run outwards", planet.name);
            for moon in &planet.children {
                assert_eq!(moon.class, BodyClass::Moon);
                assert_eq!(moon.key_facts[0], ("Class", "Moon"), "{}", moon.name);
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
            neptune.children[1].key_facts[1],
            ("Real orbit", "retrograde, drawn prograde")
        );
        let uranus = planet_named("Uranus");
        assert_eq!(
            uranus.children[0].key_facts[1],
            (
                "Real orbit",
                "round Uranus's tilted equator, drawn in the system plane"
            )
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

    /// SOL-07 / SOL-08: one empty belt from 2.2 to 3.2 AU, and nothing but
    /// neutral, surveyed natural bodies.
    #[test]
    fn sol_holds_only_natural_bodies_and_an_empty_belt() {
        let system = sol_system();
        assert_eq!(system.belt.inner_radius_km, 2.2 * KM_PER_AU);
        assert_eq!(system.belt.outer_radius_km, 3.2 * KM_PER_AU);
        assert!(system.belt.salvage_sites.is_empty());
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
                no_high.contains(&body.name),
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
