//! What a surface reads of its star (SURF-22, SURF-23): its class, which
//! colours its light, and its age, which sets the crater density. Both
//! come from the star's facts (`sim/star_facts.rs`); a system whose star
//! has none is lit like the Sun.

use crate::sim::star_facts::StarFacts;
use crate::view::tunables::Tunables;

/// A star's class as its light colour knows it: the main-sequence class
/// letters, a white dwarf and a giant. The order is the order of
/// `star_light_colours`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarClass {
    M,
    K,
    G,
    F,
    A,
    WhiteDwarf,
    Giant,
}

impl StarClass {
    /// SURF-23: the class a spectral type names: "D…" is a white dwarf,
    /// a type ending in "III" a giant, and otherwise the first letter.
    /// Anything else reads as a G star.
    pub fn from_spectral_type(spectral_type: &str) -> Self {
        let text = spectral_type.trim();
        if text.starts_with('D') {
            return StarClass::WhiteDwarf;
        }
        if text.ends_with("III") {
            return StarClass::Giant;
        }
        match text.chars().next() {
            Some('M') => StarClass::M,
            Some('K') => StarClass::K,
            Some('F') => StarClass::F,
            Some('A') => StarClass::A,
            _ => StarClass::G,
        }
    }

    /// SURF-23: the colour of the star's light, as linear RGB.
    pub fn light_colour(self, tunables: &Tunables) -> [f32; 3] {
        tunables.star_light_colours[self as usize]
    }
}

/// What the surface reads of the star.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarLight {
    pub class: StarClass,
    pub age_gyr: f64,
}

/// The star of a system whose star has no facts: the Sun's.
const SUN_AGE_GYR: f64 = 4.6;

impl StarLight {
    /// The light of a star from its facts, or the Sun's without them.
    pub fn from_facts(facts: Option<&StarFacts>) -> Self {
        match facts {
            Some(facts) => Self {
                class: StarClass::from_spectral_type(&facts.spectral_type),
                age_gyr: facts.age_gyr,
            },
            None => Self {
                class: StarClass::G,
                age_gyr: SUN_AGE_GYR,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SURF-23: each spectral type reads as its class, and each class has
    /// its own light colour in the tunables.
    #[test]
    fn spectral_types_read_as_their_class_with_its_own_colour() {
        let t = Tunables::default();
        for (text, class) in [
            ("M4V", StarClass::M),
            ("K2V", StarClass::K),
            ("G2V", StarClass::G),
            ("F7V", StarClass::F),
            ("A1V", StarClass::A),
            ("D4", StarClass::WhiteDwarf),
            ("K2III", StarClass::Giant),
            ("M0III", StarClass::Giant),
            ("", StarClass::G),
        ] {
            assert_eq!(StarClass::from_spectral_type(text), class, "{text}");
        }
        let m = StarClass::M.light_colour(&t);
        let f = StarClass::F.light_colour(&t);
        assert!(m[0] > m[2] && f[2] > f[0], "M is red, F is blue");
        assert_eq!(StarClass::Giant.light_colour(&t), t.star_light_colours[6]);
    }

    /// SURF-22 / SURF-23: the light is read from the star's facts, and a
    /// star without them is the Sun.
    #[test]
    fn the_light_comes_from_the_stars_facts_or_is_the_suns() {
        let facts = StarFacts {
            spectral_type: "K2V".to_string(),
            luminosity_lsun: 0.37,
            age_gyr: 5.1,
        };
        let light = StarLight::from_facts(Some(&facts));
        assert_eq!(light.class, StarClass::K);
        assert_eq!(light.age_gyr, 5.1);
        let sun = StarLight::from_facts(None);
        assert_eq!(sun.class, StarClass::G);
        assert_eq!(sun.age_gyr, SUN_AGE_GYR);
    }
}
