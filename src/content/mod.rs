//! Content: the data that seeds the simulation (ARCH-01). Today that is
//! the hand-made Corvane system; a generator replaces it later (SCL-03).
//!
//! Content depends on simulation *types* only (ARCH-02): it fills in a
//! `SystemData` and hands it over, and `sim/spawn.rs` does the rest.

use bevy::prelude::*;

use crate::sim::system::SystemDataResource;

pub mod corvane;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SystemDataResource(Some(corvane::hand_made_system())));
    }
}
