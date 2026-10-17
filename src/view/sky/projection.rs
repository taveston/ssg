//! Where a direction on the sky lands on screen (SKY-16). The sky is at
//! infinity, so only the camera's turn matters: its heading and tilt, and
//! the field of view of the viewport. Its position, the pivot, the frame
//! and the zoom play no part. `stars.wesl` does the same sums on the GPU;
//! `project_direction` is the plain copy the tests check.
//!
//! The **sky frame** is east, north and up, up being the orbital plane's
//! normal. Render space has the plane in XZ with +Y up and north at -Z
//! (`camera/rules.rs`), so east is +X.

use bevy::math::DVec3;
use bevy::prelude::*;

use crate::view::camera::rules::camera_rotation;

/// A sky-frame direction (east, north, up) in render space.
pub fn sky_to_render(direction: DVec3) -> Vec3 {
    Vec3::new(direction.x as f32, direction.z as f32, -direction.y as f32)
}

/// A render-space direction in the sky frame (east, north, up).
pub fn render_to_sky(direction: Vec3) -> DVec3 {
    DVec3::new(
        f64::from(direction.x),
        f64::from(-direction.z),
        f64::from(direction.y),
    )
}

/// The direction the camera looks along at a heading and tilt, in the sky
/// frame. For the standard view (heading 0, the standard tilt) it is the
/// line of sight the band is placed across (SKY-08).
pub fn line_of_sight(heading: f32, tilt: f32) -> DVec3 {
    render_to_sky(camera_rotation(heading, tilt) * Vec3::NEG_Z).normalize()
}

/// SKY-16: where a render-space `direction` at infinity lands in a
/// viewport of `viewport` pixels, for a camera turned by `rotation` with a
/// vertical field of view `fov_y`: pixels from the viewport's top left.
/// Only the turn is used, never a position, so panning, zooming and
/// changing the frame move no star. `None` for a direction behind the
/// camera, which is not drawn.
#[cfg(test)]
pub fn project_direction(
    direction: Vec3,
    rotation: Quat,
    fov_y: f32,
    viewport: Vec2,
) -> Option<Vec2> {
    let local = rotation.inverse() * direction;
    if local.z >= 0.0 {
        return None;
    }
    let half_tan = (fov_y / 2.0).tan();
    let aspect = viewport.x / viewport.y;
    let ndc = Vec2::new(
        local.x / (-local.z * half_tan * aspect),
        local.y / (-local.z * half_tan),
    );
    Some(Vec2::new(
        viewport.x * (ndc.x + 1.0) / 2.0,
        viewport.y * (1.0 - ndc.y) / 2.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::camera::rules::camera_transform;

    const VIEWPORT: Vec2 = Vec2::new(1280.0, 720.0);
    const FOV: f32 = std::f32::consts::FRAC_PI_4;

    /// The sky frame and render space agree: east is +X, north -Z, up +Y,
    /// and the round trip is exact.
    #[test]
    fn sky_frame_maps_onto_render_axes() {
        assert_eq!(sky_to_render(DVec3::X), Vec3::X);
        assert_eq!(sky_to_render(DVec3::Y), Vec3::NEG_Z);
        assert_eq!(sky_to_render(DVec3::Z), Vec3::Y);
        let d = DVec3::new(0.3, -0.5, 0.8).normalize();
        assert!((render_to_sky(sky_to_render(d)) - d).length() < 1e-6);
    }

    /// SKY-16: a star's screen position comes from the camera's turn only:
    /// cameras at different pivots and distances (a pan, a zoom, another
    /// frame body) put it in the same place.
    #[test]
    fn a_star_does_not_move_under_pan_zoom_or_a_change_of_frame() {
        let direction = sky_to_render(DVec3::new(0.1, 0.6, -0.3).normalize());
        let (heading, tilt) = (0.4, 1.0);
        let at = |pivot: Vec3, distance: f32| {
            let rotation = camera_transform(pivot, heading, tilt, distance).rotation;
            project_direction(direction, rotation, FOV, VIEWPORT).unwrap()
        };
        let start = at(Vec3::ZERO, 50.0);
        for (pivot, distance) in [
            (Vec3::new(30.0, 0.0, -12.0), 50.0),
            (Vec3::ZERO, 0.02),
            (Vec3::new(-400.0, 0.0, 900.0), 3_000.0),
        ] {
            assert!(at(pivot, distance).distance(start) < 1e-3);
        }
    }

    /// SKY-16: turning the heading by Δ turns the sky by Δ about the
    /// plane's normal: a direction turned by Δ, seen at the new heading,
    /// lands where the original did at the old one.
    #[test]
    fn a_change_of_heading_turns_the_sky_about_the_normal() {
        let direction = sky_to_render(DVec3::new(-0.4, 0.7, -0.2).normalize());
        let tilt = 0.9;
        for delta in [0.3_f32, -1.2, 2.5] {
            let before = project_direction(direction, camera_rotation(0.2, tilt), FOV, VIEWPORT);
            let turned = Quat::from_rotation_y(delta) * direction;
            let after =
                project_direction(turned, camera_rotation(0.2 + delta, tilt), FOV, VIEWPORT);
            assert!(before.unwrap().distance(after.unwrap()) < 1e-3);
        }
    }

    /// SKY-16: the line of sight lands at the viewport's centre, and a
    /// direction behind the camera is not drawn.
    #[test]
    fn the_line_of_sight_is_centred_and_behind_is_not_drawn() {
        let rotation = camera_rotation(0.7, 0.5);
        let ahead = sky_to_render(line_of_sight(0.7, 0.5));
        let centre = project_direction(ahead, rotation, FOV, VIEWPORT).unwrap();
        assert!(centre.distance(VIEWPORT / 2.0) < 1e-2);
        assert_eq!(project_direction(-ahead, rotation, FOV, VIEWPORT), None);
    }

    /// The standard view's line of sight looks down through the plane
    /// towards north, at the tilt from straight down.
    #[test]
    fn the_standard_line_of_sight_leans_north_by_the_tilt() {
        let tilt = 60_f32.to_radians();
        let los = line_of_sight(0.0, tilt);
        assert!(los.x.abs() < 1e-6);
        assert!((los.y - f64::from(tilt.sin())).abs() < 1e-6);
        assert!((los.z + f64::from(tilt.cos())).abs() < 1e-6);
    }
}
