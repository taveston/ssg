//! The generator's stages, in the order they run, and the seed each one
//! draws from (SGEN-03). Each stage's seed is derived from the sector seed
//! and the stage's name, so changing what one stage draws, or how many
//! numbers it draws, never moves the numbers of another.

use super::rng::derive_seed;

/// A stage of the sector generator, in running order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Stage 1: the slab and its sites (SGEN-10 to SGEN-12).
    Volume,
    /// Stage 2: the catalogue's stars (SGEN-20 to SGEN-29).
    Stars,
    /// Stage 3: every system's summary (SGEN-30 to SGEN-43).
    Summaries,
    /// Stage 4: which systems carry gates (SGEN-50 to SGEN-59).
    Gates,
    /// Stage 5: the old network's links (SGEN-60 to SGEN-65).
    Links,
    /// The failure of links (SGEN-70 to SGEN-76).
    Failure,
    /// Names for gated systems and their planets (SGEN-80 to SGEN-82).
    Names,
}

impl Stage {
    /// Every stage, in running order.
    pub const ALL: [Stage; 7] = [
        Stage::Volume,
        Stage::Stars,
        Stage::Summaries,
        Stage::Gates,
        Stage::Links,
        Stage::Failure,
        Stage::Names,
    ];

    /// The name its seed is derived from. Renaming a stage reshuffles it,
    /// so these never change.
    pub fn name(self) -> &'static str {
        match self {
            Stage::Volume => "volume",
            Stage::Stars => "stars",
            Stage::Summaries => "summaries",
            Stage::Gates => "gates",
            Stage::Links => "links",
            Stage::Failure => "failure",
            Stage::Names => "names",
        }
    }
}

/// The seed of every stage, derived from one sector seed (SGEN-03). Part
/// of the sector (SGEN-90).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageSeeds {
    seeds: [u64; Stage::ALL.len()],
}

impl StageSeeds {
    /// Each stage's seed: the sector seed and the stage's name, mixed.
    pub fn derive(sector_seed: u64) -> Self {
        Self {
            seeds: Stage::ALL.map(|stage| derive_seed(sector_seed, stage.name())),
        }
    }

    /// The seed `stage` draws from.
    pub fn get(&self, stage: Stage) -> u64 {
        self.seeds[stage as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-03: every stage draws from its own seed, fixed by the sector
    /// seed.
    #[test]
    fn every_stage_has_its_own_seed() {
        let seeds = StageSeeds::derive(1234);
        let mut all: Vec<u64> = Stage::ALL.iter().map(|&s| seeds.get(s)).collect();
        assert_eq!(StageSeeds::derive(1234), seeds);
        assert_ne!(StageSeeds::derive(1235), seeds);
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), Stage::ALL.len());
        assert_eq!(seeds.get(Stage::Stars), derive_seed(1234, "stars"));
    }
}
