//! What a body *is*, as far as the simulation knows: its class, who owns
//! it, what is known about it, and the authored text that goes with it
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
    /// A ship or contact. Whether it is "ours" is `Ownership`, not class.
    Ship,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ownership {
    Own,
    Foreign,
    Neutral,
}

/// How well a body is known (INS-04). Days are simulated days since the
/// campaign epoch, negative for before it.
#[derive(Component, Clone, Debug)]
pub enum Knowledge {
    Surveyed { imaged_day: f64 },
    Unsurveyed,
    Stale { last_imaged_day: f64 },
}

#[derive(Component, Clone, Debug, Default)]
pub struct DisplayName(pub String);

/// The authored one-line description of a body, as the player's records
/// have it.
#[derive(Component, Clone, Debug, Default)]
pub struct Description(pub String);

/// The authored key facts about a body, as label and value pairs.
#[derive(Component, Clone, Debug, Default)]
pub struct KeyFacts(pub Vec<(String, String)>);

/// The system's asteroid belt, an annulus in the orbital plane round the
/// star (MRK-05).
#[derive(Component, Clone, Copy, Debug)]
pub struct Belt {
    pub inner_radius_km: f64,
    pub outer_radius_km: f64,
}
