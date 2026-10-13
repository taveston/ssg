//! Shared formatting for distances and durations in every readout (UNIT-01
//! to UNIT-03). Panels call these rather than formatting figures
//! themselves, so a distance or a time reads the same everywhere.

use crate::sim::units::KM_PER_AU;

/// Significant figures a distance is shown to (UNIT-01).
const DISTANCE_SIG_FIGS: i32 = 3;
const MINUTES_PER_DAY: f64 = 1440.0;
const HOURS_PER_DAY: f64 = 24.0;
const DAYS_PER_YEAR: f64 = 365.25;

/// A distance in km as a readout: km below `au_threshold_au`, AU at or
/// above it, to three significant figures with thousands separators
/// ("182,000 km", "0.362 AU") (UNIT-01).
pub fn format_distance(km: f64, au_threshold_au: f64) -> String {
    let au = km / KM_PER_AU;
    if au >= au_threshold_au {
        format!("{} AU", significant(au))
    } else {
        format!("{} km", significant(km))
    }
}

/// A duration in days as a readout, in the largest unit it reaches: whole
/// minutes, or hours, days or years to one decimal ("5.3 h", "2.4 y")
/// (UNIT-02).
pub fn format_duration(days: f64) -> String {
    if days >= DAYS_PER_YEAR {
        format!("{:.1} y", days / DAYS_PER_YEAR)
    } else if days >= 1.0 {
        format!("{days:.1} d")
    } else if days * HOURS_PER_DAY >= 1.0 {
        format!("{:.1} h", days * HOURS_PER_DAY)
    } else {
        format!("{:.0} min", days * MINUTES_PER_DAY)
    }
}

/// `value` rounded to three significant figures, with its integer part
/// grouped in thousands. Trailing zeros after the point are kept, so the
/// precision shows ("1.30").
fn significant(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return "0".to_string();
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (DISTANCE_SIG_FIGS - 1 - magnitude).max(0);
    let step = 10f64.powi(magnitude + 1 - DISTANCE_SIG_FIGS);
    let rounded = (value / step).round() * step;
    let text = format!("{rounded:.*}", decimals as usize);
    let (int_part, frac_part) = match text.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (text.as_str(), None),
    };
    let grouped = group_thousands(int_part);
    match frac_part {
        Some(frac) => format!("{grouped}.{frac}"),
        None => grouped,
    }
}

/// "182000" as "182,000"; a leading minus sign is kept.
fn group_thousands(digits: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |rest| ("-", rest));
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const THRESHOLD_AU: f64 = 0.01;

    fn distance(km: f64) -> String {
        format_distance(km, THRESHOLD_AU)
    }

    /// UNIT-01: km below the threshold, AU from it, to three significant
    /// figures with thousands separators.
    #[test]
    fn distances_switch_from_km_to_au_at_the_threshold() {
        assert_eq!(distance(850.0), "850 km");
        assert_eq!(distance(182_345.0), "182,000 km");
        assert_eq!(distance(1_234_567.0), "1,230,000 km");
        assert_eq!(distance(0.362 * KM_PER_AU), "0.362 AU");
        assert_eq!(distance(12.43 * KM_PER_AU), "12.4 AU");
        assert_eq!(distance(1.3 * KM_PER_AU), "1.30 AU");
        assert_eq!(distance(0.5), "0.500 km");
        assert_eq!(distance(0.0099 * KM_PER_AU), "1,480,000 km");
        assert_eq!(distance(0.01 * KM_PER_AU), "0.0100 AU");
    }

    /// UNIT-02: each duration reads in the largest unit it reaches.
    #[test]
    fn durations_read_in_their_largest_unit() {
        assert_eq!(format_duration(42.0 / MINUTES_PER_DAY), "42 min");
        assert_eq!(format_duration(5.3 / HOURS_PER_DAY), "5.3 h");
        assert_eq!(format_duration(31.2), "31.2 d");
        assert_eq!(format_duration(2.4 * DAYS_PER_YEAR), "2.4 y");
        assert_eq!(format_duration(1.0), "1.0 d");
        assert_eq!(format_duration(1.0 / HOURS_PER_DAY), "1.0 h");
    }
}
