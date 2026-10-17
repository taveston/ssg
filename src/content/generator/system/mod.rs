//! The system generator: the bodies of one gated system, built on demand
//! from the sector's record of it (`docs/spec/system-generator.md`). It
//! returns the same records as the hand-made Corvane and Sol (SCL-03), so
//! the simulation and the view take a generated system as they take those.
//!
//! Detail refines the sector's summary and never re-rolls it (SYSG-03):
//! the star, the planets and the belts come through as they are, and
//! detail adds starting phases, moons, a few large bodies in each belt,
//! names, key facts and descriptions, and the gates and wrecks on the
//! gate ring. Everything is drawn from seeds derived from the sector seed
//! and the system's id, one stream per body or belt, so a system comes out
//! the same however often and in whatever order systems are generated
//! (SYSG-02, SYSG-21).
//!
//! - `star`: the star's key facts (SYSG-10).
//! - `worlds`: planet records, their facts and descriptions (SYSG-20 to
//!   SYSG-23).
//! - `moons`: each planet's moons (SYSG-30 to SYSG-35).
//! - `belt_bodies`: the belts and their large bodies (SYSG-40, SYSG-41).
//! - `ring`: the gates and wrecks of the gate ring (SYSG-50 to SYSG-54).

pub mod belt_bodies;
pub mod moons;
pub mod ring;
pub mod star;
pub mod worlds;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_invariants;

use std::fmt;

use super::Sector;
use super::bodies::Summary;
use super::catalogue::Star;
use super::network::GatedSystem;
use super::rng::{Rng, derive_indexed_seed, derive_seed};
use super::tunables::GeneratorTunables;
use crate::sim::body::Knowledge;
use crate::sim::system::SystemData;
use crate::sim::units::{SOLAR_MASS_KG, SOLAR_RADIUS_KM};

/// SYSG-01: the error for an id that is not a gated system's. Ungated
/// systems, brown dwarfs and giants have no detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotGated {
    pub id: usize,
}

impl fmt::Display for NotGated {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "entry {} is not a gated system, so it has no detail to generate",
            self.id
        )
    }
}

impl std::error::Error for NotGated {}

/// What one system's detail is drawn from: the sector's records of it and
/// its own seed.
pub struct Detail<'a> {
    pub star: &'a Star,
    pub summary: &'a Summary,
    pub gated: &'a GatedSystem,
    /// The system's age in Gyr (SGEN-23), for its bodies' tides
    /// (SGEN-44, SGEN-45).
    pub age_gyr: f64,
    /// The system's seed (SYSG-02), from which every body's own seed is
    /// derived.
    pub seed: u64,
}

impl Detail<'_> {
    /// The seed of the `index`th item of a kind (`"planet"`, `"belt"`)
    /// in its list: one stream per item, so adding or removing an item
    /// doesn't change the numbers any other draws (SYSG-21).
    pub fn item_seed(&self, kind: &str, index: usize) -> u64 {
        derive_indexed_seed(self.seed, kind, index as u64)
    }
}

/// SYSG-02: a system's seed, from the sector seed, the word "system" and
/// the system's id.
pub fn system_seed(sector_seed: u64, id: usize) -> u64 {
    derive_indexed_seed(sector_seed, "system", id as u64)
}

/// SYSG-21: a natural body's starting phase, in radians, drawn uniformly
/// round its orbit from a seed of its own.
pub fn starting_phase(seed: u64) -> f64 {
    Rng::new(derive_seed(seed, "phase")).uniform(0.0, std::f64::consts::TAU)
}

/// SYSG-01 to SYSG-04: the records of the gated system with entry id
/// `id`: its star, its planets with their moons, its belts and their
/// large bodies, and the gates and wrecks of its ring. Every natural body
/// is neutral and surveyed; there are no ships, stations or other sites.
/// The result depends only on the sector, the id and the tunables. An id
/// that is not a gated system's is an error, not a panic.
pub fn generate_system(
    sector: &Sector,
    id: usize,
    t: &GeneratorTunables,
) -> Result<SystemData, NotGated> {
    let gated = sector.gated_system(id).ok_or(NotGated { id })?;
    let entry = &sector.entries[id];
    let (Some(star), Some(summary)) = (entry.primary(), sector.summary(id)) else {
        unreachable!("a gated system has a primary star and a summary");
    };
    let detail = Detail {
        star,
        summary,
        gated,
        age_gyr: entry.age_gyr,
        seed: system_seed(sector.settings.seed, id),
    };
    let mut records = worlds::planets(&detail, t);
    records.extend(belt_bodies::bodies(&detail, t));
    let ring = ring::ring_records(sector, &detail, t);
    records.extend(ring.wrecks);
    Ok(SystemData {
        star_name: gated.names.system.clone(),
        star_key_facts: star::key_facts(star, entry.age_gyr, entry.metallicity),
        star_mass_kg: star.mass_msun * SOLAR_MASS_KG,
        star_radius_km: star.radius_rsun * SOLAR_RADIUS_KM,
        star_knowledge: Knowledge::Surveyed,
        planets: records,
        belts: belt_bodies::belts(&detail),
        gates: ring.gates,
    })
}
