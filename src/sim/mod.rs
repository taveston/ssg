//! Simulation (ARCH-01): the game state and its rules, in real units -
//! the clock, bodies and their orbits, affiliation and knowledge.
//!
//! Nothing here knows about screens, cameras or input (ARCH-02), so the
//! whole domain runs in a headless `App` with `MinimalPlugins` and
//! `SimPlugin` alone (ARCH-13). Implements SCL-01, SCL-03, SCL-04, CLK-01
//! and CLK-04.

use bevy::prelude::*;

use crate::{Boot, Stage};

pub mod body;
pub mod clock;
pub mod orbit;
pub mod spawn;
pub mod system;
pub mod track;
pub mod units;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<clock::GameClock>()
            .add_systems(Startup, spawn::spawn_system.in_set(Boot::World))
            .add_systems(
                Update,
                (clock::tick_clock, orbit::update_orbit_positions)
                    .chain()
                    .in_set(Stage::Simulation),
            );
    }
}
