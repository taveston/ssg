//! The star's authored key facts (SYSG-10): its spectral type, age,
//! luminosity and metallicity, as the player's records write them.

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

/// `value` written to `figures` significant figures, with no exponent:
/// 0.000123, 0.523, 1.00, 12.3, 457.
pub fn significant(value: f64, figures: i32) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (figures - 1 - magnitude).max(0) as usize;
    let text = format!("{value:.decimals$}");
    // Rounding up can add a digit (9.996 → "10.00"): write it again at
    // the next magnitude.
    let digits = text.chars().filter(char::is_ascii_digit).count();
    let leading_zeros = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take_while(|&c| c == '0')
        .count();
    if decimals > 0 && digits - leading_zeros > figures as usize {
        return format!("{value:.prec$}", prec = decimals - 1);
    }
    text
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

    /// SYSG-10: three significant figures, whatever the magnitude.
    #[test]
    fn luminosity_keeps_three_significant_figures() {
        assert_eq!(significant(1.0, 3), "1.00");
        assert_eq!(significant(0.000123456, 3), "0.000123");
        assert_eq!(significant(12.345, 3), "12.3");
        assert_eq!(significant(457.2, 3), "457");
        assert_eq!(significant(9.996, 3), "10.0");
        assert_eq!(significant(0.09996, 3), "0.100");
        assert_eq!(significant(999.7, 3), "1000");
    }
}
