//! The shape of a star system as data (SCL-03): a tree of body records in
//! real units, which `content/` fills by hand today and a generator will
//! fill later. `spawn.rs` turns one into entities; nothing downstream knows
//! where the records came from.

use bevy::prelude::*;

use super::body::{Affiliation, BodyClass, Knowledge};
use super::track::ShipRecord;

/// A node in the system tree. Periods are derived from `orbit_radius_km`
/// and the parent's mass, not stored, so they can't drift out of sync with
/// the radius (SCL-04).
pub struct BodyBlueprint {
    pub name: &'static str,
    pub class: BodyClass,
    pub affiliation: Affiliation,
    /// What the state knows of a ship or station (SYM-01); `None` for
    /// natural bodies.
    pub record: Option<ShipRecord>,
    pub description: &'static str,
    pub key_facts: Vec<(&'static str, &'static str)>,
    pub knowledge: Knowledge,
    pub orbit_radius_km: f64,
    /// Mean physical radius (SCL-05).
    pub radius_km: f64,
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
    pub radius_km: f64,
    pub distance_km: f64,
    pub angle_deg: f64,
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
