//! Shared formatting for distances, durations and speeds in every readout
//! (UNIT-01 to UNIT-04). Panels call these rather than formatting figures
//! themselves, so a distance, a time or a delta-v reads the same
//! everywhere.

use crate::sim::units::KM_PER_AU;

/// Decimal places a distance in AU is shown to (UNIT-01).
const AU_DECIMALS: usize = 2;
const MINUTES_PER_DAY: f64 = 1440.0;
const HOURS_PER_DAY: f64 = 24.0;
const DAYS_PER_YEAR: f64 = 365.25;

/// A distance in km as a readout: whole km below `au_threshold_au`, AU
/// to two decimal places at or above it with trailing zeros dropped, both
/// with thousands separators ("182,345 km", "0.36 AU", "2 AU") (UNIT-01).
pub fn format_distance(km: f64, au_threshold_au: f64) -> String {
    let au = km / KM_PER_AU;
    if au >= au_threshold_au {
        format!("{} AU", decimal(au, AU_DECIMALS))
    } else {
        format!("{} km", decimal(km, 0))
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

/// A speed or a delta-v in km/s as a bare figure: one decimal place at 1
/// km/s or more, two below ("9.9", "0.29"). A figure that rounds up to 1
/// reads as one ("1.0") (UNIT-04).
pub fn format_speed_figure(kms: f64) -> String {
    if (kms.abs() * 100.0).round() >= 100.0 {
        format!("{kms:.1}")
    } else {
        format!("{kms:.2}")
    }
}

/// A speed or a delta-v as a readout, with its unit: "9.9 km/s",
/// "0.29 km/s" (UNIT-04).
pub fn format_speed(kms: f64) -> String {
    format!("{} km/s", format_speed_figure(kms))
}

/// `value` rounded to `places` decimals, trailing zeros (and a bare
/// point) dropped, with thousands separators.
fn decimal(value: f64, places: usize) -> String {
    // Round half away from zero first; formatting alone rounds half to
    // even ("0.5" would read "0").
    let factor = 10f64.powi(places as i32);
    let rounded = (value * factor).round() / factor;
    let text = format!("{rounded:.places$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    let (int_part, frac_part) = match text.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (text, None),
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

    /// UNIT-01: whole km below the threshold, AU to two decimals from it
    /// with trailing zeros dropped, with thousands separators.
    #[test]
    fn distances_switch_from_km_to_au_at_the_threshold() {
        assert_eq!(distance(850.4), "850 km");
        assert_eq!(distance(182_345.0), "182,345 km");
        assert_eq!(distance(1_234_567.6), "1,234,568 km");
        assert_eq!(distance(0.5), "1 km");
        assert_eq!(distance(0.0099 * KM_PER_AU), "1,481,019 km");
        assert_eq!(distance(0.362 * KM_PER_AU), "0.36 AU");
        assert_eq!(distance(12.43 * KM_PER_AU), "12.43 AU");
        assert_eq!(distance(1.3 * KM_PER_AU), "1.3 AU");
        assert_eq!(distance(2.0 * KM_PER_AU), "2 AU");
        assert_eq!(distance(0.01 * KM_PER_AU), "0.01 AU");
        assert_eq!(distance(1234.5 * KM_PER_AU), "1,234.5 AU");
    }

    /// UNIT-04: a speed reads in km/s to one decimal from 1 km/s, and to
    /// two below.
    #[test]
    fn speeds_read_to_one_decimal_from_one_km_per_s() {
        assert_eq!(format_speed(9.9), "9.9 km/s");
        assert_eq!(format_speed(12.34), "12.3 km/s");
        assert_eq!(format_speed(1.0), "1.0 km/s");
        assert_eq!(format_speed(0.29), "0.29 km/s");
        assert_eq!(format_speed(0.076), "0.08 km/s");
        assert_eq!(format_speed(0.999), "1.0 km/s");
        assert_eq!(format_speed_figure(0.0), "0.00");
        assert_eq!(format_speed_figure(24.0), "24.0");
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
