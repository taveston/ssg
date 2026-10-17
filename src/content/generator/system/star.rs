//! The star's authored key facts (SYSG-10): its spectral type, age,
//! luminosity and metallicity, as the player's records write them.

use crate::content::figures::significant;
use crate::content::generator::catalogue::Star;

/// The star's luminosity is written to this many significant figures.
const LUMINOSITY_FIGURES: i32 = 3;

/// SYSG-10: the star's key facts: "Class" (its spectral type), "Age" in
/// Gyr to one decimal, "Luminosity" in solar units to three significant
/// figures and "Metallicity" as [Fe/H] to two decimals.
pub fn key_facts(star: &Star, age_gyr: f64, metallicity: f64) -> Vec<(String, String)> {
    vec![
        ("Class".to_string(), star.spectral_type.clone()),
        ("Age".to_string(), format!("{age_gyr:.1} Gyr")),
        (
            "Luminosity".to_string(),
            format!(
                "{} solar luminosities",
                significant(star.luminosity_lsun, LUMINOSITY_FIGURES)
            ),
        ),
        (
            "Metallicity".to_string(),
            format!("[Fe/H] {metallicity:+.2}"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::sample_star;

    /// SYSG-10: the star's key facts are its class, age, luminosity and
    /// metallicity, written as the spec says.
    #[test]
    fn the_star_lists_class_age_luminosity_and_metallicity() {
        let star = Star {
            luminosity_lsun: 0.5234,
            spectral_type: "K3V".to_string(),
            ..sample_star()
        };
        let facts = key_facts(&star, 4.567, -0.054);
        let facts: Vec<(&str, &str)> = facts.iter().map(|(k, v)| (&k[..], &v[..])).collect();
        assert_eq!(
            facts,
            [
                ("Class", "K3V"),
                ("Age", "4.6 Gyr"),
                ("Luminosity", "0.523 solar luminosities"),
                ("Metallicity", "[Fe/H] -0.05"),
            ]
        );
        assert_eq!(key_facts(&star, 1.0, 0.123)[3].1, "[Fe/H] +0.12");
    }
}
