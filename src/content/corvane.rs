//! Corvane, the prototype system (TEST-01 to TEST-04), as records in real
//! units (km, kg, days). Its values serve the system-view rules first: see
//! the TEST IDs in `docs/spec/system-view.md`.
//!
//! This is the seam a generator replaces (SCL-03): anything that builds a
//! `SystemData` the same shape can stand in for `hand_made_system`.

use crate::sim::body::{BodyClass, Knowledge, Ownership};
use crate::sim::system::{BeltBlueprint, BodyBlueprint, GateBlueprint, SystemData};
use crate::sim::units::{EARTH_MASS_KG, KM_PER_AU, SOLAR_MASS_KG};

fn surveyed_today() -> Knowledge {
    Knowledge::Surveyed { imaged_day: 0.0 }
}

pub fn hand_made_system() -> SystemData {
    let star_mass_kg = 0.78 * SOLAR_MASS_KG; // K2V

    SystemData {
        star_name: "Corvane",
        star_class: "K2V",
        star_mass_kg,
        star_knowledge: surveyed_today(),
        planets: vec![pell(), iver(), halden_iv(), morrow()],
        belt: BeltBlueprint {
            inner_radius_km: 1.15 * KM_PER_AU,
            outer_radius_km: 1.45 * KM_PER_AU,
            salvage_sites: vec![
                BodyBlueprint {
                    name: "Wreck Field",
                    class: BodyClass::SalvageSite,
                    ownership: Ownership::Neutral,
                    description: "An unsurveyed debris field scattered across the belt.",
                    key_facts: vec![("Contents", "unknown")],
                    knowledge: Knowledge::Unsurveyed,
                    orbit_radius_km: 1.3 * KM_PER_AU,
                    phase0_rad: 1.1,
                    mass_kg: 0.0,
                    children: vec![],
                },
                BodyBlueprint {
                    name: "Lantern",
                    class: BodyClass::SalvageSite,
                    ownership: Ownership::Neutral,
                    description: "The wreck of the station Lantern, last imaged years ago.",
                    key_facts: vec![("Recoverable then", "4-6 kt"), ("Since then", "unknown")],
                    knowledge: Knowledge::Stale {
                        last_imaged_day: -1_006.0,
                    },
                    orbit_radius_km: 1.22 * KM_PER_AU,
                    phase0_rad: 4.0,
                    mass_kg: 0.0,
                    children: vec![],
                },
            ],
        },
        gates: vec![
            GateBlueprint {
                name: "Gate Ostra",
                destination: "Ostra",
                distance_km: 6.0 * KM_PER_AU,
                angle_deg: 40.0,
            },
            GateBlueprint {
                name: "Gate Brenn",
                destination: "Brenn",
                distance_km: 6.3 * KM_PER_AU,
                angle_deg: 190.0,
            },
            GateBlueprint {
                name: "Gate Soll",
                destination: "Soll",
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
        ownership: Ownership::Own,
        description: "A scorched inner rock, too hot for anything but automated survey gear.",
        key_facts: vec![("Class", "Hot rocky planet"), ("Orbit", "0.35 AU")],
        knowledge: surveyed_today(),
        orbit_radius_km: 0.35 * KM_PER_AU,
        phase0_rad: 0.4,
        mass_kg: 0.28 * EARTH_MASS_KG,
        children: vec![],
    }
}

/// TEST-01: Iver, with the station Iver Yard.
fn iver() -> BodyBlueprint {
    BodyBlueprint {
        name: "Iver",
        class: BodyClass::Planet,
        ownership: Ownership::Own,
        description: "A tidally-locked world whose pre-collapse orbital yard has sat dormant for a generation.",
        key_facts: vec![("Class", "Rocky planet"), ("Orbit", "0.7 AU")],
        knowledge: surveyed_today(),
        orbit_radius_km: 0.7 * KM_PER_AU,
        phase0_rad: 2.1,
        mass_kg: 0.9 * EARTH_MASS_KG,
        children: vec![BodyBlueprint {
            name: "Iver Yard",
            class: BodyClass::Station,
            ownership: Ownership::Own,
            description: "A dormant pre-collapse shipyard, barely maintained.",
            key_facts: vec![("Status", "Dormant"), ("Owner", "State")],
            knowledge: surveyed_today(),
            orbit_radius_km: 45_000.0,
            phase0_rad: 0.9,
            mass_kg: 0.0,
            children: vec![],
        }],
    }
}

/// TEST-01 to TEST-04: Halden IV, the capital, with the satellites the
/// motion rule and grouping are tuned against.
fn halden_iv() -> BodyBlueprint {
    BodyBlueprint {
        name: "Halden IV",
        class: BodyClass::Planet,
        ownership: Ownership::Own,
        description: "Cold terrestrial capital world: three settlements under a thin nitrogen sky.",
        key_facts: vec![
            ("Class", "Cold terrestrial · capital world"),
            ("Population", "212k"),
            ("Settlements", "3"),
            ("Surface", "0.82 g, thin N2, -41 C"),
            ("Governor", "A. Voss"),
        ],
        knowledge: Knowledge::Surveyed { imaged_day: -40.0 },
        orbit_radius_km: 2.0 * KM_PER_AU,
        phase0_rad: 4.4,
        mass_kg: 1.1 * EARTH_MASS_KG,
        children: vec![
            BodyBlueprint {
                name: "Halden IVa",
                class: BodyClass::Moon,
                ownership: Ownership::Own,
                description: "The innermost of Halden IV's four moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                // Radii below are set from target periods via the
                // inverse of Kepler's third law, so the motion rule
                // has both modes visible at 1 day/s (board 6):
                // IVa 1.5 d, IVb 4.2 d, IVc 31 d, IVd 68 d,
                // Arden Highport 0.1 d.
                orbit_radius_km: 57_137.0,
                phase0_rad: 0.0,
                mass_kg: 0.0,
                children: vec![],
            },
            BodyBlueprint {
                name: "Halden IVb",
                class: BodyClass::Moon,
                ownership: Ownership::Own,
                description: "The second of Halden IV's four moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                orbit_radius_km: 113_507.0,
                phase0_rad: 1.7,
                mass_kg: 0.0,
                children: vec![],
            },
            BodyBlueprint {
                name: "Halden IVc",
                class: BodyClass::Moon,
                ownership: Ownership::Own,
                description: "An ice moon the state has not yet surveyed up close.",
                key_facts: vec![
                    ("Water ice", "likely, 2-9 Gt"),
                    ("Surface sites", "unknown"),
                ],
                knowledge: Knowledge::Unsurveyed,
                orbit_radius_km: 430_292.0,
                phase0_rad: 3.3,
                mass_kg: 0.0,
                children: vec![],
            },
            BodyBlueprint {
                name: "Halden IVd",
                class: BodyClass::Moon,
                ownership: Ownership::Own,
                description: "The outermost of Halden IV's four moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                orbit_radius_km: 726_432.0,
                phase0_rad: 5.0,
                mass_kg: 0.0,
                children: vec![],
            },
            BodyBlueprint {
                name: "Arden Highport",
                class: BodyClass::Station,
                ownership: Ownership::Own,
                description: "Halden IV's primary orbital station.",
                key_facts: vec![("Status", "Active"), ("Owner", "State")],
                knowledge: surveyed_today(),
                orbit_radius_km: 9_394.0,
                phase0_rad: 2.4,
                mass_kg: 0.0,
                children: vec![],
            },
            // Placeholder contacts so the count pill's indicators
            // have something to report. Both are plain orbiting
            // objects; there's no ship simulation behind them yet.
            BodyBlueprint {
                name: "K-3",
                class: BodyClass::Ship,
                ownership: Ownership::Foreign,
                description: "An unidentified contact holding a slow parking orbit.",
                key_facts: vec![("Class", "Unidentified contact"), ("Track", "weak")],
                knowledge: Knowledge::Unsurveyed,
                // Between IVc and IVd, as board 6 shows it: far
                // enough out to read as a slow orbit (~45 d, so it
                // stays a dot and reads as a diamond), but not so
                // far that it stretches the satellite frame.
                orbit_radius_km: 550_000.0,
                phase0_rad: 2.6,
                mass_kg: 0.0,
                children: vec![],
            },
            BodyBlueprint {
                name: "Orison",
                class: BodyClass::Ship,
                ownership: Ownership::Own,
                description: "A state vessel parked in orbit over Halden IV.",
                key_facts: vec![("Class", "Parked ship"), ("Owner", "State")],
                knowledge: surveyed_today(),
                orbit_radius_km: 25_000.0,
                phase0_rad: 1.0,
                mass_kg: 0.0,
                children: vec![],
            },
        ],
    }
}

/// TEST-01: Morrow, the outer giant with many moons.
fn morrow() -> BodyBlueprint {
    let moons = (0..11)
        .map(|i| {
            let name = MORROW_MOON_NAMES[i];
            BodyBlueprint {
                name,
                class: BodyClass::Moon,
                ownership: Ownership::Own,
                description: "One of Morrow's eleven moons.",
                key_facts: vec![("Class", "Moon")],
                knowledge: surveyed_today(),
                orbit_radius_km: 40_000.0 + i as f64 * 25_000.0,
                phase0_rad: i as f64 * 1.3,
                mass_kg: 0.0,
                children: vec![],
            }
        })
        .collect();

    BodyBlueprint {
        name: "Morrow",
        class: BodyClass::Planet,
        ownership: Ownership::Own,
        description: "An outer giant with a retinue of eleven moons.",
        key_facts: vec![("Class", "Gas giant"), ("Moons", "11")],
        knowledge: surveyed_today(),
        orbit_radius_km: 4.3 * KM_PER_AU,
        phase0_rad: 1.0,
        mass_kg: 60.0 * EARTH_MASS_KG,
        children: moons,
    }
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

    fn satellite_period_days(parent: &BodyBlueprint, name: &str) -> f64 {
        let child = parent
            .children
            .iter()
            .find(|child| child.name == name)
            .unwrap_or_else(|| panic!("{name} missing from {}", parent.name));
        kepler_period_days(child.orbit_radius_km, parent.mass_kg)
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

        let halden = planet("Halden IV");
        let k3 = halden.children.iter().find(|c| c.name == "K-3").unwrap();
        assert_eq!(k3.class, BodyClass::Ship);
        assert_eq!(k3.ownership, Ownership::Foreign);

        let parked = halden
            .children
            .iter()
            .find(|c| c.class == BodyClass::Ship && c.ownership == Ownership::Own);
        assert!(
            parked.is_some(),
            "an own ship should be parked at Halden IV"
        );
    }

    /// TEST-04: the inspection data covers all three knowledge states.
    #[test]
    fn inspection_data_covers_every_knowledge_state() {
        let system = hand_made_system();
        let halden = planet("Halden IV");
        assert!(matches!(halden.knowledge, Knowledge::Surveyed { .. }));
        let ivc = halden
            .children
            .iter()
            .find(|c| c.name == "Halden IVc")
            .unwrap();
        assert!(matches!(ivc.knowledge, Knowledge::Unsurveyed));
        let lantern = system
            .belt
            .salvage_sites
            .iter()
            .find(|s| s.name == "Lantern")
            .unwrap();
        assert!(matches!(lantern.knowledge, Knowledge::Stale { .. }));
    }
}
