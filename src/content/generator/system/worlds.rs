//! Planets as records (SYSG-20 to SYSG-23): each planet of the summary
//! round the star with its summary orbit, mass, radius and facts, the name
//! the sector gave it, a starting phase of its own and its moons. Also the
//! pieces every natural body shares: its physical facts as simulation
//! data, its one-line description and its record.

use super::{Detail, moons, starting_phase};
use crate::content::generator::bodies::{self, PlanetKind, PlanetSummary};
use crate::content::generator::summaries::SolidFacts;
use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::body::{Affiliation, BodyClass, Knowledge};
use crate::sim::planet_facts::{Atmosphere, PlanetFacts, Water, WorldKind};
use crate::sim::system::{BodyBlueprint, OrbitPlacement};
use crate::sim::units::{EARTH_MASS_KG, EARTH_RADIUS_KM, KM_PER_AU, M_PER_KM};

/// Kilograms per cubic metre in a gram per cubic centimetre.
const KG_PER_M3_PER_G_PER_CM3: f64 = 1000.0;

/// SYSG-20: every planet of the summary as a record round the star,
/// innermost first, each with its moons.
pub fn planets(detail: &Detail, t: &GeneratorTunables) -> Vec<BodyBlueprint> {
    detail
        .summary
        .planets
        .iter()
        .zip(&detail.gated.names.planets)
        .enumerate()
        .map(|(index, (planet, name))| {
            let seed = detail.item_seed("planet", index);
            let moons = moons::moons(planet, name, seed, detail, t);
            let mut record = natural_body(
                name.clone(),
                BodyClass::Planet,
                planet_facts(planet),
                planet.orbit_au * KM_PER_AU,
                starting_phase(seed),
            );
            record.radius_km = planet.radius_earth * EARTH_RADIUS_KM;
            record.mass_kg = planet.mass_earth * EARTH_MASS_KG;
            record.children = moons;
            record
        })
        .collect()
}

/// SYSG-22: a planet's kind as a world's.
pub fn world_kind(kind: PlanetKind) -> WorldKind {
    match kind {
        PlanetKind::Rocky => WorldKind::Rocky,
        PlanetKind::SuperEarth => WorldKind::SuperEarth,
        PlanetKind::SubNeptune => WorldKind::SubNeptune,
        PlanetKind::IceGiant => WorldKind::IceGiant,
        PlanetKind::GasGiant => WorldKind::GasGiant,
    }
}

/// The summary's atmosphere class as the simulation's.
pub fn atmosphere(atmosphere: bodies::Atmosphere) -> Atmosphere {
    match atmosphere {
        bodies::Atmosphere::Airless => Atmosphere::Airless,
        bodies::Atmosphere::Thin => Atmosphere::Thin,
        bodies::Atmosphere::Dense => Atmosphere::Dense,
        bodies::Atmosphere::Envelope => Atmosphere::Envelope,
    }
}

/// The summary's water as the simulation's.
pub fn water(water: bodies::Water) -> Water {
    match water {
        bodies::Water::None => Water::None,
        bodies::Water::Ice => Water::Ice,
        bodies::Water::Surface => Water::Surface,
    }
}

/// SYSG-03 / SYSG-22: a planet's facts exactly as its summary has them.
pub fn planet_facts(planet: &PlanetSummary) -> PlanetFacts {
    PlanetFacts {
        kind: world_kind(planet.kind),
        gravity_g: planet.gravity_g,
        equilibrium_temperature_k: planet.equilibrium_temperature_k,
        atmosphere: atmosphere(planet.atmosphere),
        water: water(planet.water),
    }
}

/// SYSG-22: a moon's or belt body's facts, of its kind, as worked out by
/// the rules for a rocky planet.
pub fn solid_facts(kind: WorldKind, facts: SolidFacts) -> PlanetFacts {
    PlanetFacts {
        kind,
        gravity_g: facts.gravity_g,
        equilibrium_temperature_k: facts.equilibrium_temperature_k,
        atmosphere: atmosphere(facts.atmosphere),
        water: water(facts.water),
    }
}

/// The radius in km of a ball of `mass_kg` at `density_g_cm3`:
/// R = (3 m ÷ 4 π ρ)^(1/3) (SYSG-32, SYSG-41).
pub fn radius_km_for_density(mass_kg: f64, density_g_cm3: f64) -> f64 {
    let density = density_g_cm3 * KG_PER_M3_PER_G_PER_CM3;
    (3.0 * mass_kg / (4.0 * std::f64::consts::PI * density)).cbrt() / M_PER_KM
}

/// SYSG-23: a natural body's one-line description from its facts: its
/// kind, atmosphere and water in plain words. Placeholder wording.
pub fn description(facts: &PlanetFacts) -> String {
    let kind = match facts.kind {
        WorldKind::Rocky => "A rocky planet",
        WorldKind::SuperEarth => "A super-Earth",
        WorldKind::SubNeptune => "A sub-Neptune",
        WorldKind::IceGiant => "An ice giant",
        WorldKind::GasGiant => "A gas giant",
        WorldKind::Moon => "A moon",
    };
    let air = match facts.atmosphere {
        Atmosphere::Airless => "no air",
        Atmosphere::Thin => "a thin atmosphere",
        Atmosphere::Dense => "a dense atmosphere",
        Atmosphere::Envelope => return format!("{kind} wrapped in hydrogen and helium."),
    };
    let water = match facts.water {
        Water::None => "no water",
        Water::Ice => "water ice",
        Water::Surface => "water on its surface",
    };
    format!("{kind} with {air} and {water}.")
}

/// SYSG-04 / SYSG-20 to SYSG-23: a natural body with an orbit of its own,
/// neutral and surveyed, with its facts and the description built from
/// them. Its size, mass and children are the caller's to set.
pub fn natural_body(
    name: String,
    class: BodyClass,
    facts: PlanetFacts,
    orbit_radius_km: f64,
    phase0_rad: f64,
) -> BodyBlueprint {
    BodyBlueprint {
        name,
        class,
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        group: None,
        description: description(&facts),
        key_facts: vec![],
        planet_facts: Some(facts),
        knowledge: Knowledge::Surveyed,
        orbit: OrbitPlacement::Own {
            radius_km: orbit_radius_km,
        },
        radius_km: 0.0,
        phase0_rad,
        mass_kg: 0.0,
        orbits: vec![],
        children: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(kind: WorldKind, atmosphere: Atmosphere, water: Water) -> PlanetFacts {
        PlanetFacts {
            kind,
            gravity_g: 1.0,
            equilibrium_temperature_k: 250.0,
            atmosphere,
            water,
        }
    }

    /// SYSG-23: the description says the kind, atmosphere and water in
    /// plain words.
    #[test]
    fn a_description_says_kind_atmosphere_and_water() {
        assert_eq!(
            description(&facts(WorldKind::Rocky, Atmosphere::Airless, Water::Ice)),
            "A rocky planet with no air and water ice."
        );
        assert_eq!(
            description(&facts(
                WorldKind::SuperEarth,
                Atmosphere::Dense,
                Water::Surface
            )),
            "A super-Earth with a dense atmosphere and water on its surface."
        );
        assert_eq!(
            description(&facts(WorldKind::Moon, Atmosphere::Thin, Water::None)),
            "A moon with a thin atmosphere and no water."
        );
        assert_eq!(
            description(&facts(
                WorldKind::IceGiant,
                Atmosphere::Envelope,
                Water::None
            )),
            "An ice giant wrapped in hydrogen and helium."
        );
    }

    /// SYSG-32: a ball of the Moon's mass at its density has about its
    /// radius.
    #[test]
    fn radius_follows_from_mass_and_density() {
        let radius = radius_km_for_density(7.342e22, 3.344);
        assert!((radius - 1737.0).abs() < 5.0, "{radius}");
    }
}
