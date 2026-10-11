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

/// CAM-14: the height of the view at the pivot, in render units, so a pan
/// speed given as a fraction of it looks the same at every zoom.
pub fn visible_height_at_pivot(distance: f32, fov_y: f32) -> f32 {
    2.0 * distance * (fov_y / 2.0).tan()
}

/// The settings key panning runs on (CAM-14).
#[derive(Debug, Clone, Copy)]
pub struct KeyPanSettings {
    /// Fraction of the visible height per second.
    pub screen_per_s: f32,
    /// Speed multiplier while Shift is held.
    pub fast_factor: f32,
}

/// CAM-14: the pan velocity a direction asks for, in plane units per
/// second along the screen's right (x) and up (y). Up the screen runs along
/// the tilted plane, which the view foreshortens by cos(tilt), so it is
/// sped up by the same factor to look the same at every tilt.
pub fn key_pan_velocity(
    direction: Vec2,
    visible_height: f32,
    tilt: f32,
    fast: bool,
    settings: KeyPanSettings,
) -> Vec2 {
    let Some(direction) = direction.try_normalize() else {
        return Vec2::ZERO;
    };
    let speed =
        settings.screen_per_s * visible_height * if fast { settings.fast_factor } else { 1.0 };
    let foreshortening = tilt.cos().max(MIN_FORESHORTENING);
    Vec2::new(direction.x, direction.y / foreshortening) * speed
}

/// Stops the up-screen speed-up blowing up near an edge-on view.
const MIN_FORESHORTENING: f32 = 0.2;

/// CAM-14: move a velocity towards its target, taking `ramp_s` to cover the
/// difference between rest and `full_speed`, so panning eases in and out.
pub fn ramp_velocity(current: Vec2, target: Vec2, full_speed: f32, dt: f32, ramp_s: f32) -> Vec2 {
    let step = if ramp_s > 0.0 {
        full_speed * dt / ramp_s
    } else {
        f32::INFINITY
    };
    let gap = target - current;
    if gap.length() <= step {
        target
    } else {
        current + gap.normalize() * step
    }
}

/// CAM-17: which way a cursor near the window edge scrolls, in screen
/// right (x) and up (y); zero away from the edges.
pub fn edge_scroll_direction(cursor: Vec2, window: Vec2, edge_px: f32) -> Vec2 {
    let axis = |at: f32, size: f32| {
        if at <= edge_px {
            -1.0
        } else if at >= size - edge_px {
            1.0
        } else {
            0.0
        }
    };
    // Screen y grows downwards; "up" is the top edge.
    Vec2::new(axis(cursor.x, window.x), -axis(cursor.y, window.y))
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

    const PAN: KeyPanSettings = KeyPanSettings {
        screen_per_s: 0.6,
        fast_factor: 3.0,
    };

    /// CAM-14: the same fraction of the view per second at every zoom.
    #[test]
    fn cam_14_key_pan_scales_with_the_view() {
        let near = key_pan_velocity(Vec2::X, visible_height_at_pivot(1.0, 0.8), 0.0, false, PAN);
        let far = key_pan_velocity(
            Vec2::X,
            visible_height_at_pivot(100.0, 0.8),
            0.0,
            false,
            PAN,
        );
        assert!((far.x / near.x - 100.0).abs() < 1e-3);
        assert!((near.x - 0.6 * visible_height_at_pivot(1.0, 0.8)).abs() < 1e-5);
    }

    /// CAM-14: up the screen is sped up by the tilt's foreshortening, so it
    /// looks as fast as sideways; Shift multiplies; diagonals aren't faster.
    #[test]
    fn cam_14_key_pan_looks_the_same_at_every_tilt() {
        let tilt = 1.0_f32;
        let up = key_pan_velocity(Vec2::Y, 10.0, tilt, false, PAN);
        let side = key_pan_velocity(Vec2::X, 10.0, tilt, false, PAN);
        assert!((up.y * tilt.cos() - side.x).abs() < 1e-4);
        let fast = key_pan_velocity(Vec2::X, 10.0, 0.0, true, PAN);
        assert!((fast.x - 3.0 * side.x).abs() < 1e-4);
        let diagonal = key_pan_velocity(Vec2::ONE, 10.0, 0.0, false, PAN);
        assert!((diagonal.length() - side.x).abs() < 1e-4);
        assert_eq!(
            key_pan_velocity(Vec2::ZERO, 10.0, 0.0, false, PAN),
            Vec2::ZERO
        );
    }

    /// CAM-14: the ramp reaches full speed in the ramp time, and stops in
    /// it too.
    #[test]
    fn cam_14_key_pan_ramps_up_and_down() {
        let full = Vec2::new(10.0, 0.0);
        let mut v = Vec2::ZERO;
        let dt = 0.01;
        for _ in 0..6 {
            v = ramp_velocity(v, full, 10.0, dt, 0.12);
        }
        assert!(v.x > 0.0 && v.x < 10.0, "part way after half the ramp: {v}");
        for _ in 0..7 {
            v = ramp_velocity(v, full, 10.0, dt, 0.12);
        }
        assert_eq!(v, full);
        for _ in 0..13 {
            v = ramp_velocity(v, Vec2::ZERO, 10.0, dt, 0.12);
        }
        assert_eq!(v, Vec2::ZERO);
    }

    /// CAM-17: only within the edge band does the cursor scroll.
    #[test]
    fn cam_17_edge_scroll_direction() {
        let window = Vec2::new(1600.0, 900.0);
        assert_eq!(
            edge_scroll_direction(Vec2::new(800.0, 450.0), window, 8.0),
            Vec2::ZERO
        );
        assert_eq!(
            edge_scroll_direction(Vec2::new(2.0, 450.0), window, 8.0),
            Vec2::new(-1.0, 0.0)
        );
        assert_eq!(
            edge_scroll_direction(Vec2::new(1599.0, 2.0), window, 8.0),
            Vec2::new(1.0, 1.0)
        );
        assert_eq!(
            edge_scroll_direction(Vec2::new(800.0, 898.0), window, 8.0),
            Vec2::new(0.0, -1.0)
        );
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
