//! What a body *is*, as far as the simulation knows: its class, its
//! affiliation, what is known about it, and the authored text that goes with it
//! (SCL-03). These are plain components with no presentation in them; the
//! view decides how each one is drawn.

use bevy::prelude::*;

/// What kind of object a body is. The view picks its marker from this
/// (MRK-04); grouping and inspection treat classes the same way.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyClass {
    Star,
    Planet,
    Moon,
    Station,
    Gate,
    SalvageSite,
    /// A ship or contact. Whose it is is its `Affiliation`, and what kind
    /// of ship its `ShipRecord` (SYM-01), not class.
    Ship,
}

pub use super::track::Affiliation;

/// How well a body is known (INS-04).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Knowledge {
    Surveyed,
    Unsurveyed,
}

#[derive(Component, Clone, Debug, Default)]
pub struct DisplayName(pub String);

/// The system a gate leads to, by name (EDGE-01).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct GateDestination(pub String);

/// Whether a gate can be passed (SYSG-52): working, or dormant (intact
/// but closed). A dormant gate says whether this end of its link is the
/// faulted one; why it is closed is not modelled yet.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateState {
    Working,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only the system generator makes dormant gates, which entering a system calls (SECT-31)"
        )
    )]
    Dormant {
        faulted_here: bool,
    },
}

/// PLAN-30: how closely a ship must match a gate's speed to enter it, in
/// km/s. Stored on the gate; no rule reads it yet.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct GateEntryTolerance(pub f64);

/// The authored one-line description of a body, as the player's records
/// have it.
#[derive(Component, Clone, Debug, Default)]
pub struct Description(pub String);

/// The authored key facts about a body, as label and value pairs.
#[derive(Component, Clone, Debug, Default)]
pub struct KeyFacts(pub Vec<(String, String)>);

/// A body's mean physical radius in km (SCL-05). Presentation uses it for
/// true-size discs (MRK-07) and the zoom's inner limit (ZOOM-14); it never
/// sets a marker's size.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PhysicalRadius(pub f64);

/// A body's mass in kg, for the gravity of everything round it: its
/// gravitational parameter, its Hill radius and its bands (BAND-01). Zero
/// for a body too light to matter.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Mass(pub f64);

/// A star's mass in kg. Its inset reports it (INS-08); orbits round it
/// are already worked out from it at spawn.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StellarMass(pub f64);

/// One of the system's belts, an annulus in the orbital plane round the
/// star (SCL-08). A system holds any number of them, one entity each. The
/// mass and composition are data only: nothing reads them yet.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Belt {
    pub name: String,
    pub inner_radius_km: f64,
    pub outer_radius_km: f64,
    /// The belt's total mass in kg, where it is known.
    pub mass_kg: Option<f64>,
    /// What the belt is made of, where it is known.
    pub composition: Option<BeltComposition>,
}

/// A belt's make-up as rocky, metallic and icy fractions of its mass,
/// which sum to 1 (SCL-08).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeltComposition {
    pub rocky: f64,
    pub metallic: f64,
    pub icy: f64,
}
