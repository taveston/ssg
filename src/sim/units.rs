//! Physical constants the simulation works in (SCL-01, ARCH-22). Everything
//! in `sim/` is in km, kg and days; these are the only conversions.

/// One astronomical unit, in km.
pub const KM_PER_AU: f64 = 149_597_870.7;
/// One parsec, in km: the distance at which one AU subtends one second of
/// arc, 648,000 ÷ π AU. Star positions in a sector are measured in it.
pub const KM_PER_PARSEC: f64 = KM_PER_AU * 648_000.0 / std::f64::consts::PI;
/// One light year, in km: the unit distances between stars are shown in
/// (UNIT-05).
pub const KM_PER_LIGHT_YEAR: f64 = 9.4607e12;
/// Earth's mean radius, in km, the unit generated planets' radii are in.
pub const EARTH_RADIUS_KM: f64 = 6_371.0;
/// The Sun's mean radius, in km, the unit generated stars' radii are in.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read by the system generator, which entering a system calls (SECT-31)"
    )
)]
pub const SOLAR_RADIUS_KM: f64 = 695_700.0;
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

/// Standard gravity, in m/s²: the "g" drive catalogues quote
/// accelerations in.
pub const STANDARD_GRAVITY_M_PER_S2: f64 = 9.806_65;

/// An acceleration quoted in m/s² as km/day², the unit the rails work in
/// (PLAN-20). The one place that conversion is made.
pub fn accel_km_per_day2_from_m_per_s2(m_per_s2: f64) -> f64 {
    m_per_s2 * orbits::SECONDS_PER_DAY * orbits::SECONDS_PER_DAY / M_PER_KM
}
