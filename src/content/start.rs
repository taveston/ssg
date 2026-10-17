//! What the game starts with, read from the program's arguments by plain
//! functions (SOL-22, SECT-40 to SECT-43): a hand-made system named by
//! `--system`, or a new sector of `--sector <small|medium|large|N>` with
//! an optional `--seed <n>`. With neither, the game starts on Corvane.
//! Arguments these flags don't name are ignored, so the test runner's own
//! arguments are harmless.
//!
//! `load_start` then queues what was chosen: a system's records for
//! `sim/spawn.rs`, or a generated sector (`LoadedSector`) and no system.

use bevy::prelude::*;

use super::generator::settings::{Preset, Settings, parse_seed};
use super::generator::tunables::GeneratorTunables;
use super::generator::{Sector, generate_sector};
use super::{corvane, sol};
use crate::sim::system::{SystemData, SystemDataResource};

/// The argument that names the system to load (SOL-22).
const SYSTEM_ARG: &str = "--system";
/// The argument that asks for a new sector, and its size (SECT-40).
const SECTOR_ARG: &str = "--sector";
/// The argument that sets the sector's seed (SECT-41).
const SEED_ARG: &str = "--seed";

/// Builds one system's records.
pub type SystemBuilder = fn() -> SystemData;

/// SOL-22: the systems that can be loaded, by name. The first is the
/// default.
const KNOWN_SYSTEMS: [(&str, SystemBuilder); 2] = [
    ("corvane", corvane::hand_made_system),
    ("sol", sol::sol_system),
];

/// SECT-40: the sector presets, by the name `--sector` takes.
const PRESET_NAMES: [(&str, Preset); 3] = [
    ("small", Preset::Small),
    ("medium", Preset::Medium),
    ("large", Preset::Large),
];

/// A new sector as the arguments ask for it: its number of gated systems,
/// and its seed if one was given (SECT-40, SECT-41).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectorRequest {
    pub gated_count: u32,
    pub seed: Option<u64>,
}

impl SectorRequest {
    /// The generator's settings, with a random seed if none was given
    /// (SGEN-04).
    pub fn settings(&self) -> Settings {
        match self.seed {
            Some(seed) => Settings {
                gated_count: self.gated_count,
                seed,
            },
            None => Settings::with_random_seed(self.gated_count),
        }
    }
}

/// What the game starts with.
#[derive(Debug, Clone, Copy)]
pub enum Start {
    /// A hand-made system, in the system view (SOL-22).
    System(SystemBuilder),
    /// A new sector, in the sector view (SECT-40).
    Sector(SectorRequest),
}

/// The sector the game was started with (SECT-01, SGEN-90), and the
/// generator's tunables it was made with, so a system in it can be
/// generated from the same ones (SYSG-01).
#[derive(Resource, Debug, Clone)]
pub struct LoadedSector {
    pub sector: Sector,
    #[expect(
        dead_code,
        reason = "read when a system of the sector is entered (SECT-31)"
    )]
    pub tunables: GeneratorTunables,
}

/// The value after the argument `flag`: `None` without the flag, an error
/// naming `what` if nothing follows it.
fn value_after<'a, S: AsRef<str>>(
    args: &'a [S],
    flag: &str,
    what: &str,
) -> Result<Option<&'a str>, String> {
    let Some(at) = args.iter().position(|arg| arg.as_ref() == flag) else {
        return Ok(None);
    };
    match args.get(at + 1) {
        Some(value) => Ok(Some(value.as_ref())),
        None => Err(format!("{flag} needs {what}")),
    }
}

/// SOL-22: the system the arguments ask for: the name after `--system`,
/// or the first known system with no such argument. A missing or unknown
/// name is an error whose message lists the known names.
pub fn chosen_system<S: AsRef<str>>(args: &[S]) -> Result<SystemBuilder, String> {
    let known = KNOWN_SYSTEMS.map(|(name, _)| name).join(", ");
    let Some(name) = value_after(args, SYSTEM_ARG, &format!("a system name: one of {known}"))?
    else {
        return Ok(KNOWN_SYSTEMS[0].1);
    };
    KNOWN_SYSTEMS
        .iter()
        .find(|(known_name, _)| *known_name == name)
        .map(|&(_, system)| system)
        .ok_or_else(|| format!("unknown system {name:?}: the known systems are {known}"))
}

/// SECT-40: a sector size as `--sector` takes it: a preset's name, matched
/// exactly in lower case, or a decimal count of gated systems in the
/// generator's range. Anything else is an error that lists the presets
/// and the range.
pub fn sector_size(value: &str, tunables: &GeneratorTunables) -> Result<u32, String> {
    if let Some(&(_, preset)) = PRESET_NAMES.iter().find(|(name, _)| *name == value) {
        return Ok(preset.gated_count(tunables));
    }
    let (min, max) = (tunables.min_gated_count, tunables.max_gated_count);
    let decimal = !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    match value.parse::<u32>() {
        Ok(count) if decimal && (min..=max).contains(&count) => Ok(count),
        _ => Err(format!(
            "{value:?} is not a sector size: {}",
            sector_sizes(tunables)
        )),
    }
}

/// SECT-40: what `--sector` accepts, for its error messages.
fn sector_sizes(tunables: &GeneratorTunables) -> String {
    let presets = PRESET_NAMES.map(|(name, _)| name).join(", ");
    format!(
        "give one of {presets}, or a number of gated systems from {} to {}",
        tunables.min_gated_count, tunables.max_gated_count
    )
}

/// SECT-40 to SECT-42: what the arguments ask the game to start with: a
/// new sector after `--sector`, seeded by `--seed` if given; else the
/// system of SOL-22. `--sector` with `--system`, `--seed` without
/// `--sector`, a bad size and a bad seed are errors with a message.
pub fn chosen_start<S: AsRef<str>>(
    args: &[S],
    tunables: &GeneratorTunables,
) -> Result<Start, String> {
    let sector = value_after(args, SECTOR_ARG, "a size")
        .map_err(|_| format!("{SECTOR_ARG} needs a size: {}", sector_sizes(tunables)))?;
    let seed = value_after(args, SEED_ARG, "a seed: a whole number")?;
    let system = args.iter().any(|arg| arg.as_ref() == SYSTEM_ARG);
    let Some(size) = sector else {
        if seed.is_some() {
            return Err(format!("{SEED_ARG} seeds a sector: give {SECTOR_ARG} too"));
        }
        return chosen_system(args).map(Start::System);
    };
    if system {
        return Err(format!(
            "{SECTOR_ARG} and {SYSTEM_ARG} can't be given together: start a sector or a system"
        ));
    }
    let gated_count = sector_size(size, tunables)?;
    let seed = seed
        .map(|text| parse_seed(text).map_err(|e| e.to_string()))
        .transpose()?;
    Ok(Start::Sector(SectorRequest { gated_count, seed }))
}

/// Queues what the game starts with: a system's records for the
/// simulation to spawn, or a generated sector and no system (SECT-01).
/// Called again, it replaces what an earlier call queued.
pub fn load_start(app: &mut App, start: Start) {
    match start {
        Start::System(system) => {
            app.world_mut().remove_resource::<LoadedSector>();
            app.insert_resource(SystemDataResource(Some(system())));
        }
        Start::Sector(request) => {
            let tunables = GeneratorTunables::default();
            let sector = generate_sector(request.settings(), &tunables).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(2);
            });
            app.insert_resource(LoadedSector { sector, tunables })
                .insert_resource(SystemDataResource(None));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tunables() -> GeneratorTunables {
        GeneratorTunables::default()
    }

    fn start(args: &[&str]) -> Result<Start, String> {
        chosen_start(args, &tunables())
    }

    /// The sector the arguments ask for, or why not.
    fn sector_of(args: &[&str]) -> Result<SectorRequest, String> {
        match start(args)? {
            Start::Sector(request) => Ok(request),
            Start::System(_) => Err("a system".to_string()),
        }
    }

    fn star_of(args: &[&str]) -> Result<String, String> {
        match start(args)? {
            Start::System(system) => Ok(system().star_name),
            Start::Sector(_) => Err("a sector".to_string()),
        }
    }

    /// SOL-22 / SECT-42: with neither `--system` nor `--sector` the game
    /// starts on Corvane, whatever else is passed.
    #[test]
    fn corvane_is_loaded_without_a_system_or_sector_argument() {
        assert_eq!(star_of(&[]).as_deref(), Ok("Corvane"));
        assert_eq!(star_of(&["--other", "sol"]).as_deref(), Ok("Corvane"));
    }

    /// SOL-22: `--system` picks a system by name.
    #[test]
    fn the_system_argument_picks_the_system() {
        assert_eq!(star_of(&["--system", "sol"]).as_deref(), Ok("Sol"));
        assert_eq!(star_of(&["--system", "corvane"]).as_deref(), Ok("Corvane"));
    }

    /// SOL-22: an unknown or missing name is an error listing the known
    /// names; names are matched exactly, in lower case.
    #[test]
    fn an_unknown_system_is_an_error_listing_the_known_names() {
        for args in [
            &["--system", "vega"][..],
            &["--system"][..],
            &["--system", "Sol"][..],
        ] {
            let message = star_of(args).unwrap_err();
            assert!(message.contains("corvane, sol"), "{message}");
        }
        assert!(star_of(&["--system", "vega"]).unwrap_err().contains("vega"));
    }

    /// SECT-40: `--sector` takes the presets by their lower-case names, or
    /// a count in the generator's range, and starts a sector with no seed
    /// unless one is given.
    #[test]
    fn the_sector_argument_takes_a_preset_or_a_count() {
        let t = tunables();
        for (name, count) in [
            ("small", t.preset_small),
            ("medium", t.preset_medium),
            ("large", t.preset_large),
            ("40", 40),
        ] {
            assert_eq!(
                sector_of(&["--sector", name]),
                Ok(SectorRequest {
                    gated_count: count,
                    seed: None
                })
            );
        }
        let edges = [t.min_gated_count, t.max_gated_count].map(|n| n.to_string());
        for edge in &edges {
            assert!(start(&["--sector", edge]).is_ok(), "{edge}");
        }
    }

    /// SECT-40: anything else, or no value, stops the program with a
    /// message that lists the presets and the range.
    #[test]
    fn a_bad_sector_size_is_an_error_listing_the_presets_and_range() {
        let t = tunables();
        let below = (t.min_gated_count - 1).to_string();
        let above = (t.max_gated_count + 1).to_string();
        let range = format!("from {} to {}", t.min_gated_count, t.max_gated_count);
        for args in [
            &["--sector", "Small"][..],
            &["--sector", "huge"][..],
            &["--sector", &below][..],
            &["--sector", &above][..],
            &["--sector", "+40"][..],
            &["--sector", "4.0e1"][..],
            &["--sector"][..],
        ] {
            let message = start(args).unwrap_err();
            assert!(message.contains("small, medium, large"), "{message}");
            assert!(message.contains(&range), "{message}");
        }
    }

    /// SECT-41: `--seed` sets the sector's seed as a decimal `u64`; an
    /// unparsable value, a missing one, or a seed with no `--sector` is an
    /// error.
    #[test]
    fn the_seed_argument_seeds_the_sector() {
        let request = sector_of(&["--sector", "small", "--seed", "18446744073709551615"]).unwrap();
        assert_eq!(request.seed, Some(u64::MAX));
        assert_eq!(request.settings().seed, u64::MAX);
        for args in [
            &["--sector", "small", "--seed", "twelve"][..],
            &["--sector", "small", "--seed", "-1"][..],
            &["--sector", "small", "--seed"][..],
            &["--seed", "12"][..],
        ] {
            assert!(start(args).is_err(), "{args:?}");
        }
        assert!(start(&["--seed", "12"]).unwrap_err().contains("--sector"));
    }

    /// SECT-41 / SGEN-04: without `--seed` the seed is random, so two
    /// starts almost surely differ.
    #[test]
    fn without_a_seed_the_seed_is_random() {
        let request = SectorRequest {
            gated_count: 30,
            seed: None,
        };
        let seeds: Vec<u64> = (0..4).map(|_| request.settings().seed).collect();
        assert!(seeds.windows(2).any(|w| w[0] != w[1]), "{seeds:?}");
    }

    /// SECT-42: `--sector` together with `--system` stops the program
    /// with a message naming both.
    #[test]
    fn a_sector_and_a_system_together_are_an_error() {
        for args in [
            &["--sector", "small", "--system", "sol"][..],
            &["--system", "corvane", "--sector", "small"][..],
        ] {
            let message = start(args).unwrap_err();
            assert!(message.contains("--sector") && message.contains("--system"));
        }
    }
}
