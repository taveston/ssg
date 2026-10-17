//! The km -> render-space conversions (SCL-01, SCL-02, ARCH-12).
//!
//! In the system view a single linear scale applies to every orbit -
//! planetary (AU-scale) and lunar/station (km-scale) alike - so relative
//! positions are true to scale. The sector view (SECT-03), whose
//! distances are light years, has a linear scale of its own, so a sector
//! takes about as much render space as a system and the camera's limits
//! and clip planes serve both. Nothing else in the crate multiplies by a
//! render scale. Marker *size* is a fixed pixel size by class (see
//! `markers/mod.rs`), never derived from physical size.

use crate::sim::units::{KM_PER_AU, KM_PER_LIGHT_YEAR};
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

/// SCL-02: a render-space length, in km.
pub fn render_to_km(render: f32) -> f64 {
    render as f64 / RENDER_UNITS_PER_KM
}

/// SCL-02: a position in km, in render space. The same linear scale on
/// every axis, so geometry is never distorted.
pub fn km_to_render_vec3(km: DVec3) -> Vec3 {
    (km * RENDER_UNITS_PER_KM).as_vec3()
}

/// Render units per light year in the sector view. A large sector, about
/// 130 ly across, then spans about 1,300 render units, about as far as
/// Corvane's gates.
const SECTOR_RENDER_UNITS_PER_LY: f64 = 10.0;
const SECTOR_RENDER_UNITS_PER_KM: f64 = SECTOR_RENDER_UNITS_PER_LY / KM_PER_LIGHT_YEAR;

/// SECT-03: a star's position in a sector, in km from the sector's centre
/// with z along the slab's thin axis, in the sector view's render space:
/// the slab's mean plane is the render plane (y = 0), the sector's y axis
/// points north (CAM-02) and its z axis up.
pub fn sector_km_to_render_vec3(km: DVec3) -> Vec3 {
    (DVec3::new(km.x, km.z, -km.y) * SECTOR_RENDER_UNITS_PER_KM).as_vec3()
}

/// SECT-44: a distance in km, in the sector view's render units.
pub fn sector_km_to_render(km: f64) -> f32 {
    (km * SECTOR_RENDER_UNITS_PER_KM) as f32
}

/// SECT-09 / SECT-11: a distance in light years, in the sector view's
/// render units.
pub fn ly_to_render(ly: f64) -> f32 {
    (ly * SECTOR_RENDER_UNITS_PER_LY) as f32
}

/// SECT-09: a length in the sector view's render units, in light years.
pub fn render_to_ly(render: f32) -> f64 {
    render as f64 / SECTOR_RENDER_UNITS_PER_LY
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SECT-03: the sector's scale is linear and keeps its axes: east
    /// stays east, the sector's north is the view's north (CAM-02), and
    /// the thin axis is up off the plane.
    #[test]
    fn the_sector_scale_is_linear_with_north_up() {
        let ly = KM_PER_LIGHT_YEAR;
        let east = sector_km_to_render_vec3(DVec3::new(ly, 0.0, 0.0));
        let north = sector_km_to_render_vec3(DVec3::new(0.0, 2.0 * ly, 0.0));
        let up = sector_km_to_render_vec3(DVec3::new(0.0, 0.0, 3.0 * ly));
        let north_axis = crate::view::camera::rules::NORTH;
        assert!((east - Vec3::X * ly_to_render(1.0)).length() < 1e-4);
        assert!((north - north_axis * ly_to_render(2.0)).length() < 1e-4);
        assert!((up - Vec3::Y * ly_to_render(3.0)).length() < 1e-4);
        assert!((render_to_ly(ly_to_render(12.5)) - 12.5).abs() < 1e-6);
    }

    /// SCL-02: one linear scale - doubling a distance doubles its render
    /// length, at planetary and lunar scale alike.
    #[test]
    fn the_scale_is_linear() {
        for km in [9_394.0_f64, 430_292.0, KM_PER_AU, 4.3 * KM_PER_AU] {
            let ratio = km_to_render(2.0 * km) / km_to_render(km);
            assert!((ratio - 2.0).abs() < 1e-5, "{km} km: ratio {ratio}");
        }
        assert_eq!(km_to_render(0.0), 0.0);
    }

    /// SCL-02: a vector converts exactly as its components do.
    #[test]
    fn vectors_and_distances_share_the_scale() {
        let km = DVec3::new(KM_PER_AU, 0.0, -2.0 * KM_PER_AU);
        let render = km_to_render_vec3(km);
        assert!((render.x - km_to_render(KM_PER_AU)).abs() < 1e-4);
        assert_eq!(render.y, 0.0);
        assert!((render.z + km_to_render(2.0 * KM_PER_AU)).abs() < 1e-4);
    }
}
