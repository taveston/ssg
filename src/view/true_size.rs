//! How big a body really looks on screen (MRK-07, ZOOM-14): the pixel size
//! of its true radius at a given depth, and the camera distance at which it
//! fills a given share of the map. Plain functions; the markers and the
//! zoom limits call them.

/// MRK-07: the on-screen radius, in pixels, of a sphere of `radius`
/// (render units) at `depth` in front of a perspective camera with vertical
/// field of view `fov_y`, drawn into a viewport `viewport_height` pixels
/// tall. A body at or behind the camera has no size.
pub fn projected_radius_px(radius: f32, depth: f32, fov_y: f32, viewport_height: f32) -> f32 {
    if depth <= f32::EPSILON {
        return 0.0;
    }
    radius * viewport_height / (2.0 * depth * (fov_y / 2.0).tan())
}

/// MRK-07: the half-size a disc is drawn at: its marker's, or its true
/// on-screen radius when that is larger.
pub fn disc_half_px(marker_half: f32, true_radius_px: f32) -> f32 {
    marker_half.max(true_radius_px)
}

/// ZOOM-14: the camera distance at which a body of `radius` (render units)
/// has a diameter of `fraction` of the map area's height. The viewport
/// shows `2 * distance * tan(fov / 2)` render units over `viewport_height`
/// pixels, so the diameter in pixels is `2 * radius * viewport_height /
/// (2 * distance * tan(fov / 2))`.
pub fn distance_for_screen_fraction(
    radius: f32,
    fov_y: f32,
    viewport_height: f32,
    map_height: f32,
    fraction: f32,
) -> f32 {
    let denominator = fraction * map_height * (fov_y / 2.0).tan();
    if denominator <= f32::EPSILON {
        return f32::MAX;
    }
    radius * viewport_height / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOV: f32 = std::f32::consts::FRAC_PI_4;

    /// MRK-07: a body whose radius is half the visible height fills the
    /// viewport, and halving the depth doubles the pixels.
    #[test]
    fn projected_radius_scales_with_closeness() {
        let depth = 100.0;
        let visible_height = 2.0 * depth * (FOV / 2.0).tan();
        let px = projected_radius_px(visible_height / 2.0, depth, FOV, 900.0);
        assert!((px - 450.0).abs() < 1e-2);
        let closer = projected_radius_px(visible_height / 2.0, depth / 2.0, FOV, 900.0);
        assert!((closer - 900.0).abs() < 1e-2);
        assert_eq!(projected_radius_px(1.0, 0.0, FOV, 900.0), 0.0);
    }

    /// MRK-07: the marker stands until the true size outgrows it.
    #[test]
    fn disc_keeps_its_marker_until_the_true_size_is_larger() {
        assert_eq!(disc_half_px(6.0, 2.0), 6.0);
        assert_eq!(disc_half_px(6.0, 40.0), 40.0);
    }

    /// ZOOM-14: at the computed distance the body's projected diameter is
    /// the asked fraction of the map's height.
    #[test]
    fn inner_limit_distance_fills_the_fraction() {
        let (viewport, map) = (900.0, 846.0);
        let distance = distance_for_screen_fraction(0.3, FOV, viewport, map, 0.25);
        let diameter = 2.0 * projected_radius_px(0.3, distance, FOV, viewport);
        assert!((diameter - 0.25 * map).abs() < 1e-2);
    }
}
