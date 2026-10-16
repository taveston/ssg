//! Every number the transfer spec marks *tunable*, in one resource
//! (ARCH-22), separate from the view's `Tunables`.
//!
//! The tunables table in `docs/spec/transfers.md` lists them with the spec
//! IDs that use them. They live here, documented field by field, so a
//! band altitude or a loss distance is changed in one place.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct SimTunables {
    /// A body's Low band sits this far above its mean radius (BAND-01).
    pub low_band_altitude_km: f64,
    /// A body's High band sits at this many mean radii, unless its Hill
    /// sphere caps it lower (BAND-01).
    pub high_band_radii: f64,
    /// The High band never exceeds this fraction of the body's Hill radius
    /// (BAND-01).
    pub high_band_hill_fraction: f64,
    /// A coasting ship whose orbit reaches past this many AU from the star
    /// is escaping, and is removed once it is that far out (BAND-08).
    pub lost_beyond_au: f64,
    /// A crossing whose path dips inside this many mean radii of its
    /// primary is infeasible (XFER-07).
    pub min_periapsis_radii: f64,
    /// A conic's eccentricity is kept at least this far from 1, so there
    /// is never a parabolic path (RAIL-05).
    pub parabola_epsilon: f64,
    /// Two things in one named orbit closer than this, in km, are
    /// together: a ship this close to a station is docked at it (PHAS-02).
    pub together_km: f64,
    /// A phasing leg tries meetings up to this many passes of the burn
    /// point before it gives up (PHAS-08).
    pub max_phasing_laps: u32,
    /// An Intercept trimmed to meet a station or ship is solved again at
    /// most this many times before it counts as unable to line up
    /// (PHAS-15).
    pub trim_max_iterations: u32,
}

impl Default for SimTunables {
    fn default() -> Self {
        Self {
            low_band_altitude_km: 500.0,
            high_band_radii: 10.0,
            high_band_hill_fraction: 0.5,
            lost_beyond_au: 6.0,
            min_periapsis_radii: 2.0,
            parabola_epsilon: 1e-6,
            together_km: 10.0,
            max_phasing_laps: 50,
            trim_max_iterations: 8,
        }
    }
}
