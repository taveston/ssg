//! Maps real units (AU, km) to render-space units. Deliberately not to real
//! proportions - at real scale planets are invisible specks next to their
//! orbit radius, so orbit distance and body size each get their own,
//! independently tuned scale.

use bevy::prelude::Vec3;

/// Render units per astronomical unit, for orbit *distance*.
pub const AU_TO_UNITS: f32 = 12.0;

const MIN_BODY_RADIUS: f32 = 0.18;
const MAX_BODY_RADIUS: f32 = 2.2;
const BODY_RADIUS_SCALE: f32 = 0.004;

pub fn au_to_render(au: f64) -> f32 {
    au as f32 * AU_TO_UNITS
}

/// Compressed (sqrt) scale for body *size*, with a floor and ceiling so the
/// star doesn't dwarf the planets and small bodies stay visible.
pub fn km_to_render_radius(km: f64) -> f32 {
    let radius = (km.max(1.0) as f32).sqrt() * BODY_RADIUS_SCALE;
    radius.clamp(MIN_BODY_RADIUS, MAX_BODY_RADIUS)
}

/// A moon's orbit around its planet uses its own small linear scale (moon
/// orbits are tens of thousands of km, not AU) with a floor that keeps it
/// clear of the planet's own render radius.
pub fn moon_orbit_render(orbit_radius_km: f64, planet_render_radius: f32) -> f32 {
    let scaled = (orbit_radius_km / 15_000.0) as f32;
    scaled.max(planet_render_radius + 0.15)
}

/// A static position at the system edge, given as (distance, angle) in AU/degrees.
pub fn polar_au_to_render(distance_au: f64, angle_deg: f64) -> Vec3 {
    let angle = angle_deg.to_radians();
    let distance = au_to_render(distance_au);
    Vec3::new(
        distance * angle.cos() as f32,
        0.0,
        distance * angle.sin() as f32,
    )
}
