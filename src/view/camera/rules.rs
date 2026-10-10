//! The camera's geometry as plain functions (CAM-01, CAM-02, ZOOM-13,
//! CAM-10, CAM-14): where the camera sits for a pivot, heading, tilt and
//! distance; where a cursor ray meets the orbital plane; and which way is
//! "up the screen" along the plane.
//!
//! Conventions, from the spec's section 5a: the orbital plane is XZ with
//! its normal +Y. **North** is -Z. **Tilt** is measured from straight down
//! (0 = looking down the normal). **Heading** turns the view about +Y; at
//! heading 0 north is at the top of the screen.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// North, a fixed direction in the orbital plane for every system.
pub const NORTH: Vec3 = Vec3::NEG_Z;

/// CAM-01: tilt is clamped between straight down (0) and `max_tilt_deg`
/// from it, so the camera never goes below the orbital plane. Heading has
/// no limit and is never clamped.
pub fn clamp_tilt(tilt: f32, max_tilt_deg: f32) -> f32 {
    tilt.clamp(0.0, max_tilt_deg.to_radians())
}

/// CAM-02: the camera's orientation. At heading 0 and tilt 0 it looks
/// straight down with north at the top of the screen; tilting leans it
/// towards the top of the screen, and heading turns it about the normal.
pub fn camera_rotation(heading: f32, tilt: f32) -> Quat {
    Quat::from_rotation_y(heading) * Quat::from_rotation_x(-(FRAC_PI_2 - tilt))
}

/// The camera's transform: `distance` back from the pivot along its view.
pub fn camera_transform(pivot: Vec3, heading: f32, tilt: f32, distance: f32) -> Transform {
    let rotation = camera_rotation(heading, tilt);
    let forward = rotation * Vec3::NEG_Z;
    Transform {
        translation: pivot - forward * distance,
        rotation,
        ..default()
    }
}

/// CAM-10 / CAM-14: the screen's right and up directions laid along the
/// orbital plane, for a heading.
pub fn plane_axes(heading: f32) -> (Vec3, Vec3) {
    let turn = Quat::from_rotation_y(heading);
    (turn * Vec3::X, turn * NORTH)
}

/// ZOOM-13 / CAM-10: where a ray (from the camera through the cursor)
/// meets the orbital plane, if it does in front of the camera. A ray
/// parallel to the plane or pointing away from it has no plane point.
pub fn plane_point(ray: Ray3d) -> Option<Vec3> {
    let towards = ray.direction.y;
    if towards.abs() < f32::EPSILON {
        return None;
    }
    let t = -ray.origin.y / towards;
    (t > 0.0).then(|| {
        let point = ray.origin + *ray.direction * t;
        Vec3::new(point.x, 0.0, point.z)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    /// CAM-01: tilt stops at straight down and at the limit; never below
    /// the plane.
    #[test]
    fn cam_01_tilt_is_clamped() {
        assert_eq!(clamp_tilt(-0.3, 65.0), 0.0);
        assert!((clamp_tilt(2.0, 65.0) - 65f32.to_radians()).abs() < 1e-6);
        assert_eq!(clamp_tilt(0.5, 65.0), 0.5);
    }

    /// CAM-01: heading is unlimited - a full turn and more gives the same
    /// view as the remainder, with no clamp anywhere.
    #[test]
    fn cam_01_heading_is_unlimited() {
        let tilt = 0.4;
        let once = camera_transform(Vec3::ZERO, 0.7, tilt, 10.0);
        let thrice = camera_transform(Vec3::ZERO, 0.7 + 2.0 * std::f32::consts::TAU, tilt, 10.0);
        assert!(close(once.translation, thrice.translation));
        assert!(close(once.forward().into(), thrice.forward().into()));
    }

    /// CAM-02: the start state looks straight down with north at the top of
    /// the screen.
    #[test]
    fn cam_02_level_and_north_up() {
        let pivot = Vec3::new(5.0, 0.0, -3.0);
        let camera = camera_transform(pivot, 0.0, 0.0, 10.0);
        assert!(close(camera.translation, pivot + Vec3::Y * 10.0));
        assert!(close(camera.forward().into(), Vec3::NEG_Y));
        assert!(close(camera.up().into(), NORTH));
        assert!(close(camera.right().into(), Vec3::X));
    }

    /// Tilting leans the camera back from the pivot, away from north, and
    /// it keeps looking at the pivot at the same distance.
    #[test]
    fn cam_12_tilt_leans_the_camera_and_keeps_the_pivot_centred() {
        let pivot = Vec3::new(1.0, 0.0, 1.0);
        for (heading, tilt) in [(0.0, 0.5), (1.3, 1.0), (-2.0, 0.2)] {
            let camera = camera_transform(pivot, heading, tilt, 8.0);
            assert!((camera.translation.distance(pivot) - 8.0).abs() < 1e-4);
            let towards_pivot = (pivot - camera.translation).normalize();
            assert!(close(camera.forward().into(), towards_pivot));
            assert!((camera.translation.y - 8.0 * tilt.cos()).abs() < 1e-4);
        }
        let leaning = camera_transform(Vec3::ZERO, 0.0, 0.5, 8.0);
        assert!(
            leaning.translation.z > 0.0,
            "south of the pivot, looking north"
        );
    }

    /// The plane axes match the screen's at any heading.
    #[test]
    fn cam_14_plane_axes_follow_the_screen() {
        for heading in [0.0, 0.9, -2.4] {
            let camera = camera_transform(Vec3::ZERO, heading, 0.0, 5.0);
            let (right, up) = plane_axes(heading);
            assert!(close(right, camera.right().into()));
            assert!(close(up, camera.up().into()));
        }
    }

    /// ZOOM-13 / CAM-10: a ray down onto the plane meets it; one along it or
    /// away from it doesn't.
    #[test]
    fn zoom_13_ray_meets_the_orbital_plane() {
        let down = Ray3d::new(Vec3::new(3.0, 10.0, -2.0), Dir3::NEG_Y);
        assert_eq!(plane_point(down), Some(Vec3::new(3.0, 0.0, -2.0)));
        let slant = Ray3d::new(
            Vec3::new(0.0, 4.0, 0.0),
            Dir3::new(Vec3::new(1.0, -1.0, 0.0)).unwrap(),
        );
        assert!(close(plane_point(slant).unwrap(), Vec3::new(4.0, 0.0, 0.0)));
        let along = Ray3d::new(Vec3::new(0.0, 4.0, 0.0), Dir3::X);
        assert_eq!(plane_point(along), None);
        let away = Ray3d::new(Vec3::new(0.0, 4.0, 0.0), Dir3::Y);
        assert_eq!(plane_point(away), None);
    }
}
