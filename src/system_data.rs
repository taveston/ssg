//! The Corvane system for iteration 2, in real units (km, kg, days). This is
//! the seam a future system generator replaces: anything that builds a tree
//! of `BodyBlueprint`s the same shape can drop in here.

use bevy::prelude::*;

pub const KM_PER_AU: f64 = 149_597_870.7;
const EARTH_MASS_KG: f64 = 5.972e24;
const SOLAR_MASS_KG: f64 = 1.989e30;
const GRAVITATIONAL_CONSTANT: f64 = 6.674e-11;

/// What a marker looks like and how clutter/inspection treat it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyClass {
    Star,
    Planet,
    Moon,
    Station,
    Gate,
    SalvageSite,
    /// A ship or contact. Shape follows ownership, per the sketches'
    /// marker language: triangle for own, diamond for foreign.
    Ship,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ownership {
    Own,
    Foreign,
    Neutral,
}

#[derive(Component, Clone, Debug)]
pub enum Knowledge {
    Surveyed { imaged_day: f64 },
    Unsurveyed,
    Stale { last_imaged_day: f64 },
}

#[derive(Component, Clone, Debug, Default)]
pub struct DisplayName(pub String);

/// Physical radius in km. Never used for marker size - only for the
/// magnifier's size threshold and for inspect facts.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BodyRadius(pub f64);

#[derive(Component, Clone, Debug, Default)]
pub struct Description(pub String);

#[derive(Component, Clone, Debug, Default)]
pub struct KeyFacts(pub Vec<(String, String)>);

/// Kepler's third law: the orbital period of a body at `radius_km` around a
/// `central_mass_kg` point mass. Used uniformly for planets around the star
/// and moons/stations around their planet.
fn kepler_period_days(radius_km: f64, central_mass_kg: f64) -> f64 {
    let radius_m = radius_km * 1000.0;
    let seconds = std::f64::consts::TAU
        * (radius_m.powi(3) / (GRAVITATIONAL_CONSTANT * central_mass_kg)).sqrt();
    seconds / 86_400.0
}

/// A node in the hand-made system tree. Periods are derived from
/// `orbit_radius_km` and the parent's mass, not stored, so they can't drift
/// out of sync with the radius.
pub struct BodyBlueprint {
    pub name: &'static str,
    /// Physical radius. Marker size never derives from this - it's for
    /// the magnifier's size threshold, and for inspect facts later.
    pub radius_km: f64,
    pub class: BodyClass,
    pub ownership: Ownership,
    pub description: &'static str,
    pub key_facts: Vec<(&'static str, &'static str)>,
    pub knowledge: Knowledge,
    pub orbit_radius_km: f64,
    pub phase0_rad: f64,
    /// This body's own mass, used to compute its children's periods. Leave
    /// at 0.0 for bodies with no children.
    pub mass_kg: f64,
    pub children: Vec<BodyBlueprint>,
}

pub struct BeltBlueprint {
    pub inner_radius_km: f64,
    pub outer_radius_km: f64,
    pub salvage_sites: Vec<BodyBlueprint>,
}

pub struct GateBlueprint {
    pub name: &'static str,
    pub destination: &'static str,
    pub distance_km: f64,
    pub angle_deg: f64,
}

pub struct SystemData {
    pub star_name: &'static str,
    pub star_mass_kg: f64,
    pub star_knowledge: Knowledge,
    pub planets: Vec<BodyBlueprint>,
    pub belt: BeltBlueprint,
    pub gates: Vec<GateBlueprint>,
}

fn surveyed_today() -> Knowledge {
    Knowledge::Surveyed { imaged_day: 0.0 }
}

pub fn hand_made_system() -> SystemData {
    let star_mass_kg = 0.78 * SOLAR_MASS_KG; // K2V

    SystemData {
        star_name: "Corvane",
        star_mass_kg,
        star_knowledge: surveyed_today(),
        planets: vec![
            BodyBlueprint {
                name: "Pell",
                radius_km: 2900.0,
                class: BodyClass::Planet,
                ownership: Ownership::Own,
                description: "A scorched inner rock, too hot for anything but automated survey gear.",
                key_facts: vec![("Class", "Hot rocky planet"), ("Orbit", "0.35 AU")],
                knowledge: surveyed_today(),
                orbit_radius_km: 0.35 * KM_PER_AU,
                phase0_rad: 0.4,
                mass_kg: 0.28 * EARTH_MASS_KG,
                children: vec![],
            },
            BodyBlueprint {
                name: "Iver",
                radius_km: 6050.0,
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
                    radius_km: 0.6,
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
            },
            BodyBlueprint {
                name: "Halden IV",
                radius_km: 6700.0,
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
                        radius_km: 380.0,
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
                        radius_km: 1100.0,
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
                        radius_km: 1600.0,
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
                        radius_km: 700.0,
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
                        radius_km: 0.9,
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
                        radius_km: 0.2,
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
                        radius_km: 0.15,
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
            },
            morrow(),
        ],
        belt: BeltBlueprint {
            inner_radius_km: 1.15 * KM_PER_AU,
            outer_radius_km: 1.45 * KM_PER_AU,
            salvage_sites: vec![
                BodyBlueprint {
                    name: "Wreck Field",
                    radius_km: 4.0,
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
                    radius_km: 0.5,
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

fn morrow() -> BodyBlueprint {
    let moons = (0..11)
        .map(|i| {
            let name = MORROW_MOON_NAMES[i];
            BodyBlueprint {
                name,
                radius_km: 120.0 + i as f64 * 45.0,
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
        radius_km: 28000.0,
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

pub struct SystemDataPlugin;

impl Plugin for SystemDataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(hand_made_system_resource());
    }
}

/// `SystemData` isn't `Resource` itself (it owns borrow-unfriendly data used
/// only at spawn time); wrap it so `ScenePlugin` can consume it once.
#[derive(Resource)]
pub struct SystemDataResource(pub Option<SystemData>);

fn hand_made_system_resource() -> SystemDataResource {
    SystemDataResource(Some(hand_made_system()))
}

pub fn period_days_for(radius_km: f64, central_mass_kg: f64) -> f64 {
    kepler_period_days(radius_km, central_mass_kg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Halden IV's mass, which its satellites' periods are derived from.
    const HALDEN_IV_MASS_KG: f64 = 1.1 * EARTH_MASS_KG;

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
        period_days_for(child.orbit_radius_km, parent.mass_kg)
    }

    /// SCL-04: Kepler's third law - the period squared goes as the radius
    /// cubed, so quadrupling the radius multiplies the period by eight.
    #[test]
    fn scl_04_period_follows_keplers_third_law() {
        let base = period_days_for(10_000.0, HALDEN_IV_MASS_KG);
        let quadrupled = period_days_for(40_000.0, HALDEN_IV_MASS_KG);
        assert!(
            (quadrupled / base - 8.0).abs() < 1e-6,
            "expected 8x, got {}",
            quadrupled / base
        );
    }

    /// SCL-04: and against the closed form, so a wrong constant would show.
    #[test]
    fn scl_04_period_matches_the_closed_form() {
        let radius_km: f64 = 384_400.0;
        let mass_kg: f64 = 5.972e24;
        let radius_m = radius_km * 1000.0;
        let expected = std::f64::consts::TAU
            * (radius_m.powi(3) / (GRAVITATIONAL_CONSTANT * mass_kg)).sqrt()
            / 86_400.0;
        let actual = period_days_for(radius_km, mass_kg);
        assert!((actual - expected).abs() < 1e-9);
    }

    /// SCL-04: a heavier parent pulls its satellites round faster.
    #[test]
    fn scl_04_heavier_parent_means_shorter_period() {
        let light = period_days_for(50_000.0, EARTH_MASS_KG);
        let heavy = period_days_for(50_000.0, 4.0 * EARTH_MASS_KG);
        assert!(heavy < light);
        assert!((light / heavy - 2.0).abs() < 1e-6, "period goes as 1/sqrt(M)");
    }

    /// TEST-02: the satellite periods the motion rule is tuned against, so
    /// both modes are visible at 1 d/s.
    #[test]
    fn test_02_halden_iv_satellite_periods() {
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
    fn test_02_periods_straddle_both_motion_modes_at_one_day_per_second() {
        let halden = planet("Halden IV");
        assert!(satellite_period_days(&halden, "Arden Highport") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVa") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVb") < 20.0);
        assert!(satellite_period_days(&halden, "Halden IVc") >= 25.0);
        assert!(satellite_period_days(&halden, "Halden IVd") >= 25.0);
    }

    /// TEST-01 / TEST-03: the bodies the other checks lean on are present.
    #[test]
    fn test_01_and_03_corvane_contains_the_required_bodies() {
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
        assert!(parked.is_some(), "an own ship should be parked at Halden IV");
    }
}
