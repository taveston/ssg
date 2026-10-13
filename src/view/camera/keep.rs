//! Keeping the frame body on screen while panning (CAM-07): outside the
//! star's frame, the pan offset is cut back until the body sits at least
//! `focus_keep_margin_px` inside the map area.

use bevy::prelude::*;

use super::fits::Fits;
use super::rules::project_plane_point;
use super::state::{ActiveMove, CameraRig};
use crate::view::frame::Frame;
use crate::view::screen::MapArea;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// Bisection steps when searching for the largest offset that fits.
const SEARCH_STEPS: usize = 24;

/// Whether `screen` is at least `margin` inside the map area of `viewport`.
pub fn inside_margin(screen: Vec2, viewport: Vec2, margin: f32) -> bool {
    let area = MapArea::of(viewport);
    let low = area.offset + Vec2::splat(margin);
    let high = area.offset + area.size - Vec2::splat(margin);
    screen.cmpge(low).all() && screen.cmple(high).all()
}

/// CAM-07: the largest part of `offset` (the same direction) that keeps the
/// body at `body` on screen, at least `margin` inside the map area, with the
/// camera looking at `body + offset`.
pub fn clamp_offset_to_margin(
    body: Vec3,
    offset: Vec3,
    (heading, tilt, distance): (f32, f32, f32),
    fov_y: f32,
    viewport: Vec2,
    margin: f32,
) -> Vec3 {
    let fits = |scale: f32| {
        let pivot = body + offset * scale;
        project_plane_point(body, (pivot, heading, tilt, distance), fov_y, viewport)
            .is_some_and(|screen| inside_margin(screen, viewport, margin))
    };
    if fits(1.0) || !fits(0.0) {
        return offset;
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..SEARCH_STEPS {
        let middle = (low + high) / 2.0;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    offset * low
}

/// CAM-07: outside the star's frame and not mid-move, keep the offset
/// inside the margin.
pub fn keep_frame_body_in_view(
    active: Res<ActiveMove>,
    positions: Res<WorldPositions>,
    tunables: Res<Tunables>,
    fits: Fits,
    rig: Res<CameraRig>,
    mut frame: ResMut<Frame>,
) {
    let Some(body) = frame.non_star_body() else {
        return;
    };
    let Some(&position) = positions.0.get(&body) else {
        return;
    };
    if active.0.is_some() {
        return;
    }
    let clamped = clamp_offset_to_margin(
        position,
        frame.offset,
        (rig.heading, rig.tilt, rig.distance),
        fits.fov(),
        fits.viewport(),
        tunables.focus_keep_margin_px,
    );
    if clamped != frame.offset {
        frame.offset = clamped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOV: f32 = std::f32::consts::FRAC_PI_4;
    const VIEWPORT: Vec2 = Vec2::new(1600.0, 900.0);

    /// CAM-07: a small pan is left alone; a huge one is cut back so the
    /// body ends up on the margin.
    #[test]
    fn offset_is_cut_back_to_the_margin() {
        let view = (0.0, 0.3, 50.0);
        let body = Vec3::new(10.0, 0.0, 0.0);
        let small = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(
            clamp_offset_to_margin(body, small, view, FOV, VIEWPORT, 40.0),
            small
        );
        let huge = Vec3::new(500.0, 0.0, 0.0);
        let clamped = clamp_offset_to_margin(body, huge, view, FOV, VIEWPORT, 40.0);
        assert!(clamped.length() < huge.length());
        let pivot = body + clamped;
        let screen = project_plane_point(body, (pivot, 0.0, 0.3, 50.0), FOV, VIEWPORT).unwrap();
        assert!(inside_margin(screen, VIEWPORT, 40.0 - 0.5));
        assert!(!inside_margin(screen, VIEWPORT, 40.0 + 1.0));
    }

    /// CAM-07: the projection puts the pivot at the middle of the window.
    #[test]
    fn the_pivot_projects_to_the_centre() {
        let pivot = Vec3::new(3.0, 0.0, -2.0);
        let screen = project_plane_point(pivot, (pivot, 0.4, 0.3, 20.0), FOV, VIEWPORT).unwrap();
        assert!((screen - VIEWPORT / 2.0).length() < 1e-2);
    }
}
