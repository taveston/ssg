//! Physical constants the simulation works in (SCL-01, ARCH-22). Everything
//! in `sim/` is in km, kg and days; these are the only conversions.

/// One astronomical unit, in km.
pub const KM_PER_AU: f64 = 149_597_870.7;
/// Earth's mass, the unit content uses for planets.
pub const EARTH_MASS_KG: f64 = 5.972e24;
/// The Sun's mass, the unit content uses for stars.
pub const SOLAR_MASS_KG: f64 = 1.989e30;
/// Newton's gravitational constant, in m^3 kg^-1 s^-2.
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674e-11;
/// Seconds in a day, for turning Kepler's SI result into days.
pub const SECS_PER_DAY: f64 = 86_400.0;
/// Metres in a km, for the same.
pub const M_PER_KM: f64 = 1000.0;
