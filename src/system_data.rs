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
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ownership {
    Own,
    // No foreign-owned objects in this iteration's Corvane data yet.
    #[allow(dead_code)]
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
            },
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
                        orbit_radius_km: 18_000.0,
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
                        orbit_radius_km: 32_000.0,
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
                        orbit_radius_km: 52_000.0,
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
                        orbit_radius_km: 78_000.0,
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
                        orbit_radius_km: 12_000.0,
                        phase0_rad: 2.4,
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
