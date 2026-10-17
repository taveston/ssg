//! Which seed the sky is made from (SKY-02). A game started with a sector
//! derives it from the sector's seed and the name "sky", the way the
//! generator's stages derive theirs, so each sector has its own sky and
//! the same sector always has the same one. A hand-made system loaded
//! without a sector (Corvane, Sol) uses `sky_default_seed`.

use crate::content::generator::Sector;
use crate::content::generator::rng::derive_seed;
use crate::view::tunables::Tunables;

/// The name the sky's seed is derived under, like a generator stage's.
pub const SKY_SEED_NAME: &str = "sky";

/// SKY-02: the seed the sky is made from: derived from the sector's seed
/// when there is a sector, else `sky_default_seed`.
pub fn sky_seed(sector: Option<&Sector>, tunables: &Tunables) -> u64 {
    match sector {
        Some(sector) => derive_seed(sector.settings.seed, SKY_SEED_NAME),
        None => tunables.sky_default_seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::settings::Settings;
    use crate::content::generator::tunables::GeneratorTunables;
    use crate::content::generator::{Sector, generate_sector};
    use crate::view::sky::generate::make_sky;

    fn sector(seed: u64) -> Sector {
        let gated_count = GeneratorTunables::default().preset_small;
        let settings = Settings { gated_count, seed };
        generate_sector(settings, &GeneratorTunables::default()).unwrap()
    }

    /// SKY-02: the sky's seed is the generator's derivation of the
    /// sector's seed under "sky": the same sector gives the same seed, and
    /// it follows the sector's seed, not the default.
    #[test]
    fn the_sky_seed_is_derived_from_the_sector_seed() {
        let t = Tunables::default();
        let (a, b) = (sector(7), sector(8));
        assert_eq!(sky_seed(Some(&a), &t), derive_seed(7, "sky"));
        assert_eq!(sky_seed(Some(&a), &t), sky_seed(Some(&sector(7)), &t));
        assert_ne!(sky_seed(Some(&a), &t), sky_seed(Some(&b), &t));
        assert_ne!(sky_seed(Some(&a), &t), t.sky_default_seed);
        assert_ne!(
            sky_seed(Some(&a), &t),
            7,
            "derived, not the sector seed itself"
        );
    }

    /// SKY-02 / SKY-03: a different sector gives a different sky: its
    /// stars and its band differ.
    #[test]
    fn a_different_sector_gives_a_different_sky() {
        let t = Tunables {
            sky_field_stars: 40,
            sky_band_stars: 60,
            ..Tunables::default()
        };
        let (a, b) = (sector(7), sector(8));
        let (sky_a, sky_b) = (
            make_sky(sky_seed(Some(&a), &t), &t),
            make_sky(sky_seed(Some(&b), &t), &t),
        );
        assert_ne!(sky_a.field, sky_b.field);
        assert_ne!(sky_a.band_stars, sky_b.band_stars);
        assert_ne!(sky_a.band.normal, sky_b.band.normal);
        assert_eq!(sky_a, make_sky(sky_seed(Some(&sector(7)), &t), &t));
    }

    /// SKY-02: with no sector loaded the seed is `sky_default_seed`.
    #[test]
    fn without_a_sector_the_default_seed_is_used() {
        let t = Tunables {
            sky_default_seed: 99,
            ..Tunables::default()
        };
        assert_eq!(sky_seed(None, &t), 99);
    }
}
