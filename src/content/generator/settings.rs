//! The generator's settings: how many systems carry gates and the seed
//! (SGEN-01, SGEN-04). Settings are checked against the tunables' range
//! before any stage runs, so a bad count is an error, not a panic.

use std::fmt;
use std::hash::{BuildHasher, Hasher, RandomState};

use super::tunables::GeneratorTunables;

/// A sector size the player picks from (SGEN-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Small,
    Medium,
    Large,
}

impl Preset {
    /// Every preset, smallest first.
    pub const ALL: [Preset; 3] = [Preset::Small, Preset::Medium, Preset::Large];

    /// The number of gated systems this preset asks for.
    pub fn gated_count(self, tunables: &GeneratorTunables) -> u32 {
        match self {
            Preset::Small => tunables.preset_small,
            Preset::Medium => tunables.preset_medium,
            Preset::Large => tunables.preset_large,
        }
    }
}

/// What a sector is made from: N, the number of gated systems, and a
/// seed (SGEN-01). The same settings and tunables always give the same
/// sector (SGEN-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub gated_count: u32,
    pub seed: u64,
}

impl Settings {
    /// Settings for a preset and a seed.
    pub fn preset(preset: Preset, seed: u64, tunables: &GeneratorTunables) -> Self {
        Self {
            gated_count: preset.gated_count(tunables),
            seed,
        }
    }

    /// Settings with a seed of their own, for when the player gives none
    /// (SGEN-04). This is the one place the generator's inputs come from
    /// outside: every stage then draws only from the seed.
    pub fn with_random_seed(gated_count: u32) -> Self {
        Self {
            gated_count,
            seed: random_seed(),
        }
    }

    /// The count is refused if it lies outside the tunables' range
    /// (SGEN-01).
    pub fn check(&self, tunables: &GeneratorTunables) -> Result<(), SettingsError> {
        let (min, max) = (tunables.min_gated_count, tunables.max_gated_count);
        if (min..=max).contains(&self.gated_count) {
            Ok(())
        } else {
            Err(SettingsError::GatedCountOutOfRange {
                count: self.gated_count,
                min,
                max,
            })
        }
    }
}

/// Why settings were refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// N lies outside the range the generator accepts (SGEN-01).
    GatedCountOutOfRange { count: u32, min: u32, max: u32 },
    /// A seed that is not a decimal `u64` (SGEN-04).
    BadSeed(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::GatedCountOutOfRange { count, min, max } => write!(
                f,
                "{count} gated systems is out of range: the generator takes {min} to {max}"
            ),
            SettingsError::BadSeed(text) => {
                write!(
                    f,
                    "{text:?} is not a seed: a seed is a whole number from 0 to {}",
                    u64::MAX
                )
            }
        }
    }
}

impl std::error::Error for SettingsError {}

/// A seed from the operating system's randomness, for a new game with no
/// seed given (SGEN-04). `RandomState` is keyed randomly per process.
pub fn random_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(0);
    hasher.finish()
}

/// A seed read from text, as a decimal `u64` (SGEN-04). Surrounding spaces
/// are ignored. Seeds print with `{}`, which writes the same decimal form.
pub fn parse_seed(text: &str) -> Result<u64, SettingsError> {
    text.trim()
        .parse()
        .map_err(|_| SettingsError::BadSeed(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-01: the presets are Small, Medium and Large.
    #[test]
    fn presets_ask_for_their_counts() {
        let t = GeneratorTunables::default();
        let counts: Vec<u32> = Preset::ALL.iter().map(|p| p.gated_count(&t)).collect();
        assert_eq!(counts, vec![30, 80, 200]);
        assert_eq!(Settings::preset(Preset::Medium, 5, &t).gated_count, 80);
    }

    /// SGEN-01: a count outside the range is an error naming the range.
    #[test]
    fn a_count_out_of_range_is_refused_with_the_range() {
        let t = GeneratorTunables::default();
        for count in [0, 7, 301, 1000] {
            let error = Settings {
                gated_count: count,
                seed: 1,
            }
            .check(&t)
            .unwrap_err();
            let message = error.to_string();
            assert!(message.contains("8 to 300"), "{message}");
        }
        for count in [8, 30, 300] {
            let settings = Settings {
                gated_count: count,
                seed: 1,
            };
            assert_eq!(settings.check(&t), Ok(()));
        }
    }

    /// SGEN-04: seeds read and print as decimal `u64`s.
    #[test]
    fn seeds_read_and_print_as_decimal() {
        assert_eq!(parse_seed("18446744073709551615"), Ok(u64::MAX));
        assert_eq!(parse_seed(" 42 "), Ok(42));
        assert_eq!(parse_seed("42").unwrap().to_string(), "42");
        assert!(parse_seed("-1").is_err());
        assert!(parse_seed("0x10").is_err());
        assert!(parse_seed("seed").unwrap_err().to_string().contains("seed"));
    }

    /// SGEN-04: with no seed given, the seed is random.
    #[test]
    fn a_missing_seed_is_random() {
        let seeds: Vec<u64> = (0..4)
            .map(|_| Settings::with_random_seed(30).seed)
            .collect();
        assert!(seeds.windows(2).any(|pair| pair[0] != pair[1]), "{seeds:?}");
    }
}
