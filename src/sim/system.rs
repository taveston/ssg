//! The shape of a star system as data (SCL-03): a tree of body records in
//! real units, which `content/` fills by hand today and a generator will
//! fill later. `spawn.rs` turns one into entities; nothing downstream knows
//! where the records came from.

use bevy::prelude::*;

use super::body::{BodyClass, Knowledge, Ownership};

/// A node in the system tree. Periods are derived from `orbit_radius_km`
/// and the parent's mass, not stored, so they can't drift out of sync with
/// the radius (SCL-04).
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
    /// The star's spectral class, shown as its key fact.
    pub star_class: &'static str,
    pub star_mass_kg: f64,
    pub star_knowledge: Knowledge,
    pub planets: Vec<BodyBlueprint>,
    pub belt: BeltBlueprint,
    pub gates: Vec<GateBlueprint>,
}

/// The system waiting to be spawned. `SystemData` owns borrow-unfriendly
/// data used only once, so it sits in an `Option` that `spawn.rs` takes.
#[derive(Resource)]
pub struct SystemDataResource(pub Option<SystemData>);
