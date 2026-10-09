//! The hand-made star system for iteration 1, in real units (AU, km, days).
//! This is the seam a future system generator replaces: anything that builds
//! a `SystemData` the same shape can drop in here.

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct SystemData {
    pub star: StarData,
    pub planets: Vec<PlanetData>,
    pub gates: Vec<GateData>,
}

#[derive(Clone)]
pub struct StarData {
    pub name: &'static str,
    pub radius_km: f64,
    pub color: Color,
}

#[derive(Clone)]
pub struct PlanetData {
    pub name: &'static str,
    pub radius_km: f64,
    pub orbit_radius_au: f64,
    pub period_days: f64,
    pub color: Color,
    pub moon: Option<MoonData>,
}

#[derive(Clone)]
pub struct MoonData {
    pub name: &'static str,
    pub radius_km: f64,
    pub orbit_radius_km: f64,
    pub period_days: f64,
    pub color: Color,
}

#[derive(Clone)]
pub struct GateData {
    pub name: &'static str,
    pub distance_au: f64,
    pub angle_deg: f64,
}

/// Kepler's third law for a solar-mass-like star: period in days from orbit
/// radius in AU. Close enough for flavour; not a physics requirement here.
fn period_days_from_au(orbit_radius_au: f64) -> f64 {
    365.25 * orbit_radius_au.powf(1.5)
}

/// Placeholder content for the "Veska" system. Names and numbers are
/// hand-picked flavour, not lore-locked - easy to replace wholesale.
pub fn hand_made_system() -> SystemData {
    SystemData {
        star: StarData {
            name: "Veska",
            radius_km: 660_000.0,
            color: Color::srgb(1.0, 0.95, 0.75),
        },
        planets: vec![
            PlanetData {
                name: "Ashkar",
                radius_km: 2_900.0,
                orbit_radius_au: 0.4,
                period_days: period_days_from_au(0.4),
                color: Color::srgb(0.55, 0.5, 0.45),
                moon: None,
            },
            PlanetData {
                name: "Bellweather",
                radius_km: 6_050.0,
                orbit_radius_au: 0.8,
                period_days: period_days_from_au(0.8),
                color: Color::srgb(0.8, 0.7, 0.5),
                moon: None,
            },
            PlanetData {
                name: "Caldera",
                radius_km: 6_700.0,
                orbit_radius_au: 1.3,
                period_days: period_days_from_au(1.3),
                color: Color::srgb(0.3, 0.5, 0.65),
                moon: Some(MoonData {
                    name: "Caldera-I",
                    radius_km: 1_300.0,
                    orbit_radius_km: 30_000.0,
                    period_days: 4.2,
                    color: Color::srgb(0.6, 0.6, 0.6),
                }),
            },
            PlanetData {
                name: "Dreyman",
                radius_km: 28_000.0,
                orbit_radius_au: 2.5,
                period_days: period_days_from_au(2.5),
                color: Color::srgb(0.75, 0.55, 0.35),
                moon: None,
            },
        ],
        gates: vec![
            GateData {
                name: "Gate Thresh",
                distance_au: 3.6,
                angle_deg: 30.0,
            },
            GateData {
                name: "Gate Ophir",
                distance_au: 3.8,
                angle_deg: 150.0,
            },
            GateData {
                name: "Gate Kess",
                distance_au: 3.7,
                angle_deg: 260.0,
            },
        ],
    }
}

pub struct SystemDataPlugin;

impl Plugin for SystemDataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(hand_made_system());
    }
}
