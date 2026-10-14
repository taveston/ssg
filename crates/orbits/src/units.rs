//! Unit conversions. The whole crate works in kilometres and days, so a
//! gravitational parameter is in km³/day², a speed in km/day and an
//! acceleration in km/day². Figures quoted in seconds (km/s speeds, the
//! usual km³/s² gravitational parameters) come through these functions
//! once, on the way in or out.

/// Seconds in one day: the factor behind every conversion below.
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// Turns a speed in km/s into km/day, the unit every velocity in this
/// crate uses. Use it on the way in, for a figure quoted in km/s.
///
/// ```
/// use orbits::km_per_s_to_km_per_day;
///
/// // Earth goes round the Sun at about 30 km/s.
/// let per_day = km_per_s_to_km_per_day(30.0);
/// assert_eq!(per_day, 2_592_000.0);
/// ```
pub fn km_per_s_to_km_per_day(km_per_s: f64) -> f64 {
    km_per_s * SECONDS_PER_DAY
}

/// Turns a speed in km/day into km/s. Use it on the way out, for
/// reporting a speed or a delta-v (a change of speed) in the familiar
/// unit.
///
/// ```
/// use orbits::km_per_day_to_km_per_s;
///
/// let per_s = km_per_day_to_km_per_s(2_592_000.0);
/// assert!((per_s - 30.0).abs() < 1e-9);
/// ```
pub fn km_per_day_to_km_per_s(km_per_day: f64) -> f64 {
    km_per_day / SECONDS_PER_DAY
}

/// Turns a gravitational parameter quoted in km³/s² into km³/day². The
/// gravitational parameter of a body is its mass times the gravitational
/// constant: the one number about a body that orbits round it depend on.
/// Tables give it in km³/s² (the Sun's is about 1.327e11); this crate
/// wants it per day squared.
///
/// ```
/// use orbits::km3_per_s2_to_km3_per_day2;
///
/// let sun = km3_per_s2_to_km3_per_day2(1.327e11);
/// assert!((sun - 9.906e20).abs() < 1e18);
/// ```
pub fn km3_per_s2_to_km3_per_day2(km3_per_s2: f64) -> f64 {
    km3_per_s2 * SECONDS_PER_DAY * SECONDS_PER_DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two speed conversions undo each other (RAIL-03).
    #[test]
    fn speed_conversions_round_trip() {
        let v = 7.5;
        assert!((km_per_day_to_km_per_s(km_per_s_to_km_per_day(v)) - v).abs() < 1e-12);
    }

    /// A gravitational parameter scales by the square of the seconds in a
    /// day, since it carries time squared in its denominator.
    #[test]
    fn gravitational_parameter_scales_by_seconds_squared() {
        assert_eq!(km3_per_s2_to_km3_per_day2(1.0), 86_400.0 * 86_400.0);
    }
}
