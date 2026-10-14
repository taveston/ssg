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
/// Metres in a km, for turning Newton's constant into km.
pub const M_PER_KM: f64 = 1000.0;

/// The gravitational parameter of a body of `mass_kg`: Newton's constant
/// times the mass, in the km³/day² the `orbits` crate works in. The one
/// place a mass becomes the number orbits round the body depend on.
pub fn gravitational_parameter_km3_per_day2(mass_kg: f64) -> f64 {
    let si = GRAVITATIONAL_CONSTANT * mass_kg / M_PER_KM.powi(3);
    orbits::km3_per_s2_to_km3_per_day2(si)
}
