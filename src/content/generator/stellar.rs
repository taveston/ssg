//! The physics of stars the generator needs: luminosity and radius from
//! mass, effective temperature, main-sequence lifetime and spectral type
//! (SGEN-22, SGEN-23). Plain functions of plain numbers, in solar units.

use super::catalogue::StarClass;
use super::tunables::{MassLuminosity, MassRadius, Span};

/// The Sun's effective temperature, in K: a star's temperature scales from
/// it by (L ÷ R²)^(1/4) (SGEN-22).
const SUN_TEMPERATURE_K: f64 = 5772.0;
/// The Sun's main-sequence lifetime, in Gyr; other stars' scale from it by
/// M^`LIFETIME_EXPONENT` (SGEN-23).
const SUN_LIFETIME_GYR: f64 = 10.0;
/// Heavier stars burn out faster: lifetime goes as mass to this power.
const LIFETIME_EXPONENT: f64 = -2.5;
/// A spectral type's digit runs from 0 at the hot end of the class to 9.
const SPECTRAL_DIGITS: f64 = 10.0;
/// A giant's letter comes from its temperature, by the usual boundaries
/// between classes, in K: below the first it is M, below the second K, and
/// so on; above the last it is A.
const GIANT_LETTER_BOUNDS_K: [(f64, &str); 4] =
    [(3700.0, "M"), (5200.0, "K"), (6000.0, "G"), (7500.0, "F")];

/// A main-sequence star's luminosity from its mass, both in solar units,
/// by three power laws joined at two masses (SGEN-22).
pub fn main_sequence_luminosity(mass_msun: f64, law: &MassLuminosity) -> f64 {
    if mass_msun < law.low_below {
        law.low_factor * mass_msun.powf(law.low_exponent)
    } else if mass_msun <= law.high_above {
        mass_msun.powf(law.mid_exponent)
    } else {
        law.high_factor * mass_msun.powf(law.high_exponent)
    }
}

/// A main-sequence star's radius from its mass, both in solar units
/// (SGEN-22).
pub fn main_sequence_radius(mass_msun: f64, law: &MassRadius) -> f64 {
    let exponent = if mass_msun < law.break_msun {
        law.low_exponent
    } else {
        law.high_exponent
    };
    mass_msun.powf(exponent)
}

/// A star's effective temperature, in K, from its luminosity and radius
/// in solar units: the Sun's 5,772 K × (L ÷ R²)^(1/4), since a star's
/// output goes as its surface area times the fourth power of its
/// temperature (SGEN-22).
pub fn effective_temperature_k(luminosity_lsun: f64, radius_rsun: f64) -> f64 {
    SUN_TEMPERATURE_K * (luminosity_lsun / radius_rsun.powi(2)).powf(0.25)
}

/// How long a star of this mass stays on the main sequence, in Gyr:
/// 10 Gyr × M^−2.5 (SGEN-23).
pub fn main_sequence_lifetime_gyr(mass_msun: f64) -> f64 {
    SUN_LIFETIME_GYR * mass_msun.powf(LIFETIME_EXPONENT)
}

/// The digit of a spectral type, 0 at the heavy (hot) end of the class's
/// mass range to 9 at the light end: floor(10 × (max − M) ÷ (max − min)),
/// kept to 0 to 9 (SGEN-22).
pub fn spectral_digit(mass_msun: f64, range: Span) -> u32 {
    let share = (range.max - mass_msun) / (range.max - range.min);
    (SPECTRAL_DIGITS * share)
        .floor()
        .clamp(0.0, SPECTRAL_DIGITS - 1.0) as u32
}

/// A star's spectral type (SGEN-22): its class letter, the digit and "V"
/// for a main-sequence star ("G2V"); "D" and the digit for a white dwarf
/// ("D4"); and for a giant the letter its temperature gives, the digit
/// and "III" ("K2III").
pub fn spectral_type(class: StarClass, mass_msun: f64, range: Span, temperature_k: f64) -> String {
    let digit = spectral_digit(mass_msun, range);
    match class {
        StarClass::M => format!("M{digit}V"),
        StarClass::K => format!("K{digit}V"),
        StarClass::G => format!("G{digit}V"),
        StarClass::F => format!("F{digit}V"),
        StarClass::A => format!("A{digit}V"),
        StarClass::WhiteDwarf => format!("D{digit}"),
        StarClass::Giant => format!("{}{digit}III", giant_letter(temperature_k)),
    }
}

/// The class letter a giant's temperature gives it.
fn giant_letter(temperature_k: f64) -> &'static str {
    GIANT_LETTER_BOUNDS_K
        .iter()
        .find(|(bound, _)| temperature_k < *bound)
        .map_or("A", |(_, letter)| letter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::tunables::GeneratorTunables;

    /// SGEN-22: the Sun comes out as the Sun.
    #[test]
    fn a_solar_mass_gives_the_sun() {
        let t = GeneratorTunables::default();
        let l = main_sequence_luminosity(1.0, &t.star_mass_luminosity);
        let r = main_sequence_radius(1.0, &t.star_mass_radius);
        assert_eq!((l, r), (1.0, 1.0));
        assert_eq!(effective_temperature_k(l, r), 5772.0);
        assert_eq!(main_sequence_lifetime_gyr(1.0), 10.0);
    }

    /// SGEN-22: each piece of the mass-luminosity law applies in its own
    /// mass range.
    #[test]
    fn luminosity_follows_its_three_laws() {
        let law = GeneratorTunables::default().star_mass_luminosity;
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12 * b.max(1.0);
        assert!(close(
            main_sequence_luminosity(0.2, &law),
            0.23 * 0.2f64.powf(2.3)
        ));
        assert!(close(main_sequence_luminosity(1.5, &law), 1.5f64.powi(4)));
        assert!(close(
            main_sequence_luminosity(2.5, &law),
            1.4 * 2.5f64.powf(3.5)
        ));
        let radius = GeneratorTunables::default().star_mass_radius;
        assert!(close(main_sequence_radius(0.5, &radius), 0.5f64.powf(0.8)));
        assert!(close(main_sequence_radius(2.0, &radius), 2.0f64.powf(0.57)));
    }

    /// SGEN-22: the digit runs from 0 at the heavy end of the range to 9
    /// at the light end.
    #[test]
    fn the_spectral_digit_counts_down_from_the_hot_end() {
        let g = Span::new(0.80, 1.04);
        assert_eq!(spectral_digit(1.04, g), 0);
        assert_eq!(spectral_digit(0.80, g), 9);
        // The Sun, 1.0: floor(10 × 0.04 ÷ 0.24) = 1.
        assert_eq!(spectral_digit(1.0, g), 1);
        assert_eq!(spectral_type(StarClass::G, 1.0, g, 5772.0), "G1V");
        let wd = Span::new(0.5, 0.7);
        assert_eq!(spectral_type(StarClass::WhiteDwarf, 0.6, wd, 8000.0), "D5");
        let giant = Span::new(1.0, 3.0);
        assert_eq!(spectral_type(StarClass::Giant, 3.0, giant, 4500.0), "K0III");
        assert_eq!(spectral_type(StarClass::Giant, 1.0, giant, 3500.0), "M9III");
    }

    /// SGEN-23: heavier stars live shorter lives.
    #[test]
    fn heavier_stars_burn_out_sooner() {
        assert!((main_sequence_lifetime_gyr(2.0) - 10.0 / 2.0f64.powf(2.5)).abs() < 1e-12);
        assert!(main_sequence_lifetime_gyr(0.5) > 50.0);
    }
}
