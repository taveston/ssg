//! Phase round a named orbit (BAND-05, PHAS-01 to PHAS-03, PHAS-10): where
//! things in one orbit are relative to each other, and how long one waits
//! for a point of the orbit to come round. Pure functions of angles,
//! radii and periods (ARCH-20).
//!
//! Everything in a named orbit turns at the orbit's rate, so two things in
//! it keep the same gap until one of them burns. A ship's place in the
//! orbit is therefore its phase, and waiting for a point of the orbit
//! takes under one period.

use std::f64::consts::TAU;

/// How far ahead something at `to_rad` is of something at `from_rad` in
/// one orbit, in the sense of motion: an angle in [0, 2π) (PHAS-01).
pub fn gap_rad(from_rad: f64, to_rad: f64) -> f64 {
    let gap = (to_rad - from_rad).rem_euclid(TAU);
    // rem_euclid can round a tiny negative angle up to exactly 2π.
    if gap >= TAU { 0.0 } else { gap }
}

/// The straight-line distance, in km, between two points of a circle of
/// `radius_km` whose angles are `gap_rad` apart: 2r sin(gap ÷ 2). Two
/// ships in one orbit at different phases are this far apart on every day
/// (BAND-05).
pub fn separation_km(radius_km: f64, gap_rad: f64) -> f64 {
    2.0 * radius_km * (gap_rad / 2.0).sin().abs()
}

/// Whether two things in one orbit of `radius_km`, `gap_rad` apart, are
/// close enough to count as one place: closer than `together_km`. A ship
/// together with a station is docked at it (PHAS-02).
pub fn together(radius_km: f64, gap_rad: f64, together_km: f64) -> bool {
    separation_km(radius_km, gap_rad) < together_km
}

/// The days a point of an orbit that turns once in `period_days` takes to
/// go from the angle `from_rad` round to `to_rad`, in the sense of motion:
/// always under one period. Leaving a band waits for the exit point
/// (PHAS-03), and arriving at a station waits for the station to reach
/// the capture point (PHAS-10). An orbit that doesn't turn never waits.
pub fn wait_days(from_rad: f64, to_rad: f64, period_days: f64) -> f64 {
    if !(period_days.is_finite() && period_days > 0.0) {
        return 0.0;
    }
    gap_rad(from_rad, to_rad) / TAU * period_days
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// PHAS-01: the gap is how far ahead the other is, in the sense of
    /// motion, in [0, 2π): a quarter turn behind is three quarters ahead.
    #[test]
    fn the_gap_is_measured_ahead_in_the_sense_of_motion() {
        assert!((gap_rad(0.0, PI / 2.0) - PI / 2.0).abs() < 1e-12);
        assert!((gap_rad(PI / 2.0, 0.0) - 1.5 * PI).abs() < 1e-12);
        assert!((gap_rad(1.0, 1.0 + 3.0 * TAU) - 0.0).abs() < 1e-9);
        assert_eq!(gap_rad(1.0, 1.0 - 1e-18), 0.0);
        let gap = gap_rad(-7.0, 5.0);
        assert!((0.0..TAU).contains(&gap));
    }

    /// BAND-05: two points of a circle half a turn apart are a diameter
    /// apart; a quarter turn, r√2; and the distance is the same whichever
    /// way round the gap is taken.
    #[test]
    fn separation_is_the_chord_of_the_gap() {
        assert!((separation_km(1_000.0, PI) - 2_000.0).abs() < 1e-9);
        assert!((separation_km(1_000.0, PI / 2.0) - 1_000.0 * 2f64.sqrt()).abs() < 1e-9);
        assert!((separation_km(1_000.0, 0.3) - separation_km(1_000.0, TAU - 0.3)).abs() < 1e-9);
    }

    /// PHAS-02: within `together_km` along the chord is together; farther
    /// is not.
    #[test]
    fn things_closer_than_together_km_are_together() {
        let r = 60_000.0;
        assert!(together(r, 5.0 / r, 10.0));
        assert!(together(r, TAU - 5.0 / r, 10.0));
        assert!(!together(r, 50.0 / r, 10.0));
    }

    /// PHAS-03 / PHAS-10: waiting for a point a quarter turn ahead takes a
    /// quarter period; for one just behind, nearly a whole period; never a
    /// whole period or more. An orbit with no period never waits.
    #[test]
    fn waits_are_under_one_period() {
        assert!((wait_days(0.0, PI / 2.0, 2.0) - 0.5).abs() < 1e-12);
        let behind = wait_days(0.0, -1e-6, 2.0);
        assert!(behind > 1.99 && behind < 2.0);
        assert_eq!(wait_days(0.0, 1.0, f64::INFINITY), 0.0);
        assert_eq!(wait_days(0.0, 1.0, 0.0), 0.0);
    }
}
