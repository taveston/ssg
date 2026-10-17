//! Which seed a body's surface is drawn from (SURF-02, SURF-05). A body
//! of a hand-made system carries its authored seed. A generated body
//! derives one from the sector seed and the name "surface" followed by
//! its id, the way the generator's stages derive theirs; a generated
//! body has no id of its own, so its display name, unique in its sector's
//! naming, stands in. A body with no sector and no authored seed takes
//! the default.

use crate::content::generator::rng::derive_seed;

/// The seed of a body that has neither an authored seed nor a sector to
/// derive one from (SURF-05).
pub const DEFAULT_SURFACE_SEED: u64 = 1;

/// The name a surface's seed is derived under, like a generator stage's.
const SURFACE_SEED_NAME: &str = "surface";

/// SURF-02: a body's surface seed: its authored seed if it has one; else,
/// with a sector, the sector seed derived under "surface" and the body's
/// name; else the default.
pub fn surface_seed(authored: Option<u64>, sector_seed: Option<u64>, body_name: &str) -> u64 {
    match (authored, sector_seed) {
        (Some(seed), _) => seed,
        (None, Some(sector)) => derive_seed(sector, &format!("{SURFACE_SEED_NAME}{body_name}")),
        (None, None) => DEFAULT_SURFACE_SEED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SURF-02: an authored seed wins; a sector's body gets a seed derived
    /// from the sector seed and its name, fixed and distinct per body and
    /// per sector; with neither, the default.
    #[test]
    fn the_seed_is_authored_derived_from_the_sector_or_the_default() {
        assert_eq!(surface_seed(Some(42), Some(7), "Pell"), 42);
        let a = surface_seed(None, Some(7), "Pell");
        assert_eq!(a, derive_seed(7, "surfacePell"));
        assert_eq!(a, surface_seed(None, Some(7), "Pell"));
        assert_ne!(a, surface_seed(None, Some(8), "Pell"));
        assert_ne!(a, surface_seed(None, Some(7), "Iver"));
        assert_ne!(a, derive_seed(7, "Pell"), "under the name \"surface\"");
        assert_eq!(surface_seed(None, None, "Pell"), DEFAULT_SURFACE_SEED);
    }
}
