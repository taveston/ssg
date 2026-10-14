//! Named orbits (SCL-07): the orbits a body offers the ships and stations
//! round it, as simulation entities.
//!
//! A body record carries its named orbits, each a name and a radius in km.
//! A ship or station in orbit references one of them, or the orbit of a
//! station it is docked at, and carries no radius of its own: its `Orbit`
//! elements are derived from the named orbit at spawn, as a period is
//! derived from a radius (`spawn.rs`). Natural bodies have orbits of their
//! own and never share one. Several ships in one named orbit are what the
//! view draws as a shared orbit (HOV-04); nothing is grouped by radius.
//!
//! A named orbit is not a `ChildOf` its body, so nothing that walks a
//! body's children (lists, pills, fits) sees it: it is found by `parent`.

use bevy::prelude::*;

/// One of a body's named orbits, in real units.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct NamedOrbit {
    /// The body it circles.
    pub parent: Entity,
    pub name: String,
    pub radius_km: f64,
    /// From Kepler's third law and the parent's mass (SCL-04), the same
    /// for every member.
    pub period_days: f64,
}

/// On a ship or station in orbit: the named orbit it occupies (SCL-07).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InNamedOrbit(pub Entity);

/// On the named orbit a station, gate or salvage site offers for docking
/// (BAND-02): the orbit a ship sent to that place arrives in. Names the
/// place that offers it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockingOrbitOf(pub Entity);
