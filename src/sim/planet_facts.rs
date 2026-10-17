//! The physical facts of a planet, moon or belt body: its kind, surface
//! gravity, equilibrium temperature, atmosphere, water, tidal locking and
//! geological activity (SYSG-22). They
//! are data a rule or a view can read; the inset lists them (INS-08).
//! Hand-made bodies carry none; generated ones always do.

use bevy::prelude::*;

/// What kind of world a body is (SYSG-22): a planet's kind, or a moon.
/// A large belt body is a small rocky world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldKind {
    Rocky,
    SuperEarth,
    SubNeptune,
    IceGiant,
    GasGiant,
    Moon,
}

/// A world's atmosphere class: none, thin, dense, or the deep hydrogen
/// and helium envelope of a sub-Neptune or a giant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Atmosphere {
    Airless,
    Thin,
    Dense,
    Envelope,
}

/// A world's water: none, ice, or liquid water on the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Water {
    None,
    Ice,
    Surface,
}

/// How geologically active a solid world is, from its interior heat flux
/// (SGEN-45): dead, active, or extreme (a world like Io).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Dead,
    Active,
    Extreme,
}

/// A planet's, moon's or belt body's physical facts (SYSG-22).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlanetFacts {
    pub kind: WorldKind,
    /// Surface gravity, in units of standard gravity (9.80665 m/s²).
    pub gravity_g: f64,
    /// The temperature of a black body at the world's distance from its
    /// star, in K. It ignores any greenhouse effect, so it is not the
    /// surface temperature.
    pub equilibrium_temperature_k: f64,
    pub atmosphere: Atmosphere,
    pub water: Water,
    /// Whether tides have locked the world's spin to its orbit, so it
    /// keeps one face to its parent: the star for a planet or belt body,
    /// the planet for a moon (SGEN-44).
    pub locked: bool,
    /// Its geological activity; `None` for a world with an envelope,
    /// which has no solid surface to show it (SGEN-45).
    pub activity: Option<Activity>,
}
