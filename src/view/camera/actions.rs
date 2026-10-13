//! Camera actions: the animated moves that aren't zoom steps. Home
//! (ZOOM-10, CAM-04), N (CAM-03), moving into a body's frame (ZOOM-16,
//! CAM-06), re-centring on the frame body (CAM-25), and bookmarks
//! (ZOOM-11, CAM-06).
//!
//! Zoom is in `zoom_input.rs`, drags in `drag.rs`, panning and turning keys
//! in `keys.rs`; all read input through the bindings (CAM-18) and change
//! the camera through `control.rs`.

use bevy::prelude::*;

use super::control::CameraControl;
use super::fits::Fits;
use super::requests::{FrameRequest, RecentreRequest};
use super::rules::nearest_heading;
use super::state::{BOOKMARK_SLOTS, Bookmark, Bookmarks, CameraRig, MoveTarget, PivotTarget};
use crate::view::bindings::{Bindings, held, pressed};
use crate::view::motion_mode::MotionModeState;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// North up: heading 0.
const NORTH: f32 = 0.0;

/// ZOOM-10 / CAM-04: Home returns to the system fit over the star in the
/// standard orientation (CAM-02), in one move, and sets the star's frame
/// (CAM-08).
pub fn handle_home(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    fits: Fits,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    if !pressed(&keys, &bindings.home) {
        return;
    }
    let tilt = tunables.standard_tilt_deg.to_radians();
    view.settle(&positions);
    let target = MoveTarget {
        pivot: view.star_pivot(Vec3::ZERO),
        heading: nearest_heading(view.rig.heading, NORTH),
        tilt,
        distance: fits.system_fit(tilt),
    };
    view.start(target, tunables.focus_jump_ms);
}

/// CAM-03: N returns to the standard orientation (CAM-02), keeping the
/// frame with its offset, and the scale.
pub fn handle_north_up(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
    mut view: CameraControl,
) {
    if !pressed(&keys, &bindings.north_up) {
        return;
    }
    view.settle(&positions);
    let rig: CameraRig = *view.rig;
    let target = MoveTarget {
        pivot: view.frame_pivot(),
        heading: nearest_heading(rig.heading, NORTH),
        tilt: tunables.standard_tilt_deg.to_radians(),
        distance: rig.distance,
    };
    view.start(target, tunables.focus_jump_ms);
}

/// ZOOM-16 / CAM-06: make a body the frame with zero offset and move to it
/// in one move of `focus_jump_ms`, keeping heading and tilt: to its
/// satellite fit, its parent's, or its inner limit; for the star, to the
/// system fit. Asked for by a double-click, the inset's Focus button or a
/// breadcrumb entry (`FrameRequest`), or by F for the selection. The asker
/// has already selected the body. A body in exposure-ring mode can't be a
/// frame (CAM-24), so it is left only selected.
#[allow(clippy::too_many_arguments)]
pub fn handle_frame_requests(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    selection: Res<Selection>,
    fits: Fits,
    modes: Query<&MotionModeState>,
    positions: Res<WorldPositions>,
    mut request: ResMut<FrameRequest>,
    mut view: CameraControl,
) {
    let key = pressed(&keys, &bindings.focus_selection).then_some(selection.selected);
    let Some(body) = request.0.take().or(key.flatten()) else {
        return;
    };
    if modes.get(body).is_ok_and(|mode| mode.ring_mode) {
        return;
    }
    let tilt = view.rig.tilt;
    let distance = if body == view.frame.star {
        fits.system_fit(tilt)
    } else {
        fits.jump_distance(body, tilt)
    };
    view.settle(&positions);
    let pivot = PivotTarget {
        body,
        offset: Vec3::ZERO,
    };
    let target = MoveTarget::keeping_view(&view.rig, pivot, distance);
    view.start(target, tunables.focus_jump_ms);
}

/// CAM-25: the frame body's own breadcrumb entry re-centres it with zero
/// offset at the current scale, in one move.
pub fn handle_recentre_request(
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
    mut request: ResMut<RecentreRequest>,
    mut view: CameraControl,
) {
    if !std::mem::take(&mut request.0) {
        return;
    }
    view.settle(&positions);
    let pivot = PivotTarget {
        body: view.frame.body,
        offset: Vec3::ZERO,
    };
    let distance = view
        .active
        .0
        .map_or(view.rig.distance, |m| m.target.distance);
    let target = MoveTarget::keeping_view(&view.rig, pivot, distance);
    view.start(target, tunables.focus_jump_ms);
}

/// ZOOM-11 / CAM-06: Ctrl+1..4 save the view; 1..4 recall it - scale,
/// heading, tilt, and the frame with its offset. Recalling a bookmark saved
/// in the star's frame returns the frame to the star (CAM-08).
pub fn handle_bookmarks(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    tunables: Res<Tunables>,
    positions: Res<WorldPositions>,
    mut bookmarks: ResMut<Bookmarks>,
    mut view: CameraControl,
) {
    let saving = held(&keys, &bindings.bookmark_save);
    let slots = bindings
        .bookmark_slots
        .iter()
        .enumerate()
        .take(BOOKMARK_SLOTS);
    for (index, key) in slots {
        if !keys.just_pressed(*key) {
            continue;
        }
        view.settle(&positions);
        if saving {
            bookmarks.0[index] = Some(Bookmark {
                rig: *view.rig,
                frame: *view.frame,
            });
        } else if let Some(bookmark) = bookmarks.0[index] {
            let target = bookmark_target(&bookmark, view.rig.heading);
            view.start(target, tunables.focus_jump_ms);
        }
    }
}

/// Where recalling a bookmark goes: its frame with its offset, its heading
/// reached the short way round from `heading`.
fn bookmark_target(bookmark: &Bookmark, heading: f32) -> MoveTarget {
    MoveTarget {
        pivot: PivotTarget {
            body: bookmark.frame.body,
            offset: bookmark.frame.offset,
        },
        heading: nearest_heading(heading, bookmark.rig.heading),
        tilt: bookmark.rig.tilt,
        distance: bookmark.rig.distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::frame::Frame;

    /// CAM-06: a bookmark restores the whole view, and the frame with its
    /// offset.
    #[test]
    fn bookmark_restores_heading_tilt_and_frame() {
        let mut world = World::new();
        let [star, body] = [(); 2].map(|_| world.spawn_empty().id());
        let rig = CameraRig {
            pivot: Vec3::new(3.0, 0.0, 1.0),
            heading: 1.0,
            tilt: 0.6,
            distance: 12.0,
        };
        let framed = Bookmark {
            rig,
            frame: Frame::star_frame(star, Vec3::ZERO).on(body, Vec3::X),
        };
        let target = bookmark_target(&framed, 1.2);
        assert_eq!(
            target.pivot,
            PivotTarget {
                body,
                offset: Vec3::X
            }
        );
        assert!((target.heading - 1.0).abs() < 1e-5);
        assert_eq!((target.tilt, target.distance), (0.6, 12.0));

        let free = Bookmark {
            rig,
            frame: Frame::star_frame(star, rig.pivot),
        };
        let back = bookmark_target(&free, 1.0);
        assert!(back.frame(&framed.frame).in_star_frame(), "CAM-08");
        assert_eq!(back.pivot.offset, rig.pivot);
    }
}
