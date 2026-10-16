//! The sector generator: builds the star map of one game from N, the
//! number of gated systems, and a seed (`docs/spec/sector-generator.md`).
//! It is plain Rust that returns plain data, with no Bevy rendering, UI or
//! ECS types and its own seeded random numbers (SGEN-05, SGEN-06).
//!
//! The work is a pipeline of stages, one module each, run in order by
//! `generate_sector`. Each stage draws from its own seed, derived from the
//! sector seed and the stage's name (`stages.rs`), and reads only the
//! tunables and the outputs of earlier stages, so tuning one stage leaves
//! every earlier one as it was (SGEN-02, SGEN-03):
//!
//! 1. `volume`: the slab and its sites (SGEN-10 to SGEN-12).
//! 2. `stars`: the catalogue's entries and stars (SGEN-20 to SGEN-29).
//! 3. `summaries`: each system's planets and belts (SGEN-30 to SGEN-43).
//!
//! Gate choice, links, failure and names follow as further stages.

pub mod bodies;
pub mod catalogue;
pub mod rng;
pub mod settings;
pub mod stages;
pub mod stars;
pub mod stellar;
pub mod summaries;
pub mod tunables;
pub mod volume;

#[cfg(test)]
pub mod testing;

use bodies::Summary;
use catalogue::Entry;
use rng::Rng;
use settings::{Settings, SettingsError};
use stages::{Stage, StageSeeds};
use tunables::GeneratorTunables;
use volume::Slab;

/// A generated sector (SGEN-90): its settings and stage seeds, the slab,
/// every catalogue entry and every system's summary. Later stages add
/// their own outputs as further fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Sector {
    pub settings: Settings,
    /// The seed each stage drew from (SGEN-03).
    pub seeds: StageSeeds,
    /// The slab's counts and dimensions (stage 1).
    pub slab: Slab,
    /// Every entry, by id (stage 2).
    pub entries: Vec<Entry>,
    /// Every entry's summary, by id: `None` for brown dwarfs and giants
    /// (stage 3).
    pub summaries: Vec<Option<Summary>>,
}

impl Sector {
    /// The summary of the entry with this id, if it has one.
    pub fn summary(&self, id: usize) -> Option<&Summary> {
        self.summaries.get(id).and_then(Option::as_ref)
    }
}

/// A fresh random stream for `stage`, from its own seed (SGEN-03).
pub fn stage_rng(seeds: &StageSeeds, stage: Stage) -> Rng {
    Rng::new(seeds.get(stage))
}

/// Generates the sector for these settings (SGEN-01 to SGEN-03). Settings
/// out of range are refused with an error before any stage runs. The
/// result depends on the settings and tunables only.
pub fn generate_sector(
    settings: Settings,
    tunables: &GeneratorTunables,
) -> Result<Sector, SettingsError> {
    settings.check(tunables)?;
    let seeds = StageSeeds::derive(settings.seed);
    let volume = volume::run(
        settings.gated_count,
        &mut stage_rng(&seeds, Stage::Volume),
        tunables,
    );
    let entries = stars::run(&volume, &mut stage_rng(&seeds, Stage::Stars), tunables);
    let summaries = summaries::run(&entries, &mut stage_rng(&seeds, Stage::Summaries), tunables);
    Ok(Sector {
        settings,
        seeds,
        slab: volume.slab,
        entries,
        summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testing::{check_seeds, check_sizes};

    /// SGEN-01: a count out of range is an error from the generator, not a
    /// panic.
    #[test]
    fn the_generator_refuses_a_count_out_of_range() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: 7,
            seed: 1,
        };
        let error = generate_sector(settings, &t).unwrap_err();
        assert!(error.to_string().contains("8 to 300"));
    }

    /// SGEN-02: the same settings and tunables give the same sector, field
    /// for field.
    #[test]
    fn the_same_settings_give_the_same_sector() {
        let t = GeneratorTunables::default();
        for n in check_sizes(&t) {
            for seed in check_seeds(3) {
                let settings = Settings {
                    gated_count: n,
                    seed,
                };
                let first = generate_sector(settings, &t).unwrap();
                let second = generate_sector(settings, &t).unwrap();
                assert_eq!(first, second);
                assert_eq!(first.settings, settings);
                assert_eq!(first.seeds, StageSeeds::derive(seed));
            }
        }
        let other = Settings {
            gated_count: 30,
            seed: 2,
        };
        let one = Settings {
            gated_count: 30,
            seed: 1,
        };
        assert_ne!(
            generate_sector(one, &t).unwrap().entries,
            generate_sector(other, &t).unwrap().entries
        );
    }

    /// SGEN-03: changing a tunable of one stage leaves every earlier
    /// stage's output equal.
    #[test]
    fn tuning_a_stage_leaves_earlier_stages_alone() {
        let t = GeneratorTunables::default();
        let settings = Settings {
            gated_count: 80,
            seed: 99,
        };
        let base = generate_sector(settings, &t).unwrap();

        // A summaries tunable: volume and stars hold still.
        let summaries_tuned = GeneratorTunables {
            min_mutual_hill: 12.0,
            belt_width_ratio: tunables::Span::new(1.1, 1.5),
            ..t.clone()
        };
        let tuned = generate_sector(settings, &summaries_tuned).unwrap();
        assert_eq!(tuned.slab, base.slab);
        assert_eq!(tuned.entries, base.entries);
        assert_ne!(tuned.summaries, base.summaries);

        // A stars tunable: the volume holds still.
        let stars_tuned = GeneratorTunables {
            metallicity_sigma: 0.3,
            ..t.clone()
        };
        let tuned = generate_sector(settings, &stars_tuned).unwrap();
        assert_eq!(tuned.slab, base.slab);
        let positions = |s: &Sector| s.entries.iter().map(|e| e.position_km).collect::<Vec<_>>();
        assert_eq!(positions(&tuned), positions(&base));
        assert_ne!(tuned.entries, base.entries);
    }

    /// SGEN-04: the seed is part of the sector, so the view can show it.
    #[test]
    fn the_sector_keeps_its_seed() {
        let t = GeneratorTunables::default();
        let settings = Settings::with_random_seed(30);
        let sector = generate_sector(settings, &t).unwrap();
        assert_eq!(sector.settings.seed, settings.seed);
        assert_eq!(sector.seeds, StageSeeds::derive(settings.seed));
    }

    /// SGEN-30: the summary accessor finds an entry's summary by id.
    #[test]
    fn summaries_are_found_by_entry_id() {
        let sector = testing::sector(30, 4);
        for entry in &sector.entries {
            assert_eq!(sector.summary(entry.id).is_some(), entry.has_summary());
        }
        assert!(sector.summary(sector.entries.len()).is_none());
    }
}
