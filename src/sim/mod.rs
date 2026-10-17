//! Simulation (ARCH-01): the game state and its rules, in real units -
//! the clock, bodies and their orbits, named orbits, affiliation,
//! knowledge, and ships' drives, plans and rails (`transfer/`).
//!
//! Nothing here knows about screens, cameras or input (ARCH-02), so the
//! whole domain runs in a headless `App` with `MinimalPlugins` and
//! `SimPlugin` alone (ARCH-13). Implements SCL-01, SCL-03, SCL-04, CLK-01
//! and CLK-04.

use bevy::prelude::*;

use load::SystemLoadApp;

use crate::{Boot, Stage};

pub mod body;
pub mod clock;
pub mod group;
pub mod load;
pub mod named_orbit;
pub mod orbit;
pub mod planet_facts;
pub mod rail;
pub mod ship;
pub mod spawn;
pub mod system;
pub mod track;
pub mod transfer;
pub mod tunables;
pub mod units;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<clock::GameClock>()
            .init_resource::<tunables::SimTunables>()
            .add_message::<transfer::plan::ShipOrder>()
            .add_message::<transfer::plan::OrderOutcome>()
            .add_load_systems(Boot::World, spawn::spawn_system)
            .add_systems(
                Update,
                (
                    clock::tick_clock,
                    transfer::plan::take_orders,
                    transfer::plan::run_plans,
                    orbit::update_orbit_positions,
                    transfer::plan::drive_rails,
                    transfer::plan::remove_escaping_ships,
                    group::derive_group_records,
                )
                    .chain()
                    .in_set(Stage::Simulation),
            );
    }
}
