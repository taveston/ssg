//! Helpers for the generator's tests: the sizes and seeds the invariants
//! are checked over, and shortcuts that run the first stages alone.

use super::bodies::Summary;
use super::catalogue::{Entry, Star, StarClass};
use super::settings::Settings;
use super::stages::{Stage, StageSeeds};
use super::tunables::GeneratorTunables;
use super::volume::Volume;
use super::{Sector, generate_sector, stage_rng, stars, summaries, volume};

/// The sizes the invariants are checked at: N = `min_gated_count` and the
/// three presets (SGEN-91).
pub fn check_sizes(t: &GeneratorTunables) -> Vec<u32> {
    vec![
        t.min_gated_count,
        t.preset_small,
        t.preset_medium,
        t.preset_large,
    ]
}

/// `count` fixed, well-spread seeds.
pub fn check_seeds(count: u64) -> Vec<u64> {
    (1..=count)
        .map(|i| i.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .collect()
}

/// A whole sector at the default tunables.
pub fn sector(gated_count: u32, seed: u64) -> Sector {
    let settings = Settings { gated_count, seed };
    generate_sector(settings, &GeneratorTunables::default()).expect("settings in range")
}

/// Stages 1 and 2 alone, at the default tunables, from the seeds a sector
/// with this seed would use.
pub fn stars_for(gated_count: u32, seed: u64) -> (Volume, Vec<Entry>) {
    let t = GeneratorTunables::default();
    let seeds = StageSeeds::derive(seed);
    let volume = volume::run(gated_count, &mut stage_rng(&seeds, Stage::Volume), &t);
    let entries = stars::run(&volume, &mut stage_rng(&seeds, Stage::Stars), &t);
    (volume, entries)
}

/// Stages 1 to 3 at the default tunables: the entries and their summaries.
pub fn summaries_for(gated_count: u32, seed: u64) -> (Vec<Entry>, Vec<Option<Summary>>) {
    let t = GeneratorTunables::default();
    let (_, entries) = stars_for(gated_count, seed);
    let seeds = StageSeeds::derive(seed);
    let summaries = summaries::run(&entries, &mut stage_rng(&seeds, Stage::Summaries), &t);
    (entries, summaries)
}

/// Each summary of `seeds` Medium sectors, with its primary's mass in
/// solar masses.
pub fn summaries_with_star_mass(seeds: u64) -> Vec<(f64, Summary)> {
    let mut all = Vec::new();
    for seed in check_seeds(seeds) {
        let (entries, summaries) = summaries_for(80, seed);
        for (entry, summary) in entries.iter().zip(summaries) {
            if let (Some(star), Some(summary)) = (entry.primary(), summary) {
                all.push((star.mass_msun, summary));
            }
        }
    }
    all
}

/// Every summary of `seeds` Medium sectors.
pub fn all_summaries(seeds: u64) -> Vec<Summary> {
    summaries_with_star_mass(seeds)
        .into_iter()
        .map(|(_, summary)| summary)
        .collect()
}

/// The range from `min` to `max`, widened by a part in a billion each
/// way, for values that went through floating-point arithmetic.
pub fn about(min: f64, max: f64) -> std::ops::RangeInclusive<f64> {
    min * (1.0 - 1e-9)..=max * (1.0 + 1e-9)
}

/// A star like the Sun, for tests of one rule.
pub fn sample_star() -> Star {
    Star {
        class: StarClass::G,
        mass_msun: 1.0,
        luminosity_lsun: 1.0,
        radius_rsun: 1.0,
        temperature_k: 5772.0,
        spectral_type: "G1V".to_string(),
    }
}
