//! Where the last pointer press started (CAM-10, CAM-13), shared by the
//! camera's drag handling and the click handlers: a press that moved past
//! `drag_threshold_px` was a drag, and its release must not also select.
//!
//! Its own small module so the click handlers in `hover.rs` and the camera
//! can both read it without depending on each other (ARCH-03).

use bevy::prelude::*;

/// The screen position the most recent press started at. Kept after the
/// release, so a click arriving on the release frame can still be judged.
#[derive(Resource, Default, Debug)]
pub struct PressStart(pub Option<Vec2>);

/// CAM-10 / CAM-13: a press that has moved at least `threshold_px` from
/// where it started is a drag; less is a click.
pub fn is_drag(start: Vec2, now: Vec2, threshold_px: f32) -> bool {
    start.distance(now) >= threshold_px
}

/// CAM-10: whether a release at `release` ends a click rather than a drag.
/// With no press on record, it's a click.
pub fn was_click(press: &PressStart, release: Vec2, threshold_px: f32) -> bool {
    press
        .0
        .is_none_or(|start| !is_drag(start, release, threshold_px))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-10 / CAM-13: a tiny jitter is still a click; past the threshold
    /// it's a drag.
    #[test]
    fn cam_10_click_or_drag_by_the_threshold() {
        let start = Vec2::new(100.0, 100.0);
        assert!(!is_drag(start, Vec2::new(102.0, 101.0), 4.0));
        assert!(is_drag(start, Vec2::new(104.0, 100.0), 4.0));
        assert!(is_drag(start, Vec2::new(100.0, 140.0), 4.0));
    }

    /// CAM-10: the click that ends a drag doesn't select.
    #[test]
    fn cam_10_a_drag_release_is_not_a_click() {
        let press = PressStart(Some(Vec2::new(10.0, 10.0)));
        assert!(was_click(&press, Vec2::new(11.0, 12.0), 4.0));
        assert!(!was_click(&press, Vec2::new(60.0, 10.0), 4.0));
        assert!(was_click(&PressStart::default(), Vec2::ZERO, 4.0));
    }
}
