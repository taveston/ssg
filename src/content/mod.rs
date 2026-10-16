//! Content: the data that seeds the simulation (ARCH-01). Today that is
//! one of two hand-made systems, Corvane or Sol, chosen at start-up; a
//! generator replaces them later (SCL-03).
//!
//! Content depends on simulation *types* only (ARCH-02): it fills in a
//! `SystemData` and hands it over, and `sim/spawn.rs` does the rest.

use bevy::prelude::*;

use crate::sim::system::{SystemData, SystemDataResource};

pub mod corvane;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "nothing calls the sector generator until the sector view (SECT-01)"
    )
)]
pub mod generator;
pub mod sol;

/// The argument that names the system to load (SOL-22).
const SYSTEM_ARG: &str = "--system";

/// Builds one system's records.
pub type SystemBuilder = fn() -> SystemData;

/// SOL-22: the systems that can be loaded, by name. The first is the
/// default.
const KNOWN_SYSTEMS: [(&str, SystemBuilder); 2] = [
    ("corvane", corvane::hand_made_system),
    ("sol", sol::sol_system),
];

/// SOL-22: the system the program's arguments ask for: the name after
/// `--system`, or the first known system with no such argument. Other
/// arguments are ignored. A missing or unknown name is an error whose
/// message lists the known names.
pub fn chosen_system<S: AsRef<str>>(args: &[S]) -> Result<SystemBuilder, String> {
    let Some(at) = args.iter().position(|arg| arg.as_ref() == SYSTEM_ARG) else {
        return Ok(KNOWN_SYSTEMS[0].1);
    };
    let known = KNOWN_SYSTEMS.map(|(name, _)| name).join(", ");
    let Some(name) = args.get(at + 1).map(AsRef::as_ref) else {
        return Err(format!("{SYSTEM_ARG} needs a system name: one of {known}"));
    };
    KNOWN_SYSTEMS
        .iter()
        .find(|(known_name, _)| *known_name == name)
        .map(|&(_, system)| system)
        .ok_or_else(|| format!("unknown system {name:?}: the known systems are {known}"))
}

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    /// SOL-22 / SOL-23: reads the program's arguments and queues the
    /// system they name; an unknown name stops the program.
    fn build(&self, app: &mut App) {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let system = chosen_system(&args).unwrap_or_else(|message| {
            eprintln!("{message}");
            std::process::exit(2);
        });
        app.insert_resource(SystemDataResource(Some(system())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star_of(args: &[&str]) -> Result<&'static str, String> {
        chosen_system(args).map(|system| system().star_name)
    }

    /// SOL-22: with no `--system` argument the game starts on Corvane,
    /// whatever else is passed.
    #[test]
    fn corvane_is_loaded_without_a_system_argument() {
        assert_eq!(star_of(&[]), Ok("Corvane"));
        assert_eq!(star_of(&["--other", "sol"]), Ok("Corvane"));
    }

    /// SOL-22: `--system` picks a system by name.
    #[test]
    fn the_system_argument_picks_the_system() {
        assert_eq!(star_of(&["--system", "sol"]), Ok("Sol"));
        assert_eq!(star_of(&["--system", "corvane"]), Ok("Corvane"));
    }

    /// SOL-22: an unknown or missing name is an error listing the known
    /// names.
    #[test]
    fn an_unknown_system_is_an_error_listing_the_known_names() {
        for args in [&["--system", "vega"][..], &["--system"][..]] {
            let message = star_of(args).unwrap_err();
            assert!(message.contains("corvane, sol"), "{message}");
        }
        assert!(star_of(&["--system", "vega"]).unwrap_err().contains("vega"));
    }
}
