//! The shape of a star system as data (SCL-03): a tree of body records in
//! real units, which `content/` fills by hand today and a generator will
//! fill later. `spawn.rs` turns one into entities; nothing downstream knows
//! where the records came from.

use bevy::prelude::*;

use super::body::{Affiliation, BodyClass, Knowledge};
use super::ship::Propulsion;
use super::track::ShipRecord;

/// SCL-07: where a record sits round what it orbits. A natural body has an
/// orbit of its own; a ship or station references a named orbit and
/// carries no radius. A station never has an orbit of its own. A ship
/// docked at a station references the station's band, at the station's
/// phase (BAND-02).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrbitPlacement {
    /// An orbit of its own at this radius: a natural body, or anything
    /// round the star.
    Own { radius_km: f64 },
    /// One of its parent's named orbits, by name.
    Named(&'static str),
}

/// A named orbit a body offers (SCL-07) besides its bands, which are
/// derived (BAND-01): a name and a radius; its period follows from the
/// body's mass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedOrbitBlueprint {
    pub name: &'static str,
    pub radius_km: f64,
}

/// A node in the system tree. Periods are derived from the orbit radius
/// and the parent's mass, not stored, so they can't drift out of sync with
/// the radius (SCL-04).
pub struct BodyBlueprint {
    pub name: &'static str,
    pub class: BodyClass,
    pub affiliation: Affiliation,
    /// What the faction knows of a ship or station (SYM-01); `None` for
    /// natural bodies.
    pub record: Option<ShipRecord>,
    /// A ship's drive and delta-v (PLAN-20); `None` for anything that
    /// can't be ordered to move.
    pub propulsion: Option<Propulsion>,
    pub description: &'static str,
    pub key_facts: Vec<(&'static str, &'static str)>,
    pub knowledge: Knowledge,
    /// Where it sits round its parent (SCL-07).
    pub orbit: OrbitPlacement,
    /// Mean physical radius (SCL-05).
    pub radius_km: f64,
    pub phase0_rad: f64,
    /// This body's own mass, used to compute its children's periods. Leave
    /// at 0.0 for bodies with no children.
    pub mass_kg: f64,
    /// The named orbits this body offers its ships and stations (SCL-07).
    pub orbits: Vec<NamedOrbitBlueprint>,
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
    pub radius_km: f64,
    pub distance_km: f64,
    pub angle_deg: f64,
    /// PLAN-30: how close to the gate's speed a ship must match to enter;
    /// stored, not yet read by any rule.
    pub entry_tolerance_kms: f64,
}

pub struct SystemData {
    pub star_name: &'static str,
    /// The star's spectral class, shown as its key fact.
    pub star_class: &'static str,
    pub star_mass_kg: f64,
    /// The star's mean physical radius (SCL-05).
    pub star_radius_km: f64,
    pub star_knowledge: Knowledge,
    pub planets: Vec<BodyBlueprint>,
    pub belt: BeltBlueprint,
    pub gates: Vec<GateBlueprint>,
}

/// The system waiting to be spawned. `SystemData` owns borrow-unfriendly
/// data used only once, so it sits in an `Option` that `spawn.rs` takes.
#[derive(Resource)]
pub struct SystemDataResource(pub Option<SystemData>);
