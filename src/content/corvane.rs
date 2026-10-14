//! Corvane, the prototype system (TEST-01 to TEST-10), as records in real
//! units (km, kg, days). Its values serve the system-view rules first: see
//! the TEST IDs in `docs/spec/system-view.md`.
//!
//! Natural bodies, stations and the contact K-3 have orbits of their own.
//! Halden IV offers two named orbits (SCL-07): "Low", empty, and "High",
//! which holds every ship parked there, so the system has one shared orbit
//! with many members (TEST-10).
//!
//! This is the seam a generator replaces (SCL-03): anything that builds a
//! `SystemData` the same shape can stand in for `hand_made_system`.

use crate::sim::body::{Affiliation, BodyClass, Knowledge};
use crate::sim::system::{
    BeltBlueprint, BodyBlueprint, GateBlueprint, NamedOrbitBlueprint, OrbitPlacement, SystemData,
};
use crate::sim::track::{Echelon, Formation, ShipClass, ShipRecord, Track, TroopLoad};
use crate::sim::units::{
    EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG, gravitational_parameter_km3_per_day2,
};

/// Halden IV's mass, which sets its satellites' periods.
const HALDEN_IV_MASS_KG: f64 = 1.1 * EARTH_MASS_KG;
/// Morrow IX's mass, a large icy moon's (Ganymede-like density at its
/// 2,400 km radius). It sets the periods of the moon's own satellites
/// (TEST-09).
const MORROW_IX_MASS_KG: f64 = 0.02 * EARTH_MASS_KG;
/// Where Morrow IX sits among Morrow's moons.
const MORROW_IX: usize = 8;

/// TEST-02 / TEST-10: Halden IV's orbits by period: Arden Highport's own
/// (0.1 d), the empty "Low" band just above it, "High" (12 d, every parked
/// ship, slow enough to be a dot at 1 d/s) and K-3's own slow orbit.
const HALDEN_HIGHPORT_DAYS: f64 = 0.1;
const HALDEN_LOW_DAYS: f64 = 0.15;
const HALDEN_HIGH_DAYS: f64 = 12.0;
const HALDEN_K3_DAYS: f64 = 45.0;

fn surveyed_today() -> Knowledge {
    Knowledge::Surveyed
}

/// A natural body's own orbit (SCL-07).
fn own(radius_km: f64) -> OrbitPlacement {
    OrbitPlacement::Own { radius_km }
}

pub fn hand_made_system() -> SystemData {
    let star_mass_kg = 0.78 * SOLAR_MASS_KG; // K2V

    SystemData {
        star_name: "Corvane",
        star_class: "K2V",
        star_mass_kg,
        star_radius_km: 508_000.0, // 0.73 solar radii
        star_knowledge: surveyed_today(),
        planets: vec![pell(), iver(), halden_iv(), morrow()],
        belt: BeltBlueprint {
            inner_radius_km: 1.15 * KM_PER_AU,
            outer_radius_km: 1.45 * KM_PER_AU,
            salvage_sites: vec![
                BodyBlueprint {
                    name: "Wreck Field",
                    class: BodyClass::SalvageSite,
                    affiliation: Affiliation::Neutral,
                    record: None,
                    description: "An unsurveyed debris field scattered across the belt.",
                    key_facts: vec![("Contents", "unknown")],
                    knowledge: Knowledge::Unsurveyed,
                    orbit: own(1.3 * KM_PER_AU),
                    radius_km: 2.0,
                    phase0_rad: 1.1,
                    mass_kg: 0.0,
                    orbits: vec![],
                    children: vec![],
                },
                BodyBlueprint {
                    name: "Lantern",
                    class: BodyClass::SalvageSite,
                    affiliation: Affiliation::Neutral,
                    record: None,
                    description: "The wreck of the station Lantern, last imaged years ago.",
                    key_facts: vec![("Recoverable then", "4-6 kt"), ("Since then", "unknown")],
                    knowledge: Knowledge::Unsurveyed,
                    orbit: own(1.22 * KM_PER_AU),
                    radius_km: 0.5,
                    phase0_rad: 4.0,
                    mass_kg: 0.0,
                    orbits: vec![],
                    children: vec![],
                },
            ],
        },
        gates: vec![
            GateBlueprint {
                name: "Gate Ostra",
                destination: "Ostra",
                radius_km: 5.0,
                distance_km: 6.0 * KM_PER_AU,
                angle_deg: 40.0,
            },
            GateBlueprint {
                name: "Gate Brenn",
                destination: "Brenn",
                radius_km: 5.0,
                distance_km: 6.3 * KM_PER_AU,
                angle_deg: 190.0,
            },
            GateBlueprint {
                name: "Gate Soll",
                destination: "Soll",
                radius_km: 5.0,
                distance_km: 6.1 * KM_PER_AU,
                angle_deg: 300.0,
            },
        ],
    }
}

/// TEST-01: Pell, a hot inner rock.
fn pell() -> BodyBlueprint {
    BodyBlueprint {
        name: "Pell",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        description: "A scorched inner rock, too hot for anything but automated survey gear.",
        key_facts: vec![("Class", "Hot rocky planet")],
        knowledge: surveyed_today(),
        orbit: own(0.35 * KM_PER_AU),
        radius_km: 2400.0,
        phase0_rad: 0.4,
        mass_kg: 0.28 * EARTH_MASS_KG,
        orbits: vec![],
        children: vec![],
    }
}

/// TEST-01: Iver, with the station Iver Yard in an orbit of its own.
fn iver() -> BodyBlueprint {
    BodyBlueprint {
        name: "Iver",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        description: "A tidally-locked world whose pre-collapse orbital yard has sat dormant for a generation.",
        key_facts: vec![("Class", "Rocky planet")],
        knowledge: surveyed_today(),
        orbit: own(0.7 * KM_PER_AU),
        radius_km: 5800.0,
        phase0_rad: 2.1,
        mass_kg: 0.9 * EARTH_MASS_KG,
        orbits: vec![],
        children: vec![BodyBlueprint {
            name: "Iver Yard",
            class: BodyClass::Station,
            affiliation: Affiliation::Own,
            record: Some(ShipRecord::new(ShipClass::Station, true)),
            description: "A dormant pre-collapse shipyard, barely maintained.",
            key_facts: vec![("Status", "Dormant"), ("Owner", "State")],
            knowledge: surveyed_today(),
            orbit: own(45_000.0),
            radius_km: 1.0,
            phase0_rad: 0.9,
            mass_kg: 0.0,
            orbits: vec![],
            children: vec![],
        }],
    }
}

/// TEST-01 to TEST-04, TEST-10: Halden IV, the capital, with the
/// satellites the motion rule and grouping are tuned against, its two named
/// orbits, and the ships parked in "High".
fn halden_iv() -> BodyBlueprint {
    let moon =
        |name, description, orbit_radius_km, radius_km, phase0_rad, knowledge| BodyBlueprint {
            name,
            class: BodyClass::Moon,
            affiliation: Affiliation::Own,
            record: None,
            description,
            key_facts: vec![("Class", "Moon")],
            knowledge,
            orbit: own(orbit_radius_km),
            radius_km,
            phase0_rad,
            mass_kg: 0.0,
            orbits: vec![],
            children: vec![],
        };
    BodyBlueprint {
        name: "Halden IV",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        description: "Cold terrestrial capital world: three settlements under a thin nitrogen sky.",
        key_facts: vec![
            ("Class", "Cold terrestrial · capital world"),
            ("Population", "212k"),
            ("Settlements", "3"),
            ("Surface", "0.82 g, thin N2, -41 C"),
            ("Governor", "A. Voss"),
        ],
        knowledge: Knowledge::Surveyed,
        orbit: own(2.0 * KM_PER_AU),
        radius_km: 6700.0,
        phase0_rad: 4.4,
        mass_kg: HALDEN_IV_MASS_KG,
        orbits: halden_orbits(),
        children: vec![
            // Moon radii are set from target periods via the inverse of
            // Kepler's third law, so the motion rule has both modes
            // visible at 1 day/s (board 6): IVa 1.5 d, IVb 4.2 d, IVc 31 d,
            // IVd 68 d.
            moon(
                "Halden IVa",
                "The innermost of Halden IV's four moons.",
                57_137.0,
                600.0,
                0.0,
                surveyed_today(),
            ),
            moon(
                "Halden IVb",
                "The second of Halden IV's four moons.",
                113_507.0,
                1100.0,
                1.7,
                surveyed_today(),
            ),
            BodyBlueprint {
                key_facts: vec![
                    ("Water ice", "likely, 2-9 Gt"),
                    ("Surface sites", "unknown"),
                ],
                ..moon(
                    "Halden IVc",
                    "An ice moon the state has not yet surveyed up close.",
                    430_292.0,
                    2100.0,
                    3.3,
                    Knowledge::Unsurveyed,
                )
            },
            moon(
                "Halden IVd",
                "The outermost of Halden IV's four moons.",
                726_432.0,
                900.0,
                5.0,
                surveyed_today(),
            ),
            BodyBlueprint {
                name: "Arden Highport",
                class: BodyClass::Station,
                affiliation: Affiliation::Own,
                record: Some(ShipRecord::new(ShipClass::Station, true)),
                description: "Halden IV's primary orbital station.",
                key_facts: vec![("Status", "Active"), ("Owner", "State")],
                knowledge: surveyed_today(),
                orbit: own(halden_orbit_km(HALDEN_HIGHPORT_DAYS)),
                radius_km: 1.0,
                phase0_rad: 2.4,
                mass_kg: 0.0,
                orbits: vec![],
                children: vec![],
            },
        ]
        .into_iter()
        // TEST-03 / TEST-05 / TEST-10: the ships and contacts the symbols
        // are checked against.
        .chain(ships_at_halden())
        .collect(),
    }
}

/// SCL-07 / TEST-10: Halden IV's two named orbits: "Low", empty, and
/// "High", where every parked ship sits.
fn halden_orbits() -> Vec<NamedOrbitBlueprint> {
    [("Low", HALDEN_LOW_DAYS), ("High", HALDEN_HIGH_DAYS)]
        .into_iter()
        .map(|(name, period_days)| NamedOrbitBlueprint {
            name,
            radius_km: halden_orbit_km(period_days),
        })
        .collect()
}

/// The orbit radius round Halden IV (1.1 Earth masses) for a period in
/// days, by the inverse of Kepler's third law.
fn halden_orbit_km(period_days: f64) -> f64 {
    orbit_km_for_period(HALDEN_IV_MASS_KG, period_days)
}

/// The orbit radius round a body of `mass_kg` for a period in days, by the
/// inverse of Kepler's third law.
fn orbit_km_for_period(mass_kg: f64, period_days: f64) -> f64 {
    orbits::circular_radius_for_period(gravitational_parameter_km3_per_day2(mass_kg), period_days)
}

/// A ship or contact parked round Halden IV, where `orbit` puts it.
fn parked(
    name: &'static str,
    affiliation: Affiliation,
    record: ShipRecord,
    description: &'static str,
    (orbit, phase0_rad): (OrbitPlacement, f64),
) -> BodyBlueprint {
    let own = affiliation == Affiliation::Own;
    BodyBlueprint {
        name,
        class: BodyClass::Ship,
        affiliation,
        record: Some(record),
        description,
        key_facts: vec![],
        knowledge: if own {
            surveyed_today()
        } else {
            Knowledge::Unsurveyed
        },
        orbit,
        radius_km: 0.1,
        phase0_rad,
        mass_kg: 0.0,
        orbits: vec![],
        children: vec![],
    }
}

/// TEST-10: the phase of the `i`th of `count` ships in "High": spread
/// evenly round the ring, so no two coincide.
fn spread_phase(i: usize, count: usize) -> f64 {
    std::f64::consts::TAU * i as f64 / count as f64
}

/// TEST-03 / TEST-05 / TEST-10: one of each kind of ship and contact the
/// symbols must tell apart. Every ship shares "High"; K-3 keeps a slow
/// orbit of its own.
fn ships_at_halden() -> Vec<BodyBlueprint> {
    let warship = ShipRecord::new(ShipClass::Warship, true);
    let civilian_cargo = ShipRecord::new(ShipClass::Cargo, false);
    let transport = |load| ShipRecord {
        load,
        ..ShipRecord::new(ShipClass::TroopTransport, true)
    };
    let in_high: Vec<(&'static str, Affiliation, ShipRecord, &'static str)> = vec![
        (
            "Orison",
            Affiliation::Own,
            ShipRecord {
                condition: Some(0.9),
                ..warship.clone()
            },
            "A state warship parked in high orbit over Halden IV.",
        ),
        (
            "Ardent",
            Affiliation::Own,
            ShipRecord {
                condition: Some(0.8),
                ..warship.clone()
            },
            "A state warship sharing Orison's high orbit over Halden IV.",
        ),
        (
            "RFA Tolland",
            Affiliation::Own,
            ShipRecord {
                condition: Some(0.55),
                ..ShipRecord::new(ShipClass::FleetSupply, true)
            },
            "A fleet tanker on station.",
        ),
        (
            "Transport Cassel",
            Affiliation::Own,
            transport(TroopLoad::Embarked(vec![Formation {
                name: "1st Arden Guard",
                echelon: Some(Echelon::Battalion),
            }])),
            "A troop transport with the 1st Arden Guard embarked.",
        ),
        (
            "Transport Hale",
            Affiliation::Own,
            transport(TroopLoad::Empty),
            "A troop transport, empty.",
        ),
        (
            "Transport Brask",
            Affiliation::Own,
            transport(TroopLoad::Embarked(vec![
                Formation {
                    name: "2nd Halden Regiment",
                    echelon: Some(Echelon::Regiment),
                },
                Formation {
                    name: "Halden Field Brigade",
                    echelon: Some(Echelon::Brigade),
                },
            ])),
            "A troop transport carrying a regiment and a brigade staff.",
        ),
        (
            "Varga Prospector",
            Affiliation::Own,
            civilian_cargo.clone(),
            "An ore carrier of Varga Mining, under its own orders.",
        ),
        (
            "Delve",
            Affiliation::Own,
            ShipRecord {
                commanded: true,
                requisitioned: true,
                ..civilian_cargo.clone()
            },
            "A civilian ore carrier chartered by the state.",
        ),
        (
            "Marten",
            Affiliation::Own,
            ShipRecord {
                armed: true,
                ..civilian_cargo.clone()
            },
            "A licensed armed hauler under its owner's orders.",
        ),
        (
            "Ostra trader",
            Affiliation::Neutral,
            ShipRecord::new(ShipClass::Cargo, false),
            "A neutral trader of Halvard Lines.",
        ),
        (
            "Sennet",
            Affiliation::Allied,
            ShipRecord::new(ShipClass::Warship, false),
            "An allied frigate.",
        ),
        (
            "Transport T-4",
            Affiliation::Hostile,
            ShipRecord::new(ShipClass::TroopTransport, false),
            "A hostile troop transport; what it carries is unknown.",
        ),
        (
            "Raider R-2",
            Affiliation::Hostile,
            ShipRecord {
                track: Track::Stale {
                    last_observed_day: -6.0,
                },
                ..ShipRecord::new(ShipClass::Warship, false)
            },
            "A hostile raider, last seen six days ago.",
        ),
        (
            "Contact C-9",
            Affiliation::Unknown,
            ShipRecord::new(ShipClass::Cargo, false),
            "A cargo ship whose flag is not known.",
        ),
    ];
    let count = in_high.len();
    in_high
        .into_iter()
        .enumerate()
        .map(|(i, (name, affiliation, record, description))| {
            parked(
                name,
                affiliation,
                record,
                description,
                (OrbitPlacement::Named("High"), spread_phase(i, count)),
            )
        })
        .chain([parked(
            "K-3",
            Affiliation::Unknown,
            ShipRecord {
                track: Track::Weak,
                ..ShipRecord::new(ShipClass::Unknown, false)
            },
            "An unidentified contact holding a slow parking orbit.",
            (own(halden_orbit_km(HALDEN_K3_DAYS)), 2.6),
        )])
        .collect()
}

/// TEST-01: Morrow, the outer giant with many moons.
fn morrow() -> BodyBlueprint {
    // Radii come from the target periods by the inverse of Kepler's third
    // law, for Morrow's 318 Earth masses (TEST-08): 0.35, 0.55, 0.9, 1.8,
    // 3.6, 7.2, 16, 30, 50, 75 and 110 d.
    const MOON_RADII_KM: [f64; 11] = [
        143_190.0,
        193_542.0,
        268_758.0,
        426_627.0,
        677_229.0,
        1_075_033.0,
        1_830_685.0,
        2_783_649.0,
        3_913_034.0,
        5_127_525.0,
        6_619_046.0,
    ];
    // Mean physical radii of the moons, a few hundred to a few thousand km.
    const MOON_BODY_RADII_KM: [f64; 11] = [
        350.0, 500.0, 800.0, 1_200.0, 2_600.0, 1_800.0, 700.0, 450.0, 2_400.0, 320.0, 600.0,
    ];
    let mut moons: Vec<BodyBlueprint> = (0..11)
        .map(|i| {
            let name = MORROW_MOON_NAMES[i];
            BodyBlueprint {
                name,
                class: BodyClass::Moon,
                affiliation: Affiliation::Own,
                record: None,
                description: "One of Morrow's eleven moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                orbit: own(MOON_RADII_KM[i]),
                radius_km: MOON_BODY_RADII_KM[i],
                phase0_rad: i as f64 * 1.3,
                mass_kg: 0.0,
                orbits: vec![],
                children: vec![],
            }
        })
        .collect();
    moons[MORROW_IX].mass_kg = MORROW_IX_MASS_KG;
    moons[MORROW_IX].orbits = morrow_ix_orbits();
    moons[MORROW_IX].children = morrow_ix_satellites();

    BodyBlueprint {
        name: "Morrow",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        description: "An outer giant with a retinue of eleven moons.",
        key_facts: vec![("Class", "Gas giant"), ("Moons", "11")],
        knowledge: surveyed_today(),
        orbit: own(4.3 * KM_PER_AU),
        radius_km: 71500.0,
        phase0_rad: 1.0,
        mass_kg: 318.0 * EARTH_MASS_KG,
        orbits: vec![],
        children: moons,
    }
}

/// TEST-09: Morrow IX's one named orbit, "High", for the parked trader
/// (9 d); the relay has an orbit of its own.
fn morrow_ix_orbits() -> Vec<NamedOrbitBlueprint> {
    vec![NamedOrbitBlueprint {
        name: "High",
        radius_km: orbit_km_for_period(MORROW_IX_MASS_KG, 9.0),
    }]
}

/// TEST-09: Morrow IX's own satellites, for nested folding, nested pills
/// and a frame three levels deep: a station fast enough to be an exposure
/// ring at 1 d/s, and a trader slow enough to be a dot.
fn morrow_ix_satellites() -> Vec<BodyBlueprint> {
    let satellite = |name, affiliation, record, description, orbit, phase0_rad| BodyBlueprint {
        name,
        class: BodyClass::Ship,
        affiliation,
        record: Some(record),
        description,
        key_facts: vec![],
        knowledge: Knowledge::Unsurveyed,
        orbit,
        radius_km: 0.1,
        phase0_rad,
        mass_kg: 0.0,
        orbits: vec![],
        children: vec![],
    };
    let relay = BodyBlueprint {
        class: BodyClass::Station,
        knowledge: surveyed_today(),
        key_facts: vec![("Status", "Active"), ("Owner", "State")],
        radius_km: 0.5,
        ..satellite(
            "Tern Relay",
            Affiliation::Own,
            ShipRecord::new(ShipClass::Station, true),
            "A signal relay in low orbit round Morrow IX.",
            own(orbit_km_for_period(MORROW_IX_MASS_KG, 0.3)),
            0.4,
        )
    };
    let trader = satellite(
        "Kessel trader",
        Affiliation::Neutral,
        ShipRecord::new(ShipClass::Cargo, false),
        "A neutral trader parked round Morrow IX.",
        OrbitPlacement::Named("High"),
        2.2,
    );
    vec![relay, trader]
}

const MORROW_MOON_NAMES: [&str; 11] = [
    "Morrow I",
    "Morrow II",
    "Morrow III",
    "Morrow IV",
    "Morrow V",
    "Morrow VI",
    "Morrow VII",
    "Morrow VIII",
    "Morrow IX",
    "Morrow X",
    "Morrow XI",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::orbit::kepler_period_days;

    fn planet(name: &str) -> BodyBlueprint {
        hand_made_system()
            .planets
            .into_iter()
            .find(|planet| planet.name == name)
            .unwrap_or_else(|| panic!("{name} missing from the test system"))
    }

    fn child<'a>(parent: &'a BodyBlueprint, name: &str) -> &'a BodyBlueprint {
        parent
            .children
            .iter()
            .find(|child| child.name == name)
            .unwrap_or_else(|| panic!("{name} missing from {}", parent.name))
    }

    /// SCL-07: the radius a child's placement comes to: its own, or its
    /// parent's named orbit's.
    fn orbit_radius_km(parent: &BodyBlueprint, child: &BodyBlueprint) -> f64 {
        match child.orbit {
            OrbitPlacement::Own { radius_km } => radius_km,
            OrbitPlacement::Named(orbit) => {
                parent
                    .orbits
                    .iter()
                    .find(|o| o.name == orbit)
                    .unwrap_or_else(|| panic!("{} has no orbit {orbit}", parent.name))
                    .radius_km
            }
        }
    }

    fn satellite_period_days(parent: &BodyBlueprint, name: &str) -> f64 {
        kepler_period_days(orbit_radius_km(parent, child(parent, name)), parent.mass_kg)
    }

    /// TEST-02: the satellite periods the motion rule is tuned against, so
    /// both modes are visible at 1 d/s.
    #[test]
    fn halden_iv_satellite_periods() {
        let halden = planet("Halden IV");
        for (name, expected) in [
            ("Arden Highport", 0.1),
            ("Halden IVa", 1.5),
            ("Halden IVb", 4.2),
            ("Halden IVc", 31.0),
            ("Halden IVd", 68.0),
        ] {
            let actual = satellite_period_days(&halden, name);
            let tolerance = expected * 0.02;
            assert!(
                (actual - expected).abs() < tolerance,
                "{name}: expected about {expected} d, got {actual:.3} d"
            );
        }
    }

    /// TEST-02: radii follow Kepler from Halden IV's mass, and the spread
    /// straddles both motion thresholds at 1 d/s (ring under 20 s, dot at
    /// or over 25 s), which is the whole point of these values.
    #[test]
    fn periods_straddle_both_motion_modes_at_one_day_per_second() {
        let halden = planet("Halden IV");
        assert!(satellite_period_days(&halden, "Arden Highport") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVa") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVb") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVc") >= 25.0);
        assert!(satellite_period_days(&halden, "Halden IVd") >= 25.0);
    }

    /// SCL-07 / TEST-10: every parked ship but K-3 sits in a named orbit
    /// of its parent and carries no radius of its own; stations, K-3 and
    /// every natural body have orbits of their own, and no two natural
    /// bodies share a radius.
    #[test]
    fn ships_sit_in_named_orbits_and_natural_bodies_apart() {
        fn check(parent: &BodyBlueprint) {
            let mut natural_radii: Vec<f64> = Vec::new();
            for child in &parent.children {
                match child.class {
                    BodyClass::Ship if child.name != "K-3" => assert!(
                        matches!(child.orbit, OrbitPlacement::Named(_)),
                        "{} carries a radius of its own",
                        child.name
                    ),
                    BodyClass::Ship | BodyClass::Station => assert!(
                        matches!(child.orbit, OrbitPlacement::Own { .. }),
                        "{} keeps an orbit of its own",
                        child.name
                    ),
                    _ => {
                        let OrbitPlacement::Own { radius_km } = child.orbit else {
                            panic!("{} has no orbit of its own", child.name);
                        };
                        assert!(
                            !natural_radii.contains(&radius_km),
                            "{} shares an orbit",
                            child.name
                        );
                        natural_radii.push(radius_km);
                    }
                }
                // Resolves, or panics with the missing orbit.
                orbit_radius_km(parent, child);
                check(child);
            }
        }
        for planet in hand_made_system().planets {
            check(&planet);
        }
    }

    /// TEST-10: Halden IV has exactly "Low" (empty) and "High"; "High"
    /// holds every parked ship - Orison (TEST-03), a second own commanded
    /// warship, Ardent, and the TEST-05 ships - at distinct phases, while
    /// Arden Highport and K-3 keep orbits of their own.
    #[test]
    fn halden_iv_high_orbit_holds_every_parked_ship() {
        let halden = planet("Halden IV");
        let names: Vec<&str> = halden.orbits.iter().map(|o| o.name).collect();
        assert_eq!(names, ["Low", "High"]);
        let (in_high, elsewhere): (Vec<&BodyBlueprint>, Vec<&BodyBlueprint>) = halden
            .children
            .iter()
            .filter(|c| c.record.is_some())
            .partition(|c| c.orbit == OrbitPlacement::Named("High"));
        assert!(in_high.len() >= 13, "{} ships in High", in_high.len());
        let elsewhere: Vec<&str> = elsewhere.iter().map(|c| c.name).collect();
        assert_eq!(elsewhere, ["Arden Highport", "K-3"]);
        assert!(
            !halden
                .children
                .iter()
                .any(|c| c.orbit == OrbitPlacement::Named("Low"))
        );
        for name in ["Orison", "Ardent"] {
            let ship = child(&halden, name);
            let record = ship.record.as_ref().unwrap();
            assert!(in_high.iter().any(|c| c.name == name));
            assert_eq!(ship.affiliation, Affiliation::Own);
            assert!(record.commanded && record.class == ShipClass::Warship);
        }
        let mut phases: Vec<f64> = in_high.iter().map(|c| c.phase0_rad).collect();
        phases.sort_by(f64::total_cmp);
        for pair in phases.windows(2) {
            assert!(pair[1] - pair[0] > 1e-6, "two ships share a phase");
        }
        let period = satellite_period_days(&halden, "Ardent");
        assert!((period - HALDEN_HIGH_DAYS).abs() < HALDEN_HIGH_DAYS * 0.02);
    }

    /// TEST-07, TEST-08: Morrow is Jupiter-sized and its moons keep the
    /// target periods, split six in ring mode (under 8 d, so under 8 s at
    /// 1 d/s) and five in dot mode.
    #[test]
    fn morrow_moon_periods_follow_a_jupiter_mass_giant() {
        let morrow = planet("Morrow");
        assert_eq!(morrow.mass_kg, 318.0 * EARTH_MASS_KG);
        let targets = [
            0.35, 0.55, 0.9, 1.8, 3.6, 7.2, 16.0, 30.0, 50.0, 75.0, 110.0,
        ];
        assert_eq!(morrow.children.len(), targets.len());
        for (moon, expected) in morrow.children.iter().zip(targets) {
            let actual = kepler_period_days(orbit_radius_km(&morrow, moon), morrow.mass_kg);
            assert!(
                (actual - expected).abs() < expected * 0.02,
                "{}: expected about {expected} d, got {actual:.3} d",
                moon.name
            );
        }
        assert!(orbit_radius_km(&morrow, &morrow.children[0]) >= 71_500.0);
        let ring = morrow
            .children
            .iter()
            .filter(|m| kepler_period_days(orbit_radius_km(&morrow, m), morrow.mass_kg) < 8.0)
            .count();
        assert_eq!(ring, 6);
    }

    /// TEST-09: Morrow IX holds its own station and trader, at periods of
    /// about 0.3 d (ring mode at 1 d/s) and 9 d (dot mode), inside its Hill
    /// sphere and clear of its surface.
    #[test]
    fn morrow_ix_has_two_satellites_of_its_own() {
        let morrow = planet("Morrow");
        let ix = &morrow.children[MORROW_IX];
        assert_eq!(ix.name, "Morrow IX");
        let ix_orbit_km = orbit_radius_km(&morrow, ix);
        let hill_km = ix_orbit_km * (ix.mass_kg / (3.0 * morrow.mass_kg)).cbrt();
        for (name, class, affiliation, expected) in [
            ("Tern Relay", BodyClass::Station, Affiliation::Own, 0.3),
            ("Kessel trader", BodyClass::Ship, Affiliation::Neutral, 9.0),
        ] {
            let satellite = child(ix, name);
            assert_eq!(
                (satellite.class, satellite.affiliation),
                (class, affiliation)
            );
            assert!(satellite.record.is_some(), "{name} is drawn as a symbol");
            let actual = satellite_period_days(ix, name);
            assert!(
                (actual - expected).abs() < expected * 0.02,
                "{name}: expected about {expected} d, got {actual:.3} d"
            );
            let radius = orbit_radius_km(ix, satellite);
            assert!(radius < hill_km / 2.0, "{name} inside the Hill sphere");
            assert!(radius > 2.0 * ix.radius_km, "{name} above the surface");
        }
    }

    /// TEST-01 / TEST-03: the bodies the other checks lean on are present.
    #[test]
    fn corvane_contains_the_required_bodies() {
        let system = hand_made_system();
        let planet_names: Vec<&str> = system.planets.iter().map(|p| p.name).collect();
        for name in ["Pell", "Iver", "Halden IV", "Morrow"] {
            assert!(planet_names.contains(&name), "{name} missing");
        }

        let site_names: Vec<&str> = system.belt.salvage_sites.iter().map(|s| s.name).collect();
        assert!(site_names.contains(&"Wreck Field"));
        assert!(site_names.contains(&"Lantern"));
        assert!(!system.gates.is_empty());
    }

    /// What the state knows of a body in Halden IV's orbit, by name.
    fn known(name: &str) -> (Affiliation, ShipRecord) {
        let halden = planet("Halden IV");
        let body = halden
            .children
            .into_iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} missing"));
        (body.affiliation, body.record.expect("a ship record"))
    }

    /// TEST-03: K-3 is an unknown contact of unknown class on a weak
    /// track, and an own commanded warship is parked at Halden IV.
    #[test]
    fn halden_iv_has_k3_and_an_own_warship() {
        let (affiliation, k3) = known("K-3");
        assert_eq!(affiliation, Affiliation::Unknown);
        assert_eq!((k3.class, k3.track), (ShipClass::Unknown, Track::Weak));
        let (affiliation, orison) = known("Orison");
        assert_eq!(affiliation, Affiliation::Own);
        assert!(orison.commanded && orison.class == ShipClass::Warship);
    }

    /// TEST-05: one of each kind of ship the symbols tell apart, all in
    /// slow orbits (dots at 1 d/s, under the 8 s tail), and own stations.
    #[test]
    fn corvane_has_one_of_each_kind_of_ship() {
        let check = |name: &str, want: &dyn Fn(Affiliation, &ShipRecord) -> bool| {
            let (affiliation, record) = known(name);
            assert!(want(affiliation, &record), "{name}");
        };
        let own = |a: Affiliation| a == Affiliation::Own;
        check("RFA Tolland", &|a, r| {
            own(a) && r.commanded && r.class == ShipClass::FleetSupply
        });
        check("Transport Cassel", &|a, r| {
            own(a)
                && r.load
                    == TroopLoad::Embarked(vec![Formation {
                        name: "1st Arden Guard",
                        echelon: Some(Echelon::Battalion),
                    }])
        });
        check("Transport Hale", &|a, r| {
            own(a) && r.load == TroopLoad::Empty
        });
        check("Varga Prospector", &|a, r| {
            own(a) && !r.commanded && r.class == ShipClass::Cargo && !r.armed
        });
        check("Delve", &|a, r| own(a) && r.commanded && r.requisitioned);
        check("Marten", &|a, r| own(a) && !r.commanded && r.armed);
        check("Ostra trader", &|a, r| {
            a == Affiliation::Neutral && r.class == ShipClass::Cargo
        });
        check("Sennet", &|a, r| {
            a == Affiliation::Allied && r.class == ShipClass::Warship
        });
        check("Transport T-4", &|a, r| {
            a == Affiliation::Hostile
                && r.class == ShipClass::TroopTransport
                && r.load == TroopLoad::Unknown
        });
        check("Raider R-2", &|a, r| {
            a == Affiliation::Hostile
                && r.track
                    == Track::Stale {
                        last_observed_day: -6.0,
                    }
        });
        check("Contact C-9", &|a, r| {
            a == Affiliation::Unknown && r.class == ShipClass::Cargo
        });
        check("Arden Highport", &|a, r| {
            own(a) && r.class == ShipClass::Station
        });

        let halden = planet("Halden IV");
        for ship in halden
            .children
            .iter()
            .filter(|c| c.class == BodyClass::Ship)
        {
            let period = kepler_period_days(orbit_radius_km(&halden, ship), halden.mass_kg);
            assert!(
                period >= 8.0,
                "{} is a dot at 1 d/s: {period:.1} d",
                ship.name
            );
        }
        let iver = planet("Iver");
        let yard = iver.children[0].record.as_ref().unwrap();
        assert_eq!(
            (iver.children[0].affiliation, yard.class),
            (Affiliation::Own, ShipClass::Station)
        );
    }

    /// TEST-04: the inspection data covers both knowledge states.
    #[test]
    fn inspection_data_covers_every_knowledge_state() {
        let halden = planet("Halden IV");
        assert!(matches!(halden.knowledge, Knowledge::Surveyed));
        let ivc = child(&halden, "Halden IVc");
        assert!(matches!(ivc.knowledge, Knowledge::Unsurveyed));
    }
}
