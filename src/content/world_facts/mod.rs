//! The authored facts of the hand-made systems' worlds and stars (SURF-60
//! to SURF-63). Corvane and Sol are not generated, so each planet's and
//! moon's kind, atmosphere, water, locking and activity are written out in
//! a table, with a surface seed and, for Earth alone, a look override.
//! Gravity and equilibrium temperature are not authored: they follow the
//! generator's rules from each body's mass, radius, orbit and star
//! (SGEN-38, SGEN-39), so they can't drift from the figures.

use crate::content::generator::stellar::main_sequence_luminosity;
use crate::content::generator::summaries::{equilibrium_temperature_k, gravity_g};
use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::body::BodyClass;
use crate::sim::planet_facts::{Activity, Atmosphere, LookOverride, PlanetFacts, Water, WorldKind};
use crate::sim::star_facts::StarFacts;
use crate::sim::system::{BodyBlueprint, OrbitPlacement, SystemData};
use crate::sim::units::{EARTH_MASS_KG, EARTH_RADIUS_KM, KM_PER_AU, SOLAR_MASS_KG};

pub mod corvane;
pub mod sol;

/// One world's authored facts: the row of its system's table, by name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuthoredWorld {
    pub name: &'static str,
    pub kind: WorldKind,
    pub atmosphere: Atmosphere,
    pub water: Water,
    pub locked: bool,
    /// `None` for a world with an envelope, which has no solid surface.
    pub activity: Option<Activity>,
    pub surface_seed: u64,
    pub look_override: Option<LookOverride>,
}

/// A hand-made star's facts. Its luminosity is the one given, or, if none
/// is, the main-sequence luminosity for its mass (SGEN-22).
pub fn star_facts(
    spectral_type: &str,
    mass_kg: f64,
    luminosity_lsun: Option<f64>,
    age_gyr: f64,
    t: &GeneratorTunables,
) -> StarFacts {
    let luminosity_lsun = luminosity_lsun.unwrap_or_else(|| {
        main_sequence_luminosity(mass_kg / SOLAR_MASS_KG, &t.star_mass_luminosity)
    });
    StarFacts {
        spectral_type: spectral_type.to_string(),
        luminosity_lsun,
        age_gyr,
    }
}

/// Gives every planet and moon of `system` its row of `table`: planet
/// facts, with gravity and equilibrium temperature worked out by the
/// rules, a surface seed and any look override. A moon's equilibrium
/// temperature is taken at its planet's distance from the star, as a
/// generated moon's is (SYSG-33).
///
/// Panics if a world has no row: every hand-made world carries facts.
pub fn author_worlds(system: &mut SystemData, table: &[AuthoredWorld], t: &GeneratorTunables) {
    let luminosity_lsun = system.star_facts.luminosity_lsun;
    for planet in &mut system.planets {
        let distance_au = own_radius_km(planet).map_or(0.0, |km| km / KM_PER_AU);
        author(planet, distance_au, luminosity_lsun, table, t);
    }
}

/// Fills in `body` and its natural satellites, all lit from `distance_au`.
fn author(
    body: &mut BodyBlueprint,
    distance_au: f64,
    luminosity_lsun: f64,
    table: &[AuthoredWorld],
    t: &GeneratorTunables,
) {
    if !matches!(body.class, BodyClass::Planet | BodyClass::Moon) {
        return;
    }
    let row = table
        .iter()
        .find(|row| row.name == body.name)
        .unwrap_or_else(|| panic!("{} has no authored facts", body.name));
    body.planet_facts = Some(PlanetFacts {
        kind: row.kind,
        gravity_g: gravity_g(
            body.mass_kg / EARTH_MASS_KG,
            body.radius_km / EARTH_RADIUS_KM,
        ),
        equilibrium_temperature_k: equilibrium_temperature_k(
            luminosity_lsun,
            distance_au,
            t.equilibrium_albedo,
        ),
        atmosphere: row.atmosphere,
        water: row.water,
        locked: row.locked,
        activity: row.activity,
    });
    body.surface_seed = Some(row.surface_seed);
    body.look_override = row.look_override;
    for child in &mut body.children {
        author(child, distance_au, luminosity_lsun, table, t);
    }
}

/// A body's orbit radius, if it has an orbit of its own.
fn own_radius_km(body: &BodyBlueprint) -> Option<f64> {
    match body.orbit {
        OrbitPlacement::Own { radius_km } => Some(radius_km),
        OrbitPlacement::Named(_) => None,
    }
}

/// A row with the given facts and no override, for the tables.
pub const fn world(
    name: &'static str,
    kind: WorldKind,
    (atmosphere, water): (Atmosphere, Water),
    (locked, activity): (bool, Option<Activity>),
    surface_seed: u64,
) -> AuthoredWorld {
    AuthoredWorld {
        name,
        kind,
        atmosphere,
        water,
        locked,
        activity,
        surface_seed,
        look_override: None,
    }
}

#[cfg(test)]
mod tests;
