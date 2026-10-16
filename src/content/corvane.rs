//! Corvane, the prototype system (TEST-01 to TEST-10), as records in real
//! units (km, kg, days). Its values serve the system-view rules first: see
//! the TEST IDs in `docs/spec/system-view.md`.
//!
//! Natural bodies, stations and the contact K-3 have orbits of their own.
//! Every star, planet and moon gets its two bands at spawn (BAND-01,
//! TEST-20), so Halden IV's named orbits are "Low", empty, and "High",
//! which holds every ship parked there: one shared orbit with many members
//! (TEST-10), the task force TF-1 among them as one group whose four
//! warships share its phase (TEST-06). Moons carry plausible masses so
//! their bands come out.
//!
//! This is the seam a generator replaces (SCL-03): anything that builds a
//! `SystemData` the same shape can stand in for `hand_made_system`.

use crate::sim::body::{Affiliation, BodyClass, Knowledge};
use crate::sim::ship::{Drive, Propulsion};
use crate::sim::system::{
    BeltBlueprint, BodyBlueprint, GateBlueprint, GroupBlueprint, MemberBlueprint,
    NamedOrbitBlueprint, OrbitPlacement, SystemData,
};
use crate::sim::track::{Echelon, Formation, ShipClass, ShipRecord, Track, TroopLoad};
use crate::sim::units::{
    EARTH_MASS_KG, KM_PER_AU, M_PER_KM, SOLAR_MASS_KG, STANDARD_GRAVITY_M_PER_S2,
    gravitational_parameter_km3_per_day2,
};

/// A rocky moon's density, for masses from radii.
const ROCK_DENSITY_KG_PER_M3: f64 = 3_000.0;

/// The mass of a rocky ball of `radius_km`, so a moon's gravity (and its
/// bands, BAND-01) follows from its size.
fn rocky_mass_kg(radius_km: f64) -> f64 {
    let radius_m = radius_km * M_PER_KM;
    ROCK_DENSITY_KG_PER_M3 * 4.0 / 3.0 * std::f64::consts::PI * radius_m.powi(3)
}

/// TEST-20 / TEST-30: the propulsion of the ships that can be ordered to
/// move, with each drive's display name: the warship Orison's "hydrogen
/// NTR" (9.0 km/s exhaust, 0.2 g, impulsive, 9.9 km/s of delta-v) and the
/// tanker RFA Tolland's "NEP ion/Hall" (35 km/s, 0.048 milli-g,
/// low-thrust, 24 km/s).
fn propulsion_for(name: &str) -> Option<Propulsion> {
    match name {
        "Orison" => Some(Propulsion {
            drive: Drive::new("hydrogen NTR", true, 9.0, 0.2 * STANDARD_GRAVITY_M_PER_S2),
            delta_v_kms: 9.9,
        }),
        "RFA Tolland" => Some(Propulsion {
            drive: Drive::new(
                "NEP ion/Hall",
                false,
                35.0,
                0.048e-3 * STANDARD_GRAVITY_M_PER_S2,
            ),
            delta_v_kms: 24.0,
        }),
        _ => None,
    }
}

/// Halden IV's mass, which sets its satellites' periods.
const HALDEN_IV_MASS_KG: f64 = 1.1 * EARTH_MASS_KG;
/// Morrow IX's mass, a large icy moon's (Ganymede-like density at its
/// 2,400 km radius). It sets the periods of the moon's own satellites
/// (TEST-09).
const MORROW_IX_MASS_KG: f64 = 0.02 * EARTH_MASS_KG;
/// Where Morrow IX sits among Morrow's moons.
const MORROW_IX: usize = 8;

/// TEST-03: K-3's slow orbit round Halden IV, by period: the one parked
/// object there with an orbit of its own. Arden Highport sits in the "Low"
/// band and the parked ships in the "High" band, whose radii BAND-01
/// derives.
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
                    propulsion: None,
                    group: None,
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
                    propulsion: None,
                    group: None,
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
                entry_tolerance_kms: 0.5,
            },
            GateBlueprint {
                name: "Gate Brenn",
                destination: "Brenn",
                radius_km: 5.0,
                distance_km: 6.3 * KM_PER_AU,
                angle_deg: 190.0,
                entry_tolerance_kms: 0.5,
            },
            GateBlueprint {
                name: "Gate Soll",
                destination: "Soll",
                radius_km: 5.0,
                distance_km: 6.1 * KM_PER_AU,
                angle_deg: 300.0,
                entry_tolerance_kms: 0.5,
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
        propulsion: None,
        group: None,
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

/// TEST-01: Iver, with the station Iver Yard in its High band (SCL-07),
/// the band nearest the yard's old 45,000 km orbit.
fn iver() -> BodyBlueprint {
    BodyBlueprint {
        name: "Iver",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        propulsion: None,
        group: None,
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
            propulsion: None,
            group: None,
            description: "A dormant pre-collapse shipyard, barely maintained.",
            key_facts: vec![("Status", "Dormant"), ("Owner", "Own")],
            knowledge: surveyed_today(),
            orbit: OrbitPlacement::Named("High"),
            radius_km: 1.0,
            phase0_rad: 0.9,
            mass_kg: 0.0,
            orbits: vec![],
            children: vec![],
        }],
    }
}

/// TEST-01 to TEST-04, TEST-10: Halden IV, the capital, with the
/// satellites the motion rule and grouping are tuned against, and the
/// ships parked in its "High" band.
fn halden_iv() -> BodyBlueprint {
    let moon =
        |name, description, orbit_radius_km, radius_km, phase0_rad, knowledge| BodyBlueprint {
            name,
            class: BodyClass::Moon,
            affiliation: Affiliation::Own,
            record: None,
            propulsion: None,
            group: None,
            description,
            key_facts: vec![("Class", "Moon")],
            knowledge,
            orbit: own(orbit_radius_km),
            radius_km,
            phase0_rad,
            mass_kg: rocky_mass_kg(radius_km),
            orbits: vec![],
            children: vec![],
        };
    BodyBlueprint {
        name: "Halden IV",
        class: BodyClass::Planet,
        affiliation: Affiliation::Own,
        record: None,
        propulsion: None,
        group: None,
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
        orbits: vec![],
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
                    "An ice moon the faction has not yet surveyed up close.",
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
                propulsion: None,
                group: None,
                description: "Halden IV's primary orbital station.",
                key_facts: vec![("Status", "Active"), ("Owner", "Own")],
                knowledge: surveyed_today(),
                // TEST-02 / TEST-10: in the Low band, so about 0.07 d.
                orbit: OrbitPlacement::Named("Low"),
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
        propulsion: propulsion_for(name),
        group: None,
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
    // TEST-06: every own ship here is above the condition warning, so
    // only TF-1's bar shows the warning colour.
    let transport = |load, condition| ShipRecord {
        load,
        condition: Some(condition),
        ..ShipRecord::new(ShipClass::TroopTransport, true)
    };
    let civilian = |condition| ShipRecord {
        condition: Some(condition),
        ..civilian_cargo.clone()
    };
    let in_high: Vec<(&'static str, Affiliation, ShipRecord, &'static str)> = vec![
        (
            "Orison",
            Affiliation::Own,
            ShipRecord {
                condition: Some(0.9),
                ..warship.clone()
            },
            "An own warship parked in high orbit over Halden IV.",
        ),
        (
            "Ardent",
            Affiliation::Own,
            ShipRecord {
                condition: Some(0.8),
                ..warship.clone()
            },
            "An own warship sharing Orison's high orbit over Halden IV.",
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
            transport(
                TroopLoad::Embarked(vec![Formation {
                    name: "1st Arden Guard",
                    echelon: Some(Echelon::Battalion),
                }]),
                0.85,
            ),
            "A troop transport with the 1st Arden Guard embarked.",
        ),
        (
            "Transport Hale",
            Affiliation::Own,
            transport(TroopLoad::Empty, 0.7),
            "A troop transport, empty.",
        ),
        (
            "Transport Brask",
            Affiliation::Own,
            transport(
                TroopLoad::Embarked(vec![
                    Formation {
                        name: "2nd Halden Regiment",
                        echelon: Some(Echelon::Regiment),
                    },
                    Formation {
                        name: "Halden Field Brigade",
                        echelon: Some(Echelon::Brigade),
                    },
                ]),
                0.75,
            ),
            "A troop transport carrying a regiment and a brigade staff.",
        ),
        (
            "Varga Prospector",
            Affiliation::Own,
            civilian(0.65),
            "An ore carrier of Varga Mining, under its own orders.",
        ),
        (
            "Delve",
            Affiliation::Own,
            ShipRecord {
                commanded: true,
                requisitioned: true,
                ..civilian(0.6)
            },
            "A civilian ore carrier chartered by the faction.",
        ),
        (
            "Marten",
            Affiliation::Own,
            ShipRecord {
                armed: true,
                ..civilian(0.95)
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
        // TEST-06 / TEST-10: TF-1 takes the gap between the last ship and
        // the first, so every phase in "High" stays distinct.
        .chain([task_force_tf1(spread_phase(2 * count - 1, 2 * count))])
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

/// TEST-06: the own task force TF-1, four warships parked as one in
/// Halden IV's "High" band at `phase0_rad`. Their conditions, 0.9, 0.5,
/// 0.3 and 0.1, average 0.45, below the condition warning.
fn task_force_tf1(phase0_rad: f64) -> BodyBlueprint {
    let warship = |name, condition| MemberBlueprint {
        name,
        record: ShipRecord {
            condition: Some(condition),
            ..ShipRecord::new(ShipClass::Warship, true)
        },
    };
    BodyBlueprint {
        record: None,
        group: Some(GroupBlueprint {
            task_force: true,
            members: vec![
                warship("Vigilant", 0.9),
                warship("Kestrel", 0.5),
                warship("Harrow", 0.3),
                warship("Tamsin", 0.1),
            ],
        }),
        ..parked(
            "TF-1",
            Affiliation::Own,
            ShipRecord::new(ShipClass::Warship, true),
            "An own task force of four warships under one commander.",
            (OrbitPlacement::Named("High"), phase0_rad),
        )
    }
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
                propulsion: None,
                group: None,
                description: "One of Morrow's eleven moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                orbit: own(MOON_RADII_KM[i]),
                radius_km: MOON_BODY_RADII_KM[i],
                phase0_rad: i as f64 * 1.3,
                mass_kg: rocky_mass_kg(MOON_BODY_RADII_KM[i]),
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
        propulsion: None,
        group: None,
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

/// TEST-09: Morrow IX's one named orbit of its own, "Outer", for the
/// parked trader (9 d, dot mode at 1 d/s); its bands sit closer in. The
/// relay sits in its Low band.
fn morrow_ix_orbits() -> Vec<NamedOrbitBlueprint> {
    vec![NamedOrbitBlueprint {
        name: "Outer",
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
        propulsion: None,
        group: None,
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
        key_facts: vec![("Status", "Active"), ("Owner", "Own")],
        radius_km: 0.5,
        ..satellite(
            "Tern Relay",
            Affiliation::Own,
            ShipRecord::new(ShipClass::Station, true),
            "A signal relay in low orbit round Morrow IX.",
            OrbitPlacement::Named("Low"),
            0.4,
        )
    };
    let trader = satellite(
        "Kessel trader",
        Affiliation::Neutral,
        ShipRecord::new(ShipClass::Cargo, false),
        "A neutral trader parked round Morrow IX.",
        OrbitPlacement::Named("Outer"),
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
    use crate::sim::group::group_record;
    use crate::sim::orbit::kepler_period_days;
    use crate::sim::transfer::places::{BandRadii, band_radii, hill_radius_km};
    use crate::sim::tunables::SimTunables;

    /// BAND-01: the bands a planet gets at spawn, from its radius and its
    /// Hill sphere round the star.
    fn planet_bands(planet: &BodyBlueprint) -> BandRadii {
        let OrbitPlacement::Own { radius_km } = planet.orbit else {
            panic!("{} is not round the star", planet.name);
        };
        let hill = hill_radius_km(radius_km, planet.mass_kg, hand_made_system().star_mass_kg);
        band_radii(planet.radius_km, hill, &SimTunables::default())
    }

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
    /// parent's named orbit's, which is one the parent lists or (for a
    /// planet) one of its derived bands (BAND-01).
    fn orbit_radius_km(parent: &BodyBlueprint, child: &BodyBlueprint) -> f64 {
        match child.orbit {
            OrbitPlacement::Own { radius_km } => radius_km,
            OrbitPlacement::Named(orbit) => parent
                .orbits
                .iter()
                .find(|o| o.name == orbit)
                .map(|o| o.radius_km)
                .or_else(|| match orbit {
                    "Low" => Some(planet_bands(parent).low_km),
                    "High" => planet_bands(parent).high_km,
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{} has no orbit {orbit}", parent.name)),
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
            ("Arden Highport", 0.067),
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

    /// SCL-07 / TEST-10: every station and every parked ship but K-3 sits
    /// in a named orbit of its parent and carries no radius of its own;
    /// K-3 and every natural body have orbits of their own, and no two
    /// natural bodies share a radius.
    #[test]
    fn ships_and_stations_sit_in_named_orbits_and_natural_bodies_apart() {
        fn check(parent: &BodyBlueprint) {
            let mut natural_radii: Vec<f64> = Vec::new();
            for child in &parent.children {
                match child.class {
                    BodyClass::Station => assert!(
                        matches!(child.orbit, OrbitPlacement::Named("Low" | "High")),
                        "{} is not in a band",
                        child.name
                    ),
                    BodyClass::Ship if child.name != "K-3" => assert!(
                        matches!(child.orbit, OrbitPlacement::Named(_)),
                        "{} carries a radius of its own",
                        child.name
                    ),
                    BodyClass::Ship => assert!(
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

    /// TEST-10 / TEST-20: Halden IV lists no orbits of its own, so its
    /// named orbits are exactly its two bands. "Low" holds Arden Highport
    /// alone; "High" holds every parked ship - Orison (TEST-03), a second
    /// own commanded warship, Ardent, the TEST-05 ships and the group TF-1
    /// (TEST-06) - at distinct phases, while K-3 keeps an orbit of its
    /// own. TF-1's ships have no phase of their own: they share the
    /// group's (PHAS-14).
    #[test]
    fn halden_iv_bands_hold_the_station_and_every_parked_ship() {
        let halden = planet("Halden IV");
        assert!(halden.orbits.is_empty());
        assert!(planet_bands(&halden).high_km.is_some());
        let (in_high, elsewhere): (Vec<&BodyBlueprint>, Vec<&BodyBlueprint>) = halden
            .children
            .iter()
            .filter(|c| c.record.is_some() || c.group.is_some())
            .partition(|c| c.orbit == OrbitPlacement::Named("High"));
        assert!(in_high.len() >= 13, "{} ships in High", in_high.len());
        let elsewhere: Vec<&str> = elsewhere.iter().map(|c| c.name).collect();
        assert_eq!(elsewhere, ["Arden Highport", "K-3"]);
        let in_low: Vec<&str> = halden
            .children
            .iter()
            .filter(|c| c.orbit == OrbitPlacement::Named("Low"))
            .map(|c| c.name)
            .collect();
        assert_eq!(in_low, ["Arden Highport"]);
        assert!(matches!(
            child(&halden, "K-3").orbit,
            OrbitPlacement::Own { .. }
        ));
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
        let high = planet_bands(&halden).high_km.unwrap();
        assert_eq!(orbit_radius_km(&halden, child(&halden, "Ardent")), high);
    }

    /// TEST-20: every star, planet and moon has a Low band; each has a
    /// High band where BAND-01 allows one, which with rocky masses is
    /// every moon but Morrow's two innermost (whose Hill spheres are too
    /// small at any plausible density, F-146).
    #[test]
    fn every_natural_body_carries_its_bands() {
        let system = hand_made_system();
        let tunables = SimTunables::default();
        let star = band_radii(system.star_radius_km, None, &tunables);
        assert!(star.high_km.is_some());
        for planet in &system.planets {
            let bands = planet_bands(planet);
            assert!(bands.high_km.is_some(), "{} has no High band", planet.name);
            let OrbitPlacement::Own { radius_km: a } = planet.orbit else {
                unreachable!()
            };
            for moon in planet
                .children
                .iter()
                .filter(|c| c.class == BodyClass::Moon)
            {
                let OrbitPlacement::Own { radius_km } = moon.orbit else {
                    unreachable!()
                };
                assert!(moon.mass_kg > 0.0, "{} is massless", moon.name);
                let hill = hill_radius_km(radius_km, moon.mass_kg, planet.mass_kg);
                let bands = band_radii(moon.radius_km, hill, &tunables);
                assert!(bands.low_km > moon.radius_km);
                let expect_high = !matches!(moon.name, "Morrow I" | "Morrow II");
                assert_eq!(
                    bands.high_km.is_some(),
                    expect_high,
                    "{} at {a:.0} km: {bands:?}",
                    moon.name
                );
            }
        }
    }

    /// TEST-20: Orison holds a hydrogen NTR (9.0 km/s, 0.2 g, impulsive)
    /// with 9.9 km/s of delta-v; RFA Tolland an NEP ion/Hall drive (35
    /// km/s, 0.048 milli-g, low-thrust) with 24 km/s. No other ship can be
    /// ordered to move yet.
    #[test]
    fn the_warship_and_the_tanker_carry_their_drives() {
        let halden = planet("Halden IV");
        let orison = child(&halden, "Orison").propulsion.unwrap();
        assert!(orison.drive.impulsive);
        assert_eq!(orison.drive.exhaust_velocity_kms, 9.0);
        let g = crate::sim::units::accel_km_per_day2_from_m_per_s2(STANDARD_GRAVITY_M_PER_S2);
        assert!((orison.drive.accel / g - 0.2).abs() < 1e-9);
        assert_eq!(orison.delta_v_kms, 9.9);
        let tolland = child(&halden, "RFA Tolland").propulsion.unwrap();
        assert!(!tolland.drive.impulsive);
        assert_eq!(tolland.drive.exhaust_velocity_kms, 35.0);
        assert!((tolland.drive.accel / g - 0.048e-3).abs() < 1e-12);
        assert_eq!(tolland.delta_v_kms, 24.0);
        assert!(child(&halden, "Ardent").propulsion.is_none());
    }

    /// TEST-30: the drives the planner shows carry their display names:
    /// Orison's is "hydrogen NTR" and RFA Tolland's "NEP ion/Hall".
    #[test]
    fn the_drives_carry_their_display_names() {
        let halden = planet("Halden IV");
        let name = |ship| child(&halden, ship).propulsion.unwrap().drive.name;
        assert_eq!(name("Orison"), "hydrogen NTR");
        assert_eq!(name("RFA Tolland"), "NEP ion/Hall");
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

    /// TEST-09: Morrow IX holds its own station, in its Low band, and a
    /// trader, at periods of about 0.13 d (ring mode at 1 d/s) and 9 d (dot
    /// mode), inside its Hill sphere and clear of its surface.
    #[test]
    fn morrow_ix_has_two_satellites_of_its_own() {
        let morrow = planet("Morrow");
        let ix = &morrow.children[MORROW_IX];
        assert_eq!(ix.name, "Morrow IX");
        let ix_orbit_km = orbit_radius_km(&morrow, ix);
        let hill_km = ix_orbit_km * (ix.mass_kg / (3.0 * morrow.mass_kg)).cbrt();
        assert_eq!(child(ix, "Tern Relay").orbit, OrbitPlacement::Named("Low"));
        for (name, class, affiliation, expected) in [
            ("Tern Relay", BodyClass::Station, Affiliation::Own, 0.127),
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
            assert!(radius > ix.radius_km, "{name} above the surface");
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

    /// What the faction knows of a body in Halden IV's orbit, by name.
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

    /// TEST-05: one of each kind of ship the symbols tell apart, and own
    /// stations.
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

        let iver = planet("Iver");
        let yard = iver.children[0].record.as_ref().unwrap();
        assert_eq!(
            (iver.children[0].affiliation, yard.class),
            (Affiliation::Own, ShipClass::Station)
        );
    }

    /// TEST-06: TF-1 is an own task force of four commanded warships in
    /// Halden IV's "High" band, with conditions 0.9, 0.5, 0.3 and 0.1, so
    /// its mean is 0.45; it holds no record of its own (SYM-09).
    #[test]
    fn tf1_is_a_task_force_of_four_warships() {
        let halden = planet("Halden IV");
        let tf1 = child(&halden, "TF-1");
        assert_eq!(tf1.affiliation, Affiliation::Own);
        assert_eq!(tf1.orbit, OrbitPlacement::Named("High"));
        assert!(tf1.record.is_none());
        let group = tf1.group.as_ref().expect("TF-1 is a group");
        assert!(group.task_force);
        let conditions: Vec<f64> = group
            .members
            .iter()
            .map(|m| {
                assert!(m.record.commanded && m.record.class == ShipClass::Warship);
                m.record.condition.unwrap()
            })
            .collect();
        assert_eq!(conditions, [0.9, 0.5, 0.3, 0.1]);
        let records: Vec<&ShipRecord> = group.members.iter().map(|m| &m.record).collect();
        let mean = group_record(&records).unwrap().condition.unwrap();
        assert!((mean - 0.45).abs() < 1e-12);
    }

    /// TEST-06: every own ship of TEST-03 and TEST-05 has a condition,
    /// and all of them are 0.55 or more: above the warning fraction.
    #[test]
    fn own_ships_are_in_good_condition() {
        for name in [
            "Orison",
            "RFA Tolland",
            "Transport Cassel",
            "Transport Hale",
            "Transport Brask",
            "Varga Prospector",
            "Delve",
            "Marten",
        ] {
            let (_, record) = known(name);
            assert!(record.condition.unwrap() >= 0.55, "{name}");
        }
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
