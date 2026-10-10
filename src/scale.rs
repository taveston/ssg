//! The one km -> render-space conversion (SCL-01, SCL-02, ARCH-12).
//!
//! A single linear scale applies to every orbit - planetary (AU-scale) and
//! lunar/station (km-scale) alike - so relative positions are true to
//! scale. Nothing else in the crate multiplies by a render scale. Marker
//! *size* is a fixed pixel size by class (see `markers.rs`), never derived
//! from physical size.

use crate::system_data::KM_PER_AU;
use bevy::math::DVec3;
use bevy::prelude::Vec3;

/// Render units per AU. Puts the outermost gates (~6 AU) at a few hundred
/// render units, which is what the camera's zoom range is tuned around.
const RENDER_UNITS_PER_AU: f64 = 90.0;
const RENDER_UNITS_PER_KM: f64 = RENDER_UNITS_PER_AU / KM_PER_AU;

/// SCL-02: a distance in km, in render units.
pub fn km_to_render(km: f64) -> f32 {
    (km * RENDER_UNITS_PER_KM) as f32
}

/// SCL-02: a position in km, in render space. The same linear scale on
/// every axis, so geometry is never distorted.
pub fn km_to_render_vec3(km: DVec3) -> Vec3 {
    (km * RENDER_UNITS_PER_KM).as_vec3()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SCL-02: one linear scale - doubling a distance doubles its render
    /// length, at planetary and lunar scale alike.
    #[test]
    fn scl_02_the_scale_is_linear() {
        for km in [9_394.0_f64, 430_292.0, KM_PER_AU, 4.3 * KM_PER_AU] {
            let ratio = km_to_render(2.0 * km) / km_to_render(km);
            assert!((ratio - 2.0).abs() < 1e-5, "{km} km: ratio {ratio}");
        }
        assert_eq!(km_to_render(0.0), 0.0);
    }

    /// SCL-02: a vector converts exactly as its components do.
    #[test]
    fn scl_02_vectors_and_distances_share_the_scale() {
        let km = DVec3::new(KM_PER_AU, 0.0, -2.0 * KM_PER_AU);
        let render = km_to_render_vec3(km);
        assert!((render.x - km_to_render(KM_PER_AU)).abs() < 1e-4);
        assert_eq!(render.y, 0.0);
        assert!((render.z + km_to_render(2.0 * KM_PER_AU)).abs() < 1e-4);
    }
}
