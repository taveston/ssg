//! Content: the data that seeds the simulation (ARCH-01): one of two
//! hand-made systems, Corvane or Sol, or a generated sector, chosen at
//! start-up from the program's arguments (`start.rs`, SOL-22, SECT-40).
//!
//! Content depends on simulation *types* only (ARCH-02): it fills in a
//! `SystemData` and hands it over, and `sim/spawn.rs` does the rest. A
//! sector is plain data the view reads (`start::LoadedSector`).

use bevy::prelude::*;

use generator::tunables::GeneratorTunables;

pub mod corvane;
pub mod figures;
pub mod generator;
pub mod sol;
pub mod start;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    /// SOL-22 / SOL-23 / SECT-43: reads the program's arguments and queues
    /// the system or sector they name; arguments it can't use stop the
    /// program with a message.
    fn build(&self, app: &mut App) {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let started = start::chosen_start(&args, &GeneratorTunables::default());
        let chosen = started.unwrap_or_else(|message| {
            eprintln!("{message}");
            std::process::exit(2);
        });
        start::load_start(app, chosen);
    }
}
