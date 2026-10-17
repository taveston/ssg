//! The physical facts of a system's star that rules and views read as
//! numbers: its spectral type, luminosity and age (SYSG-10, SURF-60). The
//! star's key facts show them as text; these are the values behind them.

use bevy::prelude::*;

/// A star's spectral type, luminosity and age, on the star's entity.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct StarFacts {
    /// For example "G2V" or "K2V".
    pub spectral_type: String,
    /// In solar luminosities.
    pub luminosity_lsun: f64,
    /// The star's age, and so the system's, in Gyr.
    pub age_gyr: f64,
}
