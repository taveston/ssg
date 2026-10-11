//! Focus: the object the camera is locked onto (CAM-05 to CAM-08), as
//! shared view state. The camera tracks it; the inset, the markers and the
//! top bar show it. Selection (`selection.rs`) is separate and never moves
//! the camera.
//!
//! Like `selection`, it is its own small module so every reader can depend
//! on it without depending on the camera (ARCH-03).

use bevy::prelude::*;

/// The focused object, if any, and the pan offset of the pivot from it.
/// The offset is in render units, in the orbital plane, and does not turn
/// with the object's orbit (CAM-05).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    pub body: Option<Entity>,
    pub offset: Vec3,
}

impl Focus {
    /// Whether `body` is the focused object (ZOOM-17 asks).
    pub fn is_on(&self, body: Entity) -> bool {
        self.body == Some(body)
    }
}

/// CAM-05: while an object is focused, the pivot is its position plus the
/// pan offset, recomputed every frame.
pub fn focused_pivot(body_position: Vec3, offset: Vec3) -> Vec3 {
    body_position + offset
}

/// CAM-19: the pivot never moves further than `bound` from the star (the
/// origin), in the plane.
pub fn clamp_to_pan_bound(pivot: Vec3, bound: f32) -> Vec3 {
    let flat = Vec3::new(pivot.x, 0.0, pivot.z);
    if flat.length() > bound {
        flat.normalize() * bound
    } else {
        flat
    }
}

/// CAM-07 / CAM-19 while focused: the offset that keeps `body + offset`
/// inside the bound.
pub fn clamp_offset_to_pan_bound(body_position: Vec3, offset: Vec3, bound: f32) -> Vec3 {
    let body = Vec3::new(body_position.x, 0.0, body_position.z);
    clamp_to_pan_bound(body + offset, bound) - body
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-05: the pivot is the object's position plus the offset, and the
    /// offset is not turned by where the object is in its orbit.
    #[test]
    fn cam_05_pivot_is_position_plus_offset() {
        let offset = Vec3::new(2.0, 0.0, -1.0);
        for position in [Vec3::new(180.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -180.0)] {
            assert_eq!(focused_pivot(position, offset), position + offset);
        }
        assert_eq!(focused_pivot(Vec3::ONE, Vec3::ZERO), Vec3::ONE);
    }

    #[test]
    fn zoom_17_is_on_names_the_focused_body() {
        let mut world = World::new();
        let [body, other] = [(); 2].map(|_| world.spawn_empty().id());
        let focus = Focus {
            body: Some(body),
            offset: Vec3::ZERO,
        };
        assert!(focus.is_on(body));
        assert!(!focus.is_on(other));
        assert!(!Focus::default().is_on(body));
    }

    /// CAM-19 while focused: the offset is cut back so body plus offset
    /// stays on the bound.
    #[test]
    fn cam_19_focused_offset_stays_within_the_pan_bound() {
        let body = Vec3::new(90.0, 0.0, 0.0);
        let offset = clamp_offset_to_pan_bound(body, Vec3::new(50.0, 0.0, 0.0), 100.0);
        assert!((offset - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-4);
        let small = Vec3::new(1.0, 0.0, 1.0);
        assert!((clamp_offset_to_pan_bound(body, small, 100.0) - small).length() < 1e-4);
    }

    /// CAM-19: inside the bound the pivot is untouched; outside, it stops on
    /// the bound in the same direction.
    #[test]
    fn cam_19_pivot_stays_within_the_pan_bound() {
        let inside = Vec3::new(30.0, 0.0, 40.0);
        assert_eq!(clamp_to_pan_bound(inside, 100.0), inside);
        let outside = Vec3::new(300.0, 0.0, 400.0);
        let clamped = clamp_to_pan_bound(outside, 100.0);
        assert!((clamped.length() - 100.0).abs() < 1e-3);
        assert!(clamped.normalize().dot(outside.normalize()) > 0.9999);
    }
}
