//! Maps real distances (km) to render-space units. A single linear scale
//! applies to every orbit - planetary (AU-scale) and lunar/station
//! (km-scale) alike - so relative positions are true to scale. Marker *size*
//! is a fixed pixel size by class (see `markers.rs`), never derived from
//! physical size.

use crate::system_data::KM_PER_AU;
use bevy::prelude::Vec3;

/// Render units per AU. Puts the outermost gates (~6 AU) at a few hundred
/// render units, which is what the camera's zoom range is tuned around.
const RENDER_UNITS_PER_AU: f64 = 90.0;
const RENDER_UNITS_PER_KM: f64 = RENDER_UNITS_PER_AU / KM_PER_AU;

pub fn km_to_render(km: f64) -> f32 {
    (km * RENDER_UNITS_PER_KM) as f32
}

pub fn polar_km_to_render(distance_km: f64, angle_deg: f64) -> Vec3 {
    let angle = angle_deg.to_radians();
    let distance = km_to_render(distance_km);
    Vec3::new(
        distance * angle.cos() as f32,
        0.0,
        distance * angle.sin() as f32,
    )
}
